//! Files, after the review of the bridge (milestone B1): a copy of a document that fails
//! half-way leaves nothing behind, and a program that has a document open can tell whether
//! its file is still the one it read or wrote.
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

/// What a failed copy wrote into, it leaves: a folder that was there before stays though
/// the copy leaves it empty, and a picture that was there under the name of one that the
/// copy brings stays as a file. (Its content is the new picture's by then: the copy has
/// no old one to put back, which `copy_document` says.)
#[test]
fn a_failed_copy_leaves_the_folders_and_files_that_were_there_before_it() {
    let dir = folder("keeps_same");
    let here = dir.join("here");
    fs::create_dir(&here).unwrap();
    let from = here.join("a.wombat");
    let doc = sample(1, &["pic.png"]);
    save_with_all(&doc, &from, &["pic.png"]);

    // An image folder and a folder of view states, both empty.
    let empty = dir.join("empty");
    fs::create_dir(&empty).unwrap();
    let to = empty.join("b.wombat");
    fs::create_dir(storage::images_dir(&to)).unwrap();
    fs::create_dir(empty.join(VIEW_DIR)).unwrap();
    let blocker = block_saving(&to);
    let result = storage::copy_document(&from, &to, &mut SeededIds::from_u64(2));
    assert!(result.is_err(), "{result:?}");
    let mut all = vec![VIEW_DIR.to_string(), blocker, "b.wombat.images".to_string()];
    all.sort();
    assert_eq!(names(&empty), all);
    assert!(names(&storage::images_dir(&to)).is_empty());
    assert!(names(&empty.join(VIEW_DIR)).is_empty());

    // An image folder with a picture of the same name as the one that is copied.
    let same = dir.join("same");
    fs::create_dir(&same).unwrap();
    let to = same.join("b.wombat");
    fs::create_dir(storage::images_dir(&to)).unwrap();
    let picture = storage::images_dir(&to).join("pic.png");
    fs::write(&picture, b"an older picture").unwrap();
    let blocker = block_saving(&to);
    let result = storage::copy_document(&from, &to, &mut SeededIds::from_u64(2));
    assert!(result.is_err(), "{result:?}");
    assert_eq!(names(&same), [blocker, "b.wombat.images".to_string()]);
    assert_eq!(names(&storage::images_dir(&to)), ["pic.png"]);
    assert_eq!(fs::read(&picture).unwrap(), b"picture");
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

/// What a file looked like when it was read or written, and whether it still does: how an
/// open document knows that another program changed its file.
mod file_state {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};
    use std::time::{Duration, SystemTime};
    use wombat_core::storage::{FileCheck, FileState};

    /// Only a path with nothing at it is a file that is gone, which a program may write
    /// again. A path that cannot be looked at is an error: nothing is known about it.
    #[cfg(unix)]
    #[test]
    fn a_path_that_cannot_be_looked_at_is_an_error_and_not_a_file_that_is_gone() {
        let dir = folder("no_look");
        let path = dir.join("a.wombat");
        let (_, state) = saved(&path);
        let io = |at: &Path| match state.check(at) {
            Err(wombat_core::Error::Io { path, .. }) => assert_eq!(path, at),
            other => panic!("{other:?}"),
        };
        // Below a file there is no place for anything.
        io(&path.join("b.wombat"));
        // A name longer than a name may be.
        io(&dir.join("n".repeat(300)));
        // In a folder that is not there, there is nothing: gone.
        let nowhere = dir.join("none/a.wombat");
        assert_eq!(state.check(&nowhere).unwrap(), FileCheck::Gone);
    }

    /// Saves a document as `path` and returns it with the state of the file.
    fn saved(path: &Path) -> (Document, FileState) {
        let doc = sample(1, &[]);
        let state = storage::save_document_with_state(&doc, path).unwrap();
        (doc, state)
    }

    fn modified(path: &Path) -> SystemTime {
        fs::metadata(path).unwrap().modified().unwrap()
    }

    /// Other bytes of the same number as those in `path`.
    fn other_bytes(path: &Path) -> Vec<u8> {
        let mut bytes = fs::read(path).unwrap();
        let last = bytes.last_mut().unwrap();
        *last = if *last == b' ' { b'\n' } else { b' ' };
        bytes
    }

    /// Writes `bytes` over the start of the file without replacing it, as a program that
    /// does not save atomically does, and then sets the time of modification.
    fn rewrite_in_place(path: &Path, bytes: &[u8], time: SystemTime) {
        let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        file.write_all(bytes).unwrap();
        file.set_modified(time).unwrap();
    }

    #[test]
    fn a_file_that_nobody_touched_is_as_remembered() {
        let dir = folder("unchanged");
        let path = dir.join("a.wombat");
        let (doc, written) = saved(&path);
        assert_eq!(written.check(&path).unwrap(), FileCheck::Unchanged);
        // Reading it gives the same document and the same state as writing it did.
        let (loaded, read) = storage::load_document_with_state(&path).unwrap();
        assert_eq!(loaded, doc);
        assert_eq!(read, written);
        assert_eq!(read.check(&path).unwrap(), FileCheck::Unchanged);
        // Reading changes nothing.
        assert_eq!(storage::load_document(&path).unwrap(), doc);
        assert_eq!(read.check(&path).unwrap(), FileCheck::Unchanged);
    }

    #[test]
    fn a_deleted_file_is_gone() {
        let dir = folder("gone");
        let path = dir.join("a.wombat");
        let (_, state) = saved(&path);
        fs::remove_file(&path).unwrap();
        assert_eq!(state.check(&path).unwrap(), FileCheck::Gone);
        // So is one whose folder went with it.
        fs::remove_dir(&dir).unwrap();
        assert_eq!(state.check(&path).unwrap(), FileCheck::Gone);
    }

    /// Another program saved the file the way this one does: a new file under the old
    /// name. On Unix that is seen whatever the length and the clock say, because it is
    /// another file; elsewhere only the time of modification can tell.
    #[cfg(unix)]
    #[test]
    fn a_file_replaced_by_one_of_the_same_length_was_changed() {
        let dir = folder("replaced");
        let path = dir.join("a.wombat");
        let (_, state) = saved(&path);
        let (other, time) = (other_bytes(&path), modified(&path));
        storage::write_atomic(&path, &other).unwrap();
        // Even with the old time of modification put back.
        let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_modified(time).unwrap();
        drop(file);
        assert_eq!(fs::metadata(&path).unwrap().len(), other.len() as u64);
        assert_eq!(modified(&path), time);
        assert_eq!(state.check(&path).unwrap(), FileCheck::Changed);
    }

    #[test]
    fn a_file_that_was_appended_to_was_changed() {
        let dir = folder("appended");
        let path = dir.join("a.wombat");
        let (_, state) = saved(&path);
        let time = modified(&path);
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"\n").unwrap();
        // The length alone tells, whatever the clock says.
        file.set_modified(time).unwrap();
        drop(file);
        assert_eq!(modified(&path), time);
        assert_eq!(state.check(&path).unwrap(), FileCheck::Changed);
    }

    #[test]
    fn a_file_rewritten_in_place_at_another_time_was_changed() {
        let dir = folder("rewritten");
        let path = dir.join("a.wombat");
        let (_, state) = saved(&path);
        let later = modified(&path) + Duration::from_millis(1);
        rewrite_in_place(&path, &other_bytes(&path), later);
        assert_eq!(state.check(&path).unwrap(), FileCheck::Changed);
    }

    /// The blind spot, written down: the same file (not a new one under its name) with
    /// other bytes of the same number and the same time of modification is taken for
    /// unchanged. That takes a program that rewrites the file in place and either does it
    /// within one tick of the file system's clock after it was read or written here, or
    /// puts the old time back. Seeing it would take reading the whole file at every save.
    #[test]
    fn a_file_rewritten_in_place_with_the_same_length_and_time_is_not_noticed() {
        let dir = folder("blind_spot");
        let path = dir.join("a.wombat");
        let (_, state) = saved(&path);
        let (before, time) = (fs::read(&path).unwrap(), modified(&path));
        rewrite_in_place(&path, &other_bytes(&path), time);
        assert_ne!(fs::read(&path).unwrap(), before);
        assert_eq!(state.check(&path).unwrap(), FileCheck::Unchanged);
    }

    #[test]
    fn saving_again_gives_the_state_to_remember() {
        let dir = folder("save_again");
        let path = dir.join("a.wombat");
        let (_, first) = saved(&path);
        // Another program changes the file; the document is saved over it all the same.
        fs::write(&path, b"something else altogether").unwrap();
        assert_eq!(first.check(&path).unwrap(), FileCheck::Changed);
        let doc = sample(2, &[]);
        let second = storage::save_document_with_state(&doc, &path).unwrap();
        assert_eq!(second.check(&path).unwrap(), FileCheck::Unchanged);
        assert_eq!(storage::load_document(&path).unwrap(), doc);

        // A save that fails leaves the file as it was remembered.
        let blocker = block_saving(&path);
        assert!(storage::save_document_with_state(&sample(3, &[]), &path).is_err());
        fs::remove_dir(dir.join(blocker)).unwrap();
        assert_eq!(second.check(&path).unwrap(), FileCheck::Unchanged);
        assert_eq!(names(&dir), ["a.wombat"]);

        // The file of an earlier save is not the file of a later one, even one with the
        // same bytes: each save puts a new file under the name.
        #[cfg(unix)]
        {
            let third = storage::save_document_with_state(&doc, &path).unwrap();
            assert_eq!(second.check(&path).unwrap(), FileCheck::Changed);
            assert_eq!(third.check(&path).unwrap(), FileCheck::Unchanged);
        }
    }

    #[test]
    fn something_else_at_the_name_was_changed() {
        let dir = folder("not_a_file");
        let path = dir.join("a.wombat");
        let (_, state) = saved(&path);
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(state.check(&path).unwrap(), FileCheck::Changed);
    }

    #[test]
    fn a_copy_comes_with_the_state_of_its_file() {
        let dir = folder("copy_state");
        let (from, to) = (dir.join("a.wombat"), dir.join("b.wombat"));
        let doc = sample(1, &["pic.png"]);
        save_with_all(&doc, &from, &["pic.png"]);
        let mut ids = SeededIds::from_u64(2);
        let (copy, state) = storage::copy_document_with_state(&from, &to, &mut ids).unwrap();
        assert_eq!(storage::load_document(&to).unwrap(), copy);
        assert_ne!(copy.id(), doc.id());
        assert_eq!(state.check(&to).unwrap(), FileCheck::Unchanged);
        assert_eq!(state.check(&from).unwrap(), FileCheck::Changed);
        assert_eq!(names(&storage::images_dir(&to)), ["pic.png"]);
    }

    /// A document opened through a symbolic link: the state is that of the file the link
    /// leads to, as saving replaces that file and keeps the link.
    #[cfg(unix)]
    #[test]
    fn a_link_is_followed_to_the_file_it_leads_to() {
        use std::os::unix::fs::symlink;
        let dir = folder("link");
        let (real, link) = (dir.join("real.wombat"), dir.join("link.wombat"));
        let (doc, written) = saved(&real);
        symlink(&real, &link).unwrap();
        let (loaded, state) = storage::load_document_with_state(&link).unwrap();
        assert_eq!(loaded, doc);
        assert_eq!(state, written);
        assert_eq!(state.check(&link).unwrap(), FileCheck::Unchanged);

        // Another program replaces the file through the link.
        storage::write_atomic(&link, &other_bytes(&real)).unwrap();
        let is_link = |path: &Path| fs::symlink_metadata(path).unwrap().file_type().is_symlink();
        assert!(is_link(&link));
        assert_eq!(state.check(&link).unwrap(), FileCheck::Changed);
        assert_eq!(state.check(&real).unwrap(), FileCheck::Changed);

        // Saving through the link gives the state of the file behind it.
        let state = storage::save_document_with_state(&doc, &link).unwrap();
        assert!(is_link(&link));
        assert_eq!(state.check(&link).unwrap(), FileCheck::Unchanged);
        assert_eq!(state.check(&real).unwrap(), FileCheck::Unchanged);

        // The link is made to lead to another file, even one with the same bytes.
        let elsewhere = dir.join("elsewhere.wombat");
        fs::copy(&real, &elsewhere).unwrap();
        fs::remove_file(&link).unwrap();
        symlink(&elsewhere, &link).unwrap();
        assert_eq!(state.check(&link).unwrap(), FileCheck::Changed);
        assert_eq!(state.check(&real).unwrap(), FileCheck::Unchanged);

        // The file behind the link is deleted: saving there is an error
        // (`write_atomic`), so asking is one too, and not "gone".
        fs::remove_file(&elsewhere).unwrap();
        assert!(state.check(&link).is_err());
    }
}
