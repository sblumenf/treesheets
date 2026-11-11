//! A workspace: a folder of documents (decision A6).
//!
//! The documents of a workspace are the `*.wombat` files directly in its folder. Their
//! images and view state are beside them, as [`crate::storage`] lays out.

use std::collections::{BTreeMap, HashMap};
use std::path::{Component, Path, PathBuf};

use crate::error::{invalid, Error};
use crate::id::{DocId, Id, IdSource};
use crate::model::Document;
use crate::storage::{self, EXTENSION};
use crate::view::ViewState;

/// A folder of documents.
#[derive(Clone, Debug)]
pub struct Workspace {
    dir: PathBuf,
}

/// One document file of a workspace, as [`Workspace::list`] reports it.
#[derive(Debug)]
pub struct Entry {
    /// The file name, for example `notes.wombat`.
    pub name: String,
    /// The ID of the document, or why the file's header could not be read.
    pub id: Result<DocId, Error>,
    /// The other files that hold the same document ID, sorted: copies made by copying the
    /// file, which must not stay in one workspace (decision A28). Normally empty.
    pub same_id: Vec<String>,
}

/// Two files of a workspace that use some of the same IDs, as [`Workspace::id_collisions`]
/// reports them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IdCollision {
    /// The file that comes first by name.
    pub first: String,
    /// The other file.
    pub second: String,
    /// Whether the two have the same document ID.
    pub same_document: bool,
    /// How many IDs, of cells or of the document, both files use.
    pub shared_ids: usize,
}

