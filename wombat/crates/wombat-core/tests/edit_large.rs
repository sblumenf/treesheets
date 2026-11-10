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
