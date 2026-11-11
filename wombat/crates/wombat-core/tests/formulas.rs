//! Formulas and programs: the rules that the comparison with TreeSheets
//! (`formulas_referee.rs`) does not show, such as limits, what recalculates when, how a
//! cell becomes a formula cell, and how a run goes through the editor.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod formula_support;

use formula_support::{build, editor, inside, row, shown};
use serde_json::json;
use wombat_core::eval::{self, Calc, FailureKind};
use wombat_core::{format, CellKind, Command, Editor, Error, Prop, SeededIds};

fn enter(editor: &mut Editor, ids: &mut SeededIds, cell: wombat_core::CellId, text: &str) {
    let text = text.to_string();
    editor
        .apply(Command::EnterText { cell, text }, None, ids)
        .unwrap();
}

#[test]
fn a_changed_number_recalculates_the_formulas_that_read_it_and_only_those() {
    let (mut e, mut ids) = editor(&[
        json!(["Costs", "10", "20"]),
        json!(["Prices", "1", "2"]),
        json!("=SUM(Costs)"),
        json!("=SUM(Prices)"),
        json!(["Totals", "=SUM(Costs)*2"]),
        json!("=SUM(Totals)+1"),
    ]);
    assert_eq!(shown(&e)[2..], ["30", "3", "Totals", "61"]);
    let cell = inside(&e, 0, 0, 1);
    let changes = e
        .apply(
            Command::SetText {
                cell,
                text: "25".into(),
            },
            None,
            &mut ids,
        )
        .unwrap();
    assert_eq!(changes.changed, [cell]);
    assert_eq!(shown(&e)[2..], ["35", "3", "Totals", "71"]);
    // The sum, the formula in Totals, and the formula that reads Totals: not SUM(Prices).
    let sorted = [row(&e, 2), inside(&e, 4, 0, 0), row(&e, 5)];
    assert_eq!(e.recalculated(), sorted);
    let stats = e.calc().stats();
    assert!(!stats.full);
    assert_eq!(stats.evaluated, 3);
    // Undo and redo recalculate too.
    e.undo().unwrap();
    assert_eq!(shown(&e)[2..], ["30", "3", "Totals", "61"]);
    assert_eq!(e.recalculated(), sorted);
    e.redo().unwrap();
    assert_eq!(shown(&e)[5], "71");
    // A change that no formula reads recalculates nothing; nor does a new colour.
    let lonely = inside(&e, 1, 0, 0);
    e.apply(
        Command::SetText {
            cell: lonely,
            text: "1".into(),
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert!(e.recalculated().is_empty());
    let red = Prop::CellColor(wombat_core::Color::rgb(255, 0, 0));
    e.apply(
        Command::SetProps {
            cells: vec![cell],
            props: vec![red],
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert!(e.recalculated().is_empty());
    assert!(!e.calc().stats().full);
}

#[test]
fn a_change_of_structure_or_of_a_name_recalculates_everything() {
    let (mut e, mut ids) = editor(&[
        json!(["Costs", "10", "20"]),
        json!(["Other", "1"]),
        json!("=SUM(Costs)"),
    ]);
    // Renaming the table: the name no longer means anything.
    let table = row(&e, 0);
    e.apply(
        Command::SetText {
            cell: table,
            text: "Kosten".into(),
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert!(e.calc().stats().full);
    assert_eq!(shown(&e)[2], "#NAME?");
    assert_eq!(e.recalculated(), [row(&e, 2)]);
    // Renaming another table to the name: nearest wins, here the first in the column.
    let other = row(&e, 1);
    e.apply(
        Command::SetText {
            cell: other,
            text: "Costs".into(),
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(shown(&e)[2], "1");
    // A new row in the table.
    e.apply(
        Command::InsertRows {
            cell: other,
            at: 1,
            count: 1,
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert!(e.calc().stats().full);
    let new = inside(&e, 1, 0, 1);
    e.apply(
        Command::SetText {
            cell: new,
            text: "5".into(),
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert!(!e.calc().stats().full);
    assert_eq!(shown(&e)[2], "6");
    // A colour on the table cell changes no name: nothing is worked out.
    let red = Prop::CellColor(wombat_core::Color::rgb(255, 0, 0));
    e.apply(
        Command::SetProps {
            cells: vec![other],
            props: vec![red],
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(e.calc().stats().evaluated, 0);
}

#[test]
fn entering_text_makes_and_unmakes_formula_cells() {
    let (mut e, mut ids) = editor(&[json!(["A", "1", "2"]), json!("x"), json!("y")]);
    let cell = row(&e, 1);
    enter(&mut e, &mut ids, cell, "=SUM(A)");
    assert_eq!(e.document().cell(cell).unwrap().kind, CellKind::Formula);
    assert_eq!(e.document().cell(cell).unwrap().text, "=SUM(A)");
    assert_eq!(e.shown(cell), Some("3"));
    assert_eq!(e.calc().result(cell), Some(("3", false)));
    // One unit of undo: text and kind together.
    e.undo().unwrap();
    assert_eq!(e.document().cell(cell).unwrap().kind, CellKind::Data);
    assert_eq!(e.shown(cell), Some("x"));
    e.redo().unwrap();
    assert_eq!(e.shown(cell), Some("3"));
    // Another formula keeps it a formula; a plain text makes it data; "=" alone is text.
    enter(&mut e, &mut ids, cell, "=MAX(A)");
    assert_eq!(e.shown(cell), Some("2"));
    enter(&mut e, &mut ids, cell, "plain");
    assert_eq!(e.document().cell(cell).unwrap().kind, CellKind::Data);
    assert_eq!(e.calc().result(cell), None);
    enter(&mut e, &mut ids, cell, "=");
    assert_eq!(e.document().cell(cell).unwrap().kind, CellKind::Data);
    // Cells of other kinds keep their kind.
    let other = row(&e, 2);
    let code = Prop::Kind(CellKind::Code);
    e.apply(
        Command::SetProps {
            cells: vec![other],
            props: vec![code],
        },
        None,
        &mut ids,
    )
    .unwrap();
    enter(&mut e, &mut ids, other, "=1+1");
    assert_eq!(e.document().cell(other).unwrap().kind, CellKind::Code);
    assert_eq!(e.shown(other), Some("=1+1"));
}

#[test]
fn a_cell_marked_as_a_formula_is_read_with_an_equals_sign_in_front() {
    let (mut e, mut ids) = editor(&[json!(["A", "1", "2"]), json!("SUM(A)*2"), json!("")]);
    let (cell, empty) = (row(&e, 1), row(&e, 2));
    let formula = Prop::Kind(CellKind::Formula);
    e.apply(
        Command::SetProps {
            cells: vec![cell, empty],
            props: vec![formula],
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(e.shown(cell), Some("6"));
    assert_eq!(e.document().cell(cell).unwrap().text, "SUM(A)*2");
    assert_eq!(e.shown(empty), Some("#ERROR!"));
    // Text commands edit the formula and leave the kind alone.
    e.apply(
        Command::InsertText {
            cell,
            at: 0,
            text: "1+".into(),
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(e.shown(cell), Some("7"));
    // Marked as data again, the cell shows its text: the formula, not the last result.
    let data = Prop::Kind(CellKind::Data);
    e.apply(
        Command::SetProps {
            cells: vec![cell],
            props: vec![data],
        },
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(e.shown(cell), Some("1+SUM(A)*2"));
    assert_eq!(e.calc().results().len(), 1);
}

#[test]
fn formulas_in_a_circle_say_so_and_recover() {
    let (mut e, mut ids) = editor(&[
        json!(["A", "=SUM(B)", "1"]),
        json!(["B", "=SUM(A)", "2"]),
        json!("=SUM(A)"),
    ]);
    assert_eq!(e.shown(inside(&e, 0, 0, 0)), Some("#CYCLE!"));
    assert_eq!(e.shown(inside(&e, 1, 0, 0)), Some("#CYCLE!"));
    assert_eq!(shown(&e)[2], "#CYCLE!");
    assert_eq!(e.calc().result(row(&e, 2)), Some(("#CYCLE!", true)));
    // Breaking the circle; with a circle about, everything is worked out again.
    let cell = inside(&e, 1, 0, 0);
    enter(&mut e, &mut ids, cell, "5");
    assert!(e.calc().stats().full);
    assert_eq!(shown(&e)[2], "8");
    // And small edits are small again.
    let number = inside(&e, 0, 0, 1);
    enter(&mut e, &mut ids, number, "10");
    assert!(!e.calc().stats().full);
    assert_eq!(shown(&e)[2], "17");
    // A formula that reads its own table leaves itself out.
    let own = inside(&e, 0, 0, 1);
    enter(&mut e, &mut ids, own, "=SUM(A)");
    assert_eq!(e.shown(own), Some("7"));
}

#[test]
fn a_long_chain_of_formulas_needs_no_stack() {
    // Each table holds a formula that reads the next one; the first formula in document
    // order therefore needs all 20,000 others first.
    let n = 20_000;
    let mut items = Vec::new();
    for i in 0..n {
        items.push(json!([format!("C{i}"), format!("=SUM(C{})+1", i + 1)]));
    }
    items.push(json!([format!("C{n}"), "1"]));
    let (mut e, mut ids) = editor(&items);
    assert_eq!(e.shown(inside(&e, 0, 0, 0)), Some("20001"));
    // A change at the far end goes through all of them, without working anything else out.
    let last = inside(&e, n, 0, 0);
    enter(&mut e, &mut ids, last, "2");
    assert_eq!(e.shown(inside(&e, 0, 0, 0)), Some("20002"));
    assert_eq!(
        e.calc().stats(),
        eval::Stats {
            full: false,
            evaluated: n as usize
        }
    );
    assert_eq!(e.recalculated().len(), n as usize);
}

#[test]
fn nesting_and_length_of_a_formula_are_bounded() {
    let nested = |n: usize| format!("={}1{}", "(".repeat(n), ")".repeat(n));
    let long = |n: usize| format!("={}", vec!["1"; n / 2].join("+"));
    let (e, _) = editor(&[
        json!(nested(eval::MAX_NESTING as usize - 1)),
        json!(nested(eval::MAX_NESTING as usize)),
        json!(nested(100_000)),
        json!(format!("={}5", "-".repeat(100_001))),
        json!(long(eval::MAX_FORMULA_LEN)),
        json!(long(eval::MAX_FORMULA_LEN + 2)),
        json!(format!("={}'a'{}", "LEN(".repeat(100), ")".repeat(100))),
        json!(format!(
            "=SUM({}A,0,0{})",
            "SUBSET(".repeat(100),
            ",0,0)".repeat(100)
        )),
    ]);
    assert_eq!(
        shown(&e),
        ["1", "#ERROR!", "#ERROR!", "#ERROR!", "2048", "#ERROR!", "#ERROR!", "#ERROR!"]
    );
}

#[test]
fn the_work_of_a_recalculation_is_bounded() {
    let mut items = vec![json!(["A", "1", "2", "3", "4", "5", "6", "7", "8"])];
    for _ in 0..10 {
        items.push(json!("=SUM(A)"));
    }
    let mut ids = SeededIds::from_u64(1);
    let doc = build(&formula_support::column(&items), &mut ids);
    let mut calc = Calc::new(&doc);
    assert_eq!(calc.results().values().filter(|r| **r == "36").count(), 10);
    // Each formula reads 8 cells: with room for 30, three are worked out, the fourth
    // goes over, and from there on nothing is.
    calc.set_budget(&doc, 30);
    let results: Vec<&str> = {
        let grid = doc.cell(doc.root()).unwrap().grid().unwrap();
        grid.cells()[1..]
            .iter()
            .map(|id| calc.result(*id).unwrap().0)
            .collect()
    };
    assert_eq!(results[..3], ["36", "36", "36"]);
    assert!(results[3..].iter().all(|r| *r == "#LIMIT!"), "{results:?}");
    // An editor in that state works everything out after every change, to the same end.
    let mut e = Editor::new(doc).unwrap();
    let cell = inside(&e, 0, 0, 0);
    enter(&mut e, &mut ids, cell, "2");
    assert_eq!(shown(&e)[1], "37");
}

#[test]
fn sorting_goes_by_what_formula_cells_show() {
    let rows = json!([
        ["=1+1", 6],
        ["b", 0],
        ["10", 0],
        ["c", 0],
        ["='apple'", 6],
        ["a", 0],
        ["=Missing", 6],
        ["d", 0]
    ]);
    let spec = json!(["", 0, 1, 1, [["T", 0, 2, 4, rows]]]);
    let mut ids = SeededIds::from_u64(1);
    let mut e = Editor::new(build(&spec, &mut ids)).unwrap();
    let table = row(&e, 0);
    e.apply(
        Command::SortRows {
            cell: table,
            column: 0,
            row: 0,
            count: 4,
            descending: false,
        },
        None,
        &mut ids,
    )
    .unwrap();
    let grid = e.document().cell(table).unwrap().grid().unwrap();
    let texts: Vec<&str> = grid.cells().iter().map(|c| e.shown(*c).unwrap()).collect();
    // By "#NAME?", "10", "2", "apple": not by the formulas, which all start with "=".
    assert_eq!(texts, ["#NAME?", "d", "10", "c", "2", "b", "apple", "a"]);
}

#[test]
fn a_run_fills_the_views_as_one_unit_of_undo() {
    let spec = json!([
        "",
        0,
        4,
        3,
        [
            ["2", 0],
            ["+", 1],
            ["3", 0],
            ["", 3],
            ["5", 0],
            ["x", 2],
            ["=x*2", 6],
            ["", 0],
            ["", 0, 2, 1, [["1", 0], ["2", 0]]],
            ["", 3],
            ["old", 3, 1, 2, [["a", 0], ["b", 0]]],
            ["nope", 1]
        ]
    ]);
    let mut ids = SeededIds::from_u64(1);
    let mut e = Editor::new(build(&spec, &mut ids)).unwrap();
    let doc = e.document();
    let at = |e: &Editor, x, y| e.document().child(e.document().root(), x, y).unwrap();
    let (view, formula, grid_view, old_view) =
        (at(&e, 3, 0), at(&e, 2, 1), at(&e, 1, 2), at(&e, 2, 2));
    let old_inside = doc.cell(old_view).unwrap().grid().unwrap().cells().to_vec();
    // Until the program has run, a variable is not there.
    assert_eq!(e.shown(formula), Some("#N/A"));
    let before = format::to_string(doc).unwrap();
    let changes = e.apply(Command::Run, None, &mut ids).unwrap();
    e.document().validate().unwrap();
    // The views keep their IDs and kinds; what they held is gone, what they hold is new.
    assert_eq!(e.shown(view), Some("5"));
    assert_eq!(e.document().cell(view).unwrap().kind, CellKind::ViewH);
    let filled = e.document().cell(grid_view).unwrap().grid().unwrap();
    assert_eq!((filled.xs(), filled.ys()), (2, 1));
    let texts: Vec<&str> = filled
        .cells()
        .iter()
        .map(|c| e.shown(*c).unwrap())
        .collect();
    assert_eq!(texts, ["1", "2"]);
    assert_eq!(e.shown(old_view), Some(""));
    assert!(e.document().cell(old_view).unwrap().grid().is_some());
    assert_eq!(changes.removed, {
        let mut gone = old_inside.clone();
        gone.sort();
        gone
    });
    assert_eq!(changes.created.len(), 4);
    assert!(changes.changed.contains(&view) && changes.grids.contains(&grid_view));
    // The variable is there for the formula now, and the report says what recalculated.
    assert_eq!(e.shown(formula), Some("10"));
    assert_eq!(e.recalculated(), [formula]);
    // The first failure: the unknown operation in the last row.
    let failure = e.calc().last_run().unwrap().failure.clone().unwrap();
    assert_eq!(failure.kind, FailureKind::UnknownOperation);
    assert_eq!(failure.name, "nope");
    assert_eq!(failure.cell, Some(at(&e, 3, 2)));
    // One undo gives the bytes back; the variables stay, as in TreeSheets.
    assert_eq!(e.undo_len(), 1);
    let after = format::to_string(e.document()).unwrap();
    e.undo().unwrap();
    assert_eq!(format::to_string(e.document()).unwrap(), before);
    assert_eq!(e.shown(formula), Some("10"));
    e.redo().unwrap();
    assert_eq!(format::to_string(e.document()).unwrap(), after);
    // A second run changes nothing, so it is nothing: no unit of undo, no new IDs.
    let changes = e.apply(Command::Run, None, &mut ids).unwrap();
    assert!(changes.is_empty());
    assert_eq!(e.undo_len(), 1);
    assert_eq!(format::to_string(e.document()).unwrap(), after);
}

#[test]
fn a_run_that_only_assigns_variables_still_recalculates() {
    let spec = json!(["", 0, 3, 1, [["5", 0], ["x", 2], ["=x+1", 6]]]);
    let mut ids = SeededIds::from_u64(1);
    let mut e = Editor::new(build(&spec, &mut ids)).unwrap();
    let formula = e.document().child(e.document().root(), 2, 0).unwrap();
    assert_eq!(e.shown(formula), Some("#N/A"));
    let changes = e.apply(Command::Run, None, &mut ids).unwrap();
    assert!(changes.is_empty());
    assert_eq!(e.undo_len(), 0);
    assert_eq!(e.shown(formula), Some("6"));
    assert_eq!(e.recalculated(), [formula]);
    assert_eq!(e.calc().variables().get("x").unwrap().text(), "5");
    // A document loaded again has no variables.
    assert_eq!(
        Calc::new(e.document()).result(formula),
        Some(("#N/A", true))
    );
}

#[test]
fn a_run_that_would_go_too_deep_or_take_too_long_is_refused() {
    // Each level is run three times by the level above it (twice for its two directions,
    // once more because it is all data): 3^24 is too much work.
    let mut spec = json!(["1", 0]);
    for _ in 0..24 {
        spec = json!(["", 0, 2, 2, [spec, ["2", 0], ["3", 0], ["4", 0]]]);
    }
    let mut ids = SeededIds::from_u64(1);
    let mut e = Editor::new(build(&spec, &mut ids)).unwrap();
    let before = format::to_string(e.document()).unwrap();
    let error = e.apply(Command::Run, None, &mut ids).unwrap_err();
    assert!(matches!(error, Error::Limit(_)), "{error:?}");
    assert_eq!(format::to_string(e.document()).unwrap(), before);
    assert_eq!(e.undo_len(), 0);
    // A view that would nest grids deeper than a document may: refused as well. The value
    // is 20 deep and assigned to a variable at the top; the view is 15 deep.
    let mut deep = json!(["x", 0]);
    for _ in 0..20 {
        deep = json!(["", 0, 1, 1, [deep]]);
    }
    let mut reader = json!(["", 0, 2, 1, [["v", 4], ["", 3]]]);
    for _ in 0..14 {
        reader = json!(["", 0, 1, 1, [reader]]);
    }
    let spec = json!(["", 0, 2, 2, [deep, ["v", 2], reader, ["", 0]]]);
    let mut e = Editor::new(build(&spec, &mut ids)).unwrap();
    let before = format::to_string(e.document()).unwrap();
    let error = e.apply(Command::Run, None, &mut ids).unwrap_err();
    assert!(matches!(error, Error::Limit(_)), "{error:?}");
    assert_eq!(format::to_string(e.document()).unwrap(), before);
    assert!(e.calc().variables().is_empty() && e.calc().last_run().is_none());
}

#[test]
fn a_group_reports_everything_that_recalculated_in_it() {
    let (mut e, mut ids) = editor(&[
        json!(["A", "1"]),
        json!(["B", "2"]),
        json!("=SUM(A)"),
        json!("=SUM(B)"),
    ]);
    e.begin_group();
    let (a, b) = (inside(&e, 0, 0, 0), inside(&e, 1, 0, 0));
    enter(&mut e, &mut ids, a, "10");
    enter(&mut e, &mut ids, b, "20");
    e.end_group();
    let mut both = vec![row(&e, 2), row(&e, 3)];
    both.sort();
    assert_eq!(e.recalculated(), both);
    assert_eq!(shown(&e)[2..], ["10", "20"]);
}

/// Formulas and the links of milestone 4 together: a formula cell copied or moved to
/// another document keeps its kind and formula, and means there what the names mean there.
#[test]
fn formula_cells_copied_and_moved_between_documents_recalculate_in_both() {
    use wombat_core::links::{copy_between, move_between};
    let (mut a, mut ids) = editor(&[
        json!(["Costs", "10", "20"]),
        json!("=SUM(Costs)"),
        json!(["Totals", "=SUM(Costs) * 2"]),
        json!("=SUM(Totals)"),
    ]);
    // The other document: with IDs of its own.
    let mut other = SeededIds::from_u64(99);
    let items = [
        json!(["Costs", "1", "2"]),
        json!(""),
        json!(""),
        json!("=SUM(Totals)"),
    ];
    let mut b = Editor::new(build(&formula_support::column(&items), &mut other)).unwrap();
    assert_eq!(shown(&a)[1..], ["30", "Totals", "60"]);
    assert_eq!(shown(&b)[3], "#NAME?");

    // A copy of the formula cell: same formula, the other document's Costs.
    let formula = row(&a, 1);
    let to = row(&b, 1);
    copy_between(a.document(), formula, &mut b, to, &mut ids).unwrap();
    let pasted = b.document().cell(row(&b, 1)).unwrap();
    assert_eq!(
        (pasted.kind, pasted.text.as_str()),
        (CellKind::Formula, "=SUM(Costs)")
    );
    assert_eq!(shown(&b)[1], "3");
    assert_eq!(b.recalculated(), [row(&b, 1)]);
    assert_eq!(shown(&a)[1], "30");

    // The table with a formula in it moves: the source loses the name, the target gains it.
    let totals = row(&a, 2);
    let inner = inside(&a, 2, 0, 0);
    let to = row(&b, 2);
    move_between(&mut a, totals, &mut b, to, None, &mut ids).unwrap();
    assert_eq!(shown(&a)[2..], ["", "#NAME?"]);
    assert_eq!(a.recalculated(), [row(&a, 3)]);
    assert_eq!(a.calc().result(inner), None);
    assert_eq!(row(&b, 2), totals);
    assert_eq!(b.shown(inner), Some("6"));
    assert_eq!(shown(&b)[2..], ["Totals", "6"]);
    assert!(b.recalculated().contains(&inner) && b.recalculated().contains(&row(&b, 3)));
    for e in [&a, &b] {
        let full = Calc::new(e.document());
        assert_eq!(e.calc().results(), full.results());
    }

    // Undoing both halves gives both back, results included.
    a.undo().unwrap();
    b.undo().unwrap();
    assert_eq!(shown(&a)[1..], ["30", "Totals", "60"]);
    assert_eq!(shown(&b)[1..], ["3", "", "#NAME?"]);
    b.redo().unwrap();
    a.redo().unwrap();
    assert_eq!(shown(&b)[2..], ["Totals", "6"]);
    assert_eq!(shown(&a)[3], "#NAME?");
}

/// What a link's target is shown as, and which cells an imported link without a target
/// could go to: by what cells show (decision A66).
#[test]
fn links_show_and_find_formula_cells_by_their_results() {
    use wombat_core::construct::Construct;
    use wombat_core::links::{place, place_shown, resolve_unknown, unknown_links};
    let spec = json!([
        "",
        0,
        1,
        3,
        [
            ["='Sum of ' & 3", 6, 1, 1, [["=1+2", 6]]],
            ["see three", 0],
            ["3", 0, 1, 1, [["x", 0]]]
        ]
    ]);
    let mut ids = SeededIds::from_u64(1);
    let mut doc = build(&spec, &mut ids);
    let at = |doc: &wombat_core::Document, y| doc.child(doc.root(), 0, y).unwrap();
    let inner = doc.child(at(&doc, 0), 0, 0).unwrap();
    let calc = Calc::new(&doc);
    let stored = place(&doc, inner).unwrap();
    assert_eq!(
        (stored.text.as_str(), &stored.trail[..]),
        ("=1+2", &["".to_string(), "='Sum of ' & 3".to_string()][..])
    );
    let shown = place_shown(&doc, &calc, inner).unwrap();
    assert_eq!(shown.text, "3");
    assert_eq!(shown.trail, ["", "Sum of 3"]);

    // An imported link that names "3": the formula cell that shows 3 and the data cell
    // "3" are both candidates, so nothing is guessed.
    let mut extra = wombat_core::model::Extra::new();
    extra.insert("cts_target".into(), "3".into());
    doc.cell_mut(at(&doc, 1)).unwrap().links = vec![wombat_core::Link {
        start: 4,
        len: 5,
        target: wombat_core::LinkTarget::Unknown,
        extra,
    }];
    doc.validate().unwrap();
    let found = unknown_links(&doc);
    assert_eq!(found[0].candidates, [inner, at(&doc, 2)]);
    assert!(resolve_unknown(&doc).is_empty());
    // With the data cell renamed, the formula cell is the only one, and the link goes to it.
    doc.cell_mut(at(&doc, 2)).unwrap().text = "three".into();
    let commands = resolve_unknown(&doc);
    assert!(matches!(
        &commands[..],
        [Command::SetLink { target: Some(wombat_core::LinkTarget::Cell { doc: None, cell }), .. }] if *cell == inner
    ));
}
