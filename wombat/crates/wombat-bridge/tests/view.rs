//! The view state of an open document: which level the screen shows, what each level
//! remembers and which grids are folded, on this device. It is read when the document is
//! opened, follows every change, and is written to its own file when the document is
//! closed or let go of; that file never keeps a document from opening, closing or being
//! saved.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::path::{Path, PathBuf};

use common::{example, ids, set_text, text_in_file};
use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{Command, ErrorKind, LevelInfo, StageAsk};
use wombat_core::storage;
use wombat_core::{CellId, DocId, Editor, ViewState};

fn open_example(name: &str) -> (OpenDocument, PathBuf) {
    common::open_example("view", name)
}

fn open(path: &Path) -> OpenDocument {
    OpenDocument::open_with(path, ids()).unwrap()
}

fn ask() -> StageAsk {
    StageAsk {
        x0: 0,
        y0: 0,
        cols: 100,
        rows: 100,
        full_depth: 1,
        full_cells: 24,
        preview_cols: 3,
        preview_rows: 4,
        shape_cols: 12,
        shape_rows: 20,
        text_cut: 2000,
        preview_cut: 40,
    }
}

/// The cells of the example by what they are: the root, "Shopping" (with a grid), "what
/// to buy", "Costs" (with a grid) and "Total".
struct Cells {
    root: String,
    shopping: String,
    note: String,
    costs: String,
    total: String,
}

fn cells(doc: &OpenDocument) -> Cells {
    let root = doc.root().unwrap();
    let grid = doc.grid(root.clone()).unwrap();
    let id = |n: usize| grid.cells[n].id.clone();
    Cells {
        root,
        shopping: id(0),
        note: id(1),
        costs: id(2),
        total: id(4),
    }
}

fn level(cell: &str, x: i64, y: i64, selected: Option<&str>) -> LevelInfo {
    LevelInfo {
        cell: cell.into(),
        x,
        y,
        selected: selected.map(str::to_string),
    }
}

/// What the level `cell` remembers, as a read of it as the stage says.
fn remembered(doc: &OpenDocument, cell: &str) -> Option<LevelInfo> {
    doc.stage(cell.into(), ask()).unwrap().level
}

fn folded(doc: &OpenDocument, cell: &str) -> bool {
    doc.cell(cell.into()).unwrap().folded
}

/// The file of the view state of the document in the file at `path`.
fn view_file(path: &Path) -> PathBuf {
    let id = storage::document_id(path).unwrap();
    storage::view_state_path(path.parent().unwrap(), id)
}

/// The view state in its file, as the core reads it.
fn in_file(path: &Path) -> Option<ViewState> {
    let id: DocId = storage::document_id(path).unwrap();
    storage::load_view_state(path.parent().unwrap(), id).unwrap()
}

/// Which file is under the name: every write puts a new one there.
#[cfg(unix)]
fn file_number(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).unwrap().ino()
}

/// What another program does to a file: a new file under the name, with other content.
fn change_by_someone_else(path: &Path) {
    let other = path.with_extension("other");
    let mut doc = OpenDocument::copy_with(&example(), &other, ids()).unwrap();
    doc.apply(set_text(&doc, "theirs")).unwrap();
    doc.save().unwrap();
    drop(doc);
    std::fs::rename(&other, path).unwrap();
}

