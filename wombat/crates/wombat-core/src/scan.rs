//! Indexes over a workspace folder, with a cache on disk (decisions A53, A54 and A71).
//!
//! [`scan`] gives one [`Summary`] per document of a workspace. It reads the documents one
//! at a time, so memory is that of one document and the summaries, never of all
//! documents. With a cache, a document whose file is the one the cache was made from is
//! not parsed again: its summary is read from the cache.
//!
//! The cache is one file per document, `.wombat-index/<kind>/<document file name>.idx` in
//! the workspace folder, so a changed document costs the rewriting of its own entry only.
//! It is a disposable copy (decision A13): deleting it costs time only, and anything
//! doubtful about an entry (cut short, changed, of another version or of other rules, not
//! what its own checksum says, not of this file) makes it count as missing. Whether the
//! document file is still the one an entry was made from is decided in one of two ways
//! ([`CacheMode`]): by reading the file and comparing a checksum of its bytes, or, faster,
//! by its length and modification time alone.
//!
//! Nothing here follows a symbolic link: the cache folders and files must be real folders
//! and files, or the cache is neither read nor written, so that a link planted in a folder
//! that came from elsewhere cannot make a scan overwrite a file outside it. Like
//! `storage`, this module belongs to the file layer.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::error::Error;
use crate::id::DocId;
use crate::limits::MAX_FILE_BYTES;
use crate::links::{DocLinks, LinkIndex};
use crate::storage;
use crate::summary::{digest, Summary};
use crate::workspace::Workspace;

/// The folder, inside a workspace folder, that holds index caches.
pub const CACHE_DIR: &str = ".wombat-index";
/// A cache entry larger than this is neither read nor written: twice the largest
/// document file. (A summary is smaller than its document.) Reading an entry takes memory
/// in proportion to its size ([`Summary::decode`]), so this bounds it.
pub const MAX_ENTRY_BYTES: u64 = 2 * MAX_FILE_BYTES;
/// In [`CacheMode::Fast`], a document file is believed unchanged by its length and time
/// only if it was last modified at least this long before its cache entry was written
/// (2 seconds: the coarsest time stamps of common file systems). A file modified again
/// within the same tick of the clock as the entry was written would otherwise go unseen.
pub const RACY_NANOS: u128 = 2_000_000_000;

/// Whether and how [`scan`] uses the cache.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheMode {
    /// No cache is read or written.
    Off,
    /// A document file with the length and modification time that its cache entry
    /// records is not read at all. A file that was changed and then given its old
    /// length and its old time back exactly is not noticed; everything else is.
    Fast,
    /// Every document file is read and a checksum of its bytes compared with the entry:
    /// any change is noticed, whatever the file's time says.
    Verify,
}

/// One document file of a workspace with its summary.
#[derive(Debug)]
pub struct Scanned<S> {
    /// The file name.
    pub name: String,
    /// The summary, or why the document could not be read.
    pub summary: Result<S, Error>,
    /// Whether the summary came from the cache instead of from parsing the document.
    pub cached: bool,
}

/// What became of the cache when a scan looked for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheUse {
    /// The scan was asked not to use a cache.
    Off,
    /// There was no cache folder for this kind of index.
    Missing,
    /// What is where the cache folder should be is not a real folder (a file, or a
    /// symbolic link): no cache was read or written.
    Unusable,
    /// There was a cache folder; its entries were looked at one by one.
    Read,
}

/// The result of [`scan`].
#[derive(Debug)]
pub struct Scan<S> {
    /// The document files, sorted by name.
    pub files: Vec<Scanned<S>>,
    /// What became of the cache.
    pub cache: CacheUse,
    /// Whether any cache entry was written or removed. (None is when nothing changed, and
    /// failing to write one, in a folder that cannot be written to, is not an error.)
    pub cache_written: bool,
    /// How many cache entries were there but could not be believed (damaged, of another
    /// version, or of a document file that has changed).
    pub rejected: usize,
}

