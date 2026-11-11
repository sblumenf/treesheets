//! Search and formulas together (milestones 5 and 6), by example: a formula cell is found
//! by its formula and by the result it shows, the index follows results as they change,
//! and replace works on formulas, never on results. Every search here is also held to
//! the reference scanner and to an index built from scratch.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "support/mod.rs"]
mod search_support;

use std::fs;
use std::path::Path;

use search_support::check_hit;
use wombat_core::construct::Construct;
use wombat_core::eval::Calc;
use wombat_core::scan::{Cache, CacheMode};
use wombat_core::{
    format, CellId, CellKind, Changes, Command, Document, Editor, Link, LinkTarget, SeededIds,
    Workspace,
};
use wombat_search::query::{parse, Term};
use wombat_search::replace::{replace, ReplaceOptions, Scope};
use wombat_search::{naive, workspace, Field, Hit, Options, Order, SearchIndex};

/// An editor with a search index that follows it.
struct Desk {
    editor: Editor,
    index: SearchIndex,
    ids: SeededIds,
}

impl Desk {
    fn new(doc: Document) -> Desk {
        let editor = Editor::new(doc).unwrap();
        let mut index = SearchIndex::new();
        index.insert_document(editor.document(), editor.calc(), "budget.wombat");
        Desk {
            editor,
            index,
            ids: SeededIds::from_u64(77),
        }
    }

    fn follow(&mut self, changes: &Changes) {
        self.index
            .apply(self.editor.document(), self.editor.calc(), changes)
            .unwrap();
        // The index that followed is the index of the document and its results now.
        let mut fresh = SearchIndex::new();
        let (doc, calc) = (self.editor.document(), self.editor.calc());
        fresh.insert_document(doc, calc, "budget.wombat");
        assert!(self.index.encoded() == fresh.encoded(), "the index drifted");
        self.index.check().unwrap();
    }

    fn apply(&mut self, command: Command) -> Changes {
        let changes = self.editor.apply(command, None, &mut self.ids).unwrap();
        self.follow(&changes);
        changes
    }

    fn undo(&mut self) {
        let changes = self.editor.undo().unwrap().unwrap();
        self.follow(&changes);
    }

    fn redo(&mut self) {
        let changes = self.editor.redo().unwrap().unwrap();
        self.follow(&changes);
    }

    fn enter(&mut self, cell: CellId, text: &str) {
        let text = text.to_owned();
        self.apply(Command::EnterText { cell, text });
    }

    /// Searches, in the order of the document; the reference must say the same.
    fn find(&self, text: &str) -> Vec<Hit> {
        let query = parse(text, 0).unwrap().query;
        let options = Options::all(Order::Document);
        let found = self.index.search(&query, &options).unwrap();
        let docs = [("budget.wombat", self.editor.document(), self.editor.calc())];
        assert_eq!(found, naive::search(&docs, &query, &options).unwrap());
        for hit in &found.hits {
            check_hit(hit, &docs);
        }
        found.hits
    }

    /// The cells found, as (text, field).
    fn found(&self, text: &str) -> Vec<(String, Field)> {
        let hits = self.find(text).into_iter();
        hits.map(|hit| (hit.text, hit.field)).collect()
    }
}

struct Budget {
    doc: Document,
    rent: CellId,
    total: CellId,
    note: CellId,
    program: Vec<CellId>,
    /// A formula that reads the variable of the program.
    reads: CellId,
}

