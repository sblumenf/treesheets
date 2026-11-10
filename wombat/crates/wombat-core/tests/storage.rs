//! Files on disk: atomic save, images, view state and workspaces.
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
use wombat_core::{
    CellId, DocId, Document, Error, IdCollision, IdSource, ImageRef, Link, LinkTarget, SeededIds,
    Selection, ViewState, Workspace,
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

#[cfg(unix)]
#[test]
fn saving_never_writes_through_a_link_at_the_temporary_name() {
    let dir = folder("temp_symlink");
    let victim = dir.join("victim.txt");
    fs::write(&victim, b"precious").unwrap();
    let temp = dir.join(".a.wombat.tmp");
    std::os::unix::fs::symlink(&victim, &temp).unwrap();
    let (doc, _) = sample(1);
    let path = dir.join("a.wombat");
    storage::save_document(&doc, &path).unwrap();
    assert_eq!(fs::read(&victim).unwrap(), b"precious");
    assert_eq!(storage::load_document(&path).unwrap(), doc);
    assert!(!fs::symlink_metadata(&path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(names(&dir), ["a.wombat", "victim.txt"]);

    // A link that points nowhere, and a folder in the way, do no harm either.
    std::os::unix::fs::symlink(dir.join("absent"), &temp).unwrap();
    storage::save_document(&doc, &path).unwrap();
    assert_eq!(names(&dir), ["a.wombat", "victim.txt"]);
    fs::create_dir(&temp).unwrap();
    fs::write(temp.join("keep"), b"").unwrap();
    let before = fs::read(&path).unwrap();
    assert!(matches!(
        storage::save_document(&sample(2).0, &path),
        Err(Error::Io { .. })
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(temp.join("keep").exists());
}

#[cfg(unix)]
#[test]
fn saving_keeps_the_permissions_of_the_file() {
    use std::os::unix::fs::PermissionsExt;
    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o7777;
    let dir = folder("permissions");
    let path = dir.join("a.wombat");
    let (doc, _) = sample(1);
    storage::save_document(&doc, &path).unwrap();
    for wanted in [0o600, 0o640, 0o664] {
        fs::set_permissions(&path, fs::Permissions::from_mode(wanted)).unwrap();
        storage::save_document(&sample(u64::from(wanted)).0, &path).unwrap();
        assert_eq!(mode(&path), wanted, "{wanted:o}");
    }
}

#[cfg(unix)]
#[test]
fn saving_through_a_link_changes_the_real_file_and_keeps_the_link() {
    let dir = folder("doc_symlink");
    fs::create_dir(dir.join("real")).unwrap();
    let real = dir.join("real").join("real.wombat");
    let link = dir.join("link.wombat");
    let (doc, _) = sample(1);
    storage::save_document(&doc, &real).unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let (changed, _) = sample(2);
    storage::save_document(&changed, &link).unwrap();
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(!fs::symlink_metadata(&real)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(storage::load_document(&real).unwrap(), changed);
    assert_eq!(storage::load_document(&link).unwrap(), changed);
    // The temporary file was beside the real file and is gone.
    assert_eq!(names(&dir), ["link.wombat", "real"]);
    assert_eq!(names(&dir.join("real")), ["real.wombat"]);

    // A link to nothing is not followed and not replaced: the save is refused.
    let dangling = dir.join("dangling.wombat");
    std::os::unix::fs::symlink(dir.join("real").join("absent.wombat"), &dangling).unwrap();
    assert!(matches!(
        storage::save_document(&changed, &dangling),
        Err(Error::Io { .. })
    ));
    assert!(fs::symlink_metadata(&dangling)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(names(&dir.join("real")), ["real.wombat"]);
}

#[test]
fn the_id_is_read_from_the_header_without_the_rest_of_the_file() {
    let dir = folder("header");
    let ws = Workspace::open(&dir).unwrap();
    let (doc, _) = sample(1);
    let text = wombat_core::format::to_string(&doc).unwrap();
    let id = doc.id();
    let header =
        format!("{{\"format\": \"wombat\", \"version\": 1, \"minor\": 0, \"id\": \"{id}\",");
    assert!(text.replace('\n', "").starts_with(&header));
    let root = text.split_once("\"root\": ").unwrap().1;
    let root = root.strip_suffix("\n}\n").unwrap();
    let files = [
        ("a-normal.wombat", text.clone()),
        // What follows the header is not read: not even whether it is JSON.
        ("b-cut.wombat", format!("{header} \"root\": {{\"id\": nonsense")),
        // The header fields may be anywhere and in any order; then all of the file is read.
        (
            "c-late.wombat",
            format!("{{\"root\": {root}, \"zz\": [1, {{\"id\": 2}}], \"id\": \"{id}\", \"version\": 1, \"format\": \"wombat\"}}"),
        ),
        ("d-late-cut.wombat", format!("{{\"root\": {root}, \"id\": \"{id}\", \"version\": 1")),
        // A newer or foreign file is refused from its first fields, whatever follows.
        ("e-newer.wombat", "{\"format\": \"wombat\", \"version\": 2, \x00 not JSON at all".into()),
        ("f-view.wombat", format!("{{\"format\": \"wombat-view\", \"version\": 1, \"doc\": \"{id}\"}}")),
        ("g-no-id.wombat", format!("{{\"format\": \"wombat\", \"version\": 1, \"root\": {root}}}")),
        ("h-bad-id.wombat", "{\"format\": \"wombat\", \"version\": 1, \"id\": \"1234\"}".into()),
        ("i-twice.wombat", format!("{{\"format\": \"wombat\", \"format\": \"wombat\", \"version\": 1, \"id\": \"{id}\"}}")),
        ("j-array.wombat", "[1, 2]".into()),
        ("k-zero.wombat", format!("{{\"version\": 0, \"format\": \"wombat\", \"id\": \"{id}\"}}")),
        ("l-no-version.wombat", format!("{{\"format\": \"wombat\", \"id\": \"{id}\", \"root\": {root}}}")),
        ("m-empty.wombat", String::new()),
    ];
    for (name, content) in &files {
        fs::write(dir.join(name), content).unwrap();
    }
    let listed = ws.list().unwrap();
    let outcome = |name: &str| {
        let entry = listed.iter().find(|e| e.name == name).unwrap();
        let direct = storage::document_id(&dir.join(name));
        assert_eq!(format!("{:?}", entry.id), format!("{direct:?}"));
        direct
    };
    assert_eq!(listed.len(), files.len());
    for name in ["a-normal.wombat", "b-cut.wombat", "c-late.wombat"] {
        assert_eq!(outcome(name).unwrap(), id, "{name}");
    }
    assert!(matches!(
        outcome("d-late-cut.wombat"),
        Err(Error::Syntax(_))
    ));
    assert!(matches!(
        outcome("e-newer.wombat"),
        Err(Error::NewerVersion {
            found: 2,
            supported: 1
        })
    ));
    assert!(matches!(
        outcome("f-view.wombat"),
        Err(Error::WrongFormat(_))
    ));
    assert!(matches!(outcome("g-no-id.wombat"), Err(Error::Invalid(_))));
    assert!(matches!(outcome("h-bad-id.wombat"), Err(Error::Invalid(_))));
    assert!(matches!(outcome("i-twice.wombat"), Err(Error::Invalid(_))));
    assert!(matches!(outcome("j-array.wombat"), Err(Error::Invalid(_))));
    assert!(matches!(outcome("k-zero.wombat"), Err(Error::Invalid(_))));
    assert!(matches!(
        outcome("l-no-version.wombat"),
        Err(Error::WrongFormat(_))
    ));
    assert!(matches!(outcome("m-empty.wombat"), Err(Error::Syntax(_))));

    // Loading agrees wherever the whole file is sound, and refuses the newer file the same
    // way, without trying to read the rest of it.
    assert_eq!(ws.open_name("a-normal.wombat").unwrap(), doc);
    assert_eq!(ws.open_name("c-late.wombat").unwrap().id(), id);
    assert!(matches!(
        ws.open_name("b-cut.wombat"),
        Err(Error::Syntax(_))
    ));
    assert!(matches!(
        ws.open_name("e-newer.wombat"),
        Err(Error::NewerVersion {
            found: 2,
            supported: 1
        })
    ));
    assert!(matches!(
        ws.open_name("f-view.wombat"),
        Err(Error::WrongFormat(_))
    ));
    let view_text = b"{\"format\": \"wombat-view\", \"version\": 3, \x00";
    assert!(matches!(
        ViewState::from_json(view_text),
        Err(Error::NewerVersion {
            found: 3,
            supported: 1
        })
    ));

    // A file over the size limit is refused by its length alone.
    let big = fs::File::create(dir.join("big.wombat")).unwrap();
    big.set_len(wombat_core::limits::MAX_FILE_BYTES + 1)
        .unwrap();
    assert!(matches!(
        storage::document_id(&dir.join("big.wombat")),
        Err(Error::Limit(_))
    ));
    assert!(matches!(
        storage::document_id(&dir.join("absent.wombat")),
        Err(Error::Io { .. })
    ));
}

/// A document with an image and with links of every kind, and its view state.
fn linked(dir: &Path, name: &str) -> (Document, ViewState, SeededIds) {
    let path = dir.join(name);
    let (mut doc, mut ids) = sample(11);
    let root = doc.root();
    let first = doc.child(root, 0, 0).unwrap();
    let second = doc.child(root, 1, 0).unwrap();
    let source = dir.join("photo.png");
    fs::write(&source, b"picture").unwrap();
    doc.cell_mut(second).unwrap().image =
        Some(storage::add_image(&path, &source, &mut ids).unwrap());
    let elsewhere = DocId(ids.next_id());
    let targets = [
        LinkTarget::Cell {
            doc: None,
            cell: second,
        },
        LinkTarget::Cell {
            doc: Some(elsewhere),
            cell: second,
        },
        LinkTarget::External("photo.png".into()),
    ];
    doc.cell_mut(first).unwrap().links = targets
        .into_iter()
        .zip(0..)
        .map(|(target, start)| Link {
            start,
            len: 1,
            target,
            extra: Default::default(),
        })
        .collect();
    storage::save_document(&doc, &path).unwrap();
    let mut view = ViewState::new(doc.id());
    view.folded.insert(root);
    view.zoom = Some(first);
    view.selection = Some(Selection {
        cell: second,
        xs: 1,
        ys: 1,
        cursor: Some((0, 0)),
        extra: Default::default(),
    });
    storage::save_view_state(dir, &view).unwrap();
    (doc, view, ids)
}

#[test]
fn duplicating_a_document_gives_it_ids_of_its_own() {
    let dir = folder("duplicate");
    let ws = Workspace::open(&dir).unwrap();
    let (doc, view, mut ids) = linked(&dir, "a.wombat");
    let before = fs::read(dir.join("a.wombat")).unwrap();
    let (first, second) = (
        doc.child(doc.root(), 0, 0).unwrap(),
        doc.child(doc.root(), 1, 0).unwrap(),
    );

    let copy = ws.duplicate("a.wombat", "b.wombat", &mut ids).unwrap();
    assert_eq!(ws.open_name("b.wombat").unwrap(), copy);
    assert_eq!(fs::read(dir.join("a.wombat")).unwrap(), before);
    assert_ne!(copy.id(), doc.id());
    assert_eq!(copy.cell_count(), doc.cell_count());
    for id in copy.cells_in_order() {
        assert!(doc.cell(id).is_none());
    }
    let (new_first, new_second) = (
        copy.child(copy.root(), 0, 0).unwrap(),
        copy.child(copy.root(), 1, 0).unwrap(),
    );
    assert_eq!(copy.cell(new_first).unwrap().text, "document 11");
    // The link inside the document follows the copy; the one into another document stays.
    let targets: Vec<&LinkTarget> = copy
        .cell(new_first)
        .unwrap()
        .links
        .iter()
        .map(|l| &l.target)
        .collect();
    assert_eq!(
        targets[0],
        &LinkTarget::Cell {
            doc: None,
            cell: new_second
        }
    );
    assert_eq!(targets[1], &doc.cell(first).unwrap().links[1].target);
    assert_eq!(targets[2], &LinkTarget::External("photo.png".into()));
    // The image went along, and so did the view state, for the new IDs.
    assert!(storage::missing_images(&copy, &dir.join("b.wombat")).is_empty());
    let image = &copy.cell(new_second).unwrap().image.as_ref().unwrap().file;
    assert_eq!(
        fs::read(dir.join("b.wombat.images").join(image)).unwrap(),
        b"picture"
    );
    let new_view = ws.load_view(copy.id()).unwrap().unwrap();
    assert_eq!(new_view.folded, [copy.root()].into_iter().collect());
    assert_eq!(new_view.zoom, Some(new_first));
    let selection = new_view.selection.unwrap();
    assert_eq!(
        (selection.cell, selection.cursor),
        (new_second, Some((0, 0)))
    );
    assert_eq!(ws.load_view(doc.id()).unwrap(), Some(view));
    let _ = second;

    // Nothing is replaced, and names stay inside the workspace.
    assert!(matches!(
        ws.duplicate("a.wombat", "b.wombat", &mut ids),
        Err(Error::AlreadyExists(_))
    ));
    assert!(matches!(
        ws.duplicate("a.wombat", "a.wombat", &mut ids),
        Err(Error::AlreadyExists(_))
    ));
    assert!(ws.duplicate("a.wombat", "../c.wombat", &mut ids).is_err());
    assert!(ws.duplicate("absent.wombat", "c.wombat", &mut ids).is_err());
    assert!(!dir.join("c.wombat").exists());

    // A proper copy collides with nothing.
    assert!(ws.list().unwrap().iter().all(|e| e.same_id.is_empty()));
    assert_eq!(ws.id_collisions().unwrap(), []);

    // Without view state or images there is nothing else to copy; other folders work too.
    fs::create_dir(dir.join("sub")).unwrap();
    let (plain, mut plain_ids) = sample(3);
    storage::save_document(&plain, &dir.join("sub").join("p.wombat")).unwrap();
    let to = dir.join("sub").join("deeper.wombat");
    let copied =
        storage::copy_document(&dir.join("sub").join("p.wombat"), &to, &mut plain_ids).unwrap();
    assert_ne!(copied.id(), plain.id());
    assert_eq!(names(&dir.join("sub")), ["deeper.wombat", "p.wombat"]);
}

#[test]
fn files_that_share_ids_are_reported() {
    let dir = folder("collisions");
    let ws = Workspace::open(&dir).unwrap();
    let (doc, _, _) = linked(&dir, "b.wombat");
    // What a file manager's "copy" does: the same document twice, with all its IDs.
    fs::copy(dir.join("b.wombat"), dir.join("a-copy.wombat")).unwrap();
    fs::copy(dir.join("b.wombat"), dir.join("c-copy.wombat")).unwrap();
    // A document of its own that has one cell with the ID of a cell of the others.
    let (mut other, _) = sample(12);
    let text = wombat_core::format::to_string(&other).unwrap();
    let stolen = doc.child(doc.root(), 0, 0).unwrap();
    let own = other.child(other.root(), 1, 0).unwrap();
    fs::write(
        dir.join("d.wombat"),
        text.replace(&own.to_string(), &stolen.to_string()),
    )
    .unwrap();
    // A document whose ID is the ID of a cell of another document.
    let mut skewed = SeededIds::from_u64(13);
    let unrelated = ws.create("e.wombat", &mut skewed).unwrap();
    let mut again = SeededIds::from_u64(13);
    again.next_id();
    let clash = ws.create("f.wombat", &mut again).unwrap();
    assert_eq!(clash.id().0, unrelated.root().0);
    fs::write(dir.join("g.wombat"), b"damaged").unwrap();
    other.cell_mut(own).unwrap().text = "unused".into();

    let listed = ws.list().unwrap();
    let same: Vec<(&str, Vec<&str>)> = listed
        .iter()
        .map(|e| {
            (
                e.name.as_str(),
                e.same_id.iter().map(String::as_str).collect(),
            )
        })
        .collect();
    assert_eq!(
        same,
        [
            ("a-copy.wombat", vec!["b.wombat", "c-copy.wombat"]),
            ("b.wombat", vec!["a-copy.wombat", "c-copy.wombat"]),
            ("c-copy.wombat", vec!["a-copy.wombat", "b.wombat"]),
            ("d.wombat", vec![]),
            ("e.wombat", vec![]),
            ("f.wombat", vec![]),
            ("g.wombat", vec![]),
        ]
    );
    let all = doc.cell_count() + 1;
    let pair = |first: &str, second: &str, same_document, shared_ids| IdCollision {
        first: first.into(),
        second: second.into(),
        same_document,
        shared_ids,
    };
    assert_eq!(
        ws.id_collisions().unwrap(),
        [
            pair("a-copy.wombat", "b.wombat", true, all),
            pair("a-copy.wombat", "c-copy.wombat", true, all),
            pair("a-copy.wombat", "d.wombat", false, 1),
            pair("e.wombat", "f.wombat", false, 1),
        ]
    );
}
