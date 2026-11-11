//! What a hostile or merely heavy document can make the calculation cost (found in review
//! of milestone 5; decisions A80 to A82): texts that grow, tables read over and over,
//! variables that hold copies. Each test here was a way to make the process run out of
//! memory or take many seconds. The times are printed (`cargo test --release --
//! --nocapture`); the bounds asserted are generous.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod formula_support;

use std::time::{Duration, Instant};

use formula_support::build;
use proptest::prelude::*;
use serde_json::{json, Value};
use wombat_core::eval::{self, Calc};
use wombat_core::{format, CellId, Command, Document, Editor, Error, SeededIds};

/// The most memory this process has held at once, in megabytes (Linux only).
fn peak_mb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|l| l.starts_with("VmHWM:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024)
}

fn slow() -> u32 {
    if cfg!(debug_assertions) {
        10
    } else {
        1
    }
}

fn top(doc: &Document) -> Vec<CellId> {
    doc.cell(doc.root())
        .unwrap()
        .grid()
        .unwrap()
        .cells()
        .to_vec()
}

/// The reviewer's first document: one formula of 397 characters whose text would be
/// 32^8 characters long.
fn bomb() -> String {
    let mut f = String::from("'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'");
    for _ in 0..7 {
        f = format!("SUBSTITUTE({f},'a','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')");
    }
    format!("=LEN({f})")
}

#[test]
fn a_text_that_would_grow_without_end_is_a_limit_error_and_the_document_loads() {
    let mut ids = SeededIds::from_u64(1);
    let formula = bomb();
    assert_eq!(formula.len(), 397);
    let doc = build(&json!(["", 0, 1, 1, [[formula, 6]]]), &mut ids);
    // As a file: it is read, and worked out on loading, in no time.
    let file = format::to_string(&doc).unwrap();
    assert!(file.len() < 1000, "{}", file.len());
    let start = Instant::now();
    let editor = Editor::new(format::from_slice(file.as_bytes()).unwrap()).unwrap();
    let cell = top(editor.document())[0];
    assert_eq!(editor.calc().result(cell), Some(("#LIMIT!", true)));
    assert!(start.elapsed() < Duration::from_millis(200 * u64::from(slow())));
    // One formula that is too long does not stop the others.
    assert!(!editor.calc().stats().full || editor.calc().stats().work < eval::MAX_WORK);
}

#[test]
fn texts_that_double_from_cell_to_cell_stop_at_the_limit() {
    // T0 is a text of 16 characters, every Ti is the one before it twice: the 41st would
    // have 16 * 2^40 characters.
    let mut cells = vec![json!(["T0", 0, 1, 1, [["aaaaaaaaaaaaaaaa", 0]]])];
    for i in 1..=40 {
        let formula = format!("=T{}&T{}", i - 1, i - 1);
        cells.push(json!([format!("T{i}"), 0, 1, 1, [[formula, 6]]]));
    }
    for i in [16, 17, 40] {
        cells.push(json!([format!("=LEN(T{i})"), 6]));
    }
    let mut ids = SeededIds::from_u64(1);
    let doc = build(&json!(["", 0, 1, cells.len(), cells]), &mut ids);
    let file = format::to_string(&doc).unwrap();
    let start = Instant::now();
    let mut editor = Editor::new(format::from_slice(file.as_bytes()).unwrap()).unwrap();
    let took = start.elapsed();
    let cells = top(editor.document());
    let shown = |e: &Editor, n: usize| e.shown(cells[n]).unwrap().to_string();
    // 16 * 2^16 bytes is the longest text there may be; the next one is over.
    assert_eq!(eval::MAX_TEXT_BYTES, 16 << 16);
    assert_eq!(shown(&editor, 41), "1048576");
    assert_eq!(shown(&editor, 42), "#LIMIT!");
    assert_eq!(shown(&editor, 43), "#LIMIT!");
    assert!(
        took < Duration::from_millis(300 * u64::from(slow())),
        "{took:?}"
    );
    // The texts that are held, all together, are what the limits allow and no more.
    let held: usize = editor.calc().results().values().map(|r| r.len()).sum();
    assert!(held < 4 * eval::MAX_TEXT_BYTES, "{held}");
    // It is an error like any other: a shorter text makes all of it work.
    let first = editor.document().child(cells[0], 0, 0).unwrap();
    let text = "a".to_string();
    editor
        .apply(Command::SetText { cell: first, text }, None, &mut ids)
        .unwrap();
    assert_eq!(shown(&editor, 42), "131072");
    assert_eq!(shown(&editor, 43), "#LIMIT!");
    if let Some(peak) = peak_mb() {
        println!("peak memory of the test process so far: {peak} MB");
    }
}

/// A table of `n` numbers called T.
fn table(n: usize) -> Value {
    let cells: Vec<Value> = (0..n).map(|i| json!([format!("{}", i % 97), 0])).collect();
    json!(["T", 0, 1, n, cells])
}

