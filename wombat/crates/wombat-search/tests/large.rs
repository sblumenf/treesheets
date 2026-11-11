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
use wombat_core::construct::Construct;
use wombat_core::eval::Calc;
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
            index.insert_document(doc, &Calc::new(doc), name);
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
    let calcs: Vec<Calc> = named.iter().map(|(_, doc)| Calc::new(doc)).collect();
    let named: Vec<naive::Named> = named
        .iter()
        .zip(&calcs)
        .map(|((n, d), c)| (*n, *d, c))
        .collect();
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
        let (_, time) = timed(|| index.apply(editor.document(), editor.calc(), &changes));
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
    fresh.insert_document(editor.document(), editor.calc(), "doc003.wombat");
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
        let (_, time) =
            timed(|| index.insert_document(&doc, &Calc::new(&doc), &format!("doc{n:03}.wombat")));
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

/// The queries that a review found slow: many parts, each of which most cells have or
/// which no table helps with.
fn adversarial() -> Vec<(&'static str, String)> {
    let letters: Vec<String> = ('a'..='z').map(String::from).collect();
    let mut many: Vec<String> = letters.clone();
    many.extend(('0'..='9').map(String::from));
    many.extend(
        "éüßñæ的一是在不了有和人这中大为上个国我以要他时来用们生"
            .chars()
            .map(String::from),
    );
    many.truncate(wombat_search::query::MAX_PARTS);
    let pairs = [
        "ta", "ri", "no", "me", "co", "ve", "st", "pr", "in", "ex", "ar", "lo", "qu", "de", "sc",
        "um", "er", "an", "ti", "bl", "ou", "ea", "th", "ly", "wa", "ke", "zo", "fj", "by", "gr",
        "ph", "mi", "el", "on", "ro", "ui", "en", "ch", "mp", "io", "le", "tr", "ng", "ju", "ra",
        "ho", "at", "ir", "om", "nv", "xa", "ab", "cd", "ef", "gh", "ij", "kl", "mn", "op", "qr",
        "uv", "wx", "yz", "zz",
    ];
    let negations: Vec<String> = pairs.iter().map(|pair| format!("-{pair}")).collect();
    vec![
        ("one letter", "e".into()),
        ("26 letters, any", letters.join(" OR ")),
        ("64 characters, any", many.join(" OR ")),
        ("64 letters and digits, all", many.join(" ")),
        (
            "a phrase of 1,000 bytes",
            format!("\"{}\"", "e ".repeat(499)),
        ),
        ("64 negations", negations.join(" ")),
        (
            "26 negated letters",
            letters
                .iter()
                .map(|l| format!("-{l}"))
                .collect::<Vec<_>>()
                .join(" "),
        ),
        ("exact letter", "case:e".into()),
        ("exact word or word", "case:Ta OR case:ri OR case:E".into()),
        ("every cell", String::new()),
        ("every cell, by document", "sort:document".into()),
        ("one letter, most recent", "e sort:recent".into()),
        ("16 common pairs, any", pairs[..16].join(" OR ")),
        ("64 pairs, any", pairs.join(" OR ")),
        (
            "64 pairs, any, by document",
            format!("{} sort:document", pairs.join(" OR ")),
        ),
        ("three common words", "an er in".into()),
        ("the same letter 64 times", vec!["e"; 64].join(" OR ")),
        (
            "64 phrases of a letter and a space, any",
            pairs
                .iter()
                .map(|pair| format!("\"{} \"", &pair[1..]))
                .collect::<Vec<_>>()
                .join(" OR "),
        ),
    ]
}

