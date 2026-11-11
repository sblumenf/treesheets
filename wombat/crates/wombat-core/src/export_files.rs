//! Writing an export to a file or a folder. With [`crate::storage`] and [`crate::scan`]
//! this is the file layer of the core; the texts come from [`crate::convert`].
//!
//! An export writes where the user said, under names that come partly from the document.
//! So the rules here are stricter than for saving a document (decisions A103 and A116):
//!
//! - **Nothing existing is replaced unless asked.** Without `overwrite`, a file that is
//!   there, or a folder that has anything in it, is an error.
//! - **Only a regular file is ever replaced.** A device, a FIFO, a socket, a folder or a
//!   symbolic link at the place of a file is an error, also with `overwrite`; it is not
//!   opened, not followed and not removed.
//! - **No symbolic link is ever written through**, however the path is written: a
//!   separator or `/.` after the name of a link does not make it the folder it leads to.
//!   (Links further up the path, in the folders the user named, are the user's own.)
//! - **A file is complete or absent.** The bytes go to a temporary file that is created
//!   new beside the target (anything at that name is removed first, never opened), which
//!   is then renamed over the target (`storage::write_own`, as for every other file at a
//!   name where no link is followed). Renaming replaces a name; it does not follow a link
//!   that might have appeared there meanwhile.
//! - **Names in a folder are plain names.** A name with a path separator, `.`, `..` or
//!   nothing in it is refused, whatever made it.
//! - **A folder is written into again only if it is an export of Wombat's**, and then the
//!   tables of the export before are removed: see [`write_folder`].

use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use crate::convert::{lookup, MAX_EXPORT_BYTES};
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

/// Puts `bytes` at `path` by way of a new temporary file and a rename: the one way in
/// which the storage layer writes a file at a name where no link is to be followed
/// ([`crate::storage::write_own`], decision A87).
fn replace(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    crate::storage::write_own(path, bytes)
}

/// `path` as its parts say it, without separators or `.` at its end: `link/` and `link/.`
/// name the folder a link leads to, and asked about they would never be seen to be a link.
fn named(path: &Path) -> Result<PathBuf, Error> {
    let named: PathBuf = path.components().collect();
    if named.as_os_str().is_empty() {
        return invalid(format!("{:?} is not the name of a file", path.display()));
    }
    Ok(named)
}

/// Refuses what is at the place of a file to write unless it is nothing or a regular file.
fn check_file(path: &Path) -> Result<Option<fs::FileType>, Error> {
    let kind = found(path)?;
    match kind {
        Some(kind) if kind.is_symlink() => Err(is_link(path)),
        Some(kind) if kind.is_dir() => Err(Error::Refused(format!(
            "{} is a folder, not a file",
            path.display()
        ))),
        Some(kind) if !kind.is_file() => Err(Error::Refused(format!(
            "{} is not a regular file (a device, a pipe or a socket); an export replaces \
             nothing else",
            path.display()
        ))),
        _ => Ok(kind),
    }
}

/// Writes one exported file. See the module documentation for what is refused.
pub fn write_file(path: &Path, bytes: &[u8], overwrite: bool) -> Result<(), Error> {
    let path = &named(path)?;
    if check_file(path)?.is_some() && !overwrite {
        return Err(Error::AlreadyExists(path.clone()));
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

/// The tables that the export before left in `dir`, as its `index.csv` lists them: only
/// plain names that end in `.csv`. An error if the folder has no such index: then it is
/// not a folder that Wombat exported, and nothing in it is touched.
fn tables_before(dir: &Path) -> Result<Vec<String>, Error> {
    let index = dir.join(lookup::INDEX_FILE);
    let not_ours = || {
        Error::Refused(format!(
            "the folder {} is not empty and has no {} of a Wombat export in it; nothing was \
             written",
            dir.display(),
            lookup::INDEX_FILE
        ))
    };
    if !found(&index)?.is_some_and(|kind| kind.is_file()) {
        return Err(not_ours());
    }
    let bytes = crate::storage::read_up_to(&index, MAX_EXPORT_BYTES as u64)?;
    let tables = std::str::from_utf8(&bytes)
        .ok()
        .and_then(lookup::index_tables)
        .ok_or_else(not_ours)?;
    Ok(tables
        .into_iter()
        .filter(|name| is_plain_name(name) && name.ends_with(".csv"))
        .collect())
}

/// Writes the files of an export (name, content) into the folder `dir`, in the order
/// given, so that an index given last is there only when the rest is. The folder is
/// created if it is not there (its parent must be). Refused: a folder that is a link, a
/// folder with anything in it unless `overwrite`, anything but a regular file at the name
/// of a file, a name that is not a plain name, a name twice.
///
/// With `overwrite`, a folder that has anything in it must be one that Wombat exported:
/// it has an `index.csv` of a lookup export. The tables which that index lists and which
/// this export does not write again are removed first, so that no table of a grid that is
/// gone stays beside the new index and still joins. Only those: a name the old index
/// lists is removed if it is a plain name ending in `.csv` of a regular file in this
/// folder; every other file is left alone.
///
/// Everything is checked before anything is removed or written. Not atomic as a whole:
/// if writing stops half way, the folder has the old index (or none) beside some new
/// tables, and exporting into it again with `overwrite` puts it right.
pub fn write_folder(dir: &Path, files: &[(&str, &[u8])], overwrite: bool) -> Result<(), Error> {
    let mut seen = std::collections::BTreeSet::new();
    for (name, _) in files {
        // Compared without case: on Windows and macOS two such names are one file.
        if !is_plain_name(name) || !seen.insert(name.to_lowercase()) {
            return invalid(format!("{name:?} cannot be the name of an exported file"));
        }
    }
    let dir = &named(dir)?;
    let mut stale = Vec::new();
    let mut make = false;
    match found(dir)? {
        None => make = true,
        Some(kind) if kind.is_symlink() => return Err(is_link(dir)),
        Some(kind) if !kind.is_dir() => {
            return Err(Error::Refused(format!(
                "{} is a file, not a folder",
                dir.display()
            )))
        }
        Some(_) => {
            let mut entries = fs::read_dir(dir).map_err(io(dir))?;
            if entries.next().is_some() {
                if !overwrite {
                    return Err(Error::Refused(format!(
                        "the folder {} is not empty; nothing was written",
                        dir.display()
                    )));
                }
                for name in tables_before(dir)? {
                    let path = dir.join(&name);
                    let regular = found(&path)?.is_some_and(|kind| kind.is_file());
                    if regular && !seen.contains(&name.to_lowercase()) {
                        stale.push(path);
                    }
                }
            }
        }
    }
    let targets: Vec<PathBuf> = files.iter().map(|(name, _)| dir.join(name)).collect();
    for target in &targets {
        check_file(target)?;
    }
    if make {
        fs::create_dir(dir).map_err(io(dir))?;
    }
    for path in &stale {
        fs::remove_file(path).map_err(io(path))?;
    }
    for (target, (_, bytes)) in targets.iter().zip(files) {
        // Again, right before writing: the check above was for refusing early.
        check_file(target)?;
        replace(target, bytes)?;
    }
    Ok(())
}
