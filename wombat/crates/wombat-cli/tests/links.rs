//! Smoke tests of the link commands of the `wombat` program: the demo of milestone 4.
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
        .join("cli_links")
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

/// Runs the program and returns what it printed; it must succeed.
fn ok(dir: &Path, args: &[&str]) -> String {
    let output = wombat(dir, args);
    let error = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{args:?}: {error}");
    String::from_utf8(output.stdout).unwrap()
}

/// Runs `wombat edit` with the script on standard input.
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

fn edited(dir: &Path, file: &str, script: &str) -> String {
    let output = edit(dir, file, script);
    let error = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{script}: {error}");
    String::from_utf8(output.stdout).unwrap()
}

const SHOP: &str =
    "Shop\n  Fruit | Price\n  Apple | 1.20\n    red\n    green\n  Pear | 2\n  Fig | 3\n";
const NOTES: &str = "Notes\n  see the apple here\n  and green\n";

/// Two documents; `notes` links to two cells of `shop`, `shop` to one of its own.
fn two(test: &str) -> PathBuf {
    let dir = folder(test);
    fs::write(dir.join("shop.txt"), SHOP).unwrap();
    fs::write(dir.join("notes.txt"), NOTES).unwrap();
    ok(&dir, &["new", "shop.wombat", "--from", "shop.txt"]);
    ok(&dir, &["new", "notes.wombat", "--from", "notes.txt"]);
    edited(
        &dir,
        "notes.wombat",
        "insert-rows / 3 1\n\
         link /1,1 8 5 cell-in shop.wombat /1,2\n\
         link /1,2 4 5 cell-in shop.wombat /1,2/1,2\nsave\n",
    );
    edited(&dir, "shop.wombat", "link /1,3 0 4 cell /1,2\nsave\n");
    dir
}

#[test]
fn a_link_survives_renaming_moving_and_restructuring_its_target() {
    let dir = two("survive");
    let links = ok(&dir, &["links", "notes.wombat"]);
    assert_eq!(
        links,
        "/1,1 \"see the apple here\" [8+5 \"apple\"] -> shop.wombat /1,2 \"Apple\" (in: \"Shop\")\n\
         /1,2 \"and green\" [4+5 \"green\"] -> shop.wombat /1,2/1,2 \"green\" (in: \"Shop\" > \"Apple\")\n\
         2 links, 0 of them pointing at nothing\n"
    );
    let back = ok(&dir, &["backlinks", ".", "shop.wombat", "/1,2"]);
    assert_eq!(
        back,
        "shop.wombat /1,2 \"Apple\" (in: \"Shop\")\n2 links to it\n  \
         notes.wombat /1,1 \"see the apple here\" (in: \"Notes\") [8+5 \"apple\"]\n  \
         shop.wombat /1,3 \"Pear\" (in: \"Shop\") [0+4 \"Pear\"]\n"
    );
    // No index was left in the folder: nobody asked for one.
    assert!(!dir.join(".wombat-index").exists());

    // Rename the target, move it, sort around it, wrap its neighbours, turn the grid.
    edited(
        &dir,
        "shop.wombat",
        "text /1,2 \"Bramley apple\"\nmove-rows / 2 1 4\nsort / 1 2 3 desc\n\
         wrap / 1 2 2 2\ntranspose /\nsave\n",
    );
    let links = ok(&dir, &["links", "notes.wombat"]);
    assert!(
        links.contains("-> shop.wombat /3,1 \"Bramley apple\" (in: \"Shop\")\n"),
        "{links}"
    );
    assert!(
        links.contains("-> shop.wombat /3,1/1,2 \"green\" (in: \"Shop\" > \"Bramley apple\")\n"),
        "{links}"
    );
    let back = ok(&dir, &["backlinks", ".", "shop.wombat", "/3,1"]);
    assert!(back.contains("2 links to it\n"), "{back}");
    assert!(back.contains("  shop.wombat /2,1/1,1 \"Pear\""), "{back}");
    assert_eq!(
        ok(&dir, &["broken", "."]),
        "0 links pointing at nothing in 2 documents\n"
    );
    // The same answers with the index kept on disk, the first time and from then on.
    for _ in 0..2 {
        assert_eq!(ok(&dir, &["links", "notes.wombat", "--cache"]), links);
        assert_eq!(
            ok(&dir, &["backlinks", ".", "shop.wombat", "/3,1", "--cache"]),
            back
        );
    }
    let entry = dir.join(".wombat-index/links/shop.wombat.idx");
    assert!(entry.is_file(), "one cache entry per document");
    // The index folder is not taken for a document.
    assert_eq!(ok(&dir, &["list", "."]).lines().count(), 2);
}

