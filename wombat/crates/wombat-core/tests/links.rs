//! Links across documents under random editing: the index that follows the change reports
//! always equals an index built from the documents, and links survive whatever leaves
//! their target in place. (The same runs on imported TreeSheets documents in
//! `wombat-cts/tests/links.rs`.)
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "support/links.rs"]
mod link_support;
mod support;

use link_support::{act, run, Act, Stats};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use support::{text_of, CLUSTERS};
use wombat_core::construct::Construct;
use wombat_core::links::DocLinks;
use wombat_core::summary::Summary;
use wombat_core::{CellId, Document, Editor, LinkIndex, SeededIds};

/// The shape and texts of a random document: per cell its text and the size of its grid.
type Spec = (u64, Vec<(Vec<usize>, u32, u32)>);

fn spec() -> impl Strategy<Value = Spec> {
    let cell = (
        proptest::collection::vec(0..CLUSTERS.len(), 0..4),
        0..4u32,
        0..4u32,
    );
    (any::<u64>(), proptest::collection::vec(cell, 1..14))
}

/// Builds a document of up to three levels; the root always has a grid, and cells that
/// the spec does not reach get short texts, so that there is something to link and to
/// restructure. Different seeds give different IDs.
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
    for (n, cell) in queue.iter().enumerate().skip(cells.len()) {
        let pick = [Some(0), Some(1), None, Some(1), None];
        if let Some(cluster) = pick[(*seed as usize).wrapping_add(n * 7) % pick.len()] {
            doc.cell_mut(*cell).unwrap().text = CLUSTERS[cluster].to_string();
        }
    }
    doc.validate().unwrap();
    doc
}

fn documents(specs: &[Spec]) -> Vec<Document> {
    let docs: Vec<Document> = specs
        .iter()
        .enumerate()
        .map(|(n, spec)| document(spec, n as u64))
        .collect();
    // Two specs can be equal; the documents must still not share IDs.
    let mut seen = std::collections::HashSet::new();
    for doc in &docs {
        assert!(seen.insert(doc.id().0));
        for cell in doc.cells_in_order() {
            assert!(seen.insert(cell.0), "two documents share an ID");
        }
    }
    docs
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// One document: any commands, groups, undo and redo.
    #[test]
    fn the_index_follows_one_document(
        spec in spec(),
        acts in proptest::collection::vec(act(), 0..40),
    ) {
        run(documents(&[spec]), &acts);
    }

    /// Several documents with links, copies and moves between them.
    #[test]
    fn the_index_follows_several_documents(
        specs in proptest::collection::vec(spec(), 2..4),
        acts in proptest::collection::vec(act(), 0..50),
    ) {
        run(documents(&specs), &acts);
    }

    /// The summary of any document that editing leads to survives its cache form, and a
    /// summary that followed the changes on its own equals one read from the document.
    #[test]
    fn a_summary_follows_its_document_and_reads_back(
        specs in proptest::collection::vec(spec(), 2..3),
        acts in proptest::collection::vec(act(), 0..25),
    ) {
        let docs = documents(&specs);
        let mut ids = SeededIds::from_u64(1);
        let mut editors: Vec<Editor> = docs.into_iter().map(|d| Editor::new(d).unwrap()).collect();
        let mut summaries: Vec<DocLinks> =
            editors.iter().map(|e| DocLinks::of(e.document())).collect();
        for act in &acts {
            let (d, command) = match act {
                Act::Apply(d, op) => {
                    let d = d % editors.len();
                    (d, support::command(editors[d].document(), op))
                }
                _ => continue,
            };
            if let Ok(changes) = editors[d].apply(command, None, &mut ids) {
                summaries[d].update(editors[d].document(), editors[d].calc(), &changes);
            }
            let fresh = DocLinks::of(editors[d].document());
            prop_assert!(summaries[d] == fresh);
            prop_assert!(DocLinks::decode(&fresh.encode()) == Some(fresh));
        }
    }
}

/// A fixed set of runs that must do enough of everything for the rules above to mean
/// something: the numbers are printed, and the test fails if they become small.
#[test]
fn the_random_runs_exercise_links_copies_and_moves() {
    let mut runner = TestRunner::deterministic();
    let mut total = Stats::default();
    for _ in 0..250 {
        let specs = proptest::collection::vec(spec(), 3)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        let acts = proptest::collection::vec(act(), 60)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        total.add(run(documents(&specs), &acts));
    }
    println!("links under random editing: {total:?}");
    assert!(total.applied > 1500, "{total:?}");
    assert!(total.survived > 2000 && total.broken > 60, "{total:?}");
    assert!(total.pastes > 100 && total.moves > 100, "{total:?}");
    // Links that a copy or a move between documents had to keep pointing at their cells.
    assert!(total.kept > 1500, "{total:?}");
    assert!(total.repairs > 10 && total.links > 5, "{total:?}");
    assert!(total.retargets > 100, "{total:?}");
    assert!(total.formulas > 300 && total.formula_cells > 5, "{total:?}");
}

#[test]
fn an_index_of_no_documents_answers_nothing() {
    let index = LinkIndex::new();
    assert_eq!(index.broken().len(), 0);
    assert_eq!(index.documents().count(), 0);
}
