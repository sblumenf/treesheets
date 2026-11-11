//! Import and export (milestone 7): each format by example, and the rules that hold for
//! every document: what is exported can be read by a strict reader, comes back the same
//! through the importer, loses nothing in the lookup layout, and never changes the
//! document. The comparisons with the real TreeSheets are in `import_referee.rs` and in
//! `wombat-cts/tests/export.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod convert_support;
mod support;

use std::collections::{BTreeSet, HashMap};

use convert_support::{
    build, cell, formula, grid, source, spec, strict_csv, table_tree, texts, tree, Tree,
};
use proptest::collection::vec;
use proptest::prelude::*;
use wombat_core::convert::delimited::{guard, is_risky, parse, ParseLimits, Writer};
use wombat_core::convert::lookup::{self, column_letters, ExportFile, LookupOptions};
use wombat_core::convert::{
    decode, flat_csv, grid_csv, names, outline, parse_outline, parse_table, to_document, Delimiter,
    Entry, ExportOptions, Quoting, Table,
};
use wombat_core::eval::Calc;
use wombat_core::limits::{MAX_CELLS, MAX_DEPTH};
use wombat_core::{format, CellId, CellKind, Command, Document, Editor, Error, SeededIds};

/// Options that change nothing in the text: no mark, LF, nothing guarded.
fn plain() -> ExportOptions {
    ExportOptions {
        bom: false,
        crlf: false,
        guard: false,
        ..ExportOptions::default()
    }
}

fn options() -> impl Strategy<Value = ExportOptions> {
    (
        proptest::sample::select(&[Delimiter::Comma, Delimiter::Semicolon, Delimiter::Tab][..]),
        proptest::sample::select(&[Quoting::Minimal, Quoting::All][..]),
        any::<(bool, bool, bool, bool, bool)>(),
    )
        .prop_map(
            |(delimiter, quoting, (bom, crlf, guard, formulas, notes))| ExportOptions {
                delimiter,
                quoting,
                bom,
                crlf,
                guard,
                formulas,
                notes,
            },
        )
}

/// The projects of the summary's examples: two with tasks inside, one without.
fn projects() -> Document {
    build(
        &grid(
            "",
            3,
            vec![
                cell("Apollo"),
                cell("Ana"),
                grid(
                    "tasks",
                    2,
                    vec![cell("Design"), cell("done"), cell("Build"), cell("open")],
                ),
                cell("Borealis"),
                cell("Chidi"),
                grid("", 2, vec![cell("Survey"), cell("open")]),
                cell("Cygnus"),
                cell("Eve"),
                cell("none"),
            ],
        ),
        1,
    )
}

fn export<F>(doc: &Document, f: F) -> String
where
    F: FnOnce(wombat_core::convert::Source) -> Result<String, Error>,
{
    let calc = Calc::new(doc);
    f(source(doc, &calc)).unwrap()
}

// ---- Each format by example ----

#[test]
fn a_grid_exports_as_one_line_per_row_with_only_the_text_of_cells_that_have_grids() {
    let doc = projects();
    let text = export(&doc, |s| grid_csv(s, doc.root(), &plain()));
    assert_eq!(text, "Apollo,Ana,tasks\nBorealis,Chidi,\nCygnus,Eve,none\n");
    // The defaults: a byte-order mark and CR LF.
    let text = export(&doc, |s| grid_csv(s, doc.root(), &ExportOptions::default()));
    assert_eq!(
        text,
        "\u{feff}Apollo,Ana,tasks\r\nBorealis,Chidi,\r\nCygnus,Eve,none\r\n"
    );
}

/// Where the milestones meet: Flatten and Hierarchify go by what formula cells show (A83,
/// since the review of milestone 5), and so does the flattened export (A96), so the
/// export of an outline, of the table that Flatten makes of it, and of the outline that
/// Hierarchify makes of that table are the same lines.
#[test]
fn the_flattened_export_and_the_commands_agree_on_what_formula_cells_show() {
    let with_grid = |text: &str, cells: Vec<_>| convert_support::Spec {
        formula: true,
        ..grid(text, 1, cells)
    };
    let doc = build(
        &grid(
            "",
            1,
            vec![
                with_grid("=1+2", vec![cell("x"), formula("=2*5")]),
                grid("3", 1, vec![formula("='a' & 'b'")]),
                formula("=7*1"),
            ],
        ),
        1,
    );
    let root = doc.root();
    let shown = "3,x\n3,10\n3,ab\n7,\n";
    let flat =
        |doc: &Document, options: &ExportOptions| export(doc, |s| flat_csv(s, doc.root(), options));
    assert_eq!(flat(&doc, &plain()), shown);
    // Only the option `formulas` shows the formulas, and only the export has it.
    let formulas = ExportOptions {
        formulas: true,
        ..plain()
    };
    assert_eq!(
        flat(&doc, &formulas),
        "=1+2,x\n=1+2,=2*5\n3,='a' & 'b'\n=7*1,\n"
    );

    let mut ids = SeededIds::from_u64(2);
    let mut editor = Editor::new(doc).unwrap();
    editor
        .apply(Command::Flatten { cell: root }, None, &mut ids)
        .unwrap();
    let table = editor.document().clone();
    assert_eq!(export(&table, |s| grid_csv(s, root, &plain())), shown);
    // The table holds the results as data, so nothing differs with the option.
    assert_eq!(export(&table, |s| grid_csv(s, root, &formulas)), shown);

    // Back to an outline: the rows of "=1+2" and of "3" showed the same, so they are one.
    editor
        .apply(Command::Hierarchify { cell: root }, None, &mut ids)
        .unwrap();
    assert_eq!(flat(editor.document(), &plain()), shown);

    // Hierarchify on formula cells themselves groups by what they show, as the export
    // of the table shows them.
    let doc = build(
        &grid(
            "",
            2,
            vec![
                formula("=1+2"),
                cell("x"),
                cell("3"),
                formula("=2*5"),
                formula("=7*1"),
                cell("z"),
            ],
        ),
        1,
    );
    let root = doc.root();
    let rows = export(&doc, |s| grid_csv(s, root, &plain()));
    assert_eq!(rows, "3,x\n3,10\n7,z\n");
    let mut editor = Editor::new(doc).unwrap();
    editor
        .apply(Command::Hierarchify { cell: root }, None, &mut ids)
        .unwrap();
    assert_eq!(flat(editor.document(), &plain()), rows);
    let top = editor.document().cell(root).unwrap().grid().unwrap();
    assert_eq!((top.xs(), top.ys()), (1, 2));
}

/// Where the milestones meet: a formula over the limits on calculating (A80, A81, from
/// the review of milestone 5) exports as the `#LIMIT!` that it shows, in every layout, the
/// guard leaves that alone, and the document is exported in no time.
#[test]
fn a_formula_over_the_limits_exports_as_the_error_it_shows() {
    // A text that would be 32^8 characters long.
    let mut bomb = String::from("'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'");
    for _ in 0..7 {
        bomb = format!("SUBSTITUTE({bomb},'a','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')");
    }
    let bomb = format!("=LEN({bomb})");
    let doc = build(
        &grid(
            "",
            2,
            vec![cell("big"), formula(&bomb), cell("small"), formula("=1+2")],
        ),
        1,
    );
    let root = doc.root();
    let started = std::time::Instant::now();
    let calc = Calc::new(&doc);
    let shown = calc.shown(&doc, doc.child(root, 1, 0).unwrap()).unwrap();
    assert_eq!(shown, "#LIMIT!");
    assert!(!is_risky(shown));
    let guarded = ExportOptions {
        guard: true,
        ..plain()
    };
    let table = "big,#LIMIT!\nsmall,3\n";
    assert_eq!(
        grid_csv(source(&doc, &calc), root, &guarded).unwrap(),
        table
    );
    assert_eq!(
        flat_csv(source(&doc, &calc), root, &guarded).unwrap(),
        table
    );
    assert_eq!(
        outline(source(&doc, &calc), root, false, false).unwrap(),
        "  big\n  #LIMIT!\n  small\n  3\n"
    );
    let files = lookup::export(
        source(&doc, &calc),
        root,
        &guarded,
        LookupOptions::default(),
    )
    .unwrap();
    assert!(files.iter().any(|f| f.text.contains(",#LIMIT!")));
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    // It comes back as that text, not as a formula.
    let back = parse_table(table, Delimiter::Comma, true).unwrap();
    assert_eq!(rows(&back)[0], ["big", "#LIMIT!"]);
    assert!(back.cells().iter().all(|entry| !entry.is_formula()));
}