#[test]
fn the_levels_survive_close_and_reopen() {
    let (mut doc, path) = open_example("levels");
    let c = cells(&doc);
    // A document that was never looked at starts at the root, with nothing remembered.
    assert_eq!(doc.view_stage().unwrap(), c.root);
    assert_eq!(remembered(&doc, &c.root), None);
    assert!(doc.is_view_saved().unwrap());

    // Down into "Shopping", leaving the root scrolled and with a cell selected.
    let left = level(&c.root, 1, 2, Some(&c.costs));
    doc.view_go(c.shopping.clone(), Some(left.clone())).unwrap();
    assert_eq!(doc.view_stage().unwrap(), c.shopping);
    assert_eq!(remembered(&doc, &c.root), Some(left.clone()));
    assert_eq!(remembered(&doc, &c.shopping), None);
    assert!(!doc.is_view_saved().unwrap());
    // It is no change to the document.
    assert!(doc.is_saved().unwrap());

    // The app is put away: the level that is shown is remembered too.
    let here = level(&c.shopping, 0, 3, None);
    doc.view_remember(here.clone()).unwrap();
    doc.close().unwrap();
    assert!(view_file(&path).is_file());

    let doc = open(&path);
    assert_eq!(doc.view_stage().unwrap(), c.shopping);
    assert_eq!(remembered(&doc, &c.root), Some(left));
    assert_eq!(remembered(&doc, &c.shopping), Some(here));
    assert_eq!(remembered(&doc, &c.costs), None);
    assert!(doc.is_view_saved().unwrap());
}

#[test]
fn a_fold_survives_close_and_reopen() {
    let (mut doc, path) = open_example("fold");
    let c = cells(&doc);
    assert!(!folded(&doc, &c.shopping));
    // It says whether anything changed.
    assert!(doc.set_folded(c.shopping.clone(), true).unwrap());
    assert!(!doc.set_folded(c.shopping.clone(), true).unwrap());
    assert!(!doc.set_folded(c.costs.clone(), false).unwrap());
    assert!(doc.is_saved().unwrap());
    let stage = doc.stage(c.root.clone(), ask()).unwrap();
    let folds: Vec<_> = stage.cells.iter().take(6).map(|l| l.folded).collect();
    assert_eq!(folds, [true, false, false, false, false, false]);
    doc.close().unwrap();

    let mut doc = open(&path);
    assert!(folded(&doc, &c.shopping) && !folded(&doc, &c.costs));
    assert!(doc.set_folded(c.shopping.clone(), false).unwrap());
    assert!(!folded(&doc, &c.shopping));
    doc.close().unwrap();
    assert!(!folded(&open(&path), &c.shopping));
}

/// Folding an unfolded cell again, or going to the level that is shown, is no change:
/// the file is not written for it.
#[cfg(unix)]
#[test]
fn nothing_is_written_when_nothing_changed() {
    // A document that was only read gets no view-state file, and no folder for one.
    let (mut doc, path) = open_example("unchanged");
    let c = cells(&doc);
    doc.stage(c.root.clone(), ask()).unwrap();
    doc.view_go(c.root.clone(), None).unwrap();
    assert!(!doc.set_folded(c.costs.clone(), false).unwrap());
    assert!(doc.is_view_saved().unwrap());
    doc.save_view().unwrap();
    doc.close().unwrap();
    assert!(!path.parent().unwrap().join(storage::VIEW_DIR).exists());

    // One that has a file leaves it as it is.
    let mut doc = open(&path);
    doc.view_go(c.shopping.clone(), None).unwrap();
    doc.close().unwrap();
    let written = file_number(&view_file(&path));
    let mut doc = open(&path);
    doc.view_go(c.shopping.clone(), None).unwrap();
    assert!(doc.is_view_saved().unwrap());
    doc.save_view().unwrap();
    doc.close().unwrap();
    drop(open(&path));
    assert_eq!(file_number(&view_file(&path)), written);

    // A change to the document that the view state does not notice leaves it too.
    let mut doc = open(&path);
    doc.apply(set_text(&doc, "ours")).unwrap();
    assert!(doc.is_view_saved().unwrap());
    doc.close().unwrap();
    assert_eq!(file_number(&view_file(&path)), written);
}

