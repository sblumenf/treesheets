//! Files, after the review of milestone 4: nothing is written through a link planted at a
//! name that the program itself chooses (decision A87), images go with cells that are
//! copied or moved between documents (A88), and two documents are saved together or not
//! at all (A89).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use wombat_core::construct::Construct;
use wombat_core::storage::{self, SaveStep, VIEW_DIR};
use wombat_core::{CellId, Clip, Document, Error, ImageRef, SeededIds, ViewState};

fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("storage_review")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A document of one column of `rows` cells; the first shows the image `image`, if any.
fn sample(seed: u64, rows: u32, image: Option<&str>) -> Document {
    let mut ids = SeededIds::from_u64(seed);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), 1, rows, &mut ids).unwrap();
    let first = doc.child(doc.root(), 0, 0).unwrap();
    doc.cell_mut(first).unwrap().text = format!("document {seed}");
    doc.cell_mut(first).unwrap().image = image.map(ImageRef::new);
    doc
}

fn kid(doc: &Document, y: u32) -> CellId {
    doc.child(doc.root(), 0, y).unwrap()
}

fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

#[test]
fn two_documents_are_saved_together_or_not_at_all() {
    let dir = folder("save_two");
    let (a_path, b_path) = (dir.join("a.wombat"), dir.join("b.wombat"));
    let (old_a, old_b) = (sample(1, 1, None), sample(2, 1, None));
    let (mut new_a, mut new_b) = (old_a.clone(), old_b.clone());
    for doc in [&mut new_a, &mut new_b] {
        let first = kid(doc, 0);
        doc.cell_mut(first).unwrap().text = "new".into();
    }
    let reset = || {
        storage::save_document(&old_a, &a_path).unwrap();
        storage::save_document(&old_b, &b_path).unwrap();
    };
    let load = |path: &Path| storage::load_document(path).unwrap();
    let both = [(&new_a, a_path.as_path()), (&new_b, b_path.as_path())];

    // All goes well: both new, nothing left beside them.
    reset();
    storage::save_two(both[0], both[1]).unwrap();
    assert!(load(&a_path) == new_a && load(&b_path) == new_b);
    assert_eq!(names(&dir), ["a.wombat", "b.wombat"]);

    // A failure at any step but the last: both as they were, nothing left behind, and the
    // error names the file that the step was about.
    let steps = [
        (SaveStep::WriteFirst, &a_path),
        (SaveStep::WriteSecond, &b_path),
        (SaveStep::KeepFirst, &a_path),
        (SaveStep::ReplaceFirst, &a_path),
        (SaveStep::ReplaceSecond, &b_path),
    ];
    for (fails, named) in steps {
        reset();
        let mut seen = Vec::new();
        let result = storage::save_two_checked(both[0], both[1], &mut |step| {
            seen.push(step);
            if step == fails {
                return Err(std::io::Error::other("made to fail"));
            }
            Ok(())
        });
        let Err(Error::Io { path, source }) = result else {
            panic!("{fails:?}: {result:?}");
        };
        assert_eq!(
            (&path, source.to_string().as_str()),
            (named, "made to fail")
        );
        assert!(
            load(&a_path) == old_a && load(&b_path) == old_b,
            "{fails:?}"
        );
        assert_eq!(names(&dir), ["a.wombat", "b.wombat"], "{fails:?}");
        // The first was put back only where it had been replaced.
        assert_eq!(
            seen.contains(&SaveStep::RestoreFirst),
            fails == SaveStep::ReplaceSecond
        );
    }

    // The same without a hook, by what is on the disk: a folder where a temporary file
    // or the second document has to go.
    for blocked in [".a.wombat.tmp", ".b.wombat.tmp", ".a.wombat.bak"] {
        reset();
        fs::create_dir(dir.join(blocked)).unwrap();
        fs::write(dir.join(blocked).join("in the way"), b"x").unwrap();
        assert!(storage::save_two(both[0], both[1]).is_err(), "{blocked}");
        assert!(
            load(&a_path) == old_a && load(&b_path) == old_b,
            "{blocked}"
        );
        fs::remove_dir_all(dir.join(blocked)).unwrap();
        assert_eq!(names(&dir), ["a.wombat", "b.wombat"], "{blocked}");
    }
    reset();
    let c_path = dir.join("c.wombat");
    fs::create_dir(&c_path).unwrap();
    fs::write(c_path.join("in the way"), b"x").unwrap();
    assert!(storage::save_two(both[0], (&new_b, &c_path)).is_err());
    assert!(load(&a_path) == old_a);
    assert_eq!(names(&dir), ["a.wombat", "b.wombat", "c.wombat"]);
    fs::remove_dir_all(&c_path).unwrap();

    // The second cannot be replaced and the first cannot be put back: the error says
    // which file is in which state, and the old first file is still there to copy back.
    reset();
    let result = storage::save_two_checked(both[0], both[1], &mut |step| match step {
        SaveStep::ReplaceSecond => Err(std::io::Error::other("disk gone")),
        SaveStep::RestoreFirst => Err(std::io::Error::other("still gone")),
        _ => Ok(()),
    });
    let Err(error) = result else {
        panic!("saved");
    };
    let Error::HalfSaved {
        saved,
        not_saved,
        backup,
        ..
    } = &error
    else {
        panic!("{error:?}");
    };
    assert_eq!((saved, not_saved), (&a_path, &b_path));
    assert!(load(&a_path) == new_a && load(&b_path) == old_b);
    assert!(load(backup.as_ref().unwrap()) == old_a);
    let message = error.to_string();
    for part in [
        "a.wombat",
        "b.wombat",
        ".a.wombat.bak",
        "disk gone",
        "still gone",
    ] {
        assert!(message.contains(part), "{message}");
    }

    // A first file that was not there before is taken away again.
    reset();
    fs::remove_file(dir.join(".a.wombat.bak")).unwrap();
    fs::remove_file(&a_path).unwrap();
    let result = storage::save_two_checked(both[0], both[1], &mut |step| match step {
        SaveStep::ReplaceSecond => Err(std::io::Error::other("made to fail")),
        _ => Ok(()),
    });
    assert!(result.is_err());
    assert_eq!(names(&dir), ["b.wombat"]);
    // And the same file twice is refused.
    assert!(storage::save_two(both[0], (&new_b, &a_path)).is_err());
}

