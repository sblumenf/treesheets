//! The search index of a workspace folder and its cache on disk: whatever happens to the
//! documents or to the cache, the index is the one that reading every document gives.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "support/corpus.rs"]
mod corpus;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use wombat_core::eval::Calc;
use wombat_core::scan::{self, Cache, CacheMode, CacheUse};
use wombat_core::summary::Summary;
use wombat_core::{Command, Document, Editor, SeededIds, Workspace};
use wombat_search::query::parse;
use wombat_search::workspace::{index, WorkspaceIndex};
use wombat_search::{naive, DocIndex, DocSearch, Options};

fn folder(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("search_workspace")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(dir.with_extension("cache"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The cache of a workspace of these tests: a folder beside it, never inside it.
fn cache_of(ws: &Workspace, mode: CacheMode) -> Cache {
    Cache::at(ws.dir().with_extension("cache"), mode)
}

/// Three documents of made-up text, saved.
fn workspace(test: &str) -> (Workspace, Vec<(String, Document)>) {
    let ws = Workspace::open(folder(test)).unwrap();
    let mut corpus = corpus::Corpus::new(11);
    let mut docs = Vec::new();
    for name in ["a.wombat", "b.wombat", "c.wombat"] {
        let doc = corpus.document(120);
        ws.save(&doc, name).unwrap();
        docs.push((name.to_owned(), doc));
    }
    (ws, docs)
}

const QUERIES: [&str; 8] = [
    "a",
    "ta",
    "tag:todo",
    "has:note in",
    "的",
    "edited:300d",
    "has:link",
    "",
];

/// The index with the cache, which must be the index without it, and must answer as
/// the reference scanner does on the documents as they are in the folder.
fn checked(ws: &Workspace, mode: CacheMode) -> WorkspaceIndex {
    let plain = index(ws, &cache_of(ws, CacheMode::Off)).unwrap();
    assert_eq!(plain.cache, CacheUse::Off);
    assert!(!plain.cache_written && plain.reused == 0);
    let cached = index(ws, &cache_of(ws, mode)).unwrap();
    assert!(
        cached.index.encoded() == plain.index.encoded(),
        "the cache gave another index"
    );
    cached.index.check().unwrap();
    assert_eq!(cached.names, plain.names);
    assert_eq!(cached.duplicates, plain.duplicates);
    assert_eq!(cached.failed.len(), plain.failed.len());
    let docs: Vec<(String, Document)> = plain
        .names
        .values()
        .map(|name| (name.clone(), ws.open_name(name).unwrap()))
        .collect();
    let named: Vec<(&str, &Document)> = docs.iter().map(|(n, d)| (n.as_str(), d)).collect();
    let calcs: Vec<Calc> = named.iter().map(|(_, doc)| Calc::new(doc)).collect();
    let named: Vec<naive::Named> = named
        .iter()
        .zip(&calcs)
        .map(|((n, d), c)| (*n, *d, c))
        .collect();
    let reference = naive::Reference::new(&named);
    for text in QUERIES {
        let parsed = parse(text, corpus::NOW).unwrap();
        let options = Options::default();
        let found = cached.index.search(&parsed.query, &options).unwrap();
        assert_eq!(
            found,
            reference.search(&parsed.query, &options).unwrap(),
            "{text}"
        );
    }
    cached
}

fn cache_files(ws: &Workspace) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = fs::read_dir(scan::cache_dir::<DocIndex>(&cache_of(
        ws,
        CacheMode::Verify,
    )))
    .unwrap()
    .map(|entry| {
        let entry = entry.unwrap();
        (
            entry.file_name().into_string().unwrap(),
            fs::read(entry.path()).unwrap(),
        )
    })
    .collect();
    files.sort();
    files
}

fn age(path: &Path, seconds: u64) {
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - Duration::from_secs(seconds))
        .unwrap();
}

#[test]
fn the_workspace_index_covers_documents_that_are_not_open_and_follows_the_folder() {
    let (ws, docs) = workspace("folder");
    let first = checked(&ws, CacheMode::Verify);
    assert_eq!(first.cache, CacheUse::Missing);
    assert!(first.cache_written && first.parsed == 3 && first.reused == 0);
    let files = cache_files(&ws);
    let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["a.wombat.idx", "b.wombat.idx", "c.wombat.idx"]);
    // An entry is smaller than its document.
    for (name, bytes) in &files {
        let doc = fs::metadata(ws.dir().join(name.trim_end_matches(".idx")))
            .unwrap()
            .len();
        assert!(
            (bytes.len() as u64) < doc,
            "{name}: {} against {doc}",
            bytes.len()
        );
    }
    // Nothing changed: all from the cache, which is not written again.
    let second = checked(&ws, CacheMode::Verify);
    assert_eq!(
        (second.cache, second.parsed, second.reused),
        (CacheUse::Read, 0, 3)
    );
    assert!(!second.cache_written && cache_files(&ws) == files);

    // A document is edited with the index following, and saved: the index is the one
    // that the folder gives, and only that document's entry is made again.
    let mut live = second.index;
    let (name, doc) = &docs[1];
    let mut editor = Editor::new(doc.clone()).unwrap();
    let mut ids = SeededIds::from_u64(8);
    let cells = doc.cells_in_order();
    let commands = [
        Command::SetText {
            cell: cells[5],
            text: "Something new 新しい".into(),
        },
        Command::InsertRows {
            cell: doc.root(),
            at: 0,
            count: 2,
        },
        Command::DeleteRows {
            cell: doc.root(),
            at: 0,
            count: 1,
        },
    ];
    for command in commands {
        let changes = editor.apply(command, Some(corpus::NOW), &mut ids).unwrap();
        live.apply(editor.document(), editor.calc(), &changes)
            .unwrap();
    }
    ws.save(editor.document(), name).unwrap();
    let third = checked(&ws, CacheMode::Verify);
    assert_eq!((third.parsed, third.reused), (1, 2));
    assert!(third.index.encoded() == live.encoded());
    let now = cache_files(&ws);
    assert!(now[0] == files[0] && now[1] != files[1] && now[2] == files[2]);

    // Renamed, deleted, added.
    fs::rename(ws.dir().join("a.wombat"), ws.dir().join("z.wombat")).unwrap();
    fs::remove_file(ws.dir().join("c.wombat")).unwrap();
    ws.save(&corpus::Corpus::new(5).document(40), "new.wombat")
        .unwrap();
    let fourth = checked(&ws, CacheMode::Verify);
    assert_eq!((fourth.parsed, fourth.reused), (2, 1));
    assert_eq!(fourth.index.documents().count(), 3);
    assert_eq!(fourth.index.name(docs[0].1.id()), Some("z.wombat"));
    let names: Vec<String> = cache_files(&ws).into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["b.wombat.idx", "new.wombat.idx", "z.wombat.idx"]);

    // The fast way: by length and time, once the files have settled.
    for name in ["b.wombat", "new.wombat", "z.wombat"] {
        age(&ws.dir().join(name), 3600);
    }
    let settle = checked(&ws, CacheMode::Fast);
    assert_eq!((settle.parsed, settle.reused), (0, 3));
    let fast = checked(&ws, CacheMode::Fast);
    assert_eq!((fast.parsed, fast.reused), (0, 3));
    assert!(!fast.cache_written);
    // A file that is another length, or has another time, is read again.
    ws.save(&corpus::Corpus::new(9).document(30), "b.wombat")
        .unwrap();
    age(&ws.dir().join("b.wombat"), 1800);
    let changed = checked(&ws, CacheMode::Fast);
    assert_eq!((changed.parsed, changed.reused), (1, 2));

    // Damaged documents and copies made by hand are reported.
    fs::copy(ws.dir().join("z.wombat"), ws.dir().join("z copy.wombat")).unwrap();
    fs::write(ws.dir().join("broken.wombat"), "{ not a document").unwrap();
    let odd = checked(&ws, CacheMode::Verify);
    assert_eq!(odd.failed.len(), 1);
    assert_eq!(
        odd.duplicates[&docs[0].1.id()],
        ["z copy.wombat".to_string(), "z.wombat".to_string()]
    );
    assert_eq!(odd.index.documents().count(), 3);
}