impl<S> Scan<S> {
    /// How many documents were parsed.
    pub fn parsed(&self) -> usize {
        let fresh = |file: &&Scanned<S>| file.summary.is_ok() && !file.cached;
        self.files.iter().filter(fresh).count()
    }

    /// How many summaries came from the cache.
    pub fn reused(&self) -> usize {
        self.files.iter().filter(|file| file.cached).count()
    }
}

/// The folder of the cache entries of the index kind `S` in a workspace.
pub fn cache_dir<S: Summary>(workspace: &Workspace) -> PathBuf {
    workspace.dir().join(CACHE_DIR).join(S::KIND)
}

/// The path of the cache entry of the document file `name`.
pub fn cache_path<S: Summary>(workspace: &Workspace, name: &str) -> PathBuf {
    cache_dir::<S>(workspace).join(format!("{name}.idx"))
}

/// What is at a path, without following a link.
#[derive(PartialEq)]
enum Found {
    Nothing,
    Dir,
    File,
    Other,
}

fn found(path: &Path) -> Found {
    // `symlink_metadata` describes a link itself, so a link is neither a folder nor a file.
    match fs::symlink_metadata(path) {
        Err(_) => Found::Nothing,
        Ok(meta) if meta.is_dir() => Found::Dir,
        Ok(meta) if meta.is_file() => Found::File,
        Ok(_) => Found::Other,
    }
}

/// Length and modification time (nanoseconds since 1970) of what is at a path.
fn stamp(path: &Path) -> Option<(u64, Option<u128>)> {
    let meta = fs::metadata(path).ok()?;
    let time = meta.modified().ok();
    let nanos = time.and_then(|time| Some(time.duration_since(UNIX_EPOCH).ok()?.as_nanos()));
    Some((meta.len(), nanos))
}

/// The first line of an entry: everything that must be as expected for the entry to be
/// believed, so an entry is good exactly if its first line is the one this version would
/// write for its body and its document file.
fn header<S: Summary>(
    doc_len: u64,
    doc_time: Option<u128>,
    doc_digest: u128,
    body: &[u8],
) -> String {
    let time = doc_time.map_or("-".to_owned(), |time| time.to_string());
    format!(
        "wombat-index {} {} {:016x} {doc_len} {time} {doc_digest:032x} {} {:032x}\n",
        S::KIND,
        S::VERSION,
        S::fingerprint(),
        body.len(),
        digest(body)
    )
}

/// A cache entry that is whole: what it says about its document file, and its body.
struct Entry {
    doc_len: u64,
    doc_time: Option<u128>,
    doc_digest: u128,
    body_at: usize,
    bytes: Vec<u8>,
}

impl Entry {
    fn body(&self) -> &[u8] {
        self.bytes.get(self.body_at..).unwrap_or_default()
    }
}

/// Reads the entry at `path`, which must be a real file; `None` unless every check passes.
fn read_entry<S: Summary>(path: &Path) -> Option<Entry> {
    let bytes = storage::read_up_to(path, MAX_ENTRY_BYTES).ok()?;
    let line_end = bytes.iter().position(|b| *b == b'\n')?;
    let body_at = line_end.checked_add(1)?;
    let (first, body) = bytes.split_at_checked(body_at)?;
    let mut words = std::str::from_utf8(first).ok()?.split(' ').skip(4);
    let doc_len = words.next()?.parse().ok()?;
    let doc_time = match words.next()? {
        "-" => None,
        time => Some(time.parse().ok()?),
    };
    let doc_digest = u128::from_str_radix(words.next()?, 16).ok()?;
    // The first line must be exactly what this version would write for this body.
    if first != header::<S>(doc_len, doc_time, doc_digest, body).as_bytes() {
        return None;
    }
    Some(Entry {
        doc_len,
        doc_time,
        doc_digest,
        body_at,
        bytes,
    })
}

