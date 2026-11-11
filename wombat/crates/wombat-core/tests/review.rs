//! Tests for the findings of the review of milestone 3 (see "Fixes after review" in
//! `docs/milestones/M3a.md`). Each of the first tests failed before its fix.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use wombat_core::construct::Construct;
use wombat_core::{
    format, CellId, Color, Command, Document, Editor, Error, Id, IdSource, LinkTarget, Prop,
    SeededIds,
};

const NOW: Option<i64> = Some(1_700_000_000_000);

/// A document whose root has a grid of `xs` by `ys` cells with the texts "0", "1", ...
fn table(xs: u32, ys: u32) -> (Editor, SeededIds, Vec<CellId>) {
    let mut ids = SeededIds::from_u64(3);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), xs, ys, &mut ids).unwrap();
    let cells = doc
        .cell(doc.root())
        .unwrap()
        .grid()
        .unwrap()
        .cells()
        .to_vec();
    for (n, id) in cells.iter().enumerate() {
        doc.cell_mut(*id).unwrap().text = n.to_string();
    }
    (Editor::new(doc).unwrap(), ids, cells)
}

fn saves_and_reloads(editor: &Editor) {
    let text = format::to_string(editor.document()).expect("the document must save");
    assert_eq!(
        format::from_slice(text.as_bytes()).unwrap(),
        *editor.document()
    );
}

// ---- 1: what commands build can be saved ----------------------------------------------------

#[test]
fn a_grid_too_large_to_save_is_refused_and_one_that_is_made_saves() {
    let mut ids = SeededIds::from_u64(1);
    let mut editor = Editor::new(Document::new(&mut ids)).unwrap();
    let root = editor.document().root();
    let huge = Command::CreateGrid {
        cell: root,
        xs: 1000,
        ys: 1000,
    };
    match editor.apply(huge, None, &mut ids) {
        Ok(_) => saves_and_reloads(&editor),
        Err(error) => {
            assert!(matches!(error, Error::Limit(_)), "{error}");
            assert_eq!(editor.document().cell_count(), 1);
        }
    }
}

#[test]
fn a_text_too_large_to_save_is_refused() {
    let (mut editor, mut ids, cells) = table(1, 1);
    let command = Command::SetText {
        cell: cells[0],
        text: "\u{1F600}".repeat(9_000_000),
    };
    match editor.apply(command, None, &mut ids) {
        Ok(_) => saves_and_reloads(&editor),
        Err(error) => assert!(matches!(error, Error::Limit(_)), "{error}"),
    }
    assert_eq!(editor.document().cell(cells[0]).unwrap().text, "0");
}

// ---- 2: asking for nothing new is nothing ---------------------------------------------------

fn props(cell: CellId, props: &[Prop]) -> Command {
    Command::SetProps {
        cells: vec![cell],
        props: props.to_vec(),
    }
}

#[test]
fn setting_the_colour_a_cell_has_does_not_stamp_or_enter_the_history() {
    let (mut editor, mut ids, cells) = table(2, 1);
    let cell = cells[0];
    for prop in [Prop::CellColor(Color::WHITE), Prop::TextColor(Color::BLACK)] {
        let changes = editor.apply(props(cell, &[prop]), NOW, &mut ids).unwrap();
        assert!(changes.is_empty(), "{changes:?}");
        assert_eq!(editor.undo_len(), 0);
        assert_eq!(editor.document().cell(cell).unwrap().last_edit, None);
    }
}

#[test]
fn a_colour_set_and_set_back_in_one_command_is_nothing() {
    let (mut editor, mut ids, cells) = table(2, 1);
    let cell = cells[0];
    let red = Color::rgb(255, 0, 0);
    let before = editor.document().clone();
    for there_and_back in [
        vec![Prop::CellColor(red), Prop::CellColor(Color::WHITE)],
        vec![Prop::TextColor(red), Prop::TextColor(Color::BLACK)],
        vec![Prop::RelSize(3), Prop::RelSize(0)],
        vec![Prop::Note("n".into()), Prop::Note(String::new())],
    ] {
        let changes = editor
            .apply(props(cell, &there_and_back), NOW, &mut ids)
            .unwrap();
        assert!(changes.is_empty(), "{there_and_back:?}: {changes:?}");
        assert_eq!(editor.undo_len(), 0, "{there_and_back:?}");
        assert_eq!(*editor.document(), before);
    }
    // A real change still stamps, once, and a later explicit time wins over the stamp.
    let changes = editor
        .apply(props(cell, &[Prop::CellColor(red)]), NOW, &mut ids)
        .unwrap();
    assert_eq!(changes.changed, [cell]);
    assert_eq!(editor.document().cell(cell).unwrap().last_edit, NOW);
    let both = [Prop::CellColor(Color::WHITE), Prop::LastEdit(Some(5))];
    editor.apply(props(cell, &both), NOW, &mut ids).unwrap();
    assert_eq!(editor.document().cell(cell).unwrap().last_edit, Some(5));
}

