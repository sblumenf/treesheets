//! Smoke tests of `wombat search`, `replace` and `import-all`: the demo of milestone 6.
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
        .join("cli_search")
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

/// Runs `wombat edit` with the script on standard input; it must succeed.
fn edited(dir: &Path, file: &str, script: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wombat"))
        .current_dir(dir)
        .args(["edit", file, "--script", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(script.as_bytes()).unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    let error = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{script}: {error}");
    String::from_utf8(output.stdout).unwrap()
}

const SHOP: &str =
    "Shop\n  Fruit | Price\n  Apple | 1.20\n    red apple\n    green\n  Pear | 2\n  \
                    Pineapple | 3\n  todo | 0\n";
const NOTES: &str = "Notes\n  see the Apple here\n  日本語のテキスト\n  todo\n  Café Zürich\n";

/// Two documents: a shop with fruit, and notes with a link into the shop, a note and a tag.
fn two(test: &str) -> PathBuf {
    let dir = folder(test);
    fs::write(dir.join("shop.txt"), SHOP).unwrap();
    fs::write(dir.join("notes.txt"), NOTES).unwrap();
    ok(&dir, &["new", "shop.wombat", "--from", "shop.txt"]);
    ok(&dir, &["new", "notes.wombat", "--from", "notes.txt"]);
    edited(
        &dir,
        "notes.wombat",
        "link /1,1 8 5 cell-in shop.wombat /1,2\nset /1,2 note=\"about apples\"\n\
         tag \"todo\" #ffff00 #000000\nnow 1000\ninsert /1,4 0 \"The \"\nsave\n",
    );
    dir
}

#[test]
fn search_finds_ranks_and_narrows() {
    let dir = two("search");
    let found = ok(&dir, &["search", ".", "apple"]);
    let expected = "\
shop.wombat /1,2 \"Apple\" (in: \"Shop\")
    text: «Apple»
shop.wombat /1,2/1,1 \"red apple\" (in: \"Shop\" > \"Apple\")
    text: red «apple»
notes.wombat /1,1 \"see the Apple here\" (in: \"Notes\")
    text: see the «Apple» here
shop.wombat /1,4 \"Pineapple\" (in: \"Shop\")
    text: Pine«apple»
notes.wombat /1,2 \"日本語のテキスト\" (in: \"Notes\")
    note: about «apple»s
5 results in 18 cells of 2 documents
";
    assert_eq!(found, expected);
    // Several arguments are one query; whatever the case, accents or script.
    assert_eq!(
        ok(&dir, &["search", ".", "APPLE", "red"]).lines().count(),
        3
    );
    assert!(ok(&dir, &["search", ".", "cafe zurich"]).contains("«Café» «Zürich»"));
    assert!(ok(&dir, &["search", ".", "のテキ"]).contains("text: 日本語«のテキ»スト"));
    assert!(ok(&dir, &["search", ".", "\"the apple\""]).starts_with("notes.wombat /1,1 "));
    // Narrowed by tag, by document, by branch, by time.
    let tagged = ok(&dir, &["search", ".", "tag:todo"]);
    assert_eq!(
        tagged,
        "notes.wombat /1,3 \"todo\" (in: \"Notes\")\n1 results in 18 cells of 2 documents\n"
    );
    assert!(ok(&dir, &["search", ".", "todo", "-has:tag"]).starts_with("shop.wombat /1,5 \"todo\""));
    assert!(ok(&dir, &["search", ".", "apple", "doc:notes"])
        .ends_with("2 results in 18 cells of 2 documents\n"));
    assert!(ok(&dir, &["search", "shop.wombat", "apple"])
        .ends_with("3 results in 18 cells of 2 documents\n"));
    let branch = ok(
        &dir,
        &["search", ".", "apple", "--in", "shop.wombat", "/1,2"],
    );
    assert_eq!(branch.lines().count(), 5, "{branch}");
    assert!(branch.contains("red «apple»") && !branch.contains("Pine"));
    let recent = ok(&dir, &["search", ".", "edited:1h", "--now", "2000"]);
    assert!(
        recent.starts_with("notes.wombat /1,4 \"The Café Zürich\""),
        "{recent}"
    );
    assert!(ok(&dir, &["search", ".", "edited:1h", "--now", "9999999"]).starts_with("0 results"));
    assert!(ok(&dir, &["search", ".", "has:link"]).contains("see the Apple here"));
    // Pages.
    let page = ok(
        &dir,
        &["search", ".", "apple", "--limit", "2", "--offset", "1"],
    );
    assert!(
        page.ends_with("5 results in 18 cells of 2 documents, 2 to 3 shown\n"),
        "{page}"
    );
    assert_eq!(page.lines().count(), 5);
    // The next and the previous match from a cell, round the document.
    let next = ok(
        &dir,
        &["search", ".", "apple", "--next", "shop.wombat", "/1,2"],
    );
    assert!(
        next.starts_with("shop.wombat /1,2/1,1 \"red apple\""),
        "{next}"
    );
    let next = ok(
        &dir,
        &["search", ".", "apple", "--next", "shop.wombat", "/1,4"],
    );
    assert!(next.starts_with("shop.wombat /1,2 \"Apple\""), "{next}");
    let prev = ok(
        &dir,
        &["search", ".", "apple", "--prev", "shop.wombat", "/1,2"],
    );
    assert!(prev.starts_with("shop.wombat /1,4 \"Pineapple\""), "{prev}");
    let none = ok(
        &dir,
        &["search", ".", "zebra", "--next", "shop.wombat", "/"],
    );
    assert_eq!(none, "no match in this document\n");
    // The same answers with the index kept on disk, the first time and from then on.
    assert!(
        !dir.join(".wombat-index").exists(),
        "no cache was asked for"
    );
    for mode in ["--cache", "--cache", "--cache=verify", "--cache"] {
        assert_eq!(ok(&dir, &["search", ".", "apple", mode]), expected);
    }
    assert!(dir.join(".wombat-index/search/shop.wombat.idx").is_file());
    let timed = ok(&dir, &["search", ".", "apple", "--cache", "--time"]);
    assert!(
        timed.contains("(0 documents read, 2 from the cache); search: "),
        "{timed}"
    );
    // A document changed since: found as it is now.
    edited(&dir, "shop.wombat", "text /1,3 \"Apple tart\"\nsave\n");
    let after = ok(&dir, &["search", ".", "apple", "--cache", "--time"]);
    assert!(
        after.contains("\"Apple tart\"") && after.contains("(1 documents read, 1 from the cache)"),
        "{after}"
    );
    // Mistakes are said, not crashed on.
    for bad in [
        vec!["search"],
        vec!["search", ".", "--limit"],
        vec!["search", ".", "--limit", "many"],
        vec!["search", ".", "--in", "shop.wombat"],
    ] {
        assert_eq!(wombat(&dir, &bad).status.code(), Some(2), "{bad:?}");
    }
    for bad in [
        vec!["search", "nowhere", "x"],
        vec!["search", ".", "has:wings"],
        vec!["search", ".", "x", "--in", "other.wombat", "/"],
        vec!["search", ".", "x", "--in", "shop.wombat", "/9,9"],
    ] {
        let output = wombat(&dir, &bad);
        assert_eq!(output.status.code(), Some(1), "{bad:?}");
        assert!(!output.stderr.is_empty());
    }
    let long = "a".repeat(5000);
    assert_eq!(wombat(&dir, &["search", ".", &long]).status.code(), Some(1));
    assert!(ok(&dir, &["search", "--help"]).contains("tag:NAME"));
    assert!(ok(&dir, &["help"]).contains("wombat search"));
}

#[test]
fn replace_changes_the_files_and_the_edit_script_can_undo_it() {
    let dir = two("replace");
    let dry = ok(&dir, &["replace", ".", "apple", "Fig", "--dry-run"]);
    assert_eq!(
        dry,
        "notes.wombat: 1 places in 1 cells\nshop.wombat: 3 places in 3 cells\n\
         4 places in 4 cells of 2 documents would be replaced\n"
    );
    assert!(ok(&dir, &["search", ".", "fig"]).starts_with("0 results"));
    // In one branch of one document, and only exactly these letters.
    let branch = ok(
        &dir,
        &[
            "replace",
            ".",
            "apple",
            "kiwi",
            "--case",
            "--in",
            "shop.wombat",
            "/1,2",
        ],
    );
    assert!(
        branch.ends_with("1 places in 1 cells of 1 documents replaced\n"),
        "{branch}"
    );
    assert!(ok(&dir, &["show", "shop.wombat"]).contains("\"red kiwi\""));
    // Everywhere.
    let all = ok(&dir, &["replace", ".", "apple", "Fig"]);
    assert!(
        all.ends_with("3 places in 3 cells of 2 documents replaced\n"),
        "{all}"
    );
    let shop = ok(&dir, &["show", "shop.wombat"]);
    assert!(
        shop.contains("[1,2] \"Fig\"") && shop.contains("\"PineFig\""),
        "{shop}"
    );
    assert!(ok(&dir, &["show", "notes.wombat"]).contains("\"see the Fig here\""));
    // The link on the replaced word is still there and still leads to the cell.
    let links = ok(&dir, &["links", "notes.wombat"]);
    assert!(
        links.contains("[8+3 \"Fig\"] -> shop.wombat /1,2 \"Fig\""),
        "{links}"
    );
    assert!(ok(&dir, &["replace", "shop.wombat", "zebra", "x"]).starts_with("0 places"));
    assert_eq!(wombat(&dir, &["replace", ".", "a"]).status.code(), Some(2));
    assert!(ok(&dir, &["replace", "--help"]).contains("--dry-run"));

    // In `wombat edit`: one line, one unit of undo, and undone to the same bytes.
    let before = fs::read(dir.join("shop.wombat")).unwrap();
    let out = edited(
        &dir,
        "shop.wombat",
        "find \"fig\"\nreplace-all \"FIG\" \"Plum\"\nfind \"plum\" next /1,2\nfind \"plum\" prev /1,2\n\
         replace-all \"plum\" \"x\" case /1,4\nshow\nundo\nsave\n",
    );
    assert!(
        out.starts_with("/1,2 «Fig»\n/1,4 Pine«Fig»\n2 found\n2 places in 2 cells\n"),
        "{out}"
    );
    assert!(
        out.contains("/1,4 Pine«Plum»\n1 found\n/1,4 Pine«Plum»\n1 found\n"),
        "{out}"
    );
    assert!(
        out.contains("0 places in 0 cells\n") && out.contains("\"PinePlum\""),
        "{out}"
    );
    assert_eq!(fs::read(dir.join("shop.wombat")).unwrap(), before);
}

#[test]
fn every_treesheets_example_can_be_imported_into_one_folder_and_searched() {
    let dir = folder("examples");
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../TS/examples");
    let out = ok(&dir, &["import-all", examples.to_str().unwrap(), "all"]);
    assert!(
        out.contains("imported 37 of 37 files") || out.contains(" files, "),
        "{out}"
    );
    let listed = ok(&dir, &["list", "all"]);
    assert!(listed.lines().count() >= 30, "{listed}");
    let found = ok(
        &dir,
        &["search", "all", "treesheets", "tutorial", "--limit", "3"],
    );
    assert!(
        found.contains("«TreeSheets»") || found.contains("«treesheets»"),
        "{found}"
    );
    let japanese = ok(
        &dir,
        &[
            "search",
            "all",
            "グリッド",
            "doc:tutorial-ja",
            "--limit",
            "2",
        ],
    );
    assert!(japanese.starts_with("tutorial-ja.wombat "), "{japanese}");
    // A second import beside the first does not overwrite it.
    let again = ok(&dir, &["import-all", examples.to_str().unwrap(), "all"]);
    assert!(again.contains("imported"));
    assert!(dir.join("all/tutorial-2.wombat").is_file());
}

/// As for the link cache: a cache that is a symbolic link is not written through.
#[cfg(unix)]
#[test]
fn the_search_cache_is_never_written_through_a_symbolic_link() {
    use std::os::unix::fs::symlink;
    for (test, link, to_dir) in [
        ("link-top", ".wombat-index", true),
        ("link-kind", ".wombat-index/search", true),
        ("link-entry", ".wombat-index/search/shop.wombat.idx", false),
    ] {
        let dir = two(test);
        let outside = folder(&format!("{test}-outside"));
        let precious = outside.join("important.txt");
        fs::write(&precious, "do not touch").unwrap();
        let link = dir.join(link);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink(if to_dir { &outside } else { &precious }, &link).unwrap();
        for _ in 0..2 {
            let out = ok(&dir, &["search", ".", "apple", "--cache"]);
            assert!(out.contains("5 results"), "{out}");
        }
        assert_eq!(fs::read_to_string(&precious).unwrap(), "do not touch");
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 1, "{test}");
        assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
    }
}

const BUDGET: &str = "Costs | =SUM(Costs) | =SUM(Costs) * 1.2\n  Rent | 800\n  Food | 250.5\n\
                      Income | =SUM(Income) - SUM(Costs)\n  2000\n";

/// Formula cells (milestone 5) in search: found by what they show and by their formula,
/// printed with both; replace changes formulas and never results.
#[test]
fn formula_cells_are_found_by_result_and_replaced_by_formula() {
    let dir = folder("formulas");
    fs::write(dir.join("budget.txt"), BUDGET).unwrap();
    ok(&dir, &["new", "budget.wombat", "--from", "budget.txt"]);
    // A computed value, from a document that is not open, with and without the cache.
    let expected = "budget.wombat /2,1 \"=SUM(Costs)\" = \"1050.5\" (in: \"\")\n    \
                    shows: «1050.5»\n1 results in 12 cells of 1 documents\n";
    assert_eq!(ok(&dir, &["search", ".", "1050.5"]), expected);
    assert_eq!(ok(&dir, &["search", ".", "1050.5", "--cache"]), expected);
    let cached = ok(&dir, &["search", ".", "1050.5", "--cache", "--time"]);
    assert!(cached.starts_with(expected), "{cached}");
    assert!(cached.contains("1 from the cache"), "{cached}");
    // The formula is searched too, and shown with what it comes to.
    let by_formula = ok(&dir, &["search", ".", "sum", "income"]);
    assert!(
        by_formula.starts_with(
            "budget.wombat /2,2 \"=SUM(Income) - SUM(Costs)\" = \"949.5\" (in: \"\")\n    \
             text: =«SUM»(«Income») - «SUM»(Costs)\n"
        ),
        "{by_formula}"
    );
    // In an editing session the index goes by the results of the moment.
    let session = edited(
        &dir,
        "budget.wombat",
        "find \"1050.5\"\ntext /1,1/2,1 \"300\"\nfind \"1050.5\"\nfind \"550.5\"\n\
         replace-all \"1050.5\" \"7\"\nreplace-all \"sum\" \"MAX\"\nfind \"case:MAX\"\n\
         find \"1700\"\nundo\nfind \"550.5\"\nsave\n",
    );
    assert_eq!(
        session,
        "/2,1 «1050.5»\n1 found\n0 found\n/2,1 «550.5»\n1 found\n\
         0 places in 0 cells\n4 places in 3 cells\n\
         /2,1 =«MAX»(Costs)\n/3,1 =«MAX»(Costs) * 1.2\n/2,2 =«MAX»(Income) - «MAX»(Costs)\n\
         3 found\n/2,2 «1700»\n1 found\n/2,1 «550.5»\n1 found\nsaved budget.wombat\n"
    );
    // The saved document has the new number; the scan sees the new result.
    assert!(ok(&dir, &["search", ".", "1050.5", "--cache"]).starts_with("0 results"));
    assert!(ok(&dir, &["search", ".", "550.5", "--cache"]).contains("shows: «550.5»"));
    // Across the folder: a result is not replaced, a formula is, and it recalculates.
    let dry = ok(&dir, &["replace", ".", "550.5", "7", "--dry-run"]);
    assert_eq!(
        dry,
        "0 places in 0 cells of 0 documents would be replaced\n"
    );
    let done = ok(&dir, &["replace", ".", "costs", "Kosten"]);
    assert!(done.contains("4 places in 4 cells"), "{done}");
    let shown = ok(&dir, &["show", "budget.wombat"]);
    assert!(
        shown.contains("[2,1] \"=SUM(Kosten)\" (formula) = \"550.5\""),
        "{shown}"
    );
}

/// Where the milestones meet: `import-all` and `replace` (milestone 6) write documents,
/// image folders, images and view states. A link planted at a name that the program
/// chooses is never written through (decision A87, from the review of milestone 4, holds
/// for them through the storage layer); a link at a document's own name is the user's.
#[cfg(unix)]
#[test]
fn import_all_and_replace_write_through_no_planted_link() {
    use std::os::unix::fs::symlink;
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../TS/examples");
    // An example with images, alone in a folder.
    let source = folder("planted-source");
    fs::copy(
        examples.join("tutorial-de.cts"),
        source.join("tutorial.cts"),
    )
    .unwrap();
    let source = source.to_str().unwrap();
    let elsewhere = |test: &str| {
        let outside = folder(&format!("{test}-outside"));
        fs::write(outside.join("important.txt"), "do not touch").unwrap();
        outside
    };
    let untouched = |outside: &Path, test: &str| {
        let precious = fs::read_to_string(outside.join("important.txt")).unwrap();
        assert_eq!(precious, "do not touch", "{test}");
        assert_eq!(fs::read_dir(outside).unwrap().count(), 1, "{test}");
    };
    // Without a link: the folder of images and the folder of view states are made.
    let dir = folder("planted-none");
    ok(&dir, &["import-all", source, "all"]);
    let images = fs::read_dir(dir.join("all/tutorial.wombat.images")).unwrap();
    assert!(images.count() > 0);
    assert!(dir.join("all/.wombat-view").is_dir());

    // The folders that the importer would make are links to a folder elsewhere.
    for link in ["tutorial.wombat.images", ".wombat-view"] {
        let test = format!("planted-folder-{}", link.len());
        let (dir, outside) = (folder(&test), elsewhere(&test));
        fs::create_dir(dir.join("all")).unwrap();
        symlink(&outside, dir.join("all").join(link)).unwrap();
        let output = wombat(&dir, &["import-all", source, "all"]);
        assert_eq!(output.status.code(), Some(1), "{test}");
        untouched(&outside, &test);
    }
    // (An image gets a new random name, so no link can wait at it; the importer's own
    // test, in `wombat-cts`, plants one at a name it is given.)
    // A link at the name of the document: to a file, the import takes another name; to
    // nothing, it is refused and no file appears where the link points.
    let (dir, outside) = (folder("planted-doc"), elsewhere("planted-doc"));
    fs::create_dir(dir.join("all")).unwrap();
    symlink(
        outside.join("important.txt"),
        dir.join("all/tutorial.wombat"),
    )
    .unwrap();
    ok(&dir, &["import-all", source, "all"]);
    assert!(dir.join("all/tutorial-2.wombat").is_file());
    untouched(&outside, "planted-doc");
    let (dir, outside) = (folder("planted-dangling"), elsewhere("planted-dangling"));
    fs::create_dir(dir.join("all")).unwrap();
    symlink(outside.join("new.wombat"), dir.join("all/tutorial.wombat")).unwrap();
    let output = wombat(&dir, &["import-all", source, "all"]);
    assert_eq!(output.status.code(), Some(1));
    untouched(&outside, "planted-dangling");

    // `replace` saves documents at their own names, where a link is the user's and is
    // followed (decision A27): the document that a link in the folder leads to is
    // changed as the others, the link stays, and nothing else appears beside the file.
    let (dir, outside) = (two("planted-replace"), elsewhere("planted-replace"));
    let other = outside.join("important.txt");
    fs::copy(dir.join("shop.wombat"), &other).unwrap();
    fs::remove_file(dir.join("shop.wombat")).unwrap();
    symlink(&other, dir.join("shop.wombat")).unwrap();
    let out = ok(&dir, &["replace", ".", "apple", "quince"]);
    assert!(out.contains("replaced"), "{out}");
    let changed = fs::read_to_string(&other).unwrap();
    assert!(changed.contains("quince") && !changed.contains("pple"));
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
    assert!(fs::symlink_metadata(dir.join("shop.wombat"))
        .unwrap()
        .is_symlink());
    assert!(ok(&dir, &["check", "shop.wombat"]).ends_with("deep\n"));
}
