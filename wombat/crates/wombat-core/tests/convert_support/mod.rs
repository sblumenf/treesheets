//! Shared by the tests of import and export: random documents with everything that is
//! awkward in delimited text, a strict RFC 4180 reader that owes nothing to the one under
//! test, and the tree of a document as plain values.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::collection::vec;
use proptest::prelude::*;
use wombat_core::construct::Construct;
use wombat_core::convert::Source;
use wombat_core::eval::Calc;
use wombat_core::{CellId, CellKind, Document, SeededIds};

/// A cell to build: its text, whether it is a formula cell, its note and its grid (the
/// number of columns and the cells row by row).
#[derive(Clone, Debug)]
pub struct Spec {
    pub text: String,
    pub formula: bool,
    pub note: String,
    pub grid: Option<(u32, Vec<Spec>)>,
}

/// Texts that are awkward somewhere: delimiters, quotes, what a spreadsheet would run,
/// spaces at the ends, scripts beyond ASCII, a character outside the BMP.
pub const AWKWARD: &[&str] = &[
    "",
    "a",
    "plain text",
    "a,b",
    "x;y",
    "say \"hi\"",
    "\"",
    "\"\"",
    "a\"b",
    "=1+1",
    "=",
    "+1",
    "-5",
    "-2.5",
    "-cmd|' /C calc'!A0",
    "@SUM(1)",
    "+",
    "-",
    " lead",
    "trail ",
    "'quoted",
    "Éclair",
    "日本語",
    "\u{1F600}",
    "e\u{301}",
    "row_id",
    "A_id",
];

/// Any text a cell may have, the awkward ones often.
pub fn cell_text() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => proptest::sample::select(AWKWARD).prop_map(String::from),
        1 => "[ -~é日\u{1F600}]{0,8}",
        1 => vec(any::<char>(), 0..6)
            .prop_map(|c| wombat_core::text::clean(&c.into_iter().collect::<String>())),
    ]
}

fn note() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => Just(String::new()),
        1 => Just("a note".to_string()),
        1 => Just("two\nlines, and a \"quote\"".to_string()),
        1 => Just("=HYPERLINK(1)".to_string()),
        1 => Just("old Mac\rline ends".to_string()),
    ]
}

fn leaf() -> impl Strategy<Value = Spec> {
    let formula = proptest::sample::select(&["=1+2", "=LEN('abc') * 2", "=nope(", "=2*3"][..]);
    prop_oneof![
        8 => (cell_text(), note()).prop_map(|(text, note)| Spec {
            text,
            formula: false,
            note,
            grid: None
        }),
        1 => formula.prop_map(|text| Spec {
            text: text.to_string(),
            formula: true,
            note: String::new(),
            grid: None
        }),
    ]
}

fn with_grid(inner: impl Strategy<Value = Spec>, widest: u32) -> impl Strategy<Value = Spec> {
    (leaf(), 1..=widest, 1..=3u32, vec(inner, 9)).prop_map(|(mut cell, xs, ys, mut cells)| {
        cells.truncate((xs * ys) as usize);
        cell.grid = Some((xs, cells));
        cell
    })
}

/// A document's root: always with a grid, of grids up to `widest` columns wide nested up
/// to three deep.
pub fn spec(widest: u32) -> impl Strategy<Value = Spec> {
    let cells = leaf().prop_recursive(3, 40, 9, move |inner| with_grid(inner, widest));
    with_grid(cells, widest)
}

fn fill(doc: &mut Document, id: CellId, spec: &Spec, ids: &mut SeededIds) {
    let cell = doc.cell_mut(id).unwrap();
    cell.text.clone_from(&spec.text);
    cell.note.clone_from(&spec.note);
    if spec.formula {
        cell.kind = CellKind::Formula;
    }
    if let Some((xs, cells)) = &spec.grid {
        doc.add_grid(id, *xs, cells.len() as u32 / xs, ids).unwrap();
        let children = doc.cell(id).unwrap().grid().unwrap().cells().to_vec();
        for (child, spec) in children.into_iter().zip(cells) {
            fill(doc, child, spec, ids);
        }
    }
}

/// The document of a spec, with IDs from `seed`.
pub fn build(spec: &Spec, seed: u64) -> Document {
    let mut ids = SeededIds::from_u64(seed);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    fill(&mut doc, root, spec, &mut ids);
    doc.validate().unwrap();
    doc
}

