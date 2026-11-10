//! Files: the only module of the core that touches the file system (decision A1).
//!
//! Layout, for a document saved as `notes.wombat` in some folder:
//!
//! ```text
//! folder/
//!   notes.wombat              the document (see docs/FILE_FORMAT.md)
//!   notes.wombat.images/      its images, one file each, named by the document's cells
//!   .wombat-view/<doc id>.json   its view state on this device (folds, selection, zoom)
//! ```
//!
//! Images are never decoded here, only copied and referred to by file name. View state is
//! keyed by document ID, not file name, so renaming a document keeps it; the `.wombat-view`
//! folder can be left out of sync and backups, since it differs per device (decision A8).

use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use crate::error::{invalid, Error};
use crate::id::{DocId, IdSource};
use crate::limits::MAX_FILE_BYTES;
use crate::model::{is_valid_image_name, Document, ImageRef};
use crate::view::ViewState;

/// The file extension of documents, without the dot.
pub const EXTENSION: &str = "wombat";
/// The folder, beside the documents, that holds view-state files.
pub const VIEW_DIR: &str = ".wombat-view";

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn too_large(path: &Path, max: u64) -> Error {
    Error::Limit(format!("{} is larger than {max} bytes", path.display()))
}

/// Reads a whole file, refusing one larger than [`MAX_FILE_BYTES`] without reading it all.
pub fn read_bounded(path: &Path) -> Result<Vec<u8>, Error> {
    read_up_to(path, MAX_FILE_BYTES)
}

/// Reads a whole file, refusing one larger than `max` bytes without reading it all. For
/// files that are not Wombat's own, such as one to import, whose limit is the caller's.
pub fn read_up_to(path: &Path, max: u64) -> Result<Vec<u8>, Error> {
    let file = File::open(path).map_err(io(path))?;
    let mut bytes = Vec::new();
    // `take` also bounds files that do not report a length, such as devices.
    file.take(max.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io(path))?;
    if bytes.len() as u64 > max {
        return Err(too_large(path, max));
    }
    Ok(bytes)
}

/// The ID of the document in a `.wombat` file, from the file's header alone: the file is
/// read only as far as the header (see [`crate::format::read_id`]), so this is cheap for a
/// document of any size and does not show whether the rest of the file is sound.
pub fn document_id(path: &Path) -> Result<DocId, Error> {
    let file = File::open(path).map_err(io(path))?;
    if file.metadata().map_err(io(path))?.len() > MAX_FILE_BYTES {
        return Err(too_large(path, MAX_FILE_BYTES));
    }
    // `take` in case the file grows, or does not report its length.
    let reader = BufReader::new(file.take(MAX_FILE_BYTES));
    crate::format::read_id(reader).map_err(|error| match error {
        Error::Io { source, .. } => io(path)(source),
        other => other,
    })
}

fn temp_path(path: &Path) -> Result<PathBuf, Error> {
    match path.file_name() {
        Some(name) => {
            let mut temp = std::ffi::OsString::from(".");
            temp.push(name);
            temp.push(".tmp");
            Ok(path.with_file_name(temp))
        }
        None => invalid(format!("{} is not a file name", path.display())),
    }
}

/// Writes a file so that `path` holds either the old content or all of the new content, never
/// part of it: the bytes go to `.<name>.tmp` in the same folder, which is then renamed over
/// `path` (decisions A12 and A27).
///
/// - If `path` is a symbolic link, the file it leads to is the one replaced (with the
///   temporary file beside that file) and the link stays. A link that leads nowhere is an
///   error.
/// - Whatever is at the temporary name is removed first and the temporary file is created
///   new, so a link planted there is never written through.
/// - On Unix the new file gets the permissions of the file it replaces. Its owner and group
///   are those of whoever saves.
///
/// Two programs saving the same file at once are not protected from each other.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let is_link = fs::symlink_metadata(path).is_ok_and(|found| found.file_type().is_symlink());
    let target = if is_link {
        fs::canonicalize(path).map_err(io(path))?
    } else {
        path.to_path_buf()
    };
    let temp = temp_path(&target)?;
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
        #[cfg(unix)]
        if let Ok(old) = fs::metadata(&target) {
            file.set_permissions(old.permissions())?;
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, &target)
    })();
    if let Err(source) = written {
        // Best effort: the error to report is the one from writing. What was at the
        // temporary name and could not be replaced is not ours to remove.
        if created {
            let _ = fs::remove_file(&temp);
        }
        return Err(Error::Io {
            path: path.to_path_buf(),
            source,
        });
    }
    let path = target.as_path();
    // Make the rename itself durable where the platform lets a folder be synced.
    if let Some(Ok(dir)) = path.parent().map(File::open) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// The folder a file is in, as given: `""` for a bare file name.
fn folder_of(path: &Path) -> &Path {
    path.parent().unwrap_or(Path::new(""))
}

