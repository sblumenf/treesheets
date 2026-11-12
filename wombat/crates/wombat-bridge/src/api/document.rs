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
    /// The file, by its path from the root as fixed when the document was opened.
    path: PathBuf,
    ids: SeededIds,
    clock: Clock,
    /// Whether nothing was done since the document was read from or written to its file.
    saved: bool,
    /// What the file looked like when it was read or last written, to tell at the next
    /// save whether another program changed it since.
    file: FileState,
    /// Set while a call that changes the document is under way. Found set at the start
    /// of a call, it means that such a call never finished: a panic unwound through it
    /// and the document may be half changed.
    unfinished: bool,
    /// Keeps the file from being opened a second time; gives it up when dropped.
    _claim: Claim,
    /// Makes writing the file panic, for the test of what a drop does then.
    #[cfg(test)]
    panic_in_write: bool,
    /// Makes a call that changes the document panic when the core has done its part and
    /// the call is not finished, for the test of what later calls do then.
    #[cfg(test)]
    panic_in_change: bool,
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
        // Nothing is saved from a document that may be half changed.
        if self.saved || self.unfinished {
            return;
        }
        if let Err(panic) = catch_unwind(AssertUnwindSafe(|| self.save_quietly())) {
            std::mem::forget(panic);
        }
    }
}

