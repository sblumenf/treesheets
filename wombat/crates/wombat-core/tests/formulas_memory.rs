//! What working out a small document may cost in memory, whatever its formulas (found in
//! review of milestone 5; decisions A80 to A82).
//!
//! This is the only test in this file, and must stay so. The memory it bounds is the
//! peak of the whole process, the one measure the system gives without unsafe code, and
//! the tests of one file run side by side in one process. It was in `formulas_limits.rs`,
//! where another test holds 300 MB for a moment: run alone this test peaked at 12 MB,
//! with the others at whatever they had reached by then, and failed with "443 MB" when
//! the machine was busy enough for it to finish last.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod formula_support;

use std::time::{Duration, Instant};

use formula_support::{build, peak_mb, slow};
use proptest::prelude::*;
use serde_json::{json, Value};
use wombat_core::eval::{self, Calc};
use wombat_core::{format, SeededIds};

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
    /// peak of the process can be read) a bounded amount of memory: that of all the
    /// cases so far, which nothing else in this process adds to.
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
