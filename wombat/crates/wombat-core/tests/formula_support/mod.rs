//! Shared by the tests of formulas and programs: documents written as nested lists, the
//! way `referee/formulas` writes them (see `referee/formulas/app.py`):
//! `[text, kind]` or `[text, kind, columns, rows, [cells]]`, with TreeSheets' numbers for
//! the kinds. For a formula cell the text is its formula.
#![allow(dead_code)]

use serde_json::Value;
use wombat_core::construct::Construct;
use wombat_core::{CellId, CellKind, Document, Editor, SeededIds};

pub fn kind(number: &Value) -> CellKind {
    match number.as_u64().unwrap() {
        0 => CellKind::Data,
        1 => CellKind::Code,
        2 => CellKind::VarAssign,
        3 => CellKind::ViewH,
        4 => CellKind::VarRead,
        5 => CellKind::ViewV,
        6 => CellKind::Formula,
        other => panic!("kind {other}"),
    }
}

fn fill(doc: &mut Document, id: CellId, spec: &Value, ids: &mut SeededIds) {
    let cell = doc.cell_mut(id).unwrap();
    // Control characters become spaces, as in TreeSheets.
    cell.text = wombat_core::text::clean(spec[0].as_str().unwrap());
    cell.kind = kind(&spec[1]);
    if let Some(cells) = spec.get(4) {
        let (xs, ys) = (
            spec[2].as_u64().unwrap() as u32,
            spec[3].as_u64().unwrap() as u32,
        );
        doc.add_grid(id, xs, ys, ids).unwrap();
        let children = doc.cell(id).unwrap().grid().unwrap().cells().to_vec();
        for (child, spec) in children.into_iter().zip(cells.as_array().unwrap()) {
            fill(doc, child, spec, ids);
        }
    }
}

/// The document of a case.
pub fn build(spec: &Value, ids: &mut SeededIds) -> Document {
    let mut doc = Document::new(ids);
    let root = doc.root();
    fill(&mut doc, root, spec, ids);
    doc.validate().unwrap();
    doc
}

/// A document of one column: every item is a cell, either a text (a formula cell if it
/// starts with `=`) or a table `(name, [texts])` written as `["name", "a", "b"]`.
pub fn column(items: &[Value]) -> Value {
    let cell = |text: &str| {
        let formula = text.starts_with('=') && text.len() > 1;
        serde_json::json!([text, if formula { 6 } else { 0 }])
    };
    let cells: Vec<Value> = items
        .iter()
        .map(|item| match item {
            Value::String(text) => cell(text),
            Value::Array(parts)
                if parts.first().is_some_and(Value::is_string) && parts.len() != 2
                    || parts.get(1).is_some_and(Value::is_string) =>
            {
                let name = parts[0].as_str().unwrap();
                let rows: Vec<Value> = parts[1..]
                    .iter()
                    .map(|p| cell(p.as_str().unwrap()))
                    .collect();
                serde_json::json!([name, 0, 1, rows.len(), rows])
            }
            other => other.clone(),
        })
        .collect();
    serde_json::json!(["", 0, 1, cells.len(), cells])
}

/// An editor on a document of one column (see [`column`]).
pub fn editor(items: &[Value]) -> (Editor, SeededIds) {
    let mut ids = SeededIds::from_u64(7);
    let doc = build(&column(items), &mut ids);
    (Editor::new(doc).unwrap(), ids)
}

/// The cell in row `row` of the root's one column, and a cell inside it.
pub fn row(editor: &Editor, row: u32) -> CellId {
    let doc = editor.document();
    doc.child(doc.root(), 0, row).unwrap()
}

pub fn inside(editor: &Editor, r: u32, x: u32, y: u32) -> CellId {
    editor.document().child(row(editor, r), x, y).unwrap()
}

/// What the cells of the root's column show.
pub fn shown(editor: &Editor) -> Vec<String> {
    let doc = editor.document();
    let grid = doc.cell(doc.root()).unwrap().grid().unwrap();
    grid.cells()
        .iter()
        .map(|id| editor.shown(*id).unwrap().to_string())
        .collect()
}
