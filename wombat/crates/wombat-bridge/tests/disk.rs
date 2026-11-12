//! An open document and its file: when saving writes, when it refuses because another
//! program changed the file, and what the flag "saved" means.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::path::Path;

use common::{example, folder, ids, set_text, text_in_file};
use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{BridgeError, Command, ErrorKind, Report};

fn open_example(name: &str) -> (OpenDocument, std::path::PathBuf) {
    common::open_example("disk", name)
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

fn changed_on_disk(error: BridgeError, path: &Path) {
    assert_eq!(error.kind, ErrorKind::ChangedOnDisk, "{}", error.message);
    assert_eq!(error.path, Some(path.display().to_string()));
    assert!(!error.message.is_empty());
}

/// Which file is under the name: every save puts a new one there.
#[cfg(unix)]
fn file_number(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).unwrap().ino()
}

#[test]
fn unsaved_and_the_file_untouched_is_written() {
    let (mut doc, path) = open_example("unsaved-unchanged");
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.save().unwrap();
    assert!(doc.is_saved().unwrap());
    assert_eq!(text_in_file(&path), "ours");
    // What was written is remembered as the document's own: the next save is not
    // taken for a change by someone else, however often.
    for text in ["a", "b", "c"] {
        doc.apply(set_text(&doc, text)).unwrap();
        doc.save().unwrap();
        assert_eq!(text_in_file(&path), text);
    }
}

#[cfg(unix)]
#[test]
fn saved_and_the_file_untouched_is_not_written_again() {
    let (mut doc, path) = open_example("saved-unchanged");
    let before = file_number(&path);
    doc.save().unwrap();
    assert_eq!(file_number(&path), before);
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.save().unwrap();
    let written = file_number(&path);
    assert_ne!(written, before);
    doc.save().unwrap();
    assert_eq!(file_number(&path), written);
}

#[test]
fn unsaved_and_the_file_gone_is_written() {
    let (mut doc, path) = open_example("unsaved-gone");
    doc.apply(set_text(&doc, "ours")).unwrap();
    std::fs::remove_file(&path).unwrap();
    doc.save().unwrap();
    assert!(doc.is_saved().unwrap());
    assert_eq!(text_in_file(&path), "ours");
}

#[test]
fn saved_and_the_file_gone_is_written() {
    let (mut doc, path) = open_example("saved-gone");
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.save().unwrap();
    std::fs::remove_file(&path).unwrap();
    // "Saved" is about what was done to the document, not about the disk: it does not
    // look at the file.
    assert!(doc.is_saved().unwrap());
    doc.save().unwrap();
    assert!(doc.is_saved().unwrap());
    assert_eq!(text_in_file(&path), "ours");
    // And that file is the document's own again.
    doc.apply(set_text(&doc, "more")).unwrap();
    doc.save().unwrap();
    assert_eq!(text_in_file(&path), "more");
}

#[test]
fn unsaved_and_the_file_changed_is_refused_and_not_overwritten() {
    let (mut doc, path) = open_example("unsaved-changed");
    doc.apply(set_text(&doc, "ours")).unwrap();
    change_by_someone_else(&path);
    changed_on_disk(doc.save().err().unwrap(), &path);
    assert_eq!(text_in_file(&path), "theirs");
    assert!(!doc.is_saved().unwrap());
    // Being refused once does not make the file the document's own.
    changed_on_disk(doc.save().err().unwrap(), &path);
    assert_eq!(text_in_file(&path), "theirs");
    // The document is as it was and can be worked on.
    let root = doc.grid(doc.root().unwrap()).unwrap();
    assert_eq!(root.cells[1].text, "ours");
    doc.apply(set_text(&doc, "ours still")).unwrap();
}

#[test]
fn saved_and_the_file_changed_is_refused_and_not_overwritten() {
    let (mut doc, path) = open_example("saved-changed");
    change_by_someone_else(&path);
    assert!(doc.is_saved().unwrap());
    changed_on_disk(doc.save().err().unwrap(), &path);
    assert_eq!(text_in_file(&path), "theirs");
    assert!(doc.is_saved().unwrap());
}