#[test]
fn two_cells_that_tag_one_name_back_to_what_it_was_change_nothing() {
    let (mut editor, mut ids, cells) = table(2, 1);
    let red = Color::rgb(255, 0, 0);
    for cell in [cells[0], cells[1]] {
        let text = Command::SetText {
            cell,
            text: "same".into(),
        };
        editor.apply(text, None, &mut ids).unwrap();
    }
    editor
        .apply(props(cells[0], &[Prop::CellColor(red)]), None, &mut ids)
        .unwrap();
    // The tag has the colours of the second cell already; the first would change it, the
    // second changes it back.
    let tag = Command::TagCells {
        cells: vec![cells[1]],
    };
    editor.apply(tag, None, &mut ids).unwrap();
    let units = editor.undo_len();
    let both = Command::TagCells {
        cells: vec![cells[0], cells[1]],
    };
    let changes = editor.apply(both, None, &mut ids).unwrap();
    assert!(changes.is_empty(), "{changes:?}");
    assert_eq!(editor.undo_len(), units);
}

#[test]
fn commands_that_ask_for_what_is_already_so_do_nothing() {
    let (mut editor, mut ids, cells) = table(2, 2);
    let root = editor.document().root();
    let sub = Command::CreateGrid {
        cell: cells[3],
        xs: 1,
        ys: 1,
    };
    editor.apply(sub, NOW, &mut ids).unwrap();
    editor.clear_history();
    let before = editor.document().clone();
    let nothing = [
        // The rows are in order already; one row is no sort.
        Command::SortRows {
            cell: root,
            column: 0,
            row: 0,
            count: 2,
            descending: false,
        },
        Command::SortRows {
            cell: root,
            column: 1,
            row: 1,
            count: 1,
            descending: true,
        },
        // A grid of one cell is its own transpose; a cell without a grid is passed over.
        Command::Transpose {
            cells: vec![cells[3], cells[0]],
        },
        Command::AdjustRelSize {
            cells: vec![root],
            delta: 0,
            recursive: true,
        },
        Command::AdjustColumnWidths {
            cell: root,
            column: 0,
            count: 2,
            delta: 0,
            recursive: true,
        },
        Command::AdjustColumnWidths {
            cell: root,
            column: 0,
            count: 0,
            delta: 5,
            recursive: false,
        },
        Command::SetColumnWidth {
            cell: root,
            column: 1,
            width: 80,
        },
        Command::MoveRows {
            cell: root,
            from: 1,
            count: 1,
            to: 1,
        },
        Command::MoveColumns {
            cell: root,
            from: 0,
            count: 2,
            to: 0,
        },
        Command::SwapCells {
            a: cells[1],
            b: cells[1],
        },
        Command::InsertText {
            cell: cells[0],
            at: 1,
            text: String::new(),
        },
        Command::DeleteText {
            cell: cells[0],
            start: 0,
            len: 0,
        },
        Command::ReplaceText {
            cell: cells[0],
            start: 0,
            len: 1,
            text: "0".into(),
        },
        Command::SetText {
            cell: cells[1],
            text: "1".into(),
        },
        Command::SetRangeStyle {
            cell: cells[0],
            start: 0,
            len: 1,
            set: Default::default(),
            clear: wombat_core::Style {
                bold: true,
                ..Default::default()
            },
        },
        Command::SetRangeColor {
            cell: cells[0],
            start: 0,
            len: 1,
            color: None,
        },
        Command::SetLink {
            cell: cells[0],
            start: 0,
            len: 1,
            target: None,
        },
        Command::SetStyle {
            cells: vec![cells[0], cells[0]],
            set: Default::default(),
            clear: Default::default(),
        },
        Command::InsertRows {
            cell: root,
            at: 0,
            count: 0,
        },
        Command::DeleteColumns {
            cell: root,
            at: 1,
            count: 0,
        },
        Command::TagCells { cells: vec![] },
        Command::UntagCells {
            cells: vec![cells[0]],
        },
        Command::ToggleCheck { cells: vec![] },
        Command::SetProps {
            cells: vec![cells[0], cells[3]],
            props: vec![],
        },
        Command::SetProps {
            cells: vec![cells[3]],
            props: vec![Prop::Vertical(true), Prop::OuterSpacing(3)],
        },
    ];
    for command in nothing {
        let changes = editor.apply(command.clone(), NOW, &mut ids).unwrap();
        assert!(changes.is_empty(), "{command:?}: {changes:?}");
        assert_eq!(*editor.document(), before, "{command:?}");
        assert_eq!(editor.undo_len(), 0, "{command:?}");
    }
    // Emptying cells that are empty changes nothing either.
    let clear = Command::ClearCells {
        cells: vec![cells[0], cells[2]],
    };
    editor.apply(clear.clone(), NOW, &mut ids).unwrap();
    let units = editor.undo_len();
    let again = editor.apply(clear, NOW, &mut ids).unwrap();
    assert!(again.is_empty() && editor.undo_len() == units);
}

