//! A workspace: a folder of documents (decision A6).
//!
//! The documents of a workspace are the `*.wombat` files directly in its folder. Their
//! images and view state are beside them, as [`crate::storage`] lays out.

use std::path::{Component, Path, PathBuf};

use crate::error::{invalid, Error};
use crate::id::{DocId, IdSource};
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
    /// The ID of the document, or why the file could not be loaded.
    pub id: Result<DocId, Error>,
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

    /// Lists the documents, sorted by file name. Every file is loaded to find its ID, so a
    /// damaged file is listed with its error instead of hiding the others.
    pub fn list(&self) -> Result<Vec<Entry>, Error> {
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
        Ok(names
            .into_iter()
            .map(|name| {
                let id = storage::load_document(&self.dir.join(&name)).map(|doc| doc.id());
                Entry { name, id }
            })
            .collect())
    }

    /// Opens the document with the file name `name`.
    pub fn open_name(&self, name: &str) -> Result<Document, Error> {
        storage::load_document(&self.path_of(name)?)
    }

    /// Opens the document with this ID and returns its file name with it. If a file was
    /// copied by hand, so that two files hold the same ID, the first by name is opened.
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
