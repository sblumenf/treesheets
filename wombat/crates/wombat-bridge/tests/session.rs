//! An open document as the interface uses it, without the interface: the sequence the
//! bare app goes through (open the example, read the root grid, set the text of one
//! cell, undo), and creating, copying, saving and failing.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};

use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{CellKind, Command, ErrorKind, GridInfo, Report};
use wombat_core::SeededIds;

/// The document that comes with the app.
fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/assets/example.wombat")
}

/// An empty folder for one test.
fn folder(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("session")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn text(path: &Path) -> String {
    path.display().to_string()
}

fn ids() -> SeededIds {
    SeededIds::from_u64(11)
}

/// A copy of the example to work on, as the app makes one.
fn open_example(name: &str) -> (OpenDocument, PathBuf) {
    let path = folder(name).join("example.wombat");
    let doc = OpenDocument::copy_with(&example(), &path, ids()).unwrap();
    (doc, path)
}

fn texts(grid: &GridInfo) -> Vec<&str> {
    grid.cells.iter().map(|c| c.text.as_str()).collect()
}

#[test]
fn the_sequence_of_the_bare_app() {
    let (mut doc, path) = open_example("sequence");
    assert!(doc.is_saved());
    assert_eq!(doc.path(), text(&path));

    // The root grid: two columns, three rows; nested grids and a formula with its result.
    let root = doc.root();
    let before = doc.grid(root.clone()).unwrap();
    assert_eq!((before.xs, before.ys), (2, 3));
    assert_eq!(before.cell.text, "Example");
    let expected = [
        "Shopping",
        "what to buy",
        "Costs",
        "what it costs",
        "Total",
        "=SUM(Costs)",
    ];
    assert_eq!(texts(&before), expected);
    assert!(before.cells[0].has_grid && before.cells[2].has_grid);
    assert!(!before.cells[1].has_grid);
    let total = &before.cells[5];
    assert_eq!(total.kind, CellKind::Formula);
    assert_eq!((total.result.as_deref(), total.failed), (Some("10"), false));
    assert_eq!(before.cells[4].result, None);

    // One level down.
    let costs = doc.grid(before.cells[2].id.clone()).unwrap();
    assert_eq!((costs.xs, costs.ys), (1, 3));
    assert_eq!(texts(&costs), ["3", "2", "5"]);

    // Set the text of one cell: the report names it, and the formula that reads it.
    let cell = costs.cells[0].id.clone();
    let set = Command::SetText {
        cell: cell.clone(),
        text: "30".into(),
    };
    let report = doc.apply(set).unwrap();
    let expected = Report {
        changed: vec![cell.clone()],
        recalculated: vec![total.id.clone()],
        ..Report::default()
    };
    assert_eq!(report, expected);
    assert!(!doc.is_saved());
    let after = doc.grid(root.clone()).unwrap();
    assert_eq!(after.cells[5].result.as_deref(), Some("37"));
    let costs_after = doc.grid(before.cells[2].id.clone()).unwrap();
    assert_eq!(texts(&costs_after), ["30", "2", "5"]);

    // Undo: the same cells are reported, and everything reads as at first.
    assert_eq!(doc.undo().unwrap(), Some(expected.clone()));
    assert_eq!(doc.grid(root.clone()).unwrap(), before);
    assert_eq!(doc.grid(before.cells[2].id.clone()).unwrap(), costs);
    assert_eq!(doc.undo().unwrap(), None);

    // Redo, save, and open again: the change is in the file.
    assert_eq!(doc.redo().unwrap(), Some(expected));
    assert_eq!(doc.redo().unwrap(), None);
    doc.save().unwrap();
    assert!(doc.is_saved());
    let again = OpenDocument::open_with(&path, ids()).unwrap();
    assert_eq!(again.id(), doc.id());
    assert_eq!(again.grid(root).unwrap(), after);
}

#[test]
fn a_copy_has_ids_of_its_own() {
    let original = OpenDocument::open_with(&example(), ids()).unwrap();
    let (copy, _) = open_example("copy");
    assert_ne!(copy.id(), original.id());
    assert_ne!(copy.root(), original.root());
    let (a, b) = (
        original.grid(original.root()).unwrap(),
        copy.grid(copy.root()).unwrap(),
    );
    assert_eq!(texts(&a), texts(&b));
    assert!(a.cells.iter().zip(&b.cells).all(|(a, b)| a.id != b.id));
    // A second copy does not go over the first.
    let path = copy.path();
    let again = OpenDocument::copy_with(&example(), Path::new(&path), ids());
    assert_eq!(again.err().unwrap().kind, ErrorKind::AlreadyExists);
}

#[test]
fn a_new_document_is_saved_at_once_and_has_a_root_without_a_grid() {
    let path = folder("create").join("new.wombat");
    let mut doc = OpenDocument::create_with(&path, ids()).unwrap();
    assert!(path.exists());
    assert!(doc.is_saved());
    let root = doc.root();
    let view = doc.grid(root.clone()).unwrap();
    assert_eq!((view.xs, view.ys, view.cells.len()), (0, 0, 0));
    assert!(!view.cell.has_grid);

    let grid = Command::CreateGrid {
        cell: root.clone(),
        xs: 2,
        ys: 1,
    };
    let report = doc.apply(grid).unwrap();
    assert_eq!(report.grids, std::slice::from_ref(&root));
    assert_eq!(report.created.len(), 2);
    let view = doc.grid(root.clone()).unwrap();
    let mut created: Vec<_> = view.cells.iter().map(|c| c.id.clone()).collect();
    created.sort();
    assert_eq!(created, report.created);
    doc.save().unwrap();
    let again = OpenDocument::open_with(&path, ids()).unwrap();
    assert_eq!(again.grid(root).unwrap(), view);

    // Not over a file that is there.
    let error = OpenDocument::create_with(&path, ids()).err().unwrap();
    assert_eq!(error.kind, ErrorKind::AlreadyExists);
    assert_eq!(error.path, Some(text(&path)));
}

#[test]
fn edits_stamp_the_time_of_the_clock_given() {
    let (mut doc, path) = open_example("clock");
    let root = doc.grid(doc.root()).unwrap();
    let cell = root.cells[1].id.clone();
    doc.set_fixed_time(Some(1234));
    let set = |text: &str| Command::SetText {
        cell: cell.clone(),
        text: text.into(),
    };
    doc.apply(set("a")).unwrap();
    doc.save().unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("1234"), "{saved}");
    // The device's clock, when no time is fixed: some time after 2020.
    let (mut doc, path) = open_example("clock-system");
    doc.apply(set_text(&doc, "b")).unwrap();
    doc.save().unwrap();
    let loaded = wombat_core::storage::load_document(&path).unwrap();
    let stamped = loaded
        .cells_in_order()
        .into_iter()
        .filter_map(|id| loaded.cell(id).unwrap().last_edit)
        .max();
    assert!(stamped.unwrap() > 1_577_836_800_000);
}

