//! Editing a large document: one edit costs what it touches, not what the document holds.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::time::{Duration, Instant};

use wombat_core::construct::Construct;
use wombat_core::{Changes, Command, Document, Editor, SeededIds};

/// 10 x 100 cells, each holding a 10 x 10 grid: 101,001 cells, three levels.
fn large(ids: &mut SeededIds) -> Document {
    let mut doc = Document::new(ids);
    let root = doc.root();
    doc.add_grid(root, 10, 100, ids).unwrap();
    let outer = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
    for (n, cell) in outer.iter().enumerate() {
        doc.cell_mut(*cell).unwrap().text = format!("Section {n}");
        doc.add_grid(*cell, 10, 10, ids).unwrap();
        let inner = doc.cell(*cell).unwrap().grid().unwrap().cells().to_vec();
        for (m, leaf) in inner.iter().enumerate() {
            doc.cell_mut(*leaf).unwrap().text = format!("Item {n}.{m} with some ordinary text");
        }
    }
    doc
}

fn size(changes: &Changes) -> usize {
    changes.changed.len() + changes.created.len() + changes.removed.len() + changes.grids.len()
}

/// The best of several runs of a command followed by its undo: (command, undo, report).
fn measure(
    editor: &mut Editor,
    ids: &mut SeededIds,
    command: &Command,
) -> (Duration, Duration, Changes) {
    let mut best = (Duration::MAX, Duration::MAX);
    let mut report = Changes::default();
    for _ in 0..20 {
        let start = Instant::now();
        report = editor.apply(command.clone(), Some(1), ids).unwrap();
        let applied = start.elapsed();
        let start = Instant::now();
        let undone = editor.undo().unwrap().unwrap();
        let undo = start.elapsed();
        assert_eq!(size(&undone), size(&report));
        best = (best.0.min(applied), best.1.min(undo));
    }
    (best.0, best.1, report)
}

#[test]
fn edits_of_a_hundred_thousand_cell_document_are_quick_and_report_little() {
    let mut ids = SeededIds::from_u64(100);
    let doc = large(&mut ids);
    assert_eq!(doc.cell_count(), 101_001);
    let original = doc.clone();
    let root = doc.root();
    let section = doc.child(root, 5, 50).unwrap();
    let leaf = doc.child(section, 5, 5).unwrap();
    let mut editor = Editor::new(doc).unwrap();
    // (what, command, cells in the report, limit)
    let cases = [
        (
            "insert text in one cell",
            Command::InsertText {
                cell: leaf,
                at: 4,
                text: "new ".into(),
            },
            1,
        ),
        (
            "delete text in one cell",
            Command::DeleteText {
                cell: leaf,
                start: 0,
                len: 5,
            },
            1,
        ),
        (
            "style a range in one cell",
            Command::SetRangeStyle {
                cell: leaf,
                start: 0,
                len: 4,
                set: wombat_core::Style {
                    bold: true,
                    ..Default::default()
                },
                clear: Default::default(),
            },
            1,
        ),
        (
            "insert a row in a 10 x 10 grid",
            Command::InsertRows {
                cell: section,
                at: 5,
                count: 1,
            },
            11,
        ),
        (
            "insert a row in the 10 x 100 root grid",
            Command::InsertRows {
                cell: root,
                at: 50,
                count: 1,
            },
            11,
        ),
        (
            "insert a column in the 10 x 100 root grid",
            Command::InsertColumns {
                cell: root,
                at: 5,
                count: 1,
            },
            102,
        ),
        (
            "delete a row of the root grid (1,010 cells)",
            Command::DeleteRows {
                cell: root,
                at: 50,
                count: 1,
            },
            1011,
        ),
        (
            "move a row of the root grid",
            Command::MoveRows {
                cell: root,
                from: 0,
                count: 1,
                to: 99,
            },
            1,
        ),
        (
            "swap two cells of different grids",
            Command::SwapCells {
                a: leaf,
                b: doc_child(&original, 0, 0),
            },
            2,
        ),
    ];
    // Unoptimised and on a slow machine these stay far below the limit; a command that
    // walked the whole document (over 100,000 cells) would not: validating it, the
    // cheapest full walk there is, takes several times the limit.
    let limit = Duration::from_millis(if cfg!(debug_assertions) { 20 } else { 5 });
    for (what, command, cells) in cases {
        let (applied, undone, report) = measure(&mut editor, &mut ids, &command);
        println!(
            "large document: {what}: {applied:?}, undo {undone:?}, {} cells in the report",
            size(&report)
        );
        assert_eq!(size(&report), cells, "{what}: size of the report");
        assert!(
            applied < limit && undone < limit,
            "{what}: {applied:?} / {undone:?}, over {limit:?}"
        );
    }
    let start = Instant::now();
    editor.document().validate().unwrap();
    println!(
        "large document: for comparison, checking the whole document: {:?}",
        start.elapsed()
    );
    assert_eq!(*editor.document(), original);
}