/// What a level remembers is something to write also when nothing else changed: the app
/// is put away on the level it was opened on, or a level is left for the one the view
/// state has as shown already.
#[test]
fn what_is_remembered_is_written_when_nothing_else_changed() {
    let (mut doc, path) = open_example("remember-alone");
    let c = cells(&doc);
    doc.view_remember(level(&c.root, 1, 2, None)).unwrap();
    assert!(!doc.is_view_saved().unwrap());
    doc.close().unwrap();
    let mut doc = open(&path);
    assert_eq!(remembered(&doc, &c.root), Some(level(&c.root, 1, 2, None)));
    assert!(doc.is_view_saved().unwrap());
    let left = level(&c.costs, 0, 1, None);
    doc.view_go(c.root.clone(), Some(left.clone())).unwrap();
    assert!(!doc.is_view_saved().unwrap());
    doc.close().unwrap();
    assert_eq!(remembered(&open(&path), &c.costs), Some(left));
}

#[test]
fn saving_the_view_state_writes_it_at_once() {
    let (mut doc, path) = open_example("save-view");
    let c = cells(&doc);
    doc.view_go(c.costs.clone(), None).unwrap();
    doc.save_view().unwrap();
    assert!(doc.is_view_saved().unwrap());
    let id = CellId::parse(&c.costs).unwrap();
    assert_eq!(in_file(&path).unwrap().zoom, Some(id));
    // It does not depend on the document's file: another program changed that one, the
    // document cannot be saved, and where the screen is can be written all the same.
    doc.apply(set_text(&doc, "ours")).unwrap();
    change_by_someone_else(&path);
    assert_eq!(doc.save().unwrap_err().kind, ErrorKind::ChangedOnDisk);
    doc.view_go(c.root.clone(), None).unwrap();
    doc.save_view().unwrap();
    assert_eq!(in_file(&path).unwrap().zoom, None);
}

/// A view-state file that cannot be read is no reason not to open the document: the
/// screen starts at the root. The file is left as it is until there is something to
/// write, and then replaced.
#[test]
fn a_damaged_file_is_ignored_and_then_replaced() {
    let (doc, path) = open_example("damaged");
    let c = cells(&doc);
    drop(doc);
    let file = view_file(&path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "{ this is not a view state").unwrap();

    let mut doc = open(&path);
    assert_eq!(doc.view_stage().unwrap(), c.root);
    assert!(!folded(&doc, &c.shopping));
    doc.close().unwrap();
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "{ this is not a view state"
    );

    let mut doc = open(&path);
    doc.view_go(c.shopping.clone(), None).unwrap();
    doc.close().unwrap();
    assert_eq!(in_file(&path).unwrap().zoom, CellId::parse(&c.shopping));
    assert_eq!(open(&path).view_stage().unwrap(), c.shopping);
}

/// The file may be older than the document: another program removed cells since. What
/// it says of cells that are gone is forgotten, the rest is kept.
#[test]
fn a_view_state_that_names_removed_cells_is_brought_in_line() {
    let (mut doc, path) = open_example("stale");
    let c = cells(&doc);
    doc.set_folded(c.shopping.clone(), true).unwrap();
    doc.set_folded(c.costs.clone(), true).unwrap();
    doc.view_remember(level(&c.costs, 0, 2, None)).unwrap();
    doc.view_go(
        c.shopping.clone(),
        Some(level(&c.root, 1, 2, Some(&c.note))),
    )
    .unwrap();
    doc.close().unwrap();

    // Another program removes the first row, "Shopping" and "what to buy".
    let mut editor = Editor::new(storage::load_document(&path).unwrap()).unwrap();
    let rows = wombat_core::Command::DeleteRows {
        cell: editor.document().root(),
        at: 0,
        count: 1,
    };
    editor.apply(rows, None, &mut ids()).unwrap();
    storage::save_document(editor.document(), &path).unwrap();

    let doc = open(&path);
    assert_eq!(doc.view_stage().unwrap(), c.root);
    // The root's level is kept, in the grid as it is now, without the cell that is gone.
    assert_eq!(remembered(&doc, &c.root), Some(level(&c.root, 1, 1, None)));
    assert_eq!(
        remembered(&doc, &c.costs),
        Some(level(&c.costs, 0, 2, None))
    );
    assert!(folded(&doc, &c.costs));
    assert_eq!(doc.cell(c.shopping).unwrap_err().kind, ErrorKind::NotFound);
}

