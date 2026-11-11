//! Exports compared with the real TreeSheets, byte for byte. Needs no TreeSheets at test
//! time: what it wrote is in `referee/golden/` and `referee/exports/golden/` (see
//! `referee/exports/make_exports` for how each file was made).
//!
//! - The indented text of every example and the fixture (38 documents), against
//!   `referee/golden/NAME.txt`. Those files were normalised in one way only: TreeSheets
//!   ends a line with CR LF on Linux, and the referee stored LF. Formula cells are there
//!   as what they show, so this also depends on the formulas being right.
//! - The CSV of the root's grid of the same 38 documents.
//! - Ten grids made for the purpose (`referee/exports/fixture.lobster`: quotes, empty
//!   rows, text that looks like a formula, formula cells, nested grids), as CSV, TSV and
//!   indented text, and the two menu exports of that whole document.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use wombat_core::convert::{self, Delimiter, ExportOptions, Source};
use wombat_core::eval::Calc;
use wombat_core::{CellId, Document, SeededIds};
use wombat_cts::import;

fn referee_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../referee")
}

fn cts_path(name: &str) -> PathBuf {
    match name.strip_prefix("fixtures/") {
        Some(fixture) => referee_dir().join(format!("fixtures/{fixture}.cts")),
        None => referee_dir().join(format!("../../TS/examples/{name}.cts")),
    }
}

fn names() -> Vec<String> {
    let text = fs::read_to_string(referee_dir().join("golden/MANIFEST.json")).unwrap();
    let json: Value = serde_json::from_str(&text).unwrap();
    json["files"].as_object().unwrap().keys().cloned().collect()
}

fn load(path: &Path) -> Document {
    import(&fs::read(path).unwrap(), &mut SeededIds::from_u64(1))
        .unwrap()
        .document
}

