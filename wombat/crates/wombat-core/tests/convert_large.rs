//! A large document: 100,000 cells export in every format and import again within a set
//! time. The numbers are recorded in docs/milestones/M7.md.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::time::{Duration, Instant};

use wombat_core::construct::Construct;
use wombat_core::convert::lookup::{self, LookupOptions};
use wombat_core::convert::{
    decode, flat_csv, grid_csv, outline, parse_outline, parse_table, to_document, Delimiter,
    ExportOptions, Source,
};
use wombat_core::eval::Calc;
use wombat_core::limits::MAX_DEPTH;
use wombat_core::{Command, Document, Editor, SeededIds};

fn timed<T>(what: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = f();
    let taken = start.elapsed();
    // About ten times what the slowest step takes unoptimised on the development machine.
    // Slow machines pass; a step that is no longer linear does not.
    let limit = Duration::from_secs(if cfg!(debug_assertions) { 10 } else { 3 });
    println!("large export: {what}: {taken:?} (limit {limit:?})");
    assert!(taken < limit, "{what} took {taken:?}, over {limit:?}");
    result
}

#[test]
fn hundred_thousand_cells_export_and_import() {
    let mut ids = SeededIds::from_u64(100);
    // 1,000 projects of 4 columns; in the last column of each a table of 8 x 12 tasks:
    // 100,001 cells, 1,001 grids.
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    doc.add_grid(root, 4, 1000, &mut ids).unwrap();
    for (n, id) in doc
        .cell(root)
        .unwrap()
        .grid()
        .unwrap()
        .cells()
        .to_vec()
        .into_iter()
        .enumerate()
    {
        doc.cell_mut(id).unwrap().text = format!("Project {}, \"part\" {}", n / 4, n % 4);
        if n % 4 == 3 {
            doc.add_grid(id, 8, 12, &mut ids).unwrap();
            for (m, task) in doc
                .cell(id)
                .unwrap()
                .grid()
                .unwrap()
                .cells()
                .to_vec()
                .into_iter()
                .enumerate()
            {
                doc.cell_mut(task).unwrap().text =
                    format!("task {m} of project {} with some text", n / 4);
            }
        }
    }
    assert_eq!(doc.cell_count(), 100_001);
    let calc = timed("calculate", || Calc::new(&doc));
    let source = Source {
        doc: &doc,
        calc: &calc,
    };
    let options = ExportOptions::default();

    let csv = timed("one grid as CSV (4,000 cells)", || {
        grid_csv(source, root, &options).unwrap()
    });
    assert_eq!(csv.lines().count(), 1000);
    let flat = timed("flattened", || flat_csv(source, root, &options).unwrap());
    assert_eq!(flat.lines().count(), 12_000);
    println!("large export: flattened: {} bytes", flat.len());
    let text = timed("indented text", || {
        outline(source, root, true, false).unwrap()
    });
    assert_eq!(text.lines().count(), 100_001);
    let files = timed("lookup tables", || {
        lookup::export(source, root, &options, LookupOptions::default()).unwrap()
    });
    assert_eq!(files.len(), 1003);
    let bytes: usize = files.iter().map(|f| f.text.len()).sum();
    println!(
        "large export: lookup tables: {} files, {bytes} bytes",
        files.len()
    );

    let table = timed("read the flattened CSV (144,000 cells)", || {
        parse_table(
            &decode(flat.as_bytes(), None).unwrap().0,
            Delimiter::Comma,
            false,
        )
        .unwrap()
    });
    assert_eq!((table.xs(), table.ys()), (12, 12_000));
    let imported = timed("make a document of it", || {
        to_document(&table, &mut SeededIds::from_u64(1)).unwrap()
    });
    assert_eq!(imported.cell_count(), 144_001);
    let nested = timed("read the indented text", || {
        parse_outline(&text, MAX_DEPTH, false).unwrap()
    });
    // The root's own line is empty, and an empty line is no cell.
    assert_eq!(nested.cell_count(), 100_000);

    let mut editor = timed("open an editor", || Editor::new(doc).unwrap());
    let mut ids = SeededIds::from_u64(2);
    let target = editor.document().child(root, 0, 0).unwrap();
    let changes = timed("import the text into a cell (100,000 cells)", || {
        editor
            .apply(
                Command::Import {
                    cell: target,
                    table: nested,
                    replace: true,
                },
                Some(1),
                &mut ids,
            )
            .unwrap()
    });
    assert_eq!(changes.created.len(), 100_000);
    timed("undo that", || editor.undo().unwrap().unwrap());
    assert_eq!(editor.document().cell_count(), 100_001);
}
