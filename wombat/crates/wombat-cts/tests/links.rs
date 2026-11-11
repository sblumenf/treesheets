//! The link index on real documents: every TreeSheets example, the referee fixtures and the
//! quirk files, imported (their links by text became links by ID, decision B1), edited at
//! random two and three at a time. The checks are those of `wombat-core/tests/links.rs`:
//! after every step the index that followed the change reports equals one built from the
//! documents, and links survive whatever leaves their target in place.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "../../wombat-core/tests/support/links.rs"]
mod link_support;
#[path = "../../wombat-core/tests/support/mod.rs"]
mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use link_support::{act, run, Stats};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use wombat_core::links::{self, Broken, Resolved};
use wombat_core::{Document, Editor, LinkIndex, SeededIds};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "cts") {
            out.push(path);
        }
    }
}

/// Every `.cts` at hand, imported once, with IDs that no two of them share.
fn documents() -> &'static [(String, Document)] {
    static DOCUMENTS: OnceLock<Vec<(String, Document)>> = OnceLock::new();
    DOCUMENTS.get_or_init(|| {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        for dir in [
            "../../../TS/examples",
            "../../referee/fixtures",
            "../../referee/ops",
            "tests/quirks",
        ] {
            collect(&here.join(dir), &mut files);
        }
        files.sort();
        let mut ids = SeededIds::from_u64(2025);
        files
            .iter()
            .map(|path| {
                let imported = wombat_cts::import_file(path, &mut ids).unwrap();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                (name, imported.document)
            })
            .collect()
    })
}

/// Every imported document, with the next one beside it, gets its own fixed run.
#[test]
fn the_index_follows_every_imported_document() {
    let docs = documents();
    let mut runner = TestRunner::deterministic();
    let mut total = Stats::default();
    let (mut links, mut unknown, mut cells) = (0, 0, 0);
    for (n, (name, doc)) in docs.iter().enumerate() {
        // What the importer made of the document's own links.
        let mut index = LinkIndex::new();
        index.insert_document(doc);
        for link in index.outgoing(doc.id()) {
            links += 1;
            match link.resolved {
                Resolved::Cell { moved, .. } => assert!(!moved, "{name}"),
                Resolved::External(_) => {}
                Resolved::Broken(Broken::Unknown) => {
                    unknown += 1;
                    assert!(link.link.hint.is_some(), "{name}: no text kept");
                }
                Resolved::Broken(why) => panic!("{name}: an imported link is broken: {why:?}"),
            }
        }
        // The importer resolved what could be resolved: nothing is left with exactly one
        // candidate unless TreeSheets' own rule would not go there (a formula cell's text
        // is not known, decision B15), so re-resolving must at least leave a valid document.
        let mut editor = Editor::new(doc.clone()).unwrap();
        for command in links::resolve_unknown(doc) {
            let changes = editor
                .apply(command, None, &mut SeededIds::from_u64(1))
                .unwrap();
            index.apply(editor.document(), &changes);
        }
        editor.document().validate().unwrap();
        cells += doc.cell_count();

        let acts = proptest::collection::vec(act(), 30)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        let other = docs[(n + 1) % docs.len()].1.clone();
        let result = std::panic::catch_unwind(|| run(vec![doc.clone(), other], &acts));
        total.add(result.unwrap_or_else(|_| panic!("failed on {name}")));
    }
    println!(
        "{} imported documents, {cells} cells, {links} links ({unknown} without a target): {total:?}",
        docs.len()
    );
    assert!(docs.len() >= 49 && links >= 10, "{links} links");
    assert!(total.applied > 300 && total.survived > 300, "{total:?}");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn the_index_follows_imported_documents_in_any_combination(
        which in proptest::collection::vec(any::<proptest::sample::Index>(), 2..4),
        acts in proptest::collection::vec(act(), 0..30),
    ) {
        let mut picked: Vec<usize> = which.iter().map(|i| i.index(documents().len())).collect();
        picked.sort_unstable();
        picked.dedup();
        let docs = picked.iter().map(|n| documents()[*n].1.clone()).collect();
        run(docs, &acts);
    }
}
