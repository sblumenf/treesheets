//! The fixes after the review of milestone 7 (decisions A113 to A120): each test here
//! failed because of a finding before the fix was made.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod convert_support;

use std::fs;
use std::path::{Path, PathBuf};

use convert_support::{
    build, cell, cell_text, formula, grid, source, spec, spreadsheet_csv, strict_csv,
};
use proptest::prelude::*;
use wombat_core::convert::delimited::{detect, guard, is_risky, unguard};
use wombat_core::convert::lookup::{self, LookupOptions};
use wombat_core::convert::{
    error_cells, flat_csv, grid_csv, names, parse_outline, parse_table, Delimiter, ExportOptions,
    Quoting,
};
use wombat_core::eval::Calc;
use wombat_core::export_files::{write_file, write_folder};
use wombat_core::links::LinkIndex;
use wombat_core::{CellId, Command, Editor, Error, SeededIds};

fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("convert_review")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ---- 1: a separator that is not the chosen one ----

#[test]
fn a_semicolon_in_a_cell_does_not_shift_the_columns_of_a_lookup_table() {
    let mut spec = grid(
        "",
        2,
        vec![
            grid("Ana; Bob", 1, vec![cell("x;y"), cell("p,q")]),
            cell("plain"),
        ],
    );
    spec.grid.as_mut().unwrap().1[1].note = "a\ttab".into();
    let doc = build(&spec, 1);
    let calc = Calc::new(&doc);
    let options = ExportOptions {
        notes: true,
        ..ExportOptions::default()
    };
    let files = lookup::export(
        source(&doc, &calc),
        doc.root(),
        &options,
        LookupOptions::default(),
    )
    .unwrap();
    let mut tables = 0;
    for file in files.iter().filter(|f| f.name.ends_with(".csv")) {
        // Read with comma, semicolon and tab as separators, as LibreOffice's import
        // dialog proposes, the table is what a reader of commas alone sees.
        let seen = spreadsheet_csv(&file.text);
        assert_eq!(seen, strict_csv(&file.text, ',', true), "{}", file.name);
        let width = seen[0].len();
        assert!(seen.iter().all(|row| row.len() == width), "{}", file.name);
        if let Some(at) = seen[0].iter().position(|name| name == "row_id") {
            tables += 1;
            for row in &seen[1..] {
                assert!(CellId::parse(&row[at]).is_some(), "{:?}", row[at]);
            }
        }
    }
    assert_eq!(tables, 2);
}

// ---- 2: the guard ----

#[test]
fn the_guard_leaves_ordinary_text_alone_and_guards_what_could_run() {
    for plain in [
        "-",
        "--",
        "+",
        "---",
        "- -",
        "=",
        "- item one",
        "+ a plus point",
        "@handle",
        "@",
        "@ home",
        "+44 20 7946 0958",
        "+1 (555) 010-0100",
        "+1-555-0100",
        "-5",
        "+2.5",
        "-1,5",
        "-2e10",
        "-2026-03-04",
        "+1/2",
        "- Done! Next",
        "'quoted",
        "'",
        "'-",
        " =1+1",
        "a=b",
        "",
    ] {
        assert!(!is_risky(plain), "{plain:?}");
        assert_eq!(guard(plain), plain);
    }
    for risky in [
        "=1+1",
        "==",
        "= in prose",
        "= ",
        "=cmd|' /C calc'!A0",
        "+1+1",
        "-1+1",
        "-a",
        "-item",
        "-A1",
        "+SUM(1)",
        "-cmd|' /C calc'!A0",
        "- cmd|' /C calc'!A0",
        "- SUM(1)",
        "+ HYPERLINK (\"http://x\")",
        "- call mum (tomorrow)",
        "- (optional) thing",
        "- '\\\\host\\share\\[b.xlsx]S'!A1",
        "@SUM(1)",
        "@a (b)",
        "@x|y",
        "@'[b.xlsx]S'!A1",
        "\tx",
        "\rx",
        "\u{ff1d}1+1",
        "\u{ff0b}SUM(1)",
        "\u{ff0d}SUM(1)",
        "\u{ff20}SUM(1)",
        "-٣",
    ] {
        assert!(is_risky(risky), "{risky:?}");
        assert_eq!(guard(risky), format!("'{risky}"));
        assert_eq!(unguard(&guard(risky)), Some(risky));
    }
    // A text that its author began with a quote before something risky gets one more,
    // so that taking the guard off gives back exactly what was there.
    assert!(!is_risky("'=1+1"));
    assert_eq!(guard("'=1+1"), "''=1+1");
    assert_eq!(unguard("''=1+1"), Some("'=1+1"));
    assert_eq!(unguard("'=1+1"), Some("=1+1"));
    for kept in ["'quoted", "'", "''", "'-5", "plain", "", "=1+1"] {
        assert_eq!(unguard(kept), None, "{kept:?}");
    }
}