fn half_changed() -> BridgeError {
    BridgeError {
        kind: ErrorKind::Internal,
        message: "a fault in the program: an earlier change to the document did not finish, \
                  so the document is not used any more; its file is as it was last saved"
            .into(),
        path: None,
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
/// is saved to. Dart holds it as an opaque handle; it lives until Dart lets go of it.
///
/// (The generator does not carry this comment into Dart; what a Dart developer must know
/// of it is repeated on [`OpenDocument::open`].)
///
/// **One call at a time, sent by the caller.** Two calls on one document never run at the
/// same moment: a lock keeps them apart (calls that only read may run together). But the
/// lock does not keep them in order. Each call is handed to a pool of threads, and of two
/// calls that are both waiting either may go first, whichever was made first. A caller
/// that makes a call before the one before has answered can therefore have them carried
/// out the other way round: characters typed in the wrong order, a command refused because
/// the one it builds on has not happened yet. So the caller sends one call, waits for its
/// answer, and only then sends the next. An app does that best in one place: a Dart class
/// that wraps this one and sends the calls one after another in the order they were made.
///
/// **Errors.** Every call answers with a value or with a [`BridgeError`]. After a call
/// that changes the document panicked, which is a fault in the program and not something a
/// caller can cause, every later call is refused ([`ErrorKind::Internal`]) and nothing more
/// is saved: the document may be half changed. Dispose of the handle and open the file
/// again; it is as it was last saved.
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
                path: claim.path().to_path_buf(),
                ids,
                clock: Clock::System,
                saved: true,
                file,
                unfinished: false,
                _claim: claim,
                #[cfg(test)]
                panic_in_write: false,
                #[cfg(test)]
                panic_in_change: false,
            }),
        })
    }

    /// The document, or the refusal of every call, reads and saving included, once it was
    /// closed or a change to it did not finish.
    fn session(&self) -> Result<&Session, BridgeError> {
        match &self.session {
            None => Err(closed()),
            Some(session) if session.unfinished => Err(half_changed()),
            Some(session) => Ok(session),
        }
    }

    fn session_mut(&mut self) -> Result<&mut Session, BridgeError> {
        match &mut self.session {
            None => Err(closed()),
            Some(session) if session.unfinished => Err(half_changed()),
            Some(session) => Ok(session),
        }
    }

    /// Opens the document in the file at `path`. Refused if a document has that file open
    /// ([`ErrorKind::AlreadyOpen`]).
    ///
    /// What to know about the document this gives, and those of [`OpenDocument::create`]
    /// and [`OpenDocument::open_copy`] (it is said here because the generator does not
    /// carry the comment of the type itself into Dart):
    ///
    /// - **Send one call at a time.** Two calls on one document never run at the same
    ///   moment, but they are not kept in order: of two that are both under way either
    ///   may be carried out first. Wait for the answer of each call before making the
    ///   next one on the same document.
    /// - **End with [`OpenDocument::close`]**, which saves. Disposing of a handle that was
    ///   not closed saves too if it can, but can report nothing, blocks the thread that
    ///   disposes, and does not happen when the system kills the app.
    /// - **Every call can fail** with a [`BridgeError`]: with [`ErrorKind::Closed`] after
    ///   `close`, and with [`ErrorKind::Internal`] after a call that changes the document
    ///   panicked (a fault in the program; the document may be half changed, nothing more
    ///   is saved from it, and its file is as it was last saved: dispose of the handle and
    ///   open the file again).
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
        // The claim also fixes the path from the root, which is the file from here on.
        let claim = Claim::take(path)?;
        let read = storage::load_document_with_state(claim.path())?;
        OpenDocument::with(read, ids, claim)
    }

    /// [`OpenDocument::create`] with the IDs given, for tests.
    #[frb(ignore)]
    pub fn create_with(path: &Path, mut ids: SeededIds) -> Result<OpenDocument, BridgeError> {
        let claim = Claim::take(path)?;
        // Also true for a link that leads nowhere, which `exists` would not see.
        if std::fs::symlink_metadata(claim.path()).is_ok() {
            return Err(wombat_core::Error::AlreadyExists(path.to_path_buf()).into());
        }
        let doc = Document::new(&mut ids);
        let file = storage::save_document_with_state(&doc, claim.path())?;
        OpenDocument::with((doc, file), ids, claim)
    }

    /// [`OpenDocument::open_copy`] with the IDs given, for tests.
    #[frb(ignore)]
    pub fn copy_with(
        from: &Path,
        to: &Path,
        mut ids: SeededIds,
    ) -> Result<OpenDocument, BridgeError> {
        let claim = Claim::take(to)?;
        let copy = storage::copy_document_with_state(from, claim.path(), &mut ids)?;
        OpenDocument::with(copy, ids, claim)
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

    /// Makes every later call that changes the document panic before it is finished, for
    /// the tests in `panic_tests.rs`. It exists only in this crate's own test build.
    #[cfg(test)]
    #[frb(ignore)]
    pub(crate) fn panic_in_change(&mut self) {
        if let Some(session) = &mut self.session {
            session.panic_in_change = true;
        }
    }

    /// Runs a call that changes the document, and notes that it is under way from before
    /// the core is asked until the answer is ready. The core is written not to panic, and
    /// when it refuses a command it leaves the document as it was. But should a panic
    /// unwind through `work` all the same, the note stays, and every later call is refused
    /// ([`ErrorKind::Internal`]): no lock here is poisoned by a panic, so without the note
    /// the half-changed document would be shown, edited and saved as if nothing had
    /// happened.
    fn change<T>(
        &mut self,
        work: impl FnOnce(&mut Session) -> Result<T, BridgeError>,
    ) -> Result<T, BridgeError> {
        let session = self.session_mut()?;
        session.unfinished = true;
        let result = work(session);
        #[cfg(test)]
        assert!(!session.panic_in_change, "on purpose, in a change");
        session.unfinished = false;
        result
    }

    /// The permanent ID of the document.
    pub fn id(&self) -> Result<String, BridgeError> {
        Ok(self.session()?.editor.document().id().to_string())
    }

    /// The ID of the root cell.
    pub fn root(&self) -> Result<String, BridgeError> {
        Ok(self.session()?.editor.document().root().to_string())
    }

    /// The file the document is saved to: the path it was opened with, from the root. A
    /// path that did not start at the root was taken from the working folder of the
    /// program as it was when the document was opened, and is not worked out again: the
    /// document stays with its file when the working folder changes.
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

    /// Carries out a command and returns what it changed.
    ///
    /// What holds for every command:
    ///
    /// - **On an error nothing changed**: the document and its undo history are exactly
    ///   as before, and the document is as saved or unsaved as it was.
    /// - **A command that asks for nothing new succeeds and changes nothing** (setting a
    ///   colour that a cell already has, inserting empty text): the report is empty,
    ///   nothing is added to the undo history, and the document stays saved if it was.
    ///   (`Run` can change what formula cells show without changing the document; those
    ///   cells are then in `recalculated` of a report that is otherwise empty.)
    /// - **Positions and lengths in text are counted in Unicode code points**, not in the
    ///   UTF-16 units of a Dart string, and must not cut a user-perceived character (a
    ///   letter with its accents, a flag, an emoji made of several). One that does, or is
    ///   outside the text, is refused ([`ErrorKind::Refused`]); it is never moved.
    /// - **Control characters in cell text become spaces** (everything below U+0020:
    ///   line breaks and tabs too). Notes keep them.
    /// - Cells are named by their ID as text; text that is no ID is refused
    ///   ([`ErrorKind::BadRequest`]) and an ID that no cell has is
    ///   [`ErrorKind::NotFound`]. Whole numbers out of range are refused, never cut.
    ///
    /// A command that changed something is one step of the undo history and empties the
    /// redo history.
    ///
    /// Wait for the answer before the next call on this document: calls that overlap are
    /// carried out in either order ([`OpenDocument::open`] says more).
    pub fn apply(&mut self, command: Command) -> Result<Report, BridgeError> {
        self.change(|session| {
            let command = command_in(command)?;
            let changes = session
                .editor
                .apply(command, session.clock.now(), &mut session.ids)?;
            Ok(session.report(&changes))
        })
    }

    /// Undoes the latest command and returns what that changed, or `None` if there is
    /// nothing to undo.
    pub fn undo(&mut self) -> Result<Option<Report>, BridgeError> {
        self.change(|session| {
            let changes = session.editor.undo()?;
            Ok(changes.map(|changes| session.report(&changes)))
        })
    }

    /// Redoes the command that was undone last and returns what that changed, or `None`
    /// if there is nothing to redo.
    pub fn redo(&mut self) -> Result<Option<Report>, BridgeError> {
        self.change(|session| {
            let changes = session.editor.redo()?;
            Ok(changes.map(|changes| session.report(&changes)))
        })
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

/// Runs once when the app starts the library. It makes a panic remember where it happened
/// if the environment asks for that (`RUST_BACKTRACE`), so that the exception Dart gets can
/// say so.
///
/// It is not `flutter_rust_bridge`'s `setup_default_user_utils`, which does two things
/// more. It sets `RUST_BACKTRACE` in the environment of the process, while the other
/// threads of the app are running and may be reading it: C gives no protection there.
/// And on Android, iOS and macOS it sends everything that any library logs, down to the
/// finest level, to the system's log, in release builds too; nothing in Wombat logs, so
/// that could only ever be noise or somebody's text. The price: an app that was not
/// started with `RUST_BACKTRACE=1` reports a panic by its message and place alone.
#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::PanicBacktrace::setup();
}
