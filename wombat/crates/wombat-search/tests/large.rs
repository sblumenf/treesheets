//! Search at size: a made-up workspace of 100,000 cells of text like real notes (20,000
//! in a debug build), and by hand 1,000,000. The numbers are printed (and recorded in
//! `docs/milestones/M6.md`); the bounds are the milestone's speed target in a release
//! build and generous in a debug build, to catch a change of kind. One test at a time,
//! so that the memory read from the system is the test's own.
//!
//! `cargo test --release -p wombat-search --test large -- --nocapture --test-threads 1`
//! and with `--ignored` for a million cells.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "support/corpus.rs"]
mod corpus;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

use corpus::{queries, Corpus, NOW};
use wombat_core::scan::{CacheMode, CACHE_DIR};
use wombat_core::{Command, Document, Editor, SeededIds, Workspace};
use wombat_search::query::parse;
use wombat_search::{naive, workspace, Options, SearchIndex};

const SLOW: bool = cfg!(debug_assertions);

/// (resident memory now, its peak) of this process in KiB, where the system tells.
fn memory() -> Option<(u64, u64)> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let field = |name: &str| {
        let line = status.lines().find(|line| line.starts_with(name))?;
        line.split_whitespace().nth(1)?.parse::<u64>().ok()
    };
    Some((field("VmRSS:")?, field("VmHWM:")?))
}

fn timed<T>(run: impl FnOnce() -> T) -> (T, Duration) {
    let started = Instant::now();
    let out = run();
    (out, started.elapsed())
}

fn ms(time: Duration) -> String {
    format!("{:.2} ms", time.as_secs_f64() * 1000.0)
}

/// Runs every query of the set several times; returns, per query, the text, the number
/// of results and the median time.
fn run_queries(
    index: &SearchIndex,
    corpus: &Corpus,
    rounds: usize,
) -> Vec<(String, usize, Duration)> {
    let mut out = Vec::new();
    for text in queries(corpus) {
        let parsed = parse(&text, NOW).unwrap();
        let options = Options {
            order: parsed.order.unwrap_or_default(),
            offset: 0,
            limit: 50,
        };
        let mut times = Vec::new();
        let mut total = 0;
        for _ in 0..rounds {
            let (found, time) = timed(|| index.search(&parsed.query, &options).unwrap());
            assert!(found.hits.len() <= 50);
            total = found.total;
            times.push(time);
        }
        times.sort();
        out.push((text, total, times[times.len() / 2]));
    }
    out
}

fn report(what: &str, results: &[(String, usize, Duration)]) -> (Duration, Duration) {
    let mut times: Vec<Duration> = results.iter().map(|r| r.2).collect();
    times.sort();
    let (median, worst) = (times[times.len() / 2], times[times.len() - 1]);
    println!(
        "search at size: {what}: {} queries, median {}, worst {}",
        times.len(),
        ms(median),
        ms(worst)
    );
    let mut slowest: Vec<&(String, usize, Duration)> = results.iter().collect();
    slowest.sort_by_key(|r| std::cmp::Reverse(r.2));
    for (text, total, time) in slowest.iter().take(6) {
        println!(
            "search at size:   {:>10}  {total:>8} results  {text:?}",
            ms(*time)
        );
    }
    (median, worst)
}

