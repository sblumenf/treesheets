//! Writing an export to a file or a folder. With [`crate::storage`] and [`crate::scan`]
//! this is the file layer of the core; the texts come from [`crate::convert`].
//!
//! An export writes where the user said, under names that come partly from the document.
//! So the rules here are stricter than for saving a document:
//!
//! - **Nothing existing is replaced unless asked.** Without `overwrite`, a file that is
//!   there, or a folder that has anything in it, is an error.
//! - **No symbolic link is ever written through.** A link at the place of the file, of
//!   the folder or of a file in the folder is an error, also with `overwrite`; it is not
//!   followed and not removed. (Links further up the path, in the folders the user named,
//!   are the user's own.)
//! - **A file is complete or absent.** The bytes go to a temporary file that is created
//!   new beside the target (anything at that name is removed first, never opened), which
//!   is then renamed over the target. Renaming replaces a name; it does not follow a link
//!   that might have appeared there meanwhile.
//! - **Names in a folder are plain names.** A name with a path separator, `.`, `..` or
//!   nothing in it is refused, whatever made it.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use crate::error::{invalid, Error};

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// What is at `path` itself, a link not followed: `None` if nothing.
fn found(path: &Path) -> Result<Option<fs::FileType>, Error> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(meta.file_type())),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io(path)(error)),
    }
}

fn is_link(path: &Path) -> Error {
    Error::Refused(format!(
        "{} is a symbolic link; an export is not written through a link",
        path.display()
    ))
}

/// Puts `bytes` at `path` by way of a new temporary file and a rename.
fn replace(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let Some(name) = path.file_name() else {
        return invalid(format!("{} is not a file name", path.display()));
    };
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(".tmp");
    let temp = path.with_file_name(temp_name);
    let mut created = false;
    let written = (|| {
        match fs::remove_file(&temp) {
            Err(error) if error.kind() != ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        // `create_new` fails if anything is there, link or not, instead of following it.
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        created = true;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();
    written.map_err(|source| {
        if created {
            let _ = fs::remove_file(&temp);
        }
        io(path)(source)
    })
}

/// Writes one exported file. See the module documentation for what is refused.
pub fn write_file(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), Error> {
    match found(path)? {
        None => {}
        Some(kind) if kind.is_symlink() => return Err(is_link(path)),
        Some(kind) if kind.is_dir() => {
            return Err(Error::Refused(format!(
                "{} is a folder, not a file",
                path.display()
            )))
        }
        Some(_) if !overwrite => return Err(Error::AlreadyExists(path.to_path_buf())),
        Some(_) => {}
    }
    replace(path, bytes)
}

/// Whether `name` is one plain file name: no folder, not `.` or `..`, not empty.
fn is_plain_name(name: &str) -> bool {
    let mut parts = Path::new(name).components();
    matches!(
        (parts.next(), parts.next()),
        (Some(Component::Normal(part)), None) if part.to_str() == Some(name)
    ) && !name.contains(['/', '\\', '\0'])
}

/// Writes the files of an export (name, content) into the folder `dir`, which is created
/// if it is not there (its parent must be). See the module documentation for what is
/// refused: a folder that is a link, a folder with anything in it unless `overwrite`, a
/// link or a folder at the name of a file, a name that is not a plain name, a name twice.
/// With `overwrite`, files of the folder that the export does not name are left alone.
/// Everything is checked before the first file is written.
pub fn write_folder(dir: &Path, files: &[(&str, &[u8])], overwrite: bool) -> Result<(), Error> {
    let mut seen = std::collections::BTreeSet::new();
    for (name, _) in files {
        // Compared without case: on Windows and macOS two such names are one file.
        if !is_plain_name(name) || !seen.insert(name.to_lowercase()) {
            return invalid(format!("{name:?} cannot be the name of an exported file"));
        }
    }
    match found(dir)? {
        None => fs::create_dir(dir).map_err(io(dir))?,
        Some(kind) if kind.is_symlink() => return Err(is_link(dir)),
        Some(kind) if !kind.is_dir() => {
            return Err(Error::Refused(format!(
                "{} is a file, not a folder",
                dir.display()
            )))
        }
        Some(_) => {
            let mut entries = fs::read_dir(dir).map_err(io(dir))?;
            if !overwrite && entries.next().is_some() {
                return Err(Error::Refused(format!(
                    "the folder {} is not empty; nothing was written",
                    dir.display()
                )));
            }
        }
    }
    let targets: Vec<PathBuf> = files.iter().map(|(name, _)| dir.join(name)).collect();
    for target in &targets {
        match found(target)? {
            Some(kind) if kind.is_symlink() => return Err(is_link(target)),
            Some(kind) if kind.is_dir() => {
                return Err(Error::Refused(format!(
                    "{} is a folder, not a file",
                    target.display()
                )))
            }
            _ => {}
        }
    }
    for (target, (_, bytes)) in targets.iter().zip(files) {
        // Again, right before writing: the check above was for refusing early.
        if found(target)?.is_some_and(|kind| kind.is_symlink()) {
            return Err(is_link(target));
        }
        replace(target, bytes)?;
    }
    Ok(())
}
