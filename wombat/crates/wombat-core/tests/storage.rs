//! Files on disk: atomic save, images, view state and workspaces.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use wombat_core::storage::{self, VIEW_DIR};
use wombat_core::{
    CellId, DocId, Document, Error, IdSource, ImageRef, SeededIds, Selection, ViewState, Workspace,
};

/// A fresh, empty folder for one test.
fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("storage")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn sample(seed: u64) -> (Document, SeededIds) {
    let mut ids = SeededIds::from_u64(seed);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), 2, 1, &mut ids).unwrap();
    let first = doc.child(doc.root(), 0, 0).unwrap();
    doc.cell_mut(first).unwrap().text = format!("document {seed}");
    (doc, ids)
}

#[test]
fn save_then_load_gives_the_same_document_and_leaves_no_temporary_file() {
    let dir = folder("save_load");
    let path = dir.join("a.wombat");
    let (doc, _) = sample(1);
    storage::save_document(&doc, &path).unwrap();
    assert_eq!(storage::load_document(&path).unwrap(), doc);
    assert_eq!(names(&dir), ["a.wombat"]);
    let first = fs::read(&path).unwrap();
    storage::save_document(&doc, &path).unwrap();
    assert_eq!(
        fs::read(&path).unwrap(),
        first,
        "saving twice gives identical bytes"
    );
    assert_eq!(names(&dir), ["a.wombat"]);
}

