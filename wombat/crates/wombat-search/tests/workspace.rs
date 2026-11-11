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
use wombat_core::scan::{self, CacheMode, CacheUse};
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
    fs::create_dir_all(&dir).unwrap();
    dir
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
    let plain = index(ws, CacheMode::Off).unwrap();
    assert_eq!(plain.cache, CacheUse::Off);
    assert!(!plain.cache_written && plain.reused == 0);
    let cached = index(ws, mode).unwrap();
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
    let mut files: Vec<_> = fs::read_dir(scan::cache_dir::<DocIndex>(ws))
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
        live.apply(editor.document(), editor.calc(), &changes);
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
    let path = scan::cache_path::<DocIndex>(&ws, "b.wombat");
    let bytes = fs::read(&path).unwrap();
    let expected = good.index.encoded();
    let rebuilt = |what: &str, damage: &dyn Fn()| {
        for mode in [CacheMode::Fast, CacheMode::Verify] {
            damage();
            let found = index(&ws, mode).unwrap();
            assert!(found.index.encoded() == expected, "{what}");
            found.index.check().unwrap();
            assert_eq!(cache_files(&ws), files, "{what}: no good cache again");
        }
    };
    rebuilt("deleted entry", &|| fs::remove_file(&path).unwrap());
    rebuilt("deleted folder", &|| {
        fs::remove_dir_all(ws.dir().join(scan::CACHE_DIR)).unwrap()
    });
    rebuilt("empty", &|| fs::write(&path, b"").unwrap());
    rebuilt("cut short", &|| {
        fs::write(&path, &bytes[..bytes.len() / 2]).unwrap()
    });
    rebuilt("one byte more", &|| {
        fs::write(&path, [bytes.as_slice(), b"x"].concat()).unwrap()
    });
    let other = fs::read(scan::cache_path::<DocIndex>(&ws, "a.wombat")).unwrap();
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
        let found = index(&ws, CacheMode::Verify).unwrap();
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