// ---- 3: the root keeps its grid -------------------------------------------------------------

#[test]
fn no_command_takes_the_grid_of_the_root_away() {
    let (mut editor, mut ids, cells) = table(2, 2);
    let root = editor.document().root();
    let before = editor.document().clone();
    let refused = [
        Command::RemoveGrid { cell: root },
        Command::ClearCells { cells: vec![root] },
        Command::ClearCells {
            cells: vec![cells[0], root],
        },
        Command::CopyCell {
            from: cells[0],
            to: root,
        },
        Command::DeleteRows {
            cell: root,
            at: 0,
            count: 2,
        },
        Command::DeleteColumns {
            cell: root,
            at: 0,
            count: 2,
        },
    ];
    for command in refused {
        let result = editor.apply(command.clone(), None, &mut ids);
        assert!(matches!(result, Err(Error::Refused(_))), "{command:?}");
        assert_eq!(*editor.document(), before, "{command:?}");
    }
    // Deleting every cell empties them and leaves the grid, as in TreeSheets.
    let all = Command::DeleteCells {
        cell: root,
        x: 0,
        y: 0,
        xs: 2,
        ys: 2,
    };
    editor.apply(all, None, &mut ids).unwrap();
    let grid = editor.document().cell(root).unwrap().grid().unwrap();
    assert_eq!((grid.xs(), grid.ys()), (2, 2));
    assert!(cells
        .iter()
        .all(|c| editor.document().cell(*c).unwrap().text.is_empty()));
}

// ---- 5: an ID in the history is not free ----------------------------------------------------

/// Gives the same IDs again after `rewind`.
struct Replay {
    inner: SeededIds,
}

impl IdSource for Replay {
    fn next_id(&mut self) -> Id {
        self.inner.next_id()
    }
}

#[test]
fn the_id_of_a_cell_held_in_the_history_is_not_issued_again() {
    let (mut editor, mut ids, cells) = table(2, 1);
    let mut source = Replay {
        inner: SeededIds::from_u64(42),
    };
    let grid = |cell| Command::CreateGrid { cell, xs: 1, ys: 1 };
    editor.apply(grid(cells[0]), None, &mut source).unwrap();
    let made = editor.document().child(cells[0], 0, 0).unwrap();
    let remove = Command::RemoveGrid { cell: cells[0] };
    editor.apply(remove, None, &mut ids).unwrap();
    assert!(editor.document().cell(made).is_none());
    // The removed cell can come back by undo: its ID is taken, also for another place.
    source.inner = SeededIds::from_u64(42);
    let before = editor.document().clone();
    let again = editor.apply(grid(cells[1]), None, &mut source);
    assert!(matches!(again, Err(Error::Refused(_))), "{again:?}");
    assert_eq!(*editor.document(), before);
    // The same for a cell that redo can bring back.
    editor.undo().unwrap().unwrap();
    editor.undo().unwrap().unwrap();
    assert_eq!(editor.redo_len(), 2);
    source.inner = SeededIds::from_u64(42);
    // (A refused command leaves the redo history alone.)
    assert!(editor.apply(grid(cells[1]), None, &mut source).is_err());
    assert_eq!(editor.redo_len(), 2);
    // Once the history is forgotten the cell is gone for good and the ID is free.
    editor.clear_history();
    source.inner = SeededIds::from_u64(42);
    editor.apply(grid(cells[1]), None, &mut source).unwrap();
    assert_eq!(editor.document().child(cells[1], 0, 0), Some(made));
    // The same when the unit that holds the cell falls out of a bounded history, and for
    // a cell removed inside a group that is still open.
    editor.begin_group();
    let remove = Command::RemoveGrid { cell: cells[1] };
    editor.apply(remove, None, &mut ids).unwrap();
    source.inner = SeededIds::from_u64(42);
    assert!(editor.apply(grid(cells[0]), None, &mut source).is_err());
    editor.end_group();
    source.inner = SeededIds::from_u64(42);
    assert!(editor.apply(grid(cells[0]), None, &mut source).is_err());
    editor.set_history_limit(Some(0));
    source.inner = SeededIds::from_u64(42);
    editor.apply(grid(cells[0]), None, &mut source).unwrap();
    assert_eq!(editor.document().child(cells[0], 0, 0), Some(made));
}