/// A data cell, for documents written out in a test.
pub fn cell(text: &str) -> Spec {
    Spec {
        text: text.to_string(),
        formula: false,
        note: String::new(),
        grid: None,
    }
}

/// A formula cell.
pub fn formula(text: &str) -> Spec {
    Spec {
        formula: true,
        ..cell(text)
    }
}

/// A cell with a grid of `xs` columns.
pub fn grid(text: &str, xs: u32, cells: Vec<Spec>) -> Spec {
    assert_eq!(cells.len() % xs as usize, 0);
    Spec {
        grid: Some((xs, cells)),
        formula: false,
        ..cell(text)
    }
}

/// A cell and all in it as plain values: ID, exported text, note, grid.
#[derive(Clone, PartialEq, Debug)]
pub struct Tree {
    pub id: Option<CellId>,
    pub text: String,
    pub note: String,
    pub grid: Option<(u32, Vec<Tree>)>,
}

/// The tree of a cell: what each cell shows (or with `formulas` its own text).
pub fn tree(doc: &Document, calc: &Calc, id: CellId, formulas: bool) -> Tree {
    let cell = doc.cell(id).unwrap();
    let text = if formulas {
        cell.text.clone()
    } else {
        calc.shown(doc, id).unwrap().to_string()
    };
    Tree {
        id: Some(id),
        text,
        note: cell.note.clone(),
        grid: cell.grid().map(|g| {
            let cells = g.cells().iter().map(|c| tree(doc, calc, *c, formulas));
            (g.xs(), cells.collect())
        }),
    }
}

/// The same tree without IDs and notes, to compare with what an import made.
pub fn texts(tree: &Tree) -> Tree {
    Tree {
        id: None,
        text: tree.text.clone(),
        note: String::new(),
        grid: tree
            .grid
            .as_ref()
            .map(|(xs, cells)| (*xs, cells.iter().map(texts).collect())),
    }
}

/// The tree of a table to import.
pub fn table_tree(table: &wombat_core::convert::Table) -> (u32, Vec<Tree>) {
    let cells = table.cells().iter().map(|entry| Tree {
        id: None,
        text: entry.text().to_string(),
        note: String::new(),
        grid: entry.grid().map(table_tree),
    });
    (table.xs(), cells.collect())
}

/// A document with what its formula cells show.
pub fn source<'a>(doc: &'a Document, calc: &'a Calc) -> Source<'a> {
    Source { doc, calc }
}

/// Reads delimited text by the letter of RFC 4180 and panics at anything the RFC does not
/// allow: a quote in a field that is not quoted, text after a closing quote, a line break
/// that is not the one asked for, an empty line, a quote that is not closed. A byte-order
/// mark at the very start is allowed. Line breaks inside quoted fields are kept.
pub fn strict_csv(text: &str, delimiter: char, crlf: bool) -> Vec<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;
    let mut rows = Vec::new();
    let mut row = Vec::new();
    // Whether `at` is at a line end; steps over it.
    let line_end = |at: &mut usize| -> bool {
        let end: &[char] = if crlf { &['\r', '\n'] } else { &['\n'] };
        if chars[*at..].starts_with(end) {
            *at += end.len();
            true
        } else {
            false
        }
    };
    while at < chars.len() {
        let mut field = String::new();
        if row.is_empty() {
            assert!(!matches!(chars[at], '\r' | '\n'), "an empty line");
        }
        if chars[at] == '"' {
            at += 1;
            loop {
                let c = *chars.get(at).expect("a quote is not closed");
                at += 1;
                if c != '"' {
                    field.push(c);
                } else if chars.get(at) == Some(&'"') {
                    field.push('"');
                    at += 1;
                } else {
                    break;
                }
            }
        } else {
            while at < chars.len() && !matches!(chars[at], '\r' | '\n') && chars[at] != delimiter {
                assert_ne!(chars[at], '"', "a quote in a field that is not quoted");
                field.push(chars[at]);
                at += 1;
            }
        }
        row.push(field);
        if at == chars.len() {
            break;
        }
        if chars[at] == delimiter {
            at += 1;
            // A delimiter at the very end is followed by an empty field.
            if at == chars.len() {
                row.push(String::new());
            }
            continue;
        }
        assert!(
            line_end(&mut at),
            "text after a closing quote, or a wrong line end"
        );
        rows.push(std::mem::take(&mut row));
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}
