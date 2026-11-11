//! Writing exports to disk: nothing is replaced unless asked, no symbolic link is ever
//! written through, and no name from a document reaches outside the folder.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use wombat_core::construct::Construct;
use wombat_core::convert::lookup::{self, LookupOptions};
use wombat_core::convert::{ExportOptions, Source};
use wombat_core::eval::Calc;
use wombat_core::export_files::{write_file, write_folder};
use wombat_core::{Document, Error, SeededIds};

/// A fresh, empty folder for one test.
fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("export_files")
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

const FILES: &[(&str, &[u8])] = &[
    ("index.csv", b"index"),
    ("t-1.csv", b"one"),
    ("t-2.csv", b"two"),
];

#[test]
fn a_file_is_written_whole_and_an_existing_one_only_when_asked() {
    let dir = folder("file");
    let path = dir.join("out.csv");
    write_file(&path, b"first", false).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"first");
    assert_eq!(names(&dir), ["out.csv"], "no temporary file is left");
    let again = write_file(&path, b"second", false);
    assert!(matches!(again, Err(Error::AlreadyExists(_))), "{again:?}");
    assert_eq!(fs::read(&path).unwrap(), b"first");
    write_file(&path, b"second", true).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"second");
    // A folder is not a file, and a file in a folder that is not there is an error.
    assert!(matches!(
        write_file(&dir, b"x", true),
        Err(Error::Refused(_))
    ));
    assert!(matches!(
        write_file(&dir.join("no/out.csv"), b"x", true),
        Err(Error::Io { .. })
    ));
    assert_eq!(names(&dir), ["out.csv"]);
}

#[test]
fn a_folder_is_made_and_one_with_anything_in_it_only_when_asked() {
    let dir = folder("folder");
    let out = dir.join("out");
    write_folder(&out, FILES, false).unwrap();
    assert_eq!(names(&out), ["index.csv", "t-1.csv", "t-2.csv"]);
    assert_eq!(fs::read(out.join("t-2.csv")).unwrap(), b"two");
    // Again without overwrite: refused, nothing touched.
    let changed: &[(&str, &[u8])] = &[("index.csv", b"new"), ("t-3.csv", b"three")];
    let again = write_folder(&out, changed, false);
    assert!(matches!(again, Err(Error::Refused(_))), "{again:?}");
    assert_eq!(fs::read(out.join("index.csv")).unwrap(), b"index");
    assert_eq!(names(&out).len(), 3);
    // With overwrite: files of the same name are replaced, others are left alone.
    write_folder(&out, changed, true).unwrap();
    assert_eq!(fs::read(out.join("index.csv")).unwrap(), b"new");
    assert_eq!(names(&out), ["index.csv", "t-1.csv", "t-2.csv", "t-3.csv"]);
    // An existing empty folder is fine; a file in the folder's place is not; neither is
    // a folder whose parent is not there.
    let empty = dir.join("empty");
    fs::create_dir(&empty).unwrap();
    write_folder(&empty, FILES, false).unwrap();
    assert!(matches!(
        write_folder(&out.join("index.csv"), FILES, true),
        Err(Error::Refused(_))
    ));
    assert!(matches!(
        write_folder(&dir.join("a/b"), FILES, true),
        Err(Error::Io { .. })
    ));
    assert!(!dir.join("a").exists());
}

#[test]
fn names_that_are_not_plain_file_names_are_refused_before_anything_is_written() {
    let dir = folder("names");
    fs::write(dir.join("victim"), b"untouched").unwrap();
    for bad in [
        "../victim",
        "..",
        ".",
        "",
        "/etc/passwd",
        "sub/file.csv",
        "sub\\file.csv",
        "a\0b",
        "x/../../victim",
    ] {
        let out = dir.join("out");
        let files: &[(&str, &[u8])] = &[("good.csv", b"good"), (bad, b"bad")];
        let result = write_folder(&out, files, true);
        assert!(
            matches!(result, Err(Error::Invalid(_))),
            "{bad:?}: {result:?}"
        );
        assert!(!out.exists(), "{bad:?}: the folder was made");
    }
    // The same name twice, also in another case, which is one file on some systems.
    let twice: &[(&str, &[u8])] = &[("Table.csv", b"1"), ("table.csv", b"2")];
    assert!(matches!(
        write_folder(&dir.join("out"), twice, true),
        Err(Error::Invalid(_))
    ));
    assert_eq!(fs::read(dir.join("victim")).unwrap(), b"untouched");
    assert_eq!(names(&dir), ["victim"]);
}

#[cfg(unix)]
mod links {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn a_link_at_the_file_is_not_written_through() {
        let dir = folder("link_file");
        let victim = dir.join("victim.txt");
        fs::write(&victim, b"untouched").unwrap();
        let path = dir.join("out.csv");
        symlink(&victim, &path).unwrap();
        for overwrite in [false, true] {
            let result = write_file(&path, b"export", overwrite);
            assert!(matches!(result, Err(Error::Refused(_))), "{result:?}");
        }
        assert_eq!(fs::read(&victim).unwrap(), b"untouched");
        assert!(fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink());
        // A link that leads nowhere is not followed to create its target either.
        let dangling = dir.join("dangling.csv");
        symlink(dir.join("nowhere"), &dangling).unwrap();
        assert!(matches!(
            write_file(&dangling, b"export", true),
            Err(Error::Refused(_))
        ));
        assert!(!dir.join("nowhere").exists());
    }

