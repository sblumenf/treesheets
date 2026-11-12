//! The stage read through an open document: the example as a screen would read it, what
//! is refused, that the bridge gives exactly what the core gives on any document, and what
//! the call costs on large ones.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

// The generator of commands that the core's own tests use, by its path.
#[path = "../../wombat-core/tests/support/mod.rs"]
mod support;

use std::time::{Duration, Instant};

use common::{folder, ids, open_example};
use proptest::prelude::*;
use support::{command, op, Op};
use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{self as w, CellKind, Command, ErrorKind, StageAsk};
use wombat_bridge::convert::{command_out, look_out, stage_ask_in, stage_out};
use wombat_core::construct::Construct;
use wombat_core::{storage, CellId, Document, Editor, SeededIds};

/// The core's starting values for a phone, as Dart would send them.
fn phone() -> StageAsk {
    StageAsk {
        x0: 0,
        y0: 0,
        cols: i64::from(u32::MAX),
        rows: i64::from(u32::MAX),
        full_depth: 1,
        full_cells: 24,
        preview_cols: 3,
        preview_rows: 4,
        shape_cols: 12,
        shape_rows: 20,
        text_cut: 2000,
        preview_cut: 40,
    }
}

/// The comment of `StageAsk` tells a Dart developer the core's starting values.
#[test]
fn the_starting_values_are_the_cores() {
    assert_eq!(
        stage_ask_in(phone()).unwrap(),
        wombat_core::StageAsk::default()
    );
}

fn rgb(r: i64, g: i64, b: i64) -> w::Rgb {
    w::Rgb { r, g, b }
}

#[test]
fn the_example_as_a_screen_reads_it() {
    let (doc, _) = open_example("stage", "example");
    let root = doc.root().unwrap();
    let stage = doc.stage(root.clone(), phone()).unwrap();

    assert_eq!(stage.trail.len(), 1);
    assert_eq!(stage.trail[0].id, root);
    assert_eq!(
        (stage.trail[0].shown.text.as_str(), stage.trail[0].shown.cut),
        ("Example", false)
    );

    // The root's grid, and the two grids nested in it, which are small enough to be
    // given cell by cell.
    let sizes: Vec<_> = stage
        .grids
        .iter()
        .map(|g| (g.level, g.xs, g.ys, g.cols, g.rows, g.first))
        .collect();
    assert_eq!(
        sizes,
        [(0, 2, 3, 2, 3, 0), (1, 1, 4, 1, 4, 6), (1, 1, 3, 1, 3, 10)]
    );
    assert_eq!(stage.grids[0].owner, root);
    assert_eq!(stage.grids[0].col_widths, [80, 80]);
    let texts: Vec<_> = stage.cells.iter().map(|c| c.text.as_str()).collect();
    let expected = [
        "Shopping",
        "what to buy",
        "Costs",
        "what it costs",
        "Total",
        "=SUM(Costs)",
        "Item",
        "Apples",
        "Bread",
        "Cheese",
        "3",
        "2",
        "5",
    ];
    assert_eq!(texts, expected);

    let (shopping, costs, total) = (&stage.cells[0], &stage.cells[2], &stage.cells[5]);
    assert_eq!(stage.grids[1].owner, shopping.id);
    assert_eq!(stage.grids[2].owner, costs.id);
    assert_eq!(
        (shopping.has_grid, shopping.summary, shopping.full),
        (true, Some(0), Some(1))
    );
    assert_eq!(
        (costs.has_grid, costs.summary, costs.full),
        (true, Some(1), Some(2))
    );
    assert_eq!(
        (total.has_grid, total.summary, total.full),
        (false, None, None)
    );
    assert_eq!(total.kind, CellKind::Formula);
    assert_eq!(
        (total.result.as_deref(), total.result_cut, total.failed),
        (Some("10"), false, false)
    );
    assert_eq!(shopping.result, None);
    assert_eq!(
        (shopping.cell_color, shopping.text_color),
        (rgb(255, 255, 255), rgb(0, 0, 0))
    );

    // The same two grids in reduced form.
    assert_eq!(stage.summaries.len(), 2);
    let summary = &stage.summaries[1];
    assert_eq!(summary.owner, costs.id);
    assert_eq!((summary.xs, summary.ys), (1, 3));
    assert_eq!((summary.preview_cols, summary.preview_rows), (1, 3));
    let preview: Vec<_> = summary.preview.iter().map(|t| t.text.as_str()).collect();
    assert_eq!(preview, ["3", "2", "5"]);
    assert_eq!((summary.shape_cols, summary.shape_rows), (1, 3));
    // Three white cells that each show one character.
    assert_eq!(summary.shape, [255, 255, 255, 1].repeat(3));

    // One level down the trail has two steps, and the text of a cell is cut where asked.
    let ask = StageAsk {
        text_cut: 3,
        preview_cut: 4,
        ..phone()
    };
    let below = doc.stage(shopping.id.clone(), ask).unwrap();
    let trail: Vec<_> = below
        .trail
        .iter()
        .map(|s| (s.id.as_str(), s.shown.text.as_str(), s.shown.cut))
        .collect();
    assert_eq!(
        trail,
        [
            (root.as_str(), "Exam", true),
            (shopping.id.as_str(), "Shop", true)
        ]
    );
    let cells: Vec<_> = below
        .cells
        .iter()
        .map(|c| (c.text.as_str(), c.text_cut))
        .collect();
    assert_eq!(
        cells,
        [("Ite", true), ("App", true), ("Bre", true), ("Che", true)]
    );
    assert!(below.summaries.is_empty());

    // A cell without a grid is a stage with a trail and nothing else.
    let leaf = doc.stage(total.id.clone(), phone()).unwrap();
    assert_eq!(leaf.trail.len(), 2);
    assert!(leaf.grids.is_empty() && leaf.cells.is_empty() && leaf.summaries.is_empty());

    // The window is the request's: the second row alone.
    let ask = StageAsk {
        y0: 1,
        rows: 1,
        full_depth: 0,
        ..phone()
    };
    let row = doc.stage(root, ask).unwrap();
    let texts: Vec<_> = row.cells.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(texts, ["Costs", "what it costs"]);
    assert_eq!(
        (row.grids.len(), row.grids[0].y0, row.grids[0].rows),
        (1, 1, 1)
    );
}

