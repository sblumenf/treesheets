//! Rules for formulas and programs, on random documents and random commands: working a
//! document out never panics or hangs and gives the same every time; keeping results up to
//! date after each command gives what working everything out gives, and reports exactly the
//! results that changed; undoing everything, a run included, gives the original bytes.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod formula_support;
mod support;

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use formula_support::build;
use proptest::prelude::*;
use serde_json::{json, Value};
use wombat_core::eval::Calc;
use wombat_core::{format, CellId, CellKind, Command, Document, Editor, Error, Prop, SeededIds};

/// A small generator of its own, so that one seed gives one document on every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

const NAMES: &[&str] = &["A", "B", "C", "a b", "Total", "x"];
const DATA: &[&str] = &[
    "1", "2", "10", "2.5", "-3", "0", "", "abc", "007", "1e3", "0x10", " 4 ", "A", "B", "#NAME?",
    "é", "  ", "1e400",
];
const TOKENS: &[&str] = &[
    "SUM(",
    "MIN(",
    "MAX(",
    "AVG(",
    "MEDIAN(",
    "COUNT(",
    "TREESUM(",
    "TREECOUNT(",
    "SUBSET(",
    "TREESUBSET(",
    "LEN(",
    "LEFT(",
    "RIGHT(",
    "MID(",
    "UPPER(",
    "LOWER(",
    "TRIM(",
    "SUBSTITUTE(",
    "A",
    "B",
    "C",
    "a b",
    "Total",
    "x",
    "Missing",
    "\"A\"",
    "\"a b\"",
    "\"",
    "'",
    "'t'",
    "''",
    "(",
    ")",
    ")",
    ",",
    ", ",
    "-",
    "+",
    "*",
    "/",
    "&",
    " ",
    "0",
    "1",
    "2",
    "10",
    "2.5",
    "1e3",
    "1e999",
    ".",
    "=",
    "#",
    "é",
    "\u{3000}",
    "0-1",
    "1-",
    "😀",
    "e",
    "E",
];

/// A formula: often well formed, sometimes a soup of tokens, sometimes any characters.
fn formula(r: &mut Rng) -> String {
    let mut s = String::from("=");
    match r.below(10) {
        0..=4 => {
            let terms = 1 + r.below(3);
            for n in 0..terms {
                if n > 0 {
                    s.push_str(r.pick(&[" + ", "-", "*", "/", " & "]));
                }
                let name = r.pick(NAMES);
                match r.below(8) {
                    0 => s.push_str(r.pick(&["1", "2", "0.5", "10"])),
                    1 => s.push_str(&format!("\"{name}\"")),
                    2 => s.push_str(&format!("TREESUM({name})")),
                    3 => s.push_str(&format!("SUM(SUBSET({name}, {}, -))", r.below(3))),
                    4 => s.push_str(&format!("LEN({name} & 'x')")),
                    5 => s.push_str(&format!("COUNT(TREESUBSET({name}, -, -, {}-))", r.below(2))),
                    _ => s.push_str(&format!(
                        "{}({name})",
                        r.pick(&["SUM", "MAX", "MEDIAN", "AVG"])
                    )),
                }
            }
        }
        5..=8 => {
            for _ in 0..r.below(12) {
                s.push_str(r.pick(TOKENS));
            }
        }
        _ => {
            for _ in 0..r.below(20) {
                let c = char::from_u32(0x20 + (r.next() % 0x3000) as u32).unwrap_or('?');
                s.push(c);
            }
        }
    }
    s
}

/// A cell of a random document: data, a formula, a cell of the program, or a table.
fn cell(r: &mut Rng, depth: u32) -> Value {
    match r.below(20) {
        0..=7 => json!([r.pick(DATA), 0]),
        8..=11 => json!([formula(r), 6]),
        12 => json!([
            r.pick(&["+", "inc", "sum", "if", "neg", "nope", "*", "transpose"]),
            1
        ]),
        13 => json!([r.pick(&["x", "A", ""]), 2 + 2 * r.below(2)]),
        14 => json!([r.pick(&["", "old"]), 3 + 2 * r.below(2)]),
        _ if depth < 3 => {
            let (xs, ys) = (1 + r.below(3), 1 + r.below(3));
            let cells: Vec<Value> = (0..xs * ys).map(|_| cell(r, depth + 1)).collect();
            let kind = if r.below(12) == 0 { 6 } else { 0 };
            let text = if kind == 6 {
                formula(r)
            } else {
                r.pick(NAMES).to_string()
            };
            json!([text, kind, xs, ys, cells])
        }
        _ => json!([r.pick(DATA), 0]),
    }
}