// ---- 5: only a regular file is ever replaced ----

#[cfg(unix)]
#[test]
fn overwrite_replaces_a_regular_file_and_nothing_else() {
    let dir = folder("regular");
    let fifo = dir.join("fifo");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap();
    assert!(made.success());
    let socket = dir.join("socket");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let is = |path: &Path| fs::symlink_metadata(path).unwrap().file_type();
    use std::os::unix::fs::FileTypeExt;
    for overwrite in [false, true] {
        for path in [&fifo, &socket] {
            let result = write_file(path, b"x", overwrite);
            assert!(result.is_err(), "{path:?} {overwrite}");
        }
        assert!(is(&fifo).is_fifo(), "the FIFO was replaced");
        assert!(is(&socket).is_socket(), "the socket was replaced");
    }
    // The same for a file of a folder export.
    // The same for a file of a folder export, in a folder that Wombat exported before.
    let out = dir.join("out");
    let index: &[u8] = b"table,trail,rows,columns,header_row,parent_table,parent_row_id,\
                         parent_column,cell_id,cell_text\nt-0.csv\n";
    let before: &[(&str, &[u8])] = &[("t-0.csv", b"zero"), ("index.csv", index)];
    write_folder(&out, before, false).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(out.join("t-1.csv"))
        .status()
        .unwrap();
    assert!(made.success());
    let files: &[(&str, &[u8])] = &[("t-1.csv", b"one"), ("index.csv", b"new")];
    match write_folder(&out, files, true) {
        Err(Error::Refused(message)) => assert!(message.contains("regular file"), "{message}"),
        other => panic!("{other:?}"),
    }
    assert!(is(&out.join("t-1.csv")).is_fifo());
    // Nothing was removed or written before that was seen.
    assert_eq!(fs::read(out.join("index.csv")).unwrap(), index);
    assert_eq!(fs::read(out.join("t-0.csv")).unwrap(), b"zero");
}

#[cfg(unix)]
#[test]
fn a_link_is_not_written_through_when_its_name_ends_in_a_separator() {
    let dir = folder("trailing");
    fs::create_dir(dir.join("real")).unwrap();
    std::os::unix::fs::symlink(dir.join("real"), dir.join("link")).unwrap();
    let files: &[(&str, &[u8])] = &[("t-1.csv", b"one")];
    for name in ["link", "link/", "link//", "link/."] {
        for overwrite in [false, true] {
            let result = write_folder(&dir.join(name), files, overwrite);
            assert!(
                matches!(result, Err(Error::Refused(_))),
                "{name} {result:?}"
            );
        }
    }
    assert_eq!(fs::read_dir(dir.join("real")).unwrap().count(), 0);
    // And a link to a file, named with a separator after it.
    fs::write(dir.join("precious"), "precious").unwrap();
    std::os::unix::fs::symlink(dir.join("precious"), dir.join("flink")).unwrap();
    for name in ["flink", "flink/"] {
        assert!(write_file(&dir.join(name), b"x", true).is_err(), "{name}");
    }
    assert_eq!(fs::read(dir.join("precious")).unwrap(), b"precious");
}

// ---- 8: names ----

