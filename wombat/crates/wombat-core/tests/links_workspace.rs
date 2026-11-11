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
use wombat_core::scan::{self, Cache, CacheMode, CacheUse, WorkspaceLinks};
use wombat_core::{
    storage, CellId, Command, Document, Editor, Link, LinkTarget, SeededIds, Workspace,
};

fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("links_workspace")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(dir.with_extension("cache"));
    let _ = fs::remove_file(dir.with_extension("cache"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The cache of a workspace of these tests: a folder beside it, never inside it.
fn cache_of(ws: &Workspace, mode: CacheMode) -> Cache {
    Cache::at(ws.dir().with_extension("cache"), mode)
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
fn checked_with(ws: &Workspace, mode: CacheMode) -> WorkspaceLinks {
    let plain = scan::link_index(ws, &cache_of(ws, CacheMode::Off)).unwrap();
    assert_eq!(plain.cache, CacheUse::Off);
    assert!(!plain.cache_written && plain.reused == 0);
    let cached = scan::link_index(ws, &cache_of(ws, mode)).unwrap();
    assert!(cached.index == plain.index, "the cache gave another index");
    assert_eq!(cached.names, plain.names);
    assert_eq!(cached.duplicates, plain.duplicates);
    assert_eq!(cached.failed.len(), plain.failed.len());
    cached
}

fn checked(ws: &Workspace) -> WorkspaceLinks {
    checked_with(ws, CacheMode::Verify)
}

fn cache_file(ws: &Workspace, name: &str) -> PathBuf {
    scan::cache_path::<DocLinks>(&cache_of(ws, CacheMode::Verify), name)
}

/// The names and bytes of all cache entries.
fn cache_files(ws: &Workspace) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = fs::read_dir(scan::cache_dir::<DocLinks>(&cache_of(
        ws,
        CacheMode::Verify,
    )))
    .unwrap()
    .map(|entry| {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        (name, fs::read(entry.path()).unwrap())
    })
    .collect();
    files.sort();
    files
}

/// Gives a file a modification time `seconds` before now.
fn age(path: &Path, seconds: u64) {
    let time = std::time::SystemTime::now() - std::time::Duration::from_secs(seconds);
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(time).unwrap();
}

#[test]
fn the_workspace_index_knows_documents_that_are_not_open() {
    let (ws, [a, b, c]) = workspace("closed");
    let links = scan::link_index(&ws, &cache_of(&ws, CacheMode::Off)).unwrap();
    assert!(
        !ws.dir().with_extension("cache").exists(),
        "no cache was asked for"
    );
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
    let links = scan::link_index(&ws, &cache_of(&ws, CacheMode::Off)).unwrap();
    let broken = links.index.broken();
    assert_eq!(broken.len(), 1);
    assert_eq!(broken[0].doc, b.id());
    assert_eq!(
        broken[0].resolved,
        Resolved::Broken(Broken::NoDocument(c.id()))
    );
    // A document is found by its ID wherever its file is.
    fs::rename(ws.dir().join("b.wombat"), ws.dir().join("renamed.wombat")).unwrap();
    let links = scan::link_index(&ws, &cache_of(&ws, CacheMode::Off)).unwrap();
    assert_eq!(links.names[&b.id()], "renamed.wombat");
    assert_eq!(links.index.backlinks(b.id(), cell(&b, 1)).len(), 1);
}

/// Saves `a` with its first link going elsewhere, in a file of the same length and with
/// the same modification time as before.
fn change_in_place(ws: &Workspace, a: &Document, b: &Document) {
    let path = ws.dir().join("a.wombat");
    let time = fs::metadata(&path).unwrap().modified().unwrap();
    let old_len = fs::metadata(&path).unwrap().len();
    let mut editor = Editor::new(a.clone()).unwrap();
    let relink = Command::SetLink {
        cell: cell(a, 0),
        start: 0,
        len: 1,
        target: Some(LinkTarget::Cell {
            doc: Some(b.id()),
            cell: cell(b, 0),
        }),
    };
    editor
        .apply(relink, None, &mut SeededIds::from_u64(1))
        .unwrap();
    ws.save(editor.document(), "a.wombat").unwrap();
    assert_eq!(fs::metadata(&path).unwrap().len(), old_len);
    let file = fs::File::options().write(true).open(&path).unwrap();
    file.set_modified(time).unwrap();
}

#[test]
fn the_cache_is_used_only_for_files_that_are_unchanged() {
    let (ws, [a, b, _]) = workspace("stale");
    let first = checked(&ws);
    assert_eq!(first.cache, CacheUse::Missing);
    assert!(first.cache_written && first.parsed == 3 && first.reused == 0);
    let files = cache_files(&ws);
    let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["a.wombat.idx", "b.wombat.idx", "c.wombat.idx"]);

    // Nothing changed: all from the cache, which is not written again.
    let second = checked(&ws);
    assert_eq!(second.cache, CacheUse::Read);
    assert_eq!((second.parsed, second.reused), (0, 3));
    assert!(!second.cache_written && second.rejected == 0);
    assert_eq!(cache_files(&ws), files);

    // One document changes, with a file of the same length and the same modification
    // time as before: it is still noticed, because the bytes are read. Only its entry
    // is written again.
    change_in_place(&ws, &a, &b);
    let third = checked(&ws);
    assert_eq!((third.parsed, third.reused), (1, 2));
    assert!(third.cache_written && third.rejected == 1);
    assert_eq!(third.index.backlinks(b.id(), cell(&b, 0)).len(), 1);
    assert_eq!(third.index.backlinks(b.id(), cell(&b, 1)).len(), 0);
    let now = cache_files(&ws);
    assert_ne!(now[0], files[0]);
    assert_eq!(now[1..], files[1..]);

    // A renamed file is parsed again (an entry goes by the name of its file), and the
    // entry under the old name is removed.
    fs::rename(ws.dir().join("a.wombat"), ws.dir().join("z.wombat")).unwrap();
    let fourth = checked(&ws);
    assert_eq!((fourth.parsed, fourth.reused), (1, 2));
    assert_eq!(fourth.names[&a.id()], "z.wombat");
    assert!(!cache_file(&ws, "a.wombat").exists() && cache_file(&ws, "z.wombat").exists());

    // A deleted document leaves the index, and its entry leaves the cache.
    fs::remove_file(ws.dir().join("c.wombat")).unwrap();
    let fifth = checked(&ws);
    assert_eq!(fifth.index.documents().count(), 2);
    assert!(fifth.cache_written);
    assert!(!cache_file(&ws, "c.wombat").exists());
    // A new document is parsed, the others are not.
    let new = doc(&mut SeededIds::from_u64(77), &["new"]);
    ws.save(&new, "new.wombat").unwrap();
    let sixth = checked(&ws);
    assert_eq!((sixth.parsed, sixth.reused), (1, 2));
    // What a failed write left behind is swept away; other files are left alone.
    let dir = scan::cache_dir::<DocLinks>(&cache_of(&ws, CacheMode::Verify));
    fs::write(dir.join("b.wombat.idx.tmp"), "left over").unwrap();
    fs::write(dir.join("readme.txt"), "not ours").unwrap();
    assert!(checked(&ws).cache_written);
    assert!(!dir.join("b.wombat.idx.tmp").exists() && dir.join("readme.txt").exists());
}

#[test]
fn the_fast_cache_goes_by_length_and_time_and_only_for_files_that_have_settled() {
    let (ws, [a, b, _]) = workspace("fast");
    let fast = |ws: &Workspace| checked_with(ws, CacheMode::Fast);
    let first = fast(&ws);
    assert_eq!((first.parsed, first.reused), (3, 0));
    // The documents were written a moment before their entries: a change within the
    // same tick of the clock could go unseen, so they are read and compared, and the
    // entries are written again to say when that was.
    let second = fast(&ws);
    assert_eq!((second.parsed, second.reused), (0, 3));
    assert!(
        second.cache_written,
        "files that had not settled are checked"
    );
    // Such a change is indeed seen.
    change_in_place(&ws, &a, &b);
    let third = fast(&ws);
    assert_eq!((third.parsed, third.reused), (1, 2));
    assert_eq!(third.index.backlinks(b.id(), cell(&b, 0)).len(), 1);

    // Documents last modified well before their entries were written are believed by
    // length and time alone: they are not read, shown here by making them unreadable
    // as documents in a way that keeps both.
    for name in ["a.wombat", "b.wombat", "c.wombat"] {
        age(&ws.dir().join(name), 3600);
    }
    let settle = fast(&ws);
    assert_eq!((settle.parsed, settle.reused), (0, 3));
    assert!(settle.cache_written, "the entries now record the new times");
    let files = cache_files(&ws);
    let fourth = fast(&ws);
    assert_eq!((fourth.parsed, fourth.reused), (0, 3));
    assert!(!fourth.cache_written && cache_files(&ws) == files);

    // The stated limit of the fast way: a file changed with its length and its exact
    // time kept is not noticed. The checking way notices it, and repairs the entry.
    let path = ws.dir().join("c.wombat");
    let time = fs::metadata(&path).unwrap().modified().unwrap();
    let good = fs::read(&path).unwrap();
    fs::write(&path, vec![b'x'; good.len()]).unwrap();
    let file = fs::File::options().write(true).open(&path).unwrap();
    file.set_modified(time).unwrap();
    drop(file);
    // On Unix the file system also says when a file was last changed in any way,
    // which no program can set back: there the fast way sees it too.
    let unseen = scan::link_index(&ws, &cache_of(&ws, CacheMode::Fast)).unwrap();
    let seen_fast = cfg!(unix);
    assert_eq!(
        (unseen.parsed, unseen.reused, unseen.failed.len()),
        if seen_fast { (0, 2, 1) } else { (0, 3, 0) }
    );
    let seen = scan::link_index(&ws, &cache_of(&ws, CacheMode::Verify)).unwrap();
    assert_eq!((seen.reused, seen.failed.len()), (2, 1));
    // Any other change is seen by the fast way: another length, or another time.
    fs::write(&path, &good).unwrap();
    age(&path, 1800);
    let back = fast(&ws);
    assert_eq!((back.parsed, back.reused), (0, 3));
    fs::write(&path, [good.as_slice(), b"\n"].concat()).unwrap();
    age(&path, 1800);
    assert_eq!(fast(&ws).rejected, 1);
    change_in_place(&ws, &a, &b);
    age(&ws.dir().join("a.wombat"), 900);
    let last = fast(&ws);
    assert_eq!(
        (last.parsed, last.reused),
        (0, 3),
        "same bytes, new time: reused"
    );
    assert!(last.cache_written);
}

#[test]
fn a_deleted_or_damaged_cache_is_rebuilt_and_never_believed() {
    let (ws, _) = workspace("damaged");
    let good = checked(&ws);
    let files = cache_files(&ws);
    let path = cache_file(&ws, "b.wombat");
    let bytes = fs::read(&path).unwrap();
    let rebuilt = |what: &str, state: CacheUse, parsed: usize, damage: &dyn Fn()| {
        for mode in [CacheMode::Fast, CacheMode::Verify] {
            damage();
            let links = checked_with(&ws, mode);
            assert_eq!(links.cache, state, "{what}");
            assert_eq!((links.parsed, links.reused), (parsed, 3 - parsed), "{what}");
            assert!(links.index == good.index, "{what}");
            // And it wrote good entries again.
            assert_eq!(cache_files(&ws), files, "{what}");
        }
    };
    // Deleted: one entry, the folder of the kind, or the whole folder.
    rebuilt("deleted file", CacheUse::Read, 1, &|| {
        fs::remove_file(&path).unwrap()
    });
    rebuilt("deleted folder of the kind", CacheUse::Missing, 3, &|| {
        fs::remove_dir_all(path.parent().unwrap()).unwrap()
    });
    rebuilt("deleted folder", CacheUse::Missing, 3, &|| {
        fs::remove_dir_all(ws.dir().with_extension("cache")).unwrap()
    });
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
    let word = |h: &str, n: usize, new: &str| {
        let mut words: Vec<&str> = h.split(' ').collect();
        words[n] = new;
        words.join(" ")
    };
    assert_eq!(header.split(' ').count(), 11);
    let other = fs::read(cache_file(&ws, "a.wombat")).unwrap();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty", Vec::new()),
        ("cut short", bytes[..bytes.len() / 2].to_vec()),
        ("one byte short", bytes[..bytes.len() - 1].to_vec()),
        ("one byte more", [bytes.as_slice(), b" "].concat()),
        ("only a header", bytes[..=header_end].to_vec()),
        ("no header", body.to_vec()),
        ("rubbish", b"\x00\xff rubbish \n {".to_vec()),
        ("not text", vec![0xff; 4096]),
        ("the entry of another document", other),
        ("another kind", with_header(&|h| word(h, 1, "words"))),
        ("another version", with_header(&|h| word(h, 2, "2"))),
        (
            "other rules",
            with_header(&|h| word(h, 3, "0000000000000001")),
        ),
        ("another length", with_header(&|h| word(h, 4, "1"))),
        ("no time", with_header(&|h| word(h, 5, "-"))),
        ("another time", with_header(&|h| word(h, 5, "1"))),
        ("another file", with_header(&|h| word(h, 6, "1"))),
        ("another change time", with_header(&|h| word(h, 7, "1"))),
        (
            "another document checksum",
            with_header(&|h| word(h, 8, &"0".repeat(32))),
        ),
        (
            "another length of the body",
            with_header(&|h| word(h, 9, "1")),
        ),
        (
            "another checksum",
            // (Another last digit, whatever it is: the checksum depends on times now.)
            with_header(&|h| {
                let other = if h.ends_with('0') { '1' } else { '0' };
                format!("{}{other}", &h[..h.len() - 1])
            }),
        ),
        (
            "a body of nothing",
            with_header(&|h| h.to_string())[..=header_end].to_vec(),
        ),
    ];
    for (what, damaged) in cases {
        assert_ne!(damaged, bytes, "{what}");
        // Whatever is changed in the first line, the checksum at its end, which is
        // over all of it, over the name of the document file and over the body, no
        // longer fits. (Before the review of milestone 6 the first line was only
        // compared with itself, and an entry with another time in it was believed.)
        rebuilt(what, CacheUse::Read, 1, &|| {
            fs::write(&path, &damaged).unwrap()
        });
    }
    // Any single byte changed anywhere: never a wrong index.
    for at in 0..bytes.len() {
        let mut damaged = bytes.clone();
        damaged[at] ^= 0x21;
        fs::write(&path, &damaged).unwrap();
        let links = checked(&ws);
        assert!(links.index == good.index, "byte {at} changed");
        assert_eq!(cache_files(&ws), files, "byte {at} changed");
    }
    assert!(checked(&ws).cache == CacheUse::Read);
    // An entry too large to be one is not read.
    let huge = fs::File::create(&path).unwrap();
    huge.set_len(scan::MAX_ENTRY_BYTES + 1).unwrap();
    drop(huge);
    assert_eq!(checked(&ws).rejected, 1);
    assert_eq!(cache_files(&ws), files);
    // A cache in a folder that cannot be written is not an error.
    let top = ws.dir().with_extension("cache");
    fs::remove_dir_all(&top).unwrap();
    fs::write(&top, "a file where the folder should be").unwrap();
    for mode in [CacheMode::Fast, CacheMode::Verify] {
        let links = scan::link_index(&ws, &cache_of(&ws, mode)).unwrap();
        assert!(!links.cache_written && links.index == good.index);
        assert_eq!(links.cache, CacheUse::Unusable);
    }
    assert_eq!(
        fs::read_to_string(&top).unwrap(),
        "a file where the folder should be"
    );
}

