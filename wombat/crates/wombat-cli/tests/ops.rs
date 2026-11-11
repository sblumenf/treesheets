//! Editing commands compared with the real TreeSheets.
//!
//! `referee/ops/cases/NAME.lobster` did one operation in the real app on `fixture.cts`, and
//! `referee/ops/golden/NAME.json` is the document it had afterwards. Here the fixture is
//! imported, `cases/NAME.edit` (the same operation as `wombat edit` commands) is run by the
//! `wombat` program, and the saved result is compared cell by cell. The app is not needed.
//!
//! `referee/ops/ui/` has the same for the menu actions that no script function reaches
//! (sort, transpose, hierarchy swap, ...): there `cases/NAME.ui` says which keys were pressed
//! in the real app, on `ui/fixture.cts`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use wombat_core::eval::Calc;
use wombat_core::{storage, Cell, CellId, CellKind, Color, Document, LinkTarget, Style};

fn ops() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../referee/ops")
}

fn wombat(dir: &Path, args: &[&str]) {
    let output = Command::new(env!("CARGO_BIN_EXE_wombat"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "wombat {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn bits(style: Style) -> u64 {
    u64::from(style.bold)
        | u64::from(style.italic) << 1
        | u64::from(style.fixed) << 2
        | u64::from(style.underline) << 3
        | u64::from(style.strike) << 4
}

fn hex(color: Color) -> String {
    color.to_string().to_uppercase()
}

/// Per code point: style bits, colour, link target and whether it is a link to a cell.
type Look = Vec<(u64, String, Option<(String, bool)>)>;

fn golden_look(cell: &Value) -> Look {
    let text = cell["text"].as_str().unwrap();
    let base = (
        cell["stylebits"].as_u64().unwrap(),
        cell["fg"].as_str().unwrap().to_string(),
        None,
    );
    let mut look = vec![base; text.chars().count()];
    let range = |r: &Value| {
        let start = r["start"].as_u64().unwrap() as usize;
        start..start + r["len"].as_u64().unwrap() as usize
    };
    for run in cell["runs"].as_array().unwrap() {
        for at in range(run) {
            look[at].0 = run["stylebits"].as_u64().unwrap();
            look[at].1 = run["fg"].as_str().unwrap().to_string();
        }
    }
    for link in cell["links"].as_array().unwrap() {
        for at in range(link) {
            let target = link["target"].as_str().unwrap().to_string();
            look[at].2 = Some((target, link["internal"].as_bool().unwrap()));
        }
    }
    look
}

fn wombat_look(doc: &Document, cell: &Cell, shown: &str) -> Look {
    let base = (bits(cell.style), hex(cell.text_color), None);
    let mut look = vec![base; shown.chars().count()];
    if cell.kind == CellKind::Formula {
        // What a formula cell shows has no styled ranges or links, in either program.
        return look;
    }
    for run in &cell.runs {
        for at in run.start..run.start + run.len {
            look[at as usize].0 = bits(run.style);
            look[at as usize].1 = hex(run.color.unwrap_or(cell.text_color));
        }
    }
    for link in &cell.links {
        let target = match &link.target {
            LinkTarget::External(url) => (url.clone(), false),
            // TreeSheets links to a cell by its text.
            LinkTarget::Cell { cell, .. } => (doc.cell(*cell).unwrap().text.clone(), true),
            LinkTarget::Unknown => panic!("a link without a target"),
        };
        for at in link.start..link.start + link.len {
            look[at as usize].2 = Some(target.clone());
        }
    }
    look
}

/// Compares one cell and everything inside it; returns the number of cells compared.
fn compare(doc: &Document, calc: &Calc, id: CellId, golden: &Value, case: &str) -> usize {
    let cell = doc.cell(id).unwrap();
    // In TreeSheets the text of a formula cell is what it shows.
    let shown = calc.shown(doc, id).unwrap();
    let place = format!("{case} at {}", golden["path"]);
    let kind = match cell.kind.as_str() {
        "code" => "operation",
        "var_assign" => "variable_assign",
        "var_read" => "variable_read",
        "view_h" => "view_horizontal",
        "view_v" => "view_vertical",
        other => other,
    };
    let draw = match cell.draw_style.as_str() {
        "bubbles" => "bubble",
        "lines" => "line",
        other => other,
    };
    let ours = serde_json::json!({
        "text": shown, "note": cell.note, "type": kind, "bg": hex(cell.cell_color),
        "fg": hex(cell.text_color), "relsize": -cell.rel_size, "stylebits": bits(cell.style),
        "align": cell.text_align.as_str(), "valign": cell.vert_align.as_str(),
        "render_style": draw, "has_image": cell.image.is_some(),
        "is_tag": doc.tags().contains_key(&cell.text),
    });
    for (key, value) in ours.as_object().unwrap() {
        assert_eq!(*value, golden[key], "{place}: {key}");
    }
    assert_eq!(
        wombat_look(doc, cell, shown),
        golden_look(golden),
        "{place}: styles, colours, links"
    );
    let mut count = 1;
    match (cell.grid(), &golden["grid"]) {
        (None, Value::Null) => {}
        (Some(grid), expected @ Value::Object(_)) => {
            let widths: Vec<Value> = grid.col_widths.iter().map(|w| Value::from(*w)).collect();
            assert_eq!(
                (u64::from(grid.xs()), u64::from(grid.ys())),
                (
                    expected["cols"].as_u64().unwrap(),
                    expected["rows"].as_u64().unwrap()
                ),
                "{place}: grid size"
            );
            assert_eq!(
                hex(grid.border_color),
                expected["border_color"],
                "{place}: border"
            );
            assert_eq!(
                Value::from(widths),
                expected["col_widths"],
                "{place}: column widths"
            );
            assert_eq!(
                Value::from(grid.vertical),
                golden["vertical_layout"],
                "{place}: vertical"
            );
            let cells = expected["cells"].as_array().unwrap();
            assert_eq!(grid.cells().len(), cells.len());
            for (child, expected) in grid.cells().iter().zip(cells) {
                count += compare(doc, calc, *child, expected, case);
            }
        }
        (ours, expected) => panic!(
            "{place}: grid {} here, but {} in TreeSheets",
            if ours.is_some() { "present" } else { "absent" },
            if expected.is_null() {
                "absent"
            } else {
                "present"
            }
        ),
    }
    count
}

/// Parses a golden file: the header line, then the root cell.
fn golden(set: &Path, name: &str) -> Value {
    let text = fs::read_to_string(set.join("golden").join(format!("{name}.json"))).unwrap();
    let value: Value = serde_json::from_str(&text).unwrap();
    value["root"].clone()
}

/// Runs every case of a set (a folder with `fixture.cts`, `cases/` and `golden/`), whose
/// cases have a file with the extension `how` saying how the real app was driven.
fn run_set(set: &Path, how: &str, tmp: &str, at_least: usize) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(tmp);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let fixture = set.join("fixture.cts");
    let mut names: Vec<String> = fs::read_dir(set.join("cases"))
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().into_string().unwrap();
            name.strip_suffix(".edit").map(String::from)
        })
        .collect();
    names.sort();
    assert!(names.len() >= at_least, "only {} cases found", names.len());
    let mut cells = 0;
    let mut failed = Vec::new();
    for name in &names {
        let file = format!("{name}.wombat");
        wombat(&dir, &["import", fixture.to_str().unwrap(), &file]);
        let script = fs::read_to_string(set.join("cases").join(format!("{name}.edit"))).unwrap();
        let script_file = dir.join(format!("{name}.edit"));
        fs::write(&script_file, format!("{script}check\nsave\n")).unwrap();
        wombat(
            &dir,
            &["edit", &file, "--script", script_file.to_str().unwrap()],
        );
        let doc = storage::load_document(&dir.join(&file)).unwrap();
        // Go on after a mismatch, so that one run shows every case that differs.
        let calc = Calc::new(&doc);
        let compared =
            std::panic::catch_unwind(|| compare(&doc, &calc, doc.root(), &golden(set, name), name));
        match compared {
            Ok(count) => cells += count,
            Err(_) => failed.push(name.as_str()),
        }
    }
    assert!(
        failed.is_empty(),
        "cases that differ from TreeSheets: {failed:?}"
    );
    println!("{} operation cases, {cells} cells compared", names.len());
    // Every case says how the app was driven and has golden data, and the other way round.
    for kind in ["cases", "golden"] {
        for entry in fs::read_dir(set.join(kind)).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            let stem = name.split('.').next().unwrap().to_string();
            assert!(
                names.contains(&stem) || stem == "textruns",
                "{kind}/{name} has no case"
            );
        }
    }
    for name in &names {
        for file in [format!("cases/{name}.{how}"), format!("golden/{name}.json")] {
            assert!(set.join(&file).is_file(), "{file} is missing");
        }
    }
}

#[test]
fn operations_give_what_the_real_treesheets_gave() {
    run_set(&ops(), "lobster", "ops", 40);
}

/// The menu actions, which the real app carried out on key presses.
#[test]
fn menu_actions_give_what_the_real_treesheets_gave() {
    run_set(&ops().join("ui"), "ui", "ops-ui", 1);
}
