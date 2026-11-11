//! The link index at size: a workspace of 200 documents and 200,000 cells, and one edit in
//! a document of 100,000 cells. The numbers are printed (and recorded in
//! `docs/milestones/M4.md`); the bounds are generous, to catch a change of kind, not of
//! degree. One test only, so that the memory read from the system is this test's.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use wombat_core::construct::Construct;
use wombat_core::links::DocLinks;
use wombat_core::scan;
use wombat_core::{
    CellId, Command, DocId, Document, Editor, Id, Link, LinkIndex, LinkTarget, SeededIds, Workspace,
};

/// (resident memory now, its peak) of this process in KiB, where the system tells.
fn memory() -> Option<(u64, u64)> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let field = |name: &str| {
        let line = status.lines().find(|line| line.starts_with(name))?;
        line.split_whitespace().nth(1)?.parse::<u64>().ok()
    };
    Some((field("VmRSS:")?, field("VmHWM:")?))
}

/// A document of `outer` cells with a grid of ten cells each. One cell in ten has a link:
/// to a cell of the same document, or (one in three of those) of the document `next`.
fn document(seed: u64, id: DocId, next: DocId, outer: u32) -> Document {
    // IDs are made by hand so that a document can link to cells of the next one.
    let cell_id = |doc: DocId, n: u32| CellId(Id::from_u128(doc.0.as_u128() + 1 + u128::from(n)));
    struct Counted(u128);
    impl wombat_core::IdSource for Counted {
        fn next_id(&mut self) -> Id {
            self.0 += 1;
            Id::from_u128(self.0 - 1)
        }
    }
    let mut ids = Counted(id.0.as_u128());
    let mut doc = Document::new(&mut ids);
    assert_eq!(doc.id(), id);
    let root = doc.root();
    doc.add_grid(root, 1, outer, &mut ids).unwrap();
    let rows = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
    let total = 1 + outer * 11;
    for (n, row) in rows.iter().enumerate() {
        doc.cell_mut(*row).unwrap().text = format!("Section {n}");
        doc.add_grid(*row, 2, 5, &mut ids).unwrap();
        let inner = doc.cell(*row).unwrap().grid().unwrap().cells().to_vec();
        for (m, leaf) in inner.iter().enumerate() {
            let c = doc.cell_mut(*leaf).unwrap();
            c.text = format!("Item {n}.{m} with some ordinary text in it");
            if m == 3 {
                let pick = (seed as u32).wrapping_mul(7919).wrapping_add(n as u32 * 31) % total;
                let across = n % 3 == 0;
                c.links.push(Link {
                    start: 5,
                    len: 3,
                    target: LinkTarget::Cell {
                        doc: (across && next != id).then_some(next),
                        cell: cell_id(if across { next } else { id }, pick),
                    },
                    extra: Default::default(),
                });
            }
        }
    }
    assert_eq!(doc.cell_count() as u32, total);
    doc
}

fn doc_id(n: u64) -> DocId {
    DocId(Id::from_u128(u128::from(n + 1) << 64))
}

fn timed<T>(what: &str, limit: Duration, f: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let result = f();
    let taken = start.elapsed();
    println!("links at size: {what}: {taken:?} (limit {limit:?})");
    assert!(taken < limit, "{what} took {taken:?}, over {limit:?}");
    (result, taken)
}

/// The time of `rounds` edits of a linked cell with the index following, per edit, and of
/// the index's part of it alone.
fn edit_cost(doc: Document, rounds: u32) -> (Duration, Duration) {
    let mut index = LinkIndex::new();
    index.insert_document(&doc);
    let cells = doc.cells_in_order();
    let linked: Vec<CellId> = cells
        .iter()
        .copied()
        .filter(|c| !doc.cell(*c).unwrap().links.is_empty())
        .collect();
    let mut editor = Editor::new(doc).unwrap();
    let mut ids = SeededIds::from_u64(1);
    let (mut all, mut follow) = (Duration::ZERO, Duration::ZERO);
    for round in 0..rounds {
        let cell = linked[(round as usize * 7919) % linked.len()];
        let command = Command::InsertText {
            cell,
            at: 6,
            text: "x".into(),
        };
        let start = Instant::now();
        let changes = editor.apply(command, None, &mut ids).unwrap();
        let middle = Instant::now();
        index.apply(editor.document(), &changes);
        let end = Instant::now();
        all += end - start;
        follow += end - middle;
    }
    // It did follow: the index is the one a rebuild gives.
    let mut fresh = LinkIndex::new();
    fresh.insert_document(editor.document());
    assert!(index == fresh);
    (all / rounds, follow / rounds)
}

