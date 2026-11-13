//! The files that documents of this process have open, so that no file is open twice
//! (decision A141): two open documents on one file would each save over the other's work.
//!
//! This is the one piece of state in the bridge that is shared by the whole process. The
//! core has none; a set of open files is a fact about a running program, not about a
//! document.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::api::wire::{BridgeError, ErrorKind};

static OPEN: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

/// The set, also after a thread panicked while it held the lock: nothing below can leave
/// the set half changed, and a claim must be given up even then (it is given up while a
/// document is dropped, where a panic would end the process).
fn open() -> MutexGuard<'static, BTreeSet<PathBuf>> {
    OPEN.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The names under which the file of a document opened at `asked`, a path from the root,
/// is known, however that path is spelt. There are up to three, because a document is
/// two things that can come apart while it is open: the file it read, and the name it
/// saves to.
///
/// - The real name of the file that a save replaces: where links lead, without `.` and
///   `..`. A second open through another link to the same file, or of the file itself,
///   meets this one.
/// - The name in its real folder, without following a link that the name itself is. A
///   file that is not there yet has only this one, which is what it gets once it is
///   there; and it stays the document's when the link it was opened through is removed
///   or made to lead elsewhere, since the document goes on saving to that name. If the
///   folder is not there either (it was removed under an open document, and may be made
///   again), it is the name below the nearest folder above that is there.
/// - The path as it was asked for, which stays when a link on the way to the folder
///   changes.
///
/// Two names that the file system itself takes for one are not seen to be the same: a
/// second hard link to a file, and a name that differs only by case, or by the way an
/// accented letter is written, on file systems that ignore that, for a file that is not
/// there yet.
fn names(asked: &Path) -> BTreeSet<PathBuf> {
    let real = std::fs::canonicalize(asked).ok();
    let spelt = Some(asked.to_path_buf());
    [real, in_folder(asked), spelt]
        .into_iter()
        .flatten()
        .collect()
}

/// `path` with the real name of its folder, or of the nearest folder above it that is
/// there, and the names below that as they are spelt; `None` if one of those is `..`.
fn in_folder(path: &Path) -> Option<PathBuf> {
    let mut below = Vec::new();
    let mut folder = path;
    loop {
        below.push(folder.file_name()?);
        folder = folder.parent()?;
        if let Ok(real) = std::fs::canonicalize(folder) {
            return Some(below.iter().rev().fold(real, |path, name| path.join(name)));
        }
    }
}

/// The right of one open document to its file. Dropping it gives the file up.
#[derive(Debug)]
pub(crate) struct Claim {
    path: PathBuf,
    names: BTreeSet<PathBuf>,
}

impl Claim {
    /// Claims the file at `path` for a document about to be opened or made there, or
    /// refuses because a document has it open: one that holds any of the names that this
    /// one would hold. Checking and claiming are one step under the lock, so of two
    /// threads that ask at once one is refused.
    ///
    /// A path that does not start at the root is taken from the working folder as it is
    /// now, once: see [`Claim::path`].
    pub(crate) fn take(path: &Path) -> Result<Claim, BridgeError> {
        let shown = Some(path.display().to_string());
        let fixed = std::path::absolute(path).map_err(|error| BridgeError {
            kind: ErrorKind::Io,
            message: format!("{}: {error}", path.display()),
            path: shown.clone(),
        })?;
        let names = names(&fixed);
        let mut open = open();
        if names.is_disjoint(&open) {
            open.extend(names.iter().cloned());
            Ok(Claim { path: fixed, names })
        } else {
            Err(BridgeError {
                kind: ErrorKind::AlreadyOpen,
                message: format!("{}: the document is open already", path.display()),
                path: shown,
            })
        }
    }

    /// The file as the document knows it from now on: the path that was asked for, from
    /// the root. It is fixed here so that the document reads, saves and answers for one
    /// file whatever the working folder of the program becomes later; a path that was
    /// worked out anew at every save would lead to another file after such a change,
    /// while the claim still held the first. Links are not followed for it: saving
    /// through a link goes where the link leads at that time, as the storage layer has
    /// it.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        // No other claim holds any of these: it would have been refused.
        open().retain(|name| !self.names.contains(name));
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// A bare name is a file in the current folder, there or not, like every other
    /// spelling of it.
    #[test]
    fn a_bare_name_that_is_not_there_is_in_the_current_folder() {
        let here = std::env::current_dir().unwrap().canonicalize().unwrap();
        let name = "no-such-file-of-open-files.wombat";
        let fixed = |path: &Path| names(&std::path::absolute(path).unwrap());
        assert!(fixed(Path::new(name)).contains(&here.join(name)));
        assert!(fixed(&Path::new(".").join(name)).contains(&here.join(name)));
        // Nothing can be made in a folder that is not there; the spelling is kept.
        let nowhere = here.join("no-such-folder/..").join(name);
        assert_eq!(fixed(&nowhere), BTreeSet::from([nowhere]));
    }

    /// A claim gives up every name it holds, and no other.
    #[test]
    fn a_claim_gives_up_what_it_held() {
        let (one, two) = (Path::new("/nowhere/one"), Path::new("/nowhere/two"));
        let (first, second) = (Claim::take(one).unwrap(), Claim::take(two).unwrap());
        drop(first);
        assert_eq!(Claim::take(two).unwrap_err().kind, ErrorKind::AlreadyOpen);
        drop(Claim::take(one).unwrap());
        drop(second);
        drop(Claim::take(two).unwrap());
    }

    /// A thread that dies holding the lock leaves it poisoned; claims are still taken,
    /// refused and given up.
    #[test]
    fn a_poisoned_lock_is_used_all_the_same() {
        let died = std::thread::spawn(|| {
            let _held = open();
            panic!("on purpose, while holding the lock");
        })
        .join();
        assert!(died.is_err());
        assert!(OPEN.is_poisoned());
        let path = Path::new("/nowhere/poisoned.wombat");
        let claim = Claim::take(path).unwrap();
        assert_eq!(Claim::take(path).unwrap_err().kind, ErrorKind::AlreadyOpen);
        drop(claim);
        drop(Claim::take(path).unwrap());
    }
}
