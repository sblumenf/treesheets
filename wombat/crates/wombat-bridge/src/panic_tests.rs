//! What an open document does when something panics, which the core is written never to
//! do. No panic can be caused from outside, so these tests are inside the crate and use
//! switches in `api/document.rs` that exist only in this test build: nothing that panics
//! on purpose is in the library that an app loads.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use wombat_core::SeededIds;

use crate::api::document::OpenDocument;
use crate::api::wire::{Command, ErrorKind};

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