// ---- 6: one form for a link to a cell of the same document ----------------------------------

#[test]
fn a_link_into_the_own_document_is_stored_without_the_document_id() {
    let (mut editor, mut ids, cells) = table(2, 1);
    let own = editor.document().id();
    let link = |doc| Command::SetLink {
        cell: cells[0],
        start: 0,
        len: 1,
        target: Some(LinkTarget::Cell {
            doc,
            cell: cells[1],
        }),
    };
    editor.apply(link(Some(own)), None, &mut ids).unwrap();
    let stored = LinkTarget::Cell {
        doc: None,
        cell: cells[1],
    };
    assert_eq!(
        editor.document().cell(cells[0]).unwrap().links[0].target,
        stored
    );
    // Both spellings are the same link: the second asks for nothing new.
    let changes = editor.apply(link(None), None, &mut ids).unwrap();
    assert!(changes.is_empty());
    assert_eq!(editor.undo_len(), 1);

    // A file that names the own document in such a link loads as the one form, and is
    // written back without the name; a link into another document keeps its name.
    let text = format::to_string(editor.document()).unwrap();
    let target = format!("\"cell\": \"{}\"", cells[1]);
    assert_eq!(text.matches(&target).count(), 1);
    let named = text.replace(&target, &format!("{target}, \"doc\": \"{own}\""));
    let loaded = format::from_slice(named.as_bytes()).unwrap();
    assert_eq!(loaded, *editor.document());
    assert_eq!(format::to_string(&loaded).unwrap(), text);
    let mut foreign = format::from_slice(text.as_bytes()).unwrap();
    foreign.cell_mut(cells[0]).unwrap().links[0].target = LinkTarget::Cell {
        doc: Some(wombat_core::DocId(Id::from_u128(5))),
        cell: cells[1],
    };
    let foreign_text = format::to_string(&foreign).unwrap();
    assert!(foreign_text.contains("\"doc\""));
    assert_eq!(
        format::from_slice(foreign_text.as_bytes()).unwrap(),
        foreign
    );
    // A document built with the own ID in a link is not valid, so no editor takes it.
    foreign.cell_mut(cells[0]).unwrap().links[0].target = LinkTarget::Cell {
        doc: Some(own),
        cell: cells[1],
    };
    assert!(matches!(
        Editor::new(foreign.clone()),
        Err(Error::Invalid(_))
    ));
    assert!(format::to_string(&foreign).is_err());
    // A copy with fresh IDs keeps the one form, pointing at the copy's cell.
    let (copy, fresh) = editor.document().with_fresh_ids(&mut ids).unwrap();
    let copied = &copy.cell(fresh.cell(cells[0]).unwrap()).unwrap().links[0];
    let expected = LinkTarget::Cell {
        doc: None,
        cell: fresh.cell(cells[1]).unwrap(),
    };
    assert_eq!(copied.target, expected);
}

// ---- 4: the cost of the history -------------------------------------------------------------

/// One cell with a text of `2 * runs` letters, every second one bold.
fn striped(runs: u32) -> (Editor, SeededIds, CellId) {
    let (mut editor, mut ids, cells) = table(1, 1);
    let cell = cells[0];
    let text = Command::SetText {
        cell,
        text: "ab".repeat(runs as usize),
    };
    editor.apply(text, None, &mut ids).unwrap();
    editor.begin_group();
    for n in 0..runs {
        let bold = Command::SetRangeStyle {
            cell,
            start: 2 * n,
            len: 1,
            set: wombat_core::Style {
                bold: true,
                ..Default::default()
            },
            clear: Default::default(),
        };
        editor.apply(bold, None, &mut ids).unwrap();
    }
    editor.end_group();
    assert_eq!(
        editor.document().cell(cell).unwrap().runs.len(),
        runs as usize
    );
    editor.clear_history();
    (editor, ids, cell)
}

