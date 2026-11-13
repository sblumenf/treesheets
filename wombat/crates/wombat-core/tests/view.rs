//! What each level remembers on a device, and which level is shown: the part of the view
//! state that the first screen of the interface needs. The file form, what is refused,
//! which level a view state leads to in a document that changed under it, and how it
//! follows commands, undo and redo.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use proptest::prelude::*;
use serde_json::{json, Value};
use support::{act, command, no_such_cell, Act};
use wombat_core::view::{Level, MAX_LEVELS};
use wombat_core::{CellId, Changes, Command, Document, Editor, Error, SeededIds, ViewState};

/// A document three levels deep, with the cells the tests name.
struct Nest {
    editor: Editor,
    ids: SeededIds,
    root: CellId,
    /// In the root's grid (3 by 3), with a grid of 2 by 2.
    a: CellId,
    /// In the grid of `a`, with a grid of 2 columns and 3 rows.
    b: CellId,
    /// In the grid of `b`, without a grid.
    c: CellId,
}

impl Nest {
    fn new() -> Nest {
        let mut ids = SeededIds::from_u64(5);
        let doc = Document::new(&mut ids);
        let root = doc.root();
        let mut editor = Editor::new(doc).unwrap();
        let mut grid = |editor: &mut Editor, cell, xs, ys, x, y| {
            let create = Command::CreateGrid { cell, xs, ys };
            editor.apply(create, None, &mut ids).unwrap();
            editor.document().child(cell, x, y).unwrap()
        };
        let a = grid(&mut editor, root, 3, 3, 0, 0);
        let b = grid(&mut editor, a, 2, 2, 1, 1);
        let c = grid(&mut editor, b, 2, 3, 0, 0);
        Nest {
            editor,
            ids,
            root,
            a,
            b,
            c,
        }
    }

    fn doc(&self) -> &Document {
        self.editor.document()
    }

    fn apply(&mut self, command: Command) -> Changes {
        self.editor.apply(command, None, &mut self.ids).unwrap()
    }

    fn view(&self) -> ViewState {
        ViewState::new(self.doc().id())
    }
}

fn level(cell: CellId, x: u32, y: u32, selected: Option<CellId>) -> Level {
    Level {
        x,
        y,
        selected,
        ..Level::new(cell)
    }
}

fn has_grid(doc: &Document, cell: CellId) -> bool {
    doc.cell(cell).is_some_and(|c| c.grid().is_some())
}

#[test]
fn the_stage_is_the_zoom_cell_or_the_nearest_level_above_it() {
    let mut n = Nest::new();
    let mut view = n.view();
    assert_eq!(view.stage(n.doc()), n.root);
    for (zoom, stage) in [
        (n.b, n.b),
        (n.a, n.a),
        (n.root, n.root),
        // No grid: the level it is in.
        (n.c, n.b),
        // Not in the document, and nothing known of where it was: the root.
        (no_such_cell(), n.root),
    ] {
        view.zoom = Some(zoom);
        assert_eq!(view.stage(n.doc()), stage);
    }
    // The same after the grids went, one level further up each time.
    view.zoom = Some(n.c);
    n.apply(Command::RemoveGrid { cell: n.b });
    assert!(n.doc().cell(n.c).is_none() && n.doc().cell(n.b).is_some());
    view.zoom = Some(n.b);
    assert_eq!(view.stage(n.doc()), n.a);
    n.apply(Command::RemoveGrid { cell: n.a });
    view.zoom = Some(n.a);
    assert_eq!(view.stage(n.doc()), n.root);
    // The root is a level even without a grid: a new document.
    let empty = Document::new(&mut SeededIds::from_u64(8));
    let mut view = ViewState::new(empty.id());
    assert_eq!(view.stage(&empty), empty.root());
    view.zoom = Some(n.a);
    assert_eq!(view.stage(&empty), empty.root());
}

