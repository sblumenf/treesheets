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
    // Six files, and one warning: listing reads only the header of each file, by which
    // broken.wombat is a second file with the ID of ok.wombat.
    assert_eq!(listed.lines().count(), 7);
    assert!(listed
        .lines()
        .last()
        .unwrap()
        .starts_with("warning: broken.wombat and ok.wombat have the same document ID"));
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

#[test]
fn copy_gives_new_ids_and_list_warns_of_files_copied_by_hand() {
    let dir = folder("copy");
    fs::write(
        dir.join("shop.txt"),
        "Shopping\n  Fruit | Price\n  Apple | 1.20\n",
    )
    .unwrap();
    assert!(wombat(&dir, &["new", "a.wombat", "--from", "shop.txt"])
        .status
        .success());
    let copied = wombat(&dir, &["copy", "a.wombat", "b.wombat"]);
    assert!(copied.status.success(), "{}", stderr(&copied));
    assert!(
        stdout(&copied).contains("5 cells, all with new IDs"),
        "{}",
        stdout(&copied)
    );
    // The same content under different IDs, every one of them.
    let shown = |name: &str, ids: bool| {
        let args: &[&str] = if ids {
            &["show", name, "--ids"]
        } else {
            &["show", name]
        };
        stdout(&wombat(&dir, args))
    };
    let body = |text: String| text.split_once('\n').unwrap().1.to_string();
    assert_eq!(
        body(shown("a.wombat", false)),
        body(shown("b.wombat", false))
    );
    let ids = |name: &str| -> Vec<String> {
        let text = fs::read_to_string(dir.join(name)).unwrap();
        text.split("\"id\": \"")
            .skip(1)
            .map(|rest| rest[..36].to_string())
            .collect()
    };
    let (a, b) = (ids("a.wombat"), ids("b.wombat"));
    assert_eq!((a.len(), b.len()), (6, 6));
    assert!(a.iter().all(|id| !b.contains(id)));
    assert!(wombat(&dir, &["check", "b.wombat"]).status.success());
    let clean = stdout(&wombat(&dir, &["list", "--cells"]));
    assert_eq!(clean.lines().count(), 2, "{clean}");

    // An existing file is never replaced, and the new name must be a document's.
    let again = wombat(&dir, &["copy", "a.wombat", "b.wombat"]);
    assert_eq!(again.status.code(), Some(1));
    assert!(stderr(&again).contains("already exists"));
    assert_eq!(
        wombat(&dir, &["copy", "a.wombat", "c.txt"]).status.code(),
        Some(2)
    );
    assert_eq!(
        wombat(&dir, &["copy", "absent.wombat", "c.wombat"])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(wombat(&dir, &["copy", "a.wombat"]).status.code(), Some(2));
    assert!(!dir.join("c.wombat").exists());

    // A copy made by copying the file is pointed out.
    fs::copy(dir.join("a.wombat"), dir.join("c.wombat")).unwrap();
    let listed = stdout(&wombat(&dir, &["list"]));
    assert_eq!(listed.lines().count(), 4, "{listed}");
    assert!(listed.contains(
        "warning: a.wombat and c.wombat have the same document ID: one is a copy of the file; \
         make copies with \"wombat copy\""
    ));
    let listed = stdout(&wombat(&dir, &["list", ".", "--cells"]));
    assert_eq!(listed.lines().count(), 5, "{listed}");
    assert!(
        listed.contains("warning: a.wombat and c.wombat share 6 IDs, the document ID among them"),
        "{listed}"
    );
}

fn edit(dir: &Path, file: &str, script: &str) -> Output {
    fs::write(dir.join("script.txt"), script).unwrap();
    wombat(dir, &["edit", file, "--script", "script.txt"])
}

#[test]
fn edit_runs_a_script_and_undoing_everything_gives_the_original_file() {
    let dir = folder("edit");
    fs::write(dir.join("outline.txt"), OUTLINE).unwrap();
    wombat(&dir, &["new", "shop.wombat", "--from", "outline.txt"]);
    let original = fs::read(dir.join("shop.wombat")).unwrap();

    // A series of edits of every sort, saved.
    let script = "\
# cells by their place, as `show` prints it
now 1700000000000
text /1,2 \"Apple pie \\u{1F600}\"
insert /1,2 0 \"An \"
delete /1,2 0 3
replace /1,2 0 5 \"Pear\"
style /1,2 0 4 +bold +italic
color /1,2 5 3 #ff0000
link /1,2 0 4 url \"https://example.org/pear\"
link /2,2 0 1 cell /1,2
set /1,1 /2,1 color=#ffeecc align=center note=\"a note\"
set / border=#0000ff spacing=5
cell-style /1,1 /2,1 +underline
width / 2 120
grid /2,3 2 2
text /2,3/1,1 \"inner\"
insert-rows / 2 1
insert-columns / 3 2
move-rows / 1 1 4
swap /1,1 /2,1
move /2,4 /4,4
copy /1,2 /3,1
delete-rows / 2 1
clear /4,1
delete-cells / 1 2 2 1
tag \"Pear\" #00ff00 #000000
fold /3,1
changes
show
check
save
";
    let done = edit(&dir, "shop.wombat", script);
    assert!(done.status.success(), "{}", stderr(&done));
    let out = stdout(&done);
    assert!(out.contains("tag \"Pear\"\n"), "{out}");
    assert!(
        out.contains("valid: ") && out.ends_with("saved shop.wombat\n"),
        "{out}"
    );
    let edited = fs::read(dir.join("shop.wombat")).unwrap();
    assert_ne!(edited, original);
    let check = wombat(&dir, &["check", "shop.wombat"]);
    assert!(check.status.success(), "{}", stderr(&check));
    let shown = stdout(&wombat(&dir, &["show", "shop.wombat"]));
    assert!(
        shown.contains("(folded)") && shown.contains("\"red\""),
        "{shown}"
    );

    // The same edits followed by `undo all` leave the file byte for byte as it was, and
    // `redo all` after that gives the edited file again, same IDs included.
    fs::write(dir.join("shop.wombat"), &original).unwrap();
    let _ = fs::remove_dir_all(dir.join(".wombat-view"));
    let body = script.replace("changes\nshow\ncheck\nsave\n", "");
    let undone = edit(&dir, "shop.wombat", &format!("{body}undo all\nsave\n"));
    assert!(undone.status.success(), "{}", stderr(&undone));
    assert_eq!(fs::read(dir.join("shop.wombat")).unwrap(), original);
    let redone = edit(
        &dir,
        "shop.wombat",
        &format!("{body}undo all\nredo all\nundo 3\nredo 3\nsave\n"),
    );
    assert!(redone.status.success(), "{}", stderr(&redone));
    let text = fs::read_to_string(dir.join("shop.wombat")).unwrap();
    assert!(
        text.contains("\"Pear") && text.contains("\"tags\""),
        "{text}"
    );

    // A group undoes as one; the report of an undo names what came back.
    fs::write(dir.join("shop.wombat"), &original).unwrap();
    let grouped = edit(
        &dir,
        "shop.wombat",
        "begin\ntext /1,1 \"a\"\ntext /2,1 \"b\"\nend\nundo\nchanges\nundo all\nsave\n",
    );
    let out = stdout(&grouped);
    assert_eq!(out.matches("changed /").count(), 2, "{out}");
    assert_eq!(fs::read(dir.join("shop.wombat")).unwrap(), original);
}

#[test]
fn edit_stops_at_a_bad_line_and_writes_nothing() {
    let dir = folder("edit-bad");
    fs::write(dir.join("outline.txt"), OUTLINE).unwrap();
    wombat(&dir, &["new", "shop.wombat", "--from", "outline.txt"]);
    let original = fs::read(dir.join("shop.wombat")).unwrap();
    let cases = [
        (
            "text /1,1 \"x\"\nfrobnicate /1,1\nsave\n",
            "line 2: \"frobnicate\" is not a command",
        ),
        (
            "text /9,9 \"x\"\nsave\n",
            "line 1: \"/9,9\": there is no cell 9,9 there",
        ),
        ("text /0,1 \"x\"\n", "is not column,row from 1"),
        ("text /1,1 \"unclosed\n", "a quote is not closed"),
        ("insert /1,1 99 \"x\"\n", "outside the text"),
        ("insert-rows /1,1 1 1\n", "has no grid"),
        ("insert-rows / 0 1\n", "count from 1"),
        ("set /1,1 colour=#fff\n", "is not a property"),
        ("set /1,1 color=red\n", "is not a colour"),
        ("style /1,1 0 1 bold\n", "write +bold or -bold"),
        ("undo\n", "nothing to undo"),
        (
            "text 00000000-0000-0000-0000-000000000001 \"x\"\n",
            "there is no cell",
        ),
        ("text /1,1 \"x\" extra\n", "unexpected"),
        // A character made of two code points (e + combining accent) is not cut.
        (
            "text /1,1 \"e\\u{301}x\"\ndelete /1,1 0 1\n",
            "would cut a character",
        ),
    ];
    for (script, message) in cases {
        let failed = edit(&dir, "shop.wombat", script);
        assert_eq!(failed.status.code(), Some(1), "{script}");
        assert!(
            stderr(&failed).contains(message),
            "{script}: {}",
            stderr(&failed)
        );
        assert_eq!(
            fs::read(dir.join("shop.wombat")).unwrap(),
            original,
            "{script}"
        );
    }
    // Without `save` nothing is written, and the tool says so.
    let unsaved = edit(&dir, "shop.wombat", "text /1,1 \"x\"\n");
    assert!(unsaved.status.success());
    assert!(stdout(&unsaved).contains("not saved"));
    assert_eq!(fs::read(dir.join("shop.wombat")).unwrap(), original);
    // Cells can be named by ID, and the script can come from standard input.
    let shown = stdout(&wombat(&dir, &["show", "shop.wombat", "--ids"]));
    let id = shown
        .lines()
        .find(|l| l.contains("\"Fruit\""))
        .unwrap()
        .rsplit('#')
        .next()
        .unwrap()
        .to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_wombat"))
        .current_dir(&dir)
        .args(["edit", "shop.wombat", "--script", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let script = format!("text #{id} \"Fruits\"\ntext {id} \"Fruit!\"\nchanges\nsave\n");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        stdout(&output),
        format!("changed /1,1 #{id}\nsaved shop.wombat\n")
    );
    assert!(fs::read_to_string(dir.join("shop.wombat"))
        .unwrap()
        .contains("\"Fruit!\""));
    let help = wombat(&dir, &["edit", "--help"]);
    assert!(stdout(&help).contains("delete-cells CELL COLUMN ROW COLUMNS ROWS"));
}

#[test]
fn edit_restructures_and_undoing_everything_gives_the_original_file() {
    let dir = folder("edit-restructure");
    let outline = "Shop\n  Fruit | Apple | 10\n  Veg | Leek | 9\n  Fruit | Pear | 100\n  \
                   Fruit | apple | 9\n";
    fs::write(dir.join("outline.txt"), outline).unwrap();
    wombat(&dir, &["new", "shop.wombat", "--from", "outline.txt"]);
    let original = fs::read(dir.join("shop.wombat")).unwrap();
    // Every restructuring command once; the help names each of them.
    let help = stdout(&wombat(&dir, &["edit", "--help"]));
    let script = "\
sort / 3 1 4
show
sort / 2 1 4 desc
transpose /
transpose / /1,1
hierarchify /
show
flatten /
hierarchify /
hierarchy-swap /1,1/1,1
show
wrap / 1 1 1 2
collapse /1,1 1 1 1 2
split /1,1/1,1 5
tag-cells /1,2 /1,1/1,1
untag-cells /1,2
size-by -2 deep /1,1
size-by 1 /1,1
width-by / 1 1 15 deep
toggle-check /1,2 /1,2/1,1
check
show
undo all
save
";
    for line in script.lines() {
        let verb = line.split(' ').next().unwrap();
        assert!(
            help.contains(&format!("\n  {verb} ")),
            "{verb} is not in the help"
        );
    }
    let done = edit(&dir, "shop.wombat", script);
    assert!(done.status.success(), "{}", stderr(&done));
    let out = stdout(&done);
    // Numbers sort as text, without regard to case in the second key.
    assert!(
        out.contains("[1,1] \"Fruit\"\n  [2,1] \"Apple\"\n  [3,1] \"10\"\n  [1,2] \"Fruit\"\n  [2,2] \"Pear\"\n  [3,2] \"100\""),
        "{out}"
    );
    assert!(out.contains("[1,1] \"Fruit\"\n    grid 1 x 3"), "{out}");
    assert!(out.contains("\u{2610} "), "{out}");
    assert_eq!(fs::read(dir.join("shop.wombat")).unwrap(), original);
    for (bad, message) in [
        ("sort / 9 1 2\n", "cannot sort"),
        ("sort / 1 1 2 up\n", "unexpected"),
        ("hierarchify /1,1\n", "has no grid"),
        ("flatten /1,1\n", "has no grid"),
        ("hierarchy-swap /1,1\n", "cannot move up"),
        ("wrap / 1 1 9 1\n", "no cells 9 by 1"),
        ("collapse / 1 1 1 1\n", "more than one cell"),
        ("split /1,1 99\n", "outside the text"),
        ("size-by big /1,1\n", "is not a whole number"),
        ("width-by / 9 1 5\n", "no 1 columns from 8"),
        ("transpose\n", "missing: a cell"),
    ] {
        let failed = edit(&dir, "shop.wombat", &format!("{bad}save\n"));
        assert!(!failed.status.success(), "{bad}");
        assert!(
            stderr(&failed).contains(message),
            "{bad}: {}",
            stderr(&failed)
        );
    }
    assert_eq!(fs::read(dir.join("shop.wombat")).unwrap(), original);
}
