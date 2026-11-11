//! `wombat export` and `wombat import` of text formats, through the program itself.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("convert")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn wombat(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wombat"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let output = wombat(dir, args);
    assert!(
        output.status.success(),
        "wombat {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn fails(dir: &Path, args: &[&str], code: i32, message: &str) {
    let output = wombat(dir, args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(code),
        "wombat {args:?}: {stderr}"
    );
    assert!(stderr.contains(message), "wombat {args:?}: {stderr}");
}

fn edit(dir: &Path, file: &str, script: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wombat"))
        .current_dir(dir)
        .args(["edit", file, "--script", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// The demo of the summary: projects, with tasks inside the first two.
const PROJECTS: &str = "\
Apollo | Ana | 2026
  Design the logo | Ben | done
  Write the brief | Ana | open
Borealis | Chidi | 2027
  Survey | Dee | open
Cygnus | Eve | 2028
";

fn projects(dir: &Path) {
    fs::write(dir.join("projects.txt"), PROJECTS).unwrap();
    ok(dir, &["new", "projects.wombat", "--from", "projects.txt"]);
}

#[test]
fn the_demo_of_the_summary_exports_tables_that_join() {
    let dir = folder("demo");
    projects(&dir);
    let before = fs::read(dir.join("projects.wombat")).unwrap();
    let out = ok(
        &dir,
        &["export", "projects.wombat", "--format", "lookup", "out"],
    );
    assert!(
        out.starts_with("exported projects.wombat to the folder out: 3 tables\n"),
        "{out}"
    );
    let files = names(&dir.join("out"));
    assert_eq!(files.len(), 5);
    assert_eq!(files[3], "index.csv");
    assert!(files[4].starts_with("table-"));
    let (apollo, borealis, top) = (&files[0], &files[1], &files[4]);
    assert!(apollo.starts_with("Apollo-") && borealis.starts_with("Borealis-"));
    assert_eq!(files[2], "README.txt");

    // The join, as XLOOKUP does it: parent_row_id of a task row finds one row of the top
    // table by its row_id, and that row is the project.
    let read = |name: &str| -> Vec<Vec<String>> {
        let text = fs::read_to_string(dir.join("out").join(name)).unwrap();
        let text = text
            .strip_prefix('\u{feff}')
            .expect("a byte-order mark by default");
        assert!(text.ends_with("\r\n"));
        text.lines()
            .map(|l| l.split(',').map(String::from).collect())
            .collect()
    };
    let (tasks, table) = (read(apollo), read(top));
    assert_eq!(
        tasks[0],
        [
            "A",
            "B",
            "C",
            "row_id",
            "A_id",
            "B_id",
            "C_id",
            "parent_table",
            "parent_row_id",
            "parent_cell_id"
        ]
    );
    assert_eq!(
        table[0],
        ["A", "B", "C", "row_id", "A_id", "B_id", "C_id", "A_table"]
    );
    for task in &tasks[1..] {
        let found: Vec<&Vec<String>> = table[1..].iter().filter(|row| row[3] == task[8]).collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0][0], "Apollo");
        assert_eq!(&found[0][7], apollo);
        assert_eq!(&task[7], top);
    }
    let readme = fs::read_to_string(dir.join("out/README.txt")).unwrap();
    let stem = top.trim_end_matches(".csv");
    assert!(
        readme.contains(&format!("=XLOOKUP(I2, '{stem}'!D:D, '{stem}'!A:A)")),
        "{readme}"
    );

    let flat = ok(
        &dir,
        &[
            "export",
            "projects.wombat",
            "--format",
            "flat",
            "--no-bom",
            "--lf",
            "-",
        ],
    );
    assert_eq!(
        flat,
        "Apollo,Ana,2026,Design the logo,Ben,done\n\
         Apollo,Ana,2026,Write the brief,Ana,open\n\
         Borealis,Chidi,2027,Survey,Dee,open\n\
         Cygnus,Eve,2028,,,\n"
    );
    // Exporting changes nothing.
    assert_eq!(fs::read(dir.join("projects.wombat")).unwrap(), before);
}

#[test]
fn each_format_and_option_writes_what_the_help_says() {
    let dir = folder("formats");
    projects(&dir);
    let export = |args: &[&str]| {
        let mut all = vec!["export", "projects.wombat"];
        all.extend(args);
        all.push("-");
        ok(&dir, &all)
    };
    assert_eq!(
        export(&["--format", "csv"]),
        "\u{feff}Apollo,Ana,2026\r\nBorealis,Chidi,2027\r\nCygnus,Eve,2028\r\n"
    );
    assert_eq!(
        export(&["--format", "tsv", "--no-bom", "--lf"]),
        "Apollo\tAna\t2026\nBorealis\tChidi\t2027\nCygnus\tEve\t2028\n"
    );
    assert_eq!(
        export(&["--format", "csv", "--treesheets", "--cell", "/1,2"]),
        "\"Survey\",\"Dee\",\"open\"\n"
    );
    assert_eq!(
        export(&[
            "--format",
            "csv",
            "--delimiter",
            "semicolon",
            "--no-bom",
            "--lf",
            "--quote-all",
            "--cell",
            "/1,2"
        ]),
        "\"Survey\";\"Dee\";\"open\"\n"
    );
    assert_eq!(
        export(&["--format", "text", "--cell", "/1,2"]),
        "Borealis\r\n  Survey\r\n  Dee\r\n  open\r\n"
    );
    assert_eq!(
        export(&["--format", "text", "--lf", "--cell", "/2,2"]),
        "Chidi\n"
    );
    fails(
        &dir,
        &[
            "export",
            "projects.wombat",
            "--format",
            "csv",
            "--cell",
            "/2,2",
            "-",
        ],
        1,
        "no grid",
    );
    fails(
        &dir,
        &["export", "projects.wombat", "--format", "xml", "-"],
        2,
        "not a format",
    );
    fails(&dir, &["export", "projects.wombat", "-"], 2, "--format");
    fails(
        &dir,
        &["export", "projects.wombat", "--format", "lookup", "-"],
        2,
        "folder",
    );
    fails(
        &dir,
        &[
            "export",
            "projects.wombat",
            "--format",
            "csv",
            "--bogus",
            "-",
        ],
        2,
        "unknown option",
    );
    assert!(ok(&dir, &["export", "--help"]).contains("--header-row"));
}

#[test]
fn formulas_and_risky_text_are_exported_safely_unless_asked_otherwise() {
    let dir = folder("guard");
    fs::write(dir.join("in.txt"), "=1+2 | @cmd | -5 | -run\n").unwrap();
    ok(&dir, &["new", "doc.wombat", "--from", "in.txt"]);
    let export = |args: &[&str]| {
        let mut all = vec![
            "export",
            "doc.wombat",
            "--format",
            "csv",
            "--no-bom",
            "--lf",
        ];
        all.extend(args);
        all.push("-");
        ok(&dir, &all)
    };
    // The formula cell shows 3; the text that only looks like one is guarded.
    assert_eq!(export(&[]), "3,'@cmd,-5,'-run\n");
    assert_eq!(export(&["--formulas"]), "'=1+2,'@cmd,-5,'-run\n");
    assert_eq!(export(&["--formulas", "--raw"]), "=1+2,@cmd,-5,-run\n");
    assert_eq!(export(&["--raw"]), "3,@cmd,-5,-run\n");
}

#[test]
fn an_export_never_replaces_or_follows_what_it_was_not_told_to() {
    let dir = folder("safety");
    projects(&dir);
    fs::write(dir.join("precious.csv"), "precious").unwrap();
    fails(
        &dir,
        &[
            "export",
            "projects.wombat",
            "--format",
            "csv",
            "precious.csv",
        ],
        1,
        "already exists",
    );
    assert_eq!(
        fs::read_to_string(dir.join("precious.csv")).unwrap(),
        "precious"
    );
    ok(
        &dir,
        &[
            "export",
            "projects.wombat",
            "--format",
            "csv",
            "--overwrite",
            "precious.csv",
        ],
    );
    assert!(fs::read_to_string(dir.join("precious.csv"))
        .unwrap()
        .contains("Apollo"));

    ok(
        &dir,
        &["export", "projects.wombat", "--format", "lookup", "out"],
    );
    fails(
        &dir,
        &["export", "projects.wombat", "--format", "lookup", "out"],
        1,
        "not empty",
    );
    ok(
        &dir,
        &[
            "export",
            "projects.wombat",
            "--format",
            "lookup",
            "--overwrite",
            "out",
        ],
    );
    assert_eq!(names(&dir.join("out")).len(), 5, "the same names again");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        fs::write(dir.join("victim"), "untouched").unwrap();
        symlink(dir.join("victim"), dir.join("link.csv")).unwrap();
        fails(
            &dir,
            &[
                "export",
                "projects.wombat",
                "--format",
                "csv",
                "--overwrite",
                "link.csv",
            ],
            1,
            "symbolic link",
        );
        fs::create_dir(dir.join("real")).unwrap();
        symlink(dir.join("real"), dir.join("linked")).unwrap();
        fails(
            &dir,
            &[
                "export",
                "projects.wombat",
                "--format",
                "lookup",
                "--overwrite",
                "linked",
            ],
            1,
            "symbolic link",
        );
        // A link planted at the name of a table in the folder.
        let table = names(&dir.join("out"))
            .into_iter()
            .find(|n| n.starts_with("Apollo-"))
            .unwrap();
        fs::remove_file(dir.join("out").join(&table)).unwrap();
        symlink(dir.join("victim"), dir.join("out").join(&table)).unwrap();
        fails(
            &dir,
            &[
                "export",
                "projects.wombat",
                "--format",
                "lookup",
                "--overwrite",
                "out",
            ],
            1,
            "symbolic link",
        );
        assert_eq!(fs::read_to_string(dir.join("victim")).unwrap(), "untouched");
        assert!(names(&dir.join("real")).is_empty());
    }
}