/// Writes an entry: to a new temporary file that is then renamed over `path`. Nothing is
/// opened through a link (the temporary file is created new, and a rename replaces a link
/// instead of following it). Not synced to disk: a cache entry that is cut short by a
/// crash fails its checksum and is made again.
fn write_entry(path: &Path, first: &str, body: &[u8]) -> bool {
    if found(path) == Found::Dir || found(path) == Found::Other {
        return false;
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let temp = path.with_file_name(format!("{name}.tmp"));
    let _ = fs::remove_file(&temp);
    let written = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(first.as_bytes())?;
        file.write_all(body)?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    written.is_ok()
}

/// Makes sure the cache folder of `S` is there, as real folders; `false` if it cannot be.
fn make_cache_dir<S: Summary>(workspace: &Workspace) -> bool {
    let top = workspace.dir().join(CACHE_DIR);
    for dir in [top, cache_dir::<S>(workspace)] {
        match found(&dir) {
            Found::Dir => {}
            Found::Nothing if fs::create_dir(&dir).is_ok() => {}
            _ => return false,
        }
    }
    true
}

/// Removes the entries of documents that are gone, and what failed writes left behind.
/// Only real files of the cache folder whose names end in `.idx` or `.tmp` are touched.
fn sweep<S: Summary>(workspace: &Workspace, names: &BTreeSet<String>) -> bool {
    let mut removed = false;
    let Ok(entries) = fs::read_dir(cache_dir::<S>(workspace)) else {
        return false;
    };
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let stale = match name.strip_suffix(".idx") {
            Some(doc) => !names.contains(doc),
            None => name.ends_with(".tmp"),
        };
        if stale && found(&entry.path()) == Found::File {
            removed |= fs::remove_file(entry.path()).is_ok();
        }
    }
    removed
}

/// Summarises every document of a workspace: see the module documentation. A file that
/// cannot be read as a document is listed with its error and does not stop the others.
/// With a cache, summaries are taken from the entries whose document file is unchanged,
/// the other entries are written again, and those of documents that are gone are removed.
pub fn scan<S: Summary>(workspace: &Workspace, mode: CacheMode) -> Result<Scan<S>, Error> {
    let dir = cache_dir::<S>(workspace);
    let top = found(&workspace.dir().join(CACHE_DIR));
    let state = match (mode, top, found(&dir)) {
        (CacheMode::Off, _, _) => CacheUse::Off,
        (_, Found::Dir, Found::Dir) => CacheUse::Read,
        (_, Found::Nothing, _) | (_, Found::Dir, Found::Nothing) => CacheUse::Missing,
        _ => CacheUse::Unusable,
    };
    // Made on the first write, so that a scan of a folder with no documents leaves nothing.
    let mut writable = match state {
        CacheUse::Read => Some(true),
        CacheUse::Missing => None,
        CacheUse::Off | CacheUse::Unusable => Some(false),
    };
    let mut cache_written = false;
    let mut rejected = 0usize;
    let mut files = Vec::new();
    let names = workspace.names()?;
    for name in &names {
        let path = workspace.dir().join(name);
        let entry_path = dir.join(format!("{name}.idx"));
        let mut entry = None;
        if state == CacheUse::Read && found(&entry_path) != Found::Nothing {
            if found(&entry_path) == Found::File {
                entry = read_entry::<S>(&entry_path);
            }
            if entry.is_none() {
                rejected = rejected.saturating_add(1);
            }
        }
        let before = stamp(&path);
        // The fast way: the file is as long and as old as the entry says, and was already
        // old when the entry was written.
        if let (CacheMode::Fast, Some(found), Some((len, Some(time)))) = (mode, &entry, before) {
            let settled = stamp(&entry_path)
                .and_then(|(_, written)| written?.checked_sub(time))
                .is_some_and(|age| age >= RACY_NANOS);
            if settled && found.doc_len == len && found.doc_time == Some(time) {
                if let Some(summary) = S::decode(found.body()) {
                    files.push(Scanned {
                        name: name.clone(),
                        summary: Ok(summary),
                        cached: true,
                    });
                    continue;
                }
            }
        }
        let mut cached = false;
        let summary = storage::read_bounded(&path).and_then(|bytes| {
            let sum = digest(&bytes);
            // A file that changed while it was read gets no time: never believed fast.
            let time = before
                .filter(|_| stamp(&path) == before)
                .and_then(|(_, time)| time);
            let same =
                |found: &&Entry| found.doc_len == bytes.len() as u64 && found.doc_digest == sum;
            let reused = entry.as_ref().filter(same).and_then(|found| {
                let summary = S::decode(found.body())?;
                // Brought up to date so that the next fast scan can believe it.
                if mode == CacheMode::Fast || found.doc_time != time {
                    let first = header::<S>(found.doc_len, time, sum, found.body());
                    cache_written |= write_entry(&entry_path, &first, found.body());
                }
                Some(summary)
            });
            if let Some(summary) = reused {
                cached = true;
                return Ok(summary);
            }
            if entry.is_some() {
                rejected = rejected.saturating_add(1);
            }
            let summary = S::of(&crate::format::from_slice(&bytes)?);
            if *writable.get_or_insert_with(|| make_cache_dir::<S>(workspace)) {
                let body = summary.encode();
                if body.len() as u64 <= MAX_ENTRY_BYTES {
                    let first = header::<S>(bytes.len() as u64, time, sum, &body);
                    cache_written |= write_entry(&entry_path, &first, &body);
                }
            }
            Ok(summary)
        });
        files.push(Scanned {
            name: name.clone(),
            summary,
            cached,
        });
    }
    if state == CacheUse::Read {
        cache_written |= sweep::<S>(workspace, &names.into_iter().collect());
    }
    Ok(Scan {
        files,
        cache: state,
        cache_written,
        rejected,
    })
}