#[test]
fn deleting_a_target_is_reported_as_a_broken_link() {
    let dir = two("broken");
    edited(&dir, "shop.wombat", "remove-grid /1,2\nsave\n");
    let broken = ok(&dir, &["broken", ".", "--cache"]);
    assert_eq!(broken.lines().count(), 2, "{broken}");
    assert!(
        broken.starts_with(
            "notes.wombat /1,2 \"and green\" (in: \"Notes\") [4+5 \"green\"] -> BROKEN: there is no cell "
        ),
        "{broken}"
    );
    assert!(broken.contains(" in shop.wombat\n1 links pointing at nothing in 2 documents\n"));
    let links = ok(&dir, &["links", "notes.wombat"]);
    assert!(links.ends_with("2 links, 1 of them pointing at nothing\n"));

    // A document that goes away, and a file copied by hand.
    fs::copy(dir.join("notes.wombat"), dir.join("notes2.wombat")).unwrap();
    fs::remove_file(dir.join("shop.wombat")).unwrap();
    let broken = ok(&dir, &["broken", ".", "--cache"]);
    assert!(broken.contains("there is no document "), "{broken}");
    assert!(
        broken.contains("2 links pointing at nothing in 1 documents\n"),
        "{broken}"
    );
    assert!(
        broken.contains("warning: notes.wombat and notes2.wombat have the same document ID"),
        "{broken}"
    );
    let refused = wombat(&dir, &["backlinks", ".", "notes2.wombat", "/1,1"]);
    assert_eq!(refused.status.code(), Some(1));

    // Wrong arguments are usage errors; a missing file or cell is a failure.
    assert_eq!(wombat(&dir, &["backlinks", "."]).status.code(), Some(2));
    assert_eq!(wombat(&dir, &["links"]).status.code(), Some(2));
    assert_eq!(
        wombat(&dir, &["links", "none.wombat"]).status.code(),
        Some(1)
    );
    assert_eq!(wombat(&dir, &["broken", "nowhere"]).status.code(), Some(1));
    let missing = wombat(&dir, &["backlinks", ".", "notes.wombat", "/9,9"]);
    assert_eq!(missing.status.code(), Some(1));
    let help = ok(&dir, &["help"]);
    for word in ["links", "backlinks", "broken", "copy-cell", "move-cell"] {
        assert!(help.contains(&format!("wombat {word} ")), "{word}");
    }
    let help = ok(&dir, &["edit", "--help"]);
    assert!(help.contains("cell-in FILE CELL") && help.contains("resolve-unknown"));
}

#[test]
fn cells_copy_and_move_between_documents_with_their_links() {
    let dir = two("transfer");
    // Copy `Apple` (with red and green) from shop into the empty third row of notes.
    let copied = ok(
        &dir,
        &["copy-cell", "shop.wombat", "/1,2", "notes.wombat", "/1,3"],
    );
    assert!(copied.starts_with("copied 3 cells"), "{copied}");
    let shown = ok(&dir, &["show", "notes.wombat"]);
    assert!(shown.contains("[1,3] \"Apple\"") && shown.contains("[1,2] \"green\""));
    // The originals still have their two backlinks; the copies have none.
    let back = ok(&dir, &["backlinks", ".", "shop.wombat", "/1,2"]);
    assert!(back.contains("2 links to it"), "{back}");
    let back = ok(&dir, &["backlinks", ".", "notes.wombat", "/1,3"]);
    assert!(back.contains("0 links to it"), "{back}");

    // Move `Pear`, which links to `Apple`, into a new empty row of notes.
    edited(&dir, "notes.wombat", "insert-rows / 4 1\nsave\n");
    let before = ok(&dir, &["show", "shop.wombat", "--ids"]);
    let pear_id = before
        .lines()
        .find(|line| line.contains("\"Pear\""))
        .and_then(|line| line.split('#').nth(1))
        .unwrap()
        .to_string();
    let moved = ok(
        &dir,
        &["move-cell", "shop.wombat", "/1,3", "notes.wombat", "/1,4"],
    );
    assert!(moved.starts_with("moved 1 cells"), "{moved}");
    // It has the same ID in its new document, and its link now names the shop.
    let shown = ok(&dir, &["show", "notes.wombat", "--ids"]);
    assert!(shown.contains(&format!("[1,4] \"Pear\" links: 1  #{pear_id}")));
    let links = ok(&dir, &["links", "notes.wombat"]);
    assert!(
        links.contains("/1,4 \"Pear\" [0+4 \"Pear\"] -> shop.wombat /1,2 \"Apple\" (in: \"Shop\")"),
        "{links}"
    );
    let shop = ok(&dir, &["show", "shop.wombat", "--ids"]);
    assert!(!shop.contains(&pear_id) && shop.contains("[1,3] \"\""));
    assert_eq!(
        ok(&dir, &["broken", "."]),
        "0 links pointing at nothing in 2 documents\n"
    );
    for file in ["shop.wombat", "notes.wombat"] {
        ok(&dir, &["check", file]);
    }
    // Refused moves change neither file.
    let (shop, notes) = (
        fs::read(dir.join("shop.wombat")).unwrap(),
        fs::read(dir.join("notes.wombat")).unwrap(),
    );
    for args in [
        ["move-cell", "shop.wombat", "/1,2", "notes.wombat", "/1,1"],
        ["move-cell", "shop.wombat", "/", "notes.wombat", "/1,3"],
        ["move-cell", "shop.wombat", "/1,2", "shop.wombat", "/1,3"],
        ["copy-cell", "shop.wombat", "/1,2", "notes.wombat", "/"],
        ["copy-cell", "shop.wombat", "/7,7", "notes.wombat", "/1,1"],
    ] {
        assert_eq!(wombat(&dir, &args).status.code(), Some(1), "{args:?}");
    }
    assert_eq!(fs::read(dir.join("shop.wombat")).unwrap(), shop);
    assert_eq!(fs::read(dir.join("notes.wombat")).unwrap(), notes);
}