#[test]
fn text_files_are_imported_as_new_documents_by_their_names() {
    let dir = folder("import");
    fs::write(
        dir.join("t.csv"),
        "\u{feff}name,note\r\n\"Smith, J\",\"said \"\"hi\"\"\"\r\n=1+2,\r\n",
    )
    .unwrap();
    let out = ok(&dir, &["import", "t.csv", "t.wombat"]);
    assert!(out.contains("7 cells, 2 columns by 3 rows"), "{out}");
    let shown = ok(&dir, &["show", "t.wombat"]);
    assert!(
        shown.contains("[1,2] \"Smith, J\"") && shown.contains("[2,2] \"said \\\"hi\\\"\""),
        "{shown}"
    );
    // A file's formulas are plain text unless asked for.
    assert!(shown.contains("[1,3] \"=1+2\"\n"), "{shown}");
    ok(&dir, &["import", "t.csv", "f.wombat", "--formulas"]);
    assert!(ok(&dir, &["show", "f.wombat"]).contains("[1,3] \"=1+2\" (formula) = \"3\""));

    fs::write(dir.join("t.tsv"), "a\tb\n1\t2\n").unwrap();
    ok(&dir, &["import", "t.tsv", "tsv.wombat"]);
    assert!(ok(&dir, &["show", "tsv.wombat"]).contains("[2,2] \"2\""));
    fs::write(dir.join("semi.csv"), "a;b,c\n").unwrap();
    ok(
        &dir,
        &["import", "semi.csv", "semi.wombat", "--format", "semicolon"],
    );
    assert!(ok(&dir, &["show", "semi.wombat"]).contains("[2,1] \"b,c\""));
    fs::write(dir.join("o.txt"), "Fruit\n  Apple\n  Pear\nVeg\n").unwrap();
    ok(&dir, &["import", "o.txt", "o.wombat"]);
    let shown = ok(&dir, &["show", "o.wombat"]);
    assert!(
        shown.contains("  [1,1] \"Fruit\"\n    grid 1 x 2\n    [1,1] \"Apple\""),
        "{shown}"
    );

    // What it exports raw, it imports: the document comes back cell for cell. Exported
    // with the guard, which is the default, the text that looks like a formula comes back
    // with the quote that made it harmless.
    ok(
        &dir,
        &["export", "t.wombat", "--format", "csv", "--raw", "back.csv"],
    );
    ok(&dir, &["import", "back.csv", "back.wombat"]);
    let cells = |file: &str| {
        ok(&dir, &["show", file])
            .lines()
            .skip(1)
            .map(String::from)
            .collect::<Vec<_>>()
    };
    assert_eq!(cells("back.wombat"), cells("t.wombat"));
    ok(
        &dir,
        &["export", "t.wombat", "--format", "csv", "guarded.csv"],
    );
    ok(&dir, &["import", "guarded.csv", "guarded.wombat"]);
    assert_eq!(cells("guarded.wombat")[6], "  [1,3] \"'=1+2\"");

    fails(&dir, &["import", "t.csv", "t.wombat"], 1, "already exists");
    fails(&dir, &["import", "t.dat", "x.wombat"], 2, "--format");
    fails(
        &dir,
        &["import", "t.csv", "x.wombat", "--format", "xml"],
        2,
        "not a format",
    );
    fs::write(dir.join("empty.csv"), "\n\n").unwrap();
    fails(
        &dir,
        &["import", "empty.csv", "x.wombat"],
        1,
        "nothing to import",
    );
    assert!(!dir.join("x.wombat").exists());
    assert!(ok(&dir, &["import", "--help"]).contains("--formulas"));
}