/// ```text
/// Budget
///   Costs
///     Rent | 800
///     Food | 250.5
///   =SUM(Costs)
///   see the total
///   (a grid:) 2 | + | 3 | old view (a view) | five (assigns a variable)
///   =five + 40
/// ```
/// with a link from "total" to the first formula cell.
fn budget() -> Budget {
    let mut ids = SeededIds::from_u64(5);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    doc.cell_mut(root).unwrap().text = "Budget".into();
    doc.add_grid(root, 1, 5, &mut ids).unwrap();
    let rows = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
    let set = |doc: &mut Document, cell: CellId, text: &str| {
        doc.cell_mut(cell).unwrap().text = text.into();
    };
    set(&mut doc, rows[0], "Costs");
    doc.add_grid(rows[0], 2, 2, &mut ids).unwrap();
    let costs = doc.cell(rows[0]).unwrap().grid().unwrap().cells().to_vec();
    for (cell, text) in costs.iter().zip(["Rent", "800", "Food", "250.5"]) {
        set(&mut doc, *cell, text);
    }
    set(&mut doc, rows[1], "=SUM(Costs)");
    doc.cell_mut(rows[1]).unwrap().kind = CellKind::Formula;
    set(&mut doc, rows[2], "see the total");
    doc.cell_mut(rows[2]).unwrap().links.push(Link {
        start: 8,
        len: 5,
        target: LinkTarget::Cell {
            doc: None,
            cell: rows[1],
        },
        extra: Default::default(),
    });
    doc.add_grid(rows[3], 5, 1, &mut ids).unwrap();
    let program = doc.cell(rows[3]).unwrap().grid().unwrap().cells().to_vec();
    for (cell, text) in program.iter().zip(["2", "+", "3", "old view", "five"]) {
        set(&mut doc, *cell, text);
    }
    doc.cell_mut(program[1]).unwrap().kind = CellKind::Code;
    doc.cell_mut(program[3]).unwrap().kind = CellKind::ViewH;
    doc.cell_mut(program[4]).unwrap().kind = CellKind::VarAssign;
    set(&mut doc, rows[4], "=five + 40");
    doc.cell_mut(rows[4]).unwrap().kind = CellKind::Formula;
    doc.validate().unwrap();
    Budget {
        rent: costs[1],
        total: rows[1],
        note: rows[2],
        reads: rows[4],
        program,
        doc,
    }
}

#[test]
fn a_computed_value_is_found_and_so_is_its_formula() {
    let b = budget();
    let desk = Desk::new(b.doc);
    // The result is in no cell's text: it is found in what the formula cell shows, and
    // in the cell that links to it, which goes by what its target shows.
    let hits = desk.find("1050.5");
    assert_eq!(hits.len(), 2);
    assert_eq!((hits[0].cell, hits[0].field), (b.total, Field::Shown));
    assert_eq!(hits[0].text, "=SUM(Costs)");
    assert_eq!(hits[0].shown.as_deref(), Some("1050.5"));
    assert_eq!((hits[0].matched.as_str(), hits[0].score), ("1050.5", 4));
    assert_eq!(hits[0].ranges, [(0, 6)]);
    assert_eq!((hits[1].cell, hits[1].field), (b.note, Field::LinkedText));
    assert_eq!(hits[1].matched, "1050.5");
    assert_eq!(hits[1].shown, None);
    // The formula is searched as the text it is.
    let found = desk.found("sum");
    assert_eq!(found, [("=SUM(Costs)".to_owned(), Field::Text)]);
    // "Costs" is the table and the formula that names it; the link is not found by
    // its target's formula.
    assert_eq!(desk.find("costs").len(), 2);
    assert_eq!(desk.find("kind:formula 1050").len(), 1);
    assert_eq!(desk.find("case:SUM").len(), 1);
    assert_eq!(desk.find("case:1050.5").len(), 2);
}