fn insert(cell: CellId, at: u32, text: &str) -> Command {
    Command::InsertText {
        cell,
        at,
        text: text.into(),
    }
}

#[test]
fn a_keystroke_holds_only_the_runs_it_touches() {
    let (mut editor, mut ids, cell) = striped(2000);
    let before = editor.document().clone();
    assert_eq!(editor.history_bytes(), 0);
    // At the start (every run moves), inside a run, and at the end.
    for at in [0, 1, 4001] {
        editor.apply(insert(cell, at, "x"), NOW, &mut ids).unwrap();
    }
    let held = editor.history_bytes();
    println!("3 keystrokes in a cell with 2,000 runs hold about {held} bytes");
    assert!(held < 1500, "{held} bytes for three keystrokes");
    let typed = editor.document().clone();
    assert_eq!(typed.cell(cell).unwrap().runs[1999].start, 4000);
    for _ in 0..3 {
        editor.undo().unwrap().unwrap();
    }
    assert_eq!(*editor.document(), before);
    for _ in 0..3 {
        editor.redo().unwrap().unwrap();
    }
    assert_eq!(*editor.document(), typed);
}

#[test]
fn typing_joins_the_unit_before_it_when_the_caller_says_so() {
    let (mut editor, mut ids, cells) = table(2, 1);
    let (a, b) = (cells[0], cells[1]);
    let start = editor.document().clone();
    // Three keystrokes in one cell are one unit; `apply` starts it, `apply_typing` joins.
    editor.apply(insert(a, 1, "x"), NOW, &mut ids).unwrap();
    for (at, text) in [(2, "y"), (3, "z")] {
        let changes = editor
            .apply_typing(insert(a, at, text), NOW, &mut ids)
            .unwrap();
        assert_eq!(
            changes.changed,
            [a],
            "the report is that of the one command"
        );
    }
    let delete = Command::DeleteText {
        cell: a,
        start: 3,
        len: 1,
    };
    editor.apply_typing(delete, NOW, &mut ids).unwrap();
    assert_eq!(editor.undo_len(), 1);
    assert_eq!(editor.document().cell(a).unwrap().text, "0xy");
    // Another cell is another unit, and so is a command that is not typing, and typing
    // after it; a plain `apply` never joins.
    editor
        .apply_typing(insert(b, 0, "q"), NOW, &mut ids)
        .unwrap();
    assert_eq!(editor.undo_len(), 2);
    let red = props(b, &[Prop::CellColor(Color::rgb(255, 0, 0))]);
    editor.apply_typing(red, NOW, &mut ids).unwrap();
    editor
        .apply_typing(insert(b, 0, "r"), NOW, &mut ids)
        .unwrap();
    editor.apply(insert(b, 0, "s"), NOW, &mut ids).unwrap();
    assert_eq!(editor.undo_len(), 5);
    let end = editor.document().clone();
    // A group is never joined, and typing inside a group belongs to the group.
    editor.begin_group();
    editor
        .apply_typing(insert(b, 0, "t"), NOW, &mut ids)
        .unwrap();
    editor.end_group();
    editor
        .apply_typing(insert(b, 0, "u"), NOW, &mut ids)
        .unwrap();
    assert_eq!(editor.undo_len(), 7);
    editor.undo().unwrap();
    editor.undo().unwrap();
    assert_eq!(*editor.document(), end);
    // One undo takes back the whole run of typing, one redo brings it back, and typing
    // can go on in a unit that was undone and redone.
    for _ in 0..4 {
        editor.undo().unwrap().unwrap();
    }
    assert_eq!(editor.document().cell(a).unwrap().text, "0xy");
    let changes = editor.undo().unwrap().unwrap();
    assert_eq!(changes.changed, [a]);
    assert_eq!(*editor.document(), start);
    editor.redo().unwrap().unwrap();
    assert_eq!(editor.document().cell(a).unwrap().text, "0xy");
    editor
        .apply_typing(insert(a, 3, "!"), NOW, &mut ids)
        .unwrap();
    assert_eq!((editor.undo_len(), editor.redo_len()), (1, 0));
    editor.undo().unwrap().unwrap();
    assert_eq!(*editor.document(), start);
}