#[test]
fn a_name_is_at_most_31_utf16_units_which_is_what_a_sheet_name_may_have() {
    let id = CellId(wombat_core::Id::from_u128(
        0x1234_5678_9abc_4def_8123_4567_89ab_cdef,
    ));
    // Letters outside the BMP are two units each.
    let label = "\u{20000}".repeat(30);
    let stems = names::stems(&[(vec![label.as_str()], id)]);
    assert_eq!(stems[0].encode_utf16().count(), 31, "{stems:?}");
    assert!(stems[0].ends_with("-12345678"));
    assert_eq!(stems[0].chars().count(), 11 + 9);
}

#[test]
fn only_the_tables_whose_ids_start_alike_get_a_longer_suffix() {
    let id = |bits: u128| CellId(wombat_core::Id::from_u128(bits));
    let a = id(0x1234_5678_0000_4000_8000_0000_0000_0001);
    let b = id(0x1234_5678_1111_4000_8000_0000_0000_0002);
    let c = id(0x9999_5678_0000_4000_8000_0000_0000_0003);
    let stems = names::stems(&[(vec!["x"], a), (vec!["X"], b), (vec!["other"], c)]);
    assert_eq!(
        stems,
        ["x-1234567800004000", "X-1234567811114000", "other-99995678"]
    );
    // IDs alike in their first sixteen digits (which random IDs never are): numbered.
    let d = id(0x1234_5678_0000_4000_9000_0000_0000_0004);
    let stems = names::stems(&[(vec!["x"], a), (vec!["y"], d), (vec!["other"], c)]);
    assert_eq!(stems, ["x-n1", "y-n2", "other-99995678"]);
}

// ---- 9: two nested grids in one row ----

#[test]
fn a_flattened_row_with_two_nested_grids_says_which_cell_each_line_is_inside_of() {
    let doc = build(
        &grid(
            "",
            2,
            vec![
                grid("P", 1, vec![cell("p1"), cell("p2")]),
                grid("Q", 1, vec![cell("q1")]),
                cell("solo"),
                cell(""),
            ],
        ),
        1,
    );
    let calc = Calc::new(&doc);
    let options = ExportOptions {
        bom: false,
        crlf: false,
        ..ExportOptions::default()
    };
    let text = flat_csv(source(&doc, &calc), doc.root(), &options).unwrap();
    // The last column names the column (of this table) of the cell the inner row is in.
    assert_eq!(text, "P,Q,p1,A\nP,Q,p2,A\nP,Q,q1,B\nsolo,,,\n");
    let _ = grid_csv(source(&doc, &calc), doc.root(), &options).unwrap();
}

// ---- 4: the delimiter of a file that does not say it ----

#[test]
fn the_delimiter_is_the_one_the_lines_agree_on() {
    use Delimiter::{Comma, Semicolon, Tab};
    for (text, expected) in [
        // Semicolons with decimal commas, with and without a line of names.
        ("a;b;c\n1,5;2,5;3\n", Some(Semicolon)),
        ("1,5;2,5\n3,5;4,5\n", Some(Semicolon)),
        ("\u{feff}a;b\r\n1;2\r\n", Some(Semicolon)),
        // Tabs under the name of CSV, with commas in the fields.
        ("a\tb\n1,5\t2\n", Some(Tab)),
        ("Smith, J\t5\nDoe, A\t6\n", Some(Tab)),
        ("a;b\tc;d\ne;f\tg;h\n", Some(Tab)),
        // Plain comma separated values, the other delimiters inside quotes or uneven.
        ("a,b\n1,2\n", Some(Comma)),
        ("a,b\n\"x;y\",2\n", Some(Comma)),
        ("a,b\n\"x;y;z\tw\",\"p\nq;r\"\n", Some(Comma)),
        ("name,note\nAna,likes; and tabs\nBen,\n", Some(Comma)),
        ("a,b\n\n\n1,2\n", Some(Comma)),
        // A quote in the middle of a field opens nothing.
        ("a;say \"hi, there\";c\n1;2;3\n", Some(Semicolon)),
        // One column of sentences: no delimiter makes a table of it.
        ("Hello, world\nNo commas here\nOne, two, three\n", None),
        ("first; second\nthird\n", None),
        ("just words\nmore words\n", None),
        // Both make a table, and the commas are not decimal commas: not clear.
        ("a,b;c\nd,e;f\n", None),
        ("1,x;2\n3,y;4\n", None),
        // One line, or none, shows nothing.
        ("a;b;c\n", None),
        ("a;b;c", None),
        ("", None),
        ("\n\n", None),
        // A quote that is not closed: the lines before it count, the rest is one field.
        ("a;b\nc;d\n\"e;f\ng;h\n", Some(Semicolon)),
        ("a;b\n\"e;f\ng;h\n", None),
    ] {
        assert_eq!(detect(text), expected, "{text:?}");
    }
    // Only the first hundred lines are looked at, and a long file costs no more.
    let long = format!("{}{}", "a;b\n".repeat(100), "x,y,z;1;2;3\n".repeat(50_000));
    assert_eq!(detect(&long), Some(Semicolon));
    // A first line longer than what is looked at shows nothing.
    let wide = format!("{}\n1;2\n", "a;".repeat(40_000));
    assert_eq!(detect(&wide), None);
}