fn document(seed: u64, ids: &mut SeededIds) -> Document {
    let mut r = Rng(seed | 1);
    let (xs, ys) = (1 + r.below(3), 2 + r.below(4));
    let mut cells: Vec<Value> = (0..xs * ys).map(|_| cell(&mut r, 1)).collect();
    // Always a chain: numbers, formulas that read them, and formulas that read those, so
    // that a changed number has to travel.
    let number = |r: &mut Rng| json!([r.pick(&["1", "2", "3.5", "10"]), 0]);
    let chain = [
        json!([
            "Chain",
            0,
            1,
            3,
            [number(&mut r), number(&mut r), number(&mut r)]
        ]),
        json!([
            "Sums",
            0,
            2,
            1,
            [["=SUM(Chain)", 6], ["=MAX(Chain) & '!'", 6]]
        ]),
        json!(["Top", 0, 1, 1, [["=SUM(Sums) * 2", 6]]]),
        json!(["=SUM(Top) + TREECOUNT(Chain)", 6]),
    ];
    for (n, link) in chain.into_iter().enumerate() {
        cells[n % (xs * ys)] = link;
    }
    build(&json!(["", 0, xs, ys, cells]), ids)
}

/// Every formula cell with what it shows and whether that is an error value.
fn results(calc: &Calc) -> BTreeMap<CellId, (String, bool)> {
    calc.results()
        .into_keys()
        .map(|id| {
            let (text, failed) = calc.result(id).unwrap();
            (id, (text.to_string(), failed))
        })
        .collect()
}

/// After a change: the results are those of working everything out, and the report names
/// exactly the formula cells whose result is new or different.
fn check(editor: &Editor, before: &BTreeMap<CellId, (String, bool)>, grouped: bool, what: &str) {
    let doc = editor.document();
    let full = Calc::with_variables(doc, editor.calc().variables().clone());
    let now = results(editor.calc());
    assert_eq!(now, results(&full), "after {what}");
    let formulas = doc
        .cells_in_order()
        .into_iter()
        .filter(|id| doc.cell(*id).unwrap().kind == CellKind::Formula)
        .count();
    assert_eq!(now.len(), formulas, "after {what}");
    let mut changed: Vec<CellId> = now
        .iter()
        .filter(|(id, text)| before.get(id) != Some(text))
        .map(|(id, _)| *id)
        .collect();
    changed.sort();
    let mut reported = editor.recalculated().to_vec();
    reported.sort();
    if grouped {
        // A group may name a result that changed and changed back.
        assert!(
            changed.iter().all(|id| reported.contains(id)),
            "after {what}"
        );
    } else {
        assert_eq!(reported, changed, "after {what}");
    }
}

