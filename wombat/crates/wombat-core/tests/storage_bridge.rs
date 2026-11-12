//! Files, after the review of the bridge (milestone B1): a copy of a document that fails
//! half-way leaves nothing behind.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use wombat_core::construct::Construct;
use wombat_core::storage::{self, VIEW_DIR};
use wombat_core::{Document, ImageRef, SeededIds, ViewState};

fn folder(test: &str) -> PathBuf {
    // A short name: some tests need room for long file names below it.
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("storage_bridge")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A document of one column with one cell per name in `images`, each showing that image.
fn sample(seed: u64, images: &[&str]) -> Document {
    let mut ids = SeededIds::from_u64(seed);
    let mut doc = Document::new(&mut ids);
    let rows = u32::try_from(images.len().max(1)).unwrap();
    doc.add_grid(doc.root(), 1, rows, &mut ids).unwrap();
    for (y, image) in (0..).zip(images) {
        let cell = doc.child(doc.root(), 0, y).unwrap();
        doc.cell_mut(cell).unwrap().image = Some(ImageRef::new(*image));
    }
    doc
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// Saves `doc` as `from` with a file for each of `images` and a view state.
fn save_with_all(doc: &Document, from: &Path, images: &[&str]) {
    storage::save_document(doc, from).unwrap();
    for image in images {
        storage::write_image(from, image, b"picture").unwrap();
    }
    storage::save_view_state(from.parent().unwrap(), &ViewState::new(doc.id())).unwrap();
}

/// Makes saving a document as `to` fail, as root too: the name of the temporary file is
/// taken by a folder, which is not removed. Returns that folder's name.
fn block_saving(to: &Path) -> String {
    let name = format!(".{}.tmp", to.file_name().unwrap().to_str().unwrap());
    fs::create_dir(to.with_file_name(&name)).unwrap();
    name
}

#[test]
fn a_copy_whose_document_cannot_be_saved_leaves_nothing_behind() {
    let dir = folder("save_fails");
    let (here, there) = (dir.join("here"), dir.join("there"));
    fs::create_dir(&here).unwrap();
    fs::create_dir(&there).unwrap();
    let from = here.join("a.wombat");
    let doc = sample(1, &["pic.png"]);
    save_with_all(&doc, &from, &["pic.png"]);
    let to = there.join("b.wombat");
    let blocker = block_saving(&to);

    let result = storage::copy_document(&from, &to, &mut SeededIds::from_u64(2));
    assert!(result.is_err(), "{result:?}");
    // Not the image folder with the picture, not the folder of view states with one for
    // a document that is not there.
    assert_eq!(names(&there), [blocker]);
    assert_eq!(storage::load_document(&from).unwrap(), doc);
    assert_eq!(names(&storage::images_dir(&from)), ["pic.png"]);
}

#[test]
fn a_failed_copy_removes_only_what_it_made() {
    let dir = folder("keeps_others");
    let from = dir.join("a.wombat");
    let doc = sample(1, &["pic.png"]);
    save_with_all(&doc, &from, &["pic.png"]);
    let to = dir.join("b.wombat");
    // The image folder of the copy is there already, with a file in it, and so is the
    // folder of view states, with that of the original.
    fs::create_dir(storage::images_dir(&to)).unwrap();
    fs::write(storage::images_dir(&to).join("other.png"), b"kept").unwrap();
    let views = names(&dir.join(VIEW_DIR));
    assert_eq!(views.len(), 1);
    let blocker = block_saving(&to);

    let result = storage::copy_document(&from, &to, &mut SeededIds::from_u64(2));
    assert!(result.is_err(), "{result:?}");
    assert_eq!(names(&storage::images_dir(&to)), ["other.png"]);
    assert_eq!(
        fs::read(storage::images_dir(&to).join("other.png")).unwrap(),
        b"kept"
    );
    assert_eq!(names(&dir.join(VIEW_DIR)), views);
    let mut all = vec![
        VIEW_DIR.to_string(),
        blocker,
        "a.wombat".to_string(),
        "a.wombat.images".to_string(),
        "b.wombat.images".to_string(),
    ];
    all.sort();
    assert_eq!(names(&dir), all);
}

/// The second image cannot be copied (its name is as long as a name may be, so the name
/// of the temporary file beside it is too long): the first one does not stay.
#[cfg(unix)]
#[test]
fn a_copy_whose_images_cannot_all_be_copied_leaves_nothing_behind() {
    let dir = folder("image_fails");
    let from = dir.join("a.wombat");
    let long = format!("{}.png", "z".repeat(251));
    let doc = sample(1, &["pic.png", &long]);
    storage::save_document(&doc, &from).unwrap();
    fs::create_dir(storage::images_dir(&from)).unwrap();
    for image in ["pic.png", &long] {
        fs::write(storage::images_dir(&from).join(image), b"picture").unwrap();
    }
    let to = dir.join("b.wombat");

    let result = storage::copy_document(&from, &to, &mut SeededIds::from_u64(2));
    assert!(result.is_err(), "{result:?}");
    assert_eq!(names(&dir), ["a.wombat", "a.wombat.images"]);
}