#[test]
fn images_go_with_a_clip_and_keep_showing_the_same_pictures() {
    let dir = folder("carry");
    let (from, to) = (dir.join("a.wombat"), dir.join("b.wombat"));
    let mut a = sample(1, 3, Some("same.png"));
    for (row, name) in [(1, "other.png"), (2, "new.png")] {
        let cell = kid(&a, row);
        a.cell_mut(cell).unwrap().image = Some(ImageRef::new(name));
    }
    let (from_dir, to_dir) = (storage::images_dir(&from), storage::images_dir(&to));
    fs::create_dir(&from_dir).unwrap();
    fs::create_dir(&to_dir).unwrap();
    for name in ["same.png", "other.png", "new.png"] {
        fs::write(from_dir.join(name), format!("a's {name}")).unwrap();
    }
    fs::write(to_dir.join("same.png"), "a's same.png").unwrap();
    fs::write(to_dir.join("other.png"), "b's own picture").unwrap();
    let mut ids = SeededIds::from_u64(7);
    let mut clip = Clip::of(&a, a.root()).unwrap();
    let carried = storage::carry_images(&mut clip, &from, &to, &mut ids).unwrap();
    assert_eq!(carried.reused, ["same.png"]);
    let [(old, new)] = &carried.renamed[..] else {
        panic!("{carried:?}");
    };
    assert_eq!(old, "other.png");
    assert!(new.ends_with(".png") && new != "other.png");
    assert_eq!(carried.copied, ["new.png".to_string(), new.clone()]);
    assert_eq!(clip.image_names(), {
        let mut names = vec!["new.png".to_string(), new.clone(), "same.png".to_string()];
        names.sort();
        names
    });
    assert_eq!(
        fs::read(to_dir.join("other.png")).unwrap(),
        b"b's own picture"
    );
    assert_eq!(fs::read(to_dir.join(new)).unwrap(), b"a's other.png");
    assert_eq!(fs::read(to_dir.join("new.png")).unwrap(), b"a's new.png");
    // Again: everything is there already, under the names found.
    let mut again = Clip::of(&a, a.root()).unwrap();
    let second = storage::carry_images(&mut again, &from, &to, &mut ids).unwrap();
    assert!(second.copied.is_empty(), "{second:?}");
    assert_eq!(again, clip);
    // Taking back removes what was copied, and nothing else.
    carried.take_back(&to);
    assert_eq!(names(&to_dir), ["other.png", "same.png"]);

    // A missing image: nothing is copied, not even the ones before it.
    fs::remove_file(from_dir.join("other.png")).unwrap();
    let mut clip = Clip::of(&a, a.root()).unwrap();
    let error = storage::carry_images(&mut clip, &from, &to, &mut ids).unwrap_err();
    assert!(error.to_string().contains("other.png"), "{error}");
    assert_eq!(names(&to_dir), ["other.png", "same.png"]);
    assert_eq!(clip, Clip::of(&a, a.root()).unwrap());
}