#[test]
fn a_file_is_imported_into_a_cell_as_one_unit_of_undo() {
    let dir = folder("into_cell");
    projects(&dir);
    fs::write(dir.join("extra.csv"), "Launch,Eve,open\nReview,Ana,\n").unwrap();
    fs::write(dir.join("notes.txt"), "first\n  =2*3\n").unwrap();
    let script = "import /1,3 extra.csv\nchanges\nshow\nundo\nchanges\nredo\n\
                  import /2,3 notes.txt formulas\nsave\n";
    let output = edit(&dir, "projects.wombat", script);
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // One command made six cells in the grid of /1,3; one undo took them away again.
    assert_eq!(out.matches("created /1,3/").count(), 6, "{out}");
    assert_eq!(out.matches("removed (gone)").count(), 6, "{out}");
    assert!(
        out.contains("  [1,3] \"Cygnus\"\n    grid 3 x 2\n    [1,1] \"Launch\""),
        "{out}"
    );
    let shown = ok(&dir, &["show", "projects.wombat"]);
    assert!(shown.contains("[3,2] \"\"\n"), "{shown}");
    assert!(
        shown.contains("[1,1] \"=2*3\" (formula) = \"6\""),
        "{shown}"
    );
    assert!(ok(&dir, &["check", "projects.wombat"]).contains("ok"));

    let bad = edit(&dir, "projects.wombat", "import /1,1 nowhere.csv\n");
    assert_eq!(bad.status.code(), Some(1));
    let bad = edit(&dir, "projects.wombat", "import /9,9 extra.csv\n");
    assert_eq!(bad.status.code(), Some(1));
}

#[test]
fn a_treesheets_file_is_exported_on_the_spot_and_still_imported_as_before() {
    let dir = folder("cts");
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../TS/examples/sales.cts");
    let example = example.to_str().unwrap();
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../referee/golden/sales.txt");
    let text = ok(&dir, &["export", example, "--format", "text", "--lf", "-"]);
    // The menu export starts with the root's own line, which is empty here.
    assert_eq!(text, format!("\n{}", fs::read_to_string(golden).unwrap()));
    let out = ok(&dir, &["import", example, "sales.wombat"]);
    assert!(out.contains("TreeSheets format version"), "{out}");
    fails(
        &dir,
        &["import", example, "again.wombat", "--formulas"],
        2,
        "--formulas",
    );
}
