//! The evaluator compared with the real TreeSheets (`referee/formulas`): hand-written and
//! random documents, built in the real app by `referee/formulas/make_formulas`, with what
//! the app showed for every cell; the same for running programs; and series of edits with
//! the document after every step. Needs no TreeSheets at test time.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod formula_support;

use std::fs;
use std::path::PathBuf;

use formula_support::{build, kind};
use serde_json::Value;
use wombat_core::eval::Calc;
use wombat_core::{CellId, CellKind, Command, Document, Editor, Prop, SeededIds};

fn golden(name: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../referee/formulas/golden")
        .join(format!("{name}.jsonl"));
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[derive(Default)]
struct Report {
    cells: usize,
    formulas: usize,
    bad: Vec<String>,
}

/// Compares what every cell shows, its kind and its grid's size with the app's.
fn compare(doc: &Document, calc: &Calc, id: CellId, theirs: &Value, place: &str, r: &mut Report) {
    let cell = doc.cell(id).unwrap();
    r.cells += 1;
    if cell.kind == CellKind::Formula {
        r.formulas += 1;
    }
    let shown = calc.shown(doc, id).unwrap();
    if shown != theirs[0].as_str().unwrap() || cell.kind != kind(&theirs[1]) {
        r.bad.push(format!(
            "{place}: Wombat {shown:?} ({:?}, text {:?}), TreeSheets {} (kind {})",
            cell.kind, cell.text, theirs[0], theirs[1]
        ));
    }
    let size = cell.grid().map(|g| (u64::from(g.xs()), u64::from(g.ys())));
    let their_size = theirs
        .get(4)
        .map(|_| (theirs[2].as_u64().unwrap(), theirs[3].as_u64().unwrap()));
    if size != their_size {
        r.bad
            .push(format!("{place}: grid {size:?}, TreeSheets {their_size:?}"));
        return;
    }
    if let Some(grid) = cell.grid() {
        for (n, (child, theirs)) in grid
            .cells()
            .iter()
            .zip(theirs[4].as_array().unwrap())
            .enumerate()
        {
            let n = n as u32;
            let place = format!("{place}/{},{}", n % grid.xs(), n / grid.xs());
            compare(doc, calc, *child, theirs, &place, r);
        }
    }
}

fn check(r: &Report, what: &str, at_least: usize) {
    println!(
        "{what}: {} cells compared, {} of them formula cells, {} mismatches",
        r.cells,
        r.formulas,
        r.bad.len()
    );
    assert!(r.bad.is_empty(), "{}", r.bad.join("\n"));
    assert!(r.cells >= at_least);
}

#[test]
fn formulas_match_treesheets() {
    let mut r = Report::default();
    let cases = golden("formulas");
    for case in &cases {
        let mut ids = SeededIds::from_u64(1);
        let doc = build(&case["doc"], &mut ids);
        let editor = Editor::new(doc).unwrap();
        let name = case["name"].as_str().unwrap();
        compare(
            editor.document(),
            editor.calc(),
            editor.document().root(),
            &case["shown"],
            name,
            &mut r,
        );
    }
    println!("{} documents", cases.len());
    check(&r, "formulas", 10_000);
}

/// The cases added after the review of milestone 5: letters outside ASCII in the name of
/// `SUBSET`, and which characters can be part of a name.
#[test]
fn review_cases_match_treesheets() {
    let mut r = Report::default();
    for case in &golden("review") {
        let mut ids = SeededIds::from_u64(1);
        let editor = Editor::new(build(&case["doc"], &mut ids)).unwrap();
        let name = case["name"].as_str().unwrap();
        compare(
            editor.document(),
            editor.calc(),
            editor.document().root(),
            &case["shown"],
            name,
            &mut r,
        );
    }
    check(&r, "review cases", 70);
}

/// Formula cells with a grid, whose result is their name (decision B51).
#[test]
fn formula_cells_with_grids_match_treesheets() {
    let mut r = Report::default();
    for case in &golden("fgrid") {
        let mut ids = SeededIds::from_u64(1);
        let editor = Editor::new(build(&case["doc"], &mut ids)).unwrap();
        let name = case["name"].as_str().unwrap();
        compare(
            editor.document(),
            editor.calc(),
            editor.document().root(),
            &case["shown"],
            name,
            &mut r,
        );
    }
    check(&r, "formula cells with grids", 10);
}

#[test]
fn programs_match_treesheets() {
    let mut r = Report::default();
    let cases = golden("programs");
    let mut results = 0;
    for case in &cases {
        let mut ids = SeededIds::from_u64(1);
        let doc = build(&case["doc"], &mut ids);
        let mut editor = Editor::new(doc).unwrap();
        let name = case["name"].as_str().unwrap();
        let root = editor.document().root();
        compare(
            editor.document(),
            editor.calc(),
            root,
            &case["shown"],
            name,
            &mut r,
        );
        editor.apply(Command::Run, None, &mut ids).unwrap();
        editor.document().validate().unwrap();
        // `ts.evaluate` does not keep the variables for the formulas as the menu item
        // does, so the formulas are shown as they are without them.
        let calc = Calc::new(editor.document());
        compare(
            editor.document(),
            &calc,
            root,
            &case["after"],
            &format!("{name} (run)"),
            &mut r,
        );
        let run = editor.calc().last_run().unwrap();
        let ours = run
            .result
            .as_ref()
            .map(|v| v.to_text(editor.document()))
            .unwrap_or_default();
        results += 1;
        if ours != case["result"].as_str().unwrap() {
            r.bad.push(format!(
                "{name}: result {ours:?}, TreeSheets {}",
                case["result"]
            ));
        }
    }
    println!("{} documents, {results} results of runs", cases.len());
    check(&r, "programs", 5_000);
}

fn at(doc: &Document, path: &Value) -> CellId {
    let mut id = doc.root();
    for step in path.as_array().unwrap() {
        id = doc
            .child(
                id,
                step[0].as_u64().unwrap() as u32,
                step[1].as_u64().unwrap() as u32,
            )
            .unwrap();
    }
    id
}

/// One step of a sequence, as Wombat commands that do what the script function or menu
/// item of TreeSheets does.
fn step(editor: &mut Editor, step: &Value, ids: &mut SeededIds) {
    let n = |i: usize| step[i].as_u64().unwrap() as u32;
    let doc = editor.document();
    // Where Wombat refuses (a row inserted into a cell without a grid, rows to sort that
    // are not there), the script functions and menu items of TreeSheets do nothing.
    let apply = |editor: &mut Editor, command: Command, ids: &mut SeededIds| match editor
        .apply(command, None, ids)
    {
        Ok(_) | Err(wombat_core::Error::Refused(_)) => {}
        Err(other) => panic!("{other:?}"),
    };
    match step[0].as_str().unwrap() {
        // ts.set_text: the text, and a formula cell becomes a data cell.
        "text" => {
            let cell = at(doc, &step[1]);
            let formula = doc.cell(cell).unwrap().kind == CellKind::Formula;
            editor.begin_group();
            apply(
                editor,
                Command::SetText {
                    cell,
                    text: step[2].as_str().unwrap().into(),
                },
                ids,
            );
            if formula {
                apply(
                    editor,
                    Command::SetProps {
                        cells: vec![cell],
                        props: vec![Prop::Kind(CellKind::Data)],
                    },
                    ids,
                );
            }
            editor.end_group();
        }
        // ts.set_formula: the text, and the cell becomes a formula cell whatever the text.
        "formula" => {
            let cell = at(doc, &step[1]);
            editor.begin_group();
            apply(
                editor,
                Command::SetText {
                    cell,
                    text: step[2].as_str().unwrap().into(),
                },
                ids,
            );
            apply(
                editor,
                Command::SetProps {
                    cells: vec![cell],
                    props: vec![Prop::Kind(CellKind::Formula)],
                },
                ids,
            );
            editor.end_group();
        }
        // ts.set_cell_type. In TreeSheets the text of a formula cell is its result, which
        // stays when the cell stops being a formula; in Wombat the text is the formula
        // (decision B55), so here the result is made the text first.
        "kind" => {
            let cell = at(doc, &step[1]);
            let new = kind(&step[2]);
            let result = (doc.cell(cell).unwrap().kind == CellKind::Formula
                && new != CellKind::Formula)
                .then(|| editor.shown(cell).unwrap().to_string());
            editor.begin_group();
            if let Some(text) = result {
                apply(editor, Command::SetText { cell, text }, ids);
            }
            apply(
                editor,
                Command::SetProps {
                    cells: vec![cell],
                    props: vec![Prop::Kind(new)],
                },
                ids,
            );
            editor.end_group();
        }
        "insert-row" => apply(
            editor,
            Command::InsertRows {
                cell: at(doc, &step[1]),
                at: n(2),
                count: 1,
            },
            ids,
        ),
        "insert-column" => {
            apply(
                editor,
                Command::InsertColumns {
                    cell: at(doc, &step[1]),
                    at: n(2),
                    count: 1,
                },
                ids,
            );
        }
        "delete" => {
            let cell = at(doc, &step[1]);
            apply(
                editor,
                Command::DeleteCells {
                    cell,
                    x: n(2),
                    y: n(3),
                    xs: n(4),
                    ys: n(5),
                },
                ids,
            );
        }
        "sort" => {
            let cell = at(doc, &step[1]);
            let descending = step[5].as_bool().unwrap();
            apply(
                editor,
                Command::SortRows {
                    cell,
                    column: n(2),
                    row: n(3),
                    count: n(4),
                    descending,
                },
                ids,
            );
        }
        "run" => apply(editor, Command::Run, ids),
        "undo" => {
            editor.undo().unwrap();
        }
        "redo" => {
            editor.redo().unwrap();
        }
        other => panic!("step {other}"),
    }
}

#[test]
fn sequences_of_edits_match_treesheets() {
    let mut r = Report::default();
    let cases = golden("sequences");
    let mut steps = 0;
    for case in &cases {
        let mut ids = SeededIds::from_u64(1);
        let doc = build(&case["doc"], &mut ids);
        let mut editor = Editor::new(doc).unwrap();
        let name = case["name"].as_str().unwrap();
        compare(
            editor.document(),
            editor.calc(),
            editor.document().root(),
            &case["shown"],
            name,
            &mut r,
        );
        for (n, s) in case["steps"].as_array().unwrap().iter().enumerate() {
            step(&mut editor, &s["do"], &mut ids);
            editor.document().validate().unwrap();
            steps += 1;
            let place = format!("{name} step {} {}", n + 1, s["do"]);
            compare(
                editor.document(),
                editor.calc(),
                editor.document().root(),
                &s["shown"],
                &place,
                &mut r,
            );
            // Keeping up to date after each step gives what working everything out gives.
            let full = Calc::with_variables(editor.document(), editor.calc().variables().clone());
            assert_eq!(editor.calc().results(), full.results(), "{place}");
        }
    }
    println!("{} sequences, {steps} steps", cases.len());
    check(&r, "sequences", 5_000);
}