#[test]
fn two_hundred_documents_and_one_edit_in_a_large_one() {
    const DOCS: u64 = 200;
    const OUTER: u32 = 91; // 1 + 91 * 11 = 1,002 cells per document
    let slow = cfg!(debug_assertions);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("links_large");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let ws = Workspace::open(&dir).unwrap();
    let mut bytes = 0;
    for n in 0..DOCS {
        let doc = document(n, doc_id(n), doc_id((n + 1) % DOCS), OUTER);
        let name = format!("doc{n:03}.wombat");
        ws.save(&doc, &name).unwrap();
        bytes += fs::metadata(dir.join(&name)).unwrap().len();
    }
    let cells = DOCS * (1 + u64::from(OUTER) * 11);
    println!("links at size: {DOCS} documents, {cells} cells, {bytes} bytes of files");

    let before = memory();
    let limit = Duration::from_secs(if slow { 120 } else { 30 });
    let (first, cold) = timed("build, reading every document", limit, || {
        scan::link_index(&ws, scan::CacheMode::Verify).unwrap()
    });
    let after = memory();
    assert_eq!((first.parsed, first.reused), (DOCS as usize, 0));
    assert!(first.cache_written && first.failed.is_empty());
    assert_eq!(first.index.documents().count() as u64, DOCS);
    let links: usize = first.index.documents().map(|d| d.links().count()).sum();
    assert_eq!(links as u64, DOCS * u64::from(OUTER));
    // Every link resolves: the targets were made to exist.
    let (broken, _) = timed("resolve every link", limit, || first.index.broken().len());
    assert_eq!(broken, 0);
    if let (Some((rss0, _)), Some((rss1, peak))) = (before, after) {
        // The index stays; the documents were read one at a time and are gone.
        let kept = rss1.saturating_sub(rss0);
        println!(
            "links at size: memory: {rss0} KiB before, {rss1} KiB after (the index: about \
             {kept} KiB), peak {peak} KiB"
        );
        // 200,000 cells: under 100 bytes a cell for the index, and a peak that is far
        // from the 600 MiB or so that holding all documents in memory would take.
        assert!(kept < 64 * 1024, "the index takes {kept} KiB");
        assert!(peak.saturating_sub(rss0) < 160 * 1024, "peak {peak} KiB");
    }
    let cache: u64 = fs::read_dir(scan::cache_dir::<DocLinks>(&ws))
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum();
    println!("links at size: cache files: {cache} bytes");

    let (second, warm) = timed("build again, from the cache", limit, || {
        scan::link_index(&ws, scan::CacheMode::Verify).unwrap()
    });
    assert_eq!((second.parsed, second.reused), (0, DOCS as usize));
    assert!(second.index == first.index);
    assert!(
        warm < cold,
        "the cache did not help: {warm:?} against {cold:?}"
    );
    // One document changes: one is parsed.
    let changed = document(999, doc_id(7), doc_id(8), OUTER);
    ws.save(&changed, "doc007.wombat").unwrap();
    let (third, _) = timed("build again after one document changed", limit, || {
        scan::link_index(&ws, scan::CacheMode::Verify).unwrap()
    });
    assert_eq!((third.parsed, third.reused), (1, DOCS as usize - 1));
    let (plain, _) = timed("build without the cache", limit, || {
        scan::link_index(&ws, scan::CacheMode::Off).unwrap()
    });
    assert!(plain.index == third.index);
    let target = third.index.outgoing(doc_id(0))[0].clone();
    let (found, lookup) = timed("backlinks of one cell", Duration::from_secs(1), || {
        let wombat_core::links::Resolved::Cell { doc, cell, .. } = target.resolved else {
            panic!("not resolved")
        };
        third.index.backlinks(doc, cell).len()
    });
    assert!(found >= 1 && lookup < Duration::from_millis(50));
    drop((first, second, third, plain));
    fs::remove_dir_all(&dir).unwrap();

    // One edit: the cost of following it does not depend on the size of the document.
    let rounds = 2000;
    let small = edit_cost(document(1, doc_id(300), doc_id(300), 91), rounds);
    let large = edit_cost(document(2, doc_id(301), doc_id(301), 9091), rounds);
    println!(
        "links at size: one edit with the index following: {:?} in 1,002 cells, {:?} in \
         100,002 cells; the index's part: {:?} and {:?}",
        small.0, large.0, small.1, large.1
    );
    // A hundred times the cells; a rescan would take a hundred times as long (about 30 ms
    // here). Memory that no longer fits the processor's cache makes it a few times slower.
    assert!(
        large.1 < small.1 * 10 + Duration::from_micros(20),
        "following an edit depends on the document's size: {:?} against {:?}",
        large.1,
        small.1
    );
    assert!(large.1 < Duration::from_micros(if slow { 500 } else { 100 }));
}
