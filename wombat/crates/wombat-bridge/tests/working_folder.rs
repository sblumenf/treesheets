//! An open document knows its file by the path from the root that it had when it was
//! opened: where the program's working folder moves to afterwards changes nothing.
//!
//! This is the only test in this file, and must stay so: the working folder belongs to the
//! whole process, and the tests of one file run side by side in one process.
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
use wombat_bridge::api::wire::ErrorKind;
use wombat_core::SeededIds;

#[test]
fn a_relative_path_names_the_same_file_after_the_working_folder_changed() {
    let one = folder("working_folder", "one").canonicalize().unwrap();
    let two = folder("working_folder", "two").canonicalize().unwrap();
    std::env::set_current_dir(&one).unwrap();
    let name = Path::new("rel.wombat");
    let mut doc = OpenDocument::copy_with(&example(), name, ids()).unwrap();
    let file = one.join(name);
    assert_eq!(doc.path().unwrap(), file.display().to_string());
    doc.apply(set_text(&doc, "mine")).unwrap();

    std::env::set_current_dir(&two).unwrap();
    assert_eq!(doc.path().unwrap(), file.display().to_string());
    doc.save().unwrap();
    assert!(doc.is_saved().unwrap());
    assert_eq!(text_in_file(&file), "mine");
    assert!(!two.join(name).exists());

    // The name now means another file, which is free; the document's own is not.
    let other = OpenDocument::copy_with(&example(), name, SeededIds::from_u64(12)).unwrap();
    assert_eq!(other.path().unwrap(), two.join(name).display().to_string());
    let again = OpenDocument::open_with(&file, ids());
    assert_eq!(again.err().unwrap().kind, ErrorKind::AlreadyOpen);

    // Closing and dropping go to the same file as saving.
    doc.apply(set_text(&doc, "closed")).unwrap();
    doc.close().unwrap();
    assert_eq!(text_in_file(&file), "closed");
    let mut doc = OpenDocument::open_with(&file, ids()).unwrap();
    std::env::set_current_dir(&one).unwrap();
    doc.apply(set_text(&doc, "dropped")).unwrap();
    drop(doc);
    assert_eq!(text_in_file(&file), "dropped");
    assert_eq!(text_in_file(&two.join(name)), "what to buy");
}
