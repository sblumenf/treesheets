//! One open document per file: opening, creating or copying to a file that a document of
//! this process has open is refused, however the path is spelt, and the file is free
//! again when the document is let go of.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};

use common::{example, folder, ids, set_text, text_in_file};
use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{BridgeError, ErrorKind};
use wombat_core::SeededIds;

fn open_example(name: &str) -> (OpenDocument, PathBuf) {
    common::open_example("open_files", name)
}

fn already_open(result: Result<OpenDocument, BridgeError>, path: &Path) {
    let error = result.err().expect("opened a second time");
    assert_eq!(error.kind, ErrorKind::AlreadyOpen, "{}", error.message);
    assert_eq!(error.path, Some(path.display().to_string()));
    assert!(!error.message.is_empty());
}

#[test]
fn a_file_is_open_once_and_free_when_the_document_is_dropped() {
    let (mut doc, path) = open_example("twice");
    already_open(OpenDocument::open_with(&path, ids()), &path);
    already_open(OpenDocument::open(path.display().to_string()), &path);
    // The refusals took nothing from the first.
    doc.apply(set_text(&doc, "ours")).unwrap();
    doc.save().unwrap();
    drop(doc);
    let again = OpenDocument::open_with(&path, ids()).unwrap();
    assert_eq!(
        again.grid(again.root().unwrap()).unwrap().cells[1].text,
        "ours"
    );
    already_open(OpenDocument::open_with(&path, ids()), &path);
}

/// The file of an open document can be gone (another program removed it); the document
/// will write it again, so nothing else may be made there.
#[test]
fn nothing_is_created_or_copied_where_a_document_is_open() {
    let (doc, path) = open_example("create");
    // While the file is there, this is said first: "it is open" tells more than "it
    // exists".
    already_open(OpenDocument::create_with(&path, ids()), &path);
    already_open(OpenDocument::copy_with(&example(), &path, ids()), &path);
    std::fs::remove_file(&path).unwrap();
    already_open(OpenDocument::create_with(&path, ids()), &path);
    already_open(OpenDocument::create(path.display().to_string()), &path);
    already_open(OpenDocument::copy_with(&example(), &path, ids()), &path);
    let text = |path: &Path| path.display().to_string();
    already_open(
        OpenDocument::open_copy(text(&example()), text(&path)),
        &path,
    );
    assert!(!path.exists());
    drop(doc);
    OpenDocument::create_with(&path, ids()).unwrap();
}

/// What is copied from may be open: a copy only reads it.
#[test]
fn a_copy_may_be_made_of_a_document_that_is_open() {
    let (doc, path) = open_example("copy-from");
    let to = path.with_file_name("copy.wombat");
    // Other IDs than the first copy drew, which had the same seed.
    let copy = OpenDocument::copy_with(&path, &to, SeededIds::from_u64(12)).unwrap();
    assert_ne!(copy.id().unwrap(), doc.id().unwrap());
    // And the copy is open in its turn.
    already_open(OpenDocument::open_with(&to, ids()), &to);
}

/// `path` from the current folder, through as many `..` as it takes to reach the root.
#[cfg(unix)]
fn relative(path: &Path) -> PathBuf {
    let here = std::env::current_dir().unwrap();
    let mut up: PathBuf = here.components().skip(1).map(|_| "..").collect();
    up.push(path.strip_prefix("/").unwrap());
    assert!(up.is_relative());
    up
}

#[cfg(unix)]
#[test]
fn another_spelling_of_the_path_is_the_same_file() {
    let (doc, path) = open_example("spelling");
    let dir = path.parent().unwrap();
    std::fs::create_dir(dir.join("sub")).unwrap();
    std::os::unix::fs::symlink(dir, dir.join("folder-link")).unwrap();
    std::os::unix::fs::symlink(&path, dir.join("file-link.wombat")).unwrap();
    let spellings = [
        dir.join("sub/../example.wombat"),
        dir.join("./example.wombat"),
        dir.join("folder-link/example.wombat"),
        dir.join("file-link.wombat"),
        relative(&path),
    ];
    for other in &spellings {
        already_open(OpenDocument::open_with(other, ids()), other);
    }
    // Also where the file is not there, so that only its folder has a real name.
    std::fs::remove_file(&path).unwrap();
    for other in &spellings {
        let created = OpenDocument::create_with(other, ids());
        let copied = OpenDocument::copy_with(&example(), other, ids());
        if other.ends_with("file-link.wombat") {
            // The link leads nowhere now, so it does not name the file any more; but
            // nothing is ever made through such a link.
            assert_eq!(created.err().unwrap().kind, ErrorKind::AlreadyExists);
            assert_eq!(copied.err().unwrap().kind, ErrorKind::AlreadyExists);
        } else {
            already_open(created, other);
            already_open(copied, other);
        }
    }
    assert!(!path.exists());
    drop(doc);
    // A document opened through a link holds the file the link leads to.
    let real = dir.join("real.wombat");
    drop(OpenDocument::copy_with(&example(), &real, ids()).unwrap());
    let link = dir.join("link.wombat");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let through = OpenDocument::open_with(&link, ids()).unwrap();
    already_open(OpenDocument::open_with(&real, ids()), &real);
    already_open(OpenDocument::open_with(&link, ids()), &link);
    drop(through);
    OpenDocument::open_with(&real, ids()).unwrap();
}

