//! The search index of a workspace folder (decisions A54, A71).
//!
//! The only module of this crate that touches files, and only through
//! [`wombat_core::scan`]: every document of the folder is read once, open or not, for
//! the words and the links together, and the result is kept in a cache if asked, in a
//! folder that the caller names and that is never inside the workspace
//! ([`wombat_core::scan::Cache`]).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use wombat_core::scan::{scan, Cache, CacheUse};
use wombat_core::{storage, CellId, DocId, Document, Editor, Error, IdSource, Workspace};

use crate::engine::SearchIndex;
use crate::index::DocIndex;
use crate::query::Term;
use crate::replace::{plan, replace, Counts, ReplaceOptions, Scope};

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
pub fn index(workspace: &Workspace, cache: &Cache) -> Result<WorkspaceIndex, Error> {
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

/// One document of a replace across a folder: what was found in it.
#[derive(Clone, PartialEq, Debug)]
pub struct FileReplace {
    /// The file name.
    pub name: String,
    /// What was found, and what was or would be replaced.
    pub counts: Counts,
}

/// What a replace across a folder did ([`replace_in_folder`]).
#[derive(Debug, Default)]
pub struct FolderReplace {
    /// Every document in which the text was found, changed or not (a document with
    /// matches only in formula cells or inside characters is listed and not changed).
    pub files: Vec<FileReplace>,
    /// The documents that were changed and saved, in the order they were saved.
    pub saved: Vec<String>,
    /// The document whose saving failed, with the reason. Saving stopped there.
    pub failed: Option<(String, Error)>,
    /// The documents that were to be changed and were not, because saving stopped
    /// before their turn. Their files are as they were.
    pub not_saved: Vec<String>,
    /// The documents that could not be read, with the reason. They are left alone.
    pub unreadable: Vec<(String, Error)>,
}

/// Replaces `term` by `with` in several documents of a workspace: in each of `targets`,
/// a file name with the cell to stay inside of, or `None` for the whole document.
///
/// All documents are read and changed in memory first. If a replacement is refused in
/// any of them ([`replace`]), nothing is saved and the refusal is returned with the
/// file's name. Then the changed documents are saved one by one with `save` (the tool
/// passes [`wombat_core::storage::save_document`]). **This is not atomic:** each file is
/// replaced whole or not at all, but if saving one fails, the ones before it stay
/// changed. The result then says exactly which were saved, which failed and which were
/// not reached, and nothing more is written. With `dry_run` nothing is saved at all.
/// The changed documents are all in memory until they are saved.
#[allow(clippy::too_many_arguments)]
pub fn replace_in_folder(
    workspace: &Workspace,
    targets: &[(String, Option<CellId>)],
    term: &Term,
    with: &str,
    options: ReplaceOptions,
    dry_run: bool,
    now: Option<i64>,
    ids: &mut dyn IdSource,
    save: &mut dyn FnMut(&Document, &Path) -> Result<(), Error>,
) -> Result<FolderReplace, (String, Error)> {
    let mut out = FolderReplace::default();
    let mut changed: Vec<(String, PathBuf, Document)> = Vec::new();
    for (name, branch) in targets {
        let named = |error: Error| (name.clone(), error);
        let path = workspace.path_of(name).map_err(named)?;
        let doc = match storage::load_document(&path) {
            Ok(doc) => doc,
            Err(error) => {
                out.unreadable.push((name.clone(), error));
                continue;
            }
        };
        let scope = branch.map_or(Scope::Document, Scope::Branch);
        let counts = if dry_run {
            plan(&doc, term, with, scope, options).counts
        } else {
            let mut editor = Editor::new(doc).map_err(named)?;
            let done = replace(&mut editor, term, with, scope, options, now, ids).map_err(named)?;
            if !done.changes.is_empty() {
                changed.push((name.clone(), path, editor.into_document()));
            }
            done.counts
        };
        if counts.places > 0 || counts.formula_places > 0 || !counts.skipped.is_empty() {
            let name = name.clone();
            out.files.push(FileReplace { name, counts });
        }
    }
    let mut rest = changed.into_iter();
    for (name, path, doc) in rest.by_ref() {
        match save(&doc, &path) {
            Ok(()) => out.saved.push(name),
            Err(error) => {
                out.failed = Some((name, error));
                break;
            }
        }
    }
    out.not_saved = rest.map(|(name, _, _)| name).collect();
    Ok(out)
}
