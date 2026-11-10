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

use std::fs::{self, File};
use std::io::{Read, Write};
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

/// Reads a whole file, refusing one larger than [`MAX_FILE_BYTES`] without reading it all.
pub fn read_bounded(path: &Path) -> Result<Vec<u8>, Error> {
    let file = File::open(path).map_err(io(path))?;
    let mut bytes = Vec::new();
    // `take` also bounds files that do not report a length, such as devices.
    file.take(MAX_FILE_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io(path))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(Error::Limit(format!(
            "{} is larger than {MAX_FILE_BYTES} bytes",
            path.display()
        )));
    }
    Ok(bytes)
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
/// `path` (decision A12). Two programs saving the same file at once are not protected from
/// each other.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let temp = temp_path(path)?;
    let written = (|| {
        let mut file = File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();
    if let Err(source) = written {
        // Best effort: the error to report is the one from writing.
        let _ = fs::remove_file(&temp);
        return Err(Error::Io {
            path: path.to_path_buf(),
            source,
        });
    }
    // Make the rename itself durable where the platform lets a folder be synced.
    if let Some(Ok(dir)) = path.parent().map(File::open) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// Loads a document from a `.wombat` file.
pub fn load_document(path: &Path) -> Result<Document, Error> {
    crate::format::from_slice(&read_bounded(path)?)
}

/// Saves a document to a `.wombat` file, atomically. An invalid document is not written.
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