// ---- 2, by rule ----

/// Whether a formula made of `text` could do something: an independent, cruder reading of
/// what the guard is for. A text that starts like a formula and calls a function, has a
/// DDE link or refers to another sheet.
fn could_do_something(text: &str) -> bool {
    let starts = text.starts_with(['=', '+', '-', '@']);
    let calls = text
        .char_indices()
        .any(|(at, c)| c == '(' && text[..at].ends_with(|b: char| b.is_alphabetic()));
    let refers = text
        .char_indices()
        .any(|(at, c)| c == '!' && text[at + 1..].starts_with(|n: char| n.is_ascii_alphabetic()));
    starts && (calls || text.contains('|') || refers)
}

fn risky_text() -> impl Strategy<Value = String> {
    prop_oneof![
        cell_text(),
        "[-+=@' ]{0,3}[-+=@'()|!$ a-zA-Z0-9.,/]{0,10}",
        "[-+][0-9 ()./-]{0,12}",
        "[-+@=][A-Z]{1,4}\\([0-9A-Z,;]{0,5}\\)",
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    /// The guard changes a text only by one quote in front; what it leaves alone is the
    /// same bytes; what it changes no longer starts like a formula; nothing that could do
    /// something is left alone; and taking the guard off gives back the text.
    #[test]
    fn the_guard_adds_one_quote_or_nothing_and_comes_off_again(text in risky_text()) {
        let guarded = guard(&text);
        if is_risky(&text) {
            prop_assert_eq!(guarded.as_ref(), format!("'{text}"));
        }
        if guarded != text.as_str() {
            prop_assert_eq!(guarded.strip_prefix('\''), Some(text.as_str()));
        }
        prop_assert!(!is_risky(&guarded), "{:?}", guarded);
        prop_assert!(!could_do_something(&guarded));
        if could_do_something(&text) {
            prop_assert!(is_risky(&text), "{:?} is left alone", text);
        }
        // What starts with = is always guarded, but for the sign alone.
        if text.starts_with('=') && text != "=" {
            prop_assert!(is_risky(&text));
        }
        // What starts with none of the characters is never touched.
        if !text.starts_with(['=', '+', '-', '@', '\t', '\r', '\n', '\'', '\u{ff1d}', '\u{ff0b}', '\u{ff0d}', '\u{ff20}']) {
            prop_assert_eq!(guarded.as_ref(), text.as_str());
        }
        prop_assert_eq!(unguard(&guarded).unwrap_or(&guarded), text.as_str());
    }
}

// ---- 1, by rule ----

fn any_options() -> impl Strategy<Value = ExportOptions> {
    (
        proptest::sample::select(&[Delimiter::Comma, Delimiter::Semicolon, Delimiter::Tab][..]),
        proptest::sample::select(&[Quoting::Minimal, Quoting::All][..]),
        any::<(bool, bool, bool, bool, bool)>(),
        proptest::option::of(any::<bool>()),
    )
        .prop_map(
            |(delimiter, quoting, (bom, crlf, guard, formulas, notes), parent_columns)| {
                ExportOptions {
                    delimiter,
                    quoting,
                    bom,
                    crlf,
                    guard,
                    formulas,
                    notes,
                    parent_columns,
                }
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, ..ProptestConfig::default() })]

    /// Every table of every export is the same table to a reader that splits at commas,
    /// semicolons and tabs at once, as a spreadsheet's import dialog proposes, as to one
    /// that knows the delimiter: rectangular, and with IDs in the ID columns.
    #[test]
    fn every_export_is_the_same_table_whichever_separators_a_spreadsheet_takes(
        spec in spec(3),
        options in any_options(),
        header_row in any::<bool>(),
    ) {
        let doc = build(&spec, 3);
        let calc = Calc::new(&doc);
        let s = source(&doc, &calc);
        let mut texts = vec![
            ("grid".to_string(), grid_csv(s, doc.root(), &options).unwrap()),
            ("flat".to_string(), flat_csv(s, doc.root(), &options).unwrap()),
        ];
        let files = lookup::export(s, doc.root(), &options, LookupOptions { header_row }).unwrap();
        texts.extend(files.into_iter().filter(|f| f.name.ends_with(".csv")).map(|f| (f.name, f.text)));
        for (name, text) in &texts {
            let seen = spreadsheet_csv(text);
            prop_assert_eq!(&seen, &strict_csv(text, options.delimiter.as_char(), options.crlf), "{}", name);
            let width = seen[0].len();
            prop_assert!(seen.iter().all(|row| row.len() == width), "{}", name);
            if name == "grid" || name == "flat" || name == lookup::INDEX_FILE {
                continue;
            }
            // A table of the lookup export: the key columns hold IDs, and the key of the
            // row is the ID of its first cell.
            if header_row {
                continue;
            }
            let key = seen[0].iter().position(|n| n == "row_id").unwrap();
            prop_assert_eq!(&seen[0][key + 1], "A_id");
            for row in &seen[1..] {
                for (n, field) in seen[0].iter().zip(row) {
                    if n.ends_with("_id") {
                        prop_assert!(CellId::parse(field).is_some(), "{}: {:?}", name, row);
                    }
                }
                prop_assert_eq!(&row[key], &row[key + 1]);
            }
        }
    }
}

