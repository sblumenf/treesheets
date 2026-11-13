//! Letting go of an open document: closing saves and gives the document up, and a
//! document that is dropped unsaved is saved as a safety net, never over another
//! program's work.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::path::{Path, PathBuf};

use common::{example, folder, ids, set_text, text_in_file};
use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{BridgeError, Command, ErrorKind};

fn open_example(name: &str) -> (OpenDocument, PathBuf) {
    common::open_example("close", name)
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

/// Which file is under the name: every save puts a new one there.
#[cfg(unix)]
fn file_number(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).unwrap().ino()
}

#[test]
fn dropping_an_unsaved_document_saves_it() {
    let (mut doc, path) = open_example("drop-unsaved");
    doc.apply(set_text(&doc, "ours")).unwrap();
    drop(doc);
    assert_eq!(text_in_file(&path), "ours");
    // Also where the file is gone: the document is the only copy.
    let mut doc = OpenDocument::open_with(&path, ids()).unwrap();
    doc.apply(set_text(&doc, "again")).unwrap();
    std::fs::remove_file(&path).unwrap();
    drop(doc);
    assert_eq!(text_in_file(&path), "again");
}

/// A drop has no one to tell, so it leaves alone what it could only overwrite.
#[test]
fn dropping_never_goes_over_a_file_that_someone_else_changed() {
    let (mut doc, path) = open_example("drop-changed");
    doc.apply(set_text(&doc, "ours")).unwrap();
    change_by_someone_else(&path);
    drop(doc);
    assert_eq!(text_in_file(&path), "theirs");
}

#[cfg(unix)]
#[test]
fn dropping_a_saved_document_writes_nothing() {
    let (mut doc, path) = open_example("drop-saved");
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.save().unwrap();
    let written = file_number(&path);
    drop(doc);
    assert_eq!(file_number(&path), written);
    // Nor does it bring back a file that was removed.
    let doc = OpenDocument::open_with(&path, ids()).unwrap();
    std::fs::remove_file(&path).unwrap();
    drop(doc);
    assert!(!path.exists());
}

/// Through a link that leads nowhere nothing can be saved; the drop must take that
/// quietly.
#[cfg(unix)]
#[test]
fn dropping_where_nothing_can_be_saved_is_quiet() {
    let dir = folder("close", "drop-dangling");
    let (real, link) = (dir.join("real.wombat"), dir.join("link.wombat"));
    drop(OpenDocument::copy_with(&example(), &real, ids()).unwrap());
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let mut doc = OpenDocument::open_with(&link, ids()).unwrap();
    doc.apply(set_text(&doc, "ours")).unwrap();
    std::fs::remove_file(&real).unwrap();
    drop(doc);
    assert!(!real.exists());
    // The file is free all the same.
    std::fs::copy(example(), &real).unwrap();
    OpenDocument::open_with(&link, ids()).unwrap();
}

fn is_closed<T: std::fmt::Debug>(result: Result<T, BridgeError>) {
    let error = result.expect_err("a call on a closed document was answered");
    assert_eq!(error.kind, ErrorKind::Closed, "{}", error.message);
    assert_eq!(error.path, None);
    assert!(!error.message.is_empty());
}

/// Every call there is, on a document that was closed.
fn every_call_is_refused(doc: &mut OpenDocument) {
    let nowhere = "00000000-0000-4000-8000-000000000000".to_string();
    is_closed(doc.id());
    is_closed(doc.root());
    is_closed(doc.path());
    is_closed(doc.is_saved());
    is_closed(doc.grid(nowhere.clone()));
    // What is wrong with the request does not come before that the document is closed.
    is_closed(doc.grid("no ID".into()));
    is_closed(doc.apply(Command::RemoveGrid {
        cell: nowhere.clone(),
    }));
    is_closed(doc.apply(Command::RemoveGrid {
        cell: "no ID".into(),
    }));
    is_closed(doc.undo());
    is_closed(doc.redo());
    is_closed(doc.save());
    is_closed(doc.overwrite());
    is_closed(doc.close());
    doc.set_fixed_time(Some(1));
}

#[test]
fn closing_saves_and_gives_the_document_up() {
    let (mut doc, path) = open_example("close");
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.close().unwrap();
    assert_eq!(text_in_file(&path), "ours");
    every_call_is_refused(&mut doc);
    // The file is free while the handle is still held, and what the handle held is gone:
    // dropping it later touches nothing.
    let mut again = OpenDocument::open_with(&path, ids()).unwrap();
    again.apply(set_text(&again, "the second")).unwrap();
    again.save().unwrap();
    drop(doc);
    assert_eq!(text_in_file(&path), "the second");
    // What was undone at the time is what is closed.
    again.undo().unwrap();
    again.close().unwrap();
    assert_eq!(text_in_file(&path), "ours");
}