#[test]
fn an_imported_treesheets_link_without_a_target_can_be_given_one_later() {
    let dir = folder("unknown");
    // A document as the importer leaves it: a link that kept the text it named.
    fs::write(dir.join("a.txt"), "Top\n  go to target\n  other\n").unwrap();
    ok(&dir, &["new", "a.wombat", "--from", "a.txt"]);
    let path = dir.join("a.wombat");
    let text = fs::read_to_string(&path).unwrap();
    let with_link = text.replace(
        "\"text\": \"go to target\"",
        "\"text\": \"go to target\", \"links\": [{\"start\": 6, \"len\": 6, \"cts_target\": \"target\"}]",
    );
    assert_ne!(with_link, text);
    fs::write(&path, with_link).unwrap();
    let links = ok(&dir, &["links", "a.wombat"]);
    assert!(
        links.contains("-> NO TARGET: a TreeSheets link to a cell with the text \"target\""),
        "{links}"
    );
    assert!(ok(&dir, &["broken", "."]).starts_with("a.wombat /1,1"));
    // No cell has that text yet: nothing happens. Then one does.
    let out = edited(&dir, "a.wombat", "resolve-unknown\n");
    assert_eq!(out, "0 links got a target\n");
    let out = edited(
        &dir,
        "a.wombat",
        "text /1,2 \"target\"\nresolve-unknown\nsave\n",
    );
    assert!(out.starts_with("1 links got a target\n"), "{out}");
    let links = ok(&dir, &["links", "a.wombat"]);
    assert!(
        links.contains("[6+6 \"target\"] -> a.wombat /1,2 \"target\" (in: \"Top\")"),
        "{links}"
    );
    assert!(!fs::read_to_string(&path).unwrap().contains("cts_target"));
}

/// Found in review of milestone 4: the cache was written through symbolic links, so a
/// folder that came from elsewhere could make `--cache` overwrite any file of the user.
#[cfg(unix)]
#[test]
fn a_cache_that_is_a_symbolic_link_is_never_written_through() {
    use std::os::unix::fs::symlink;
    let plant = |test: &str, link: &str, to_dir: bool| {
        let dir = two(test);
        let outside = folder(&format!("{test}-outside"));
        let precious = outside.join("important.txt");
        fs::write(&precious, "do not touch").unwrap();
        let link = dir.join(link);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink(if to_dir { &outside } else { &precious }, &link).unwrap();
        for _ in 0..2 {
            let out = ok(&dir, &["broken", ".", "--cache"]);
            assert!(out.contains("0 links pointing at nothing"), "{out}");
        }
        assert_eq!(fs::read_to_string(&precious).unwrap(), "do not touch");
        let listed: Vec<_> = fs::read_dir(&outside).unwrap().collect();
        assert_eq!(listed.len(), 1, "{test}: something was written outside");
        // (A link where the temporary file goes is replaced, not followed.)
        let still = fs::symlink_metadata(&link).is_ok_and(|found| found.is_symlink());
        assert_eq!(still, !link.ends_with("shop.wombat.idx.tmp"), "{test}");
    };
    // The cache file of milestone 4, the folder, and the folders and files of now.
    plant("link-old-file", ".wombat-index/links.idx", false);
    plant("link-top", ".wombat-index", true);
    plant("link-kind", ".wombat-index/links", true);
    plant("link-entry", ".wombat-index/links/shop.wombat.idx", false);
    plant(
        "link-temp",
        ".wombat-index/links/shop.wombat.idx.tmp",
        false,
    );
}