/// The link index of a workspace, with what is needed to show its results.
#[derive(Debug)]
pub struct WorkspaceLinks {
    /// The index of all documents that could be read.
    pub index: LinkIndex,
    /// The file name of each indexed document.
    pub names: BTreeMap<DocId, String>,
    /// Document IDs that several files have (copies made by hand, decision A28), with
    /// those files, sorted. Only the first file is in the index, and links from other
    /// documents into such a document are reported as ambiguous.
    pub duplicates: BTreeMap<DocId, Vec<String>>,
    /// The files that could not be read, with the reason.
    pub failed: Vec<(String, Error)>,
    /// What became of the cache file.
    pub cache: CacheUse,
    /// How many documents were parsed.
    pub parsed: usize,
    /// How many summaries came from the cache.
    pub reused: usize,
    /// Whether any cache entry was written or removed.
    pub cache_written: bool,
    /// How many cache entries were there but could not be believed.
    pub rejected: usize,
}

/// Builds the link index of a workspace: of all its documents, open or not, by reading
/// each file once ([`scan`]). An interface keeps the index and follows the documents it
/// edits with [`LinkIndex::apply`]; after documents were saved, added or deleted, building
/// it again with a cache parses only the files that changed.
pub fn link_index(workspace: &Workspace, cache: CacheMode) -> Result<WorkspaceLinks, Error> {
    let scanned = scan::<DocLinks>(workspace, cache)?;
    let mut links = WorkspaceLinks {
        index: LinkIndex::new(),
        names: BTreeMap::new(),
        duplicates: BTreeMap::new(),
        failed: Vec::new(),
        cache: scanned.cache,
        parsed: scanned.parsed(),
        reused: scanned.reused(),
        cache_written: scanned.cache_written,
        rejected: scanned.rejected,
    };
    for file in scanned.files {
        let summary = match file.summary {
            Ok(summary) => summary,
            Err(error) => {
                links.failed.push((file.name, error));
                continue;
            }
        };
        let id = summary.id();
        match links.names.get(&id) {
            None => {
                links.names.insert(id, file.name);
                links.index.insert(summary);
            }
            Some(first) => {
                links.index.mark_ambiguous(id);
                let files = links
                    .duplicates
                    .entry(id)
                    .or_insert_with(|| vec![first.clone()]);
                files.push(file.name);
            }
        }
    }
    Ok(links)
}