// ---- 11: importing over what is there ----

#[test]
fn an_import_replaces_a_grid_with_content_only_when_asked() {
    let doc = build(
        &grid(
            "",
            2,
            vec![
                grid("full", 1, vec![cell("x"), cell("")]),
                grid("empty", 2, vec![cell(""), cell("")]),
                grid("deep", 1, vec![grid("", 1, vec![cell("")])]),
                cell("leaf"),
            ],
        ),
        1,
    );
    let root = doc.root();
    let at = |x, y| doc.child(root, x, y).unwrap();
    let table = || parse_table("a,b\n", Delimiter::Comma, false).unwrap();
    let mut ids = SeededIds::from_u64(9);
    let before = wombat_core::format::to_string(&doc).unwrap();
    let mut editor = Editor::new(doc.clone()).unwrap();
    for cell in [at(0, 0), at(0, 1), root] {
        let result = editor.apply(
            Command::Import {
                cell,
                table: table(),
                replace: false,
            },
            None,
            &mut ids,
        );
        match result {
            Err(Error::Refused(message)) => assert!(message.contains("replace"), "{message}"),
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(
        wombat_core::format::to_string(editor.document()).unwrap(),
        before
    );
    assert_eq!(editor.undo_len(), 0);
    // A cell without a grid, and one whose grid has nothing in it, take it unasked.
    for cell in [at(1, 0), at(1, 1)] {
        let changes = editor
            .apply(
                Command::Import {
                    cell,
                    table: table(),
                    replace: false,
                },
                None,
                &mut ids,
            )
            .unwrap();
        assert_eq!(changes.created.len(), 2);
    }
    // Asked, the grid goes: the report has the cells, and the link index, asked before,
    // the links that will point at nothing.
    let mut editor = Editor::new(doc.clone()).unwrap();
    let x = doc.child(at(0, 0), 0, 0).unwrap();
    editor
        .apply(
            Command::SetLink {
                cell: at(1, 1),
                start: 0,
                len: 4,
                target: Some(wombat_core::LinkTarget::Cell { doc: None, cell: x }),
            },
            None,
            &mut ids,
        )
        .unwrap();
    let mut index = LinkIndex::new();
    index.insert_document(editor.document());
    let inside = editor
        .document()
        .cell(at(0, 0))
        .unwrap()
        .grid()
        .unwrap()
        .cells()
        .to_vec();
    assert_eq!(index.would_break(editor.document(), &inside).len(), 1);
    let changes = editor
        .apply(
            Command::Import {
                cell: at(0, 0),
                table: table(),
                replace: true,
            },
            None,
            &mut ids,
        )
        .unwrap();
    assert_eq!(changes.removed.len(), 2);
    assert_eq!(changes.created.len(), 2);
}

// ---- 13: error values ----

#[test]
fn the_cells_exported_as_error_values_are_counted() {
    let doc = build(
        &grid(
            "",
            2,
            vec![
                formula("=nope("),
                grid(
                    "in",
                    1,
                    vec![formula("=1/0"), formula("=1+1"), cell("#N/A")],
                ),
                formula("=2*3"),
                cell("=text"),
            ],
        ),
        1,
    );
    let calc = Calc::new(&doc);
    let s = source(&doc, &calc);
    assert_eq!(error_cells(s, doc.root(), false), 1);
    assert_eq!(error_cells(s, doc.root(), true), 2);
    let inner = doc.child(doc.root(), 1, 0).unwrap();
    assert_eq!(error_cells(s, inner, true), 1);
    assert_eq!(
        error_cells(s, doc.child(doc.root(), 0, 0).unwrap(), true),
        0
    );
}

// ---- 2: taking the guard off a whole table ----

#[test]
fn a_table_exported_with_the_guard_comes_back_without_it_when_asked() {
    let doc = build(
        &grid(
            "",
            1,
            vec![
                grid("=1+2", 1, vec![cell("-run"), cell("'=kept")]),
                cell("- item"),
                cell("'quoted"),
            ],
        ),
        1,
    );
    let calc = Calc::new(&doc);
    let s = source(&doc, &calc);
    let guarded = ExportOptions {
        bom: false,
        ..ExportOptions::default()
    };
    let text = flat_csv(s, doc.root(), &guarded).unwrap();
    assert_eq!(
        text,
        "'=1+2,'-run\r\n'=1+2,''=kept\r\n- item,\r\n'quoted,\r\n"
    );
    let table = parse_table(&text, Delimiter::Comma, true).unwrap();
    let texts = |table: &wombat_core::convert::Table| -> Vec<(String, bool)> {
        table
            .cells()
            .iter()
            .map(|e| (e.text().to_string(), e.is_formula()))
            .collect()
    };
    let plain = |t: &str| (t.to_string(), false);
    let back = texts(&table.clone().unguarded(false));
    assert_eq!(
        back,
        [
            plain("=1+2"),
            plain("-run"),
            plain("=1+2"),
            plain("'=kept"),
            plain("- item"),
            plain(""),
            plain("'quoted"),
            plain("")
        ]
    );
    // With formulas asked for, what is a formula once the quote is off becomes one.
    let back = texts(&table.unguarded(true));
    assert_eq!(back[0], ("=1+2".to_string(), true));
    assert_eq!(back[3], plain("'=kept"));
    // In the tables inside an outline too.
    let outline = parse_outline("a\n  '=x\n  'y\n", 8, false).unwrap();
    let inner = outline.unguarded(false);
    let inner = inner.cells()[0].grid().unwrap();
    assert_eq!(texts(inner), [plain("=x"), plain("'y")]);
}