fn set(editor: &mut Editor, ids: &mut SeededIds, cell: CellId, text: &str) -> Duration {
    let text = text.to_string();
    let start = Instant::now();
    editor
        .apply(Command::SetText { cell, text }, None, ids)
        .unwrap();
    start.elapsed()
}

#[test]
fn a_table_that_one_formula_reads_450_times_is_read_once() {
    // The reviewer's second document: 100,000 numbers, and one formula of 3,150
    // characters that adds them up 450 times.
    let formula = format!("={}", ["SUM(T)"; 450].join("+"));
    let cells = vec![table(100_000), json!([formula, 6]), json!(["x", 0])];
    let mut ids = SeededIds::from_u64(1);
    let doc = build(&json!(["", 0, 1, 3, cells]), &mut ids);
    let start = Instant::now();
    let mut editor = Editor::new(doc).unwrap();
    let load = start.elapsed();
    let cells = top(editor.document());
    let total: u64 = (0..100_000u64).map(|i| i % 97).sum::<u64>() * 450;
    assert_eq!(editor.shown(cells[1]), Some(total.to_string().as_str()));
    // A cell that no formula reads: nothing is worked out.
    let other = set(&mut editor, &mut ids, cells[2], "y");
    let stats = editor.calc().stats();
    assert_eq!((stats.full, stats.evaluated), (false, 0), "{stats:?}");
    // A cell of the table: the one formula, once.
    let cell = editor.document().child(cells[0], 0, 0).unwrap();
    let inside = set(&mut editor, &mut ids, cell, "5");
    let stats = editor.calc().stats();
    assert_eq!(stats.evaluated, 1);
    assert_eq!(
        editor.shown(cells[1]),
        Some((total + 5 * 450).to_string().as_str())
    );
    println!(
        "450 x SUM of 100,000 cells: load {load:?}, edit elsewhere {other:?}, edit in the \
         table {inside:?}"
    );
    let ms = |n: u64| Duration::from_millis(n * u64::from(slow()));
    assert!(load < ms(300), "{load:?}");
    assert!(other < ms(5), "{other:?}");
    assert!(inside < ms(200), "{inside:?}");
}

#[test]
fn the_most_work_a_change_can_cost_is_one_recalculation_within_the_limit() {
    // Nothing here can be shared: every SUBSET is read anew. 14 of them over 100,000
    // cells is over two thirds of the work allowed, so everything is worked out after
    // every change that a formula reads: once, not twice.
    let formula = format!("={}", ["SUM(SUBSET(T,0,-))"; 14].join("+"));
    let cells = vec![table(100_000), json!([formula, 6]), json!(["x", 0])];
    let mut ids = SeededIds::from_u64(1);
    let doc = build(&json!(["", 0, 1, 3, cells]), &mut ids);
    let start = Instant::now();
    let mut editor = Editor::new(doc).unwrap();
    let load = start.elapsed();
    let cells = top(editor.document());
    let total: u64 = (0..100_000u64).map(|i| i % 97).sum::<u64>() * 14;
    assert_eq!(editor.shown(cells[1]), Some(total.to_string().as_str()));
    let all = editor.calc().stats().work;
    assert!(all > 1_400_000 && all * 2 > eval::MAX_WORK && all < eval::MAX_WORK);
    let other = set(&mut editor, &mut ids, cells[2], "y");
    let stats = editor.calc().stats();
    assert_eq!((stats.full, stats.evaluated, stats.work), (false, 0, 0));
    let cell = editor.document().child(cells[0], 0, 0).unwrap();
    let inside = set(&mut editor, &mut ids, cell, "5");
    let stats = editor.calc().stats();
    assert_eq!((stats.full, stats.evaluated, stats.work), (true, 1, all));
    // One more and it is over the limit, in no more time.
    let formula = format!("={}", ["SUM(SUBSET(T,0,-))"; 7].join("+"));
    let start = Instant::now();
    let text = formula.clone();
    editor
        .apply(
            Command::EnterText {
                cell: cells[2],
                text,
            },
            None,
            &mut ids,
        )
        .unwrap();
    let over = start.elapsed();
    assert_eq!(editor.shown(cells[2]), Some("#LIMIT!"));
    assert!(editor.calc().stats().work <= eval::MAX_WORK + 1);
    println!(
        "14 x SUM(SUBSET) of 100,000 cells ({all} of {} work): load {load:?}, edit \
         elsewhere {other:?}, edit in the table {inside:?}; over the limit {over:?}",
        eval::MAX_WORK
    );
    let ms = |n: u64| Duration::from_millis(n * u64::from(slow()));
    assert!(load < ms(1500), "{load:?}");
    assert!(other < ms(5), "{other:?}");
    assert!(inside < ms(1500), "{inside:?}");
    assert!(over < ms(2000), "{over:?}");
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n.max(1) as u64) as usize
    }
}