#[test]
fn search_at_size() {
    let (docs, per_doc) = if SLOW { (10, 2_000) } else { (20, 5_000) };
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("search_large");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let ws = Workspace::open(&dir).unwrap();
    let mut corpus = Corpus::new(6);
    let mut documents: Vec<(String, Document)> = Vec::new();
    let mut file_bytes = 0;
    for n in 0..docs {
        let doc = corpus.document(per_doc);
        let name = format!("doc{n:03}.wombat");
        ws.save(&doc, &name).unwrap();
        file_bytes += fs::metadata(dir.join(&name)).unwrap().len();
        documents.push((name, doc));
    }
    let cells: usize = documents.iter().map(|(_, doc)| doc.cell_count()).sum();
    println!(
        "search at size: {docs} documents, {cells} cells, {} bytes of text and notes, {file_bytes} bytes of files",
        corpus.text_bytes
    );
    assert!(cells >= docs * per_doc * 9 / 10, "{cells} cells");

    // Building: from documents in memory, and from the folder.
    let before = memory();
    let (index, build) = timed(|| {
        let mut index = SearchIndex::new();
        for (name, doc) in &documents {
            index.insert_document(doc, name);
        }
        index
    });
    let after = memory();
    println!(
        "search at size: build from documents in memory: {}",
        ms(build)
    );
    let (counted, text) = (index.memory_bytes(), index.text_bytes());
    println!(
        "search at size: memory: {counted} bytes counted for {text} bytes of searched text ({:.1} times)",
        counted as f64 / text as f64
    );
    if let (Some((rss0, _)), Some((rss1, _))) = (before, after) {
        println!(
            "search at size: memory: the process grew by {} KiB while building",
            rss1.saturating_sub(rss0)
        );
    }
    assert_eq!(index.cell_count(), cells);
    assert!(
        counted < text * 16,
        "the index takes {counted} bytes for {text} of text"
    );

    let limit = Duration::from_secs(if SLOW { 120 } else { 20 });
    let (cold, cold_time) = timed(|| workspace::index(&ws, CacheMode::Verify).unwrap());
    println!(
        "search at size: build from the folder, parsing every document: {}",
        ms(cold_time)
    );
    assert_eq!((cold.parsed, cold.reused), (docs, 0));
    assert!(cold.cache_written && cold_time < limit);
    assert_eq!(cold.index.encoded(), index.encoded());
    let cache_bytes: u64 = fs::read_dir(dir.join(CACHE_DIR).join("search"))
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum();
    println!("search at size: cache: {cache_bytes} bytes");
    let (warm, warm_time) = timed(|| workspace::index(&ws, CacheMode::Verify).unwrap());
    println!(
        "search at size: load from the cache, reading every document to check: {}",
        ms(warm_time)
    );
    assert_eq!((warm.parsed, warm.reused), (0, docs));
    assert_eq!(warm.index.encoded(), index.encoded());
    // The fast way believes files that were old when their entries were written.
    for (name, _) in &documents {
        let file = fs::File::options()
            .write(true)
            .open(dir.join(name))
            .unwrap();
        file.set_modified(SystemTime::now() - Duration::from_secs(3600))
            .unwrap();
    }
    let settle = workspace::index(&ws, CacheMode::Fast).unwrap();
    assert_eq!((settle.parsed, settle.reused), (0, docs));
    let (fast, fast_time) = timed(|| workspace::index(&ws, CacheMode::Fast).unwrap());
    println!(
        "search at size: load from the cache, going by length and time: {}",
        ms(fast_time)
    );
    assert_eq!((fast.parsed, fast.reused), (0, docs));
    assert!(!fast.cache_written);
    assert_eq!(fast.index.encoded(), index.encoded());
    assert!(
        fast_time < cold_time,
        "the cache did not help: {fast_time:?} against {cold_time:?}"
    );
    drop((cold, warm, settle));

    // Searching: the index from the cache, so that it is that one which is measured.
    let index = fast.index;
    let results = run_queries(&index, &corpus, if SLOW { 3 } else { 9 });
    let (median, worst) = report("queries", &results);
    // The milestone's target: comfortably under 50 ms. (Debug builds: ten times that.)
    let scale = if SLOW { 10 } else { 1 };
    assert!(
        median < Duration::from_millis(10 * scale),
        "median {median:?}"
    );
    assert!(worst < Duration::from_millis(50 * scale), "worst {worst:?}");
    let nothing: Vec<&str> = results
        .iter()
        .filter(|r| r.1 == 0)
        .map(|r| r.0.as_str())
        .collect();
    assert!(
        nothing.len() <= if SLOW { 10 } else { 3 },
        "queries that find nothing measure little: {nothing:?}"
    );
    // The answers are the right ones: a few against the reference, which reads all.
    let named: Vec<(&str, &Document)> = documents.iter().map(|(n, d)| (n.as_str(), d)).collect();
    let reference = naive::Reference::new(&named);
    for text in queries(&corpus).iter().step_by(9) {
        let parsed = parse(text, NOW).unwrap();
        let options = Options {
            order: parsed.order.unwrap_or_default(),
            offset: 0,
            limit: 30,
        };
        let found = index.search(&parsed.query, &options).unwrap();
        assert_eq!(
            found,
            reference.search(&parsed.query, &options).unwrap(),
            "{text}"
        );
    }
    drop(reference);

    // Editing: typing into cells all over one document, the index following.
    let mut index = index;
    let (_, doc) = documents.swap_remove(3);
    let order = doc.cells_in_order();
    let mut editor = Editor::new(doc).unwrap();
    let mut ids = SeededIds::from_u64(1);
    let rounds = 2_000;
    let (mut apply_time, mut follow_time) = (Duration::ZERO, Duration::ZERO);
    for n in 0..rounds {
        let cell = order[(n * 7919) % order.len()];
        let command = match n % 10 {
            // Mostly typing; now and then a new row, which moves cells.
            0 => Command::InsertRows {
                cell: editor.document().root(),
                at: 0,
                count: 1,
            },
            _ => Command::InsertText {
                cell,
                at: 0,
                text: corpus.words[n % 5000].chars().take(1).collect(),
            },
        };
        let (changes, time) = timed(|| editor.apply(command, Some(NOW), &mut ids).unwrap());
        apply_time += time;
        let (_, time) = timed(|| index.apply(editor.document(), &changes));
        follow_time += time;
    }
    let per_edit = follow_time / rounds as u32;
    println!(
        "search at size: one edit: {:.1} µs for the command, {:.1} µs for the index to follow",
        apply_time.as_secs_f64() * 1e6 / rounds as f64,
        per_edit.as_secs_f64() * 1e6
    );
    assert!(
        per_edit < Duration::from_micros(if SLOW { 5_000 } else { 500 }),
        "{per_edit:?}"
    );
    // It followed rightly: equal to an index of the document as it is now.
    index.check().unwrap();
    let mut fresh = SearchIndex::new();
    fresh.insert_document(editor.document(), "doc003.wombat");
    let followed = index.encoded();
    let of_doc = followed
        .iter()
        .find(|(id, _)| *id == editor.document().id())
        .unwrap();
    assert!(of_doc.1 == fresh.encoded()[0].1, "the index drifted");
    // The first search after cells moved works the order of the document out again.
    let query = parse(&corpus.words[900], NOW).unwrap().query;
    let (_, first) = timed(|| index.search(&query, &Options::default()).unwrap());
    let (_, second) = timed(|| index.search(&query, &Options::default()).unwrap());
    println!(
        "search at size: a search right after cells moved: {}, and again: {}",
        ms(first),
        ms(second)
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// A million cells, in memory only (200 documents of 5,000 cells). By hand: it takes
/// about a gigabyte and a minute.
#[test]
#[ignore = "takes a gigabyte of memory; run by hand in a release build"]
fn search_in_a_million_cells() {
    let mut corpus = Corpus::new(6);
    let before = memory();
    let mut index = SearchIndex::new();
    let mut build = Duration::ZERO;
    let mut cells = 0;
    for n in 0..200 {
        let doc = corpus.document(5_000);
        cells += doc.cell_count();
        let (_, time) = timed(|| index.insert_document(&doc, &format!("doc{n:03}.wombat")));
        build += time;
    }
    let after = memory();
    println!(
        "search at size: a million: {cells} cells, {} bytes of text and notes",
        corpus.text_bytes
    );
    println!(
        "search at size: a million: build from documents in memory: {}",
        ms(build)
    );
    let (counted, text) = (index.memory_bytes(), index.text_bytes());
    println!(
        "search at size: a million: memory: {counted} bytes counted for {text} bytes of searched text ({:.1} times)",
        counted as f64 / text as f64
    );
    if let (Some((rss0, _)), Some((rss1, _))) = (before, after) {
        println!(
            "search at size: a million: the process grew by {} KiB",
            rss1.saturating_sub(rss0)
        );
    }
    let results = run_queries(&index, &corpus, 5);
    report("a million: queries", &results);
}
