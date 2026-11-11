//! Indexes over a workspace folder, with a cache on disk (decisions A53, A54, A71, A112).
//!
//! [`scan`] gives one [`Summary`] per document of a workspace. It reads the documents one
//! at a time, so memory is that of one document and the summaries, never of all
//! documents. With a cache, a document whose file is the one the cache was made from is
//! not parsed again: its summary is read from the cache.
//!
//! **The cache is never inside the workspace folder.** A workspace can come from an
//! archive, a shared drive or a sync service, and nothing in a cache proves where it
//! came from: an entry planted there, made to fit a document, would be believed. So the
//! caller says where the cache is ([`Cache`]): a folder of the user's own, such as the
//! cache directory of the system or of the app, with one folder per workspace in it
//! ([`Cache::in_base`]). A cache folder that turns out to be inside the workspace is not
//! used, and a `.wombat-index` folder inside a workspace (where the cache was before) is
//! never read, written or deleted.
//!
//! The cache is one file per document, `<cache folder>/<kind>/<document file name>.idx`,
//! so a changed document costs the rewriting of its own entry only. It is a disposable
//! copy (decision A13): deleting it costs time only ([`clear_cache`] does it), and
//! anything doubtful about an entry (cut short, changed, of another version or of other
//! rules, of another file, not what its own checksum says) makes it count as missing.
//! Whether the document file is still the one an entry was made from is decided in one
//! of two ways ([`CacheMode`]): by reading the file and comparing a checksum of its
//! bytes, or, faster and less sure, by what the file system says about the file.
//!
//! Nothing here follows a symbolic link: the cache folder of a workspace, the folders in
//! it and the entries must be real folders and files, or the cache is neither read nor
//! written. Like `storage`, this module belongs to the file layer.

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

/// The folder inside a workspace folder that held index caches before they moved out of
/// it. It is ignored: never read, never written, never deleted.
pub const OLD_CACHE_DIR: &str = ".wombat-index";
/// The file, in the cache folder of a workspace, that says which folder it is the cache
/// of (for a person who finds it).
pub const MARKER: &str = "workspace.txt";
/// A cache entry larger than this is neither read nor written: twice the largest
/// document file. (A summary is smaller than its document.) Reading an entry takes memory
/// in proportion to its size ([`Summary::decode`]), so this bounds it.
pub const MAX_ENTRY_BYTES: u64 = 2 * MAX_FILE_BYTES;
/// In [`CacheMode::Fast`], a document file is believed unchanged without being read only
/// if it was last modified at least this long before its cache entry was written
/// (2 seconds: the coarsest time stamps of common file systems). A file modified again
/// within the same tick of the clock as the entry was written would otherwise go unseen.
pub const RACY_NANOS: u128 = 2_000_000_000;

/// Whether and how [`scan`] uses the cache.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheMode {
    /// No cache is read or written.
    Off,
    /// Every document file is read and a checksum of its bytes compared with the entry:
    /// any change is noticed, whatever the file system says. **This is the mode to
    /// use** unless reading every file is too slow; it still saves all the parsing.
    Verify,
    /// A document file is not read at all if the file system says of it what its cache
    /// entry recorded: the same length and modification time, and on Unix the same
    /// file (inode) and the same time of last change to it (which a program cannot set
    /// back). **The risk:** documents are plain text, so an edit that keeps the length
    /// is ordinary, and a tool that restores modification times (a sync client, an
    /// archive unpacker, a backup being put back) then leaves nothing to see on a
    /// system without the Unix checks, or on a file system that does not keep them:
    /// the index is stale and nothing says so. Ask for this only where that is
    /// acceptable, and offer [`clear_cache`] ("rebuild the index") beside it.
    Fast,
}

/// Where the cache of one workspace is, and how it is checked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cache {
    dir: PathBuf,
    mode: CacheMode,
    now: Option<u128>,
}

impl Cache {
    /// No cache.
    pub fn off() -> Cache {
        Cache::at(PathBuf::new(), CacheMode::Off)
    }