/// A random command that has to do with formulas, on cells that exist.
fn command(r: &mut Rng, doc: &Document) -> Command {
    let order = doc.cells_in_order();
    let any = |r: &mut Rng| order[r.below(order.len())];
    let grids: Vec<CellId> = order
        .iter()
        .copied()
        .filter(|id| doc.cell(*id).unwrap().grid().is_some())
        .collect();
    let grid = |r: &mut Rng| grids[r.below(grids.len())];
    let size = |id: CellId| {
        let g = doc.cell(id).unwrap().grid().unwrap();
        (g.xs() as usize, g.ys() as usize)
    };
    // Often a cell of the first table in the document: the start of the chain, if it is
    // still there.
    let first = |r: &mut Rng| {
        let table = grids.get(1).copied().unwrap_or(doc.root());
        let cells = doc.cell(table).unwrap().grid().unwrap().cells();
        cells[r.below(cells.len())]
    };
    match r.below(18) {
        16..=17 => Command::EnterText {
            cell: first(r),
            text: r.pick(&["4", "5", "0.5", "x"]).into(),
        },
        0..=2 => Command::EnterText {
            cell: any(r),
            text: r.pick(DATA).into(),
        },
        3..=4 => Command::EnterText {
            cell: any(r),
            text: formula(r),
        },
        5 => Command::SetText {
            cell: any(r),
            text: r.pick(NAMES).into(),
        },
        6 => {
            let kind = CellKind::ALL[r.below(CellKind::ALL.len())];
            Command::SetProps {
                cells: vec![any(r)],
                props: vec![Prop::Kind(kind)],
            }
        }
        7 => {
            let cell = grid(r);
            Command::InsertRows {
                cell,
                at: r.below(size(cell).1 + 1) as u32,
                count: 1,
            }
        }
        8 => {
            let cell = grid(r);
            Command::InsertColumns {
                cell,
                at: r.below(size(cell).0 + 1) as u32,
                count: 1,
            }
        }
        9 => {
            let cell = grid(r);
            let (xs, ys) = size(cell);
            Command::DeleteCells {
                cell,
                x: r.below(xs) as u32,
                y: r.below(ys) as u32,
                xs: 1,
                ys: 1,
            }
        }
        10 => {
            let cell = grid(r);
            let (xs, ys) = size(cell);
            Command::SortRows {
                cell,
                column: r.below(xs) as u32,
                row: 0,
                count: ys as u32,
                descending: r.below(2) == 0,
            }
        }
        11 => Command::SwapCells {
            a: any(r),
            b: any(r),
        },
        12 => Command::CopyCell {
            from: any(r),
            to: any(r),
        },
        13 => Command::ClearCells {
            cells: vec![any(r)],
        },
        _ => Command::Run,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    /// Any text read as a number, and any number written as text.
    #[test]
    fn numbers_are_read_and_written_without_panic(
        text in "[ +\\-0-9a-fxXpPeE.infaN]{0,24}",
        bits in any::<u64>(),
    ) {
        use wombat_core::eval::{num_text, stream_num, strict_num};
        if let Some(d) = strict_num(&text) {
            prop_assert!(d.is_finite());
        }
        let _ = stream_num(&text);
        let d = f64::from_bits(bits);
        let written = num_text(d);
        prop_assert!(!written.is_empty() && !written.contains('e'));
        // What is written is read back as a number near the same, unless it is not finite.
        if d.is_finite() {
            let back = strict_num(&written);
            prop_assert!(back.is_some_and(|b| (b - d).abs() <= 1e-10 || b == d), "{}", written);
        }
    }

    /// Any document: worked out without panic, soon, and to the same results every time;
    /// its program runs or is refused.
    #[test]
    fn any_document_is_worked_out_soon_and_the_same_every_time(seed in any::<u64>()) {
        let mut ids = SeededIds::from_u64(seed);
        let doc = document(seed, &mut ids);
        let start = Instant::now();
        let first = Calc::new(&doc);
        let again = Calc::new(&format::from_slice(format::to_string(&doc).unwrap().as_bytes()).unwrap());
        prop_assert_eq!(results(&first), results(&again));
        let mut editor = Editor::new(doc).unwrap();
        match editor.apply(Command::Run, None, &mut ids) {
            Ok(_) | Err(Error::Limit(_)) => {}
            Err(other) => panic!("{other:?}"),
        }
        editor.document().validate().unwrap();
        prop_assert!(start.elapsed() < Duration::from_secs(20));
    }

    /// Any formula text at all.
    #[test]
    fn any_text_is_a_formula_with_some_result(text in ".{0,60}", soup in proptest::collection::vec(0..TOKENS.len(), 0..40)) {
        let soup: String = soup.iter().map(|n| TOKENS[*n]).collect();
        let mut ids = SeededIds::from_u64(1);
        let table = json!(["A", 0, 2, 2, [["1", 0], ["x", 0], [format!("={soup}"), 6], ["B", 0, 1, 1, [["2", 0]]]]]);
        let spec = json!(["", 0, 1, 4, [table, [text, 6], [format!("={text}"), 6], [format!("={soup}"), 6]]]);
        let doc = build(&spec, &mut ids);
        let calc = Calc::new(&doc);
        prop_assert_eq!(calc.results().len(), 4);
        prop_assert!(calc.results().values().all(|r| r.chars().count() < 10_000));
    }

    /// Random commands, groups that are ended or cancelled, undo and redo: the results are
    /// always those of working everything out, the report of what recalculated is exact
    /// (after a cancel: what taking the group back recalculated), and undoing everything
    /// gives the original bytes and results.
    #[test]
    fn keeping_up_to_date_equals_working_everything_out(
        seed in any::<u64>(),
        steps in proptest::collection::vec((any::<u64>(), 0..12u8, support::op()), 1..40),
    ) {
        let mut ids = SeededIds::from_u64(seed);
        let doc = document(seed, &mut ids);
        let original = format::to_string(&doc).unwrap();
        let mut editor = Editor::new(doc).unwrap();
        let first = results(editor.calc());
        let mut grouped = false;
        let mut group_before = first.clone();
        // Whether a run in the open group gave the variables other values: those stay
        // when the group is cancelled, as they stay when a run is undone.
        let mut group_ran = false;
        let mut group_bytes = original.clone();
        // What each command of the open group reported as recalculated.
        let mut group_said: Vec<CellId> = Vec::new();
        for (n, (step_seed, kind, op)) in steps.iter().enumerate() {
            let before = results(editor.calc());
            let mut r = Rng(*step_seed | 1);
            let what;
            match kind {
                0 if !grouped => {
                    what = format!("step {n}: undo");
                    if editor.undo().unwrap().is_none() { continue; }
                }
                1 if !grouped => {
                    what = format!("step {n}: redo");
                    if editor.redo().unwrap().is_none() { continue; }
                }
                2 if !grouped => {
                    editor.begin_group();
                    grouped = true;
                    group_before = before;
                    group_ran = false;
                    group_said.clear();
                    group_bytes = format::to_string(editor.document()).unwrap();
                    continue;
                }
                4 if grouped => {
                    editor.cancel_group().unwrap();
                    prop_assert_eq!(
                        format::to_string(editor.document()).unwrap(),
                        group_bytes.clone()
                    );
                    grouped = false;
                    what = format!("step {n}: cancel of group");
                    // Exact, where the end of a group may name too much: only the
                    // results that differ from those just before the cancel.
                    check(&editor, &before, false, &what);
                    if !group_ran {
                        prop_assert_eq!(results(editor.calc()), group_before.clone(), "{}", what);
                    }
                    continue;
                }
                3 if grouped => {
                    editor.end_group();
                    grouped = false;
                    check(&editor, &group_before, true, &format!("step {n}: end of group"));
                    // Everything that a command of this group reported and that still
                    // exists, by ID, and nothing from before the group.
                    group_said.retain(|id| editor.document().cell(*id).is_some());
                    group_said.sort();
                    group_said.dedup();
                    prop_assert_eq!(editor.recalculated(), group_said.clone(), "end of group");
                    continue;
                }
                _ => {
                    // One in three is any command of milestone 3, the others are about
                    // formulas.
                    let command = if kind % 3 == 0 {
                        support::command(editor.document(), op)
                    } else {
                        command(&mut r, editor.document())
                    };
                    what = format!("step {n}: {command:?}");
                    let variables = editor.calc().variables().clone();
                    match editor.apply(command, None, &mut ids) {
                        Ok(_) => group_ran |= *editor.calc().variables() != variables,
                        Err(Error::Refused(_) | Error::NotFound(_) | Error::Limit(_)) => {
                            prop_assert_eq!(results(editor.calc()), before, "refused: {}", what);
                            continue;
                        }
                        Err(other) => panic!("{what}: {other:?}"),
                    }
                }
            }
            editor.document().validate().unwrap();
            check(&editor, &before, false, &what);
            group_said.extend(editor.recalculated());
        }
        if grouped {
            editor.end_group();
        }
        while editor.undo().unwrap().is_some() {}
        prop_assert_eq!(format::to_string(editor.document()).unwrap(), original);
        // The variables of the runs stay (as in TreeSheets), so only without any run are
        // the results those of the start.
        if editor.calc().last_run().is_none() {
            prop_assert_eq!(results(editor.calc()), first);
        }
    }
}
