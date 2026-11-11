//! Search on real documents: every TreeSheets example, the referee fixtures and the
//! quirk files, imported. The index of each is held to the reference scanner for
//! queries made from its own content, as it is and while it is edited at random together
//! with a second document.
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

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use link_support::{act, run_with, Hook};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use search_support::{compare, query, Compared, Follower, Rng};
use wombat_core::summary::Summary;
use wombat_core::{Changes, Document, Editor, SeededIds};
use wombat_search::query::parse;
use wombat_search::{DocIndex, Options, SearchIndex};

impl Hook for Follower {
    fn changed(&mut self, editor: &Editor, changes: &Changes) {
        Follower::changed(self, editor, changes);
    }
    fn step(&mut self, editors: &[Editor], what: &str) {
        Follower::step(self, editors, what);
    }
}

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
            "../wombat-cts/tests/quirks",
        ] {
            collect(&here.join(dir), &mut files);
        }
        files.sort();
        let mut ids = SeededIds::from_u64(2026);
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

/// All imported documents as one workspace: what the index says is what reading every
/// cell says, for queries made from their content; and every summary survives its cache
/// form.
#[test]
fn search_over_all_imported_documents_agrees_with_the_reference() {
    let docs = documents();
    let named: Vec<(String, &Document)> = docs
        .iter()
        .enumerate()
        .map(|(n, (name, doc))| (format!("{n:02}-{name}.wombat"), doc))
        .collect();
    let named: Vec<(&str, &Document)> = named.iter().map(|(n, d)| (n.as_str(), *d)).collect();
    let calcs = search_support::calcs(&named);
    let named = search_support::named(&named, &calcs);
    let mut index = SearchIndex::new();
    let (mut cells, mut bytes) = (0, 0);
    for (name, doc, _) in &named {
        let summary = DocIndex::of(doc);
        let encoded = summary.encode();
        let back = DocIndex::decode(&encoded).unwrap_or_else(|| panic!("{name}: not read back"));
        assert_eq!(back.encode(), encoded, "{name}");
        bytes += encoded.len();
        cells += doc.cell_count();
        index.insert(back, name);
    }
    index.check().unwrap();
    assert_eq!(index.cell_count(), cells);
    let mut rng = Rng(0xc75);
    let mut stats = Compared::default();
    let reference = wombat_search::naive::Reference::new(&named);
    for _ in 0..120 {
        let query = query(&mut rng, &named, 0);
        compare(
            &[&index],
            &reference,
            &named,
            &query,
            &mut rng,
            "import",
            &mut stats,
        );
    }
    // And some that a person would type, in the languages of the examples.
    for text in [
        "treesheets",
        "the",
        "tree sheet",
        "グリッド",
        "한국어",
        "中",
        "は",
        "über",
        "e",
        "has:image",
        "has:link",
    ] {
        let query = parse(text, 0).unwrap().query;
        compare(
            &[&index],
            &reference,
            &named,
            &query,
            &mut rng,
            text,
            &mut stats,
        );
    }
    let all = index
        .search(&parse("", 0).unwrap().query, &Options::default())
        .unwrap();
    assert_eq!(all.total, cells);
    println!(
        "{} imported documents, {cells} cells, {} bytes of text, {bytes} bytes of summaries: {stats:?}",
        docs.len(),
        index.text_bytes()
    );
    assert!(docs.len() >= 49 && stats.selective > 80, "{stats:?}");
}

/// Every imported document, with the next one beside it, gets its own run of random
/// edits, with the search index following.
#[test]
fn search_follows_every_imported_document() {
    let docs = documents();
    let mut runner = TestRunner::deterministic();
    let mut total = Compared::default();
    for (n, (name, doc)) in docs.iter().enumerate() {
        let acts = proptest::collection::vec(act(), 8)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        let other = docs[(n + 1) % docs.len()].1.clone();
        // Large documents cost the reference scanner long; they get fewer queries.
        let queries = if doc.cell_count() + other.cell_count() > 3000 {
            1
        } else {
            2
        };
        let result = std::panic::catch_unwind(|| {
            let pair = vec![doc.clone(), other];
            let mut follower = Follower::new(&pair, n as u64 + 1, queries);
            run_with(pair, &acts, &mut follower);
            follower.stats
        });
        total.add(result.unwrap_or_else(|_| panic!("failed on {name}")));
    }
    println!("search following imported documents: {total:?}");
    assert!(total.queries > 400 && total.selective > 150, "{total:?}");
}