#[test]
fn a_changed_number_changes_what_is_found_and_undo_changes_it_back() {
    let b = budget();
    let mut desk = Desk::new(b.doc);
    let changes = desk.apply(Command::SetText {
        cell: b.rent,
        text: "300".into(),
    });
    // The formula cell did not change, so the report does not name it; its result did.
    assert_eq!(changes.changed, [b.rent]);
    assert_eq!(desk.editor.recalculated(), [b.total]);
    assert!(desk.find("1050.5").is_empty(), "the old result is gone");
    let hits = desk.find("550.5");
    assert_eq!(hits.len(), 2);
    assert_eq!((hits[0].cell, hits[0].field), (b.total, Field::Shown));
    assert_eq!(hits[1].field, Field::LinkedText);
    desk.undo();
    assert!(desk.find("550.5").is_empty());
    assert_eq!(desk.find("1050.5").len(), 2);
    desk.redo();
    assert_eq!(desk.find("550.5").len(), 2);
    // A new row in the table: everything is worked out again, and the index follows.
    let costs = desk.editor.document().cell(b.rent).unwrap().parent();
    desk.apply(Command::InsertRows {
        cell: costs.unwrap(),
        at: 2,
        count: 1,
    });
    let grid = desk.editor.document().cell(costs.unwrap()).unwrap();
    let new = grid.grid().unwrap().cells()[5];
    desk.enter(new, "1000");
    assert_eq!(desk.find("1550.5").len(), 2);
    // A group of commands is followed from its one report.
    desk.editor.begin_group();
    for text in ["1", "2"] {
        let (cell, text) = (b.rent, text.to_owned());
        let set = Command::SetText { cell, text };
        desk.editor.apply(set, None, &mut desk.ids).unwrap();
    }
    let changes = desk.editor.end_group();
    desk.follow(&changes);
    assert_eq!(desk.find("1252.5").len(), 2);
    assert!(desk.find("1550.5").is_empty());
}

#[test]
fn becoming_and_ceasing_to_be_a_formula_cell_is_followed() {
    let b = budget();
    let mut desk = Desk::new(b.doc);
    // A cell that shows an error is found by it; a trail is what the cells show.
    desk.enter(b.rent, "=SUM(Nowhere)");
    let hits = desk.find("#NAME?");
    assert_eq!(hits[0].cell, b.rent);
    assert_eq!(hits[0].field, Field::Shown);
    assert_eq!(hits[0].trail, ["Budget", "Costs"]);
    // A formula cell with a grid: the cells inside it are shown under its result.
    let costs = desk.editor.document().cell(b.rent).unwrap().parent();
    desk.enter(costs.unwrap(), "='All ' & 'costs'");
    let hits = desk.find("food");
    assert_eq!(hits[0].trail, ["Budget", "All costs"]);
    desk.undo();
    assert_eq!(desk.find("food")[0].trail, ["Budget", "Costs"]);
    // Typing a number over the formula: it shows nothing else than its text any more.
    desk.enter(b.total, "12");
    assert!(desk.find("sum").iter().all(|hit| hit.cell != b.total));
    // The link now leads to a plain cell and is found by that cell's text.
    let found = desk.found("12");
    let link = ("see the total".to_owned(), Field::LinkedText);
    assert_eq!(found, [("12".to_owned(), Field::Text), link]);
}

#[test]
fn a_run_changes_view_cells_and_the_index_follows() {
    let b = budget();
    let mut desk = Desk::new(b.doc);
    let view = b.program[3];
    assert_eq!(desk.find("\"old view\"").len(), 1);
    // A formula that reads a variable of the program shows #N/A until the program ran.
    assert_eq!(desk.find("#N/A")[0].cell, b.reads);
    let changes = desk.apply(Command::Run);
    assert!(changes.changed.contains(&view));
    assert!(desk.find("\"old view\"").is_empty());
    assert_eq!(desk.editor.shown(view), Some("5"));
    let hits = desk.find("kind:view_h 5");
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].cell, hits[0].field), (view, Field::Text));
    // The formula was not changed by the run, and the report does not name it; what
    // it shows was.
    assert!(!changes.changed.contains(&b.reads));
    assert!(desk.find("#N/A").is_empty());
    let hits = desk.find("45");
    assert_eq!((hits[0].cell, hits[0].field), (b.reads, Field::Shown));
    // Undoing the run brings the old view back. The variables stay, as in TreeSheets,
    // so the formula still shows its number: the index goes by the editor's results.
    desk.undo();
    assert_eq!(desk.find("\"old view\"").len(), 1);
    assert_eq!(desk.editor.shown(b.reads), Some("45"));
    assert_eq!(desk.find("45")[0].cell, b.reads);
    assert!(desk.find("#N/A").is_empty());
}

