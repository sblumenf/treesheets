//! Runs and links under editing, compared with TreeSheets' own bookkeeping code.
//!
//! `referee/ops/golden/textruns.jsonl` is what `src/textruns.h` of TreeSheets (compiled as
//! it is, by `referee/ops/make_ops`) does to the runs and links of a 7-character text for a
//! few thousand inserts, deletions, replacements, stylings, colourings and linkings. Each
//! line is replayed here as the Wombat command for the same edit.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;
use wombat_core::construct::Construct;
use wombat_core::{
    CellId, Color, Command, Document, Editor, Link, LinkTarget, Prop, Run, SeededIds, Style,
};

fn style(bits: u64) -> Style {
    Style {
        bold: bits & 1 != 0,
        italic: bits & 2 != 0,
        fixed: bits & 4 != 0,
        underline: bits & 8 != 0,
        strike: bits & 16 != 0,
    }
}

fn color(number: u64) -> Color {
    Color::rgb((number >> 16) as u8, (number >> 8) as u8, number as u8)
}

fn runs(value: &Value) -> Vec<Run> {
    let run = |r: &Value| Run {
        start: r[0].as_u64().unwrap() as u32,
        len: r[1].as_u64().unwrap() as u32,
        style: style(r[2].as_u64().unwrap()),
        color: r[3].as_u64().map(color),
        extra: Default::default(),
    };
    value.as_array().unwrap().iter().map(run).collect()
}

fn links(value: &Value) -> Vec<Link> {
    let link = |l: &Value| Link {
        start: l[0].as_u64().unwrap() as u32,
        len: l[1].as_u64().unwrap() as u32,
        target: LinkTarget::External(l[2].as_str().unwrap().into()),
        extra: Default::default(),
    };
    value.as_array().unwrap().iter().map(link).collect()
}

fn document(case: &Value, ids: &mut SeededIds) -> (Document, CellId) {
    let mut doc = Document::new(ids);
    let root = doc.root();
    let cell = doc.cell_mut(root).unwrap();
    cell.text = "abcdefg".into();
    cell.style = style(case["base"].as_u64().unwrap());
    cell.runs = runs(&case["runs"]);
    cell.links = links(&case["links"]);
    (doc, root)
}

#[test]
fn commands_adjust_runs_and_links_as_treesheets_does() {
    let file =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../referee/ops/golden/textruns.jsonl");
    let text = std::fs::read_to_string(file).unwrap();
    let mut ids = SeededIds::from_u64(1);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut changed_something = 0;
    for line in text.lines() {
        let case: Value = serde_json::from_str(line).unwrap();
        let (doc, cell) = document(&case, &mut ids);
        let before = doc.clone();
        let mut editor = Editor::new(doc).unwrap();
        let number = |key: &str| case[key].as_u64().unwrap() as u32;
        let (a, b, c) = (number("a"), number("b"), number("c"));
        let op = case["op"].as_str().unwrap();
        let command = match op {
            "insert" => Command::InsertText {
                cell,
                at: a,
                text: "x".repeat(b as usize),
            },
            "remove" => Command::DeleteText {
                cell,
                start: a,
                len: b,
            },
            "replace" => Command::ReplaceText {
                cell,
                start: a,
                len: b,
                text: "x".repeat(c as usize),
            },
            "style" => Command::SetRangeStyle {
                cell,
                start: a,
                len: b,
                set: style(u64::from(c / 32)),
                clear: style(u64::from(c % 32)),
            },
            "color" => Command::SetRangeColor {
                cell,
                start: a,
                len: b,
                color: Some(color(u64::from(c))),
            },
            "link" => Command::SetLink {
                cell,
                start: a,
                len: b,
                target: Some(case["target"].as_str().unwrap())
                    .filter(|target| !target.is_empty())
                    .map(|target| LinkTarget::External(target.into())),
            },
            "cellstyle" if b == 1 => Command::SetStyle {
                cells: vec![cell],
                set: style(u64::from(a)),
                clear: Style::default(),
            },
            "cellstyle" => Command::SetStyle {
                cells: vec![cell],
                set: Style::default(),
                clear: style(u64::from(a)),
            },
            // Setting the text colour takes the colours off the runs.
            "uncolor" => Command::SetProps {
                cells: vec![cell],
                props: vec![Prop::TextColor(Color::BLACK)],
            },
            other => panic!("unknown operation {other}"),
        };
        editor.apply(command, None, &mut ids).unwrap();
        let after = editor.document().cell(cell).unwrap();
        assert_eq!(after.runs, runs(&case["out_runs"]), "runs after {line}");
        assert_eq!(after.links, links(&case["out_links"]), "links after {line}");
        assert_eq!(
            after.style,
            style(case["out_base"].as_u64().unwrap()),
            "{line}"
        );
        editor.document().validate().unwrap();
        if *editor.document() != before {
            changed_something += 1;
            editor.undo().unwrap().unwrap();
            assert_eq!(*editor.document(), before, "undo after {line}");
        }
        *counts.entry(op.to_string()).or_default() += 1;
    }
    println!(
        "compared with TreeSheets' textruns.h: {counts:?}; {changed_something} changed something"
    );
    assert!(counts.values().sum::<usize>() > 3000 && counts.len() == 8);
}