#[test]
fn fields_are_quoted_only_where_delimited_text_needs_it() {
    let doc = build(
        &grid(
            "",
            3,
            vec![
                cell("a,b"),
                cell("say \"hi\""),
                cell("x;y"),
                cell(" lead"),
                cell("tab"),
                cell(""),
            ],
        ),
        1,
    );
    let csv = |options: &ExportOptions| export(&doc, |s| grid_csv(s, doc.root(), options));
    assert_eq!(
        csv(&plain()),
        "\"a,b\",\"say \"\"hi\"\"\",x;y\n lead,tab,\n"
    );
    let semicolon = ExportOptions {
        delimiter: Delimiter::Semicolon,
        ..plain()
    };
    assert_eq!(
        csv(&semicolon),
        "a,b;\"say \"\"hi\"\"\";\"x;y\"\n lead;tab;\n"
    );
    let all = ExportOptions {
        quoting: Quoting::All,
        ..plain()
    };
    assert_eq!(
        csv(&all),
        "\"a,b\",\"say \"\"hi\"\"\",\"x;y\"\n\" lead\",\"tab\",\"\"\n"
    );
}

#[test]
fn a_row_that_is_one_empty_cell_is_written_as_two_quotes_and_read_back() {
    let doc = build(&grid("", 1, vec![cell("a"), cell(""), cell("b")]), 1);
    let text = export(&doc, |s| grid_csv(s, doc.root(), &plain()));
    assert_eq!(text, "a\n\"\"\nb\n");
    let table = parse_table(&text, Delimiter::Comma, false).unwrap();
    assert_eq!((table.xs(), table.ys()), (1, 3));
}

#[test]
fn a_formula_cell_exports_what_it_shows_or_with_the_option_its_formula() {
    let doc = build(
        &grid("", 2, vec![formula("=1+2"), formula("=LEN('abc') * 2")]),
        1,
    );
    assert_eq!(
        doc.cell(doc.child(doc.root(), 0, 0).unwrap()).unwrap().kind,
        CellKind::Formula
    );
    let text = export(&doc, |s| grid_csv(s, doc.root(), &plain()));
    assert_eq!(text, "3,6\n");
    let formulas = ExportOptions {
        formulas: true,
        ..plain()
    };
    let text = export(&doc, |s| grid_csv(s, doc.root(), &formulas));
    assert_eq!(text, "=1+2,=LEN('abc') * 2\n");
    // Guarded, which is the default, the formulas cannot run in a spreadsheet either.
    let guarded = ExportOptions {
        formulas: true,
        bom: false,
        crlf: false,
        ..ExportOptions::default()
    };
    let text = export(&doc, |s| grid_csv(s, doc.root(), &guarded));
    assert_eq!(text, "'=1+2,'=LEN('abc') * 2\n");
}

#[test]
fn notes_are_a_column_after_each_column_when_asked_for() {
    let mut spec = grid("", 2, vec![cell("a"), cell("b")]);
    spec.grid.as_mut().unwrap().1[1].note = "second\nline".into();
    let doc = build(&spec, 1);
    let notes = ExportOptions {
        notes: true,
        ..plain()
    };
    let text = export(&doc, |s| grid_csv(s, doc.root(), &notes));
    assert_eq!(text, "a,,b,\"second\nline\"\n");
    // A carriage return alone is a line break too, and needs the quotes as much.
    let mut spec = grid("", 1, vec![cell("a")]);
    spec.grid.as_mut().unwrap().1[0].note = "old\rMac".into();
    let doc = build(&spec, 1);
    let text = export(&doc, |s| grid_csv(s, doc.root(), &notes));
    assert_eq!(text, "a,\"old\rMac\"\n");
}

#[test]
fn text_that_a_spreadsheet_would_run_gets_a_quote_in_front_and_numbers_do_not() {
    for risky in [
        "=1+1",
        "=cmd|' /C calc'!A0",
        "+1+1",
        "-1+1",
        "@SUM(1)",
        "-",
        "+",
        "=",
        "-a",
        "--5",
        "-5-",
        "-1.2.3",
        "+1e",
        "-e5",
        "-.",
        "\tx",
        "\rx",
        "-5 ",
        "- 5",
        "-٣",
    ] {
        assert!(is_risky(risky), "{risky:?}");
        assert_eq!(guard(risky), format!("'{risky}"));
    }
    for safe in [
        "", "a", "1+1", "a=b", "-5", "+5", "-2.5", "-2,5", "-.5", "+1e10", "-1.5E-3", "'=x",
        " =1+1", "1-2",
    ] {
        assert!(!is_risky(safe), "{safe:?}");
        assert_eq!(guard(safe), safe);
    }
}

#[test]
fn the_guard_covers_every_export_and_every_kind_of_field() {
    let mut spec = grid(
        "=root",
        2,
        vec![
            cell("=evil"),
            grid("@inner", 1, vec![cell("-run"), cell("-5")]),
        ],
    );
    spec.grid.as_mut().unwrap().1[0].note = "+note".into();
    let doc = build(&spec, 1);
    let calc = Calc::new(&doc);
    let s = source(&doc, &calc);
    let options = ExportOptions {
        notes: true,
        ..ExportOptions::default()
    };
    let mut all = vec![
        grid_csv(s, doc.root(), &options).unwrap(),
        flat_csv(s, doc.root(), &options).unwrap(),
    ];
    for header_row in [false, true] {
        let files = lookup::export(s, doc.root(), &options, LookupOptions { header_row }).unwrap();
        all.extend(
            files
                .into_iter()
                .filter(|f| f.name.ends_with(".csv"))
                .map(|f| f.text),
        );
    }
    let mut fields = 0;
    for text in &all {
        for row in strict_csv(text, ',', true) {
            for field in row {
                assert!(!is_risky(&field), "{field:?} in {text:?}");
                fields += 1;
            }
        }
    }
    assert!(fields > 40);
    // Every risky text is there, made harmless; the number is as it was.
    let joined = all.concat();
    for guarded in ["'=evil", "'@inner", "'-run", "'+note", "'=root"] {
        assert!(joined.contains(guarded), "{guarded}");
    }
    assert!(joined.contains(",-5") || joined.contains("\n-5"));
    // Raw, they are exported as they are.
    let raw = ExportOptions {
        guard: false,
        notes: true,
        ..ExportOptions::default()
    };
    let text = grid_csv(s, doc.root(), &raw).unwrap();
    assert!(text.contains("=evil,+note,@inner"));
}

#[test]
fn flattening_puts_each_innermost_row_after_the_rows_it_is_inside_of() {
    let doc = projects();
    let before = format::to_string(&doc).unwrap();
    let text = export(&doc, |s| flat_csv(s, doc.root(), &plain()));
    assert_eq!(
        text,
        "Apollo,Ana,tasks,Design,done\n\
         Apollo,Ana,tasks,Build,open\n\
         Borealis,Chidi,,Survey,open\n\
         Cygnus,Eve,none,,\n"
    );
    assert_eq!(format::to_string(&doc).unwrap(), before);
}

#[test]
fn flattening_irregular_nesting_gives_a_rectangle_with_one_block_of_columns_per_level() {
    // Level 1 is two columns wide. Level 2 has a grid of three columns and one of one.
    // Level 3 exists only under "deep". One row has two nested grids, one is all empty.
    let doc = build(
        &grid(
            "",
            2,
            vec![
                grid("A", 3, vec![cell("a1"), cell("a2"), cell("a3")]),
                grid(
                    "B",
                    1,
                    vec![grid("deep", 1, vec![cell("d1"), cell("d2")]), cell("b2")],
                ),
                cell("solo"),
                cell(""),
                cell(""),
                cell(""),
            ],
        ),
        1,
    );
    let text = export(&doc, |s| flat_csv(s, doc.root(), &plain()));
    assert_eq!(
        text,
        "A,B,a1,a2,a3,\n\
         A,B,deep,,,d1\n\
         A,B,deep,,,d2\n\
         A,B,b2,,,\n\
         solo,,,,,\n\
         ,,,,,\n"
    );
}

