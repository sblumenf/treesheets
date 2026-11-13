//! What an interface in another language needs of the core: one grid to read
//! (`Editor::grid_view`), clips that leave the program and come back
//! (`Clip::as_document`, `Clip::from_document`), and an editor that may move to another
//! thread.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use proptest::prelude::*;
use support::{command, no_such_cell, op, Op};
use wombat_core::construct::Construct;
use wombat_core::model::{Extra, Link};
use wombat_core::{
    format, CellId, CellKind, Clip, Command, DocId, Document, Editor, Error, LinkTarget, SeededIds,
};

/// An editor on a document that random commands made of a 3 by 3 grid.
fn edited(seed: u64, ops: &[Op]) -> Editor {
    let mut ids = SeededIds::from_u64(seed);
    let doc = Document::new(&mut ids);
    let root = doc.root();
    let mut editor = Editor::new(doc).unwrap();
    let grid = Command::CreateGrid {
        cell: root,
        xs: 3,
        ys: 3,
    };
    editor.apply(grid, None, &mut ids).unwrap();
    for op in ops {
        // Commands that cannot be carried out are refused and change nothing.
        let _ = editor.apply(command(editor.document(), op), Some(1), &mut ids);
    }
    editor
}

fn enter(editor: &mut Editor, x: u32, y: u32, text: &str) {
    let doc = editor.document();
    let cell = doc.child(doc.root(), x, y).unwrap();
    let text = text.to_string();
    let mut ids = SeededIds::from_u64(7);
    editor
        .apply(Command::EnterText { cell, text }, None, &mut ids)
        .unwrap();
}

#[test]
fn a_grid_view_lists_the_cells_with_what_they_show() {
    let mut editor = edited(1, &[]);
    enter(&mut editor, 0, 0, "2");
    enter(&mut editor, 1, 0, "=20+22");
    enter(&mut editor, 2, 0, "=nosuchfunction(1)");
    let root = editor.document().root();
    let inner = editor.document().child(root, 0, 1).unwrap();
    let grid = Command::CreateGrid {
        cell: inner,
        xs: 1,
        ys: 2,
    };
    editor
        .apply(grid, None, &mut SeededIds::from_u64(9))
        .unwrap();

    let view = editor.grid_view(root).unwrap();
    assert_eq!((view.xs, view.ys, view.cells.len()), (3, 3, 9));
    assert_eq!(view.cell.id, root);
    assert!(view.cell.has_grid);
    let ids: Vec<_> = view.cells.iter().map(|c| c.id).collect();
    assert_eq!(
        ids,
        editor
            .document()
            .cell(root)
            .unwrap()
            .grid()
            .unwrap()
            .cells()
    );

    let data = &view.cells[0];
    assert_eq!((data.text.as_str(), data.kind), ("2", CellKind::Data));
    assert_eq!((data.result.as_deref(), data.failed), (None, false));

    let formula = &view.cells[1];
    assert_eq!(formula.text, "=20+22");
    assert_eq!(formula.kind, CellKind::Formula);
    assert_eq!(
        (formula.result.as_deref(), formula.failed),
        (Some("42"), false)
    );

    let broken = &view.cells[2];
    assert_eq!(broken.kind, CellKind::Formula);
    assert!(broken.failed, "{:?}", broken.result);
    assert_eq!(broken.result.as_deref(), editor.shown(broken.id));

    assert!(view.cells[3].has_grid);
    assert!(!view.cells[4].has_grid);
    let inside = editor.grid_view(inner).unwrap();
    assert_eq!((inside.xs, inside.ys, inside.cells.len()), (1, 2, 2));
}

#[test]
fn a_cell_without_a_grid_has_a_view_of_no_cells() {
    let mut ids = SeededIds::from_u64(2);
    let doc = Document::new(&mut ids);
    let root = doc.root();
    let editor = Editor::new(doc).unwrap();
    let view = editor.grid_view(root).unwrap();
    assert_eq!((view.xs, view.ys), (0, 0));
    assert!(view.cells.is_empty());
    assert!(!view.cell.has_grid);
    assert_eq!(view.cell.id, root);
}