/// Found in review of milestone 4: the cache was written through symbolic links.
#[cfg(unix)]
#[test]
fn nothing_is_read_or_written_through_a_symbolic_link() {
    use std::os::unix::fs::symlink;
    let (ws, _) = workspace("symlinks");
    let good = scan::link_index(&ws, &cache_of(&ws, CacheMode::Off)).unwrap();
    let outside = folder("symlinks-outside");
    let precious = outside.join("important.txt");
    let top = ws.dir().with_extension("cache");
    let kind = scan::cache_dir::<DocLinks>(&cache_of(&ws, CacheMode::Verify));
    let entry = cache_file(&ws, "a.wombat");
    let temp = kind.join("a.wombat.idx.tmp");
    // A good entry of another content, to see that a linked entry is not read either.
    let (other, _) = workspace("symlinks-other");
    scan::link_index(&other, &cache_of(&other, CacheMode::Verify)).unwrap();
    let lure = fs::read(cache_file(&other, "b.wombat")).unwrap();
    for (link, to_dir, state) in [
        (&top, true, CacheUse::Unusable),
        (&kind, true, CacheUse::Unusable),
        (&entry, false, CacheUse::Read),
        (&temp, false, CacheUse::Read),
    ] {
        let _ = fs::remove_dir_all(&top);
        let _ = fs::remove_file(&top);
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&outside).unwrap();
        fs::write(&precious, &lure).unwrap();
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink(if to_dir { &outside } else { &precious }, link).unwrap();
        for mode in [CacheMode::Verify, CacheMode::Fast, CacheMode::Verify] {
            let links = scan::link_index(&ws, &cache_of(&ws, mode)).unwrap();
            assert_eq!(links.cache, state, "{}", link.display());
            assert!(links.index == good.index);
            assert_eq!(fs::read(&precious).unwrap(), lure, "{}", link.display());
            assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
        }
        // A link where the temporary file goes is replaced, not followed.
        assert_eq!(
            fs::symlink_metadata(link).is_ok_and(|found| found.is_symlink()),
            link != &temp,
            "{}",
            link.display()
        );
    }
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