#[test]
fn indented_text_is_one_line_per_cell_two_spaces_per_level() {
    let mut spec = grid(
        "top",
        2,
        vec![
            grid("Fruit", 1, vec![cell("Apple"), cell("")]),
            cell("=1+2"),
        ],
    );
    spec.grid.as_mut().unwrap().1[1].formula = true;
    let mut doc = build(&spec, 1);
    let pictured = doc.child(doc.root(), 1, 0).unwrap();
    {
        use wombat_core::construct::Construct;
        doc.cell_mut(pictured).unwrap().image = Some(wombat_core::ImageRef::new("p.png"));
    }
    let calc = Calc::new(&doc);
    let s = source(&doc, &calc);
    // As the script function: the grid alone. A cell with an image has a space after it.
    assert_eq!(
        outline(s, doc.root(), false, false).unwrap(),
        "  Fruit\n    Apple\n    \n  3 \n"
    );
    // As the menu export: the cell's own line first; CR LF as TreeSheets on Linux.
    assert_eq!(
        outline(s, doc.root(), true, true).unwrap(),
        "top\r\n  Fruit\r\n    Apple\r\n    \r\n  3 \r\n"
    );
    // A cell without a grid: its text and nothing else.
    let apple = doc
        .child(doc.child(doc.root(), 0, 0).unwrap(), 0, 0)
        .unwrap();
    assert_eq!(outline(s, apple, false, false).unwrap(), "Apple");
    assert_eq!(outline(s, apple, true, false).unwrap(), "Apple\n");
}