/// The document is reached through a link whose file is then removed: saving would have
/// to guess where to write, and the core refuses to.
#[cfg(unix)]
#[test]
fn a_link_that_leads_nowhere_is_an_error_saved_or_not() {
    let dir = folder("disk", "dangling");
    let (real, link) = (dir.join("real.wombat"), dir.join("link.wombat"));
    drop(OpenDocument::copy_with(&example(), &real, ids()).unwrap());
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let mut doc = OpenDocument::open_with(&link, ids()).unwrap();
    // Through a link that leads somewhere, saving keeps the link and knows its file.
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.save().unwrap();
    doc.apply(set_text(&doc, "again")).unwrap();
    doc.save().unwrap();
    assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
    assert_eq!(text_in_file(&real), "again");

    std::fs::remove_file(&real).unwrap();
    let saved = doc.save().err().unwrap();
    assert_eq!(saved.kind, ErrorKind::Io);
    assert_eq!(saved.path, Some(link.display().to_string()));
    assert!(doc.is_saved().unwrap());
    doc.apply(set_text(&doc, "unsaved")).unwrap();
    let unsaved = doc.save().err().unwrap();
    assert_eq!(unsaved.kind, ErrorKind::Io);
    assert!(!doc.is_saved().unwrap());
    // Nor on purpose: there is no file to go over.
    assert_eq!(doc.overwrite().err().unwrap().kind, ErrorKind::Io);
    assert!(!doc.is_saved().unwrap());
    assert!(!real.exists());
    assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
}

#[test]
fn a_new_document_and_a_copy_know_their_files() {
    let path = folder("disk", "create").join("new.wombat");
    let mut doc = OpenDocument::create_with(&path, ids()).unwrap();
    let grid = Command::CreateGrid {
        cell: doc.root().unwrap(),
        xs: 1,
        ys: 1,
    };
    doc.apply(grid).unwrap();
    doc.save().unwrap();
    change_by_someone_else(&path);
    doc.undo().unwrap();
    changed_on_disk(doc.save().err().unwrap(), &path);

    let (mut copy, path) = open_example("copy");
    copy.apply(set_text(&copy, "ours")).unwrap();
    copy.save().unwrap();
    change_by_someone_else(&path);
    changed_on_disk(copy.save().err().unwrap(), &path);
}

#[test]
fn a_command_that_changes_nothing_leaves_the_document_saved() {
    let (mut doc, _) = open_example("no-op");
    let same = doc.grid(doc.root().unwrap()).unwrap().cells[1].text.clone();
    let report = doc.apply(set_text(&doc, &same)).unwrap();
    assert_eq!(report, Report::default());
    assert!(doc.is_saved().unwrap());
    assert_eq!(doc.undo().unwrap(), None);
    assert!(doc.is_saved().unwrap());
    assert_eq!(doc.redo().unwrap(), None);
    assert!(doc.is_saved().unwrap());
}

#[test]
fn undo_and_redo_make_the_document_unsaved() {
    let (mut doc, path) = open_example("undo");
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.save().unwrap();
    // The file has "ours"; after undo the document does not.
    assert!(doc.undo().unwrap().is_some());
    assert!(!doc.is_saved().unwrap());
    doc.save().unwrap();
    assert_eq!(text_in_file(&path), "what to buy");
    assert!(doc.redo().unwrap().is_some());
    assert!(!doc.is_saved().unwrap());
    doc.save().unwrap();
    assert_eq!(text_in_file(&path), "ours");
    // Undo and then redo give the document that was saved, but "saved" is a flag that any
    // change clears, not a comparison: it says unsaved, and saving writes the same bytes.
    let bytes = std::fs::read(&path).unwrap();
    doc.undo().unwrap();
    doc.redo().unwrap();
    assert!(!doc.is_saved().unwrap());
    doc.save().unwrap();
    assert!(doc.is_saved().unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

/// `create` goes over nothing, not even a link that leads nowhere, which a save would
/// refuse to write through anyway but `Path::exists` does not see.
#[cfg(unix)]
#[test]
fn a_new_document_does_not_go_over_a_link_that_leads_nowhere() {
    let dir = folder("disk", "create-dangling");
    let link = dir.join("link.wombat");
    std::os::unix::fs::symlink(dir.join("nowhere.wombat"), &link).unwrap();
    let error = OpenDocument::create_with(&link, ids()).err().unwrap();
    assert_eq!(error.kind, ErrorKind::AlreadyExists);
    assert_eq!(error.path, Some(link.display().to_string()));
    assert!(!dir.join("nowhere.wombat").exists());
}

#[test]
fn overwriting_is_asked_for_by_name() {
    let (mut doc, path) = open_example("overwrite");
    doc.apply(set_text(&doc, "ours")).unwrap();
    change_by_someone_else(&path);
    changed_on_disk(doc.save().err().unwrap(), &path);
    doc.overwrite().unwrap();
    assert!(doc.is_saved().unwrap());
    assert_eq!(text_in_file(&path), "ours");
    // The file is the document's own again.
    doc.apply(set_text(&doc, "more")).unwrap();
    doc.save().unwrap();
    assert_eq!(text_in_file(&path), "more");
    // With nothing unsaved it writes too: the file may hold anything.
    change_by_someone_else(&path);
    assert!(doc.is_saved().unwrap());
    doc.overwrite().unwrap();
    assert_eq!(text_in_file(&path), "more");
    // And where the file is gone.
    std::fs::remove_file(&path).unwrap();
    doc.overwrite().unwrap();
    assert_eq!(text_in_file(&path), "more");
}
