//! Formulas and the program, compared with the real TreeSheets on its own examples: every
//! example `.cts` and the fixture is imported, and what every cell shows (for a formula
//! cell its result) is compared with `referee/golden/<name>.json`; then the program is run
//! and the document and the run's result are compared with `<name>.run.txt` and
//! `<name>.run-result.txt`, for the documents that have operation cells. Needs no
//! TreeSheets at test time.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use wombat_core::eval::Calc;
use wombat_core::{format, CellId, CellKind, Command, Document, Editor, SeededIds};
use wombat_cts::import;

fn referee_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../referee")
}

fn cts_path(name: &str) -> PathBuf {
    match name.strip_prefix("fixtures/") {
        Some(fixture) => referee_dir().join(format!("fixtures/{fixture}.cts")),
        None => referee_dir().join(format!("../../TS/examples/{name}.cts")),
    }
}

fn names() -> Vec<String> {
    let text = fs::read_to_string(referee_dir().join("golden/MANIFEST.json")).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();
    json["files"].as_object().unwrap().keys().cloned().collect()
}

/// The document as TreeSheets' text export writes it: one line per cell of the root's
/// grid and below, two spaces per level, a space after the text of a cell with an image.
fn text_export(doc: &Document, calc: &Calc) -> String {
    let mut out = String::new();
    let mut todo: Vec<(CellId, usize)> = Vec::new();
    let push = |todo: &mut Vec<(CellId, usize)>, id: CellId, indent: usize| {
        if let Some(grid) = doc.cell(id).unwrap().grid() {
            todo.extend(grid.cells().iter().rev().map(|c| (*c, indent)));
        }
    };
    push(&mut todo, doc.root(), 2);
    while let Some((id, indent)) = todo.pop() {
        out.push_str(&" ".repeat(indent));
        out.push_str(calc.shown(doc, id).unwrap());
        if doc.cell(id).unwrap().image.is_some() {
            out.push(' ');
        }
        out.push('\n');
        push(&mut todo, id, indent + 2);
    }
    out
}

fn compare_cells(
    editor: &Editor,
    id: CellId,
    golden: &Value,
    place: &str,
    counts: &mut (usize, usize),
    bad: &mut Vec<String>,
) {
    let doc: &Document = editor.document();
    let cell = doc.cell(id).unwrap();
    let shown = editor.shown(id).unwrap();
    let theirs = golden["text"].as_str().unwrap();
    counts.0 += 1;
    if cell.kind == CellKind::Formula {
        counts.1 += 1;
        assert_eq!(golden["type"], "formula", "{place}");
    }
    if shown != theirs {
        bad.push(format!(
            "{place}: Wombat shows {shown:?}, TreeSheets {theirs:?} (text {:?})",
            cell.text
        ));
    }
    if let Some(grid) = cell.grid() {
        let cells = golden["grid"]["cells"].as_array().unwrap();
        assert_eq!(cells.len(), grid.cells().len(), "{place}");
        for (n, (child, theirs)) in grid.cells().iter().zip(cells).enumerate() {
            let place = format!("{place}/{},{}", n as u32 % grid.xs() + 1, n as u32 / grid.xs() + 1);
            compare_cells(editor, *child, theirs, &place, counts, bad);
        }
    }
}

#[test]
fn formula_results_of_every_example_match_treesheets() {
    let mut bad = Vec::new();
    let mut counts = (0, 0);
    let mut files = 0;
    for name in names() {
        let mut ids = SeededIds::from_u64(5);
        let imported = import(&fs::read(cts_path(&name)).unwrap(), &mut ids).unwrap();
        let editor = Editor::new(imported.document).unwrap();
        let text = fs::read_to_string(referee_dir().join(format!("golden/{name}.json"))).unwrap();
        let golden: Value = serde_json::from_str(&text).unwrap();
        let root = editor.document().root();
        compare_cells(&editor, root, &golden["root"], &name, &mut counts, &mut bad);
        files += 1;
    }
    println!(
        "{files} documents, {} cells compared, {} of them formula cells, {} mismatches",
        counts.0,
        counts.1,
        bad.len()
    );
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    assert!(counts.1 >= 168, "the examples have 168 formula cells");
}

#[test]
fn running_the_program_of_every_example_matches_treesheets() {
    let mut bad = Vec::new();
    let (mut files, mut lines) = (0, 0);
    for name in names() {
        let golden = referee_dir().join(format!("golden/{name}.run.txt"));
        let Ok(theirs) = fs::read_to_string(&golden) else {
            continue;
        };
        let result = fs::read_to_string(golden.with_extension("").with_extension("run-result.txt"))
            .unwrap();
        let mut ids = SeededIds::from_u64(5);
        let imported = import(&fs::read(cts_path(&name)).unwrap(), &mut ids).unwrap();
        let mut editor = Editor::new(imported.document).unwrap();
        let before = format::to_string(editor.document()).unwrap();
        editor.apply(Command::Run, None, &mut ids).unwrap();
        editor.document().validate().unwrap();
        // The golden data is from the script function `ts.evaluate`, which unlike the menu
        // item does not keep the run's variables for the formulas: so the formulas are
        // shown here as they are without variables. (With them, the tutorial's formula
        // that reads a variable shows 18 instead of #N/A: see referee/formulas/sequences.)
        let ours = text_export(editor.document(), &Calc::new(editor.document()));
        files += 1;
        lines += theirs.lines().count() + result.lines().count();
        if ours != theirs {
            let at = ours.lines().zip(theirs.lines()).position(|(a, b)| a != b);
            bad.push(format!(
                "{name}: the document after the run differs, first at line {at:?}: {:?} / {:?}",
                at.and_then(|n| ours.lines().nth(n)),
                at.and_then(|n| theirs.lines().nth(n)),
            ));
        }
        let run = editor.calc().last_run().unwrap();
        let ours = run
            .result
            .as_ref()
            .map(|v| v.to_text(editor.document()))
            .unwrap_or_default();
        if ours != result {
            let at = ours.lines().zip(result.lines()).position(|(a, b)| a != b);
            bad.push(format!(
                "{name}: the result of the run differs, first at line {at:?}: {:?} / {:?}",
                at.and_then(|n| ours.lines().nth(n)),
                at.and_then(|n| result.lines().nth(n)),
            ));
        }
        // The run is one unit of undo, and undoing it gives the bytes back.
        if editor.undo_len() > 0 {
            editor.undo().unwrap();
        }
        assert_eq!(format::to_string(editor.document()).unwrap(), before, "{name}");
    }
    println!("{files} documents run, {lines} lines compared, {} mismatches", bad.len());
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    assert_eq!(files, 18);
}