#[test]
fn a_deleted_or_damaged_search_cache_is_rebuilt_and_never_believed() {
    let (ws, _) = workspace("damaged");
    let good = checked(&ws, CacheMode::Verify);
    let files = cache_files(&ws);
    let path = scan::cache_path::<DocIndex>(&cache_of(&ws, CacheMode::Verify), "b.wombat");
    let bytes = fs::read(&path).unwrap();
    let expected = good.index.encoded();
    let rebuilt = |what: &str, damage: &dyn Fn()| {
        for mode in [CacheMode::Fast, CacheMode::Verify] {
            damage();
            let found = index(&ws, &cache_of(&ws, mode)).unwrap();
            assert!(found.index.encoded() == expected, "{what}");
            found.index.check().unwrap();
            assert_eq!(cache_files(&ws), files, "{what}: no good cache again");
        }
    };
    rebuilt("deleted entry", &|| fs::remove_file(&path).unwrap());
    rebuilt("deleted folder", &|| {
        fs::remove_dir_all(ws.dir().with_extension("cache")).unwrap()
    });
    rebuilt("empty", &|| fs::write(&path, b"").unwrap());
    rebuilt("cut short", &|| {
        fs::write(&path, &bytes[..bytes.len() / 2]).unwrap()
    });
    rebuilt("one byte more", &|| {
        fs::write(&path, [bytes.as_slice(), b"x"].concat()).unwrap()
    });
    let other = fs::read(scan::cache_path::<DocIndex>(
        &cache_of(&ws, CacheMode::Verify),
        "a.wombat",
    ))
    .unwrap();
    rebuilt("the entry of another document", &|| {
        fs::write(&path, &other).unwrap()
    });
    // The first line names the rules by which the index was made: an entry made under
    // other rules, or by another version, is not believed.
    let header_end = bytes.iter().position(|b| *b == b'\n').unwrap();
    let header = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
    let words: Vec<&str> = header.split(' ').collect();
    assert_eq!(words[1], "search");
    assert_eq!(words[2], DocIndex::VERSION.to_string());
    assert_eq!(words[3], format!("{:016x}", DocIndex::fingerprint()));
    for (n, new) in [(2, "999"), (3, "00000000deadbeef")] {
        let mut changed = words.clone();
        changed[n] = new;
        let mut damaged = changed.join(" ").into_bytes();
        damaged.extend_from_slice(&bytes[header_end..]);
        rebuilt("other rules", &|| fs::write(&path, &damaged).unwrap());
    }
    // Any single byte changed: never a wrong index. (Every 29th byte, in turn.)
    for at in (0..bytes.len()).step_by(29) {
        let mut damaged = bytes.clone();
        damaged[at] ^= 0x21;
        fs::write(&path, &damaged).unwrap();
        let found = index(&ws, &cache_of(&ws, CacheMode::Verify)).unwrap();
        assert!(found.index.encoded() == expected, "byte {at} changed");
        assert_eq!(found.parsed, 1, "byte {at} changed");
    }
    assert_eq!(cache_files(&ws), files);
}