#[test]
fn going_to_a_level_needs_a_grid_there() {
    let n = Nest::new();
    let mut view = n.view();
    view.go(n.doc(), n.b).unwrap();
    assert_eq!((view.zoom, view.stage(n.doc())), (Some(n.b), n.b));
    assert!(matches!(view.go(n.doc(), n.c), Err(Error::Refused(_))));
    assert!(matches!(
        view.go(n.doc(), no_such_cell()),
        Err(Error::NotFound(_))
    ));
    assert_eq!(view.zoom, Some(n.b));
    // The root is not zoomed, and it is a level even in a document without a grid.
    view.go(n.doc(), n.root).unwrap();
    assert_eq!((view.zoom, view.stage(n.doc())), (None, n.root));
    let empty = Document::new(&mut SeededIds::from_u64(8));
    let mut view = ViewState::new(empty.id());
    view.go(&empty, empty.root()).unwrap();
    assert_eq!(view, ViewState::new(empty.id()));
    // Going somewhere remembers nothing and is nothing a file would show apart from the zoom.
    let mut gone = n.view();
    gone.go(n.doc(), n.b).unwrap();
    let mut set = n.view();
    set.zoom = Some(n.b);
    assert_eq!(gone, set);
    assert_eq!(gone.to_json().unwrap(), set.to_json().unwrap());
}

#[test]
fn a_level_is_remembered_once_and_the_oldest_are_forgotten() {
    let n = Nest::new();
    let mut view = n.view();
    assert_eq!(view.level(n.a), None);
    view.remember(n.doc(), level(n.a, 1, 0, Some(n.b))).unwrap();
    view.remember(n.doc(), level(n.root, 2, 1, None)).unwrap();
    assert_eq!(view.level(n.a), Some(&level(n.a, 1, 0, Some(n.b))));
    assert_eq!(view.level(n.b), None);
    // Again: in place of what was there, and now the one remembered last.
    view.remember(n.doc(), level(n.a, 0, 1, None)).unwrap();
    assert_eq!(
        view.levels,
        [level(n.root, 2, 1, None), level(n.a, 0, 1, None)]
    );
    // A position beyond the grid (2 columns, 3 rows) is its last column and row.
    view.remember(n.doc(), level(n.b, 2, 3, None)).unwrap();
    assert_eq!(view.level(n.b), Some(&level(n.b, 1, 2, None)));
    view.remember(n.doc(), level(n.b, u32::MAX, u32::MAX, None))
        .unwrap();
    assert_eq!(view.level(n.b), Some(&level(n.b, 1, 2, None)));
    // What is not a level, or names a cell that is not there, is refused and changes
    // nothing.
    let before = view.clone();
    assert!(matches!(
        view.remember(n.doc(), level(n.c, 0, 0, None)),
        Err(Error::Refused(_))
    ));
    assert!(matches!(
        view.remember(n.doc(), level(no_such_cell(), 0, 0, None)),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        view.remember(n.doc(), level(n.a, 0, 0, Some(no_such_cell()))),
        Err(Error::NotFound(_))
    ));
    assert_eq!(view, before);
}

/// A document whose root has a grid of `count` cells in one row, each with a grid.
fn wide(count: u32) -> (Editor, Vec<CellId>) {
    let mut ids = SeededIds::from_u64(6);
    let doc = Document::new(&mut ids);
    let root = doc.root();
    let mut editor = Editor::new(doc).unwrap();
    let row = Command::CreateGrid {
        cell: root,
        xs: count,
        ys: 1,
    };
    editor.apply(row, None, &mut ids).unwrap();
    let cells: Vec<_> = (0..count)
        .map(|x| editor.document().child(root, x, 0).unwrap())
        .collect();
    for cell in &cells {
        let grid = Command::CreateGrid {
            cell: *cell,
            xs: 1,
            ys: 1,
        };
        editor.apply(grid, None, &mut ids).unwrap();
    }
    (editor, cells)
}

#[test]
fn no_more_levels_are_remembered_than_the_limit() {
    let over = 3;
    let (editor, cells) = wide(MAX_LEVELS as u32 + over);
    let doc = editor.document();
    let mut view = ViewState::new(doc.id());
    for (i, cell) in cells.iter().enumerate() {
        view.remember(doc, Level::new(*cell)).unwrap();
        assert_eq!(view.levels.len(), (i + 1).min(MAX_LEVELS));
    }
    // The first ones went; the rest are in the order they were remembered.
    let kept: Vec<_> = view.levels.iter().map(|l| l.cell).collect();
    assert_eq!(kept, cells[over as usize..]);
    // Remembering one that is there forgets none.
    view.remember(doc, Level::new(cells[10])).unwrap();
    assert_eq!(view.levels.len(), MAX_LEVELS);
    assert_eq!(view.levels.last().unwrap().cell, cells[10]);
    assert_eq!(view.levels[0].cell, cells[over as usize]);

    // As many as that are written and read; one more is neither.
    let text = view.to_json().unwrap();
    assert_eq!(ViewState::from_json(text.as_bytes()).unwrap(), view);
    view.levels.push(Level::new(cells[0]));
    assert!(matches!(view.to_json(), Err(Error::Limit(_))));
    let mut file: Value = serde_json::from_str(&text).unwrap();
    let one = json!({"cell": cells[0].to_string()});
    file["levels"].as_array_mut().unwrap().push(one);
    assert!(matches!(
        ViewState::from_json(file.to_string().as_bytes()),
        Err(Error::Limit(_))
    ));
}

