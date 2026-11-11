//! Ten thousand formula cells: how long working them all out takes, and how long after one
//! changed number. The numbers are printed (`cargo test --release -- --nocapture`); the
//! bounds asserted are generous.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod formula_support;

use std::time::{Duration, Instant};

use formula_support::build;
use serde_json::{json, Value};
use wombat_core::eval::Calc;
use wombat_core::{Command, Editor, SeededIds};

/// A budget of 1,000 departments: each a table of ten amounts, nine formulas about it and
/// one total; and at the end one formula over all of it. 10,001 formula cells, 22,005
/// cells.
fn budget() -> Value {
    let mut rows = Vec::new();
    for n in 0..1000 {
        let amounts: Vec<Value> = (0..10)
            .map(|m| json!([format!("{}", n + m * 3), 0]))
            .collect();
        let name = format!("Dept {n}");
        let mut row = vec![json!([name, 0, 1, 10, amounts])];
        for f in ["SUM", "MIN", "MAX", "AVG", "MEDIAN", "COUNT"] {
            row.push(json!([format!("={f}({name})"), 6]));
        }
        row.push(json!([format!("=SUM({name}) * 1.2 - MIN({name})"), 6]));
        row.push(json!([
            format!("=SUM(SUBSET({name}, 0, 2-5)) / COUNT({name})"),
            6
        ]));
        row.push(json!([format!("='{name}: ' & SUM({name})"), 6]));
        row.push(json!([
            format!("Total {n}"),
            0,
            1,
            1,
            [[format!("=SUM({name})"), 6]]
        ]));
        rows.extend(row);
    }
    let all = json!(["All", 0, 11, 1000, rows]);
    let grand = json!(["Grand", 0, 2, 1, [["=TREESUM(All)", 6], ["", 0]]]);
    json!(["", 0, 1, 2, [all, grand]])
}

fn best(times: u32, mut f: impl FnMut()) -> Duration {
    (0..times)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed()
        })
        .min()
        .unwrap()
}

#[test]
fn ten_thousand_formulas_are_worked_out_quickly() {
    let mut ids = SeededIds::from_u64(3);
    let doc = build(&budget(), &mut ids);
    assert_eq!(doc.cell_count(), 22_005);
    let full = best(5, || {
        let calc = Calc::new(&doc);
        assert_eq!(calc.results().len(), 10_001);
    });
    let mut editor = Editor::new(doc).unwrap();
    let doc = editor.document();
    let all = doc.child(doc.root(), 0, 0).unwrap();
    let dept = doc.child(all, 0, 500).unwrap();
    let amount = doc.child(dept, 0, 3).unwrap();
    let first = doc.child(all, 1, 500).unwrap();
    assert_eq!(editor.shown(first), Some("5135"));

    // One number changes: the ten formulas of its department and the grand total.
    let mut n = 0;
    let one = best(20, || {
        n += 1;
        let text = format!("{n}");
        editor
            .apply(Command::SetText { cell: amount, text }, None, &mut ids)
            .unwrap();
    });
    let stats = editor.calc().stats();
    assert!(!stats.full);
    assert_eq!(stats.evaluated, 11);
    // Not all eleven results differ: the count, for one, stays the same.
    assert!((5..=11).contains(&editor.recalculated().len()));
    assert_eq!(
        editor.shown(first),
        Some(format!("{}", 5135 - 509 + n).as_str())
    );

    // A row is inserted: names may mean something else now, so everything is worked out.
    let structural = best(5, || {
        editor
            .apply(
                Command::InsertRows {
                    cell: dept,
                    at: 0,
                    count: 1,
                },
                None,
                &mut ids,
            )
            .unwrap();
        assert!(editor.calc().stats().full);
        editor.undo().unwrap();
    });
    println!(
        "10,001 formulas in 22,005 cells: all {full:?}, after one changed number {one:?}, \
         after an inserted row and its undo {structural:?}"
    );
    let slow = cfg!(debug_assertions);
    assert!(
        full < Duration::from_millis(if slow { 3000 } else { 500 }),
        "{full:?}"
    );
    assert!(
        one < Duration::from_millis(if slow { 300 } else { 50 }),
        "{one:?}"
    );
    assert!(
        structural < Duration::from_millis(if slow { 6000 } else { 1000 }),
        "{structural:?}"
    );
}

#[test]
fn a_document_without_formulas_pays_nothing() {
    // 101,001 plain cells: structural edits cost what they did before formulas existed.
    let cells: Vec<Value> = (0..1000)
        .map(|n| {
            let inner: Vec<Value> = (0..100).map(|m| json!([format!("{m}"), 0])).collect();
            json!([format!("S{n}"), 0, 10, 10, inner])
        })
        .collect();
    let mut ids = SeededIds::from_u64(3);
    let doc = build(&json!(["", 0, 10, 100, cells]), &mut ids);
    let mut editor = Editor::new(doc).unwrap();
    let root = editor.document().root();
    let insert = best(20, || {
        editor
            .apply(
                Command::InsertRows {
                    cell: root,
                    at: 5,
                    count: 1,
                },
                None,
                &mut ids,
            )
            .unwrap();
        editor.undo().unwrap();
    });
    println!("inserting a row into 101,001 cells without formulas, and undoing it: {insert:?}");
    assert_eq!(editor.calc().stats().evaluated, 0);
    assert!(insert < Duration::from_millis(if cfg!(debug_assertions) { 20 } else { 5 }));
}