#[test]
fn the_history_is_bounded_by_default_in_units_and_in_memory() {
    use wombat_core::edit::{DEFAULT_HISTORY_BYTES, DEFAULT_HISTORY_UNITS};
    let (mut editor, mut ids, cells) = table(1, 1);
    let cell = cells[0];
    for n in 0..DEFAULT_HISTORY_UNITS + 10 {
        let text = Command::SetText {
            cell,
            text: n.to_string(),
        };
        editor.apply(text, None, &mut ids).unwrap();
    }
    assert_eq!(editor.undo_len(), DEFAULT_HISTORY_UNITS);
    assert!(editor.history_bytes() < DEFAULT_HISTORY_BYTES);
    // By memory: each of these units holds a text of 1,000 bytes.
    editor.set_history_budget(Some(10_000));
    let kept = editor.undo_len();
    assert!(editor.history_bytes() <= 10_000 && (10..DEFAULT_HISTORY_UNITS).contains(&kept));
    for n in 0..20 {
        let text = Command::SetText {
            cell,
            text: n.to_string().repeat(1000),
        };
        editor.apply(text, None, &mut ids).unwrap();
        assert!(
            editor.history_bytes() <= 10_000,
            "{}",
            editor.history_bytes()
        );
    }
    assert!(
        (3..10).contains(&editor.undo_len()),
        "{}",
        editor.undo_len()
    );
    // The newest unit stays even when it alone is over the budget, so that the last
    // command can always be undone.
    editor.set_history_budget(Some(10));
    assert_eq!(editor.undo_len(), 1);
    let before = editor.document().clone();
    let text = Command::SetText {
        cell,
        text: "short".into(),
    };
    editor.apply(text, None, &mut ids).unwrap();
    assert_eq!(editor.undo_len(), 1);
    editor.undo().unwrap().unwrap();
    assert_eq!(*editor.document(), before);
    // No bounds at all, and none of the history.
    editor.set_history_budget(None);
    editor.set_history_limit(None);
    for n in 0..DEFAULT_HISTORY_UNITS + 10 {
        let text = Command::SetText {
            cell,
            text: n.to_string(),
        };
        editor.apply(text, None, &mut ids).unwrap();
    }
    assert_eq!(editor.undo_len(), DEFAULT_HISTORY_UNITS + 10);
    editor.set_history_limit(Some(0));
    assert_eq!((editor.undo_len(), editor.history_bytes()), (0, 0));
}

// ---- 7: the report of a group ---------------------------------------------------------------

#[test]
fn a_group_reports_what_it_changed_as_a_whole() {
    let (mut editor, mut ids, cells) = table(2, 1);
    let (a, b) = (cells[0], cells[1]);
    let root = editor.document().root();
    assert!(editor.end_group().is_empty(), "no group is open");
    editor.begin_group();
    // A grid made and removed again: its cells never existed for the outside.
    let made = editor
        .apply(
            Command::CreateGrid {
                cell: a,
                xs: 2,
                ys: 2,
            },
            NOW,
            &mut ids,
        )
        .unwrap();
    assert_eq!(made.created.len(), 4);
    editor.begin_group();
    editor
        .apply(Command::RemoveGrid { cell: a }, NOW, &mut ids)
        .unwrap();
    assert!(
        editor.end_group().is_empty(),
        "an inner group reports nothing"
    );
    // Something that stays.
    editor.apply(insert(b, 0, "x"), NOW, &mut ids).unwrap();
    let row = Command::InsertRows {
        cell: root,
        at: 1,
        count: 1,
    };
    let added = editor.apply(row, NOW, &mut ids).unwrap().created;
    editor
        .apply(
            Command::SetTag {
                name: "t".into(),
                cell_color: Color::WHITE,
                text_color: Color::BLACK,
            },
            NOW,
            &mut ids,
        )
        .unwrap();
    let changes = editor.end_group();
    assert_eq!(changes.created, added);
    assert!(changes.removed.is_empty());
    assert!(changes.changed.contains(&b));
    // (The cell whose grid came and went may be named too: a group's report is complete,
    // not tight.)
    assert!(changes.grids.contains(&root));
    assert_eq!(changes.tags, ["t"]);
    assert_eq!(editor.undo_len(), 1);
    // A group in which nothing happened reports nothing and is no unit.
    editor.begin_group();
    assert!(editor.end_group().is_empty());
    assert_eq!(editor.undo_len(), 1);
}