#[test]
fn exports_of_a_cell_without_a_grid_or_of_no_cell_are_refused() {
    let doc = projects();
    let calc = Calc::new(&doc);
    let s = source(&doc, &calc);
    let leaf = doc.child(doc.root(), 0, 0).unwrap();
    let nowhere = support::no_such_cell();
    let options = plain();
    assert!(matches!(
        grid_csv(s, leaf, &options),
        Err(Error::Refused(_))
    ));
    assert!(matches!(
        flat_csv(s, leaf, &options),
        Err(Error::Refused(_))
    ));
    assert!(matches!(
        lookup::export(s, leaf, &options, LookupOptions::default()),
        Err(Error::Refused(_))
    ));
    assert!(matches!(
        grid_csv(s, nowhere, &options),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        outline(s, nowhere, true, false),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn an_export_stops_at_its_size_limit() {
    let options = plain();
    let mut out = Writer::with_limit(&options, 10);
    out.field("12345").unwrap();
    out.field("678").unwrap();
    assert!(matches!(out.field("90"), Err(Error::Limit(_))));
}

// ---- The lookup layout ----

/// One table read back from its file.
struct Read {
    names: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Read {
    fn column(&self, name: &str) -> usize {
        self.names
            .iter()
            .position(|n| n == name)
            .unwrap_or_else(|| panic!("no column {name} in {:?}", self.names))
    }
}

fn read(file: &ExportFile, options: &ExportOptions) -> Read {
    let mut rows = strict_csv(&file.text, options.delimiter.as_char(), options.crlf);
    let names = rows.remove(0);
    for row in &rows {
        assert_eq!(row.len(), names.len(), "{}", file.name);
    }
    Read { names, rows }
}

fn file<'a>(files: &'a [ExportFile], name: &str) -> &'a ExportFile {
    files
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no file {name}"))
}

/// Rebuilds the tree of a cell from the folder alone: `index.csv` says which tables there
/// are and where each belongs, the tables have the texts and the IDs. Uses no knowledge
/// of the document, and only the layout of the default options (letters as names).
fn rebuild(files: &[ExportFile], options: &ExportOptions) -> Tree {
    let index = read(file(files, lookup::INDEX_FILE), options);
    let id = |text: &str| CellId::parse(text).unwrap_or_else(|| panic!("{text:?} is no ID"));
    // The table inside each cell, by the cell's ID.
    let mut inside: HashMap<CellId, (&Vec<String>, Read)> = HashMap::new();
    let mut top = None;
    for row in &index.rows {
        let table = read(file(files, &row[index.column("table")]), options);
        let owner = id(&row[index.column("cell_id")]);
        if row[index.column("parent_table")].is_empty() {
            assert!(top.replace(owner).is_none(), "two top tables");
        }
        assert!(inside.insert(owner, (row, table)).is_none());
    }
    fn cell_of(
        owner: CellId,
        text: String,
        note: String,
        index: &Read,
        inside: &HashMap<CellId, (&Vec<String>, Read)>,
        notes: bool,
    ) -> Tree {
        let grid = inside.get(&owner).map(|(row, table)| {
            let xs: usize = row[index.column("columns")].parse().unwrap();
            assert_eq!(row[index.column("rows")], table.rows.len().to_string());
            let step = if notes { 2 } else { 1 };
            let row_id = xs * step;
            assert_eq!(table.names[row_id], "row_id");
            let mut cells = Vec::new();
            for line in &table.rows {
                assert_eq!(
                    line[row_id],
                    line[row_id + 1],
                    "row_id is the first cell's ID"
                );
                for x in 0..xs {
                    assert_eq!(
                        table.names[row_id + 1 + x],
                        format!("{}_id", column_letters(x))
                    );
                    let child = CellId::parse(&line[row_id + 1 + x]).unwrap();
                    let note = if notes {
                        line[x * step + 1].clone()
                    } else {
                        String::new()
                    };
                    cells.push(cell_of(
                        child,
                        line[x * step].clone(),
                        note,
                        index,
                        inside,
                        notes,
                    ));
                }
            }
            (xs as u32, cells)
        });
        Tree {
            id: Some(owner),
            text,
            note,
            grid,
        }
    }
    let top = top.expect("no top table");
    let text = inside[&top].0[index.column("cell_text")].clone();
    let tree = cell_of(top, text, String::new(), &index, &inside, options.notes);
    // Every table was reached from the top.
    fn count(tree: &Tree) -> usize {
        tree.grid
            .as_ref()
            .map_or(0, |(_, cells)| 1 + cells.iter().map(count).sum::<usize>())
    }
    assert_eq!(count(&tree), inside.len());
    tree
}

/// Checks the join that the layout promises, for every nested table: `parent_row_id`
/// finds exactly one row in the `row_id` column of the table named by `parent_table`, that
/// row has `parent_cell_id` among its cells, and names this table in the `_table` column
/// of that cell; and it is the right row of the document.
fn check_joins(doc: &Document, files: &[ExportFile], options: &ExportOptions) -> usize {
    let index = read(file(files, lookup::INDEX_FILE), options);
    let mut joined = 0;
    for entry in &index.rows {
        let name = &entry[index.column("table")];
        let parent_name = &entry[index.column("parent_table")];
        if parent_name.is_empty() {
            continue;
        }
        let table = read(file(files, name), options);
        let parent = read(file(files, parent_name), options);
        let key = parent.column("row_id");
        for line in &table.rows {
            assert_eq!(&line[table.column("parent_table")], parent_name);
            let row_id = &line[table.column("parent_row_id")];
            let cell_id = &line[table.column("parent_cell_id")];
            assert_eq!(row_id, &entry[index.column("parent_row_id")]);
            assert_eq!(cell_id, &entry[index.column("cell_id")]);
            // What XLOOKUP does: the rows whose key equals the value.
            let found: Vec<&Vec<String>> =
                parent.rows.iter().filter(|r| &r[key] == row_id).collect();
            assert_eq!(found.len(), 1, "{row_id} in {parent_name}");
            // The last such field: for the first cell of a row, `row_id` holds its ID too.
            let at = found[0].iter().rposition(|f| f == cell_id).unwrap();
            let column = &entry[index.column("parent_column")];
            assert_eq!(parent.names[at], format!("{column}_id"));
            assert_eq!(&found[0][parent.column(&format!("{column}_table"))], name);
            // In the document: the cell the table sits in is in the row that starts with
            // the cell whose ID is the key.
            let owner = CellId::parse(cell_id).unwrap();
            let around = doc
                .cell(doc.cell(owner).unwrap().parent().unwrap())
                .unwrap();
            let g = around.grid().unwrap();
            let place = g.cells().iter().position(|c| *c == owner).unwrap();
            let first = g.cells()[place - place % g.xs() as usize];
            assert_eq!(row_id, &first.to_string());
            joined += 1;
        }
    }
    joined
}

#[test]
fn the_lookup_export_of_the_projects_is_what_the_summary_shows() {
    let doc = projects();
    let calc = Calc::new(&doc);
    let options = plain();
    let files = lookup::export(
        source(&doc, &calc),
        doc.root(),
        &options,
        LookupOptions::default(),
    )
    .unwrap();
    let id = |x: u32, y: u32| doc.child(doc.root(), x, y).unwrap();
    let short = |id: CellId| id.to_string()[..8].to_string();
    let top = format!("table-{}.csv", short(doc.root()));
    let tasks = format!("tasks-{}.csv", short(id(2, 0)));
    // The cell without text is called after the first text of its row.
    let other = format!("Borealis-{}.csv", short(id(2, 1)));
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["README.txt", "index.csv", &top, &tasks, &other]);

    assert_eq!(
        file(&files, &top).text,
        format!(
            "A,B,C,row_id,A_id,B_id,C_id,C_table\n\
             Apollo,Ana,tasks,{a},{a},{},{},{tasks}\n\
             Borealis,Chidi,,{b},{b},{},{},{other}\n\
             Cygnus,Eve,none,{c},{c},{},{},\n",
            id(1, 0),
            id(2, 0),
            id(1, 1),
            id(2, 1),
            id(1, 2),
            id(2, 2),
            a = id(0, 0),
            b = id(0, 1),
            c = id(0, 2),
        )
    );
    let task = |x: u32, y: u32| doc.child(id(2, 0), x, y).unwrap();
    assert_eq!(
        file(&files, &tasks).text,
        format!(
            "A,B,row_id,A_id,B_id,parent_table,parent_row_id,parent_cell_id\n\
             Design,done,{d},{d},{},{top},{row},{cell}\n\
             Build,open,{b},{b},{},{top},{row},{cell}\n",
            task(1, 0),
            task(1, 1),
            d = task(0, 0),
            b = task(0, 1),
            row = id(0, 0),
            cell = id(2, 0),
        )
    );
    assert_eq!(
        file(&files, "index.csv").text,
        format!(
            "table,trail,rows,columns,header_row,parent_table,parent_row_id,parent_column,cell_id,cell_text\n\
             {top},,3,3,no,,,,{},\n\
             {tasks},tasks,2,2,no,{top},{},C,{},tasks\n\
             {other},Borealis,1,2,no,{top},{},C,{},\n",
            doc.root(),
            id(0, 0),
            id(2, 0),
            id(0, 1),
            id(2, 1),
        )
    );
    // The formula of the README names the real files and the real columns: in the tasks
    // table parent_row_id is the 7th column (G), in the top table row_id is the 4th (D).
    let readme = &file(&files, "README.txt").text;
    let stem = |name: &str| name.trim_end_matches(".csv").to_string();
    let formula = format!("=XLOOKUP(G2, '{0}'!D:D, '{0}'!A:A)", stem(&top));
    assert!(readme.contains(&formula), "{readme}");
    assert!(readme.contains(&format!("=COUNTIF('{}'!G:G, D2)", stem(&tasks))));
    assert_eq!(
        file(&files, &tasks)
            .text
            .lines()
            .next()
            .unwrap()
            .split(',')
            .nth(6),
        Some("parent_row_id")
    );
    assert_eq!(check_joins(&doc, &files, &options), 3);
    assert_eq!(
        rebuild(&files, &options),
        tree(&doc, &calc, doc.root(), false)
    );
}

#[test]
fn with_the_header_option_the_first_row_names_the_columns_where_it_can() {
    let doc = build(
        &grid(
            "",
            3,
            vec![
                cell("Project"),
                cell("project"),
                cell(""),
                cell("Apollo"),
                cell("Ana"),
                grid("", 2, vec![cell("only"), cell("row")]),
                cell("Borealis"),
                cell("Chidi"),
                grid(
                    "",
                    1,
                    vec![grid("in the first row", 1, vec![cell("x")]), cell("y")],
                ),
            ],
        ),
        1,
    );
    let calc = Calc::new(&doc);
    let options = plain();
    let lookup = LookupOptions { header_row: true };
    let files = lookup::export(source(&doc, &calc), doc.root(), &options, lookup).unwrap();
    let index = read(file(&files, "index.csv"), &options);
    let headers: Vec<&str> = index
        .rows
        .iter()
        .map(|r| r[index.column("header_row")].as_str())
        .collect();
    // The top table has a row to spare; a table of one row has not; a table with a grid
    // in its first row keeps all rows, so that the grid's rows find their parent.
    assert_eq!(headers, ["yes", "no", "no", "no"]);
    let top = read(&files[2], &options);
    // Same name twice (in any case): the second gets a number. No text: the letter.
    assert_eq!(
        top.names,
        [
            "Project",
            "project_2",
            "C",
            "row_id",
            "Project_id",
            "project_2_id",
            "C_id",
            "C_table"
        ]
    );
    assert_eq!(top.rows.len(), 2);
    assert_eq!(top.rows[0][0], "Apollo");
    assert_eq!(index.rows[0][index.column("rows")], "2");
    assert_eq!(index.rows[1][index.column("parent_column")], "C");
    assert_eq!(check_joins(&doc, &files, &options), 4);
}

#[test]
fn a_column_named_like_a_key_column_does_not_take_its_place() {
    let doc = build(
        &grid(
            "",
            3,
            vec![
                cell("row_id"),
                cell("A_id"),
                cell("=SUM(1)"),
                cell("1"),
                cell("2"),
                grid("", 1, vec![cell("x")]),
            ],
        ),
        1,
    );
    let calc = Calc::new(&doc);
    let options = ExportOptions {
        formulas: true,
        ..plain()
    };
    let lookup = LookupOptions { header_row: true };
    let files = lookup::export(source(&doc, &calc), doc.root(), &options, lookup).unwrap();
    let top = read(&files[2], &options);
    assert_eq!(
        top.names,
        [
            "row_id",
            "A_id",
            "=SUM(1)",
            "row_id_2",
            "row_id_id",
            "A_id_id",
            "=SUM(1)_id",
            "=SUM(1)_table"
        ]
    );
    // The key is where the layout says, whatever it is called: after the content.
    assert_eq!(
        top.rows[0][3],
        doc.child(doc.root(), 0, 1).unwrap().to_string()
    );
    // Guarded, a name cannot run either.
    let guarded = ExportOptions {
        formulas: true,
        bom: false,
        crlf: false,
        ..ExportOptions::default()
    };
    let files = lookup::export(source(&doc, &calc), doc.root(), &guarded, lookup).unwrap();
    assert!(files[2].text.starts_with("row_id,A_id,'=SUM(1),"));
}

#[test]
fn column_letters_are_those_of_a_spreadsheet() {
    let letters: Vec<String> = [0, 1, 25, 26, 27, 51, 52, 701, 702, 16383]
        .into_iter()
        .map(column_letters)
        .collect();
    assert_eq!(
        letters,
        ["A", "B", "Z", "AA", "AB", "AZ", "BA", "ZZ", "AAA", "XFD"]
    );
}

#[test]
fn more_tables_than_the_limit_are_refused() {
    let mut ids = SeededIds::from_u64(1);
    let mut doc = Document::new(&mut ids);
    {
        use wombat_core::construct::Construct;
        let root = doc.root();
        doc.add_grid(root, 1, lookup::MAX_TABLES as u32, &mut ids)
            .unwrap();
        for id in doc.cell(root).unwrap().grid().unwrap().cells().to_vec() {
            doc.add_grid(id, 1, 1, &mut ids).unwrap();
        }
    }
    let calc = Calc::new(&doc);
    let result = lookup::export(
        source(&doc, &calc),
        doc.root(),
        &plain(),
        LookupOptions::default(),
    );
    assert!(matches!(result, Err(Error::Limit(_))));
}

// ---- File names ----

const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// What a file name must be to be safe on Windows, macOS and Linux, and as a sheet name.
fn check_name(stem: &str) {
    assert!(!stem.is_empty());
    assert!(stem.chars().count() <= names::MAX_STEM_CHARS, "{stem:?}");
    assert!(stem.len() <= 200, "{stem:?}");
    assert!(
        stem.chars().all(|c| c.is_alphanumeric() || c == '-'),
        "{stem:?}"
    );
    assert!(stem.chars().next().unwrap().is_alphanumeric(), "{stem:?}");
    assert!(
        stem.chars().last().unwrap().is_ascii_alphanumeric(),
        "{stem:?}"
    );
    for bad in [
        '/', '\\', ':', '*', '?', '"', '<', '>', '|', '.', ' ', '\0', '[', ']', '\'',
    ] {
        assert!(!stem.contains(bad), "{stem:?}");
    }
    assert!(!stem.chars().any(char::is_control));
    assert!(!RESERVED.contains(&stem.to_lowercase().as_str()));
    assert!(stem != "index" && stem != "README");
}

#[test]
fn hostile_texts_give_harmless_file_names() {
    let id = CellId(wombat_core::Id::from_u128(
        0x1234_5678_9abc_4def_8123_4567_89ab_cdef,
    ));
    for (labels, expected) in [
        (vec!["../../etc/passwd"], "etc-passwd-12345678"),
        (vec!["/absolute/path"], "absolute-path-12345678"),
        (vec!["C:\\Windows\\System32"], "C-Windows-System32-12345678"),
        (vec!["CON"], "CON-12345678"),
        (vec!["nul.txt"], "nul-txt-12345678"),
        (vec![".."], "table-12345678"),
        (vec!["."], "table-12345678"),
        (vec![""], "table-12345678"),
        (vec!["   "], "table-12345678"),
        (vec!["-rf"], "rf-12345678"),
        (vec!["a\u{0}b\nc"], "a-b-c-12345678"),
        (
            vec!["Projects", "Apollo", "Tasks"],
            "Projects-Apollo-Tasks-12345678",
        ),
        (
            vec!["a very long project name", "with an even longer task name"],
            "even-longer-task-name-12345678",
        ),
        (vec!["日本語 の 表"], "日本語-の-表-12345678"),
        (vec!["index"], "index-12345678"),
    ] {
        let stems = names::stems(&[(labels.clone(), id)]);
        assert_eq!(stems, [expected], "{labels:?}");
        check_name(&stems[0]);
    }
}

#[test]
fn tables_whose_ids_start_alike_get_numbers_instead() {
    let a = CellId(wombat_core::Id::from_u128(
        0x1234_5678_0000_4000_8000_0000_0000_0001,
    ));
    let b = CellId(wombat_core::Id::from_u128(
        0x1234_5678_0000_4000_8000_0000_0000_0002,
    ));
    let stems = names::stems(&[(vec!["x"], a), (vec!["X"], b)]);
    assert_eq!(stems, ["x-n1", "X-n2"]);
}

// ---- Import by example ----

fn rows(table: &Table) -> Vec<Vec<&str>> {
    table
        .cells()
        .chunks(table.xs() as usize)
        .map(|row| row.iter().map(Entry::text).collect())
        .collect()
}

#[test]
fn delimited_text_is_read_as_rfc_4180_says() {
    let text = "name,quote,empty\r\n\"a,b\",\"say \"\"hi\"\"\",\r\n\"two\r\nlines\",\"\",last";
    let table = parse_table(text, Delimiter::Comma, false).unwrap();
    assert_eq!(
        rows(&table),
        [
            ["name", "quote", "empty"],
            ["a,b", "say \"hi\"", ""],
            ["two lines", "", "last"]
        ]
    );
    // Short rows are padded to a rectangle; empty lines are no rows; CR alone ends a line.
    let table = parse_table("a\r\rb,c,d\n\n\ne,f", Delimiter::Comma, false).unwrap();
    assert_eq!(
        rows(&table),
        [["a", "", ""], ["b", "c", "d"], ["e", "f", ""]]
    );
    // Other delimiters; a tab inside a quoted field becomes a space, as any control character.
    let table = parse_table("a;b,c\n\"x;y\";z", Delimiter::Semicolon, false).unwrap();
    assert_eq!(rows(&table), [["a", "b,c"], ["x;y", "z"]]);
    let table = parse_table("a\tb\n\"q\tq\"\t\u{1}", Delimiter::Tab, false).unwrap();
    assert_eq!(rows(&table), [["a", "b"], ["q q", " "]]);
}

#[test]
fn what_is_not_valid_csv_is_read_without_complaint() {
    let read = |text: &str| {
        let table = parse_table(text, Delimiter::Comma, false).unwrap();
        rows(&table)
            .into_iter()
            .map(|r| r.into_iter().map(String::from).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    };
    // A quote in a field that did not start with one is a character.
    assert_eq!(read("a\"b,c\"d\""), [["a\"b", "c\"d\""]]);
    // Text after a closing quote stays with the field.
    assert_eq!(read("\"abc\"x,def"), [["abcx", "def"]]);
    // A quote that is not closed runs to the end.
    assert_eq!(read("a,\"bc\nd,e\n"), [["a", "bc d,e"]]);
    // A delimiter at the end of a line is followed by an empty field.
    assert_eq!(read("a,b,\nc,d,"), [["a", "b", ""], ["c", "d", ""]]);
    // Spaces are part of the field.
    assert_eq!(read(" a , \"b\" "), [[" a ", " \"b\" "]]);
}

#[test]
fn a_file_with_nothing_in_it_is_refused() {
    for text in ["", "\n", "\r\n\r\n"] {
        assert!(matches!(
            parse_table(text, Delimiter::Comma, false),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            parse_outline(text, MAX_DEPTH, false),
            Err(Error::Invalid(_))
        ));
    }
    // A line of spaces is a cell.
    assert_eq!(
        parse_table(" ", Delimiter::Comma, false)
            .unwrap()
            .cell_count(),
        1
    );
}

#[test]
fn files_are_decoded_whatever_a_spreadsheet_saved_them_as() {
    assert_eq!(decode(b"\xef\xbb\xbfa,\xc3\xa9"), "a,é");
    assert_eq!(decode("a,é,日".as_bytes()), "a,é,日");
    // UTF-16 with a byte-order mark, either way round, with a character outside the BMP.
    let utf16: Vec<u16> = "a\té😀".encode_utf16().collect();
    let le: Vec<u8> = [0xff, 0xfe]
        .into_iter()
        .chain(utf16.iter().flat_map(|u| u.to_le_bytes()))
        .collect();
    let be: Vec<u8> = [0xfe, 0xff]
        .into_iter()
        .chain(utf16.iter().flat_map(|u| u.to_be_bytes()))
        .collect();
    assert_eq!(decode(&le), "a\té😀");
    assert_eq!(decode(&be), "a\té😀");
    // Not UTF-8: Windows-1252, where 0xE9 is é and 0x80 the euro sign.
    assert_eq!(decode(b"caf\xe9,\x80 5"), "café,€ 5");
    // Half a UTF-16 unit at the end, or half a pair, does not stop it.
    assert_eq!(decode(&[0xff, 0xfe, 0x41, 0x00, 0x42]), "A");
    assert_eq!(decode(&[0xff, 0xfe, 0x3d, 0xd8]), "\u{fffd}");
}

#[test]
fn indented_text_becomes_grids_of_one_column() {
    let text = "Projects\n  Apollo\n    Design\n    Build\n  Borealis\nNotes\n\tone\n";
    let table = parse_outline(text, MAX_DEPTH, false).unwrap();
    let expected = (
        1,
        vec![
            convert_support::texts(&tree_of(
                "Projects",
                vec![
                    tree_of(
                        "Apollo",
                        vec![tree_of("Design", vec![]), tree_of("Build", vec![])],
                    ),
                    tree_of("Borealis", vec![]),
                ],
            )),
            tree_of("Notes", vec![tree_of("one", vec![])]),
        ],
    );
    assert_eq!(table_tree(&table), expected);
    assert_eq!((table.height(), table.cell_count()), (3, 7));
}

fn tree_of(text: &str, inside: Vec<Tree>) -> Tree {
    Tree {
        id: None,
        text: text.to_string(),
        note: String::new(),
        grid: (!inside.is_empty()).then_some((1, inside)),
    }
}

#[test]
fn indentation_is_read_as_treesheets_reads_it() {
    let read = |text: &str, levels: u32| table_tree(&parse_outline(text, levels, false).unwrap()).1;
    // Any deeper indentation is one level; coming back part of the way goes into the grid
    // of the line before at that level.
    assert_eq!(
        read(
            "A\n    deep\n  shallower\n      deeper\n  back\nB",
            MAX_DEPTH
        ),
        [
            tree_of(
                "A",
                vec![
                    tree_of("deep", vec![]),
                    tree_of("shallower", vec![tree_of("deeper", vec![])]),
                    tree_of("back", vec![])
                ]
            ),
            tree_of("B", vec![])
        ]
    );
    // The first line sets the top level; lines indented less are of the top level too,
    // and so is a line indented more than those but less than the first.
    assert_eq!(
        read("    first\n      inside\n  less\nnone\n   more", MAX_DEPTH),
        [
            tree_of("first", vec![tree_of("inside", vec![])]),
            tree_of("less", vec![]),
            tree_of("none", vec![]),
            tree_of("more", vec![]),
        ]
    );
    // Empty lines are skipped, a line of spaces is an empty cell at its indentation, and
    // spaces at the end of a line stay.
    assert_eq!(
        read("a\n\n  b\n\n\n  c\n   \nd  \n", MAX_DEPTH),
        [
            tree_of(
                "a",
                vec![
                    tree_of("b", vec![]),
                    tree_of("c", vec![tree_of("", vec![])])
                ]
            ),
            tree_of("d  ", vec![]),
        ]
    );
    // Nesting stops at the number of levels allowed: deeper lines are rows of the last.
    assert_eq!(
        read("a\n b\n  c\n   d\n b2", 2),
        [tree_of(
            "a",
            vec![
                tree_of("b", vec![]),
                tree_of("c", vec![]),
                tree_of("d", vec![]),
                tree_of("b2", vec![])
            ]
        )]
    );
}

#[test]
fn text_that_starts_with_an_equals_sign_is_a_formula_only_when_asked() {
    let text = "=1+2,=,plain\n";
    let table = parse_table(text, Delimiter::Comma, false).unwrap();
    assert!(table.cells().iter().all(|e| !e.is_formula()));
    let doc = to_document(&table, &mut SeededIds::from_u64(1)).unwrap();
    assert!(doc
        .cells_in_order()
        .iter()
        .all(|id| doc.cell(*id).unwrap().kind == CellKind::Data));

    let table = parse_table(text, Delimiter::Comma, true).unwrap();
    let formulas: Vec<bool> = table.cells().iter().map(Entry::is_formula).collect();
    // `=` alone is no formula, as when it is typed.
    assert_eq!(formulas, [true, false, false]);
    let doc = to_document(&table, &mut SeededIds::from_u64(1)).unwrap();
    let first = doc.child(doc.root(), 0, 0).unwrap();
    assert_eq!(doc.cell(first).unwrap().kind, CellKind::Formula);
    assert_eq!(Calc::new(&doc).shown(&doc, first), Some("3"));
    let outline = parse_outline("=2*3\n  =x", MAX_DEPTH, true).unwrap();
    assert!(outline.cells()[0].is_formula());
    assert!(outline.cells()[0].grid().unwrap().cells()[0].is_formula());
}

#[test]
fn a_table_is_always_a_rectangle_within_the_limits() {
    assert!(Table::new(0, vec![Entry::new("a")]).is_err());
    assert!(Table::new(1, vec![]).is_err());
    assert!(Table::new(2, vec![Entry::new("a")]).is_err());
    let mut table = Table::new(1, vec![Entry::new("a\tb\n")]).unwrap();
    assert_eq!(table.cells()[0].text(), "a b ");
    for _ in 1..MAX_DEPTH {
        table = Table::new(1, vec![Entry::new("").with_grid(table)]).unwrap();
    }
    assert_eq!(table.height(), MAX_DEPTH);
    assert!(matches!(
        Table::new(1, vec![Entry::new("").with_grid(table)]),
        Err(Error::Limit(_))
    ));
}

#[test]
fn the_limits_of_a_document_stop_an_import() {
    // More cells than a document may have: many rows, many columns, or their product.
    let tall = "a\n".repeat(MAX_CELLS);
    assert!(matches!(
        parse_table(&tall, Delimiter::Comma, false),
        Err(Error::Limit(_))
    ));
    assert!(matches!(
        parse_outline(&tall, MAX_DEPTH, false),
        Err(Error::Limit(_))
    ));
    let wide = ",".repeat(MAX_CELLS);
    assert!(matches!(
        parse_table(&wide, Delimiter::Comma, false),
        Err(Error::Limit(_))
    ));
    // 1,000 rows of one field and one row of 1,000: a million cells after padding.
    let ragged = format!("{}{}", "a\n".repeat(1000), ",".repeat(1000));
    assert!(matches!(
        parse_table(&ragged, Delimiter::Comma, false),
        Err(Error::Limit(_))
    ));
    // Just inside the limit is read.
    let fits = "a\n".repeat(MAX_CELLS - 2);
    assert_eq!(
        parse_table(&fits, Delimiter::Comma, false).unwrap().ys() as usize,
        MAX_CELLS - 2
    );
    // A field longer than a text may be, with a small bound to keep the test quick.
    let limits = ParseLimits {
        cells: 100,
        field: 5,
    };
    assert!(parse("abcde,\"abcde\"", Delimiter::Comma, limits).is_ok());
    assert!(matches!(
        parse("abcdef", Delimiter::Comma, limits),
        Err(Error::Limit(_))
    ));
    assert!(matches!(
        parse("\"abc\"\"ef\"", Delimiter::Comma, limits),
        Err(Error::Limit(_))
    ));
    assert!(matches!(
        parse("\"abcde\"x", Delimiter::Comma, limits),
        Err(Error::Limit(_))
    ));
    // The rectangle that padding would make is counted before it is made: three rows
    // that are three wide at their widest are nine cells.
    let limits = ParseLimits { cells: 8, field: 5 };
    let over = |text: &str| matches!(parse(text, Delimiter::Comma, limits), Err(Error::Limit(_)));
    assert!(!over("a\nb\nc,d"));
    assert!(over("a\nb\nc,d,e"));
    assert!(over("a,b,c\nd\ne"));
    assert!(over(",,,,,,,,"));
}

// ---- Import into a cell: a command like any other ----

fn editor(doc: Document) -> Editor {
    Editor::new(doc).unwrap()
}

#[test]
fn importing_into_a_cell_gives_it_the_grid_and_undoes_as_one_unit() {
    let mut editor = editor(projects());
    let mut ids = SeededIds::from_u64(7);
    let before = support::saved(editor.document());
    let root = editor.document().root();
    let target = editor.document().child(root, 1, 2).unwrap();
    let table = parse_outline("x\n  =1+2\ny", MAX_DEPTH, true).unwrap();
    let doc_before = editor.document().clone();
    let changes = editor
        .apply(
            Command::Import {
                cell: target,
                table,
            },
            Some(5),
            &mut ids,
        )
        .unwrap();
    support::check_report(&doc_before, editor.document(), &changes, "import");
    support::check_document(editor.document(), false);
    support::check_bound(&editor, "import");
    let doc = editor.document();
    // The cell keeps its text and gets the grid; the new cells are stamped.
    assert_eq!(doc.cell(target).unwrap().text, "Eve");
    assert_eq!(changes.created.len(), 3);
    assert_eq!(changes.grids, [target]);
    let x = doc.child(target, 0, 0).unwrap();
    assert_eq!(doc.cell(x).unwrap().last_edit, Some(5));
    let formula = doc.child(x, 0, 0).unwrap();
    assert_eq!(doc.cell(formula).unwrap().kind, CellKind::Formula);
    assert_eq!(editor.shown(formula), Some("3"));
    assert_eq!(editor.undo_len(), 1);
    editor.undo().unwrap();
    assert_eq!(support::saved(editor.document()), before);
    editor.redo().unwrap();
    assert_eq!(editor.shown(formula), Some("3"));
}

#[test]
fn importing_over_a_grid_replaces_it_and_into_the_root_is_allowed() {
    let mut editor = editor(projects());
    let mut ids = SeededIds::from_u64(7);
    let before = support::saved(editor.document());
    let root = editor.document().root();
    let old: BTreeSet<CellId> = editor.document().cells_in_order().into_iter().collect();
    let table = parse_table("a,b\n1,2\n", Delimiter::Comma, false).unwrap();
    let changes = editor
        .apply(Command::Import { cell: root, table }, None, &mut ids)
        .unwrap();
    assert_eq!(changes.removed.len(), old.len() - 1);
    assert_eq!(changes.created.len(), 4);
    let doc = editor.document();
    assert_eq!(doc.cell_count(), 5);
    assert_eq!(doc.cell(doc.child(root, 1, 1).unwrap()).unwrap().text, "2");
    // The IDs are new ones, not those of the cells that went.
    assert!(changes.created.iter().all(|id| !old.contains(id)));
    editor.undo().unwrap();
    assert_eq!(support::saved(editor.document()), before);
}

#[test]
fn an_import_that_does_not_fit_is_refused_and_changes_nothing() {
    let mut ids = SeededIds::from_u64(1);
    // A chain of grids as deep as allowed, and a cell at the bottom.
    let mut doc = Document::new(&mut ids);
    let mut at = doc.root();
    {
        use wombat_core::construct::Construct;
        for _ in 0..MAX_DEPTH {
            doc.add_grid(at, 1, 1, &mut ids).unwrap();
            at = doc.child(at, 0, 0).unwrap();
        }
    }
    let mut editor = editor(doc);
    let before = support::saved(editor.document());
    let table = || parse_table("a", Delimiter::Comma, false).unwrap();
    let result = editor.apply(
        Command::Import {
            cell: at,
            table: table(),
        },
        None,
        &mut ids,
    );
    assert!(matches!(result, Err(Error::Limit(_))), "{result:?}");
    // No such cell.
    let cell = support::no_such_cell();
    let result = editor.apply(
        Command::Import {
            cell,
            table: table(),
        },
        None,
        &mut ids,
    );
    assert!(matches!(result, Err(Error::NotFound(_))));
    // Too large to save.
    editor.set_size_limit(before.len() as u64 + 200);
    let big = parse_table(&"abcdefgh,abcdefgh\n".repeat(50), Delimiter::Comma, false).unwrap();
    let root = editor.document().root();
    let result = editor.apply(
        Command::Import {
            cell: root,
            table: big,
        },
        None,
        &mut ids,
    );
    assert!(matches!(result, Err(Error::Limit(_))), "{result:?}");
    assert_eq!(support::saved(editor.document()), before);
    assert_eq!(editor.undo_len(), 0);
}

// ---- Rules ----

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 200,
        ..ProptestConfig::default()
    }
}

