//! An open document: what an interface holds while it shows and edits one.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use flutter_rust_bridge::frb;
use wombat_core::storage::{self, FileCheck, FileState};
use wombat_core::{Document, Editor, SeededIds};

use super::wire::{BridgeError, Command, ErrorKind, GridInfo, Report};
use crate::convert::{cell_id, command_in, grid_out, report_out};
use crate::open_files::Claim;

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

/// What an open document is made of. It is apart from [`OpenDocument`] so that closing
/// can drop it, the file's claim and the memory with it, while Dart still holds the handle.
struct Session {
    editor: Editor,
    path: PathBuf,
    ids: SeededIds,
    clock: Clock,
    /// Whether nothing was done since the document was read from or written to its file.
    saved: bool,
    /// What the file looked like when it was read or last written, to tell at the next
    /// save whether another program changed it since.
    file: FileState,
    /// Keeps the file from being opened a second time; gives it up when dropped.
    _claim: Claim,
    /// Makes writing the file panic, for the test of what a drop does then.
    #[cfg(test)]
    panic_in_write: bool,
}

impl Session {
    fn report(&mut self, changes: &wombat_core::Changes) -> Report {
        let report = report_out(changes, self.editor.recalculated());
        if report != Report::default() {
            self.saved = false;
        }
        report
    }

    fn save(&mut self) -> Result<(), BridgeError> {
        match self.file.check(&self.path)? {
            FileCheck::Unchanged if self.saved => Ok(()),
            FileCheck::Unchanged | FileCheck::Gone => self.write(),
            FileCheck::Changed => Err(BridgeError {
                kind: ErrorKind::ChangedOnDisk,
                message: format!(
                    "{}: the file was changed by another program since it was read or saved",
                    self.path.display()
                ),
                path: Some(self.path.display().to_string()),
            }),
        }
    }

    fn write(&mut self) -> Result<(), BridgeError> {
        #[cfg(test)]
        assert!(!self.panic_in_write, "on purpose, while writing the file");
        // Every save puts a new file under the name, so the state is noted each time.
        self.file = storage::save_document_with_state(self.editor.document(), &self.path)?;
        self.saved = true;
        Ok(())
    }

    /// What a drop may save: unsaved work whose file is as it was or gone. A file that
    /// someone else changed is left alone, because a drop has no one to ask or tell.
    fn save_quietly(&mut self) {
        let free = matches!(
            self.file.check(&self.path),
            Ok(FileCheck::Unchanged | FileCheck::Gone)
        );
        if free {
            let _ = self.write();
        }
    }
}

/// The safety net under [`OpenDocument::close`]: a document that is let go of unsaved is
/// saved, as far as that can be done without asking.
///
/// This runs where nothing can be reported and nothing may go wrong: when Dart disposes
/// of the handle or its garbage collector finds it, on the thread that did so, inside a
/// function that `flutter_rust_bridge` generates without a guard, so that a panic coming
/// out of here would end the app. Hence `catch_unwind`, and the payload of a caught
/// panic is forgotten, not dropped, because dropping it is the one step left that could
/// panic. It also blocks that thread for as long as the save takes, and it never runs at
/// all when the system kills the app: an interface saves by calling `save` or `close`,
/// and does not count on this.
impl Drop for Session {
    fn drop(&mut self) {
        if self.saved {
            return;
        }
        if let Err(panic) = catch_unwind(AssertUnwindSafe(|| self.save_quietly())) {
            std::mem::forget(panic);
        }
    }
}

fn closed() -> BridgeError {
    BridgeError {
        kind: ErrorKind::Closed,
        message: "the document was closed".into(),
        path: None,
    }
}

/// A document that is open for reading and editing, with its undo history and the file it
/// is saved to. Dart holds it as an opaque handle; it lives until Dart lets go of it. Two
/// calls on one document never run at the same time: the second waits for the first.
///
/// A file is open in one document at a time: while this one is open, opening its file
/// again is refused ([`ErrorKind::AlreadyOpen`]).
///
/// **Letting go.** [`OpenDocument::close`] is how to end: it saves and gives the document
/// up, or says why it could not. A handle that is disposed of, or forgotten and found by
/// the garbage collector, without having been closed saves what is unsaved if it can do so
/// without going over another program's work, and gives up the file either way; that is a
/// safety net which cannot report anything, runs on the thread that disposes, and never
/// runs when the system kills the app.
#[frb(opaque)]
pub struct OpenDocument {
    /// `None` once the document was closed.
    session: Option<Session>,
}