/// The bound on the saved size is a sum of parts (`edit/size.rs`). Each part must be
/// counted at least as long as it is written: adding one thing to a document may not make
/// the file grow by more than the bound grows.
#[test]
fn every_part_of_a_document_is_counted_at_least_as_long_as_it_is_written() {
    use serde_json::json;
    use wombat_core::edit::size_bound;
    use wombat_core::{
        CellKind, DocId, DrawStyle, ImageRef, Link, Run, Style, Tag, TextAlign, VertAlign,
    };
    let mut ids = SeededIds::from_u64(8);
    let mut base = Document::new(&mut ids);
    base.add_grid(base.root(), 1, 1, &mut ids).unwrap();
    let cell = base.child(base.root(), 0, 0).unwrap();
    base.cell_mut(cell).unwrap().text = "abcdef".into();
    let other = DocId(Id::from_u128(9));
    let unknown = || json!({"k": [1, "a\"\n", null, {"x": -1.5e300}]});
    let all = Style {
        bold: true,
        italic: true,
        fixed: true,
        underline: true,
        strike: true,
    };
    let run = |start, style, color| Run {
        start,
        len: 1,
        style,
        color,
        extra: Default::default(),
    };
    let link = |start, target| Link {
        start,
        len: 1,
        target,
        extra: Default::default(),
    };
    type Change = Box<dyn Fn(&mut Document)>;
    let on_cell = |change: fn(&mut wombat_core::Cell)| -> Change {
        Box::new(move |doc: &mut Document| change(doc.cell_mut(cell).unwrap()))
    };
    let on_grid = |change: fn(&mut wombat_core::Grid)| -> Change {
        Box::new(move |doc: &mut Document| {
            let root = doc.root();
            change(doc.cell_mut(root).unwrap().grid_mut().unwrap());
        })
    };
    let mut changes: Vec<(&str, Change)> = vec![
        ("text", on_cell(|c| c.text = "ab\"cd\\ef \u{1F600}".into())),
        ("no text", on_cell(|c| c.text.clear())),
        ("kind", on_cell(|c| c.kind = CellKind::ALL[4])),
        ("colour", on_cell(|c| c.cell_color = Color::rgb(1, 2, 3))),
        (
            "text colour",
            on_cell(|c| c.text_color = Color::rgb(1, 2, 3)),
        ),
        ("draw style", on_cell(|c| c.draw_style = DrawStyle::ALL[1])),
        ("align", on_cell(|c| c.text_align = TextAlign::ALL[2])),
        (
            "vertical align",
            on_cell(|c| c.vert_align = VertAlign::ALL[3]),
        ),
        ("size", on_cell(|c| c.rel_size = i32::MIN)),
        (
            "note",
            on_cell(|c| c.note = "line\n\ttab \u{1} \"q\" \\ \u{7f}".into()),
        ),
        ("edited", on_cell(|c| c.last_edit = Some(i64::MIN))),
        (
            "unknown field",
            on_cell(|c| drop(c.extra.insert("z\"z".into(), json!([[]])))),
        ),
        ("border", on_grid(|g| g.border_color = Color::rgb(1, 2, 3))),
        ("spacing", on_grid(|g| g.outer_spacing = u32::MAX)),
        ("not vertical", on_grid(|g| g.vertical = false)),
        ("width", on_grid(|g| g.col_widths = vec![u32::MAX])),
        (
            "grid unknown field",
            on_grid(|g| drop(g.extra.insert("zz".into(), json!("v")))),
        ),
    ];
    for style in [
        all,
        Style {
            bold: true,
            ..Default::default()
        },
        Style {
            strike: true,
            ..Default::default()
        },
    ] {
        changes.push((
            "style",
            Box::new(move |doc| doc.cell_mut(cell).unwrap().style = style),
        ));
        changes.push((
            "runs",
            Box::new(move |doc| {
                let c = doc.cell_mut(cell).unwrap();
                c.runs = vec![
                    run(0, style, None),
                    run(2, style, Some(Color::BLACK)),
                    run(5, all, None),
                ];
                c.runs[1].extra.insert("zz".into(), unknown());
            }),
        ));
    }
    for target in [
        LinkTarget::External("https://example.org/\"x\"".into()),
        LinkTarget::Cell { doc: None, cell },
        LinkTarget::Cell {
            doc: Some(other),
            cell,
        },
        LinkTarget::Unknown,
    ] {
        changes.push((
            "links",
            Box::new(move |doc| {
                let c = doc.cell_mut(cell).unwrap();
                c.links = vec![link(1, target.clone()), link(4, target.clone())];
                c.links[0].extra.insert("zz".into(), unknown());
            }),
        ));
    }
    for scale in [1.0, 0.1, 1.0e-300, 123_456_789.123_456_79] {
        changes.push((
            "image",
            Box::new(move |doc| {
                let mut image = ImageRef::new("picture-1.png");
                image.scale = scale;
                image.extra.insert("zz".into(), unknown());
                doc.cell_mut(cell).unwrap().image = Some(image);
            }),
        ));
    }
    changes.push((
        "document unknown field",
        Box::new(move |doc| drop(doc.extra_mut().insert("zz".into(), unknown()))),
    ));
    for name in ["t", "tag \"two\" \\"] {
        changes.push((
            "tag",
            Box::new(move |doc| {
                let mut tag = Tag {
                    cell_color: Color::rgb(1, 2, 3),
                    text_color: Color::rgb(4, 5, 6),
                    extra: Default::default(),
                };
                tag.extra.insert("zz".into(), unknown());
                doc.tags_mut().insert(name.into(), tag.clone());
                doc.tags_mut().insert(format!("{name}2"), tag);
            }),
        ));
    }
    let size = |doc: &Document| format::to_string(doc).unwrap().len() as i64;
    let bound = |doc: &Document| size_bound(doc) as i64;
    assert!(size(&base) <= bound(&base));
    for (what, change) in &changes {
        let mut doc = base.clone();
        change(&mut doc);
        let (written, counted) = (size(&doc) - size(&base), bound(&doc) - bound(&base));
        assert!(
            written <= counted,
            "{what}: {written} bytes written, {counted} counted"
        );
    }
    // And a cell, at the deepest place there is: one more cell is at least `CELL` more.
    let mut deep = base.clone();
    let mut at = cell;
    for _ in 2..wombat_core::limits::MAX_DEPTH {
        deep.add_grid(at, 1, 1, &mut ids).unwrap();
        at = deep.child(at, 0, 0).unwrap();
    }
    let mut deeper = deep.clone();
    deeper.add_grid(at, 2, 2, &mut ids).unwrap();
    assert!(size(&deeper) - size(&deep) <= bound(&deeper) - bound(&deep));
    let mut wide = deep.clone();
    wide.add_grid(at, u32::try_from(1_000).unwrap(), 1, &mut ids)
        .unwrap();
    assert!(size(&wide) - size(&deep) <= bound(&wide) - bound(&deep));
}

