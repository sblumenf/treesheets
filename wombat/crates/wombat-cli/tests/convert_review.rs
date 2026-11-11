//! The fixes after the review of milestone 7, through the program itself: each test here
//! failed because of a finding before the fix was made. Everything happens in a fresh
//! folder under the build directory; devices and other special files are stood in for
//! by FIFOs, sockets and links made there.
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
        .join("convert_review")
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

/// The message of a command that must fail with `code`.
fn fails(dir: &Path, args: &[&str], code: i32) -> String {
    let output = wombat(dir, args);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert_eq!(
        output.status.code(),
        Some(code),
        "wombat {args:?}: {stderr}"
    );
    stderr
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

// ---- 3: the encoding is never guessed ----

#[test]
fn a_file_that_is_not_utf8_is_refused_with_the_place_and_read_only_as_told() {
    let dir = folder("encoding");
    fs::write(
        dir.join("bad.csv"),
        b"caf\xc3\xa9,\xe6\x97\xa5\xe6\x9c\xac,bad\xff\n",
    )
    .unwrap();
    let message = fails(&dir, &["import", "bad.csv", "bad.wombat"], 1);
    assert!(message.contains("byte 16"), "{message}");
    assert!(message.contains("--encoding"), "{message}");
    assert!(!dir.join("bad.wombat").exists());
    // UTF-16 without a byte-order mark is not guessed either.
    let utf16: Vec<u8> = "a,é\n".encode_utf16().flat_map(u16::to_le_bytes).collect();
    fs::write(dir.join("wide.csv"), &utf16).unwrap();
    let message = fails(&dir, &["import", "wide.csv", "wide.wombat"], 1);
    assert!(message.contains("--encoding"), "{message}");
    let out = ok(
        &dir,
        &[
            "import",
            "wide.csv",
            "wide.wombat",
            "--encoding",
            "utf-16le",
        ],
    );
    assert!(out.contains("UTF-16 (little-endian)"), "{out}");
    assert!(ok(&dir, &["show", "wide.wombat"]).contains("[2,1] \"é\""));
    // Windows-1252 only when asked for.
    fs::write(dir.join("old.csv"), b"caf\xe9,\x80 5\n").unwrap();
    let message = fails(&dir, &["import", "old.csv", "old.wombat"], 1);
    assert!(message.contains("byte 3"), "{message}");
    let out = ok(
        &dir,
        &[
            "import",
            "old.csv",
            "old.wombat",
            "--encoding",
            "windows-1252",
        ],
    );
    assert!(out.contains("Windows-1252"), "{out}");
    let shown = ok(&dir, &["show", "old.wombat"]);
    assert!(
        shown.contains("\"café\"") && shown.contains("\"€ 5\""),
        "{shown}"
    );
    // The usual case says what it was read as, and a byte-order mark is no text.
    fs::write(dir.join("fine.csv"), "\u{feff}é,b\n").unwrap();
    let out = ok(&dir, &["import", "fine.csv", "fine.wombat"]);
    assert!(out.contains("UTF-8"), "{out}");
    assert!(ok(&dir, &["show", "fine.wombat"]).contains("[1,1] \"é\""));
    // In `wombat edit` too.
    projects(&dir);
    let bad = edit(&dir, "projects.wombat", "import /3,3 bad.csv\n");
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("byte 16"));
    let good = edit(
        &dir,
        "projects.wombat",
        "import /3,3 old.csv windows-1252\nsave\n",
    );
    assert!(good.status.success());
    assert!(String::from_utf8_lossy(&good.stdout).contains("Windows-1252"));
}

// ---- 4: the delimiter of a .csv file is found, and said ----

#[test]
fn a_semicolon_file_named_csv_is_read_by_its_semicolons() {
    let dir = folder("delimiter");
    fs::write(dir.join("semi.csv"), "a;b;c\n1,5;2,5;3\n").unwrap();
    let out = ok(&dir, &["import", "semi.csv", "semi.wombat"]);
    assert!(out.contains("3 columns by 2 rows"), "{out}");
    assert!(out.contains("semicolon"), "{out}");
    assert!(ok(&dir, &["show", "semi.wombat"]).contains("[1,2] \"1,5\""));
    // Said otherwise, it is read otherwise.
    let out = ok(
        &dir,
        &["import", "semi.csv", "comma.wombat", "--delimiter", "comma"],
    );
    assert!(out.contains("3 columns by 2 rows"), "{out}");
    assert!(ok(&dir, &["show", "comma.wombat"]).contains("[1,1] \"a;b;c\""));
    fs::write(dir.join("tabs.csv"), "a\tb\n1,5\t2\n").unwrap();
    let out = ok(&dir, &["import", "tabs.csv", "tabs.wombat"]);
    assert!(
        out.contains("2 columns by 2 rows") && out.contains("tab"),
        "{out}"
    );
    fs::write(dir.join("plain.csv"), "a,b\n\"x;y\",2\n").unwrap();
    let out = ok(&dir, &["import", "plain.csv", "plain.wombat"]);
    assert!(out.contains("comma"), "{out}");
    assert!(ok(&dir, &["show", "plain.wombat"]).contains("[1,2] \"x;y\""));
}