impl Workspace {
    /// Opens an existing folder as a workspace.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Workspace, Error> {
        let dir = dir.into();
        if !dir.is_dir() {
            return Err(Error::NotFound(format!("the folder {}", dir.display())));
        }
        Ok(Workspace { dir })
    }

    /// The folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The path of the document file `name`, which must be a plain file name ending in
    /// `.wombat` (no folders, no `..`), so that a name cannot lead out of the workspace.
    pub fn path_of(&self, name: &str) -> Result<PathBuf, Error> {
        let path = Path::new(name);
        let mut parts = path.components();
        let plain = matches!(
            (parts.next(), parts.next()),
            (Some(Component::Normal(_)), None)
        ) && !name.contains(['/', '\\'])
            && !name.starts_with('.')
            && path.extension().is_some_and(|e| e == EXTENSION);
        if !plain {
            return invalid(format!(
                "{name:?} is not a document file name like \"notes.wombat\""
            ));
        }
        Ok(self.dir.join(name))
    }

    /// The file names of the documents, sorted.
    pub(crate) fn names(&self) -> Result<Vec<String>, Error> {
        let io = |source| Error::Io {
            path: self.dir.clone(),
            source,
        };
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.dir).map_err(io)? {
            let entry = entry.map_err(io)?;
            // Names that are not Unicode cannot be documents made by Wombat; skip them.
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if self.path_of(&name).is_ok() && entry.path().is_file() {
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }

    /// Lists the documents, sorted by file name, with their IDs. Only the header of each
    /// file is read ([`storage::document_id`]), so listing is cheap however large the
    /// documents are; a file whose header cannot be read is listed with the error instead
    /// of hiding the others, and damage further on in a file shows only when it is opened.
    /// Files that hold the same document ID name each other in [`Entry::same_id`].
    pub fn list(&self) -> Result<Vec<Entry>, Error> {
        let mut entries: Vec<Entry> = self
            .names()?
            .into_iter()
            .map(|name| Entry {
                id: storage::document_id(&self.dir.join(&name)),
                name,
                same_id: Vec::new(),
            })
            .collect();
        let mut by_id: BTreeMap<DocId, Vec<String>> = BTreeMap::new();
        for entry in &entries {
            if let Ok(id) = entry.id {
                by_id.entry(id).or_default().push(entry.name.clone());
            }
        }
        for entry in &mut entries {
            if let Some(names) = entry.id.as_ref().ok().and_then(|id| by_id.get(id)) {
                entry.same_id = names
                    .iter()
                    .filter(|n| **n != entry.name)
                    .cloned()
                    .collect();
            }
        }
        Ok(entries)
    }

    /// Finds the files that share IDs, of the document or of any cell, which decision A5
    /// rules out and which copying a file by hand causes. Unlike [`Workspace::list`] this
    /// loads every document completely, one at a time; files that do not load are left out.
    /// Each ID counts for the first file by name that uses it, so three copies of one file
    /// are reported as two pairs. Sorted by file names.
    pub fn id_collisions(&self) -> Result<Vec<IdCollision>, Error> {
        let names = self.names()?;
        let mut first_use: HashMap<Id, usize> = HashMap::new();
        let mut doc_ids: Vec<Option<DocId>> = Vec::with_capacity(names.len());
        let mut shared: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for (index, name) in names.iter().enumerate() {
            let Ok(doc) = self.open_name(name) else {
                doc_ids.push(None);
                continue;
            };
            doc_ids.push(Some(doc.id()));
            let cells = doc.cells_in_order();
            for id in std::iter::once(doc.id().0).chain(cells.iter().map(|cell| cell.0)) {
                let first = *first_use.entry(id).or_insert(index);
                if first != index {
                    let count = shared.entry((first, index)).or_default();
                    *count = count.saturating_add(1);
                }
            }
        }
        Ok(shared
            .into_iter()
            .filter_map(|((first, second), shared_ids)| {
                Some(IdCollision {
                    first: names.get(first)?.clone(),
                    second: names.get(second)?.clone(),
                    same_document: doc_ids.get(first)? == doc_ids.get(second)?,
                    shared_ids,
                })
            })
            .collect())
    }

    /// Saves a copy of the document `name` as the new document `new_name`, with its images
    /// and view state. The copy has IDs of its own ([`storage::copy_document`]).
    pub fn duplicate(
        &self,
        name: &str,
        new_name: &str,
        ids: &mut dyn IdSource,
    ) -> Result<Document, Error> {
        storage::copy_document(&self.path_of(name)?, &self.path_of(new_name)?, ids)
    }

    /// Opens the document with the file name `name`.
    pub fn open_name(&self, name: &str) -> Result<Document, Error> {
        storage::load_document(&self.path_of(name)?)
    }

    /// Opens the document with this ID and returns its file name with it. If a file was
    /// copied by hand, so that two files hold the same ID, the first by name is opened
    /// ([`Workspace::list`] reports such files).
    pub fn open_id(&self, id: DocId) -> Result<(String, Document), Error> {
        for entry in self.list()? {
            if entry.id.is_ok_and(|found| found == id) {
                let doc = self.open_name(&entry.name)?;
                return Ok((entry.name, doc));
            }
        }
        Err(Error::NotFound(format!("a document with the ID {id}")))
    }

    /// Creates an empty document, saves it as `name` and returns it. An existing file is
    /// never replaced.
    pub fn create(&self, name: &str, ids: &mut dyn IdSource) -> Result<Document, Error> {
        let path = self.path_of(name)?;
        if path.exists() {
            return Err(Error::AlreadyExists(path));
        }
        let doc = Document::new(ids);
        storage::save_document(&doc, &path)?;
        Ok(doc)
    }

    /// Saves a document as `name`, replacing that file.
    pub fn save(&self, doc: &Document, name: &str) -> Result<(), Error> {
        storage::save_document(doc, &self.path_of(name)?)
    }

    /// Loads the view state of a document, if any was saved.
    pub fn load_view(&self, doc: DocId) -> Result<Option<ViewState>, Error> {
        storage::load_view_state(&self.dir, doc)
    }

    /// Saves the view state of a document.
    pub fn save_view(&self, view: &ViewState) -> Result<(), Error> {
        storage::save_view_state(&self.dir, view)
    }
}
