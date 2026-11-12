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

/// The name under which the file that a save to `path` replaces is known, however `path`
/// is spelt: where links lead, without `.` and `..`, from the root.
///
/// A file that is not there yet has no such name; it is known by that of its folder and
/// its own, which is what it gets once it is there. If the folder is not there either, the
/// path from the root as it is spelt has to do: nothing can be opened or made there.
///
/// Two names that the file system itself takes for one are not seen to be the same: a
/// second hard link to a file, and a name that differs only by case, or by the way an
/// accented letter is written, on file systems that ignore that, for a file that is not
/// there yet.
fn key(path: &Path) -> PathBuf {
    if let Ok(real) = std::fs::canonicalize(path) {
        return real;
    }
    let spelt = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    match (spelt.parent().map(std::fs::canonicalize), spelt.file_name()) {
        (Some(Ok(folder)), Some(name)) => folder.join(name),
        _ => spelt,
    }
}

/// The right of one open document to its file. Dropping it gives the file up.
#[derive(Debug)]
pub(crate) struct Claim(PathBuf);

impl Claim {
    /// Claims the file at `path` for a document about to be opened or made there, or
    /// refuses because a document has it open. Checking and claiming are one step under
    /// the lock, so of two threads that ask at once one is refused.
    pub(crate) fn take(path: &Path) -> Result<Claim, BridgeError> {
        let key = key(path);
        if open().insert(key.clone()) {
            Ok(Claim(key))
        } else {
            Err(BridgeError {
                kind: ErrorKind::AlreadyOpen,
                message: format!("{}: the document is open already", path.display()),
                path: Some(path.display().to_string()),
            })
        }
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        open().remove(&self.0);
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
        assert_eq!(key(Path::new(name)), here.join(name));
        assert_eq!(key(&Path::new(".").join(name)), here.join(name));
        assert_eq!(key(&here.join("no-such-folder/..").join(name)), {
            // Nothing can be made in a folder that is not there; the spelling is kept.
            here.join("no-such-folder/..").join(name)
        });
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