/// The grid of a cell as rows of exported texts.
fn grid_texts(
    doc: &Document,
    calc: &Calc,
    id: CellId,
    options: &ExportOptions,
) -> Vec<Vec<String>> {
    let g = doc.cell(id).unwrap().grid().unwrap();
    g.cells()
        .chunks(g.xs() as usize)
        .map(|row| {
            let mut fields = Vec::new();
            for c in row {
                let cell = doc.cell(*c).unwrap();
                let text = if options.formulas {
                    &cell.text
                } else {
                    calc.shown(doc, *c).unwrap()
                };
                fields.push(text.to_string());
                if options.notes {
                    fields.push(cell.note.clone());
                }
            }
            fields
        })
        .collect()
}

fn guarded(rows: Vec<Vec<String>>, options: &ExportOptions) -> Vec<Vec<String>> {
    if !options.guard {
        return rows;
    }
    rows.into_iter()
        .map(|row| row.iter().map(|f| guard(f).into_owned()).collect())
        .collect()
}

/// What the importer makes of a field that the strict reader read: line breaks (only notes
/// have them) become spaces, as every control character.
fn as_imported(rows: Vec<Vec<String>>) -> Vec<Vec<String>> {
    rows.into_iter()
        .map(|row| {
            row.iter()
                .map(|f| wombat_core::text::clean(&f.replace("\r\n", "\n")))
                .collect()
        })
        .collect()
}