/// What an editor opens on: the whole text with its styles, whatever the stage cut.
#[test]
fn one_cell_is_read_whole() {
    let (mut doc, _) = open_example("stage", "cell");
    let root = doc.root().unwrap();
    let cell = doc.grid(root.clone()).unwrap().cells[1].id.clone();
    let bold = w::Style {
        bold: true,
        ..w::Style::default()
    };
    doc.apply(Command::SetRangeStyle {
        cell: cell.clone(),
        start: 5,
        len: 2,
        set: bold,
        clear: w::Style::default(),
    })
    .unwrap();
    let link = w::LinkTarget::Cell {
        doc: None,
        cell: root.clone(),
    };
    doc.apply(Command::SetLink {
        cell: cell.clone(),
        start: 8,
        len: 3,
        target: Some(link.clone()),
    })
    .unwrap();
    let look = doc.cell(cell.clone()).unwrap();
    assert_eq!(
        (look.id.as_str(), look.text.as_str(), look.text_cut),
        (cell.as_str(), "what to buy", false)
    );
    let run = w::RunInfo {
        start: 5,
        len: 2,
        style: bold,
        color: None,
    };
    assert_eq!(look.runs, [run]);
    let link = w::LinkInfo {
        start: 8,
        len: 3,
        target: link,
    };
    assert_eq!(look.links, [link]);
    assert_eq!(
        (look.summary, look.full, look.has_grid),
        (None, None, false)
    );
    // The root has a grid, and says so, with no place in lists that are not there.
    let look = doc.cell(root).unwrap();
    assert_eq!((look.summary, look.full, look.has_grid), (None, None, true));
}