/// The summary of a document as bytes: read back to the same, and anything else refused
/// or, exactly, the bytes of another summary.
#[test]
fn a_summary_reads_back_and_bytes_that_are_not_one_are_refused() {
    let doc = corpus::Corpus::new(3).document(60);
    let summary = DocSearch::of(&doc);
    let good = summary.encode();
    let back = DocSearch::decode(&good).unwrap();
    assert_eq!(back.encode(), good);
    back.check().unwrap();
    assert_eq!(DocSearch::decode(&[]).map(|s| s.cell_count()), None);
    for cut in (0..good.len()).step_by(3) {
        assert!(DocSearch::decode(&good[..cut]).is_none(), "cut at {cut}");
    }
    assert!(DocSearch::decode(&[good.as_slice(), &[0]].concat()).is_none());
    let mut refused = 0;
    for at in (0..good.len()).step_by(5) {
        for flip in [0x01, 0x80] {
            let mut bytes = good.clone();
            bytes[at] ^= flip;
            match DocSearch::decode(&bytes) {
                None => refused += 1,
                Some(other) => {
                    other.check().unwrap();
                    assert!(other.encode() == bytes, "byte {at} ^ {flip}");
                }
            }
        }
    }
    assert!(refused > 20, "{refused}");
    // A count that the bytes could not hold is refused before anything is allocated.
    let mut huge = good[..16].to_vec();
    huge.extend_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0x0f]);
    assert!(DocSearch::decode(&huge).is_none());
    // The two parts of a document's summary must be of the same document.
    let both = DocIndex::of(&doc);
    let other = DocIndex::of(&corpus::Corpus::new(4).document(60));
    let mixed = DocIndex {
        search: both.search.clone(),
        links: other.links,
    };
    assert!(DocIndex::decode(&both.encode()).is_some());
    assert!(DocIndex::decode(&mixed.encode()).is_none());
}