/// Found in review of milestone 6: a cache inside the workspace folder was believed,
/// and a workspace can come from anywhere. An entry made to fit a document (its header
/// taken from the document's own entry, its body from another) made a search find what
/// the document no longer said. The cache is no longer in the workspace, and what is
/// found there is not looked at.
#[test]
fn a_cache_inside_the_workspace_is_never_used() {
    let (ws, [a, b, _]) = workspace("planted");
    let good = scan::link_index(&ws, &Cache::off()).unwrap();
    // The old place, with entries that are good ones by every check: made by a real
    // scan of these very documents, then moved to where the cache used to be.
    let outside = cache_of(&ws, CacheMode::Verify);
    scan::link_index(&ws, &outside).unwrap();
    let old = ws.dir().join(scan::OLD_CACHE_DIR);
    fs::create_dir_all(old.join("links")).unwrap();
    for (name, bytes) in cache_files(&ws) {
        fs::write(old.join("links").join(name), bytes).unwrap();
    }
    fs::remove_dir_all(outside.dir()).unwrap();
    let planted = |dir: &Path| {
        let mut files: Vec<_> = fs::read_dir(dir.join("links"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .map(|path| (path.clone(), fs::read(path).unwrap()))
            .collect();
        files.sort();
        files
    };
    let before = planted(&old);
    assert_eq!(before.len(), 3);
    // The documents change; the planted entries still say what they said.
    change_in_place(&ws, &a, &b);
    let now = scan::link_index(&ws, &Cache::off()).unwrap();
    assert!(now.index != good.index);
    for mode in [CacheMode::Verify, CacheMode::Fast] {
        // The cache where it belongs: nothing of the old folder is read.
        let found = scan::link_index(&ws, &cache_of(&ws, mode)).unwrap();
        assert!(found.index == now.index);
        // Asked to use the old folder, or any folder inside the workspace: refused,
        // whether it is there or not.
        for inside in [
            old.clone(),
            ws.dir().join("new-cache"),
            ws.dir().join("a/b/c"),
        ] {
            let found = scan::link_index(&ws, &Cache::at(&inside, mode)).unwrap();
            assert_eq!(found.cache, CacheUse::Unusable, "{}", inside.display());
            assert!(found.index == now.index && found.reused == 0 && !found.cache_written);
            assert_eq!(scan::clear_cache(&Cache::at(&inside, mode), &ws), 0);
        }
        assert!(!ws.dir().join("new-cache").exists() && !ws.dir().join("a").exists());
        // Never read, never written, never deleted.
        assert_eq!(planted(&old), before);
    }
    // A way round through a link is no way round.
    #[cfg(unix)]
    {
        let link = ws.dir().with_extension("link");
        let _ = fs::remove_file(&link);
        std::os::unix::fs::symlink(&old, &link).unwrap();
        let found = scan::link_index(&ws, &Cache::at(&link, CacheMode::Verify)).unwrap();
        assert_eq!(found.cache, CacheUse::Unusable);
        assert!(found.index == now.index);
        fs::remove_file(&link).unwrap();
    }
    assert_eq!(planted(&old), before);
}

/// Each workspace has its own cache folder under the base, named so that a person can
/// tell them apart; an entry is of one document file and of no other; and the whole
/// cache of a workspace can be thrown away.
#[test]
fn caches_are_kept_per_workspace_and_can_be_cleared() {
    let (ws, _) = workspace("per_workspace");
    let (other, _) = workspace("per_workspace_other");
    let base = folder("per_workspace_base");
    let cache = Cache::in_base(&base, &ws, CacheMode::Verify).unwrap();
    let second = Cache::in_base(&base, &other, CacheMode::Verify).unwrap();
    assert_ne!(cache.dir(), second.dir());
    assert_eq!(cache.dir().parent(), Some(base.as_path()));
    let name = cache.dir().file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("per_workspace-") && name.len() == "per_workspace-".len() + 16);
    // The same folder reached another way is the same cache.
    let dotted = Workspace::open(ws.dir().join("../per_workspace")).unwrap();
    assert_eq!(
        Cache::in_base(&base, &dotted, CacheMode::Verify).unwrap(),
        cache
    );
    let first = scan::link_index(&ws, &cache).unwrap();
    assert_eq!((first.parsed, first.cache), (3, CacheUse::Missing));
    scan::link_index(&other, &second).unwrap();
    let again = scan::link_index(&ws, &cache).unwrap();
    assert_eq!(
        (again.parsed, again.reused, again.cache),
        (0, 3, CacheUse::Read)
    );
    // What the folder is the cache of is written in it.
    let marker = fs::read_to_string(cache.dir().join(scan::MARKER)).unwrap();
    assert!(marker.contains("per_workspace"), "{marker}");
    // An entry under another document's name is not that document's entry, although
    // it is whole and its own checksum is right.
    let a = scan::cache_path::<DocLinks>(&cache, "a.wombat");
    let b = scan::cache_path::<DocLinks>(&cache, "b.wombat");
    let entry = fs::read(&b).unwrap();
    fs::copy(&a, &b).unwrap();
    let mixed = scan::link_index(&ws, &cache).unwrap();
    assert_eq!((mixed.parsed, mixed.reused, mixed.rejected), (1, 2, 1));
    assert!(mixed.index == first.index);
    assert_eq!(fs::read(&b).unwrap(), entry, "written again as it was");
    // "Rebuild the index": everything of this workspace goes, nothing of the other.
    fs::write(cache.dir().join("links").join("notes.txt"), "not ours").unwrap();
    assert_eq!(scan::clear_cache(&cache, &ws), 3);
    assert!(!a.exists() && cache.dir().join("links").join("notes.txt").exists());
    assert!(scan::cache_path::<DocLinks>(&second, "a.wombat").exists());
    let rebuilt = scan::link_index(&ws, &cache).unwrap();
    assert_eq!((rebuilt.parsed, rebuilt.reused), (3, 0));
    assert!(rebuilt.index == first.index);
    fs::remove_file(cache.dir().join("links").join("notes.txt")).unwrap();
    assert_eq!(scan::clear_cache(&cache, &ws), 3);
    assert!(!cache.dir().exists(), "an empty cache folder is removed");
    assert_eq!(scan::clear_cache(&Cache::off(), &ws), 0);
}

/// Found in review of milestone 6: for two seconds after documents were saved, every
/// fast scan wrote every entry again, to no effect (an entry written so soon after its
/// file is not believed by length and time anyway). Told the time, the scan leaves them
/// until writing them again can help.
#[test]
fn the_fast_way_leaves_entries_alone_that_are_too_new_to_settle() {
    let (ws, _) = workspace("too_new");
    let clock = || {
        let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
        since.unwrap().as_nanos()
    };
    let fast = |now: u128| {
        let cache = cache_of(&ws, CacheMode::Fast).with_now(now);
        scan::link_index(&ws, &cache).unwrap()
    };
    assert_eq!(fast(clock()).parsed, 3);
    let files = cache_files(&ws);
    for _ in 0..2 {
        let again = fast(clock());
        assert_eq!((again.parsed, again.reused), (0, 3), "read and compared");
        assert!(!again.cache_written, "and nothing written");
        assert_eq!(cache_files(&ws), files);
    }
    // Ten seconds on by the caller's clock, writing them again is worth it.
    assert!(fast(clock() + 10_000_000_000).cache_written);
    // Not told the time, the scan writes them, as it always did.
    let untold = scan::link_index(&ws, &cache_of(&ws, CacheMode::Fast)).unwrap();
    assert!(untold.cache_written);
}
