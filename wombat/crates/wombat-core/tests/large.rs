//! A large document: 100,000 cells save and load within a set time.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::Path;
use std::time::{Duration, Instant};

use wombat_core::{format, storage, Document, Link, LinkTarget, Run, SeededIds, Style};

fn timed<T>(what: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = f();
    let taken = start.elapsed();
    // About ten times what the slowest step (loading) takes on the development machine,
    // recorded in docs/milestones/M1.md: 1.1 s unoptimised, 0.4 s optimised. Slow machines
    // pass; a step that has become several times slower, or no longer linear, does not.
    let limit = Duration::from_secs(if cfg!(debug_assertions) { 10 } else { 4 });
    println!("large document: {what}: {taken:?} (limit {limit:?})");
    assert!(taken < limit, "{what} took {taken:?}, over {limit:?}");
    result
}

#[test]
fn hundred_thousand_cells_save_and_load() {
    let mut ids = SeededIds::from_u64(100);
    let doc = timed("build", || {
        let mut doc = Document::new(&mut ids);
        let root = doc.root();
        // 10 x 100 cells, each holding a 10 x 10 grid: 101,001 cells, three levels.
        doc.add_grid(root, 10, 100, &mut ids).unwrap();
        let outer = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
        for (n, cell) in outer.iter().enumerate() {
            doc.cell_mut(*cell).unwrap().text = format!("Section {n}");
            doc.add_grid(*cell, 10, 10, &mut ids).unwrap();
            let inner = doc.cell(*cell).unwrap().grid().unwrap().cells().to_vec();
            for (m, leaf) in inner.iter().enumerate() {
                let c = doc.cell_mut(*leaf).unwrap();
                c.text = format!("Item {n}.{m} with some ordinary text in it");
                c.last_edit = Some(1_700_000_000_000 + (n * 100 + m) as i64);
                if m % 10 == 0 {
                    c.runs.push(Run {
                        start: 0,
                        len: 4,
                        style: Style {
                            bold: true,
                            ..Style::default()
                        },
                        color: None,
                        extra: Default::default(),
                    });
                    c.links.push(Link {
                        start: 5,
                        len: 3,
                        target: LinkTarget::Cell {
                            doc: None,
                            cell: root,
                        },
                        extra: Default::default(),
                    });
                }
            }
        }
        doc
    });
    assert_eq!(doc.cell_count(), 101_001);

    let text = timed("save to text", || format::to_string(&doc).unwrap());
    println!("large document: {} bytes", text.len());
    // The size is part of the format's promise of one compact line per cell.
    assert!(text.len() < 150 * doc.cell_count(), "{} bytes", text.len());
    let loaded = timed("load from text", || {
        format::from_slice(text.as_bytes()).unwrap()
    });
    assert!(timed("compare", || loaded == doc));
    assert_eq!(format::to_string(&loaded).unwrap(), text);

    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("large");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("large.wombat");
    timed("save to file", || {
        storage::save_document(&doc, &path).unwrap()
    });
    let from_file = timed("load from file", || storage::load_document(&path).unwrap());
    assert_eq!(from_file.cell_count(), 101_001);
    // Finding the ID reads the header only: a thousandth of the time of loading.
    let started = Instant::now();
    assert_eq!(storage::document_id(&path).unwrap(), doc.id());
    let taken = started.elapsed();
    println!("large document: read the ID from the file: {taken:?}");
    assert!(
        taken < Duration::from_millis(50),
        "reading the ID took {taken:?}"
    );
    let copy = timed("copy with fresh IDs", || {
        doc.with_fresh_ids(&mut ids).unwrap().0
    });
    assert_eq!(copy.cell_count(), 101_001);
    // Lookup by ID and of the parent does not depend on the size of the document.
    let order = timed("walk in order", || loaded.cells_in_order());
    assert_eq!(order.len(), 101_001);
    let last = *order.last().unwrap();
    assert_eq!(loaded.depth(last), Some(2));
    assert!(loaded.cell(last).unwrap().parent().is_some());
}