/// A document that was opened through a link is the file the link leads to and the
/// file of that name: it saves to the name, so nothing else may be made or opened there
/// when the link is gone or leads elsewhere.
#[cfg(unix)]
#[test]
fn a_document_opened_through_a_link_holds_both_names() {
    use std::os::unix::fs::symlink;
    let (doc, target) = open_example("link-names");
    drop(doc);
    let dir = target.parent().unwrap();
    let (link, second) = (dir.join("link.wombat"), dir.join("second.wombat"));
    symlink(&target, &link).unwrap();
    symlink(&target, &second).unwrap();
    let mut doc = OpenDocument::open_with(&link, ids()).unwrap();
    assert_eq!(doc.path().unwrap(), link.display().to_string());
    already_open(OpenDocument::open_with(&second, ids()), &second);
    already_open(OpenDocument::open_with(&target, ids()), &target);

    // The link is removed: its name is still this document's.
    doc.apply(set_text(&doc, "ours")).unwrap();
    std::fs::remove_file(&link).unwrap();
    already_open(OpenDocument::create_with(&link, ids()), &link);
    already_open(OpenDocument::copy_with(&example(), &link, ids()), &link);
    assert!(std::fs::symlink_metadata(&link).is_err());
    // The file is gone as far as the document can tell, so saving writes it.
    doc.save().unwrap();
    assert_eq!(text_in_file(&link), "ours");
    assert_eq!(text_in_file(&target), "what to buy");
    drop(doc);

    // The link is made to lead to another file: opening it would give a second document
    // that saves to the same name.
    std::fs::remove_file(&link).unwrap();
    symlink(&target, &link).unwrap();
    let doc = OpenDocument::open_with(&link, ids()).unwrap();
    let other = dir.join("other.wombat");
    drop(OpenDocument::copy_with(&example(), &other, SeededIds::from_u64(12)).unwrap());
    std::fs::remove_file(&link).unwrap();
    symlink(&other, &link).unwrap();
    already_open(OpenDocument::open_with(&link, ids()), &link);
    already_open(OpenDocument::open_with(&target, ids()), &target);
    // Every name is free again with the document.
    drop(doc);
    drop(OpenDocument::open_with(&target, ids()).unwrap());
    drop(OpenDocument::open_with(&link, ids()).unwrap());
    std::fs::remove_file(&link).unwrap();
    drop(OpenDocument::create_with(&link, ids()).unwrap());
}

/// The folder of an open document is removed, and made again: the document is still open
/// and will write its file there, so the name is not free at any moment of that, however
/// it is spelt.
#[cfg(unix)]
#[test]
fn a_document_holds_its_name_while_its_folder_is_gone_and_when_it_is_back() {
    let dir = folder("open_files", "folder-gone");
    let sub = dir.join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::os::unix::fs::symlink(&dir, dir.join("link")).unwrap();
    let real = sub.join("x.wombat");
    let through = dir.join("link/sub/x.wombat");
    let refused = |path: &Path| {
        already_open(OpenDocument::create_with(path, ids()), path);
        already_open(OpenDocument::copy_with(&example(), path, ids()), path);
        already_open(OpenDocument::open_with(path, ids()), path);
    };
    for (opened, other) in [(&real, &through), (&through, &real)] {
        let mut doc = OpenDocument::copy_with(&example(), opened, ids()).unwrap();
        doc.apply(set_text(&doc, "ours")).unwrap();
        // The folder is gone: nothing could be made there, but "it is open" is the answer.
        std::fs::remove_dir_all(&sub).unwrap();
        refused(opened);
        refused(other);
        assert_eq!(doc.save().unwrap_err().kind, ErrorKind::Io);
        // The folder is back, empty.
        std::fs::create_dir(&sub).unwrap();
        refused(opened);
        refused(other);
        assert!(!real.exists());
        doc.save().unwrap();
        assert_eq!(text_in_file(&real), "ours");
        drop(doc);
        std::fs::remove_file(&real).unwrap();
        drop(OpenDocument::create_with(other, ids()).unwrap());
        std::fs::remove_file(&real).unwrap();
    }
}