#[test]
fn what_a_read_refuses() {
    let (mut doc, _) = open_example("stage", "refused");
    let root = doc.root().unwrap();
    let kind = |result: Result<w::StageInfo, w::BridgeError>| result.unwrap_err().kind;
    assert_eq!(
        kind(doc.stage("no ID".into(), phone())),
        ErrorKind::BadRequest
    );
    let nobody = support::no_such_cell().to_string();
    assert_eq!(
        kind(doc.stage(nobody.clone(), phone())),
        ErrorKind::NotFound
    );
    let ask = StageAsk {
        rows: -1,
        ..phone()
    };
    assert_eq!(kind(doc.stage(root.clone(), ask)), ErrorKind::BadRequest);
    assert_eq!(
        doc.cell("no ID".into()).unwrap_err().kind,
        ErrorKind::BadRequest
    );
    assert_eq!(doc.cell(nobody).unwrap_err().kind, ErrorKind::NotFound);
    // Reading changes nothing.
    assert!(doc.is_saved().unwrap());
    doc.close().unwrap();
    assert_eq!(kind(doc.stage(root.clone(), phone())), ErrorKind::Closed);
    assert_eq!(doc.cell(root).unwrap_err().kind, ErrorKind::Closed);
}

/// A request made of small numbers, with now and then the largest there is.
fn ask() -> impl Strategy<Value = StageAsk> {
    let n = || prop_oneof![8 => 0i64..6, 1 => Just(i64::from(u32::MAX)), 1 => 0i64..60];
    (
        (n(), n(), n(), n()),
        (0i64..4, n(), n(), n()),
        (n(), n(), n(), n()),
    )
        .prop_map(|(a, b, c)| StageAsk {
            x0: a.0,
            y0: a.1,
            cols: a.2,
            rows: a.3,
            full_depth: b.0,
            full_cells: b.1,
            preview_cols: b.2,
            preview_rows: b.3,
            shape_cols: c.0,
            shape_rows: c.1,
            text_cut: c.2,
            preview_cut: c.3,
        })
}

/// The same commands to an open document and to an editor of the core beside it, and
/// then the same reads of both.
fn read_both(ops: &[Op], asks: &[(StageAsk, u32)], seed: u64) {
    let path = folder("stage", "random").join("document.wombat");
    std::fs::copy(common::example(), &path).unwrap();
    let mut bridge = OpenDocument::open_with(&path, SeededIds::from_u64(seed)).unwrap();
    bridge.set_fixed_time(Some(1));
    let mut ids = SeededIds::from_u64(seed);
    let mut shadow = Editor::new(storage::load_document(&path).unwrap()).unwrap();
    for op in ops {
        let core = command(shadow.document(), op);
        let wire = command_out(&core).unwrap();
        let (got, want) = (bridge.apply(wire), shadow.apply(core, Some(1), &mut ids));
        assert_eq!(got.is_ok(), want.is_ok());
    }
    let cells = shadow.document().cells_in_order();
    for (ask, pick) in asks {
        let id = cells[*pick as usize % cells.len()];
        let core = stage_ask_in(*ask).unwrap();
        let want = stage_out(shadow.stage_view(id, &core).unwrap());
        assert_eq!(bridge.stage(id.to_string(), *ask).unwrap(), want, "{ask:?}");
        let want = look_out(shadow.cell_look(id).unwrap());
        assert_eq!(bridge.cell(id.to_string()).unwrap(), want);
    }
    drop(bridge);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, ..ProptestConfig::default() })]

    /// Whatever the document and the request, the bridge gives what the core gives, in
    /// the carried form: it asks for the cell and the request it was given, and adds and
    /// loses nothing.
    #[test]
    fn a_read_through_the_bridge_is_the_cores(
        ops in proptest::collection::vec(op(), 0..30),
        asks in proptest::collection::vec((ask(), any::<u32>()), 1..6),
        seed in any::<u64>(),
    ) {
        read_both(&ops, &asks, seed);
    }
}

// ---- What it costs ----

/// Gives `cell` a grid whose cells have the texts `text(x, y)`.
fn grid(
    doc: &mut Document,
    ids: &mut SeededIds,
    cell: CellId,
    (xs, ys): (u32, u32),
    text: impl Fn(u32, u32) -> String,
) {
    doc.add_grid(cell, xs, ys, ids).unwrap();
    for y in 0..ys {
        for x in 0..xs {
            let id = doc.child(cell, x, y).unwrap();
            doc.cell_mut(id).unwrap().text = text(x, y);
        }
    }
}

