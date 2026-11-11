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
    let _ = fs::remove_dir_all(dir.with_extension("cache"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Where the program is told to keep its index caches in these tests: beside the
/// folder of the test, not in the cache directory of whoever runs them.
fn cache_base(dir: &Path) -> PathBuf {
    dir.with_extension("cache")
}

/// The cache entry of a document of the workspace `dir`, if there is one.
fn cache_entry(dir: &Path, kind: &str, name: &str) -> Option<PathBuf> {
    let of_workspaces = fs::read_dir(cache_base(dir)).ok()?;
    let entries = of_workspaces.map(|folder| folder.unwrap().path().join(kind).join(name));
    entries.into_iter().find(|entry| entry.is_file())
}

fn wombat(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wombat"))
        .current_dir(dir)
        .env("WOMBAT_CACHE_DIR", cache_base(dir))
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
    // No index is ever left in the folder itself.
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
    assert!(
        !dir.join(".wombat-index").exists(),
        "the index is not kept here"
    );
    // The cache options are those of `wombat search`, for all three commands.
    for option in ["--no-cache", "--rebuild", "--cache=fast", "--cache=verify"] {
        assert_eq!(ok(&dir, &["links", "notes.wombat", option]), links);
        assert_eq!(
            ok(&dir, &["backlinks", ".", "shop.wombat", "/3,1", option]),
            back
        );
        assert!(ok(&dir, &["broken", option, "."]).starts_with("0 links"));
    }
    let elsewhere = folder("survives-elsewhere");
    let there = elsewhere.to_str().unwrap();
    assert_eq!(
        ok(&dir, &["links", "notes.wombat", "--cache-dir", there]),
        links
    );
    assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 1);
    let missing = wombat(&dir, &["broken", ".", "--cache-dir"]);
    assert_eq!(missing.status.code(), Some(2));
    let entry = cache_entry(&dir, "links", "shop.wombat.idx");
    assert!(entry.is_some(), "one cache entry per document");
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
        // A cache folder inside the workspace is not used at all (decision A112), so
        // whatever is planted in it stays as it is. The cache was made elsewhere.
        assert!(fs::symlink_metadata(&link).is_ok_and(|found| found.is_symlink()));
        assert!(
            cache_entry(&dir, "links", "shop.wombat.idx").is_some(),
            "{test}"
        );
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

/// Found in review of milestone 4: a copied or moved cell took the name of its image along
/// and not the file.
#[test]
fn images_tags_and_folds_go_with_cells_between_documents() {
    let dir = folder("images");
    fs::write(
        dir.join("a.txt"),
        "A\n  pic\n    x\n    y\n  other\n  third\n",
    )
    .unwrap();
    fs::write(dir.join("b.txt"), "B\n  e1\n  e2\n  e3\n  has one\n").unwrap();
    ok(&dir, &["new", "a.wombat", "--from", "a.txt"]);
    ok(&dir, &["new", "b.wombat", "--from", "b.txt"]);
    for (folder, file, bytes) in [
        ("a.wombat.images", "pic.png", "picture of a"),
        ("a.wombat.images", "two.png", "second picture"),
        (
            "b.wombat.images",
            "pic.png",
            "another picture with the same name",
        ),
    ] {
        fs::create_dir_all(dir.join(folder)).unwrap();
        fs::write(dir.join(folder).join(file), bytes).unwrap();
    }
    edited(
        &dir,
        "a.wombat",
        "set /1,1 image=pic.png\nset /1,2 image=two.png\nset /1,3 image=pic.png\n\
         tag \"pic\" #ff0000 #00ff00\nfold /1,1\nsave\n",
    );
    edited(
        &dir,
        "b.wombat",
        "clear /1,1 /1,2 /1,3\nset /1,4 image=pic.png\nsave\n",
    );
    let read = |file: &str| fs::read_to_string(dir.join(file)).unwrap();
    let image = |folder: &str, file: &str| fs::read_to_string(dir.join(folder).join(file));

    // A copy: the file comes along. B has another picture called pic.png, so the copy's
    // gets a new name, and B's own cell still shows its own.
    let out = ok(&dir, &["copy-cell", "a.wombat", "/1,1", "b.wombat", "/1,1"]);
    assert!(out.contains("1 image file copied"), "{out}");
    assert_eq!(
        image("b.wombat.images", "pic.png").unwrap(),
        "another picture with the same name"
    );
    let b = wombat_core::storage::load_document(&dir.join("b.wombat")).unwrap();
    let cell = |doc: &wombat_core::Document, y: u32| {
        doc.cell(doc.child(doc.root(), 0, y).unwrap())
            .unwrap()
            .clone()
    };
    let renamed = cell(&b, 0).image.unwrap().file;
    assert_ne!(renamed, "pic.png");
    assert!(renamed.ends_with(".png"), "{renamed}");
    assert_eq!(image("b.wombat.images", &renamed).unwrap(), "picture of a");
    assert_eq!(cell(&b, 3).image.unwrap().file, "pic.png");
    assert!(ok(&dir, &["check", "b.wombat"]).ends_with("deep\n"));
    // The tag that the copied cell has comes along.
    assert!(read("b.wombat").contains("\"pic\""));
    assert!(b.tags().contains_key("pic"));

    // A second copy of the same cell finds the picture there already, under that name.
    let out = ok(&dir, &["copy-cell", "a.wombat", "/1,3", "b.wombat", "/1,2"]);
    assert!(out.contains("0 image files copied"), "{out}");
    let b = wombat_core::storage::load_document(&dir.join("b.wombat")).unwrap();
    assert_eq!(cell(&b, 1).image.unwrap().file, renamed);
    assert_eq!(
        fs::read_dir(dir.join("b.wombat.images")).unwrap().count(),
        2
    );

    // A move: the file goes over (no other two.png is there), and the source, which no
    // longer shows it, loses its copy; pic.png stays, another cell of A still shows it.
    let out = ok(&dir, &["move-cell", "a.wombat", "/1,2", "b.wombat", "/1,3"]);
    assert!(out.contains("1 image file copied"), "{out}");
    assert_eq!(
        image("b.wombat.images", "two.png").unwrap(),
        "second picture"
    );
    assert!(image("a.wombat.images", "two.png").is_err());
    assert_eq!(image("a.wombat.images", "pic.png").unwrap(), "picture of a");
    for file in ["a.wombat", "b.wombat"] {
        assert!(ok(&dir, &["check", file]).ends_with("deep\n"), "{file}");
    }

    // A fold moves with its cell, and is not copied.
    let folded = |file: &str| {
        let doc = wombat_core::storage::load_document(&dir.join(file)).unwrap();
        let view = wombat_core::storage::load_view_state(&dir, doc.id()).unwrap();
        view.map_or(0, |v| {
            doc.cells_in_order()
                .iter()
                .filter(|c| v.is_folded(**c))
                .count()
        })
    };
    assert_eq!((folded("a.wombat"), folded("b.wombat")), (1, 0));
    edited(&dir, "b.wombat", "clear /1,2\nsave\n");
    ok(&dir, &["move-cell", "a.wombat", "/1,1", "b.wombat", "/1,2"]);
    assert_eq!((folded("a.wombat"), folded("b.wombat")), (0, 1));

    // An image that is not there: refused before anything is written.
    fs::remove_file(dir.join("a.wombat.images").join("pic.png")).unwrap();
    edited(&dir, "b.wombat", "clear /1,1\nsave\n");
    let (a, b) = (read("a.wombat"), read("b.wombat"));
    let output = wombat(&dir, &["copy-cell", "a.wombat", "/1,3", "b.wombat", "/1,1"]);
    assert_eq!(output.status.code(), Some(1));
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("pic.png"), "{error}");
    assert_eq!((read("a.wombat"), read("b.wombat")), (a, b));
}
