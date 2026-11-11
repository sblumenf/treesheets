//! Writes a made-up workspace of documents with text like real notes, for trying search
//! at size: `cargo run --release -p wombat-search --example generate -- <folder> [cells]`
//! (100,000 cells if not said, in documents of 5,000).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "../tests/support/corpus.rs"]
mod corpus;

use std::path::Path;

use wombat_core::Workspace;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(folder) = args.first() else {
        eprintln!("usage: generate <folder> [cells]");
        std::process::exit(2);
    };
    let cells: usize = args.get(1).and_then(|n| n.parse().ok()).unwrap_or(100_000);
    std::fs::create_dir_all(folder).expect("the folder could not be made");
    let workspace = Workspace::open(Path::new(folder)).expect("not a folder");
    let mut corpus = corpus::Corpus::new(6);
    let mut made = 0;
    for n in 0..cells.div_ceil(5_000) {
        let doc = corpus.document(5_000.min(cells - made));
        made += doc.cell_count();
        workspace
            .save(&doc, &format!("doc{n:03}.wombat"))
            .expect("a document could not be saved");
    }
    println!(
        "{made} cells in {} documents in {folder}; words that are in them: {:?} {:?} {:?} {:?}",
        cells.div_ceil(5_000),
        corpus.words[3],
        corpus.words[900],
        corpus.words[15_000],
        corpus.chinese
    );
}