/// The 100,000-cell document of the core's own measurement (`wombat-core/tests/stage.rs`):
/// 10 by 100 cells, each holding 10 by 10.
fn hundred_thousand() -> Document {
    let mut ids = SeededIds::from_u64(100);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    grid(&mut doc, &mut ids, root, (10, 100), |x, y| {
        format!("Section {}", y * 10 + x)
    });
    for n in 0..1000 {
        let cell = doc.child(root, n % 10, n / 10).unwrap();
        let text = |x, y| format!("Item {n}.{} with some ordinary text in it", y * 10 + x);
        grid(&mut doc, &mut ids, cell, (10, 10), text);
    }
    assert_eq!(doc.cell_count(), 101_001);
    doc
}

/// The six-level document of the core's own measurement: the root's grid and one grid on
/// each level 12 by 20, a third of the cells of every grid holding a grid, smaller ones
/// further down.
fn six_levels() -> Document {
    let mut ids = SeededIds::from_u64(60);
    let mut doc = Document::new(&mut ids);
    let sizes = [(12, 20), (4, 5), (3, 4), (2, 3), (2, 2), (1, 2)];
    let mut level = vec![doc.root()];
    for (depth, size) in sizes.iter().enumerate() {
        let mut next = Vec::new();
        for (n, cell) in level.iter().enumerate() {
            let (xs, ys) = if n == 0 { (12, 20) } else { *size };
            let text = |x, y| format!("Level {depth} cell {x}.{y} text");
            grid(&mut doc, &mut ids, *cell, (xs, ys), text);
            for m in (0..xs * ys).step_by(3) {
                next.push(doc.child(*cell, m % xs, m / xs).unwrap());
            }
        }
        level = next;
    }
    assert_eq!(doc.cell_count(), 68_749);
    doc
}

/// The middle of five timings of `work`.
fn middle<T>(mut work: impl FnMut() -> T) -> (Duration, T) {
    let mut times = Vec::new();
    let mut last = None;
    for _ in 0..5 {
        let start = Instant::now();
        last = Some(work());
        times.push(start.elapsed());
    }
    times.sort();
    (times[2], last.unwrap())
}

/// Times the whole call of the bridge's Rust side, and the core's part of it alone on
/// the same document, and prints both (`--nocapture`).
fn timed(what: &str, doc: &OpenDocument, editor: &Editor, ask: StageAsk) -> Duration {
    let root = editor.document().root();
    let core = stage_ask_in(ask).unwrap();
    let (alone, view) = middle(|| editor.stage_view(root, &core).unwrap());
    let (whole, stage) = middle(|| doc.stage(root.to_string(), ask).unwrap());
    assert_eq!(stage, stage_out(view));
    let shapes: usize = stage.summaries.iter().map(|s| s.shape.len() / 4).sum();
    println!(
        "stage: {what}: {whole:?} through the bridge, {alone:?} the core alone \
         ({} cells, {} summaries, {shapes} shape cells)",
        stage.cells.len(),
        stage.summaries.len(),
    );
    whole
}

/// Measures the stage read through the bridge on the documents the core measured its own
/// part on. The limits are those of the core's test, at least ten times what the
/// development machine takes: a slow or busy machine passes, a conversion that has become
/// the larger part does not.
#[test]
fn large_stages_are_read_within_a_set_time() {
    let limit = |fast: u64| {
        Duration::from_millis(if cfg!(debug_assertions) {
            fast * 10
        } else {
            fast
        })
    };
    let dir = folder("stage", "large");
    let open = |name: &str, doc: Document| {
        let path = dir.join(name);
        storage::save_document(&doc, &path).unwrap();
        let editor = Editor::new(doc).unwrap();
        (OpenDocument::open_with(&path, ids()).unwrap(), editor)
    };

    let (doc, editor) = open("hundred-thousand.wombat", hundred_thousand());
    let all = timed("100,000 cells, the whole root", &doc, &editor, phone());
    assert!(all < limit(400), "{all:?}");
    let window = StageAsk {
        y0: 30,
        rows: 40,
        ..phone()
    };
    let rows = timed("100,000 cells, 40 rows of the root", &doc, &editor, window);
    assert!(rows < limit(160), "{rows:?}");

    let (doc, editor) = open("six-levels.wombat", six_levels());
    let top = timed("six levels, the root", &doc, &editor, phone());
    assert!(top < limit(120), "{top:?}");
    let two = StageAsk {
        full_depth: 2,
        ..phone()
    };
    let more = timed(
        "six levels, the root, two levels cell by cell",
        &doc,
        &editor,
        two,
    );
    assert!(more < limit(200), "{more:?}");
}