    #[test]
    fn a_link_at_the_temporary_name_is_not_written_through() {
        let dir = folder("link_temp");
        let victim = dir.join("victim.txt");
        fs::write(&victim, b"untouched").unwrap();
        symlink(&victim, dir.join(".out.csv.tmp")).unwrap();
        write_file(&dir.join("out.csv"), b"export", false).unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"untouched");
        assert_eq!(fs::read(dir.join("out.csv")).unwrap(), b"export");
        assert_eq!(names(&dir), ["out.csv", "victim.txt"]);
        // The same inside a folder.
        let out = dir.join("out");
        fs::create_dir(&out).unwrap();
        symlink(&victim, out.join(".t-1.csv.tmp")).unwrap();
        write_folder(&out, FILES, true).unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"untouched");
        assert_eq!(names(&out), ["index.csv", "t-1.csv", "t-2.csv"]);
    }

    #[test]
    fn a_link_at_the_folder_is_not_written_through() {
        let dir = folder("link_folder");
        let elsewhere = dir.join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        fs::write(elsewhere.join("index.csv"), b"untouched").unwrap();
        let out = dir.join("out");
        symlink(&elsewhere, &out).unwrap();
        for overwrite in [false, true] {
            let result = write_folder(&out, FILES, overwrite);
            assert!(matches!(result, Err(Error::Refused(_))), "{result:?}");
        }
        assert_eq!(names(&elsewhere), ["index.csv"]);
        assert_eq!(fs::read(elsewhere.join("index.csv")).unwrap(), b"untouched");
        // Nor through a link to a folder that is empty, or to nothing.
        let empty = dir.join("empty");
        fs::create_dir(&empty).unwrap();
        let to_empty = dir.join("to_empty");
        symlink(&empty, &to_empty).unwrap();
        assert!(matches!(
            write_folder(&to_empty, FILES, false),
            Err(Error::Refused(_))
        ));
        assert!(names(&empty).is_empty());
        let dangling = dir.join("dangling");
        symlink(dir.join("nowhere"), &dangling).unwrap();
        assert!(matches!(
            write_folder(&dangling, FILES, true),
            Err(Error::Refused(_))
        ));
        assert!(!dir.join("nowhere").exists());
    }

    #[test]
    fn a_link_at_the_name_of_a_table_is_not_written_through_and_nothing_else_is_written() {
        let dir = folder("link_table");
        let victim = dir.join("victim.txt");
        fs::write(&victim, b"untouched").unwrap();
        let out = dir.join("out");
        fs::create_dir(&out).unwrap();
        // The last file of the export: the others must not be written before it is found.
        symlink(&victim, out.join("t-2.csv")).unwrap();
        let result = write_folder(&out, FILES, true);
        assert!(matches!(result, Err(Error::Refused(_))), "{result:?}");
        assert_eq!(fs::read(&victim).unwrap(), b"untouched");
        assert_eq!(names(&out), ["t-2.csv"]);
        // A folder at the name of a file is refused too.
        fs::remove_file(out.join("t-2.csv")).unwrap();
        fs::create_dir(out.join("index.csv")).unwrap();
        assert!(matches!(
            write_folder(&out, FILES, true),
            Err(Error::Refused(_))
        ));
        assert_eq!(names(&out), ["index.csv"]);
    }
}

#[test]
fn cell_text_cannot_steer_where_the_tables_of_a_lookup_export_go() {
    let dir = folder("hostile");
    fs::write(dir.join("victim.csv"), b"untouched").unwrap();
    let mut ids = SeededIds::from_u64(3);
    let mut doc = Document::new(&mut ids);
    let hostile = [
        "../victim",
        "..",
        "/etc/passwd",
        "C:\\Windows\\win.ini",
        "CON",
        "nul.csv",
        "index",
        "README.txt",
        "a/b/c",
        "~/.ssh/authorized_keys",
        "",
        ".hidden",
        "-rf",
        "x\u{202e}vsc.exe",
    ];
    let root = doc.root();
    doc.cell_mut(root).unwrap().text = "../../top".into();
    doc.add_grid(root, 1, hostile.len() as u32, &mut ids)
        .unwrap();
    for (id, text) in doc
        .cell(root)
        .unwrap()
        .grid()
        .unwrap()
        .cells()
        .to_vec()
        .into_iter()
        .zip(hostile)
    {
        doc.cell_mut(id).unwrap().text = text.into();
        doc.add_grid(id, 1, 1, &mut ids).unwrap();
    }
    let calc = Calc::new(&doc);
    let source = Source {
        doc: &doc,
        calc: &calc,
    };
    let files = lookup::export(
        source,
        root,
        &ExportOptions::default(),
        LookupOptions::default(),
    )
    .unwrap();
    assert_eq!(files.len(), hostile.len() + 3);
    for f in &files {
        assert!(
            !f.name.contains(['/', '\\', ':', '~', '\u{202e}']),
            "{}",
            f.name
        );
        assert!(!f.name.starts_with(['.', '-']), "{}", f.name);
    }
    let out = dir.join("out");
    let parts: Vec<(&str, &[u8])> = files
        .iter()
        .map(|f| (f.name.as_str(), f.text.as_bytes()))
        .collect();
    write_folder(&out, &parts, false).unwrap();
    // Everything is in the folder, nothing beside it, and the bystander is as it was.
    assert_eq!(names(&dir), ["out", "victim.csv"]);
    assert_eq!(fs::read(dir.join("victim.csv")).unwrap(), b"untouched");
    let written = names(&out);
    assert_eq!(written.len(), files.len());
    assert!(written.iter().all(|name| out.join(name).is_file()));
    assert_eq!(
        written
            .iter()
            .filter(|n| n.as_str() == "index.csv" || n.as_str() == "README.txt")
            .count(),
        2
    );
}
