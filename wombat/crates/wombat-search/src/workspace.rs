//! The search index of a workspace folder (decisions A54, A71).
//!
//! The only module of this crate that touches files, and only through
//! [`wombat_core::scan`]: every document of the folder is read once, open or not, for
//! the words and the links together, and the result is kept in the cache
//! `.wombat-index/search/` if asked.

use std::collections::BTreeMap;

use wombat_core::scan::{scan, CacheMode, CacheUse};
use wombat_core::{DocId, Error, Workspace};

use crate::engine::SearchIndex;
use crate::index::DocIndex;

/// The search index of a workspace, with what is needed to show its results.
pub struct WorkspaceIndex {
    /// The index of all documents that could be read.
    pub index: SearchIndex,
    /// The file name of each indexed document.
    pub names: BTreeMap<DocId, String>,
    /// Document IDs that several files have (copies made by hand, decision A28), with
    /// those files, sorted. Only the first file is in the index.
    pub duplicates: BTreeMap<DocId, Vec<String>>,
    /// The files that could not be read, with the reason.
    pub failed: Vec<(String, Error)>,
    /// What became of the cache.
    pub cache: CacheUse,
    /// How many documents were parsed.
    pub parsed: usize,
    /// How many summaries came from the cache.
    pub reused: usize,
    /// Whether any cache entry was written or removed.
    pub cache_written: bool,
}

/// Builds the search index of a workspace: of all its documents, open or not, by reading
/// each file once. An interface keeps the index and follows the documents it edits with
/// [`SearchIndex::apply`]; building it again with a cache parses only the files that
/// changed.
pub fn index(workspace: &Workspace, cache: CacheMode) -> Result<WorkspaceIndex, Error> {
    let scanned = scan::<DocIndex>(workspace, cache)?;
    let mut out = WorkspaceIndex {
        index: SearchIndex::new(),
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
                out.failed.push((file.name, error));
                continue;
            }
        };
        let id = summary.search.id();
        match out.names.get(&id) {
            None => {
                out.index.insert(summary, &file.name);
                out.names.insert(id, file.name);
            }
            Some(first) => {
                out.index.links_mut().mark_ambiguous(id);
                let files = out
                    .duplicates
                    .entry(id)
                    .or_insert_with(|| vec![first.clone()]);
                files.push(file.name);
            }
        }
    }
    Ok(out)
}