/// The rules of folding are part of what a cache entry is believed by. This pins them:
/// if this fails because folding was changed on purpose, old caches are already
/// refused (the number is in every entry's first line); put the new number here.
#[test]
fn the_rules_of_folding_are_pinned() {
    assert_eq!(
        wombat_search::fold::fingerprint(),
        PINNED,
        "{:#x}",
        wombat_search::fold::fingerprint()
    );
}

const PINNED: u64 = 0x2c76_9938_d1d6_b80d;

/// Replacing across a folder prepares every document before it saves any; when saving
/// one fails, it stops and says which files were changed, which one failed and which
/// were not reached. (In the tool, a failure in a later document used to return after
/// the earlier ones were saved, and the list of what was changed was lost.)
#[test]
fn a_replace_across_a_folder_says_what_was_saved_when_saving_fails() {
    use wombat_core::construct::Construct;
    use wombat_core::{storage, CellKind, Error};
    use wombat_search::query::Term;
    use wombat_search::replace::ReplaceOptions;
    use wombat_search::workspace::replace_in_folder;

    let ws = Workspace::open(folder("replace_folder")).unwrap();
    let mut ids = SeededIds::from_u64(5);
    let names = ["a.wombat", "b.wombat", "c.wombat", "d.wombat", "e.wombat"];
    for (n, name) in names.iter().enumerate() {
        let mut doc = Document::new(&mut ids);
        let root = doc.root();
        // d has the word in a formula only, e not at all.
        doc.cell_mut(root).unwrap().text = match n {
            3 => "=apple".to_owned(),
            4 => "pear".to_owned(),
            _ => format!("apple {n}{n}"),
        };
        if n == 3 {
            doc.cell_mut(root).unwrap().kind = CellKind::Formula;
        }
        ws.save(&doc, name).unwrap();
    }
    fs::write(ws.dir().join("zz.wombat"), "not a document").unwrap();
    let before: Vec<Vec<u8>> = names
        .iter()
        .map(|name| fs::read(ws.dir().join(name)).unwrap())
        .collect();
    let read = |name: &str| ws.open_name(name).unwrap();
    let text = |name: &str| read(name).cell(read(name).root()).unwrap().text.clone();
    let mut targets: Vec<(String, Option<wombat_core::CellId>)> =
        names.iter().map(|name| (name.to_string(), None)).collect();
    targets.push(("zz.wombat".into(), None));
    let term = Term::new("apple");
    let options = ReplaceOptions::default();

    // A dry run saves nothing and says what would happen.
    let mut never = |_: &Document, _: &Path| -> Result<(), Error> { panic!("a dry run saves") };
    let dry = replace_in_folder(
        &ws, &targets, &term, "fig", options, true, None, &mut ids, &mut never,
    )
    .unwrap();
    let found: Vec<(&str, usize, usize)> = dry
        .files
        .iter()
        .map(|file| {
            (
                file.name.as_str(),
                file.counts.places,
                file.counts.formula_places,
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("a.wombat", 1, 0),
            ("b.wombat", 1, 0),
            ("c.wombat", 1, 0),
            ("d.wombat", 0, 1)
        ]
    );
    assert!(dry.saved.is_empty() && dry.failed.is_none() && dry.not_saved.is_empty());
    assert_eq!(dry.unreadable.len(), 1);

    // A refusal in a later document: nothing at all is saved.
    let mut saved = 0;
    let mut count = |doc: &Document, path: &Path| {
        saved += 1;
        storage::save_document(doc, path)
    };
    let refused = replace_in_folder(
        &ws,
        &targets,
        &Term::new("1"),
        "\u{301}",
        options,
        false,
        None,
        &mut ids,
        &mut count,
    );
    let (name, error) = refused.unwrap_err();
    assert_eq!(name, "b.wombat");
    assert!(matches!(error, Error::Refused(_)), "{error:?}");
    assert_eq!(saved, 0);

    // Saving the second document fails: the first is changed, the second and third are
    // as they were, and the result says exactly that.
    let mut calls = 0;
    let mut failing = |doc: &Document, path: &Path| {
        calls += 1;
        if calls == 2 {
            return Err(Error::Refused("the disk is full".into()));
        }
        storage::save_document(doc, path)
    };
    let done = replace_in_folder(
        &ws,
        &targets,
        &term,
        "fig",
        options,
        false,
        Some(9),
        &mut ids,
        &mut failing,
    )
    .unwrap();
    assert_eq!(done.saved, ["a.wombat"]);
    assert_eq!(
        done.failed.as_ref().map(|(name, _)| name.as_str()),
        Some("b.wombat")
    );
    assert_eq!(done.not_saved, ["c.wombat"]);
    assert_eq!(calls, 2, "nothing is written after a failure");
    assert_eq!(text("a.wombat"), "fig 00");
    for n in 1..5 {
        assert_eq!(
            fs::read(ws.dir().join(names[n])).unwrap(),
            before[n],
            "{}",
            names[n]
        );
    }
    // Run again without the failure: the rest is done.
    let done = replace_in_folder(
        &ws,
        &targets,
        &term,
        "fig",
        options,
        false,
        None,
        &mut ids,
        &mut storage::save_document,
    )
    .unwrap();
    assert_eq!(done.saved, ["b.wombat", "c.wombat"]);
    assert_eq!(
        (text("b.wombat"), text("d.wombat")),
        ("fig 11".into(), "=apple".into())
    );
}

/// The review's reproduction: the document fields of the current entry with the body of
/// an older one, in a cache inside the workspace, made `apple` be found in a document
/// that now says `pears`, also when every file was checked. A cache in the workspace is
/// no longer looked at, so the entry cannot be planted where it would be read; and an
/// entry put together like that is not believed anywhere, because its checksum is over
/// all of it.
#[test]
fn a_planted_entry_is_not_believed() {
    use wombat_core::construct::Construct;
    let ws = Workspace::open(folder("planted")).unwrap();
    let mut ids = SeededIds::from_u64(21);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    doc.cell_mut(root).unwrap().text = "apple".into();
    ws.save(&doc, "fruit.wombat").unwrap();
    let cache = cache_of(&ws, CacheMode::Verify);
    let count = |cache: &Cache, word: &str| {
        let built = index(&ws, cache).unwrap();
        let query = parse(word, 0).unwrap().query;
        let total = built
            .index
            .search(&query, &Options::default())
            .unwrap()
            .total;
        (total, built.reused, built.cache)
    };
    assert_eq!(count(&cache, "apple"), (1, 0, CacheUse::Missing));
    let entry = scan::cache_path::<DocIndex>(&cache, "fruit.wombat");
    let old = fs::read(&entry).unwrap();
    // The document now says pears, in the same number of bytes.
    doc.cell_mut(root).unwrap().text = "pears".into();
    ws.save(&doc, "fruit.wombat").unwrap();
    assert_eq!(count(&cache, "pears"), (1, 0, CacheUse::Read));
    let new = fs::read(&entry).unwrap();
    // The planted entry: the first line of the new one, the body of the old one.
    let line = |bytes: &[u8]| bytes.iter().position(|b| *b == b'\n').unwrap() + 1;
    let planted = [&new[..line(&new)], &old[line(&old)..]].concat();
    assert_eq!(planted.len(), new.len());
    // In the workspace, where the cache used to be: not read, not touched.
    let inside = ws.dir().join(scan::OLD_CACHE_DIR).join("search");
    fs::create_dir_all(&inside).unwrap();
    fs::write(inside.join("fruit.wombat.idx"), &planted).unwrap();
    for mode in [CacheMode::Verify, CacheMode::Fast] {
        let cache = cache_of(&ws, mode);
        assert_eq!(count(&cache, "apple").0, 0);
        assert_eq!(count(&cache, "pears").0, 1);
        let pointed = Cache::at(ws.dir().join(scan::OLD_CACHE_DIR), mode);
        assert_eq!(count(&pointed, "apple"), (0, 0, CacheUse::Unusable));
        assert_eq!(count(&pointed, "pears"), (1, 0, CacheUse::Unusable));
    }
    assert_eq!(fs::read(inside.join("fruit.wombat.idx")).unwrap(), planted);
    // In the real cache (which only the user can write to): refused, by its checksum.
    for mode in [CacheMode::Verify, CacheMode::Fast] {
        fs::write(&entry, &planted).unwrap();
        let cache = cache_of(&ws, mode);
        assert_eq!(count(&cache, "apple"), (0, 0, CacheUse::Read));
        assert_eq!(fs::read(&entry).unwrap(), new, "and made good again");
    }
}

/// Replace across a folder saves through the storage layer's one way of writing a
/// document (`storage::save_document`). A document that is a symbolic link is the user's
/// (decision A27): the file it leads to is replaced whole and the link stays. A link that
/// leads nowhere, or round in a circle, is no document: it is not read, nothing is
/// written through it, and saving to it is refused.
#[cfg(unix)]
#[test]
fn a_replace_across_a_folder_follows_a_linked_document_and_no_broken_link() {
    use std::os::unix::fs::symlink;
    use wombat_core::construct::Construct;
    use wombat_core::storage;
    use wombat_search::query::Term;
    use wombat_search::replace::ReplaceOptions;
    use wombat_search::workspace::replace_in_folder;

    let ws = Workspace::open(folder("replace_links")).unwrap();
    let outside = folder("replace_links_outside");
    let mut ids = SeededIds::from_u64(5);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    doc.cell_mut(root).unwrap().text = "apple".into();
    let real = outside.join("real.wombat");
    storage::save_document(&doc, &real).unwrap();
    symlink(&real, ws.dir().join("linked.wombat")).unwrap();
    symlink(
        outside.join("nothing.wombat"),
        ws.dir().join("dangling.wombat"),
    )
    .unwrap();
    symlink(ws.dir().join("loop.wombat"), ws.dir().join("loop.wombat")).unwrap();
    let targets: Vec<(String, Option<wombat_core::CellId>)> =
        ["dangling.wombat", "linked.wombat", "loop.wombat"]
            .iter()
            .map(|name| (name.to_string(), None))
            .collect();
    let done = replace_in_folder(
        &ws,
        &targets,
        &Term::new("apple"),
        "fig",
        ReplaceOptions::default(),
        false,
        None,
        &mut ids,
        &mut storage::save_document,
    )
    .unwrap();
    assert_eq!(done.saved, ["linked.wombat"]);
    let unreadable: Vec<&str> = done
        .unreadable
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(unreadable, ["dangling.wombat", "loop.wombat"]);
    // The real file has the change, whole; the link is still a link; nothing else was
    // made beside either, and nothing where the dangling link points.
    let saved = storage::load_document(&real).unwrap();
    assert_eq!(saved.cell(saved.root()).unwrap().text, "fig");
    for name in ["linked.wombat", "dangling.wombat", "loop.wombat"] {
        let found = fs::symlink_metadata(ws.dir().join(name)).unwrap();
        assert!(found.file_type().is_symlink(), "{name}");
    }
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
    assert_eq!(fs::read_dir(ws.dir()).unwrap().count(), 3);
    // Saving to such a link is refused outright.
    for name in ["dangling.wombat", "loop.wombat"] {
        assert!(
            storage::save_document(&doc, &ws.dir().join(name)).is_err(),
            "{name}"
        );
    }
    assert!(!outside.join("nothing.wombat").exists());
}