/// The adversarial queries, timed; in a release build they must stay near the target.
#[test]
fn adversarial_queries_stay_bounded() {
    let (docs, per_doc) = if SLOW { (10, 2_000) } else { (20, 5_000) };
    let mut corpus = Corpus::new(6);
    let mut index = SearchIndex::new();
    let mut documents = Vec::new();
    for n in 0..docs {
        let doc = corpus.document(per_doc);
        let name = format!("doc{n:03}.wombat");
        index.insert_document(&doc, &Calc::new(&doc), &name);
        documents.push((name, doc));
    }
    let calcs: Vec<Calc> = documents.iter().map(|(_, doc)| Calc::new(doc)).collect();
    let named: Vec<naive::Named> = documents
        .iter()
        .zip(&calcs)
        .map(|((n, d), c)| (n.as_str(), d, c))
        .collect();
    let reference = naive::Reference::new(&named);
    let cells = index.cell_count();
    let mut worst = Duration::ZERO;
    for (what, text) in adversarial() {
        let parsed = parse(&text, NOW).unwrap();
        let options = Options {
            order: parsed.order.unwrap_or_default(),
            offset: 0,
            limit: 50,
        };
        let mut times = Vec::new();
        let mut total = 0;
        for _ in 0..5 {
            let (found, time) = timed(|| index.search(&parsed.query, &options).unwrap());
            total = found.total;
            times.push(time);
        }
        times.sort();
        println!(
            "adversarial: {:>10}  {total:>8} results  {what}",
            ms(times[times.len() / 2])
        );
        worst = worst.max(times[times.len() / 2]);
        // And they are answered rightly.
        let options = Options {
            limit: 10,
            ..options
        };
        let found = index.search(&parsed.query, &options).unwrap();
        assert_eq!(
            found,
            reference.search(&parsed.query, &options).unwrap(),
            "{what}"
        );
    }
    println!("adversarial: {cells} cells, worst {}", ms(worst));
    // None of them may be far from the target of 50 ms at 100,000 cells: before the
    // review's fixes the worst of these took half a second. (Three times the target,
    // because the machine is shared; ten times that in a debug build, on a fifth of
    // the cells.)
    let bound = Duration::from_millis(if SLOW { 1500 } else { 150 });
    assert!(worst < bound, "the worst took {worst:?}");
}

/// One very long cell: making a result must take time in proportion to the cell, not
/// to its square. (For every occurrence the end of the field was looked for again, to
/// the end of the text.)
#[test]
fn a_result_in_a_long_cell_takes_time_in_proportion_to_the_cell() {
    let time_for = |chars: usize| {
        let mut rng = corpus::Rng(77);
        let text: String = (0..chars)
            .map(|_| char::from(b'a' + rng.below(26) as u8))
            .collect();
        let mut ids = SeededIds::from_u64(2);
        let mut doc = Document::new(&mut ids);
        let root = doc.root();
        wombat_core::construct::Construct::cell_mut(&mut doc, root)
            .unwrap()
            .text = text;
        let mut index = SearchIndex::new();
        index.insert_document(&doc, &Calc::new(&doc), "long.wombat");
        let query = parse("q", NOW).unwrap().query;
        let mut best = Duration::MAX;
        let mut ranges = 0;
        for _ in 0..3 {
            let (found, time) = timed(|| index.search(&query, &Options::default()).unwrap());
            assert_eq!(found.total, 1);
            ranges = found.hits[0].ranges.len();
            best = best.min(time);
        }
        println!(
            "a long cell: {chars} characters: {} ({ranges} ranges returned)",
            ms(best)
        );
        best
    };
    let (small, large) = if SLOW {
        (50_000, 400_000)
    } else {
        (250_000, 2_000_000)
    };
    let (short, long) = (time_for(small), time_for(large));
    // Eight times the text: eight times the time if linear, sixty-four if quadratic.
    assert!(
        long < short * 24 + Duration::from_millis(5),
        "{small} characters took {short:?}, {large} took {long:?}"
    );
}

/// A document of `cells` cells of varied Chinese text: every cell `len` characters
/// drawn evenly from 3,000, so that nearly every pair of neighbours is a new one. (The
/// made-up text of the other tests repeats 40 syllables, which flatters the table.)
fn varied(cells: u32, len: usize, seed: u64) -> Document {
    let mut rng = corpus::Rng(seed);
    let mut ids = SeededIds::from_u64(seed);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    doc.add_grid(root, 1, cells, &mut ids).unwrap();
    for id in doc.cell(root).unwrap().grid().unwrap().cells().to_vec() {
        let text: String = (0..len)
            .map(|_| char::from_u32(0x4e00 + rng.below(3000) as u32).unwrap())
            .collect();
        wombat_core::construct::Construct::cell_mut(&mut doc, id)
            .unwrap()
            .text = text;
    }
    doc
}

