//! The link index of a workspace folder and its cache on disk: whatever happens to the
//! documents or to the cache file, the index is the one that reading every document gives.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use wombat_core::construct::Construct;
use wombat_core::links::{Broken, DocLinks, Resolved};
use wombat_core::scan::{self, CacheUse, WorkspaceLinks};
use wombat_core::{
    storage, CellId, Command, Document, Editor, Link, LinkTarget, SeededIds, Workspace,
};

fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("links_workspace")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A document of one column of cells with these texts.
fn doc(ids: &mut SeededIds, texts: &[&str]) -> Document {
    let mut doc = Document::new(ids);
    let root = doc.root();
    doc.add_grid(root, 1, texts.len() as u32, ids).unwrap();
    for (n, text) in texts.iter().enumerate() {
        let cell = doc.child(root, 0, n as u32).unwrap();
        doc.cell_mut(cell).unwrap().text = text.to_string();
    }
    doc
}

fn cell(doc: &Document, row: u32) -> CellId {
    doc.child(doc.root(), 0, row).unwrap()
}

fn link(doc: &mut Document, row: u32, to: &Document, to_row: u32) {
    let target = LinkTarget::Cell {
        doc: (to.id() != doc.id()).then_some(to.id()),
        cell: cell(to, to_row),
    };
    let from = cell(doc, row);
    doc.cell_mut(from).unwrap().links = vec![Link {
        start: 0,
        len: 1,
        target,
        extra: Default::default(),
    }];
}

/// Three documents: `a` links to `b` and to itself, `b` to `c`, `c` to `a`.
fn workspace(test: &str) -> (Workspace, [Document; 3]) {
    let dir = folder(test);
    let mut ids = SeededIds::from_u64(11);
    let mut a = doc(&mut ids, &["alpha", "apple", "avocado"]);
    let mut b = doc(&mut ids, &["beta", "banana"]);
    let mut c = doc(&mut ids, &["gamma"]);
    link(&mut a, 0, &b, 1);
    let own = a.clone();
    link(&mut a, 1, &own, 2);
    link(&mut b, 0, &c, 0);
    link(&mut c, 0, &a, 0);
    let ws = Workspace::open(&dir).unwrap();
    ws.save(&a, "a.wombat").unwrap();
    ws.save(&b, "b.wombat").unwrap();
    ws.save(&c, "c.wombat").unwrap();
    (ws, [a, b, c])
}

/// The index with the cache, which must be the index without it.
fn checked(ws: &Workspace) -> WorkspaceLinks {
    let plain = scan::link_index(ws, false).unwrap();
    assert_eq!(plain.cache, CacheUse::Off);
    assert!(!plain.cache_written && plain.reused == 0);
    let cached = scan::link_index(ws, true).unwrap();
    assert!(cached.index == plain.index, "the cache gave another index");
    assert_eq!(cached.names, plain.names);
    assert_eq!(cached.duplicates, plain.duplicates);
    assert_eq!(cached.failed.len(), plain.failed.len());
    cached
}

fn cache_file(ws: &Workspace) -> PathBuf {
    scan::cache_path::<DocLinks>(ws)
}

#[test]
fn the_workspace_index_knows_documents_that_are_not_open() {
    let (ws, [a, b, c]) = workspace("closed");
    let links = scan::link_index(&ws, false).unwrap();
    assert!(!cache_file(&ws).exists(), "no cache was asked for");
    assert_eq!(links.parsed, 3);
    assert_eq!(links.names[&a.id()], "a.wombat");
    assert_eq!(links.index.broken().len(), 0);
    let back = links.index.backlinks(b.id(), cell(&b, 1));
    assert_eq!(back.len(), 1);
    assert_eq!((back[0].doc, back[0].cell), (a.id(), cell(&a, 0)));
    assert_eq!(back[0].text, "a");
    assert_eq!(links.index.backlinks(a.id(), cell(&a, 0))[0].doc, c.id());
    // Deleting a document: the warning beforehand, and broken links afterwards.
    assert_eq!(links.index.would_break_document(c.id()).len(), 1);
    fs::remove_file(ws.dir().join("c.wombat")).unwrap();
    let links = scan::link_index(&ws, false).unwrap();
    let broken = links.index.broken();
    assert_eq!(broken.len(), 1);
    assert_eq!(broken[0].doc, b.id());
    assert_eq!(
        broken[0].resolved,
        Resolved::Broken(Broken::NoDocument(c.id()))
    );
    // A document is found by its ID wherever its file is.
    fs::rename(ws.dir().join("b.wombat"), ws.dir().join("renamed.wombat")).unwrap();
    let links = scan::link_index(&ws, false).unwrap();
    assert_eq!(links.names[&b.id()], "renamed.wombat");
    assert_eq!(links.index.backlinks(b.id(), cell(&b, 1)).len(), 1);
}