fn set_text(doc: &OpenDocument, text: &str) -> Command {
    let root = doc.grid(doc.root()).unwrap();
    Command::SetText {
        cell: root.cells[1].id.clone(),
        text: text.into(),
    }
}

#[test]
fn errors_are_values_with_a_kind_and_leave_the_document_as_it_was() {
    let missing = folder("errors").join("none.wombat");
    let error = OpenDocument::open(text(&missing)).err().unwrap();
    assert_eq!(error.kind, ErrorKind::Io);
    assert_eq!(error.path, Some(text(&missing)));
    assert!(!error.message.is_empty());

    let garbage = folder("errors-garbage").join("bad.wombat");
    std::fs::write(&garbage, "not json").unwrap();
    let error = OpenDocument::open(text(&garbage)).err().unwrap();
    assert_eq!(error.kind, ErrorKind::Syntax);

    let (mut doc, _) = open_example("errors-doc");
    let root = doc.root();
    let before = doc.grid(root.clone()).unwrap();
    // A cell that is not there.
    let nowhere = "00000000-0000-4000-8000-000000000000".to_string();
    assert_eq!(
        doc.grid(nowhere.clone()).err().unwrap().kind,
        ErrorKind::NotFound
    );
    let set = Command::SetText {
        cell: nowhere,
        text: "x".into(),
    };
    assert_eq!(doc.apply(set).err().unwrap().kind, ErrorKind::NotFound);
    // Text that is no ID.
    assert_eq!(
        doc.grid("root".into()).err().unwrap().kind,
        ErrorKind::Invalid
    );
    // A command the document does not allow.
    let remove = Command::RemoveGrid { cell: root.clone() };
    assert_eq!(doc.apply(remove).err().unwrap().kind, ErrorKind::Refused);
    let rows = Command::InsertRows {
        cell: root.clone(),
        at: 99,
        count: 1,
    };
    assert_eq!(doc.apply(rows).err().unwrap().kind, ErrorKind::Refused);
    assert_eq!(doc.grid(root).unwrap(), before);
    assert!(doc.is_saved());
    assert_eq!(doc.undo().unwrap(), None);
}

#[test]
fn the_public_entry_points_seed_ids_from_the_system() {
    let dir = folder("public");
    let a = OpenDocument::create(text(&dir.join("a.wombat"))).unwrap();
    let b = OpenDocument::create(text(&dir.join("b.wombat"))).unwrap();
    assert_ne!(a.id(), b.id());
    let copy = OpenDocument::open_copy(text(&example()), text(&dir.join("c.wombat"))).unwrap();
    let again = OpenDocument::open(copy.path()).unwrap();
    assert_eq!(copy.id(), again.id());
}

/// Commands that were recorded as JSON play back to the same document.
#[test]
fn recorded_commands_play_back() {
    use wombat_bridge::convert::command_out;
    use wombat_bridge::record::{command_from_json, command_to_json};
    let (mut doc, _) = open_example("record");
    let root = doc.grid(doc.root()).unwrap();
    let cell = root.cells[1].id.clone();
    let commands = vec![
        Command::SetText {
            cell: cell.clone(),
            text: "first".into(),
        },
        Command::InsertText {
            cell: cell.clone(),
            at: 5,
            text: " and more".into(),
        },
        Command::CreateGrid { cell, xs: 1, ys: 2 },
    ];
    let mut recorded = Vec::new();
    for command in commands {
        let core = wombat_bridge::convert::command_in(command.clone()).unwrap();
        recorded.push(command_to_json(&core).unwrap());
        doc.apply(command).unwrap();
    }
    let after = doc.grid(doc.root()).unwrap();
    while doc.undo().unwrap().is_some() {}
    for line in &recorded {
        let core = command_from_json(line).unwrap();
        doc.apply(command_out(&core).unwrap()).unwrap();
    }
    assert_eq!(doc.grid(doc.root()).unwrap(), after);
}