fn doc_child(doc: &Document, x: u32, y: u32) -> wombat_core::CellId {
    let section = doc.child(doc.root(), x, y).unwrap();
    doc.child(section, 0, 0).unwrap()
}

/// A grid of three columns and 10,000 rows: a group, an item with a number that does not
/// sort like text, a value. 50 groups of 200 items.
fn long_table(ids: &mut SeededIds) -> Document {
    let mut doc = Document::new(ids);
    let root = doc.root();
    doc.add_grid(root, 3, 10_000, ids).unwrap();
    let cells = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
    for (row, cells) in cells.chunks(3).enumerate() {
        // Scattered, so that sorting has work to do.
        let n = row * 7919 % 10_000;
        let texts = [
            format!("Group {}", n % 50),
            format!("item {n}"),
            format!("{}", n * 31 % 977),
        ];
        for (cell, text) in cells.iter().zip(texts) {
            doc.cell_mut(*cell).unwrap().text = text;
        }
    }
    doc
}

/// Restructuring a large grid: the time is that of the grid restructured, and is recorded.
#[test]
fn restructuring_ten_thousand_rows_is_quick() {
    let mut ids = SeededIds::from_u64(200);
    let doc = long_table(&mut ids);
    assert_eq!(doc.cell_count(), 30_001);
    let original = doc.clone();
    let root = doc.root();
    let mut editor = Editor::new(doc).unwrap();
    let limit = Duration::from_millis(if cfg!(debug_assertions) { 3000 } else { 500 });
    let mut run = |editor: &mut Editor, what: &str, command: Command| {
        let start = Instant::now();
        let report = editor.apply(command, Some(1), &mut ids).unwrap();
        let applied = start.elapsed();
        println!(
            "10,000 rows: {what}: {applied:?}, {} cells in the report, {} cells now",
            size(&report),
            editor.document().cell_count()
        );
        assert!(applied < limit, "{what}: {applied:?}, over {limit:?}");
    };
    let sort = |column, descending| Command::SortRows {
        cell: root,
        column,
        row: 0,
        count: 10_000,
        descending,
    };
    run(
        &mut editor,
        "sort by a column of 50 different texts",
        sort(0, false),
    );
    run(
        &mut editor,
        "sort by a column of 10,000 different texts",
        sort(1, true),
    );
    run(
        &mut editor,
        "transpose",
        Command::Transpose { cells: vec![root] },
    );
    run(
        &mut editor,
        "transpose back",
        Command::Transpose { cells: vec![root] },
    );
    run(
        &mut editor,
        "hierarchify",
        Command::Hierarchify { cell: root },
    );
    assert_eq!(
        editor.document().cell(root).unwrap().grid().unwrap().ys(),
        50
    );
    run(&mut editor, "flatten", Command::Flatten { cell: root });
    assert_eq!(
        editor.document().cell(root).unwrap().grid().unwrap().ys(),
        10_000
    );
    assert_eq!(editor.document().cell_count(), 30_001);
    run(
        &mut editor,
        "wrap 9,000 rows",
        Command::Wrap {
            cell: root,
            x: 0,
            y: 500,
            xs: 3,
            ys: 9000,
        },
    );
    editor.document().validate().unwrap();
    let start = Instant::now();
    let mut undone = 0;
    while editor.undo().unwrap().is_some() {
        undone += 1;
    }
    println!("10,000 rows: undo of all {undone}: {:?}", start.elapsed());
    assert_eq!(*editor.document(), original);
}