fn imported(text: &str, delimiter: Delimiter) -> Vec<Vec<String>> {
    let table = parse_table(&decode(text.as_bytes()), delimiter, false).unwrap();
    rows(&table)
        .into_iter()
        .map(|row| row.into_iter().map(String::from).collect())
        .collect()
}

proptest! {
    #![proptest_config(config())]

    /// A grid exported as delimited text is valid RFC 4180 with exactly the texts of its
    /// cells as fields, and the importer reads the same grid back. With the guard on, the
    /// texts that a spreadsheet would run come back with the quote in front, and no field
    /// is left that a spreadsheet would run.
    #[test]
    fn a_grid_comes_back_from_delimited_text(spec in spec(4), options in options()) {
        let doc = build(&spec, 3);
        let calc = Calc::new(&doc);
        let before = format::to_string(&doc).unwrap();
        let text = grid_csv(source(&doc, &calc), doc.root(), &options).unwrap();
        prop_assert_eq!(text.starts_with('\u{feff}'), options.bom);
        let expected = guarded(grid_texts(&doc, &calc, doc.root(), &options), &options);
        let strict = strict_csv(&text, options.delimiter.as_char(), options.crlf);
        prop_assert_eq!(&strict, &expected);
        prop_assert_eq!(imported(&text, options.delimiter), as_imported(expected));
        if options.guard {
            prop_assert!(strict.iter().flatten().all(|f| !is_risky(f)));
        }
        // The same bytes again, and the document as it was.
        prop_assert_eq!(&grid_csv(source(&doc, &calc), doc.root(), &options).unwrap(), &text);
        prop_assert_eq!(format::to_string(&doc).unwrap(), before);
    }

    /// The flattened export is valid, rectangular, has one line per innermost row with
    /// that row's cells at its level, and reads back as the same table.
    #[test]
    fn a_flattened_export_is_a_rectangle_of_the_innermost_rows(
        spec in spec(3),
        options in options(),
    ) {
        let doc = build(&spec, 3);
        let calc = Calc::new(&doc);
        let text = flat_csv(source(&doc, &calc), doc.root(), &options).unwrap();
        let strict = strict_csv(&text, options.delimiter.as_char(), options.crlf);
        let width = strict[0].len();
        prop_assert!(strict.iter().all(|row| row.len() == width));
        // The innermost rows, each with the rows around it, found another way.
        fn innermost(doc: &Document, id: CellId, trail: &mut Vec<Vec<CellId>>, out: &mut Vec<Vec<Vec<CellId>>>) {
            let g = doc.cell(id).unwrap().grid().unwrap();
            for row in g.cells().chunks(g.xs() as usize) {
                trail.push(row.to_vec());
                let nested: Vec<CellId> = row.iter().copied()
                    .filter(|c| doc.cell(*c).unwrap().grid().is_some()).collect();
                if nested.is_empty() {
                    out.push(trail.clone());
                }
                for c in nested {
                    innermost(doc, c, trail, out);
                }
                trail.pop();
            }
        }
        let mut lines = Vec::new();
        innermost(&doc, doc.root(), &mut Vec::new(), &mut lines);
        prop_assert_eq!(strict.len(), lines.len());
        let per_cell = if options.notes { 2 } else { 1 };
        let levels = lines.iter().map(Vec::len).max().unwrap();
        let widths: Vec<usize> = (0..levels)
            .map(|l| lines.iter().filter_map(|t| t.get(l)).map(Vec::len).max().unwrap())
            .collect();
        prop_assert_eq!(width, widths.iter().sum::<usize>() * per_cell);
        for (fields, trail) in strict.iter().zip(&lines) {
            let mut at = 0;
            for (level, level_width) in widths.iter().enumerate() {
                let cells = trail.get(level).map_or(&[][..], Vec::as_slice);
                for (x, c) in cells.iter().enumerate() {
                    let cell = doc.cell(*c).unwrap();
                    let own = if options.formulas { cell.text.as_str() } else { calc.shown(&doc, *c).unwrap() };
                    let own = if options.guard { guard(own).into_owned() } else { own.to_string() };
                    prop_assert_eq!(&fields[at + x * per_cell], &own);
                }
                // What is not a cell of this row is empty.
                for field in &fields[at + cells.len() * per_cell..at + level_width * per_cell] {
                    prop_assert_eq!(field, "");
                }
                at += level_width * per_cell;
            }
        }
        // A table that is all empty in one column reads back wider only by padding, never
        // narrower: the importer gives the same rectangle.
        prop_assert_eq!(imported(&text, options.delimiter), as_imported(strict));
        prop_assert_eq!(&flat_csv(source(&doc, &calc), doc.root(), &options).unwrap(), &text);
    }

    /// Where every grid has one column, the flattened export is the table that the
    /// Flatten command makes (which was compared with the real TreeSheets in 3b). Both go
    /// by what formula cells show (A83, A96): the command since the review of milestone
    /// 5, before which this test compared the formulas themselves.
    #[test]
    fn flattening_an_outline_is_what_the_flatten_command_makes(spec in spec(1)) {
        let doc = build(&spec, 3);
        let calc = Calc::new(&doc);
        let options = plain();
        let text = flat_csv(source(&doc, &calc), doc.root(), &options).unwrap();
        let mut editor = Editor::new(doc.clone()).unwrap();
        let root = doc.root();
        editor.apply(Command::Flatten { cell: root }, None, &mut SeededIds::from_u64(9)).unwrap();
        let flat = editor.document();
        let expected = grid_texts(flat, editor.calc(), root, &options);
        prop_assert_eq!(strict_csv(&text, ',', false), expected);
    }

    /// Indented text comes back as the same tree where the format can say it: grids of
    /// one column, and no text that starts with a space (which would read as indentation).
    #[test]
    fn an_outline_comes_back_from_indented_text(spec in spec(1), crlf in any::<bool>()) {
        let doc = build(&spec, 3);
        let calc = Calc::new(&doc);
        let whole = tree(&doc, &calc, doc.root(), false);
        fn sayable(tree: &Tree) -> bool {
            !tree.text.starts_with([' ', '\t'])
                && tree.grid.iter().flat_map(|(_, cells)| cells).all(sayable)
        }
        let text = outline(source(&doc, &calc), doc.root(), false, crlf).unwrap();
        let table = parse_outline(&text, MAX_DEPTH, false).unwrap();
        if whole.grid.iter().flat_map(|(_, cells)| cells).all(sayable) {
            prop_assert_eq!(Some(table_tree(&table)), texts(&whole).grid);
        }
        prop_assert_eq!(text.lines().count(), doc.cell_count() - 1);
        prop_assert_eq!(table.cell_count(), doc.cell_count() - 1);
    }

    /// The lookup export loses nothing and joins correctly: the tree of texts, notes and
    /// IDs rebuilt from the folder alone is the document's, every nested row finds its one
    /// parent row, every file is valid RFC 4180, the names are safe and distinct, and a
    /// second export gives the same bytes.
    #[test]
    fn the_lookup_export_is_complete_and_joins(spec in spec(3), mut options in options()) {
        options.guard = false;
        let doc = build(&spec, 3);
        let calc = Calc::new(&doc);
        let before = format::to_string(&doc).unwrap();
        let s = source(&doc, &calc);
        let files = lookup::export(s, doc.root(), &options, LookupOptions::default()).unwrap();
        let mut expected = tree(&doc, &calc, doc.root(), options.formulas);
        fn drop_notes(tree: &mut Tree, top: bool, keep: bool) {
            if top || !keep {
                tree.note.clear();
            }
            for cell in tree.grid.iter_mut().flat_map(|(_, cells)| cells) {
                drop_notes(cell, false, keep);
            }
        }
        drop_notes(&mut expected, true, options.notes);
        prop_assert_eq!(rebuild(&files, &options), expected);
        let grids = doc.cells_in_order().iter()
            .filter(|id| doc.cell(**id).unwrap().grid().is_some()).count();
        prop_assert_eq!(files.len(), grids + 2);
        let nested_rows: usize = doc.cells_in_order().iter()
            .filter(|id| **id != doc.root())
            .filter_map(|id| doc.cell(*id).unwrap().grid())
            .map(|g| g.ys() as usize).sum();
        prop_assert_eq!(check_joins(&doc, &files, &options), nested_rows);
        let mut seen = BTreeSet::new();
        for f in &files[2..] {
            let stem = f.name.strip_suffix(".csv").unwrap();
            check_name(stem);
            prop_assert!(seen.insert(stem.to_lowercase()));
        }
        prop_assert_eq!(&lookup::export(s, doc.root(), &options, LookupOptions::default()).unwrap(), &files);
        prop_assert_eq!(format::to_string(&doc).unwrap(), before);
    }

    /// With the guard on and with a header row, the files are still valid, rectangular,
    /// joined, and hold nothing a spreadsheet would run.
    #[test]
    fn the_lookup_export_holds_with_every_option(
        spec in spec(3),
        options in options(),
        header_row in any::<bool>(),
    ) {
        let doc = build(&spec, 3);
        let calc = Calc::new(&doc);
        let files = lookup::export(source(&doc, &calc), doc.root(), &options, LookupOptions { header_row }).unwrap();
        for f in &files[1..] {
            let table = read(f, &options);
            let lower: BTreeSet<String> = table.names.iter().map(|n| n.to_lowercase()).collect();
            prop_assert_eq!(lower.len(), table.names.len(), "column names differ");
            if options.guard {
                prop_assert!(table.names.iter().chain(table.rows.iter().flatten()).all(|f| !is_risky(f)));
            }
        }
        if !header_row {
            check_joins(&doc, &files, &options);
        }
    }

    /// Any labels give a file name that is valid on every system, and the names of one
    /// export differ.
    #[test]
    fn any_text_gives_safe_and_distinct_file_names(
        tables in vec((vec(any::<String>(), 0..4), any::<u128>()), 1..6),
        same_ids in any::<bool>(),
    ) {
        let tables: Vec<(Vec<&str>, CellId)> = tables.iter().enumerate()
            .map(|(n, (labels, bits))| {
                // Sometimes IDs that start alike, which random IDs never do.
                let bits = if same_ids { (*bits & 0xffff) | (n as u128) << 16 } else { *bits };
                (labels.iter().map(String::as_str).collect(), CellId(wombat_core::Id::from_u128(bits)))
            })
            .collect();
        let ids: BTreeSet<CellId> = tables.iter().map(|(_, id)| *id).collect();
        prop_assume!(ids.len() == tables.len());
        let stems = names::stems(&tables);
        let mut seen = BTreeSet::new();
        for stem in &stems {
            check_name(stem);
            // Distinct without regard to case, and by the ASCII suffix alone.
            prop_assert!(seen.insert(stem.rsplit('-').next().unwrap().to_lowercase()));
        }
        prop_assert_eq!(&names::stems(&tables), &stems);
    }

    /// Importing never panics or hangs, whatever the bytes: it gives an error or a table
    /// within the limits, which makes a valid document.
    #[test]
    fn arbitrary_bytes_import_or_fail_cleanly(
        bytes in prop_oneof![
            vec(any::<u8>(), 0..300),
            vec(proptest::sample::select(&b"a,;\t\"\r\n =-\xff\xfe"[..]), 0..300),
        ],
        delimiter in proptest::sample::select(&[Delimiter::Comma, Delimiter::Semicolon, Delimiter::Tab][..]),
        formulas in any::<bool>(),
        levels in 0..40u32,
    ) {
        let text = decode(&bytes);
        for table in [parse_table(&text, delimiter, formulas), parse_outline(&text, levels, formulas)] {
            match table {
                Ok(table) => {
                    prop_assert!(table.cell_count() < MAX_CELLS && table.height() <= MAX_DEPTH);
                    prop_assert_eq!(table.cells().len(), (table.xs() * table.ys()) as usize);
                    let doc = to_document(&table, &mut SeededIds::from_u64(1)).unwrap();
                    prop_assert_eq!(doc.cell_count(), table.cell_count() + 1);
                    support::check_document(&doc, false);
                }
                Err(error) => prop_assert!(matches!(error, Error::Invalid(_) | Error::Limit(_))),
            }
        }
    }

    /// A valid export with bytes changed, added or removed still imports or fails cleanly.
    #[test]
    fn damaged_exports_import_or_fail_cleanly(
        spec in spec(3),
        options in options(),
        edits in vec((any::<usize>(), any::<u8>(), 0..3u8), 1..6),
    ) {
        let doc = build(&spec, 3);
        let calc = Calc::new(&doc);
        let mut bytes = flat_csv(source(&doc, &calc), doc.root(), &options).unwrap().into_bytes();
        for (at, byte, kind) in edits {
            let at = at % (bytes.len() + 1);
            match kind {
                0 => bytes.insert(at, byte),
                1 if at < bytes.len() => bytes[at] = byte,
                _ if at < bytes.len() => { bytes.remove(at); }
                _ => {}
            }
        }
        let text = decode(&bytes);
        if let Ok(table) = parse_table(&text, options.delimiter, true) {
            let doc = to_document(&table, &mut SeededIds::from_u64(1)).unwrap();
            support::check_document(&doc, false);
        }
        let _ = parse_outline(&text, MAX_DEPTH, true);
    }

    /// Importing into any cell of any document is one command: the report is complete and
    /// tight, the document valid and within its bound, the cell has the table's texts with
    /// new IDs, and one undo gives back the same bytes.
    #[test]
    fn importing_into_a_cell_is_a_reversible_command(
        spec in spec(3),
        incoming in spec(3),
        pick in any::<prop::sample::Index>(),
        as_outline in any::<bool>(),
        formulas in any::<bool>(),
        now in proptest::option::of(0..1000i64),
    ) {
        let doc = build(&spec, 3);
        let from = build(&incoming, 4);
        let calc = Calc::new(&from);
        let options = ExportOptions { formulas: true, ..plain() };
        let table = if as_outline {
            let text = outline(source(&from, &calc), from.root(), false, false).unwrap();
            parse_outline(&text, MAX_DEPTH, formulas).unwrap()
        } else {
            let text = flat_csv(source(&from, &calc), from.root(), &options).unwrap();
            parse_table(&text, Delimiter::Comma, formulas).unwrap()
        };
        let cells = doc.cells_in_order();
        let target = cells[pick.index(cells.len())];
        let mut editor = Editor::new(doc.clone()).unwrap();
        let before = support::saved(&doc);
        let mut ids = SeededIds::from_u64(5);
        let changes = editor.apply(Command::Import { cell: target, table: table.clone() }, now, &mut ids).unwrap();
        let after = editor.document().clone();
        support::check_report(&doc, &after, &changes, "import");
        support::check_document(&after, false);
        support::check_bound(&editor, "import");
        prop_assert_eq!(after.cell(target).unwrap().text.as_str(), doc.cell(target).unwrap().text.as_str());
        let got = tree(&after, editor.calc(), target, true);
        prop_assert_eq!(texts(&got).grid, Some(table_tree(&table)));
        prop_assert_eq!(changes.created.len(), table.cell_count());
        prop_assert!(changes.created.iter().all(|id| doc.cell(*id).is_none()));
        for id in &changes.created {
            let cell = after.cell(*id).unwrap();
            prop_assert_eq!(cell.last_edit, now);
            let formula = formulas && cell.text.starts_with('=') && cell.text.chars().count() > 1;
            prop_assert_eq!(cell.kind == CellKind::Formula, formula);
        }
        prop_assert_eq!(editor.undo_len(), 1);
        let undone = editor.undo().unwrap().unwrap();
        support::check_report(&after, editor.document(), &undone, "undo of import");
        prop_assert_eq!(support::saved(editor.document()), before);
        support::check_bound(&editor, "undo of import");
        editor.redo().unwrap().unwrap();
        prop_assert_eq!(editor.document(), &after);
    }
}
