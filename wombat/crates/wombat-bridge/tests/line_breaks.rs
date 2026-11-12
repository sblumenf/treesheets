//! Line breaks in cell text through an open document: a text of several lines goes in by
//! a command and by an editor's text, comes out of every read, and is in the file; and
//! the line ends of other systems arrive as one line feed.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use common::{ids, open_example};
use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{Command, Report, StageAsk};

fn ask() -> StageAsk {
    StageAsk {
        x0: 0,
        y0: 0,
        cols: 100,
        rows: 100,
        full_depth: 1,
        full_cells: 24,
        preview_cols: 3,
        preview_rows: 4,
        shape_cols: 12,
        shape_rows: 20,
        text_cut: 2000,
        preview_cut: 40,
    }
}

#[test]
fn a_text_of_several_lines_survives_every_way_in_and_out() {
    let (mut doc, path) = open_example("line-breaks", "survives");
    let root = doc.root().unwrap();
    let grid = doc.grid(root.clone()).unwrap();
    // "what to buy", and "Costs", which has a grid: its text is in the trail one level
    // down, and its cells are previewed in the root.
    let (note, costs) = (grid.cells[1].id.clone(), grid.cells[2].id.clone());
    let first_cost = doc.grid(costs.clone()).unwrap().cells[0].id.clone();
    let text = |doc: &OpenDocument, cell: &str| doc.cell(cell.into()).unwrap().text;
    let set = |cell: &str, text: &str| Command::SetText {
        cell: cell.into(),
        text: text.into(),
    };

    // By a command. The line ends of Windows and of old Macs become one line feed each,
    // so the text in the cell is shorter than what was sent; a tab is still a space.
    doc.apply(set(&note, "one\r\ntwo\rthree\nfour\tfive"))
        .unwrap();
    assert_eq!(text(&doc, &note), "one\ntwo\nthree\nfour five");
    doc.apply(Command::EnterText {
        cell: costs.clone(),
        text: "Costs\r\nof it".into(),
    })
    .unwrap();
    assert_eq!(text(&doc, &costs), "Costs\nof it");
    doc.apply(set(&first_cost, "3\n")).unwrap();

    // By the text of an editor: only what differs is replaced, and what differs is
    // found in the text as a cell holds it, not as it was sent.
    let report = doc
        .retype(
            note.clone(),
            "one\r\ntwo\r\nthree\r\nfour five\r\nsix".into(),
        )
        .unwrap();
    assert_eq!(report.changed, std::slice::from_ref(&note));
    assert_eq!(text(&doc, &note), "one\ntwo\nthree\nfour five\nsix");
    let same = doc.retype(note.clone(), "one\ntwo\rthree\r\nfour five\nsix".into());
    assert_eq!(same.unwrap(), Report::default());
    // One step of the undo history each, and the line breaks come back with it.
    doc.undo().unwrap().unwrap();
    assert_eq!(text(&doc, &note), "one\ntwo\nthree\nfour five");
    doc.redo().unwrap().unwrap();

    let check = |doc: &OpenDocument| {
        assert_eq!(text(doc, &note), "one\ntwo\nthree\nfour five\nsix");
        assert_eq!(text(doc, &costs), "Costs\nof it");
        // The stage gives a cell's text with its lines.
        let stage = doc.stage(root.clone(), ask()).unwrap();
        assert_eq!(stage.cells[1].text, "one\ntwo\nthree\nfour five\nsix");
        assert_eq!(stage.cells[2].text, "Costs\nof it");
        assert_eq!(
            doc.grid(root.clone()).unwrap().cells[2].text,
            "Costs\nof it"
        );
        // What is shown on one line has a space where the line broke: a preview and
        // the trail.
        let summary = &stage.summaries[stage.cells[2].summary.unwrap() as usize];
        assert_eq!(summary.preview[0].text, "3 ");
        let below = doc.stage(costs.clone(), ask()).unwrap();
        assert_eq!(below.trail[1].shown.text, "Costs of it");
        assert_eq!(below.cells[0].text, "3\n");
    };
    check(&doc);
    doc.save().unwrap();
    check(&doc);
    doc.close().unwrap();
    check(&OpenDocument::open_with(&path, ids()).unwrap());
}