    /// The cache in exactly this folder, which must not be inside the workspace (if it
    /// is, it is not used: [`CacheUse::Unusable`]).
    pub fn at(dir: impl Into<PathBuf>, mode: CacheMode) -> Cache {
        Cache {
            dir: dir.into(),
            mode,
            now: None,
        }
    }

    /// The cache of `workspace` among those kept under `base`, a folder of the user's
    /// own (the system's cache directory for the program, an app's cache directory): the
    /// folder in it named after the workspace folder and a checksum of its full path,
    /// so that every workspace has its own.
    pub fn in_base(base: &Path, workspace: &Workspace, mode: CacheMode) -> Result<Cache, Error> {
        let dir = workspace.dir();
        let io = |source| Error::Io {
            path: dir.to_path_buf(),
            source,
        };
        let full = fs::canonicalize(dir).map_err(io)?;
        let name = full.file_name().map(|name| name.to_string_lossy());
        let plain = |c: &char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_');
        let name: String = name
            .unwrap_or_default()
            .chars()
            .filter(plain)
            .take(32)
            .collect();
        let sum = digest(full.as_os_str().as_encoded_bytes());
        let key = format!("{name}-{:016x}", (sum >> 64) as u64);
        Ok(Cache::at(base.join(key), mode))
    }

    /// Tells the scan what time it is (nanoseconds since 1970), which it does not read
    /// itself. Only [`CacheMode::Fast`] uses it, to leave alone the entries of files
    /// that were modified so recently that writing them again could not make them
    /// believable yet.
    pub fn with_now(mut self, nanos: u128) -> Cache {
        self.now = Some(nanos);
        self
    }