/// A view state with every part of a level in it, and its file.
fn written() -> (Nest, ViewState, String) {
    let n = Nest::new();
    let mut view = n.view();
    view.zoom = Some(n.b);
    view.folded.insert(n.a);
    view.extra.insert("scale".into(), json!(1.5));
    view.levels = vec![
        level(n.root, 0, 0, None),
        level(n.a, 1, 0, Some(n.b)),
        level(n.b, 0, 2, None),
    ];
    view.levels[1].extra.insert("later".into(), json!([1]));
    let text = view.to_json().unwrap();
    (n, view, text)
}

#[test]
fn levels_are_written_in_their_order_one_per_line() {
    let (n, view, text) = written();
    let (doc, root, a, b) = (n.doc().id(), n.root, n.a, n.b);
    assert_eq!(
        text,
        format!(
            "{{\n\"format\": \"wombat-view\", \"version\": 1, \"minor\": 1, \"doc\": \"{doc}\",\n\
             \"zoom\": \"{b}\",\n\
             \"levels\": [\n \
             {{\"cell\": \"{root}\"}},\n \
             {{\"cell\": \"{a}\", \"x\": 1, \"selected\": \"{b}\", \"later\": [1]}},\n \
             {{\"cell\": \"{b}\", \"y\": 2}}\n\
             ],\n\
             \"scale\": 1.5,\n\
             \"folded\": [\n \"{a}\"\n]\n}}\n"
        )
    );
    let back = ViewState::from_json(text.as_bytes()).unwrap();
    assert_eq!(back, view);
    assert_eq!(back.to_json().unwrap(), text);
    // The order is what was remembered last, not the order of the IDs: turned round it is
    // another view state and another file.
    let mut turned = view.clone();
    turned.levels.reverse();
    let other = turned.to_json().unwrap();
    assert_ne!(turned, view);
    assert_ne!(other, text);
    assert_eq!(ViewState::from_json(other.as_bytes()).unwrap(), turned);
}

#[test]
fn a_file_from_before_levels_loads_as_it_did() {
    let (n, view, _) = written();
    let (doc, a, b) = (n.doc().id(), n.a, n.b);
    // As minor version 0 wrote it, and with no minor version at all.
    for minor in ["\"minor\": 0, ", ""] {
        let old = format!(
            "{{\n\"format\": \"wombat-view\", \"version\": 1, {minor}\"doc\": \"{doc}\",\n\
             \"zoom\": \"{b}\",\n\"scale\": 1.5,\n\"folded\": [\n \"{a}\"\n]\n}}\n"
        );
        let loaded = ViewState::from_json(old.as_bytes()).unwrap();
        assert!(loaded.levels.is_empty());
        let mut expected = view.clone();
        expected.levels.clear();
        assert_eq!(loaded, expected);
        // Written again it says the minor version of this writer and nothing else new.
        assert_eq!(
            loaded.to_json().unwrap(),
            old.replace(minor, "")
                .replace("\"version\": 1, ", "\"version\": 1, \"minor\": 1, ")
        );
    }
}

/// A minor version may only add fields, which an older reader keeps as unknown ones. So
/// to a reader of minor version 0 this file is one of its own with a field it does not
/// know: without `levels` and with the old number it must be exactly what version 0 wrote.
/// (That the reader of version 0 does keep the field and writes it back was tried once
/// with that reader itself, before this was built; this test cannot run old code.)
#[test]
fn levels_are_only_an_added_field_to_an_older_reader() {
    let (_, view, text) = written();
    let mut file: Value = serde_json::from_str(&text).unwrap();
    let added = file.as_object_mut().unwrap().remove("levels");
    assert!(added.is_some());
    let mut without = view.clone();
    without.levels.clear();
    let old: Value = serde_json::from_str(&without.to_json().unwrap()).unwrap();
    assert_eq!(file, old);
    // And a newer file read here: fields this version does not know are kept, in the
    // view state and in a level, and the minor number is written back.
    let mut file: Value = serde_json::from_str(&text).unwrap();
    file["minor"] = json!(9);
    file["levels"][0]["zz"] = json!({"a": 1});
    file["zz"] = json!([2]);
    let newer = ViewState::from_json(file.to_string().as_bytes()).unwrap();
    assert_eq!(newer.levels[0].extra.get("zz"), Some(&json!({"a": 1})));
    let again: Value = serde_json::from_str(&newer.to_json().unwrap()).unwrap();
    assert_eq!(again, file);
}