impl OpenDocument {
    fn with(
        (doc, file): (Document, FileState),
        path: &Path,
        ids: SeededIds,
        claim: Claim,
    ) -> Result<OpenDocument, BridgeError> {
        // This cannot fail after a new document or a copy was written: the editor refuses
        // only what `Document::validate` refuses, and saving ran that on this very
        // document before it wrote. So no file is left without a document that is open.
        let editor = Editor::new(doc)?;
        Ok(OpenDocument {
            session: Some(Session {
                editor,
                path: path.to_path_buf(),
                ids,
                clock: Clock::System,
                saved: true,
                file,
                _claim: claim,
                #[cfg(test)]
                panic_in_write: false,
            }),
        })
    }

    /// The document, or the refusal of every call once it was closed.
    fn session(&self) -> Result<&Session, BridgeError> {
        self.session.as_ref().ok_or_else(closed)
    }

    fn session_mut(&mut self) -> Result<&mut Session, BridgeError> {
        self.session.as_mut().ok_or_else(closed)
    }

    /// Opens the document in the file at `path`. Refused if a document has that file open
    /// ([`ErrorKind::AlreadyOpen`]).
    pub fn open(path: String) -> Result<OpenDocument, BridgeError> {
        OpenDocument::open_with(Path::new(&path), os_ids()?)
    }

    /// Makes a new document with an empty root cell and saves it at `path`, where no file
    /// may be yet ([`ErrorKind::AlreadyExists`]) and no document may be open
    /// ([`ErrorKind::AlreadyOpen`]).
    pub fn create(path: String) -> Result<OpenDocument, BridgeError> {
        OpenDocument::create_with(Path::new(&path), os_ids()?)
    }

    /// Makes a copy of the document in the file `from` at `to`, where no file may be yet
    /// and no document may be open, and opens the copy. The copy has new IDs for itself
    /// and every cell: this is how a document that many people get the same file of, such
    /// as an example that comes with the app, becomes a document of one's own. `from` may
    /// be open; what is copied is its file, not what was done since it was saved. If the
    /// copy fails, nothing of it stays.
    pub fn open_copy(from: String, to: String) -> Result<OpenDocument, BridgeError> {
        OpenDocument::copy_with(Path::new(&from), Path::new(&to), os_ids()?)
    }

    /// [`OpenDocument::open`] with the IDs for new cells given, for tests.
    #[frb(ignore)]
    pub fn open_with(path: &Path, ids: SeededIds) -> Result<OpenDocument, BridgeError> {
        // The file is claimed before it is touched and the claim is dropped with whatever
        // fails, here and in the two below: a file is never held by a document that is not.
        let claim = Claim::take(path)?;
        OpenDocument::with(storage::load_document_with_state(path)?, path, ids, claim)
    }

    /// [`OpenDocument::create`] with the IDs given, for tests.
    #[frb(ignore)]
    pub fn create_with(path: &Path, mut ids: SeededIds) -> Result<OpenDocument, BridgeError> {
        let claim = Claim::take(path)?;
        // Also true for a link that leads nowhere, which `exists` would not see.
        if std::fs::symlink_metadata(path).is_ok() {
            return Err(wombat_core::Error::AlreadyExists(path.to_path_buf()).into());
        }
        let doc = Document::new(&mut ids);
        let file = storage::save_document_with_state(&doc, path)?;
        OpenDocument::with((doc, file), path, ids, claim)
    }

    /// [`OpenDocument::open_copy`] with the IDs given, for tests.
    #[frb(ignore)]
    pub fn copy_with(
        from: &Path,
        to: &Path,
        mut ids: SeededIds,
    ) -> Result<OpenDocument, BridgeError> {
        let claim = Claim::take(to)?;
        let copy = storage::copy_document_with_state(from, to, &mut ids)?;
        OpenDocument::with(copy, to, ids, claim)
    }

    /// Makes edits stamp this time on cells instead of the time of the device's clock
    /// (`None`: no time), for tests. Nothing happens on a document that was closed.
    #[frb(ignore)]
    pub fn set_fixed_time(&mut self, now: Option<i64>) {
        if let Some(session) = &mut self.session {
            session.clock = Clock::Fixed(now);
        }
    }

    /// Makes every later write of the file panic, for the tests in `panic_tests.rs`. It
    /// exists only in this crate's own test build.
    #[cfg(test)]
    #[frb(ignore)]
    pub(crate) fn panic_in_write(&mut self) {
        if let Some(session) = &mut self.session {
            session.panic_in_write = true;
        }
    }

    /// The permanent ID of the document.
    pub fn id(&self) -> Result<String, BridgeError> {
        Ok(self.session()?.editor.document().id().to_string())
    }