    /// The cache folder of the workspace.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// How the cache is checked.
    pub fn mode(&self) -> CacheMode {
        self.mode
    }
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
    /// The cache folder is inside the workspace, or what is where it or a folder in it
    /// should be is not a real folder (a file, or a symbolic link): no cache was read or
    /// written.
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

/// The folder of the cache entries of the index kind `S`.
pub fn cache_dir<S: Summary>(cache: &Cache) -> PathBuf {
    cache.dir.join(S::KIND)
}

/// The path of the cache entry of the document file `name`.
pub fn cache_path<S: Summary>(cache: &Cache, name: &str) -> PathBuf {
    cache_dir::<S>(cache).join(format!("{name}.idx"))
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

/// What the file system says about a file, as far as it helps to tell that the file is
/// the one seen before: its length, its modification time (nanoseconds since 1970) and,
/// on Unix, which file it is (inode) and when it was last changed in any way, which
/// unlike the modification time cannot be set by a program.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Stamp {
    len: u64,
    time: Option<u128>,
    inode: Option<u64>,
    changed: Option<i128>,
}

fn stamp(path: &Path) -> Option<Stamp> {
    let meta = fs::metadata(path).ok()?;
    let time = meta.modified().ok();
    let time = time.and_then(|time| Some(time.duration_since(UNIX_EPOCH).ok()?.as_nanos()));
    #[cfg(unix)]
    let (inode, changed) = {
        use std::os::unix::fs::MetadataExt;
        let changed = i128::from(meta.ctime()) * 1_000_000_000 + i128::from(meta.ctime_nsec());
        (Some(meta.ino()), Some(changed))
    };
    #[cfg(not(unix))]
    let (inode, changed) = (None, None);
    Some(Stamp {
        len: meta.len(),
        time,
        inode,
        changed,
    })
}

/// The first line of an entry: everything that must be as expected for the entry to be
/// believed, and at its end one checksum over all of it, the name of the document file
/// and the body. So an entry is good exactly if its first line is the one this version
/// would write for this body, this document file and this name: an entry whose parts
/// come from two entries, or that was moved to another name, is not.
fn header<S: Summary>(name: &str, doc: &Stamp, doc_digest: u128, body: &[u8]) -> String {
    let or_dash = |text: Option<String>| text.unwrap_or_else(|| "-".to_owned());
    let line = format!(
        "wombat-index {} {} {:016x} {} {} {} {} {doc_digest:032x} {}",
        S::KIND,
        S::VERSION,
        S::fingerprint(),
        doc.len,
        or_dash(doc.time.map(|time| time.to_string())),
        or_dash(doc.inode.map(|inode| inode.to_string())),
        or_dash(doc.changed.map(|changed| changed.to_string())),
        body.len(),
    );
    let about = digest(format!("{line}\0{name}").as_bytes());
    let mut both = about.to_le_bytes().to_vec();
    both.extend(digest(body).to_le_bytes());
    format!("{line} {:032x}\n", digest(&both))
}

/// A cache entry that is whole: what it says about its document file, and its body.
struct Entry {
    doc: Stamp,
    doc_digest: u128,
    body_at: usize,
    bytes: Vec<u8>,
}

impl Entry {
    fn body(&self) -> &[u8] {
        self.bytes.get(self.body_at..).unwrap_or_default()
    }
}

/// Reads the entry of the document file `name` at `path`, which must be a real file;
/// `None` unless every check passes.
fn read_entry<S: Summary>(path: &Path, name: &str) -> Option<Entry> {
    let bytes = storage::read_up_to(path, MAX_ENTRY_BYTES).ok()?;
    let line_end = bytes.iter().position(|b| *b == b'\n')?;
    let body_at = line_end.checked_add(1)?;
    let (first, body) = bytes.split_at_checked(body_at)?;
    let mut words = std::str::from_utf8(first).ok()?.split(' ').skip(4);
    fn number<T: std::str::FromStr>(word: &str) -> Option<Option<T>> {
        match word {
            "-" => Some(None),
            word => Some(Some(word.parse().ok()?)),
        }
    }
    let doc = Stamp {
        len: words.next()?.parse().ok()?,
        time: number(words.next()?)?,
        inode: number(words.next()?)?,
        changed: number(words.next()?)?,
    };
    let doc_digest = u128::from_str_radix(words.next()?, 16).ok()?;
    // The first line must be exactly what this version would write for this body.
    if first != header::<S>(name, &doc, doc_digest, body).as_bytes() {
        return None;
    }
    Some(Entry {
        doc,
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

/// Whether the cache folder is, or would be, inside the workspace folder (or cannot be
/// told not to be): then it must not be used.
fn inside_workspace(cache: &Cache, workspace: &Workspace) -> bool {
    let Ok(folder) = fs::canonicalize(workspace.dir()) else {
        return true;
    };
    // The cache folder need not exist yet: the part of its path that does says where
    // it would be.
    let mut existing = cache.dir.as_path();
    let mut rest: Vec<&std::ffi::OsStr> = Vec::new();
    let full = loop {
        match fs::canonicalize(existing) {
            Ok(full) => break full,
            Err(_) => match (existing.parent(), existing.file_name()) {
                (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => {
                    rest.push(name);
                    existing = parent;
                }
                (_, Some(name)) => {
                    rest.push(name);
                    match fs::canonicalize(".") {
                        Ok(here) => break here,
                        Err(_) => return true,
                    }
                }
                _ => return true,
            },
        }
    };
    let full = rest.iter().rev().fold(full, |path, name| path.join(name));
    full.starts_with(&folder)
}

/// Makes sure the cache folder of `S` is there, as real folders; `false` if it cannot be.
/// The folder that the caller named is made with the folders above it (they are the
/// user's own); it and the folder of the kind must then be real folders.
fn make_cache_dir<S: Summary>(cache: &Cache, workspace: &Workspace) -> bool {
    if found(&cache.dir) == Found::Nothing {
        if fs::create_dir_all(&cache.dir).is_err() {
            return false;
        }
        // For a person who finds the folder: what it is the cache of.
        let about = format!(
            "Wombat index cache of the folder\n{}\nIt can be deleted at any time.\n",
            workspace.dir().display()
        );
        let _ = fs::write(cache.dir.join(MARKER), about);
    }
    for dir in [cache.dir.clone(), cache_dir::<S>(cache)] {
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
fn sweep(dir: &Path, names: &BTreeSet<String>) -> usize {
    let mut removed = 0usize;
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        let stale = match name.strip_suffix(".idx") {
            Some(doc) => !names.contains(doc),
            None => name.ends_with(".tmp"),
        };
        if stale && found(&entry.path()) == Found::File && fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Discards the cache of a workspace: "rebuild the index". Every entry of every kind of
/// index in the cache folder is removed, and the folders if that leaves them empty; the
/// next scan parses every document again. Only real folders and real entry files are
/// touched, and nothing if the folder is not a cache folder that [`scan`] would use
/// (inside the workspace, or a link). Returns how many entries were removed.
pub fn clear_cache(cache: &Cache, workspace: &Workspace) -> usize {
    if found(&cache.dir) != Found::Dir || inside_workspace(cache, workspace) {
        return 0;
    }
    let mut removed = 0usize;
    for kind in fs::read_dir(&cache.dir).into_iter().flatten().flatten() {
        if found(&kind.path()) == Found::Dir {
            removed += sweep(&kind.path(), &BTreeSet::new());
            let _ = fs::remove_dir(kind.path());
        }
    }
    let _ = fs::remove_file(cache.dir.join(MARKER));
    let _ = fs::remove_dir(&cache.dir);
    removed
}

/// Summarises every document of a workspace: see the module documentation. A file that
/// cannot be read as a document is listed with its error and does not stop the others.
/// With a cache, summaries are taken from the entries whose document file is unchanged,
/// the other entries are written again, and those of documents that are gone are removed.
pub fn scan<S: Summary>(workspace: &Workspace, cache: &Cache) -> Result<Scan<S>, Error> {
    let mode = cache.mode;
    let dir = cache_dir::<S>(cache);
    let state = if mode == CacheMode::Off {
        CacheUse::Off
    } else if inside_workspace(cache, workspace) {
        CacheUse::Unusable
    } else {
        match (found(&cache.dir), found(&dir)) {
            (Found::Dir, Found::Dir) => CacheUse::Read,
            (Found::Nothing, _) | (Found::Dir, Found::Nothing) => CacheUse::Missing,
            _ => CacheUse::Unusable,
        }
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
                entry = read_entry::<S>(&entry_path, name);
            }
            if entry.is_none() {
                rejected = rejected.saturating_add(1);
            }
        }
        let before = stamp(&path);
        // Whether the file was already old when its entry was written.
        let settled = |entry: &Entry| {
            let written = stamp(&entry_path).and_then(|entry| entry.time);
            let age = written
                .zip(entry.doc.time)
                .and_then(|(w, time)| w.checked_sub(time));
            age.is_some_and(|age| age >= RACY_NANOS)
        };
        // The fast way: the file system says of the file what the entry says.
        if let (CacheMode::Fast, Some(found), Some(now)) = (mode, &entry, before) {
            if now.time.is_some() && found.doc == now && settled(found) {
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
            let mut now = before.filter(|_| stamp(&path) == before).unwrap_or(Stamp {
                len: 0,
                time: None,
                inode: None,
                changed: None,
            });
            now.len = bytes.len() as u64;
            let same = |found: &&Entry| found.doc.len == now.len && found.doc_digest == sum;
            let reused = entry.as_ref().filter(same).and_then(|found| {
                let summary = S::decode(found.body())?;
                // Brought up to date so that the next fast scan can believe it: when
                // the file system says something else of the file now, or (in fast
                // mode) when the entry was written too soon after the file, unless the
                // file is still too new for writing it again to help.
                let too_new = cache.now.zip(now.time).is_some_and(|(clock, time)| {
                    clock.checked_sub(time).is_none_or(|age| age < RACY_NANOS)
                });
                let unsettled = mode == CacheMode::Fast && !settled(found) && !too_new;
                if found.doc != now || unsettled {
                    let first = header::<S>(name, &now, sum, found.body());
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
            if *writable.get_or_insert_with(|| make_cache_dir::<S>(cache, workspace)) {
                let body = summary.encode();
                if body.len() as u64 <= MAX_ENTRY_BYTES {
                    let first = header::<S>(name, &now, sum, &body);
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
        cache_written |= sweep(&dir, &names.into_iter().collect()) > 0;
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
pub fn link_index(workspace: &Workspace, cache: &Cache) -> Result<WorkspaceLinks, Error> {
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