#[test]
fn a_failed_save_leaves_the_old_file_untouched() {
    let dir = folder("failed_save");
    let path = dir.join("a.wombat");
    let (doc, _) = sample(1);
    storage::save_document(&doc, &path).unwrap();
    let before = fs::read(&path).unwrap();

    // An invalid document is refused before anything is written.
    let mut bad = doc.clone();
    let root = bad.root();
    bad.cell_mut(root).unwrap().image = Some(ImageRef::new("../escape.png"));
    assert!(matches!(
        storage::save_document(&bad, &path),
        Err(Error::Invalid(_))
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(names(&dir), ["a.wombat"]);

    // The final rename fails (the target is a folder): an error, and the temporary file is gone.
    let blocked = dir.join("folder.wombat");
    fs::create_dir(&blocked).unwrap();
    fs::write(blocked.join("keep"), b"x").unwrap();
    assert!(matches!(
        storage::save_document(&doc, &blocked),
        Err(Error::Io { .. })
    ));
    assert_eq!(names(&dir), ["a.wombat", "folder.wombat"]);
    assert_eq!(names(&blocked), ["keep"]);

    // The folder does not exist: an error.
    let nowhere = dir.join("missing").join("a.wombat");
    assert!(matches!(
        storage::save_document(&doc, &nowhere),
        Err(Error::Io { .. })
    ));
}

#[test]
fn a_half_written_temporary_file_from_a_crash_is_harmless() {
    let dir = folder("crash");
    let path = dir.join("a.wombat");
    let (doc, _) = sample(1);
    storage::save_document(&doc, &path).unwrap();
    // What a crash in the middle of a save leaves behind: the old file and part of the new.
    fs::write(dir.join(".a.wombat.tmp"), b"{\n\"format\": \"wom").unwrap();
    assert_eq!(storage::load_document(&path).unwrap(), doc);
    let listed = Workspace::open(&dir).unwrap().list().unwrap();
    assert_eq!(listed.len(), 1);
    // The next save replaces the leftover and removes it.
    let (other, _) = sample(2);
    storage::save_document(&other, &path).unwrap();
    assert_eq!(storage::load_document(&path).unwrap(), other);
    assert_eq!(names(&dir), ["a.wombat"]);
}

#[test]
fn write_atomic_replaces_whole_content() {
    let dir = folder("write_atomic");
    let path = dir.join("f");
    storage::write_atomic(&path, b"a long first version").unwrap();
    storage::write_atomic(&path, b"short").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"short");
    assert!(storage::write_atomic(Path::new("/"), b"x").is_err());
}

#[test]
fn loading_bad_files_is_an_error() {
    let dir = folder("bad_files");
    assert!(matches!(
        storage::load_document(&dir.join("none.wombat")),
        Err(Error::Io { .. })
    ));
    assert!(matches!(
        storage::load_document(&dir),
        Err(Error::Io { .. })
    ));
    fs::write(dir.join("empty.wombat"), b"").unwrap();
    assert!(matches!(
        storage::load_document(&dir.join("empty.wombat")),
        Err(Error::Syntax(_))
    ));
    fs::write(dir.join("cts.wombat"), b"TSFF\x1f\x00\x01\x01\x00\x00D").unwrap();
    assert!(matches!(
        storage::load_document(&dir.join("cts.wombat")),
        Err(Error::Syntax(_))
    ));
}

#[test]
fn a_file_over_the_size_limit_is_refused() {
    let dir = folder("too_large");
    let path = dir.join("big.wombat");
    // A sparse file: it takes no disk space, and reading stops one byte past the limit.
    let file = fs::File::create(&path).unwrap();
    file.set_len(wombat_core::limits::MAX_FILE_BYTES + 1000)
        .unwrap();
    drop(file);
    assert!(matches!(
        storage::load_document(&path),
        Err(Error::Limit(_))
    ));
}

#[test]
fn images_are_copied_beside_the_document_and_referenced_by_name() {
    let dir = folder("images");
    let path = dir.join("a.wombat");
    let (mut doc, mut ids) = sample(1);
    let source = dir.join("My Photo.PNG");
    fs::write(&source, b"not really a png").unwrap();

    let image = storage::add_image(&path, &source, &mut ids).unwrap();
    assert!(
        image.file.ends_with(".png") && image.file.len() == 40,
        "{}",
        image.file
    );
    assert_eq!(storage::images_dir(&path), dir.join("a.wombat.images"));
    let stored = storage::image_path(&path, &image.file).unwrap();
    assert_eq!(fs::read(&stored).unwrap(), b"not really a png");

    let root = doc.root();
    doc.cell_mut(root).unwrap().image = Some(image.clone());
    let first = doc.child(root, 0, 0).unwrap();
    doc.cell_mut(first).unwrap().image = Some(ImageRef {
        scale: 2.0,
        ..image.clone()
    });
    let second = doc.child(root, 1, 0).unwrap();
    doc.cell_mut(second).unwrap().image = Some(ImageRef::new("lost.jpg"));
    storage::save_document(&doc, &path).unwrap();
    assert_eq!(storage::load_document(&path).unwrap(), doc);
    assert_eq!(
        storage::image_names(&doc),
        [image.file.clone(), "lost.jpg".to_string()]
    );
    assert_eq!(storage::missing_images(&doc, &path), ["lost.jpg"]);

    // "Save as" elsewhere takes the images that exist along.
    let other = dir.join("sub");
    fs::create_dir(&other).unwrap();
    let copy = other.join("b.wombat");
    assert_eq!(storage::copy_images(&doc, &path, &copy).unwrap(), 1);
    assert_eq!(
        fs::read(storage::image_path(&copy, &image.file).unwrap())
            .unwrap()
            .len(),
        16
    );
    assert_eq!(storage::copy_images(&doc, &path, &path).unwrap(), 0);

    // A name cannot lead out of the image folder, and odd extensions are dropped.
    assert!(storage::image_path(&path, "../a.wombat").is_err());
    assert!(storage::image_path(&path, "a/b.png").is_err());
    let odd = dir.join("archive.tar gz");
    fs::write(&odd, b"x").unwrap();
    assert_eq!(
        storage::add_image(&path, &odd, &mut ids)
            .unwrap()
            .file
            .len(),
        36
    );
    assert!(storage::add_image(&path, &dir.join("absent.png"), &mut ids).is_err());
}

#[test]
fn view_state_is_saved_apart_from_the_document_keyed_by_id() {
    let dir = folder("view_state");
    let (doc, _) = sample(1);
    let path = dir.join("a.wombat");
    storage::save_document(&doc, &path).unwrap();
    let before = fs::read(&path).unwrap();
    assert_eq!(storage::load_view_state(&dir, doc.id()).unwrap(), None);

    let mut view = ViewState::new(doc.id());
    view.folded.insert(doc.root());
    view.zoom = doc.child(doc.root(), 1, 0);
    view.selection = Some(Selection {
        cell: doc.child(doc.root(), 0, 0).unwrap(),
        xs: 1,
        ys: 1,
        cursor: None,
        extra: Default::default(),
    });
    storage::save_view_state(&dir, &view).unwrap();
    assert_eq!(
        storage::load_view_state(&dir, doc.id()).unwrap(),
        Some(view.clone())
    );
    assert_eq!(
        fs::read(&path).unwrap(),
        before,
        "the document file is not touched"
    );
    assert_eq!(names(&dir.join(VIEW_DIR)), [format!("{}.json", doc.id())]);

    // Renaming the document keeps its view state, because the key is the ID.
    fs::rename(&path, dir.join("renamed.wombat")).unwrap();
    let ws = Workspace::open(&dir).unwrap();
    let renamed = ws.open_name("renamed.wombat").unwrap();
    assert_eq!(ws.load_view(renamed.id()).unwrap(), Some(view.clone()));
    view.folded.clear();
    ws.save_view(&view).unwrap();
    assert_eq!(ws.load_view(doc.id()).unwrap(), Some(view.clone()));

    // A view-state file under the wrong ID, or a damaged one, is an error and not a crash.
    let (other, _) = sample(2);
    fs::copy(
        storage::view_state_path(&dir, doc.id()),
        storage::view_state_path(&dir, other.id()),
    )
    .unwrap();
    assert!(matches!(
        storage::load_view_state(&dir, other.id()),
        Err(Error::Invalid(_))
    ));
    fs::write(storage::view_state_path(&dir, doc.id()), b"{\"format\"").unwrap();
    assert!(matches!(
        storage::load_view_state(&dir, doc.id()),
        Err(Error::Syntax(_))
    ));
}

#[test]
fn workspace_creates_lists_opens_and_saves() {
    let dir = folder("workspace");
    let ws = Workspace::open(&dir).unwrap();
    assert_eq!(ws.dir(), dir);
    assert!(ws.list().unwrap().is_empty());
    let mut ids = SeededIds::from_u64(5);

    let b = ws.create("b.wombat", &mut ids).unwrap();
    let mut a = ws.create("a.wombat", &mut ids).unwrap();
    assert!(matches!(
        ws.create("a.wombat", &mut ids),
        Err(Error::AlreadyExists(_))
    ));
    assert_ne!(a.id(), b.id());

    // Things that are not documents are ignored; a damaged document is listed with its error.
    fs::write(dir.join("notes.txt"), b"hello").unwrap();
    fs::write(dir.join(".hidden.wombat"), b"hello").unwrap();
    fs::create_dir(dir.join("folder.wombat")).unwrap();
    fs::write(dir.join("c.wombat"), b"{ damaged").unwrap();
    let listed = ws.list().unwrap();
    let listed_names: Vec<&str> = listed.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(listed_names, ["a.wombat", "b.wombat", "c.wombat"]);
    assert_eq!(listed[0].id.as_ref().unwrap(), &a.id());
    assert_eq!(listed[1].id.as_ref().unwrap(), &b.id());
    assert!(matches!(listed[2].id, Err(Error::Syntax(_))));

    a.add_grid(a.root(), 1, 2, &mut ids).unwrap();
    let root = a.root();
    a.cell_mut(root).unwrap().text = "changed".into();
    ws.save(&a, "a.wombat").unwrap();
    assert_eq!(ws.open_name("a.wombat").unwrap(), a);
    let (name, found) = ws.open_id(a.id()).unwrap();
    assert_eq!((name.as_str(), &found), ("a.wombat", &a));
    assert_eq!(ws.open_id(b.id()).unwrap().0, "b.wombat");
    let unknown = DocId(ids.next_id());
    assert!(matches!(ws.open_id(unknown), Err(Error::NotFound(_))));
    assert!(matches!(ws.open_name("zzz.wombat"), Err(Error::Io { .. })));
    let _ = CellId(ids.next_id());
}

#[test]
fn workspace_names_cannot_leave_the_folder() {
    let dir = folder("workspace_names");
    let ws = Workspace::open(&dir).unwrap();
    let mut ids = SeededIds::from_u64(5);
    for bad in [
        "",
        "a",
        "a.txt",
        "../a.wombat",
        "sub/a.wombat",
        "sub\\a.wombat",
        "/tmp/a.wombat",
        ".wombat",
        "..",
        ".a.wombat",
    ] {
        assert!(
            matches!(ws.create(bad, &mut ids), Err(Error::Invalid(_))),
            "{bad:?}"
        );
        assert!(ws.open_name(bad).is_err(), "{bad:?}");
    }
    assert!(names(&dir).is_empty());
    assert!(matches!(
        Workspace::open(dir.join("absent")),
        Err(Error::NotFound(_))
    ));
    fs::write(dir.join("file"), b"").unwrap();
    assert!(Workspace::open(dir.join("file")).is_err());
}
