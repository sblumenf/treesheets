//! Indexes over a workspace folder, with a cache on disk (decisions A53 and A54).
//!
//! [`scan`] gives one [`Summary`] per document of a workspace. It reads every document
//! file, one at a time, so memory is that of one document and the summaries, never of all
//! documents. With the cache on, a file whose bytes are the ones the cache was made from
//! is not parsed again: its summary is read from the cache.
//!
//! The cache is `.wombat-index/<kind>.idx` in the workspace folder. It is a disposable
//! copy (decision A13): deleting it costs time only, and anything doubtful about it (cut
//! short, changed, of another version, not what its own checksum says) makes it count as
//! missing. An entry is found by the length and a checksum of the document file's bytes,
//! which are read in full every time, so a file that changed in any way, whatever its
//! modification time says, is parsed again, and a renamed file is not. Like `storage`,
//! this module belongs to the file layer.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::Error;
use crate::id::DocId;
use crate::links::{DocLinks, LinkIndex};
use crate::storage;
use crate::summary::{checksum, Summary};
use crate::workspace::Workspace;

/// The folder, inside a workspace folder, that holds index caches.
pub const CACHE_DIR: &str = ".wombat-index";
/// A cache larger than this is neither read nor written.
pub const MAX_CACHE_BYTES: u64 = 256 * 1024 * 1024;

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

/// What became of the cache file when a scan looked for it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheUse {
    /// The scan was asked not to use a cache.
    Off,
    /// There was no cache file.
    Missing,
    /// There was one, but it could not be trusted and was ignored.
    Damaged,
    /// It was read.
    Read,
}

/// The result of [`scan`].
#[derive(Debug)]
pub struct Scan<S> {
    /// The document files, sorted by name.
    pub files: Vec<Scanned<S>>,
    /// What became of the cache file.
    pub cache: CacheUse,
    /// Whether a new cache file was written. (It is not when nothing changed, and failing
    /// to write it, in a folder that cannot be written to, is not an error.)
    pub cache_written: bool,
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

/// The path of the cache file of the index kind `S` in a workspace.
pub fn cache_path<S: Summary>(workspace: &Workspace) -> PathBuf {
    workspace
        .dir()
        .join(CACHE_DIR)
        .join(format!("{}.idx", S::KIND))
}

fn header<S: Summary>(body: &[u8]) -> String {
    format!(
        "wombat-index {} {} {} {:032x}\n",
        S::KIND,
        S::VERSION,
        body.len(),
        checksum(body)
    )
}

/// The entries of the cache file, by key; `None` unless every check passes.
fn read_cache<S: Summary>(workspace: &Workspace) -> Result<BTreeMap<String, Value>, CacheUse> {
    let path = cache_path::<S>(workspace);
    if !path.exists() {
        return Err(CacheUse::Missing);
    }
    let read = || {
        let bytes = storage::read_up_to(&path, MAX_CACHE_BYTES).ok()?;
        let line_end = bytes.iter().position(|b| *b == b'\n')?;
        let (first, body) = bytes.split_at_checked(line_end.checked_add(1)?)?;
        // The first line must be exactly what this version would write for this body.
        if first != header::<S>(body).as_bytes() {
            return None;
        }
        serde_json::from_slice::<BTreeMap<String, Value>>(body).ok()
    };
    read().ok_or(CacheUse::Damaged)
}

fn write_cache<S: Summary>(workspace: &Workspace, entries: &BTreeMap<String, Value>) -> bool {
    let Ok(body) = serde_json::to_vec(entries) else {
        return false;
    };
    let mut bytes = header::<S>(&body).into_bytes();
    bytes.extend_from_slice(&body);
    if bytes.len() as u64 > MAX_CACHE_BYTES {
        return false;
    }
    let path = cache_path::<S>(workspace);
    let made = path.parent().is_some_and(|dir| {
        std::fs::create_dir_all(dir).is_ok() && storage::write_atomic(&path, &bytes).is_ok()
    });
    made
}

/// Summarises every document of a workspace: see the module documentation. A file that
/// cannot be read as a document is listed with its error and does not stop the others.
/// With `cache`, summaries are taken from the cache file where the document file is
/// unchanged, and the cache file is written again if it no longer matches the folder.
pub fn scan<S: Summary>(workspace: &Workspace, cache: bool) -> Result<Scan<S>, Error> {
    let (mut old, state) = match (cache, read_cache::<S>(workspace)) {
        (false, _) => (BTreeMap::new(), CacheUse::Off),
        (true, Ok(entries)) => (entries, CacheUse::Read),
        (true, Err(state)) => (BTreeMap::new(), state),
    };
    let mut new: BTreeMap<String, Value> = BTreeMap::new();
    let mut files = Vec::new();
    for name in workspace.names()? {
        let path = workspace.dir().join(&name);
        let mut cached = false;
        let summary = storage::read_bounded(&path).and_then(|bytes| {
            let key = format!("{}-{:032x}", bytes.len(), checksum(&bytes));
            if let Some(value) = old.remove(&key) {
                new.insert(key.clone(), value);
            }
            if let Some(summary) = new.get(&key).and_then(S::decode) {
                cached = true;
                return Ok(summary);
            }
            let summary = S::of(&crate::format::from_slice(&bytes)?);
            if cache {
                new.insert(key, summary.encode());
            }
            Ok(summary)
        });
        files.push(Scanned {
            name,
            summary,
            cached,
        });
    }
    // Entries left in `old` are of files that are gone or changed.
    let stale = state != CacheUse::Read || !old.is_empty();
    let fresh = files
        .iter()
        .any(|file| file.summary.is_ok() && !file.cached);
    let cache_written = cache && (stale || fresh) && write_cache::<S>(workspace, &new);
    Ok(Scan {
        files,
        cache: state,
        cache_written,
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
    /// Whether a new cache file was written.
    pub cache_written: bool,
}

/// Builds the link index of a workspace: of all its documents, open or not, by reading
/// each file once ([`scan`]). An interface keeps the index and follows the documents it
/// edits with [`LinkIndex::apply`]; after documents were saved, added or deleted, building
/// it again with `cache` parses only the files that changed.
pub fn link_index(workspace: &Workspace, cache: bool) -> Result<WorkspaceLinks, Error> {
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
