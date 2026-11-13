//! Import compared with the real TreeSheets: every file in `referee/exports/imports/`
//! was imported by the app through its File > Import menu, and the document it made is
//! in `imports/golden/NAME.json` (`referee/exports/make_exports`). Here the same bytes go
//! through Wombat's readers and the two trees are compared: texts, grid shapes, and that
//! no cell became a formula. Needs no TreeSheets at test time.
//!
//! One difference is taken out before comparing, by [`as_treesheets_holds`]: a line break
//! inside a quoted field, which Wombat keeps and TreeSheets cannot hold.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use wombat_core::convert::{decode, parse_outline, parse_table, Delimiter, Table};
use wombat_core::limits::MAX_DEPTH;

fn imports() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../referee/exports/imports")
}

/// A text of Wombat as the real TreeSheets would hold it: its cells have no line breaks,
/// and its importer makes one space of the line breaks inside a quoted field, however
/// many follow each other (`Grid::CSVImport`). Wombat keeps each as a line feed, a
/// deliberate difference (cell text may hold line breaks), so the golden files, which
/// are what the app made, are compared with this and are not changed.
fn as_treesheets_holds(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            while chars.next_if_eq(&'\n').is_some() {}
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

/// A table in the form of the golden files. With `kept`, the texts are Wombat's own;
/// without, as TreeSheets would hold them.
fn tree(table: &Table, kept: bool) -> Value {
    let cells: Vec<Value> = table
        .cells()
        .iter()
        .map(|entry| {
            let text = if kept {
                entry.text().to_string()
            } else {
                as_treesheets_holds(entry.text())
            };
            json!({
                "text": text,
                "type": if entry.is_formula() { "formula" } else { "data" },
                "grid": entry.grid().map(|grid| tree(grid, kept)),
            })
        })
        .collect();
    json!({ "cols": table.xs(), "rows": table.ys(), "cells": cells })
}

fn ours(name: &str) -> Value {
    ours_as(name, false)
}

/// What Wombat reads from a file, with its line breaks (`kept`) or without.
fn ours_as(name: &str, kept: bool) -> Value {
    let (text, _) = decode(&fs::read(imports().join(name)).unwrap(), None).unwrap();
    let table = match Path::new(name).extension().unwrap().to_str().unwrap() {
        "csv" => {
            // Looking for the delimiter of a file called CSV, as the tool does, never
            // finds another one in these: the comparison is of what the tool reads.
            let found = wombat_core::convert::delimited::detect(&text);
            assert!(matches!(found, None | Some(Delimiter::Comma)), "{name}");
            parse_table(&text, Delimiter::Comma, false)
        }
        "ssv" => parse_table(&text, Delimiter::Semicolon, false),
        "tsv" => parse_table(&text, Delimiter::Tab, false),
        "txt" => parse_outline(&text, MAX_DEPTH, false),
        other => panic!("{other}"),
    };
    tree(&table.unwrap(), kept)
}

fn theirs(name: &str) -> Value {
    let text = fs::read_to_string(imports().join(format!("golden/{name}.json"))).unwrap();
    let root: Value = serde_json::from_str(&text).unwrap();
    // The app imports into the grid of a new document's root cell, which has no text.
    assert_eq!(root["text"], "");
    root["grid"].clone()
}

/// The texts of a golden grid, row by row.
fn rows(grid: &Value) -> Vec<Vec<String>> {
    let cols = grid["cols"].as_u64().unwrap() as usize;
    let cells: Vec<String> = grid["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["text"].as_str().unwrap().to_string())
        .collect();
    cells.chunks(cols).map(<[String]>::to_vec).collect()
}

#[test]
fn every_imported_file_gives_the_document_treesheets_made() {
    let mut names: Vec<String> = fs::read_dir(imports())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != "golden")
        .collect();
    names.sort();
    assert_eq!(names.len(), 25);
    let mut differ = Vec::new();
    let mut cells = 0;
    for name in &names {
        let (ours, theirs) = (ours(name), theirs(name));
        cells += rows(&theirs).iter().map(Vec::len).sum::<usize>();
        if ours != theirs {
            differ.push(name.as_str());
        }
    }
    assert!(cells > 100);
    // Decision B82: the two files where Wombat reads delimited text differently.
    assert_eq!(differ, ["after_quote.csv", "trailing_delimiter.csv"]);

    // `"abc"x,def`: TreeSheets drops the character after a closing quote, whatever it is,
    // and with it the delimiter's place. Wombat keeps the text with the field.
    assert_eq!(
        rows(&theirs("after_quote.csv")),
        [["abc", "", "def"], ["g", "i", "j"]]
    );
    assert_eq!(
        rows(&ours("after_quote.csv")),
        [["abcx", "def"], ["ghi", "j"]]
    );
    // `a,b,`: TreeSheets drops an empty field at the end of a line. Wombat keeps it, as
    // RFC 4180 says, so that a table whose last column is empty comes back whole.
    assert_eq!(
        rows(&theirs("trailing_delimiter.csv")),
        [["a", "b"], ["c", "d"]]
    );
    assert_eq!(
        rows(&ours("trailing_delimiter.csv")),
        [["a", "b", ""], ["c", "d", ""]]
    );
}

#[test]
fn line_breaks_inside_quotes_are_kept_where_treesheets_makes_a_space() {
    // The files whose texts differ from the app's before the line breaks are taken out,
    // and exactly how: nothing else hides behind the helper.
    let mut names: Vec<String> = fs::read_dir(imports())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != "golden")
        .filter(|name| ours_as(name, true) != ours_as(name, false))
        .collect();
    names.sort();
    assert_eq!(names, ["quotes.csv", "unclosed_quote.csv"]);
    // `"multi⏎line","two⏎⏎breaks"`.
    assert_eq!(
        rows(&theirs("quotes.csv")),
        [
            ["a,b", "say \"hi\"", "plain"],
            ["multi line", "two breaks", "x"]
        ]
    );
    assert_eq!(
        rows(&ours_as("quotes.csv", true)),
        [
            ["a,b", "say \"hi\"", "plain"],
            ["multi\nline", "two\n\nbreaks", "x"]
        ]
    );
    // `a,"bc⏎d,e⏎`: the quote runs to the end of the file, whose last line end is no text.
    assert_eq!(rows(&theirs("unclosed_quote.csv")), [["a", "bc d,e"]]);
    assert_eq!(
        rows(&ours_as("unclosed_quote.csv", true)),
        [["a", "bc\nd,e"]]
    );
    assert_eq!(as_treesheets_holds("a\n\n\nb\nc\n"), "a b c ");
}

#[test]
fn treesheets_imports_text_that_starts_with_an_equals_sign_as_plain_text() {
    // The safe default of Wombat is what the app does: `=1+2` in a file is data.
    for name in ["formulas.csv", "formulas.txt"] {
        let text = fs::read_to_string(imports().join(format!("golden/{name}.json"))).unwrap();
        assert!(text.contains("\"text\": \"=1+2\""));
        assert!(!text.contains("\"type\": \"formula\""));
    }
}