#[test]
fn replace_changes_formulas_and_never_results() {
    let b = budget();
    let before = format::to_string(&b.doc).unwrap();
    let mut desk = Desk::new(b.doc);
    let all = ReplaceOptions { formulas: true };
    let run = |desk: &mut Desk, what: &str, with: &str, options: ReplaceOptions| {
        let term = Term::new(what);
        let done = replace(
            &mut desk.editor,
            &term,
            with,
            Scope::Document,
            options,
            None,
            &mut desk.ids,
        )
        .unwrap();
        desk.follow(&done.changes);
        (done.counts.cells, done.counts.places)
    };
    let go = |desk: &mut Desk, what: &str, with: &str| run(desk, what, with, all);
    // A result is not text that can be replaced: nothing happens.
    assert_eq!(go(&mut desk, "1050.5", "0"), (0, 0));
    assert_eq!(format::to_string(desk.editor.document()).unwrap(), before);
    // A formula is not replaced in unless that is asked for (decision A106).
    assert_eq!(
        run(&mut desk, "sum", "MAX", ReplaceOptions::default()),
        (0, 0)
    );
    assert_eq!(format::to_string(desk.editor.document()).unwrap(), before);
    // In the formula it is: the cell stays a formula cell, and shows the new result.
    assert_eq!(go(&mut desk, "sum", "MAX"), (1, 1));
    let cell = desk.editor.document().cell(b.total).unwrap();
    assert_eq!(
        (cell.text.as_str(), cell.kind),
        ("=MAX(Costs)", CellKind::Formula)
    );
    assert_eq!(desk.editor.shown(b.total), Some("800"));
    assert!(desk.find("1050.5").is_empty());
    assert_eq!(
        desk.find("800").len(),
        3,
        "the number, the result, the link to it"
    );
    // Renaming a table everywhere: its cell and the formulas that name it, so the
    // result stays.
    assert_eq!(go(&mut desk, "costs", "Kosten"), (2, 2));
    assert_eq!(desk.editor.shown(b.total), Some("800"));
    // Replacing only the name of the table leaves the formula without it.
    let costs = desk.editor.document().cell(b.rent).unwrap().parent();
    let only = [costs.unwrap()];
    let done = replace(
        &mut desk.editor,
        &Term::new("Kosten"),
        "Spend",
        Scope::Cells(&only),
        ReplaceOptions::default(),
        None,
        &mut desk.ids,
    )
    .unwrap();
    desk.follow(&done.changes);
    assert_eq!(desk.find("#NAME?")[0].cell, b.total);
    // One undo per replace, to the byte.
    for _ in 0..3 {
        desk.undo();
    }
    assert_eq!(format::to_string(desk.editor.document()).unwrap(), before);
    assert_eq!(desk.find("1050.5").len(), 2);
}

