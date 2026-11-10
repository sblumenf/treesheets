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

fn fixture() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../referee/fixtures/features.cts");
    path.to_str().unwrap().to_string()
}

#[test]
fn import_converts_a_treesheets_file_with_its_images_and_view() {
    let dir = folder("import");
    let done = wombat(&dir, &["import", &fixture(), "f.wombat"]);
    assert!(done.status.success(), "{}", stderr(&done));
    let said = stdout(&done);
    assert!(said.contains("(TreeSheets format version 31) as f.wombat: document "));
    assert!(said.contains(", 47 cells, nested 4 deep\n"), "{said}");
    assert!(said.contains("1 images in f.wombat.images\n"), "{said}");
    assert!(said.contains("1 folded grids"), "{said}");
    assert!(
        said.contains("links to a cell with the text \"no such cell\", which does not exist"),
        "{said}"
    );

    // The document is a valid Wombat file with nothing missing.
    let checked = wombat(&dir, &["check", "f.wombat"]);
    assert!(checked.status.success(), "{}", stderr(&checked));
    assert!(stdout(&checked).contains("47 cells, nested 4 deep"));
    assert!(!stdout(&checked).contains("missing"));
    let images: Vec<_> = fs::read_dir(dir.join("f.wombat.images")).unwrap().collect();
    assert_eq!(images.len(), 1);
    let image = fs::read(images[0].as_ref().unwrap().path()).unwrap();
    assert!(image.starts_with(b"\x89PNG"));
    assert_eq!(fs::read_dir(dir.join(".wombat-view")).unwrap().count(), 1);
    let text = fs::read_to_string(dir.join("f.wombat")).unwrap();
    assert!(text.contains("\"cts_target\": \"no such cell\""), "{text}");

    // The outline: text, kinds, notes, links, and the fold from the view state.
    let shown = stdout(&wombat(&dir, &["show", "f.wombat"]));
    assert!(
        shown.contains("  grid 4 x 7\n  [1,1] \"plain\"\n  [2,1] \"bold\"\n"),
        "{shown}"
    );
    assert!(
        shown.contains("  [3,4] \"=SUM(Numbers) - 10\" (formula)\n"),
        "{shown}"
    );
    assert!(shown.contains("  [2,3] \"a web link and a file link\" links: 2\n"));
    assert!(shown.contains("  [2,5] \"has a note\" note: \"The note, with"));
    assert!(
        shown.contains("  [1,7] \"folded\"\n    grid 2 x 2 (folded)\n"),
        "{shown}"
    );
    assert!(shown.contains("\u{1F600}"));
    // The .cts itself shows the same outline (under another document ID).
    let direct = wombat(&dir, &["show", &fixture()]);
    assert!(direct.status.success(), "{}", stderr(&direct));
    // Without the first line (the ID) and the image's name, which are new on every import.
    let tail = |text: &str| -> Vec<String> {
        let lines = text.lines().skip(1);
        lines
            .map(|line| line.split(" image: ").next().unwrap().to_string())
            .collect()
    };
    assert!(shown.contains("  [4,7] \"image\" image: "));
    assert_eq!(tail(&stdout(&direct)), tail(&shown));

    // Nothing is overwritten, and the workspace lists the new document.
    let again = wombat(&dir, &["import", &fixture(), "f.wombat"]);
    assert_eq!(again.status.code(), Some(1));
    assert!(stderr(&again).contains("already exists"));
    assert_eq!(fs::read_to_string(dir.join("f.wombat")).unwrap(), text);
    assert!(stdout(&wombat(&dir, &["list"])).contains("  f.wombat\n"));
}

#[test]
fn import_refuses_what_it_cannot_read() {
    let dir = folder("import-bad");
    fs::write(dir.join("not.cts"), "hello").unwrap();
    fs::write(dir.join("locked.cts"), b"TSFF\x1f\x01rest").unwrap();
    fs::write(dir.join("newer.cts"), b"TSFF\x20\0").unwrap();
    let mut cut = fs::read(fixture()).unwrap();
    cut.truncate(cut.len() / 2);
    fs::write(dir.join("cut.cts"), cut).unwrap();
    for (file, message) in [
        ("not.cts", "not a TreeSheets file"),
        ("locked.cts", "password-protected files are not supported"),
        ("newer.cts", "format version 32"),
        ("cut.cts", "damaged TreeSheets file"),
        ("missing.cts", "missing.cts"),
    ] {
        for args in [&["import", file, "out.wombat"][..], &["show", file]] {
            let output = wombat(&dir, args);
            assert_eq!(output.status.code(), Some(1), "{args:?}");
            assert!(
                stderr(&output).contains(message),
                "{args:?}: {}",
                stderr(&output)
            );
            assert!(!stderr(&output).contains("panicked"));
        }
        assert!(!dir.join("out.wombat").exists());
    }
    let wrong = wombat(&dir, &["import", &fixture(), "out.txt"]);
    assert_eq!(wrong.status.code(), Some(2));
    for args in [
        &["import"][..],
        &["import", "a.cts"],
        &["import", "a", "b", "c"],
    ] {
        assert_eq!(wombat(&dir, args).status.code(), Some(2), "{args:?}");
    }
}