/// A formula made to grow: texts joined and substituted, the same names many times.
fn greedy(r: &mut Rng, names: usize, depth: u32) -> String {
    let name = |r: &mut Rng| format!("N{}", r.below(names));
    if depth == 0 {
        return match r.below(5) {
            0 => "'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'".to_string(),
            1 => "'ab'".to_string(),
            2 => "123456789".to_string(),
            _ => name(r),
        };
    }
    let sub = |r: &mut Rng| greedy(r, names, depth - 1);
    match r.below(9) {
        0 | 1 => {
            let a = sub(r);
            format!("{a}&{a}&{a}&{a}")
        }
        2 | 3 => format!("SUBSTITUTE({},'a',{})", sub(r), sub(r)),
        4 => format!("SUBSTITUTE({},'',{})", sub(r), sub(r)),
        5 => format!("UPPER({})&LOWER({})", sub(r), sub(r)),
        6 => format!("LEN({})", sub(r)),
        7 => format!("SUM({})+TREESUM({})", name(r), name(r)),
        _ => format!("RIGHT({},1000000)&MID({},1,99999999)", sub(r), sub(r)),
    }
}

/// A small document of tables N0, N1, ... that each hold formulas about the others.
fn greedy_document(seed: u64) -> Value {
    let mut r = Rng(seed | 1);
    let names = 2 + r.below(6);
    let mut tables = Vec::new();
    for n in 0..names {
        let rows = 1 + r.below(3);
        let cells: Vec<Value> = (0..rows)
            .map(|_| match r.below(4) {
                0 => json!([
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    0
                ]),
                1 => json!(["7", 0]),
                _ => {
                    let depth = 1 + r.below(8) as u32;
                    json!([format!("={}", greedy(&mut r, names, depth)), 6])
                }
            })
            .collect();
        tables.push(json!([format!("N{n}"), 0, 1, rows, cells]));
    }
    json!(["", 0, 1, tables.len(), tables])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// Whatever formulas a small document has, working it out stays inside a fixed
    /// envelope: every result within the longest text, all results together and all work
    /// within what one recalculation may cost, a bounded time, and (on Linux, where the
    /// peak of the process can be read) a bounded amount of memory.
    #[test]
    fn small_documents_cannot_cost_more_than_the_limits(seed in any::<u64>()) {
        let mut ids = SeededIds::from_u64(1);
        let spec = greedy_document(seed);
        let doc = build(&spec, &mut ids);
        prop_assume!(format::to_string(&doc).unwrap().len() < 300_000);
        let start = Instant::now();
        let calc = Calc::new(&doc);
        let took = start.elapsed();
        let results = calc.results();
        let held: usize = results.values().map(|r| r.len()).sum();
        prop_assert!(results.values().all(|r| r.len() <= eval::MAX_TEXT_BYTES));
        prop_assert!(calc.stats().work <= eval::MAX_WORK + 1);
        prop_assert!(held as u64 <= 16 * eval::MAX_WORK, "{held}");
        prop_assert!(took < Duration::from_millis(1000 * u64::from(slow())), "{took:?}");
        // The same again gives the same.
        let again = Calc::new(&doc);
        prop_assert_eq!(again.results(), results);
        if let Some(peak) = peak_mb() {
            prop_assert!(peak < 400, "{peak} MB");
        }
    }
}

#[test]
fn a_run_cannot_hold_more_than_twice_a_document_in_variables() {
    // The reviewer's third document: one row with a grid of 100,000 cells and 150 cells
    // that each assign it (and each of its cells) to variables.
    let n = 100_000;
    let cells: Vec<Value> = (0..n).map(|i| json!([format!("{}", i % 97), 0])).collect();
    let mut row = vec![json!(["", 0, 1, n, cells])];
    for i in 0..150 {
        row.push(json!([format!("v{i}"), 2]));
    }
    let mut ids = SeededIds::from_u64(1);
    let doc = build(&json!(["", 0, row.len(), 1, row]), &mut ids);
    let mut editor = Editor::new(doc).unwrap();
    let before = format::to_string(editor.document()).unwrap();
    let start = Instant::now();
    let result = editor.apply(Command::Run, None, &mut ids);
    let took = start.elapsed();
    let Err(Error::Limit(message)) = result else {
        panic!("{result:?}");
    };
    assert!(message.contains("variables"), "{message}");
    assert!(editor.calc().variables().is_empty());
    assert_eq!(format::to_string(editor.document()).unwrap(), before);
    println!(
        "a run refused for what its variables would hold: {took:?}, peak {:?} MB",
        peak_mb()
    );
    // Ten of them are fine, and the variables are there afterwards.
    let assigns = top(editor.document());
    for cell in &assigns[11..] {
        let props = vec![wombat_core::Prop::Kind(wombat_core::CellKind::Data)];
        let cells = vec![*cell];
        editor
            .apply(Command::SetProps { cells, props }, None, &mut ids)
            .unwrap();
    }
    editor.apply(Command::Run, None, &mut ids).unwrap();
    assert_eq!(editor.calc().variables().len(), 10);
}