#[test]
fn the_cache_is_used_only_for_files_that_are_unchanged() {
    let (ws, [a, b, _]) = workspace("stale");
    let first = checked(&ws);
    assert_eq!(first.cache, CacheUse::Missing);
    assert!(first.cache_written && first.parsed == 3 && first.reused == 0);
    let bytes = fs::read(cache_file(&ws)).unwrap();

    // Nothing changed: all from the cache, which is not written again.
    let second = checked(&ws);
    assert_eq!(second.cache, CacheUse::Read);
    assert_eq!((second.parsed, second.reused), (0, 3));
    assert!(!second.cache_written);
    assert_eq!(fs::read(cache_file(&ws)).unwrap(), bytes);

    // One document changes, with a file of the same length and the same modification
    // time as before: it is still noticed, because the bytes are read.
    let path = ws.dir().join("a.wombat");
    let time = fs::metadata(&path).unwrap().modified().unwrap();
    let old_len = fs::metadata(&path).unwrap().len();
    let mut editor = Editor::new(a.clone()).unwrap();
    let relink = Command::SetLink {
        cell: cell(&a, 0),
        start: 0,
        len: 1,
        target: Some(LinkTarget::Cell {
            doc: Some(b.id()),
            cell: cell(&b, 0),
        }),
    };
    editor
        .apply(relink, None, &mut SeededIds::from_u64(1))
        .unwrap();
    ws.save(editor.document(), "a.wombat").unwrap();
    assert_eq!(fs::metadata(&path).unwrap().len(), old_len);
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(time)
        .unwrap();
    let third = checked(&ws);
    assert_eq!((third.parsed, third.reused), (1, 2));
    assert!(third.cache_written);
    assert_eq!(third.index.backlinks(b.id(), cell(&b, 0)).len(), 1);
    assert_eq!(third.index.backlinks(b.id(), cell(&b, 1)).len(), 0);

    // A renamed file is the same bytes: nothing is parsed, the name follows.
    fs::rename(&path, ws.dir().join("z.wombat")).unwrap();
    let fourth = checked(&ws);
    assert_eq!((fourth.parsed, fourth.reused), (0, 3));
    assert_eq!(fourth.names[&a.id()], "z.wombat");

    // A deleted document leaves the index, and its entry leaves the cache.
    fs::remove_file(ws.dir().join("c.wombat")).unwrap();
    let fifth = checked(&ws);
    assert_eq!(fifth.index.documents().count(), 2);
    assert!(fifth.cache_written);
    assert!(fs::read(cache_file(&ws)).unwrap().len() < bytes.len());
    // A new document is parsed, the others are not.
    let new = doc(&mut SeededIds::from_u64(77), &["new"]);
    ws.save(&new, "new.wombat").unwrap();
    let sixth = checked(&ws);
    assert_eq!((sixth.parsed, sixth.reused), (1, 2));
}