/// The view state follows what is done to the document: what refers to a removed cell
/// goes, and does not come back with the cell (decision A38).
#[test]
fn every_change_to_the_document_keeps_the_view_state_in_step() {
    let (mut doc, _) = open_example("in-step");
    let c = cells(&doc);
    doc.set_folded(c.shopping.clone(), true).unwrap();
    doc.view_go(c.shopping.clone(), None).unwrap();
    doc.save_view().unwrap();
    let rows = Command::DeleteRows {
        cell: c.root.clone(),
        at: 0,
        count: 1,
    };
    doc.apply(rows.clone()).unwrap();
    assert_eq!(doc.view_stage().unwrap(), c.root);
    assert!(!doc.is_view_saved().unwrap());
    doc.undo().unwrap().unwrap();
    assert_eq!(doc.view_stage().unwrap(), c.root);
    assert!(!folded(&doc, &c.shopping));

    // A cancelled group gives back the document, not the view state.
    doc.set_folded(c.shopping.clone(), true).unwrap();
    doc.view_go(c.shopping.clone(), None).unwrap();
    doc.begin_group().unwrap();
    doc.apply(rows.clone()).unwrap();
    doc.cancel_group().unwrap();
    assert_eq!(doc.view_stage().unwrap(), c.root);
    assert!(!folded(&doc, &c.shopping));

    // And so with a group that is ended, and a cell that only loses its grid.
    doc.set_folded(c.costs.clone(), true).unwrap();
    doc.view_remember(level(&c.costs, 0, 1, None)).unwrap();
    doc.save_view().unwrap();
    doc.begin_group().unwrap();
    doc.apply(Command::RemoveGrid {
        cell: c.costs.clone(),
    })
    .unwrap();
    assert!(!doc.is_view_saved().unwrap());
    doc.end_group().unwrap();
    doc.undo().unwrap().unwrap();
    assert!(!folded(&doc, &c.costs));
    assert_eq!(remembered(&doc, &c.costs), None);
}