/// At the real limit: the largest grid that is accepted saves and loads, one more row is
/// refused, and all of it can be undone.
#[test]
fn the_largest_grid_that_commands_make_can_be_saved() {
    use wombat_core::limits::MAX_FILE_BYTES;
    let mut ids = SeededIds::from_u64(1);
    let mut editor = Editor::new(Document::new(&mut ids)).unwrap();
    let root = editor.document().root();
    // The bound counts 81 bytes for an empty cell.
    let rows = u32::try_from((MAX_FILE_BYTES - editor.size_bound()) / 81 / 10 - 1).unwrap();
    let grid = Command::CreateGrid {
        cell: root,
        xs: 10,
        ys: rows,
    };
    editor.apply(grid, None, &mut ids).unwrap();
    assert!(editor.size_bound() <= MAX_FILE_BYTES);
    let row = Command::InsertRows {
        cell: root,
        at: 0,
        count: 2,
    };
    let refused = editor.apply(row, None, &mut ids);
    assert!(matches!(refused, Err(Error::Limit(_))), "{refused:?}");
    assert_eq!(editor.document().cell_count() as u32, rows * 10 + 1);
    // A long text in one of the cells is refused as well, a short one is not.
    let cell = editor.document().child(root, 0, 0).unwrap();
    let long = Command::SetText {
        cell,
        text: "x".repeat(2000),
    };
    assert!(matches!(
        editor.apply(long, None, &mut ids),
        Err(Error::Limit(_))
    ));
    editor.apply(insert(cell, 0, "x"), None, &mut ids).unwrap();
    let text = format::to_string(editor.document()).unwrap();
    println!(
        "{} cells, {} bytes saved, bound {}",
        rows * 10 + 1,
        text.len(),
        editor.size_bound()
    );
    assert_eq!(
        format::from_slice(text.as_bytes()).unwrap(),
        *editor.document()
    );
    editor.undo().unwrap().unwrap();
    editor.undo().unwrap().unwrap();
    assert_eq!(editor.document().cell_count(), 1);
}