#[cfg(unix)]
mod links {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn an_image_is_not_added_through_a_linked_image_folder() {
        let dir = folder("add_image_dir");
        let elsewhere = dir.join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        let doc_path = dir.join("a.wombat");
        symlink(&elsewhere, storage::images_dir(&doc_path)).unwrap();
        let picture = dir.join("pic.png");
        fs::write(&picture, b"picture").unwrap();
        let mut ids = SeededIds::from_u64(1);
        let result = storage::add_image(&doc_path, &picture, &mut ids);
        assert!(result.is_err(), "{result:?}");
        assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);
    }

    #[test]
    fn copying_a_document_does_not_write_images_through_links() {
        let dir = folder("copy_images");
        let from = dir.join("a.wombat");
        let doc = sample(1, 2, Some("pic.png"));
        storage::save_document(&doc, &from).unwrap();
        fs::create_dir(storage::images_dir(&from)).unwrap();
        fs::write(storage::images_dir(&from).join("pic.png"), b"new picture").unwrap();
        // Someone put a link where the copy's image will go.
        let victim = dir.join("victim.txt");
        fs::write(&victim, b"precious").unwrap();
        let to = dir.join("b.wombat");
        fs::create_dir(storage::images_dir(&to)).unwrap();
        let planted = storage::images_dir(&to).join("pic.png");
        symlink(&victim, &planted).unwrap();
        let mut ids = SeededIds::from_u64(2);
        storage::copy_document(&from, &to, &mut ids).unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"precious");
        assert!(!is_link(&planted));
        assert_eq!(fs::read(&planted).unwrap(), b"new picture");

        // And the image folder itself a link to another folder: refused, nothing written.
        let elsewhere = dir.join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        let to = dir.join("c.wombat");
        symlink(&elsewhere, storage::images_dir(&to)).unwrap();
        let result = storage::copy_document(&from, &to, &mut ids);
        assert!(result.is_err(), "{result:?}");
        assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);
        assert!(!to.exists());
    }

    #[test]
    fn view_state_is_not_written_through_links() {
        let dir = folder("view_links");
        let doc = sample(1, 1, None);
        let view = ViewState::new(doc.id());
        // A link at the very name of the view-state file.
        let victim = dir.join("victim.txt");
        fs::write(&victim, b"precious").unwrap();
        fs::create_dir(dir.join(VIEW_DIR)).unwrap();
        let path = storage::view_state_path(&dir, doc.id());
        symlink(&victim, &path).unwrap();
        storage::save_view_state(&dir, &view).unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"precious");
        assert!(!is_link(&path));
        assert_eq!(
            storage::load_view_state(&dir, doc.id()).unwrap(),
            Some(view.clone())
        );

        // The folder of view states a link to another folder: refused.
        let other = folder("view_links_dir");
        let elsewhere = other.join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        symlink(&elsewhere, other.join(VIEW_DIR)).unwrap();
        let result = storage::save_view_state(&other, &view);
        assert!(result.is_err(), "{result:?}");
        assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);
    }

    #[test]
    fn carried_images_are_not_written_or_read_through_links() {
        let dir = folder("carry_links");
        let (from, to) = (dir.join("a.wombat"), dir.join("b.wombat"));
        let a = sample(1, 1, Some("pic.png"));
        let (from_dir, to_dir) = (storage::images_dir(&from), storage::images_dir(&to));
        fs::create_dir(&from_dir).unwrap();
        fs::write(from_dir.join("pic.png"), b"picture").unwrap();
        let victim = dir.join("victim.txt");
        fs::write(&victim, b"precious").unwrap();
        let mut ids = SeededIds::from_u64(7);

        // A link where the image would go: it is another "file" of that name, so the
        // image gets a name of its own, and the link is left alone.
        fs::create_dir(&to_dir).unwrap();
        symlink(&victim, to_dir.join("pic.png")).unwrap();
        let mut clip = Clip::of(&a, a.root()).unwrap();
        let carried = storage::carry_images(&mut clip, &from, &to, &mut ids).unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"precious");
        assert_eq!(carried.renamed.len(), 1);
        assert!(is_link(&to_dir.join("pic.png")));
        carried.take_back(&to);
        assert_eq!(fs::read(&victim).unwrap(), b"precious");

        // The target's image folder a link: refused.
        fs::remove_dir_all(&to_dir).unwrap();
        let elsewhere = dir.join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        symlink(&elsewhere, &to_dir).unwrap();
        let mut clip = Clip::of(&a, a.root()).unwrap();
        assert!(storage::carry_images(&mut clip, &from, &to, &mut ids).is_err());
        assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);

        // The source image a link to some other file: not copied out as if it were one.
        fs::remove_file(&to_dir).unwrap();
        fs::remove_file(from_dir.join("pic.png")).unwrap();
        symlink(&victim, from_dir.join("pic.png")).unwrap();
        let mut clip = Clip::of(&a, a.root()).unwrap();
        assert!(storage::carry_images(&mut clip, &from, &to, &mut ids).is_err());
        assert!(!to_dir.exists());

        // Removing unused images removes a link itself, not what it leads to.
        let empty = sample(1, 1, None);
        let removed =
            storage::remove_unused_images(&empty, &from, &["pic.png".to_string()]).unwrap();
        assert_eq!(removed, ["pic.png"]);
        assert_eq!(fs::read(&victim).unwrap(), b"precious");
        assert!(!is_link(&from_dir.join("pic.png")));
        // A name that would leave the folder is refused.
        let bad = ["../victim.txt".to_string()];
        assert!(storage::remove_unused_images(&empty, &from, &bad).is_err());
        assert_eq!(fs::read(&victim).unwrap(), b"precious");
    }

    #[test]
    fn two_documents_are_not_saved_through_links_at_the_names_beside_them() {
        let dir = folder("save_two_links");
        let (a_path, b_path) = (dir.join("a.wombat"), dir.join("b.wombat"));
        let (a, b) = (sample(1, 1, None), sample(2, 1, None));
        storage::save_document(&a, &a_path).unwrap();
        storage::save_document(&b, &b_path).unwrap();
        let victim = dir.join("victim.txt");
        fs::write(&victim, b"precious").unwrap();
        for planted in [".a.wombat.tmp", ".b.wombat.tmp", ".a.wombat.bak"] {
            symlink(&victim, dir.join(planted)).unwrap();
        }
        storage::save_two((&a, &a_path), (&b, &b_path)).unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"precious");
        assert_eq!(names(&dir), ["a.wombat", "b.wombat", "victim.txt"]);
        assert!(!is_link(&a_path) && !is_link(&b_path));
    }
}