// ---- 5: what an export may replace ----

#[cfg(unix)]
#[test]
fn an_export_replaces_only_a_regular_file_and_follows_no_link_with_a_separator_after_it() {
    let dir = folder("special");
    projects(&dir);
    assert!(Command::new("mkfifo")
        .arg(dir.join("pipe"))
        .status()
        .unwrap()
        .success());
    use std::os::unix::fs::FileTypeExt;
    for format in ["csv", "flat", "text"] {
        let message = fails(
            &dir,
            &[
                "export",
                "projects.wombat",
                "--format",
                format,
                "--overwrite",
                "pipe",
            ],
            1,
        );
        assert!(message.contains("not a regular file"), "{message}");
        assert!(fs::symlink_metadata(dir.join("pipe"))
            .unwrap()
            .file_type()
            .is_fifo());
    }
    fs::create_dir(dir.join("real")).unwrap();
    std::os::unix::fs::symlink(dir.join("real"), dir.join("link")).unwrap();
    for output in ["link", "link/"] {
        for overwrite in [&[][..], &["--overwrite"][..]] {
            let mut args = vec!["export", "projects.wombat", "--format", "lookup"];
            args.extend(overwrite);
            args.push(output);
            let message = fails(&dir, &args, 1);
            assert!(message.contains("symbolic link"), "{output}: {message}");
        }
    }
    assert_eq!(names(&dir.join("real")), [""; 0]);
    // The new document of an import: nothing that is there is replaced, whatever it is.
    fs::write(dir.join("t.csv"), "a,b\n").unwrap();
    std::os::unix::fs::symlink(dir.join("nowhere.wombat"), dir.join("dangling.wombat")).unwrap();
    for output in ["pipe.wombat", "dangling.wombat", "real.wombat/"] {
        if output == "pipe.wombat" {
            assert!(Command::new("mkfifo")
                .arg(dir.join(output))
                .status()
                .unwrap()
                .success());
        }
        let message = fails(&dir, &["import", "t.csv", output], 1);
        assert!(
            message.contains("already exists") || message.contains("not a file name"),
            "{output}: {message}"
        );
    }
    assert!(!dir.join("nowhere.wombat").exists());
    assert!(!dir.join("real.wombat").exists());
}

// ---- 6: tables of an earlier export ----

#[test]
fn overwriting_a_lookup_folder_removes_the_tables_of_the_export_before_and_nothing_else() {
    let dir = folder("stale");
    projects(&dir);
    ok(
        &dir,
        &["export", "projects.wombat", "--format", "lookup", "out"],
    );
    let before = names(&dir.join("out"));
    assert_eq!(before.len(), 5);
    fs::write(dir.join("out/mine.csv"), "mine").unwrap();
    fs::write(dir.join("out/notes.txt"), "notes").unwrap();
    // The grid inside Apollo goes; so must its table.
    let output = edit(&dir, "projects.wombat", "remove-grid /1,1\nsave\n");
    assert!(output.status.success());
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
    let after = names(&dir.join("out"));
    assert!(
        !after.iter().any(|name| name.starts_with("Apollo")),
        "{after:?}"
    );
    assert_eq!(after.len(), 4 + 2, "{after:?}");
    assert_eq!(
        fs::read_to_string(dir.join("out/mine.csv")).unwrap(),
        "mine"
    );
    assert_eq!(
        fs::read_to_string(dir.join("out/notes.txt")).unwrap(),
        "notes"
    );
    // A folder with things in it that is no export of Wombat's is not written into.
    fs::create_dir(dir.join("other")).unwrap();
    fs::write(dir.join("other/index.csv"), "my,own,index\n").unwrap();
    let message = fails(
        &dir,
        &[
            "export",
            "projects.wombat",
            "--format",
            "lookup",
            "--overwrite",
            "other",
        ],
        1,
    );
    assert!(message.contains("index.csv"), "{message}");
    assert_eq!(names(&dir.join("other")), ["index.csv"]);
}

// ---- 10: a file too large for a document ----

#[test]
fn a_file_with_more_cells_than_a_document_holds_is_refused_with_the_numbers() {
    let dir = folder("large");
    fs::write(dir.join("big.csv"), "a\n".repeat(600_000)).unwrap();
    let message = fails(&dir, &["import", "big.csv", "big.wombat"], 1);
    assert!(message.contains("600000 cells"), "{message}");
    assert!(message.contains("at most"), "{message}");
    assert!(!message.contains("the file would be larger"), "{message}");
    assert!(!dir.join("big.wombat").exists());
}

// ---- 11: importing over what is there ----