#[test]
fn what_the_view_calls_refuse_changes_nothing() {
    let (mut doc, _) = open_example("refused");
    let c = cells(&doc);
    let nobody = "00000000-0000-0000-0000-000000000001".to_string();
    let kind = |result: Result<(), wombat_bridge::api::wire::BridgeError>| result.unwrap_err().kind;
    let left = || Some(level(&c.root, 1, 1, None));

    // Where to go: a cell that has a grid, or the root.
    assert_eq!(
        kind(doc.view_go(c.total.clone(), left())),
        ErrorKind::Refused
    );
    assert_eq!(
        kind(doc.view_go(nobody.clone(), left())),
        ErrorKind::NotFound
    );
    assert_eq!(
        kind(doc.view_go("no ID".into(), left())),
        ErrorKind::BadRequest
    );
    // What was left: a cell that has a grid, a position the core can take, a selected
    // cell that exists.
    let go = |doc: &mut OpenDocument, left: LevelInfo| doc.view_go(c.shopping.clone(), Some(left));
    assert_eq!(
        kind(go(&mut doc, level(&c.total, 0, 0, None))),
        ErrorKind::Refused
    );
    assert_eq!(
        kind(go(&mut doc, level(&nobody, 0, 0, None))),
        ErrorKind::NotFound
    );
    assert_eq!(
        kind(go(&mut doc, level("no ID", 0, 0, None))),
        ErrorKind::BadRequest
    );
    assert_eq!(
        kind(go(&mut doc, level(&c.root, -1, 0, None))),
        ErrorKind::BadRequest
    );
    assert_eq!(
        kind(go(&mut doc, level(&c.root, 0, -1, None))),
        ErrorKind::BadRequest
    );
    let wide = i64::from(u32::MAX) + 1;
    assert_eq!(
        kind(go(&mut doc, level(&c.root, wide, 0, None))),
        ErrorKind::BadRequest
    );
    assert_eq!(
        kind(go(&mut doc, level(&c.root, 0, wide, None))),
        ErrorKind::BadRequest
    );
    assert_eq!(
        kind(go(&mut doc, level(&c.root, 0, 0, Some(&nobody)))),
        ErrorKind::NotFound
    );
    assert_eq!(
        kind(go(&mut doc, level(&c.root, 0, 0, Some("no ID")))),
        ErrorKind::BadRequest
    );
    assert_eq!(
        kind(doc.view_remember(level(&c.total, 0, 0, None))),
        ErrorKind::Refused
    );
    assert_eq!(
        kind(doc.view_remember(level(&c.root, -1, 0, None))),
        ErrorKind::BadRequest
    );
    assert_eq!(
        doc.set_folded(c.total.clone(), true).unwrap_err().kind,
        ErrorKind::Refused
    );
    assert_eq!(
        doc.set_folded(nobody, true).unwrap_err().kind,
        ErrorKind::NotFound
    );
    assert_eq!(
        doc.set_folded("no ID".into(), true).unwrap_err().kind,
        ErrorKind::BadRequest
    );

    // After all that: still at the root, nothing remembered, nothing to write.
    assert_eq!(doc.view_stage().unwrap(), c.root);
    assert_eq!(remembered(&doc, &c.root), None);
    assert!(doc.is_view_saved().unwrap());

    // The largest position there is is taken, and becomes the last column and row.
    let far = i64::from(u32::MAX);
    doc.view_remember(level(&c.root, far, far, None)).unwrap();
    assert_eq!(remembered(&doc, &c.root), Some(level(&c.root, 1, 2, None)));

    doc.close().unwrap();
    assert_eq!(doc.view_stage().unwrap_err().kind, ErrorKind::Closed);
    assert_eq!(kind(doc.view_go(c.root.clone(), None)), ErrorKind::Closed);
    assert_eq!(
        kind(doc.view_remember(level(&c.root, 0, 0, None))),
        ErrorKind::Closed
    );
    assert_eq!(
        doc.set_folded(c.costs, true).unwrap_err().kind,
        ErrorKind::Closed
    );
    assert_eq!(doc.is_view_saved().unwrap_err().kind, ErrorKind::Closed);
    assert_eq!(kind(doc.save_view()), ErrorKind::Closed);
}

/// Where the view state cannot be written: a file is in the place of its folder.
fn block_view_state(path: &Path) {
    std::fs::write(path.parent().unwrap().join(storage::VIEW_DIR), "in the way").unwrap();
}

/// The view state is a convenience. That it cannot be written is told to who asks for
/// just that, and never stands in the way of the document.
#[test]
fn a_view_state_that_cannot_be_written_does_not_fail_closing() {
    // A saved document closes.
    let (mut doc, path) = open_example("blocked-saved");
    let c = cells(&doc);
    block_view_state(&path);
    doc.view_go(c.shopping.clone(), None).unwrap();
    // (The core will not write into a folder that is not one: `Invalid`. A disk that is
    // full or a folder that may not be written would be `Io`.)
    assert_eq!(doc.save_view().unwrap_err().kind, ErrorKind::Invalid);
    assert!(!doc.is_view_saved().unwrap());
    doc.close().unwrap();
    assert_eq!(doc.id().unwrap_err().kind, ErrorKind::Closed);
    // The file is free again, and the screen starts at the root.
    assert_eq!(open(&path).view_stage().unwrap(), c.root);

    // An unsaved one is saved and closes.
    let (mut doc, path) = open_example("blocked-unsaved");
    block_view_state(&path);
    doc.view_go(cells(&doc).shopping, None).unwrap();
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.close().unwrap();
    assert_eq!(text_in_file(&path), "ours");

    // And one that is dropped is saved all the same.
    let (mut doc, path) = open_example("blocked-dropped");
    block_view_state(&path);
    doc.view_go(cells(&doc).shopping, None).unwrap();
    doc.apply(set_text(&doc, "ours")).unwrap();
    drop(doc);
    assert_eq!(text_in_file(&path), "ours");
}

