//! What the bridge does when something panics, which the core is written never to do,
//! and how the library starts with that in mind. No panic can be caused from outside, so
//! these tests are inside the crate and use switches in `api/document.rs` that exist only
//! in this test build: nothing that panics on purpose is in the library that an app loads.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use wombat_core::SeededIds;

use crate::api::document::OpenDocument;
use crate::api::wire::{BridgeError, Command, ErrorKind};

/// A new document with unsaved work in it: the root has a grid of one cell, the file
/// does not have it yet.
fn unsaved(name: &str) -> (OpenDocument, PathBuf) {
    let dir = std::env::temp_dir().join(format!("wombat-bridge-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("new.wombat");
    let mut doc = OpenDocument::create_with(&path, SeededIds::from_u64(5)).unwrap();
    let grid = Command::CreateGrid {
        cell: doc.root().unwrap(),
        xs: 1,
        ys: 1,
    };
    doc.apply(grid).unwrap();
    assert!(!doc.is_saved().unwrap());
    (doc, path)
}

fn remove(path: &std::path::Path) {
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

/// The drop of a handle runs where a panic that came out would end the app.
#[test]
fn a_panic_while_a_dropped_document_is_saved_stays_inside_the_drop() {
    let (mut doc, path) = unsaved("drop");
    let before = std::fs::read(&path).unwrap();
    doc.panic_in_write();
    // The switch works: a save that is asked for does panic.
    assert!(catch_unwind(AssertUnwindSafe(|| doc.save())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| drop(doc))).is_ok());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    // The file is free all the same.
    let again = OpenDocument::open_with(&path, SeededIds::from_u64(5)).unwrap();
    assert_eq!(again.is_saved().map_err(|e| e.kind), Ok(true));
    let twice = OpenDocument::open_with(&path, SeededIds::from_u64(5));
    assert_eq!(twice.err().unwrap().kind, ErrorKind::AlreadyOpen);
    drop(again);
    remove(&path);
}

fn is_internal<T: std::fmt::Debug>(result: Result<T, BridgeError>) {
    let error = result.expect_err("a call on a document that may be half changed was answered");
    assert_eq!(error.kind, ErrorKind::Internal, "{}", error.message);
    assert!(!error.message.is_empty());
}

/// A panic in the middle of a change leaves the document in a state nobody designed. For
/// each call that changes a document: after it panicked, nothing is read from the
/// document, nothing is done to it and nothing is saved from it, and its file is free
/// once the handle is dropped.
#[test]
fn after_a_change_that_did_not_finish_the_document_is_not_used_again() {
    type Change = fn(&mut OpenDocument);
    let apply: Change = |doc| {
        let text = Command::SetText {
            cell: doc.root().unwrap(),
            text: "half done".into(),
        };
        let _ = doc.apply(text);
    };
    let undo: Change = |doc| drop(doc.undo());
    let redo: Change = |doc| {
        doc.undo().unwrap().unwrap();
        doc.panic_in_change();
        let _ = doc.redo();
    };
    for (name, change) in [("apply", apply), ("undo", undo), ("redo", redo)] {
        let (mut doc, path) = unsaved(name);
        // Half of what there is to lose is in the file, half only in the document.
        doc.save().unwrap();
        let more = Command::SetText {
            cell: doc.root().unwrap(),
            text: "unsaved".into(),
        };
        doc.apply(more).unwrap();
        let (root, file) = (doc.root().unwrap(), std::fs::read(&path).unwrap());
        if name != "redo" {
            doc.panic_in_change();
        }
        assert!(catch_unwind(AssertUnwindSafe(|| change(&mut doc))).is_err());

        is_internal(doc.id());
        is_internal(doc.root());
        is_internal(doc.path());
        is_internal(doc.is_saved());
        is_internal(doc.grid(root.clone()));
        is_internal(doc.grid("no ID".into()));
        is_internal(doc.apply(Command::RemoveGrid { cell: root.clone() }));
        is_internal(doc.undo());
        is_internal(doc.redo());
        is_internal(doc.save());
        is_internal(doc.overwrite());
        is_internal(doc.close());
        assert_eq!(std::fs::read(&path).unwrap(), file, "{name}");
        // Not closed: it holds its file until the handle is let go of.
        let held = OpenDocument::open_with(&path, SeededIds::from_u64(5));
        assert_eq!(held.err().unwrap().kind, ErrorKind::AlreadyOpen);
        // Dropping it saves nothing, though it was unsaved, and frees the file. Also
        // where the file is gone: half a change is not what to bring it back with.
        if name == "undo" {
            std::fs::remove_file(&path).unwrap();
            drop(doc);
            assert!(!path.exists());
            std::fs::write(&path, &file).unwrap();
        } else {
            drop(doc);
            assert_eq!(std::fs::read(&path).unwrap(), file, "{name}");
        }
        let again = OpenDocument::open_with(&path, SeededIds::from_u64(5)).unwrap();
        assert_eq!(again.grid(root).unwrap().cell.text, "");
        drop(again);
        remove(&path);
    }
}

/// The switch itself changes nothing: a call that fails with an error value, as calls
/// do every day, leaves the document in use.
#[test]
fn a_change_that_is_refused_with_an_error_is_finished() {
    let (mut doc, path) = unsaved("refused");
    let root = doc.root().unwrap();
    let refused = doc.apply(Command::RemoveGrid {
        cell: "no ID".into(),
    });
    assert_eq!(refused.unwrap_err().kind, ErrorKind::BadRequest);
    let rows = Command::InsertRows {
        cell: root.clone(),
        at: 99,
        count: 1,
    };
    assert_eq!(doc.apply(rows).unwrap_err().kind, ErrorKind::Refused);
    assert_eq!(doc.grid(root).unwrap().cells.len(), 1);
    doc.close().unwrap();
    remove(&path);
}

/// Starting the library must not write to the environment of the process: other threads
/// of the app are running by then and may be reading it, which is not safe to do at the
/// same time in C. (`flutter_rust_bridge`'s default start sets `RUST_BACKTRACE`.)
#[test]
fn starting_the_library_leaves_the_environment_alone() {
    // The test itself writes to the environment, to see the variable appear. That is
    // safe here and not in an app: everything else in this test program reads the
    // environment through Rust's standard library, which takes a lock.
    let name = "RUST_BACKTRACE";
    let before = std::env::var_os(name);
    std::env::remove_var(name);
    crate::api::document::init_app();
    let after = std::env::var_os(name);
    if let Some(before) = before {
        std::env::set_var(name, before);
    }
    assert_eq!(after, None);
}