    /// The ID of the root cell.
    pub fn root(&self) -> Result<String, BridgeError> {
        Ok(self.session()?.editor.document().root().to_string())
    }

    /// The file the document is saved to, as it was given when the document was opened.
    pub fn path(&self) -> Result<String, BridgeError> {
        Ok(self.session()?.path.display().to_string())
    }

    /// Whether nothing was done to the document since it was read from or written to its
    /// file. This does not look at the file: that another program removed or changed it
    /// shows when saving. Undoing or redoing after a save makes the document unsaved, also
    /// when the two together lead back to what was saved: any change clears the flag.
    pub fn is_saved(&self) -> Result<bool, BridgeError> {
        Ok(self.session()?.saved)
    }

    /// The cell `cell` and the cells of its grid, each with its text, what it shows if it
    /// is a formula cell, and whether it has a grid. A cell without a grid gives 0 by 0
    /// cells.
    pub fn grid(&self, cell: String) -> Result<GridInfo, BridgeError> {
        let session = self.session()?;
        Ok(grid_out(session.editor.grid_view(cell_id(&cell)?)?))
    }

    /// Carries out a command and returns what it changed. On an error the document is
    /// exactly as before.
    pub fn apply(&mut self, command: Command) -> Result<Report, BridgeError> {
        let session = self.session_mut()?;
        let command = command_in(command)?;
        let changes = session
            .editor
            .apply(command, session.clock.now(), &mut session.ids)?;
        Ok(session.report(&changes))
    }

    /// Undoes the latest command and returns what that changed, or `None` if there is
    /// nothing to undo.
    pub fn undo(&mut self) -> Result<Option<Report>, BridgeError> {
        let session = self.session_mut()?;
        let changes = session.editor.undo()?;
        Ok(changes.map(|changes| session.report(&changes)))
    }

    /// Redoes the command that was undone last and returns what that changed, or `None`
    /// if there is nothing to redo.
    pub fn redo(&mut self) -> Result<Option<Report>, BridgeError> {
        let session = self.session_mut()?;
        let changes = session.editor.redo()?;
        Ok(changes.map(|changes| session.report(&changes)))
    }

    /// Makes the file hold the document, without ever going over what another program
    /// put there. It looks at the file first:
    ///
    /// - the file is as this document read or last wrote it: it is written if anything was
    ///   done since ([`OpenDocument::is_saved`] is false), and left alone otherwise;
    /// - the file is gone: it is written, whether anything was done or not;
    /// - the file was changed or replaced by someone else: nothing is written and the
    ///   error is [`ErrorKind::ChangedOnDisk`] with the path, whether anything was done or
    ///   not. [`OpenDocument::overwrite`] is the way to write all the same;
    /// - the file cannot be looked at, or the path is a link that leads nowhere:
    ///   [`ErrorKind::Io`].
    ///
    /// On an error nothing was written, and the document is as unsaved as it was.
    pub fn save(&mut self) -> Result<(), BridgeError> {
        self.session_mut()?.save()
    }

    /// Writes the document to its file whatever is there, replacing what another program
    /// saved since: for when [`OpenDocument::save`] answered [`ErrorKind::ChangedOnDisk`]
    /// and the user chose to keep this document. It is a call of its own, not a switch on
    /// `save`, so that no ordinary save can overwrite by a wrong argument.
    pub fn overwrite(&mut self) -> Result<(), BridgeError> {
        self.session_mut()?.write()
    }

    /// Ends the work on the document: saves what is unsaved and gives the document up.
    ///
    /// - If anything was done since the last save, it is saved as by
    ///   [`OpenDocument::save`], with the same refusals. If that fails, the error is
    ///   returned and the document **stays open** exactly as it was, so that nothing is
    ///   lost: the caller can try again, [`OpenDocument::overwrite`] and close, or keep
    ///   working.
    /// - If nothing was done since the last save, nothing is written and the file is not
    ///   looked at: closing a document that was only read never fails, never brings back a
    ///   file that was removed, and never complains about a file that someone else changed.
    ///
    /// After a close that succeeded the file may be opened again, the memory is freed, and
    /// every later call on this handle, `close` included, is refused with
    /// [`ErrorKind::Closed`]. The handle itself is still Dart's to dispose of.
    pub fn close(&mut self) -> Result<(), BridgeError> {
        let session = self.session_mut()?;
        if !session.saved {
            session.save()?;
        }
        self.session = None;
        Ok(())
    }
}

/// Runs once when the app starts the library.
#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}