#[test]
fn closing_writes_a_file_that_is_gone_if_there_is_unsaved_work() {
    let (mut doc, path) = open_example("close-gone");
    doc.apply(set_text(&doc, "ours")).unwrap();
    std::fs::remove_file(&path).unwrap();
    doc.close().unwrap();
    assert_eq!(text_in_file(&path), "ours");
    every_call_is_refused(&mut doc);
}

/// A document that was only read, or saved already, closes without a look at its file:
/// there is nothing of it to lose and nothing to put anywhere.
#[cfg(unix)]
#[test]
fn closing_with_nothing_unsaved_writes_nothing_whatever_the_file() {
    let (mut doc, path) = open_example("close-saved");
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.save().unwrap();
    let written = file_number(&path);
    doc.close().unwrap();
    assert_eq!(file_number(&path), written);
    every_call_is_refused(&mut doc);

    // Changed by someone else: theirs stays, and the close succeeds.
    let mut doc = OpenDocument::open_with(&path, ids()).unwrap();
    change_by_someone_else(&path);
    doc.close().unwrap();
    assert_eq!(text_in_file(&path), "theirs");
    every_call_is_refused(&mut doc);

    // Removed: it is not brought back.
    let mut doc = OpenDocument::open_with(&path, ids()).unwrap();
    std::fs::remove_file(&path).unwrap();
    doc.close().unwrap();
    assert!(!path.exists());

    // A link that leads nowhere by now.
    let real = path.with_file_name("real.wombat");
    drop(OpenDocument::copy_with(&example(), &real, ids()).unwrap());
    std::os::unix::fs::symlink(&real, &path).unwrap();
    let mut doc = OpenDocument::open_with(&path, ids()).unwrap();
    std::fs::remove_file(&real).unwrap();
    doc.close().unwrap();
    assert!(!real.exists());
}

#[test]
fn a_close_that_cannot_save_leaves_the_document_open() {
    let (mut doc, path) = open_example("close-refused");
    doc.apply(set_text(&doc, "ours")).unwrap();
    change_by_someone_else(&path);
    for _ in 0..2 {
        let error = doc.close().err().unwrap();
        assert_eq!(error.kind, ErrorKind::ChangedOnDisk);
        assert_eq!(error.path, Some(path.display().to_string()));
        assert_eq!(text_in_file(&path), "theirs");
    }
    // Open as it was: readable, unsaved, with its history, and holding its file.
    assert!(!doc.is_saved().unwrap());
    let root = doc.grid(doc.root().unwrap()).unwrap();
    assert_eq!(root.cells[1].text, "ours");
    let held = OpenDocument::open_with(&path, ids()).err().unwrap();
    assert_eq!(held.kind, ErrorKind::AlreadyOpen);
    assert!(doc.undo().unwrap().is_some());
    assert!(doc.redo().unwrap().is_some());
    // The way out that keeps this document: overwrite, then close.
    doc.overwrite().unwrap();
    doc.close().unwrap();
    assert_eq!(text_in_file(&path), "ours");
    every_call_is_refused(&mut doc);
    OpenDocument::open_with(&path, ids()).unwrap();
}

/// The way out that keeps the other program's file: let go of the handle. The drop sees
/// the change too and writes nothing.
#[test]
fn after_a_refused_close_dropping_keeps_the_file_of_the_other() {
    let (mut doc, path) = open_example("close-refused-drop");
    doc.apply(set_text(&doc, "ours")).unwrap();
    change_by_someone_else(&path);
    assert_eq!(doc.close().err().unwrap().kind, ErrorKind::ChangedOnDisk);
    drop(doc);
    assert_eq!(text_in_file(&path), "theirs");
    OpenDocument::open_with(&path, ids()).unwrap();
}

#[cfg(unix)]
#[test]
fn a_close_through_a_link_that_leads_nowhere_fails_and_leaves_the_document_open() {
    let dir = folder("close", "close-dangling");
    let (real, link) = (dir.join("real.wombat"), dir.join("link.wombat"));
    drop(OpenDocument::copy_with(&example(), &real, ids()).unwrap());
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let mut doc = OpenDocument::open_with(&link, ids()).unwrap();
    doc.apply(set_text(&doc, "ours")).unwrap();
    std::fs::remove_file(&real).unwrap();
    let error = doc.close().err().unwrap();
    assert_eq!(error.kind, ErrorKind::Io);
    assert!(!doc.is_saved().unwrap());
    // Once the link is gone too, nothing is at the path, and closing writes there.
    std::fs::remove_file(&link).unwrap();
    doc.close().unwrap();
    assert_eq!(text_in_file(&link), "ours");
}
