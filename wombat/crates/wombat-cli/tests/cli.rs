//! Smoke tests of the `wombat` program itself.
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
        .join("cli")
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

const OUTLINE: &str = "Shopping\n  Fruit | Price\n  Apple | 1.20\n    red\n    green\n  Pear | 2\n";

#[test]
fn new_creates_a_readable_file_and_never_overwrites() {
    let dir = folder("new");
    let made = wombat(&dir, &["new", "a.wombat"]);
    assert!(made.status.success(), "{}", stderr(&made));
    assert!(stdout(&made).starts_with("created a.wombat: document "));
    let text = fs::read_to_string(dir.join("a.wombat")).unwrap();
    assert!(text.starts_with("{\n\"format\": \"wombat\", \"version\": 1, \"minor\": 0, \"id\": \""));
    assert_eq!(text.lines().count(), 4);

    let again = wombat(&dir, &["new", "a.wombat"]);
    assert_eq!(again.status.code(), Some(1));
    assert!(stderr(&again).contains("already exists"));
    assert_eq!(fs::read_to_string(dir.join("a.wombat")).unwrap(), text);

    // Two new documents never share an ID.
    assert!(wombat(&dir, &["new", "b.wombat"]).status.success());
    let other = fs::read_to_string(dir.join("b.wombat")).unwrap();
    assert_ne!(text.lines().nth(1), other.lines().nth(1));

    assert_eq!(wombat(&dir, &["new", "c.txt"]).status.code(), Some(2));
    assert!(!dir.join("c.txt").exists());
}

#[test]
fn outline_to_document_to_outline() {
    let dir = folder("outline");
    fs::write(dir.join("shop.txt"), OUTLINE).unwrap();
    let made = wombat(&dir, &["new", "shop.wombat", "--from", "shop.txt"]);
    assert!(made.status.success(), "{}", stderr(&made));
    assert!(stdout(&made).ends_with(", 9 cells\n"));

    let shown = wombat(&dir, &["show", "shop.wombat"]);
    assert!(shown.status.success());
    let lines: Vec<String> = stdout(&shown).lines().skip(1).map(String::from).collect();
    assert_eq!(
        lines,
        [
            "\"Shopping\"",
            "  grid 2 x 3",
            "  [1,1] \"Fruit\"",
            "  [2,1] \"Price\"",
            "  [1,2] \"Apple\"",
            "    grid 1 x 2",
            "    [1,1] \"red\"",
            "    [1,2] \"green\"",
            "  [2,2] \"1.20\"",
            "  [1,3] \"Pear\"",
            "  [2,3] \"2\"",
        ]
    );
    let with_ids = stdout(&wombat(&dir, &["show", "shop.wombat", "--ids"]));
    assert_eq!(with_ids.matches("  #").count(), 9);

    let checked = wombat(&dir, &["check", "shop.wombat"]);
    assert!(checked.status.success());
    assert!(stdout(&checked).contains("ok. document "));
    assert!(stdout(&checked).contains("9 cells, nested 2 deep"));
}

#[test]
fn outline_from_standard_input() {
    let dir = folder("stdin");
    let mut child = Command::new(env!("CARGO_BIN_EXE_wombat"))
        .current_dir(&dir)
        .args(["new", "in.wombat", "--from", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"a | b\nc | d\n")
        .unwrap();
    assert!(child.wait_with_output().unwrap().status.success());
    let shown = stdout(&wombat(&dir, &["show", "in.wombat"]));
    assert!(
        shown.contains("\"\"\n  grid 2 x 2\n  [1,1] \"a\"\n  [2,1] \"b\"\n"),
        "{shown}"
    );

    fs::write(dir.join("bad.txt"), "a\n    b\n  c\n").unwrap();
    let bad = wombat(&dir, &["new", "bad.wombat", "--from", "bad.txt"]);
    assert_eq!(bad.status.code(), Some(1));
    assert!(stderr(&bad).contains("line 3"));
    assert!(!dir.join("bad.wombat").exists());
}

#[test]
fn check_and_show_report_bad_files_without_crashing() {
    let dir = folder("check");
    assert!(wombat(&dir, &["new", "ok.wombat"]).status.success());
    let good = fs::read_to_string(dir.join("ok.wombat")).unwrap();
    let cases = [
        (
            "truncated.wombat",
            good[..good.len() / 2].to_string(),
            "not valid JSON",
        ),
        (
            "newer.wombat",
            good.replace("\"version\": 1", "\"version\": 2"),
            "newer Wombat",
        ),
        (
            "other.wombat",
            "{\"hello\": 1}".to_string(),
            "not a Wombat file",
        ),
        (
            "broken.wombat",
            good.replace("\"root\": {\"id\"", "\"root\": {\"idx\""),
            "no \"id\"",
        ),
        (
            "binary.wombat",
            "TSFF\u{1f}\u{0}".to_string(),
            "not valid JSON",
        ),
    ];
    for (name, content, message) in cases {
        fs::write(dir.join(name), content).unwrap();
        for command in ["check", "show"] {
            let result = wombat(&dir, &[command, name]);
            assert_eq!(result.status.code(), Some(1), "{command} {name}");
            assert!(
                stderr(&result).contains(message),
                "{name}: {}",
                stderr(&result)
            );
            assert!(stdout(&result).is_empty());
        }
    }
    let missing = wombat(&dir, &["check", "absent.wombat"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(stderr(&missing).contains("absent.wombat"));

    let listed = wombat(&dir, &["list"]);
    assert!(listed.status.success());
    let listed = stdout(&listed);
    assert_eq!(listed.lines().count(), 6);
    assert!(listed
        .lines()
        .any(|l| l.ends_with("  ok.wombat") && !l.starts_with("damaged")));
    assert!(listed
        .lines()
        .any(|l| l.starts_with("damaged") && l.contains("newer.wombat")));
    assert!(stdout(&wombat(&dir, &["list", "."])).contains("ok.wombat"));
    assert_eq!(
        wombat(&dir, &["list", "no-such-folder"]).status.code(),
        Some(1)
    );
}

#[test]
fn usage() {
    let dir = folder("usage");
    for args in [&[][..], &["help"], &["--help"]] {
        let result = wombat(&dir, args);
        assert!(result.status.success());
        assert!(stdout(&result).contains("wombat new <file.wombat>"));
    }
    assert!(stdout(&wombat(&dir, &["--version"])).starts_with("wombat "));
    for args in [
        &["frobnicate"][..],
        &["new"],
        &["show"],
        &["show", "a", "b", "c"],
    ] {
        let result = wombat(&dir, args);
        assert_eq!(result.status.code(), Some(2), "{args:?}");
        assert!(stderr(&result).contains("Usage:"));
    }
}
