//! An open document: what an interface holds while it shows and edits one.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use flutter_rust_bridge::frb;
use wombat_core::{storage, Document, Editor, SeededIds};

use super::wire::{BridgeError, Command, ErrorKind, GridInfo, Report};
use crate::convert::{cell_id, command_in, grid_out, report_out};

/// Where the time that edits stamp on cells comes from.
enum Clock {
    /// The clock of the device.
    System,
    /// Always this, for tests.
    Fixed(Option<i64>),
}

impl Clock {
    /// Milliseconds since 1970 UTC, or `None` to stamp nothing.
    fn now(&self) -> Option<i64> {
        match self {
            Clock::Fixed(now) => *now,
            Clock::System => {
                let since = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
                i64::try_from(since.as_millis()).ok()
            }
        }
    }
}

/// An ID source seeded from the operating system, so that IDs are unique across documents.
fn os_ids() -> Result<SeededIds, BridgeError> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| BridgeError {
        kind: ErrorKind::Io,
        message: format!("no randomness from the operating system: {e}"),
        path: None,
    })?;
    Ok(SeededIds::from_seed(seed))
}

/// A document that is open for reading and editing, with its undo history and the file it
/// is saved to. Dart holds it as an opaque handle; it lives until Dart lets go of it. Two
/// calls on one document never run at the same time: the second waits for the first.
#[frb(opaque)]
pub struct OpenDocument {
    editor: Editor,
    path: PathBuf,
    ids: SeededIds,
    clock: Clock,
    saved: bool,
}

impl OpenDocument {
    fn with(doc: Document, path: &Path, ids: SeededIds) -> Result<OpenDocument, BridgeError> {
        Ok(OpenDocument {
            editor: Editor::new(doc)?,
            path: path.to_path_buf(),
            ids,
            clock: Clock::System,
            saved: true,
        })
    }

    /// Opens the document in the file at `path`.
    pub fn open(path: String) -> Result<OpenDocument, BridgeError> {
        OpenDocument::open_with(Path::new(&path), os_ids()?)
    }

    /// Makes a new document with an empty root cell and saves it at `path`, where no file
    /// may be yet.
    pub fn create(path: String) -> Result<OpenDocument, BridgeError> {
        OpenDocument::create_with(Path::new(&path), os_ids()?)
    }

    /// Makes a copy of the document in the file `from` at `to`, where no file may be yet,
    /// and opens the copy. The copy has new IDs for itself and every cell: this is how a
    /// document that many people get the same file of, such as an example that comes with
    /// the app, becomes a document of one's own.
    pub fn open_copy(from: String, to: String) -> Result<OpenDocument, BridgeError> {
        OpenDocument::copy_with(Path::new(&from), Path::new(&to), os_ids()?)
    }

    /// [`OpenDocument::open`] with the IDs for new cells given, for tests.
    #[frb(ignore)]
    pub fn open_with(path: &Path, ids: SeededIds) -> Result<OpenDocument, BridgeError> {
        OpenDocument::with(storage::load_document(path)?, path, ids)
    }

    /// [`OpenDocument::create`] with the IDs given, for tests.
    #[frb(ignore)]
    pub fn create_with(path: &Path, mut ids: SeededIds) -> Result<OpenDocument, BridgeError> {
        // Also true for a link that leads nowhere, which `exists` would not see.
        if std::fs::symlink_metadata(path).is_ok() {
            return Err(wombat_core::Error::AlreadyExists(path.to_path_buf()).into());
        }
        let doc = Document::new(&mut ids);
        storage::save_document(&doc, path)?;
        OpenDocument::with(doc, path, ids)
    }

    /// [`OpenDocument::open_copy`] with the IDs given, for tests.
    #[frb(ignore)]
    pub fn copy_with(
        from: &Path,
        to: &Path,
        mut ids: SeededIds,
    ) -> Result<OpenDocument, BridgeError> {
        let doc = storage::copy_document(from, to, &mut ids)?;
        OpenDocument::with(doc, to, ids)
    }

    /// Makes edits stamp this time on cells instead of the time of the device's clock
    /// (`None`: no time), for tests.
    #[frb(ignore)]
    pub fn set_fixed_time(&mut self, now: Option<i64>) {
        self.clock = Clock::Fixed(now);
    }

    /// The permanent ID of the document.
    pub fn id(&self) -> String {
        self.editor.document().id().to_string()
    }

    /// The ID of the root cell.
    pub fn root(&self) -> String {
        self.editor.document().root().to_string()
    }

    /// The file the document is saved to.
    pub fn path(&self) -> String {
        self.path.display().to_string()
    }

    /// Whether the file has everything that was done.
    pub fn is_saved(&self) -> bool {
        self.saved
    }

    /// The cell `cell` and the cells of its grid, each with its text, what it shows if it
    /// is a formula cell, and whether it has a grid. A cell without a grid gives 0 by 0
    /// cells.
    pub fn grid(&self, cell: String) -> Result<GridInfo, BridgeError> {
        Ok(grid_out(self.editor.grid_view(cell_id(&cell)?)?))
    }

    fn report(&mut self, changes: &wombat_core::Changes) -> Report {
        let report = report_out(changes, self.editor.recalculated());
        if report != Report::default() {
            self.saved = false;
        }
        report
    }

    /// Carries out a command and returns what it changed. On an error the document is
    /// exactly as before.
    pub fn apply(&mut self, command: Command) -> Result<Report, BridgeError> {
        let command = command_in(command)?;
        let changes = self
            .editor
            .apply(command, self.clock.now(), &mut self.ids)?;
        Ok(self.report(&changes))
    }

    /// Undoes the latest command and returns what that changed, or `None` if there is
    /// nothing to undo.
    pub fn undo(&mut self) -> Result<Option<Report>, BridgeError> {
        let changes = self.editor.undo()?;
        Ok(changes.map(|changes| self.report(&changes)))
    }

    /// Redoes the command that was undone last and returns what that changed, or `None`
    /// if there is nothing to redo.
    pub fn redo(&mut self) -> Result<Option<Report>, BridgeError> {
        let changes = self.editor.redo()?;
        Ok(changes.map(|changes| self.report(&changes)))
    }

    /// Saves the document to its file, if anything was done since it was saved last.
    pub fn save(&mut self) -> Result<(), BridgeError> {
        if !self.saved {
            storage::save_document(self.editor.document(), &self.path)?;
            self.saved = true;
        }
        Ok(())
    }
}

/// Runs once when the app starts the library.
#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
