//! The `wombat` program and cell text of several lines: written with the escapes of
//! `wombat edit`, shown with the same escapes (so that a line of output is still one cell),
//! searched, exported and imported.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("line_breaks")
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

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn edit(dir: &Path, file: &str, script: &str) -> Output {
    fs::write(dir.join("script.txt"), script).unwrap();
    wombat(dir, &["edit", file, "--script", "script.txt"])
}

/// A 2 by 2 document: texts of several lines, one from each kind of line end.
fn made(dir: &Path) {
    assert!(wombat(dir, &["new", "a.wombat"]).status.success());
    let script = "\
grid / 2 2
text /1,1 \"two\\nlines\"
text /2,1 \"a\\r\\nb\\rc\\td\"
text /1,2 \"x\"
insert /1,2 1 \"\\u{d}\\u{a}\"
set /2,2 note=\"kept\\r\\nas it\\0is\"
save
";
    let done = edit(dir, "a.wombat", script);
    assert!(done.status.success(), "{}", stderr(&done));
}

/// The lines of `wombat show` for the four cells.
fn shown(dir: &Path, file: &str) -> Vec<String> {
    let out = wombat(dir, &["show", file]);
    assert!(out.status.success(), "{}", stderr(&out));
    stdout(&out)
        .lines()
        .filter(|line| line.trim_start().starts_with('['))
        .map(|line| line.trim_start().to_string())
        .collect()
}

#[test]
fn an_edit_script_writes_line_breaks_and_show_prints_each_cell_on_one_line() {
    let dir = folder("edit");
    made(&dir);
    // Every way to end a line is one break in the cell; a tab is a space; a note is
    // kept as it was given. Each cell is one line of output, with the script's escapes.
    assert_eq!(
        shown(&dir, "a.wombat"),
        [
            "[1,1] \"two\\nlines\"",
            "[2,1] \"a\\nb\\nc d\"",
            "[1,2] \"x\\n\"",
            "[2,2] \"\" note: \"kept\\r\\nas it\\0is\"",
        ]
    );
    assert!(wombat(&dir, &["check", "a.wombat"]).status.success());
    // In the file a break is the two characters \\n inside the string.
    let file = fs::read_to_string(dir.join("a.wombat")).unwrap();
    assert!(file.contains("\"text\": \"two\\nlines\""), "{file}");

    // What `show` prints can be written back as it stands.
    let script = "\
text /1,2 \"two\\nlines\"
set /1,1 note=\"kept\\r\\nas it\\0is\"
replace /2,1 1 1 \" \"
show
changes
save
";
    let done = edit(&dir, "a.wombat", script);
    assert!(done.status.success(), "{}", stderr(&done));
    assert_eq!(
        shown(&dir, "a.wombat"),
        [
            "[1,1] \"two\\nlines\" note: \"kept\\r\\nas it\\0is\"",
            "[2,1] \"a b\\nc d\"",
            "[1,2] \"two\\nlines\"",
            "[2,2] \"\" note: \"kept\\r\\nas it\\0is\"",
        ]
    );
}

#[test]
fn a_cell_of_several_lines_is_found_and_printed_on_one_line() {
    let dir = folder("search");
    made(&dir);
    // In a script: the phrase is found across the break, and shown as one line.
    let done = edit(
        &dir,
        "a.wombat",
        "find \"\\\"two lines\\\"\"\nfind \"b c\"\n",
    );
    assert!(done.status.success(), "{}", stderr(&done));
    assert_eq!(
        stdout(&done),
        "/1,1 «two lines»\n1 found\n/2,1 a «b» «c» d\n1 found\n"
    );
    // From the command line: the text with the script's escape, the match as one line.
    let found = wombat(&dir, &["search", "a.wombat", "\"two lines\""]);
    assert!(found.status.success(), "{}", stderr(&found));
    let out = stdout(&found);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0], "a.wombat /1,1 \"two\\nlines\" (in: \"\")",
        "{out}"
    );
    assert_eq!(lines[1], "    text: «two lines»");
    // Replacing the phrase takes the break with it.
    let replaced = wombat(&dir, &["replace", "a.wombat", "two lines", "one"]);
    assert!(replaced.status.success(), "{}", stderr(&replaced));
    assert_eq!(shown(&dir, "a.wombat")[0], "[1,1] \"one\"");
}

#[test]
fn a_cell_of_several_lines_goes_out_and_comes_back_as_delimited_text() {
    let dir = folder("convert");
    made(&dir);
    for (format, name) in [("csv", "out.csv"), ("tsv", "out.tsv")] {
        let out = wombat(
            &dir,
            &["export", "a.wombat", "--format", format, "--raw", name],
        );
        assert!(out.status.success(), "{}", stderr(&out));
        let back = format!("{format}.wombat");
        let made = wombat(&dir, &["import", name, &back]);
        assert!(made.status.success(), "{}", stderr(&made));
        assert_eq!(
            shown(&dir, &back),
            [
                "[1,1] \"two\\nlines\"",
                "[2,1] \"a\\nb\\nc d\"",
                "[1,2] \"x\\n\"",
                "[2,2] \"\"",
            ]
        );
    }
    // A field with a break is in quotes, the break as it is; rows end in CR LF.
    assert_eq!(
        fs::read_to_string(dir.join("out.csv")).unwrap(),
        "\u{feff}\"two\nlines\",\"a\nb\nc d\"\r\n\"x\n\",\r\n"
    );
    // Indented text has a line per cell: a break is a space there.
    let out = wombat(
        &dir,
        &["export", "a.wombat", "--format", "text", "--lf", "-"],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "\n  two lines\n  a b c d\n  x \n  \n");
    let help = stdout(&wombat(&dir, &["export", "--help"]));
    assert!(
        help.contains("a line break inside a text is written as a space"),
        "{help}"
    );
    let help = stdout(&wombat(&dir, &["import", "--help"]));
    assert!(
        help.contains("a line break inside a quoted field is kept"),
        "{help}"
    );
}

#[test]
fn a_cell_of_several_lines_cannot_be_made_a_tag_and_the_script_stops_there() {
    let dir = folder("tag");
    made(&dir);
    let before = fs::read(dir.join("a.wombat")).unwrap();
    let done = edit(&dir, "a.wombat", "tag-cells /1,2 /1,1\nsave\n");
    assert_eq!(done.status.code(), Some(1));
    assert!(stderr(&done).contains("line break"), "{}", stderr(&done));
    assert_eq!(fs::read(dir.join("a.wombat")).unwrap(), before);
    let done = edit(&dir, "a.wombat", "tag \"a\\nb\" #ffffff #000000\n");
    assert_eq!(done.status.code(), Some(1));
    assert!(stderr(&done).contains("line break"), "{}", stderr(&done));
}

#[test]
fn the_help_of_edit_names_the_escapes_it_reads() {
    let dir = folder("help");
    let help = stdout(&wombat(&dir, &["edit", "--help"]));
    assert!(help.contains("\\n (a line break"), "{help}");
    assert!(help.contains("\\r"), "{help}");
}