/// Saves a copy of the document in the file `from` as the new file `to`, which must not
/// exist. The copy shares no ID with the original ([`Document::with_fresh_ids`]): this, and
/// not copying the file, is how to duplicate a document, because two files with the same IDs
/// make links and search ambiguous (decision A28). The images the document shows are copied
/// to the image folder of `to`, and the view state on this device is copied for the new IDs.
/// Returns the copy.
pub fn copy_document(from: &Path, to: &Path, ids: &mut dyn IdSource) -> Result<Document, Error> {
    // Also true for a link that leads nowhere, which `exists` would not see.
    if fs::symlink_metadata(to).is_ok() {
        return Err(Error::AlreadyExists(to.to_path_buf()));
    }
    let original = load_document(from)?;
    let (copy, fresh) = original.with_fresh_ids(ids)?;
    // The document goes last, so that a copy that is there is complete.
    copy_images(&copy, from, to)?;
    // View state is a convenience: a missing or damaged one is not copied.
    if let Ok(Some(view)) = load_view_state(folder_of(from), original.id()) {
        save_view_state(folder_of(to), &view.with_fresh_ids(&fresh))?;
    }
    save_document(&copy, to)?;
    Ok(copy)
}

/// Loads a document from a `.wombat` file.
pub fn load_document(path: &Path) -> Result<Document, Error> {
    crate::format::from_slice(&read_bounded(path)?)
}

/// Saves a document to a `.wombat` file, atomically. An invalid document, or one too large
/// to be loaded again, is not written.
pub fn save_document(doc: &Document, path: &Path) -> Result<(), Error> {
    let text = crate::format::to_string(doc)?;
    write_atomic(path, text.as_bytes())
}

/// The folder that holds the images of the document saved at `doc_path`: the document's file
/// name with `.images` added, in the same folder.
pub fn images_dir(doc_path: &Path) -> PathBuf {
    let mut name = doc_path
        .file_name()
        .map(std::ffi::OsString::from)
        .unwrap_or_default();
    name.push(".images");
    doc_path.with_file_name(name)
}

/// The path of the image file `name` of the document saved at `doc_path`.
pub fn image_path(doc_path: &Path, name: &str) -> Result<PathBuf, Error> {
    if !is_valid_image_name(name) {
        return invalid(format!("{name:?} is not a valid image file name"));
    }
    Ok(images_dir(doc_path).join(name))
}

/// Copies the file `source` into the image folder of the document saved at `doc_path`, under
/// a new unique name that keeps the extension, and returns a reference to put in a cell.
pub fn add_image(
    doc_path: &Path,
    source: &Path,
    ids: &mut dyn IdSource,
) -> Result<ImageRef, Error> {
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty() && e.len() <= 8 && e.bytes().all(|b| b.is_ascii_alphanumeric()))
        .map(|e| format!(".{}", e.to_ascii_lowercase()))
        .unwrap_or_default();
    let name = format!("{}{extension}", ids.next_id());
    let dir = images_dir(doc_path);
    fs::create_dir_all(&dir).map_err(io(&dir))?;
    let target = image_path(doc_path, &name)?;
    fs::copy(source, &target).map_err(io(source))?;
    Ok(ImageRef::new(name))
}

/// The distinct image file names the document refers to, sorted.
pub fn image_names(doc: &Document) -> Vec<String> {
    let names: std::collections::BTreeSet<&str> = doc
        .cells_in_order()
        .into_iter()
        .filter_map(|id| doc.cell(id)?.image.as_ref())
        .map(|image| image.file.as_str())
        .collect();
    names.into_iter().map(String::from).collect()
}

/// The images the document refers to that are not in its image folder, sorted.
pub fn missing_images(doc: &Document, doc_path: &Path) -> Vec<String> {
    let dir = images_dir(doc_path);
    image_names(doc)
        .into_iter()
        .filter(|name| !dir.join(name).is_file())
        .collect()
}

/// Copies the images a document refers to from the image folder of one document path to that
/// of another, for "save as". Returns how many were copied; images that are missing at the
/// source are skipped.
pub fn copy_images(
    doc: &Document,
    from_doc_path: &Path,
    to_doc_path: &Path,
) -> Result<usize, Error> {
    let (from, to) = (images_dir(from_doc_path), images_dir(to_doc_path));
    let mut copied = 0usize;
    for name in image_names(doc) {
        let source = from.join(&name);
        if from == to || !source.is_file() {
            continue;
        }
        fs::create_dir_all(&to).map_err(io(&to))?;
        fs::copy(&source, to.join(&name)).map_err(io(&source))?;
        copied = copied.saturating_add(1);
    }
    Ok(copied)
}

/// The path of the view-state file of a document whose file is in `folder`.
pub fn view_state_path(folder: &Path, doc: DocId) -> PathBuf {
    folder.join(VIEW_DIR).join(format!("{doc}.json"))
}

/// Loads the view state of a document whose file is in `folder`; `None` if none was saved.
pub fn load_view_state(folder: &Path, doc: DocId) -> Result<Option<ViewState>, Error> {
    let path = view_state_path(folder, doc);
    if !path.exists() {
        return Ok(None);
    }
    let view = ViewState::from_json(&read_bounded(&path)?)?;
    if view.doc != doc {
        return invalid(format!(
            "{} is the view state of another document",
            path.display()
        ));
    }
    Ok(Some(view))
}

/// Saves the view state of a document whose file is in `folder`, atomically.
pub fn save_view_state(folder: &Path, view: &ViewState) -> Result<(), Error> {
    let text = view.to_json()?;
    let path = view_state_path(folder, view.doc);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io(dir))?;
    }
    write_atomic(&path, text.as_bytes())
}
