//! The search index under random editing (the main test of milestone 6): after any
//! sequence of commands, groups, undos, redos, copies and moves on one or several
//! documents with formula cells in them, formulas and numbers typed in and programs run
//! (`EnterText` and `Run` of milestone 5) among the commands, the index that followed the change reports equals an index built from
//! scratch, and both give, for queries made from what the documents contain, exactly
//! the results of the reference scanner that reads every cell.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "../../wombat-core/tests/support/links.rs"]
mod link_support;
#[path = "support/mod.rs"]
mod search_support;
#[path = "../../wombat-core/tests/support/mod.rs"]
mod support;

use link_support::{act, run_with, Act, Hook};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use search_support::{Compared, Follower};
use support::{text_of, CLUSTERS};
use wombat_core::construct::Construct;
use wombat_core::{
    CellId, Changes, Color, Document, Editor, ImageRef, Link, LinkTarget, SeededIds, Tag,
};

impl Hook for Follower {
    fn changed(&mut self, editor: &Editor, changes: &Changes) {
        Follower::changed(self, editor, changes);
    }
    fn step(&mut self, editors: &[Editor], what: &str) {
        Follower::step(self, editors, what);
    }
}

/// The shape and texts of a random document: per cell its text and the size of its grid.
type Spec = (u64, Vec<(Vec<usize>, u32, u32)>);

fn spec() -> impl Strategy<Value = Spec> {
    let cell = (
        proptest::collection::vec(0..CLUSTERS.len(), 0..5),
        0..4u32,
        0..4u32,
    );
    (any::<u64>(), proptest::collection::vec(cell, 1..14))
}

const WORDS: &[&str] = &[
    "Apple pie",
    "apple",
    "PINEAPPLE",
    "Straße",
    "strasse 12",
    "日本語のテキスト",
    "本日",
    "Café au lait",
    "cafe",
    "ｶﾞｲﾄﾞ",
    "ガイド",
    "to-do list",
    "한국어",
];

/// As the documents of the link tests, and then given what search looks at: notes,
/// images, links to web addresses and to cells, tags, colours, kinds and edit times.
fn document((seed, cells): &Spec, salt: u64) -> Document {
    let mut ids = SeededIds::from_u64(seed.wrapping_mul(31).wrapping_add(salt));
    let mut doc = Document::new(&mut ids);
    let mut queue: Vec<CellId> = vec![doc.root()];
    for (next, (clusters, xs, ys)) in cells.iter().enumerate() {
        let Some(cell) = queue.get(next).copied() else {
            break;
        };
        doc.cell_mut(cell).unwrap().text = text_of(clusters);
        let (xs, ys) = if next == 0 {
            ((*xs).max(1), (*ys).max(2))
        } else {
            (*xs, *ys)
        };
        if xs > 0 && ys > 0 && doc.depth(cell).unwrap() < 3 {
            doc.add_grid(cell, xs, ys, &mut ids).unwrap();
            queue.extend(doc.cell(cell).unwrap().grid().unwrap().cells());
        }
    }
    let mut rng = search_support::Rng(seed | 1);
    let all = doc.cells_in_order();
    for id in &all {
        let other = all[rng.below(all.len())];
        let named = doc.cell(doc.root()).unwrap().text.clone();
        let cell = doc.cell_mut(*id).unwrap();
        if rng.below(3) == 0 {
            cell.text = WORDS[rng.below(WORDS.len())].to_owned();
        }
        match rng.below(12) {
            0 => cell.note = format!("Note about {}\nsecond line", WORDS[rng.below(WORDS.len())]),
            1 => cell.image = Some(ImageRef::new(format!("Photo{}.png", rng.below(3)))),
            2 => cell.cell_color = Color::rgb(0xff, 0, rng.below(2) as u8),
            3 => cell.last_edit = Some(rng.below(6) as i64 - 2),
            4 | 5 if !cell.text.is_empty() => {
                let target = if rng.below(2) == 0 {
                    LinkTarget::External(format!("https://example.com/{}", rng.below(3)))
                } else {
                    LinkTarget::Cell {
                        doc: None,
                        cell: other,
                    }
                };
                cell.links = vec![Link {
                    start: 0,
                    len: 1,
                    target,
                    extra: Default::default(),
                }];
            }
            6 => cell.kind = wombat_core::CellKind::ALL[rng.below(7)],
            7 => cell.style.bold = true,
            // Formula cells, whose results are searched: some that read a grid by the
            // text of its cell (so edits elsewhere change what they show), some not.
            8 | 9 => {
                cell.kind = wombat_core::CellKind::Formula;
                cell.text = match rng.below(5) {
                    0 => "='apple ' & 6 * 7".to_owned(),
                    1 => "=1/3".to_owned(),
                    2 => format!("=TREECOUNT(\"{}\") & ' cafés'", named.replace('"', "")),
                    3 => format!("=SUM(\"{}\") + 1", named.replace('"', "")),
                    _ => format!("=COUNT(\"{}\")", named.replace('"', "")),
                };
            }
            _ => {}
        }
    }
    for word in ["apple", "cafe", "a"] {
        if rng.below(2) == 0 {
            doc.tags_mut().insert(word.to_owned(), Tag::default());
        }
    }
    doc.validate().unwrap();
    doc
}

fn documents(specs: &[Spec]) -> Vec<Document> {
    specs
        .iter()
        .enumerate()
        .map(|(n, spec)| document(spec, n as u64))
        .collect()
}

fn follow(docs: Vec<Document>, acts: &[Act], seed: u64, queries: usize, early: bool) -> Compared {
    let mut follower = Follower::new(&docs, seed, queries);
    if early {
        follower = follower.compact_early();
    }
    run_with(docs, acts, &mut follower);
    follower.stats
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// One document: any commands, groups, undo and redo.
    #[test]
    fn search_follows_one_document(
        spec in spec(),
        acts in proptest::collection::vec(act(), 0..30),
        seed in any::<u64>(),
    ) {
        follow(documents(&[spec]), &acts, seed, 3, seed % 2 == 0);
    }

    /// Several documents with links, copies and moves between them.
    #[test]
    fn search_follows_several_documents(
        specs in proptest::collection::vec(spec(), 2..4),
        acts in proptest::collection::vec(act(), 0..30),
        seed in any::<u64>(),
    ) {
        follow(documents(&specs), &acts, seed, 3, seed % 2 == 0);
    }
}

/// A fixed run that says how much was compared, and fails if that is little: queries
/// that match nothing or everything prove little.
#[test]
fn the_comparison_with_the_reference_compares_enough() {
    let mut runner = TestRunner::deterministic();
    let mut total = Compared::default();
    for case in 0..40u64 {
        let specs = proptest::collection::vec(spec(), 1..4)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        let acts = proptest::collection::vec(act(), 25)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        total.add(follow(documents(&specs), &acts, case + 1, 4, case % 2 == 0));
    }
    println!("search against the reference: {total:?}");
    assert!(total.queries > 3000, "{total:?}");
    assert!(total.selective * 3 > total.queries, "{total:?}");
    assert!(
        total.highlighted > 3000 && total.by_linked_text > 20,
        "{total:?}"
    );
    // Formula cells are among the results, and found by what they show.
    assert!(
        total.formula_cells > 500 && total.by_shown > 50,
        "{total:?}"
    );
}