#[test]
fn bad_levels_are_refused() {
    let (n, view, text) = written();
    let good: Value = serde_json::from_str(&text).unwrap();
    let with = |change: &dyn Fn(&mut Value)| {
        let mut file = good.clone();
        change(&mut file["levels"]);
        ViewState::from_json(file.to_string().as_bytes())
    };
    assert_eq!(with(&|_| ()).unwrap(), view);
    let bad: [&dyn Fn(&mut Value); 13] = [
        &|levels| *levels = json!({}),
        &|levels| *levels = json!("x"),
        &|levels| levels[0] = json!("x"),
        &|levels| levels[0] = json!([]),
        &|levels| levels[0] = json!({}),
        &|levels| levels[0] = json!({"x": 1}),
        &|levels| levels[0]["cell"] = json!("x"),
        &|levels| levels[0]["cell"] = json!(1),
        &|levels| levels[1]["x"] = json!(-1),
        &|levels| levels[1]["x"] = json!(4_294_967_296_u64),
        &|levels| levels[2]["y"] = json!(1.5),
        &|levels| levels[2]["y"] = json!("2"),
        &|levels| levels[1]["selected"] = json!("x"),
    ];
    for (i, change) in bad.iter().enumerate() {
        assert!(matches!(with(change), Err(Error::Invalid(_))), "case {i}");
    }
    // The largest position there is.
    let far = with(&|levels| levels[1]["x"] = json!(u32::MAX)).unwrap();
    assert_eq!(far.levels[1].x, u32::MAX);
    // No level twice.
    let twice = with(&|levels| levels[2]["cell"] = json!(n.a.to_string()));
    assert!(matches!(twice, Err(Error::Invalid(_))));
    // No key twice in a level, as everywhere.
    let repeated = text.replacen("\"y\": 2", "\"y\": 2, \"y\": 2", 1);
    assert_ne!(repeated, text);
    assert!(matches!(
        ViewState::from_json(repeated.as_bytes()),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn levels_that_could_not_be_read_back_are_not_written() {
    let (n, good, _) = written();
    let mut view = good.clone();
    view.levels.push(Level::new(n.a));
    assert!(matches!(view.to_json(), Err(Error::Invalid(_))));
    let mut view = good.clone();
    view.levels[0].extra.insert("selected".into(), json!(1));
    assert!(matches!(view.to_json(), Err(Error::Invalid(_))));
    let mut view = good.clone();
    view.extra.insert("levels".into(), json!([]));
    assert!(matches!(view.to_json(), Err(Error::Invalid(_))));
    // An unknown field of a level may nest as deep as any other, and no deeper.
    let nested = |depth: usize| (0..depth).fold(json!(1), |inner, _| json!([inner]));
    let mut view = good.clone();
    let at = wombat_core::limits::MAX_UNKNOWN_DEPTH as usize;
    view.levels[0].extra.insert("zz".into(), nested(at));
    let text = view.to_json().unwrap();
    assert_eq!(ViewState::from_json(text.as_bytes()).unwrap(), view);
    view.levels[0].extra.insert("zz".into(), nested(at + 1));
    assert!(matches!(view.to_json(), Err(Error::Limit(_))));
}

#[test]
fn stale_levels_are_dropped() {
    let (n, mut view, _) = written();
    let doc = n.doc();
    view.levels[0].selected = Some(no_such_cell());
    // Beyond the grid of `a` (2 by 2), as after columns and rows were removed elsewhere.
    view.levels[1].x = 7;
    view.levels[1].y = 9;
    // A cell without a grid is no level, and one that is not there neither.
    view.levels.push(level(n.c, 0, 0, None));
    view.levels.push(level(no_such_cell(), 1, 1, Some(n.a)));
    view.retain_existing(doc);
    let mut second = level(n.a, 1, 1, Some(n.b));
    second.extra.insert("later".into(), json!([1]));
    assert_eq!(
        view.levels,
        [level(n.root, 0, 0, None), second, level(n.b, 0, 2, None)]
    );
    assert_eq!(view.zoom, Some(n.b));
    // A zoom cell that is in the document stays, grid or not; one that is not goes, and
    // where it was is not known.
    view.zoom = Some(n.c);
    view.retain_existing(doc);
    assert_eq!((view.zoom, view.stage(doc)), (Some(n.c), n.b));
    view.zoom = Some(no_such_cell());
    view.retain_existing(doc);
    assert_eq!((view.zoom, view.stage(doc)), (None, n.root));
}

#[test]
fn the_levels_follow_a_copy_with_fresh_ids() {
    let (n, mut view, _) = written();
    view.levels.push(level(no_such_cell(), 3, 4, None));
    view.levels[2].selected = Some(no_such_cell());
    let (copy, fresh) = n
        .doc()
        .with_fresh_ids(&mut SeededIds::from_u64(40))
        .unwrap();
    let moved = view.with_fresh_ids(&fresh);
    let to = |cell| fresh.cell(cell).unwrap();
    let mut second = level(to(n.a), 1, 0, Some(to(n.b)));
    second.extra.insert("later".into(), json!([1]));
    // Every ID is the copy's; the level of a cell the document does not have is dropped,
    // and so is a selected cell it does not have.
    assert_eq!(
        moved.levels,
        [
            level(to(n.root), 0, 0, None),
            second,
            level(to(n.b), 0, 2, None)
        ]
    );
    for l in &moved.levels {
        assert!(has_grid(&copy, l.cell) && n.doc().cell(l.cell).is_none());
    }
    assert_eq!((moved.zoom, moved.stage(&copy)), (Some(to(n.b)), to(n.b)));
    assert_eq!(moved.doc, copy.id());
    // Nothing of the original is left in the file of the copy.
    let text = moved.to_json().unwrap();
    for id in n.doc().cells_in_order() {
        assert!(!text.contains(&id.to_string()));
    }
}

/// The view state after `changes`, checked: it names only cells that are there.
fn follow(view: &mut ViewState, doc: &Document, changes: &Changes) {
    let before = view.clone();
    let changed = view.apply_changes(doc, changes);
    // It says whether there is something new to write to the file.
    assert_eq!(changed, *view != before);
    assert_eq!(
        changed,
        view.to_json().unwrap() != before.to_json().unwrap()
    );
    check_view(view, doc);
}

/// Everything a view state promises of a document it has followed.
fn check_view(view: &ViewState, doc: &Document) {
    for id in &view.folded {
        assert!(has_grid(doc, *id), "fold of {id}");
    }
    if let Some(selection) = &view.selection {
        assert!(doc.cell(selection.cell).is_some(), "selection");
    }
    if let Some(zoom) = view.zoom {
        assert!(doc.cell(zoom).is_some(), "zoom");
    }
    assert!(view.levels.len() <= MAX_LEVELS);
    let mut seen = std::collections::BTreeSet::new();
    for l in &view.levels {
        let grid = doc.cell(l.cell).and_then(|c| c.grid());
        let grid = grid.unwrap_or_else(|| panic!("the level {} has no grid", l.cell));
        assert!(l.x < grid.xs() && l.y < grid.ys(), "position of {}", l.cell);
        assert!(l.selected.is_none_or(|id| doc.cell(id).is_some()));
        assert!(seen.insert(l.cell), "the level {} twice", l.cell);
    }
    let stage = view.stage(doc);
    assert!(has_grid(doc, stage) || stage == doc.root(), "stage");
    // Whatever it is, it can be written and read.
    let text = view.to_json().unwrap();
    assert_eq!(&ViewState::from_json(text.as_bytes()).unwrap(), view);
}

#[test]
fn what_a_level_remembers_goes_with_its_cell_and_does_not_come_back() {
    let mut n = Nest::new();
    let mut view = n.view();
    for l in [
        level(n.root, 2, 2, Some(n.a)),
        level(n.a, 1, 1, Some(n.b)),
        level(n.b, 1, 2, Some(n.c)),
    ] {
        view.remember(n.doc(), l).unwrap();
    }
    // A row of `b` goes: the position moves into what is left, the rest stays.
    let changes = n.apply(Command::DeleteRows {
        cell: n.b,
        at: 1,
        count: 2,
    });
    follow(&mut view, n.doc(), &changes);
    assert_eq!(view.level(n.b), Some(&level(n.b, 1, 0, Some(n.c))));
    // The selected cell goes: only that is forgotten.
    let changes = n.apply(Command::DeleteColumns {
        cell: n.b,
        at: 0,
        count: 1,
    });
    follow(&mut view, n.doc(), &changes);
    assert!(n.doc().cell(n.c).is_none());
    assert_eq!(view.level(n.b), Some(&level(n.b, 0, 0, None)));
    // The grid goes and the cell stays: no level any more.
    let changes = n.apply(Command::RemoveGrid { cell: n.b });
    follow(&mut view, n.doc(), &changes);
    assert_eq!(view.level(n.b), None);
    assert_eq!(view.level(n.a), Some(&level(n.a, 1, 1, Some(n.b))));
    // Undo brings the grid back, not what was remembered (A38).
    let changes = n.editor.undo().unwrap().unwrap();
    follow(&mut view, n.doc(), &changes);
    assert!(has_grid(n.doc(), n.b));
    assert_eq!(view.level(n.b), None);
    // The cell itself goes, with the level in it: what `a` remembered is forgotten, and
    // the root no longer has it selected, nor a third row.
    let changes = n.apply(Command::DeleteRows {
        cell: n.root,
        at: 0,
        count: 1,
    });
    follow(&mut view, n.doc(), &changes);
    assert!(n.doc().cell(n.a).is_none());
    assert_eq!(view.levels, [level(n.root, 2, 1, None)]);
    let changes = n.editor.undo().unwrap().unwrap();
    follow(&mut view, n.doc(), &changes);
    assert!(has_grid(n.doc(), n.a));
    assert_eq!(view.levels, [level(n.root, 2, 1, None)]);
}

#[test]
fn when_the_level_shown_is_removed_the_screen_goes_up_as_little_as_it_must() {
    let mut n = Nest::new();
    let mut view = n.view();
    view.go(n.doc(), n.b).unwrap();
    // A cell inside changes: nothing moves.
    let changes = n.apply(Command::EnterText {
        cell: n.c,
        text: "x".into(),
    });
    follow(&mut view, n.doc(), &changes);
    assert_eq!((view.zoom, view.stage(n.doc())), (Some(n.b), n.b));
    // The level shown is removed with the grid it is in: the level above it.
    let changes = n.apply(Command::RemoveGrid { cell: n.a });
    follow(&mut view, n.doc(), &changes);
    assert!(n.doc().cell(n.b).is_none() && !has_grid(n.doc(), n.a));
    // `a` is still there but has no grid, so the one above that: the root, not zoomed.
    assert_eq!((view.zoom, view.stage(n.doc())), (None, n.root));
    // Undo brings `b` back; the zoom stays where it went (A38).
    let changes = n.editor.undo().unwrap().unwrap();
    follow(&mut view, n.doc(), &changes);
    assert!(has_grid(n.doc(), n.b));
    assert_eq!((view.zoom, view.stage(n.doc())), (None, n.root));

    // Removed from further up, while a level between survives with its grid.
    view.go(n.doc(), n.b).unwrap();
    let row = n.apply(Command::DeleteRows {
        cell: n.a,
        at: 1,
        count: 1,
    });
    follow(&mut view, n.doc(), &row);
    assert!(n.doc().cell(n.b).is_none() && has_grid(n.doc(), n.a));
    assert_eq!((view.zoom, view.stage(n.doc())), (Some(n.a), n.a));
    let changes = n.editor.undo().unwrap().unwrap();
    follow(&mut view, n.doc(), &changes);
    assert_eq!((view.zoom, view.stage(n.doc())), (Some(n.a), n.a));

    // A zoom set without `go` is followed too, from the next report on.
    let mut view = n.view();
    view.zoom = Some(n.b);
    follow(&mut view, n.doc(), &Changes::default());
    let changes = n.editor.redo().unwrap().unwrap();
    follow(&mut view, n.doc(), &changes);
    assert_eq!((view.zoom, view.stage(n.doc())), (Some(n.a), n.a));

    // The level shown only loses its grid: the zoom stays on the cell, the stage is the
    // level above for as long as that lasts.
    let changes = n.editor.undo().unwrap().unwrap();
    follow(&mut view, n.doc(), &changes);
    view.go(n.doc(), n.b).unwrap();
    let changes = n.apply(Command::RemoveGrid { cell: n.b });
    follow(&mut view, n.doc(), &changes);
    assert_eq!((view.zoom, view.stage(n.doc())), (Some(n.b), n.a));
    // Then the cell goes as well: still the level above, not the root.
    let changes = n.apply(Command::DeleteRows {
        cell: n.a,
        at: 1,
        count: 1,
    });
    follow(&mut view, n.doc(), &changes);
    assert_eq!((view.zoom, view.stage(n.doc())), (Some(n.a), n.a));
}

/// What the view state does between two moves of a walk: raw numbers, fitted to the
/// document as it is then.
#[derive(Clone, Debug)]
struct Look {
    kind: u8,
    n: [u32; 4],
}

fn look() -> impl Strategy<Value = Look> {
    (0..6u8, any::<[u32; 4]>()).prop_map(|(kind, n)| Look { kind, n })
}

/// Goes somewhere, remembers a level, folds or selects, as a screen would; what the view
/// state refuses is refused and changes nothing.
fn do_look(view: &mut ViewState, doc: &Document, look: &Look) {
    let cells = doc.cells_in_order();
    let pick = |n: u32| cells[n as usize % cells.len()];
    let with_grid: Vec<_> = cells.iter().filter(|c| has_grid(doc, **c)).collect();
    // Mostly a cell that has a grid, since the others are refused.
    let level_cell = |n: u32| match with_grid.len() {
        0 => pick(n),
        _ if n.is_multiple_of(8) => pick(n / 8),
        len => *with_grid[n as usize % len],
    };
    let before = view.clone();
    let result = match look.kind {
        0 | 1 => view.go(doc, level_cell(look.n[0])),
        2 | 3 => {
            let cell = level_cell(look.n[0]);
            let selected = (!look.n[3].is_multiple_of(3)).then(|| pick(look.n[3]));
            view.remember(doc, level(cell, look.n[1] % 5, look.n[2] % 5, selected))
        }
        4 => view
            .set_folded(doc, level_cell(look.n[0]), look.n[1].is_multiple_of(2))
            .map(|_| ()),
        _ => {
            view.selection = Some(wombat_core::Selection {
                cell: pick(look.n[0]),
                xs: 1,
                ys: 1,
                cursor: None,
                extra: Default::default(),
            });
            Ok(())
        }
    };
    if result.is_err() {
        assert_eq!(*view, before, "refused, but changed something");
    }
    check_view(view, doc);
}

/// Where the screen has to be after a change, worked out from the document before it: the
/// zoom cell, or the first cell above where it was that is still there, and from there up
/// to the first with a grid.
fn expected_stage(before: &Document, after: &Document, zoom: Option<CellId>) -> CellId {
    let mut line = Vec::new();
    let mut at = zoom;
    while let Some(cell) = at.and_then(|id| before.cell(id)) {
        line.push(cell.id());
        at = cell.parent();
    }
    let mut at = line.into_iter().find(|id| after.cell(*id).is_some());
    while let Some(cell) = at.and_then(|id| after.cell(id)) {
        if cell.grid().is_some() {
            return cell.id();
        }
        at = cell.parent();
    }
    after.root()
}

/// Follows one report and checks where the screen went.
fn follow_from(view: &mut ViewState, before: &Document, editor: &Editor, changes: &Changes) {
    let zoom = view.zoom;
    let levels: Vec<_> = view.levels.iter().map(|l| l.cell).collect();
    let doc = editor.document();
    follow(view, doc, changes);
    assert_eq!(view.stage(doc), expected_stage(before, doc, zoom));
    // A level is kept exactly as long as its cell has a grid, in the order it had.
    let kept: Vec<_> = view.levels.iter().map(|l| l.cell).collect();
    let should: Vec<_> = levels.into_iter().filter(|c| has_grid(doc, *c)).collect();
    assert_eq!(kept, should);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// After any commands, typing, groups, undos and redos, with the view state told of
    /// every report, it names only cells that exist, every level has a grid, and the level
    /// to show is the one nearest to where the screen was.
    #[test]
    fn the_view_state_follows_any_walk_through_the_history(
        steps in proptest::collection::vec((act(), look(), look()), 0..40),
    ) {
        let n = Nest::new();
        let (mut editor, mut ids) = (n.editor, n.ids);
        let mut view = ViewState::new(editor.document().id());
        for (act, first, second) in &steps {
            do_look(&mut view, editor.document(), first);
            let before = editor.document().clone();
            match act {
                Act::Apply(op) | Act::Type(op) => {
                    let command = command(&before, op);
                    let result = match act {
                        Act::Type(_) => editor.apply_typing(command, Some(1), &mut ids),
                        _ => editor.apply(command, Some(1), &mut ids),
                    };
                    if let Ok(changes) = result {
                        follow_from(&mut view, &before, &editor, &changes);
                    }
                }
                Act::Group(ops) => {
                    editor.begin_group();
                    for op in ops {
                        // The view state is told of each command as it comes, and looks
                        // around in between, as a screen does while a group is open.
                        let before = editor.document().clone();
                        let command = command(&before, op);
                        if let Ok(changes) = editor.apply(command, Some(1), &mut ids) {
                            follow_from(&mut view, &before, &editor, &changes);
                        }
                        do_look(&mut view, editor.document(), second);
                    }
                    let changes = editor.end_group();
                    // Told again of the whole group: nothing more to do.
                    let told = view.clone();
                    follow(&mut view, editor.document(), &changes);
                    prop_assert_eq!(&view, &told);
                }
                Act::Cancelled(ops) => {
                    editor.begin_group();
                    for op in ops {
                        let before = editor.document().clone();
                        let command = command(&before, op);
                        if let Ok(changes) = editor.apply(command, Some(1), &mut ids) {
                            follow_from(&mut view, &before, &editor, &changes);
                        }
                        do_look(&mut view, editor.document(), second);
                    }
                    // The cancel is followed like an undo: from the document just before it.
                    let before = editor.document().clone();
                    let changes = editor.cancel_group().unwrap();
                    follow_from(&mut view, &before, &editor, &changes);
                }
                Act::Undo => {
                    if let Some(changes) = editor.undo().unwrap() {
                        follow_from(&mut view, &before, &editor, &changes);
                    }
                }
                Act::Redo => {
                    if let Some(changes) = editor.redo().unwrap() {
                        follow_from(&mut view, &before, &editor, &changes);
                    }
                }
                Act::Limit(units) => editor.set_history_limit(Some(*units)),
            }
            do_look(&mut view, editor.document(), second);
            // What following left is what a fresh look at the document would leave.
            let mut again = view.clone();
            again.retain_existing(editor.document());
            prop_assert_eq!(&again, &view);
        }
    }
}

#[test]
fn a_level_that_moved_is_followed_from_where_it_is_now() {
    let mut n = Nest::new();
    let mut view = n.view();
    view.go(n.doc(), n.b).unwrap();
    view.remember(n.doc(), level(n.b, 1, 2, Some(n.c))).unwrap();
    // `b` trades places with a cell of the root's grid, in its last row.
    let other = n.doc().child(n.root, 2, 2).unwrap();
    let changes = n.apply(Command::SwapCells { a: n.b, b: other });
    follow(&mut view, n.doc(), &changes);
    assert_eq!(n.doc().cell(n.b).unwrap().parent(), Some(n.root));
    // Still the level shown, with what it remembered.
    assert_eq!((view.zoom, view.stage(n.doc())), (Some(n.b), n.b));
    assert_eq!(view.level(n.b), Some(&level(n.b, 1, 2, Some(n.c))));
    // Removed from there, the screen goes to what is above it now, the root, and not to
    // `a`, which was above it when it was gone to and is still there with its grid.
    let changes = n.apply(Command::DeleteRows {
        cell: n.root,
        at: 2,
        count: 1,
    });
    follow(&mut view, n.doc(), &changes);
    assert!(n.doc().cell(n.b).is_none() && has_grid(n.doc(), n.a));
    assert_eq!((view.zoom, view.stage(n.doc())), (None, n.root));
    assert!(view.levels.is_empty());
}

#[test]
fn following_a_change_says_whether_the_view_state_changed() {
    let mut n = Nest::new();
    let mut view = n.view();
    view.remember(n.doc(), level(n.b, 1, 2, None)).unwrap();
    let text = n.apply(Command::EnterText {
        cell: n.c,
        text: "x".into(),
    });
    assert!(!view.apply_changes(n.doc(), &text));
    let row = Command::DeleteRows {
        cell: n.b,
        at: 2,
        count: 1,
    };
    let changes = n.apply(row);
    assert!(view.apply_changes(n.doc(), &changes));
    assert!(!view.apply_changes(n.doc(), &changes));
    // A fold that goes counts as well, also when no level does.
    view.set_folded(n.doc(), n.b, true).unwrap();
    view.levels.clear();
    let changes = n.apply(Command::RemoveGrid { cell: n.b });
    assert!(view.apply_changes(n.doc(), &changes));
    assert!(!view.is_folded(n.b));
}