fn scratch(test: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("search_formulas")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(dir.with_extension("cache"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Results are not in the file: for a document that nobody has open, the scan of the
/// workspace works them out, and the cache keeps them.
#[test]
fn documents_that_are_not_open_are_searched_by_their_results() {
    let b = budget();
    let dir = scratch("unopened");
    let ws = Workspace::open(&dir).unwrap();
    ws.save(&b.doc, "budget.wombat").unwrap();
    let saved = fs::read_to_string(dir.join("budget.wombat")).unwrap();
    assert!(!saved.contains("1050.5"), "a result was saved");
    let find = |mode: CacheMode, text: &str| {
        let cache = Cache::at(dir.with_extension("cache"), mode);
        let built = workspace::index(&ws, &cache).unwrap();
        let query = parse(text, 0).unwrap().query;
        let found = built.index.search(&query, &Options::default()).unwrap();
        let doc = ws.open_name("budget.wombat").unwrap();
        let calc = Calc::new(&doc);
        let docs = [("budget.wombat", &doc, &calc)];
        let expected = naive::search(&docs, &query, &Options::default()).unwrap();
        assert_eq!(found, expected);
        (found.total, built.reused)
    };
    assert_eq!(find(CacheMode::Off, "1050.5"), (2, 0));
    assert_eq!(find(CacheMode::Verify, "1050.5"), (2, 0));
    // From the cache: the result is there without working it out again.
    assert_eq!(find(CacheMode::Verify, "1050.5"), (2, 1));
    // The document is edited and saved: the next scan has the new result.
    let mut editor = Editor::new(ws.open_name("budget.wombat").unwrap()).unwrap();
    let set = Command::SetText {
        cell: b.rent,
        text: "300".into(),
    };
    editor
        .apply(set, None, &mut SeededIds::from_u64(1))
        .unwrap();
    ws.save(editor.document(), "budget.wombat").unwrap();
    assert_eq!(find(CacheMode::Verify, "1050.5"), (0, 0));
    assert_eq!(find(CacheMode::Verify, "550.5"), (2, 1));
}

/// A document made to be slow cannot hold a scan up for ever: the evaluator's limit on
/// work (decision A67) applies to a scan as to an editor, the formulas past it show
/// `#LIMIT!`, and the summary is the same every time. Measured in a release build:
/// 4.9 seconds for this document, which is what the limit of 50 million cells costs;
/// ten times that in a debug build, so it runs in release builds only
/// (`cargo test --release -p wombat-search --test formulas`).
#[test]
#[cfg_attr(debug_assertions, ignore = "takes two minutes in a debug build")]
fn a_document_of_heavy_formulas_is_summarised_in_bounded_time() {
    use wombat_core::summary::Summary;
    let mut ids = SeededIds::from_u64(9);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    // A table of 5,000 numbers, and 60 formulas that each add it up 500 times: 150
    // million cells to read, three times what one recalculation may read.
    doc.add_grid(root, 1, 61, &mut ids).unwrap();
    let cells = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
    doc.cell_mut(cells[0]).unwrap().text = "T".into();
    doc.add_grid(cells[0], 1, 5000, &mut ids).unwrap();
    let numbers = doc.cell(cells[0]).unwrap().grid().unwrap().cells().to_vec();
    for (n, cell) in numbers.iter().enumerate() {
        doc.cell_mut(*cell).unwrap().text = n.to_string();
    }
    let heavy = format!("={}0", "SUM(T)+".repeat(500));
    for cell in &cells[1..] {
        let formula = doc.cell_mut(*cell).unwrap();
        formula.text = heavy.clone();
        formula.kind = CellKind::Formula;
    }
    doc.validate().unwrap();
    let started = std::time::Instant::now();
    let summary = wombat_search::DocIndex::of(&doc);
    let took = started.elapsed();
    println!("summary of a document that asks for 150 million cell reads: {took:?}");
    assert!(took < std::time::Duration::from_secs(30), "{took:?}");
    let calc = Calc::new(&doc);
    let last = cells.last().copied().unwrap();
    assert_eq!(calc.result(cells[1]).unwrap().0, "6248750000");
    assert_eq!(calc.result(last).unwrap().0, "#LIMIT!");
    let mut index = SearchIndex::new();
    index.insert(summary.clone(), "heavy.wombat");
    let query = parse("#LIMIT!", 0).unwrap().query;
    let found = index.search(&query, &Options::default()).unwrap();
    assert!(found.total > 10 && found.total < 60, "{}", found.total);
    assert!(summary.encode() == wombat_search::DocIndex::of(&doc).encode());
}