/// Two files of the same name in different folders are different files.
#[test]
fn another_file_of_the_same_name_is_not_refused() {
    let (_doc, path) = open_example("same-name");
    let other = folder("open_files", "same-name-2").join("example.wombat");
    OpenDocument::copy_with(&path, &other, SeededIds::from_u64(12)).unwrap();
}

#[test]
fn a_file_that_could_not_be_opened_is_not_held() {
    let dir = folder("open_files", "failed");
    // No such file: then create it.
    let path = dir.join("a.wombat");
    for _ in 0..2 {
        let error = OpenDocument::open_with(&path, ids()).err().unwrap();
        assert_eq!(error.kind, ErrorKind::Io);
    }
    drop(OpenDocument::create_with(&path, ids()).unwrap());
    // No path at all, which has no place from the root either.
    for _ in 0..2 {
        let error = OpenDocument::open(String::new()).err().unwrap();
        assert_eq!(
            (error.kind, error.path),
            (ErrorKind::Io, Some(String::new()))
        );
        let error = OpenDocument::create(String::new()).err().unwrap();
        assert_eq!(error.kind, ErrorKind::Io);
    }
    // Not a document: then put one there.
    let bad = dir.join("bad.wombat");
    std::fs::write(&bad, "not json").unwrap();
    for _ in 0..2 {
        let error = OpenDocument::open_with(&bad, ids()).err().unwrap();
        assert_eq!(error.kind, ErrorKind::Syntax);
    }
    std::fs::copy(example(), &bad).unwrap();
    drop(OpenDocument::open_with(&bad, ids()).unwrap());
    // A copy of nothing, twice, then of something.
    let to = dir.join("copy.wombat");
    for _ in 0..2 {
        let error = OpenDocument::copy_with(&dir.join("none.wombat"), &to, ids());
        assert_eq!(error.err().unwrap().kind, ErrorKind::Io);
    }
    drop(OpenDocument::copy_with(&example(), &to, ids()).unwrap());
    // Creating where a file is, twice: the second answer is the first again.
    for _ in 0..2 {
        let error = OpenDocument::create_with(&to, ids()).err().unwrap();
        assert_eq!(error.kind, ErrorKind::AlreadyExists);
    }
    // Creating in a folder that is not there.
    let deep = dir.join("no/such/folder/new.wombat");
    for _ in 0..2 {
        let error = OpenDocument::create_with(&deep, ids()).err().unwrap();
        assert_eq!(error.kind, ErrorKind::Io);
    }
    std::fs::create_dir_all(deep.parent().unwrap()).unwrap();
    OpenDocument::create_with(&deep, ids()).unwrap();
}

/// Many threads open one file at the same moment: one gets it. What each of the others
/// would save over the first is what this is for.
#[test]
fn of_opens_that_race_one_wins() {
    let (doc, path) = open_example("race");
    drop(doc);
    for round in 0..50 {
        let barrier = Arc::new(Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let (barrier, path) = (barrier.clone(), path.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    OpenDocument::open_with(&path, ids())
                })
            })
            .collect();
        let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        let won = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(won, 1, "round {round}");
        for result in results {
            if let Err(error) = result {
                assert_eq!(error.kind, ErrorKind::AlreadyOpen);
            }
        }
    }
    // And creating: one makes the file, and no other is told it may.
    let new = path.with_file_name("new.wombat");
    for round in 0..50 {
        let _ = std::fs::remove_file(&new);
        let barrier = Arc::new(Barrier::new(8));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let (barrier, new) = (barrier.clone(), new.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    OpenDocument::create_with(&new, ids())
                })
            })
            .collect();
        let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(
            results.iter().filter(|r| r.is_ok()).count(),
            1,
            "round {round}"
        );
    }
    assert_eq!(text_in_file(&path), "what to buy");
}