/// Memory on text that is not made of a few syllables: random Chinese, and the real
/// Chinese, Japanese and Korean tutorials of TreeSheets. (A review measured 17 to 19
/// times the text on random Chinese, where the summary of the milestone said 8.5.)
#[test]
fn memory_on_varied_text() {
    let measure = |what: &str, docs: &[Document]| {
        let before = memory();
        let mut index = SearchIndex::new();
        for (n, doc) in docs.iter().enumerate() {
            index.insert_document(doc, &Calc::new(doc), &format!("d{n}.wombat"));
        }
        let after = memory();
        let (counted, text) = (index.memory_bytes(), index.text_bytes());
        let grew = match (before, after) {
            (Some((a, _)), Some((b, _))) => b.saturating_sub(a) * 1024,
            _ => 0,
        };
        let times = counted as f64 / text as f64;
        println!(
            "varied text: {what}: {} cells, {text} bytes of text, {counted} counted ({times:.1} times), the process grew by {grew} ({:.1} times)",
            index.cell_count(),
            grew as f64 / text as f64
        );
        index.check().unwrap();
        times
    };
    let cells = if SLOW { 2_000 } else { 5_000 };
    let short: Vec<Document> = (0..4).map(|n| varied(cells, 12, n + 1)).collect();
    let long: Vec<Document> = (0..4).map(|n| varied(cells / 10, 400, n + 9)).collect();
    let short_times = measure("random Chinese, 12 characters a cell", &short);
    let long_times = measure("random Chinese, 400 characters a cell", &long);
    // What a cell costs besides its text is the same for a short and a long one, so
    // short cells cost more per byte; the table itself is bounded by the text.
    assert!(short_times < 9.0, "{short_times}");
    assert!(long_times < 4.0, "{long_times}");
    // And every pair is still found through the table: a search for two characters
    // of one cell looks at few cells and finds it.
    let mut index = SearchIndex::new();
    index.insert_document(&short[0], &Calc::new(&short[0]), "d.wombat");
    let some = short[0].cells_in_order()[77];
    let text = &short[0].cell(some).unwrap().text;
    let pair: String = text.chars().skip(3).take(2).collect();
    let found = index
        .search(&parse(&pair, NOW).unwrap().query, &Options::default())
        .unwrap();
    assert!(found.hits.iter().any(|hit| hit.cell == some));
    // The real tutorials, where the files are at hand.
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../TS/examples");
    let mut real = Vec::new();
    for name in [
        "tutorial-zh_CN.cts",
        "tutorial-zh_TW.cts",
        "tutorial-ja.cts",
        "tutorial-ko.cts",
    ] {
        let mut ids = SeededIds::from_u64(5);
        if let Ok(imported) = wombat_cts::import_file(&examples.join(name), &mut ids) {
            real.push(imported.document);
        }
    }
    if !real.is_empty() {
        measure("the Chinese, Japanese and Korean tutorials", &real);
    }
}

/// One keystroke in a cell of a megabyte: what the index pays to follow. (It used to
/// sort every pair of characters of the old text and of the new: 25 to 60 ms.)
#[test]
fn an_edit_in_a_long_cell() {
    let size = if SLOW { 100_000 } else { 1_000_000 };
    let mut rng = corpus::Rng(5);
    let text: String = (0..size)
        .map(|_| char::from(b'a' + rng.below(26) as u8))
        .collect();
    let mut ids = SeededIds::from_u64(2);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    doc.add_grid(root, 1, 50, &mut ids).unwrap();
    let first = doc.cell(root).unwrap().grid().unwrap().cells()[0];
    wombat_core::construct::Construct::cell_mut(&mut doc, first)
        .unwrap()
        .text = text;
    let mut editor = Editor::new(doc).unwrap();
    let mut index = SearchIndex::new();
    index.insert_document(editor.document(), editor.calc(), "long.wombat");
    let (mut command_time, mut follow_time) = (Duration::MAX, Duration::MAX);
    for n in 0..8 {
        let command = Command::InsertText {
            cell: first,
            at: n * 1000,
            text: "日".into(),
        };
        let (changes, time) = timed(|| editor.apply(command, Some(NOW), &mut ids).unwrap());
        command_time = command_time.min(time);
        let (done, time) = timed(|| index.apply(editor.document(), editor.calc(), &changes));
        done.unwrap();
        follow_time = follow_time.min(time);
    }
    println!(
        "an edit in a long cell: {size} bytes: {} for the command, {} for the index to follow",
        ms(command_time),
        ms(follow_time)
    );
    index.check().unwrap();
    let mut fresh = SearchIndex::new();
    fresh.insert_document(editor.document(), editor.calc(), "long.wombat");
    assert!(index.encoded() == fresh.encoded());
    let query = parse("日", NOW).unwrap().query;
    assert_eq!(
        index.search(&query, &Options::default()).unwrap().hits[0].range_count,
        8
    );
    assert!(
        follow_time < Duration::from_millis(if SLOW { 200 } else { 20 }),
        "{follow_time:?}"
    );
}