#[test]
fn an_import_over_cells_with_content_needs_the_word_replace_and_says_what_went() {
    let dir = folder("replace");
    projects(&dir);
    fs::write(dir.join("extra.csv"), "Launch,Eve\n").unwrap();
    // A link from Cygnus to "Design the logo", which is inside Apollo.
    let linked = edit(
        &dir,
        "projects.wombat",
        "link /1,3 0 6 cell /1,1/1,1\nsave\n",
    );
    assert!(linked.status.success());
    let before = ok(&dir, &["show", "projects.wombat"]);
    for target in ["/1,1", "/"] {
        let refused = edit(
            &dir,
            "projects.wombat",
            &format!("import {target} extra.csv\nsave\n"),
        );
        assert_eq!(refused.status.code(), Some(1));
        let message = String::from_utf8_lossy(&refused.stderr);
        assert!(message.contains("replace"), "{message}");
    }
    assert_eq!(ok(&dir, &["show", "projects.wombat"]), before);
    let replaced = edit(
        &dir,
        "projects.wombat",
        "import /1,1 extra.csv replace\nsave\n",
    );
    assert!(
        replaced.status.success(),
        "{}",
        String::from_utf8_lossy(&replaced.stderr)
    );
    let out = String::from_utf8_lossy(&replaced.stdout);
    assert!(out.contains("removed 6 cells"), "{out}");
    assert!(out.contains("1 link"), "{out}");
    // A cell without a grid, or with a grid of empty cells, takes an import as it is.
    let fresh = edit(
        &dir,
        "projects.wombat",
        "import /2,3 extra.csv\ngrid /3,3 2 2\nimport /3,3 extra.csv\nsave\n",
    );
    assert!(
        fresh.status.success(),
        "{}",
        String::from_utf8_lossy(&fresh.stderr)
    );
}

// ---- 13: error values in an export ----

#[test]
fn an_export_says_how_many_cells_went_out_as_error_values() {
    let dir = folder("errors");
    projects(&dir);
    let output = edit(
        &dir,
        "projects.wombat",
        "enter /3,3 \"=nope(\"\nenter /1,1/3,1 \"=1/0\"\nenter /2,3 \"=1+1\"\nsave\n",
    );
    assert!(output.status.success());
    let out = ok(
        &dir,
        &["export", "projects.wombat", "--format", "flat", "flat.csv"],
    );
    assert!(
        out.contains("2 cells were exported as error values"),
        "{out}"
    );
    // Of the top grid alone, one; as formulas, none.
    let out = ok(
        &dir,
        &["export", "projects.wombat", "--format", "csv", "top.csv"],
    );
    assert!(
        out.contains("1 cell was exported as an error value"),
        "{out}"
    );
    let out = ok(
        &dir,
        &[
            "export",
            "projects.wombat",
            "--format",
            "csv",
            "--formulas",
            "f.csv",
        ],
    );
    assert!(!out.contains("error value"), "{out}");
    let out = ok(
        &dir,
        &["export", "projects.wombat", "--format", "lookup", "out"],
    );
    assert!(
        out.contains("2 cells were exported as error values"),
        "{out}"
    );
    // To standard output the table is all that is printed there; the warning is apart.
    let output = wombat(
        &dir,
        &[
            "export",
            "projects.wombat",
            "--format",
            "csv",
            "--no-bom",
            "-",
        ],
    );
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("Apollo,"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("1 cell was exported"));
}

// ---- 9: two nested grids in one row ----

#[test]
fn a_flattened_export_names_the_parent_cell_where_a_row_has_several_grids() {
    let dir = folder("parents");
    ok(&dir, &["new", "two.wombat"]);
    let made = edit(
        &dir,
        "two.wombat",
        "grid / 2 1\ntext /1,1 \"P\"\ntext /2,1 \"Q\"\ngrid /1,1 1 2\ngrid /2,1 1 1\n\
         text /1,1/1,1 \"p1\"\ntext /1,1/1,2 \"p2\"\ntext /2,1/1,1 \"q1\"\nsave\n",
    );
    assert!(made.status.success());
    let flat = |file: &str, more: &[&str]| {
        let mut args = vec!["export", file, "--format", "flat", "--no-bom", "--lf"];
        args.extend(more);
        args.push("-");
        ok(&dir, &args)
    };
    assert_eq!(flat("two.wombat", &[]), "P,Q,p1,A\nP,Q,p2,A\nP,Q,q1,B\n");
    assert_eq!(
        flat("two.wombat", &["--no-parent-columns"]),
        "P,Q,p1\nP,Q,p2\nP,Q,q1\n"
    );
    // Where no row has more than one grid they are there only when asked for.
    projects(&dir);
    let plain = flat("projects.wombat", &[]);
    assert!(
        plain.starts_with("Apollo,Ana,2026,Design the logo,Ben,done\n"),
        "{plain}"
    );
    let asked = flat("projects.wombat", &["--parent-columns"]);
    assert!(
        asked.starts_with("Apollo,Ana,2026,Design the logo,Ben,done,A\n")
            && asked.ends_with("Cygnus,Eve,2028,,,,\n"),
        "{asked}"
    );
    assert!(ok(&dir, &["export", "--help"]).contains("--parent-columns"));
}