/// A close that cannot save the document leaves it open, and the view state unwritten
/// but not lost: it goes out with the close that works.
#[test]
fn a_close_that_fails_keeps_the_view_state_for_the_next() {
    let (mut doc, path) = open_example("close-fails");
    let c = cells(&doc);
    doc.view_go(c.costs.clone(), None).unwrap();
    doc.apply(set_text(&doc, "ours")).unwrap();
    change_by_someone_else(&path);
    assert_eq!(doc.close().unwrap_err().kind, ErrorKind::ChangedOnDisk);
    assert!(!view_file(&path).exists());
    assert_eq!(doc.view_stage().unwrap(), c.costs);
    doc.overwrite().unwrap();
    doc.close().unwrap();
    assert_eq!(in_file(&path).unwrap().zoom, CellId::parse(&c.costs));
}

/// A handle that is let go of unclosed writes the view state as it saves the document:
/// if it can, without a word. Also when the document itself is left alone because
/// another program changed its file: where the screen was is this device's to say.
#[test]
fn dropping_writes_the_view_state() {
    let (mut doc, path) = open_example("drop");
    let c = cells(&doc);
    doc.view_go(c.shopping.clone(), None).unwrap();
    drop(doc);
    assert_eq!(in_file(&path).unwrap().zoom, CellId::parse(&c.shopping));

    let mut doc = open(&path);
    let id = DocId::parse(&doc.id().unwrap()).unwrap();
    doc.view_go(c.costs.clone(), None).unwrap();
    doc.apply(set_text(&doc, "ours")).unwrap();
    change_by_someone_else(&path);
    drop(doc);
    assert_eq!(text_in_file(&path), "theirs");
    // (The other program's document is a copy with an ID of its own, so the file of this
    // document's view state is found by the ID it had.)
    let view = storage::load_view_state(path.parent().unwrap(), id).unwrap();
    assert_eq!(view.unwrap().zoom, CellId::parse(&c.costs));
}

/// A copy of a document takes the view state along, on its own cells.
#[test]
fn a_copy_starts_where_the_original_was_left() {
    let (mut doc, path) = open_example("copy-from");
    let c = cells(&doc);
    doc.set_folded(c.costs.clone(), true).unwrap();
    doc.view_go(
        c.shopping.clone(),
        Some(level(&c.root, 1, 2, Some(&c.total))),
    )
    .unwrap();
    doc.close().unwrap();

    let to = path.with_file_name("copy.wombat");
    let copy = OpenDocument::copy_with(&path, &to, wombat_core::SeededIds::from_u64(12)).unwrap();
    let k = cells(&copy);
    assert_ne!(k.shopping, c.shopping);
    assert_eq!(copy.view_stage().unwrap(), k.shopping);
    assert_eq!(
        remembered(&copy, &k.root),
        Some(level(&k.root, 1, 2, Some(&k.total)))
    );
    assert!(folded(&copy, &k.costs) && !folded(&copy, &k.shopping));
    assert!(copy.is_view_saved().unwrap());
}

/// A new document starts at its root, which has no grid yet: there is no level to
/// remember, and an interface that is put away there is told so and goes on.
#[test]
fn a_new_document_has_no_level_to_remember() {
    let path = common::folder("view", "new").join("new.wombat");
    let mut doc = OpenDocument::create_with(&path, ids()).unwrap();
    let root = doc.root().unwrap();
    assert_eq!(doc.view_stage().unwrap(), root);
    doc.view_go(root.clone(), None).unwrap();
    let refused = doc.view_remember(level(&root, 0, 0, None)).unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Refused);
    assert!(doc.is_view_saved().unwrap());
    assert_eq!(remembered(&doc, &root), None);
}