#[test]
fn a_deleted_or_damaged_cache_is_rebuilt_and_never_believed() {
    let (ws, _) = workspace("damaged");
    let good = checked(&ws);
    let path = cache_file(&ws);
    let bytes = fs::read(&path).unwrap();
    let rebuilt = |what: &str, state: CacheUse| {
        let links = checked(&ws);
        assert_eq!(links.cache, state, "{what}");
        assert_eq!((links.parsed, links.reused), (3, 0), "{what}");
        assert!(links.index == good.index, "{what}");
        // And it wrote a good cache again.
        assert_eq!(fs::read(&path).unwrap(), bytes, "{what}");
    };
    // Deleted: the file, or the whole folder.
    fs::remove_file(&path).unwrap();
    rebuilt("deleted file", CacheUse::Missing);
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
    rebuilt("deleted folder", CacheUse::Missing);
    // Damaged in every way that comes to mind.
    let header_end = bytes.iter().position(|b| *b == b'\n').unwrap();
    let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
    let body = &bytes[header_end + 1..];
    let with_header = |change: &dyn Fn(&str) -> String| {
        let mut out = change(&header).into_bytes();
        out.push(b'\n');
        out.extend_from_slice(body);
        out
    };
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", Vec::new()),
        ("cut short", bytes[..bytes.len() / 2].to_vec()),
        ("one byte short", bytes[..bytes.len() - 1].to_vec()),
        ("one byte more", [bytes.as_slice(), b" "].concat()),
        ("only a header", bytes[..=header_end].to_vec()),
        ("no header", body.to_vec()),
        ("rubbish", b"\x00\xff rubbish \n {".to_vec()),
        ("not text", vec![0xff; 4096]),
        (
            "another kind",
            with_header(&|h| h.replace(" links ", " words ")),
        ),
        (
            "another version",
            with_header(&|h| h.replace(" links 1 ", " links 2 ")),
        ),
        (
            "another checksum",
            with_header(&|h| format!("{}0", &h[..h.len() - 1])),
        ),
        (
            "a body of nothing",
            with_header(&|h| h.to_string())[..=header_end].to_vec(),
        ),
    ];
    for (what, damaged) in cases {
        assert_ne!(damaged, bytes, "{what}");
        fs::write(&path, damaged).unwrap();
        rebuilt(what, CacheUse::Damaged);
    }
    // Any single byte changed anywhere: never a wrong index.
    for at in (0..bytes.len()).step_by(3) {
        let mut damaged = bytes.clone();
        damaged[at] ^= 0x21;
        fs::write(&path, &damaged).unwrap();
        rebuilt(&format!("byte {at} changed"), CacheUse::Damaged);
    }
    assert!(checked(&ws).cache == CacheUse::Read);
    // A cache in a folder that cannot be written is not an error.
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path.parent().unwrap(), "a file where the folder should be").unwrap();
    let links = scan::link_index(&ws, true).unwrap();
    assert!(!links.cache_written && links.index == good.index);
    assert_eq!(links.cache, CacheUse::Missing);
}

#[test]
fn damaged_documents_and_duplicated_ids_are_reported() {
    let (ws, [a, b, c]) = workspace("duplicates");
    // A copy of a file made by hand: two files, one document ID.
    fs::copy(ws.dir().join("b.wombat"), ws.dir().join("b copy.wombat")).unwrap();
    fs::write(ws.dir().join("broken.wombat"), "{ not a document").unwrap();
    fs::write(ws.dir().join("notes.txt"), "not a document at all").unwrap();
    let links = checked(&ws);
    assert_eq!(links.failed.len(), 1);
    assert_eq!(links.failed[0].0, "broken.wombat");
    assert_eq!(
        links.duplicates[&b.id()],
        ["b copy.wombat".to_string(), "b.wombat".to_string()]
    );
    assert_eq!(links.names[&b.id()], "b copy.wombat");
    // The link from `a` into the duplicated document is not resolved to either file.
    let out = links.index.outgoing(a.id());
    let ambiguous = Resolved::Broken(Broken::AmbiguousDocument(b.id()));
    assert_eq!(out.iter().filter(|l| l.resolved == ambiguous).count(), 1);
    assert_eq!(links.index.broken().len(), 1);
    // The duplicated document's own link, into `c`, is one backlink, not two.
    assert_eq!(links.index.backlinks(c.id(), cell(&c, 0)).len(), 1);
    // Once the copy is a copy with IDs of its own, all is well.
    fs::remove_file(ws.dir().join("b copy.wombat")).unwrap();
    storage::copy_document(
        &ws.dir().join("b.wombat"),
        &ws.dir().join("b2.wombat"),
        &mut SeededIds::from_u64(5),
    )
    .unwrap();
    let links = checked(&ws);
    assert!(links.duplicates.is_empty());
    assert_eq!(links.index.broken().len(), 0);
    assert_eq!(links.index.backlinks(c.id(), cell(&c, 0)).len(), 2);
}

#[test]
fn an_open_document_is_followed_and_agrees_with_the_folder_after_saving() {
    let (ws, [a, b, _]) = workspace("open");
    let mut links = checked(&ws);
    let mut editor = Editor::new(a.clone()).unwrap();
    let mut ids = SeededIds::from_u64(8);
    let commands = [
        Command::DeleteRows {
            cell: a.root(),
            at: 2,
            count: 1,
        },
        Command::SetLink {
            cell: cell(&a, 1),
            start: 0,
            len: 5,
            target: Some(LinkTarget::Cell {
                doc: Some(b.id()),
                cell: cell(&b, 0),
            }),
        },
    ];
    for command in commands {
        let changes = editor.apply(command, None, &mut ids).unwrap();
        links.index.apply(editor.document(), &changes);
    }
    assert_eq!(links.index.backlinks(b.id(), cell(&b, 0)).len(), 1);
    ws.save(editor.document(), "a.wombat").unwrap();
    let fresh = checked(&ws);
    assert!(fresh.index == links.index);
    assert_eq!((fresh.parsed, fresh.reused), (1, 2));
}