#[test]
fn a_view_of_no_such_cell_is_not_found() {
    let editor = edited(3, &[]);
    assert!(matches!(
        editor.grid_view(no_such_cell()),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn a_clip_does_not_come_back_with_an_id_of_its_document_or_inside_itself() {
    let editor = edited(4, &[]);
    let doc = editor.document();
    let cell = doc.child(doc.root(), 1, 1).unwrap();
    let clip = Clip::of(doc, doc.root()).unwrap();
    let out = clip.as_document().unwrap();
    let invalid = |result: Result<Clip, Error>| match result {
        Err(Error::Invalid(message)) => message,
        other => panic!("{other:?}"),
    };
    // The top cell was not inside a cell with the ID of its document: no cell has that.
    let parent = Some(CellId(clip.source().0));
    assert!(invalid(Clip::from_document(&out, parent)).contains("the ID of the document"));
    // Nor inside a cell that is inside it.
    assert!(invalid(Clip::from_document(&out, Some(cell))).contains("among the cells"));
    assert_eq!(Clip::from_document(&out, None).unwrap(), clip);
}

/// An ID source that gives the same ID every time, which no real one does.
struct OneId;

impl wombat_core::IdSource for OneId {
    fn next_id(&mut self) -> wombat_core::Id {
        wombat_core::Id::from_u128(5)
    }
}

/// A document under construction is not checked, so a clip can be asked of one that
/// breaks rules which only the whole document has. Neither way out gives it.
#[test]
fn a_clip_of_a_document_that_breaks_the_rules_does_not_leave() {
    // A cell with the ID of its document.
    let doc = Document::new(&mut OneId);
    assert_eq!(doc.root().0, doc.id().0);
    assert!(doc.validate().is_err());
    let clip = Clip::of(&doc, doc.root()).unwrap();
    assert!(matches!(clip.as_document(), Err(Error::Invalid(_))));
    assert!(matches!(
        Clip::from_document(&doc, None),
        Err(Error::Invalid(_))
    ));

    // A link to a cell of the document that names the document.
    let mut ids = SeededIds::from_u64(6);
    let mut doc = Document::new(&mut ids);
    let (id, root) = (doc.id(), doc.root());
    let cell = doc.cell_mut(root).unwrap();
    cell.text = "x".into();
    cell.links.push(Link {
        start: 0,
        len: 1,
        target: LinkTarget::Cell {
            doc: Some(id),
            cell: root,
        },
        extra: Extra::new(),
    });
    assert!(doc.validate().is_err());
    let clip = Clip::of(&doc, root).unwrap();
    assert!(matches!(clip.as_document(), Err(Error::Invalid(_))));
}

/// Where the cells of a clip came from is said by the ID of the document that carries
/// them, and by nothing else.
#[test]
fn the_source_of_a_clip_is_the_id_of_the_document_it_comes_back_from() {
    let editor = edited(5, &[]);
    let doc = editor.document();
    let clip = Clip::of(doc, doc.child(doc.root(), 0, 0).unwrap()).unwrap();
    assert_eq!(clip.source(), doc.id());
    let out = clip.as_document().unwrap();
    assert_eq!(out.id(), doc.id());
    assert_eq!(
        Clip::from_document(&out, clip.parent()).unwrap().source(),
        doc.id()
    );
    // The same cells in a document with another ID are a clip from that document.
    let other = DocId(wombat_core::Id::from_u128(9));
    let text = format::to_string(&out).unwrap();
    let text = text.replace(&doc.id().to_string(), &other.to_string());
    let moved = format::from_slice(text.as_bytes()).unwrap();
    assert_eq!(moved.id(), other);
    let back = Clip::from_document(&moved, clip.parent()).unwrap();
    assert_eq!(back.source(), other);
    assert_eq!(back.as_document().unwrap(), moved);
    assert_ne!(back, clip);
}

/// The editor holds nothing that ties it to the thread it was made on, so an interface can
/// open and edit documents away from the thread that draws.
#[test]
fn an_editor_can_move_to_another_thread() {
    fn movable<T: Send + Sync + 'static>() {}
    movable::<Editor>();
    movable::<Clip>();
    movable::<Command>();
    movable::<Error>();
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// The view of every cell is the document and the results, and nothing else.
    #[test]
    fn views_agree_with_the_document(seed: u64, ops in proptest::collection::vec(op(), 0..30)) {
        let editor = edited(seed, &ops);
        let doc = editor.document();
        for id in doc.cells_in_order() {
            let cell = doc.cell(id).unwrap();
            let view = editor.grid_view(id).unwrap();
            prop_assert_eq!(view.cell.id, id);
            prop_assert_eq!(&view.cell.text, &cell.text);
            prop_assert_eq!(view.cell.has_grid, cell.grid().is_some());
            prop_assert_eq!(view.cell.kind, cell.kind);
            // Only a formula cell has a result, and every formula cell has one.
            prop_assert_eq!(view.cell.result.is_some(), cell.kind == CellKind::Formula);
            prop_assert_eq!(view.cell.result.as_deref().or(Some(&cell.text)), editor.shown(id));
            let cells: Vec<_> = view.cells.iter().map(|c| c.id).collect();
            match cell.grid() {
                Some(grid) => {
                    prop_assert_eq!((view.xs, view.ys), (grid.xs(), grid.ys()));
                    prop_assert_eq!(cells, grid.cells());
                }
                None => prop_assert_eq!((view.xs, view.ys, cells.len()), (0, 0, 0)),
            }
            for child in &view.cells {
                let found = doc.cell(child.id).unwrap();
                prop_assert_eq!(&child.text, &found.text);
                prop_assert_eq!(child.kind, found.kind);
                prop_assert_eq!(child.has_grid, found.grid().is_some());
                let shown = child.result.as_deref().unwrap_or(&child.text);
                prop_assert_eq!(Some(shown), editor.shown(child.id));
                prop_assert_eq!(child.result.is_some(), found.kind == CellKind::Formula);
            }
        }
    }

    /// A clip that left as a document, and that as the text of a file, comes back equal.
    #[test]
    fn clips_come_back_as_they_left(seed: u64, ops in proptest::collection::vec(op(), 0..30)) {
        let editor = edited(seed, &ops);
        let doc = editor.document();
        for id in doc.cells_in_order() {
            let clip = Clip::of(doc, id).unwrap();
            prop_assert_eq!(clip.parent(), doc.cell(id).unwrap().parent());
            let out = clip.as_document().unwrap();
            prop_assert_eq!(out.cell_count(), clip.cell_count());
            let back = Clip::from_document(&out, clip.parent()).unwrap();
            prop_assert_eq!(&back, &clip);
            let text = format::to_string(&out).unwrap();
            let read = format::from_slice(text.as_bytes()).unwrap();
            let back = Clip::from_document(&read, clip.parent()).unwrap();
            prop_assert_eq!(&back, &clip);
        }
    }
}