/// The first place where two texts differ, to make a failure readable.
fn first_difference(ours: &str, theirs: &str) -> String {
    let at = ours
        .bytes()
        .zip(theirs.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(ours.len().min(theirs.len()));
    let show = |text: &str| {
        let bytes =
            &text.as_bytes()[at.saturating_sub(30).min(text.len())..(at + 30).min(text.len())];
        String::from_utf8_lossy(bytes).into_owned()
    };
    format!(
        "at byte {at}: Wombat {:?}, TreeSheets {:?}",
        show(ours),
        show(theirs)
    )
}

#[test]
fn the_text_export_of_every_example_is_what_treesheets_wrote() {
    let names = names();
    assert_eq!(names.len(), 38);
    let mut bytes = 0;
    let mut differ = Vec::new();
    for name in &names {
        let doc = load(&cts_path(name));
        let calc = Calc::new(&doc);
        let source = Source {
            doc: &doc,
            calc: &calc,
        };
        let ours = convert::outline(source, doc.root(), false, false).unwrap();
        let theirs = fs::read_to_string(referee_dir().join(format!("golden/{name}.txt"))).unwrap();
        bytes += theirs.len();
        if ours != theirs {
            differ.push(format!("{name}: {}", first_difference(&ours, &theirs)));
        }
        // With CR LF it is what the app wrote before the referee normalised it.
        let crlf = convert::outline(source, doc.root(), false, true).unwrap();
        assert_eq!(crlf, theirs.replace('\n', "\r\n"), "{name}");
    }
    assert!(differ.is_empty(), "{}", differ.join("\n"));
    assert!(bytes > 400_000, "{bytes} bytes compared");
}

#[test]
fn the_csv_export_of_every_example_is_what_treesheets_wrote() {
    let names = names();
    let options = ExportOptions::treesheets(Delimiter::Comma);
    let mut differ = Vec::new();
    let mut bytes = 0;
    for name in &names {
        let doc = load(&cts_path(name));
        let calc = Calc::new(&doc);
        let source = Source {
            doc: &doc,
            calc: &calc,
        };
        let ours = convert::grid_csv(source, doc.root(), &options).unwrap();
        let path = referee_dir().join(format!("exports/golden/examples/{name}.csv"));
        let theirs = fs::read_to_string(path).unwrap();
        bytes += theirs.len();
        if ours != theirs {
            differ.push(format!("{name}: {}", first_difference(&ours, &theirs)));
        }
    }
    assert!(differ.is_empty(), "{}", differ.join("\n"));
    assert!(bytes > 5_000, "{bytes} bytes compared");
}

/// The cases of the fixture: the cells of the root's grid, by their text.
fn cases(doc: &Document) -> Vec<(String, CellId)> {
    let grid = doc.cell(doc.root()).unwrap().grid().unwrap();
    grid.cells()
        .iter()
        .map(|id| (doc.cell(*id).unwrap().text.clone(), *id))
        .collect()
}

fn golden(name: &str) -> String {
    fs::read_to_string(referee_dir().join("exports/golden").join(name)).unwrap()
}

/// TreeSheets' rule for a field of its TSV export (`Cell::ToText`), written out here to
/// show that the two cases that differ do so for this reason and no other: a field is
/// quoted only if it starts with a quote or holds a tab.
fn treesheets_tsv(doc: &Document, calc: &Calc, cell: CellId) -> String {
    let grid = doc.cell(cell).unwrap().grid().unwrap();
    let mut out = String::new();
    for row in grid.cells().chunks(grid.xs() as usize) {
        let fields: Vec<String> = row
            .iter()
            .map(|id| {
                let text = calc.shown(doc, *id).unwrap();
                if text.starts_with('"') || text.contains('\t') {
                    format!("\"{}\"", text.replace('"', "\"\""))
                } else {
                    text.to_string()
                }
            })
            .collect();
        out.push_str(&fields.join("\t"));
        out.push('\n');
    }
    out
}

#[test]
fn the_exports_of_the_fixture_are_what_treesheets_wrote() {
    let doc = load(&referee_dir().join("exports/fixture.cts"));
    let calc = Calc::new(&doc);
    let source = Source {
        doc: &doc,
        calc: &calc,
    };
    let cases = cases(&doc);
    assert_eq!(cases.len(), 10);
    let csv = ExportOptions::treesheets(Delimiter::Comma);
    let tsv = ExportOptions::treesheets(Delimiter::Tab);
    let mut tsv_differs = Vec::new();
    for (name, cell) in &cases {
        assert_eq!(
            convert::grid_csv(source, *cell, &csv).unwrap(),
            golden(&format!("{name}.csv")),
            "{name}.csv"
        );
        // The script function gives the text as the app has it on Linux: CR LF.
        assert_eq!(
            convert::outline(source, *cell, false, true).unwrap(),
            golden(&format!("{name}.txt")),
            "{name}.txt"
        );
        let theirs = golden(&format!("{name}.tsv"));
        assert_eq!(
            treesheets_tsv(&doc, &calc, *cell),
            theirs,
            "{name}.tsv by TreeSheets' rule"
        );
        let ours = convert::grid_csv(source, *cell, &tsv).unwrap();
        if ours != theirs {
            tsv_differs.push(name.as_str());
            // What Wombat wrote instead reads back as the grid; what TreeSheets wrote
            // does not (a quote in a bare field, or an empty line for a row).
            let table = convert::parse_table(&ours, Delimiter::Tab, false).unwrap();
            let grid = doc.cell(*cell).unwrap().grid().unwrap();
            let texts: Vec<&str> = table.cells().iter().map(|e| e.text()).collect();
            let cells: Vec<&str> = grid
                .cells()
                .iter()
                .map(|c| calc.shown(&doc, *c).unwrap())
                .collect();
            assert_eq!(texts, cells, "{name}");
        }
    }
    // Decision B81: where TreeSheets' TSV is not valid delimited text, Wombat's is.
    //  - quotes: TreeSheets writes `say "hi"` and `a"b` bare; a quote in a field that is
    //    not quoted is not allowed. Wombat quotes them.
    //  - blank, and the last row of tree: TreeSheets writes a row of one empty cell as an
    //    empty line, which no reader (its own included) takes for a row. Wombat writes `""`.
    assert_eq!(tsv_differs, ["quotes", "blank", "tree"]);
    assert_eq!(
        convert::grid_csv(source, cases[1].1, &tsv).unwrap(),
        "\"say \"\"hi\"\"\"\t\"\"\"quoted\"\"\"\na,b\tx;y\n lead\ttrail \nit's\t\"a\"\"b\"\n"
    );
    assert_eq!(
        convert::grid_csv(source, cases[3].1, &tsv).unwrap(),
        "\"\"\n\"\"\n"
    );

    // The menu exports of the whole document: the text starts with the root's own line.
    assert_eq!(
        convert::outline(source, doc.root(), true, true).unwrap(),
        golden("document.txt")
    );
    assert_eq!(
        convert::grid_csv(source, doc.root(), &csv).unwrap(),
        golden("document.csv")
    );
    // The menu and the script function write the same CSV.
    assert_eq!(golden("plain.menu.csv"), golden("plain.csv"));
}

#[test]
fn the_flattened_export_of_the_fixture_reads_as_expected() {
    // TreeSheets has no flattened export to compare with (its Flatten changes the
    // document, and was compared in milestone 3b); this pins the two nested cases.
    let doc = load(&referee_dir().join("exports/fixture.cts"));
    let calc = Calc::new(&doc);
    let source = Source {
        doc: &doc,
        calc: &calc,
    };
    let cases = cases(&doc);
    let options = ExportOptions {
        bom: false,
        crlf: false,
        ..ExportOptions::default()
    };
    let flat = |name: &str| {
        let cell = cases.iter().find(|(n, _)| n == name).unwrap().1;
        convert::flat_csv(source, cell, &options).unwrap()
    };
    assert_eq!(
        flat("wide"),
        "Apollo,Ana,,Design,done\nApollo,Ana,,Build,open\nBorealis,Chidi,,Survey,open\nCygnus,Eve,none,,\n"
    );
    // The second row has two cells with grids: the rows of both follow the same outer row.
    // Since the review of milestone 7 (A118) every line of such a table ends with the
    // columns that say which cell its inner rows are in: without them nothing told that
    // "in a cell without text" is inside the empty cell and Leek inside Veg.
    assert_eq!(
        flat("nested"),
        "Fruit,has text,Apple,red,,A,\nFruit,has text,Pear,green,,A,\n\
         ,Veg,in a cell without text,,,A,\n,Veg,Leek,,green,B,C\n,Veg,Kale,,,B,\nlast,,,,,,\n"
    );
    // Without them, the table as it was before.
    let bare = ExportOptions {
        parent_columns: Some(false),
        ..options.clone()
    };
    let nested = cases.iter().find(|(n, _)| n == "nested").unwrap().1;
    assert_eq!(
        convert::flat_csv(source, nested, &bare).unwrap(),
        "Fruit,has text,Apple,red,\nFruit,has text,Pear,green,\n\
         ,Veg,in a cell without text,,\n,Veg,Leek,,green\n,Veg,Kale,,\nlast,,,,\n"
    );
    assert_eq!(
        flat("tree"),
        "Projects,Apollo,Design\nProjects,Apollo,Build\nProjects,Borealis,\nNotes,one note,\n,,\n"
    );
}
