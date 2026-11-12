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
/// `path` (decisions A12 and A27). This is how a document is saved, at the name its user
/// chose:
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
    let target = real_path(path)?;
    let temp = write_temp(&target, &mut |file| file.write_all(bytes)).map_err(io(path))?;
    replace(&temp, &target).map_err(io(path))
}

/// As [`write_atomic`] for a file whose name the program chooses itself (an image in an
/// image folder, a view-state file): a link at that name is not what a user set up but
/// what someone planted, so it is replaced by the file and never written through
/// (decision A87).
pub fn write_own(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let temp = write_temp(path, &mut |file| file.write_all(bytes)).map_err(io(path))?;
    replace(&temp, path).map_err(io(path))
}

/// The file that saving to `path` replaces: where a link leads, or the path itself.
fn real_path(path: &Path) -> Result<PathBuf, Error> {
    let is_link = fs::symlink_metadata(path).is_ok_and(|found| found.file_type().is_symlink());
    if is_link {
        fs::canonicalize(path).map_err(io(path))
    } else {
        Ok(path.to_path_buf())
    }
}

/// Makes the temporary file beside `target` with what `fill` writes, ready to be renamed
/// over it. Nothing is left behind on an error.
fn write_temp(
    target: &Path,
    fill: &mut dyn FnMut(&mut File) -> std::io::Result<()>,
) -> std::io::Result<PathBuf> {
    let Ok(temp) = temp_path(target) else {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "not a file name",
        ));
    };
    match fs::remove_file(&temp) {
        Err(error) if error.kind() != ErrorKind::NotFound => return Err(error),
        _ => {}
    }
    // `create_new` fails if anything is there, link or not, instead of following it.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let written = (|| {
        #[cfg(unix)]
        if let Some(old) = fs::symlink_metadata(target)
            .ok()
            .filter(fs::Metadata::is_file)
        {
            file.set_permissions(old.permissions())?;
        }
        fill(&mut file)?;
        file.sync_all()
    })();
    match written {
        Ok(()) => Ok(temp),
        Err(error) => {
            // Best effort: the error to report is the one from writing.
            let _ = fs::remove_file(&temp);
            Err(error)
        }
    }
}

/// Renames the finished temporary file over `target`; on an error the temporary file is
/// removed.
fn replace(temp: &Path, target: &Path) -> std::io::Result<()> {
    if let Err(error) = fs::rename(temp, target) {
        let _ = fs::remove_file(temp);
        return Err(error);
    }
    sync_folder(target);
    Ok(())
}

/// Makes a rename durable where the platform lets a folder be synced.
fn sync_folder(path: &Path) {
    if let Some(Ok(dir)) = path.parent().map(File::open) {
        let _ = dir.sync_all();
    }
}

/// Makes sure that `dir`, a folder whose name the program chooses itself (an image folder,
/// the folder of view states), is there and is a real folder: one that is a link to
/// somewhere else is refused, so that nothing is written into a folder someone pointed it
/// at (decision A87).
fn own_dir(dir: &Path) -> Result<(), Error> {
    match fs::symlink_metadata(dir) {
        Ok(found) if found.is_dir() => Ok(()),
        Ok(_) => invalid(format!(
            "{} is a link or a file, not a folder; nothing is written there",
            dir.display()
        )),
        Err(error) if error.kind() == ErrorKind::NotFound => fs::create_dir(dir).map_err(io(dir)),
        Err(error) => Err(io(dir)(error)),
    }
}

/// Copies the file `source` to `target`, a name the program chooses itself, through a
/// temporary file: `target` has all of it or is as before, and a link there is replaced,
/// not followed.
fn copy_own(source: &Path, target: &Path) -> Result<(), Error> {
    let mut from = File::open(source).map_err(io(source))?;
    let temp = write_temp(target, &mut |file| {
        std::io::copy(&mut from, file).map(|_| ())
    })
    .map_err(io(target))?;
    replace(&temp, target).map_err(io(target))
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
///
/// If any of this fails, the image files, the view state and the folders for them that
/// were not there before the call are removed again. What was there is not removed (but an
/// image file of the same name that was in the folder of `to` has been replaced by then).
pub fn copy_document(from: &Path, to: &Path, ids: &mut dyn IdSource) -> Result<Document, Error> {
    // Also true for a link that leads nowhere, which `exists` would not see.
    if fs::symlink_metadata(to).is_ok() {
        return Err(Error::AlreadyExists(to.to_path_buf()));
    }
    let original = load_document(from)?;
    let (copy, fresh) = original.with_fresh_ids(ids)?;
    // What the copy may make besides the document, and is not there yet: to be removed
    // again if the copy fails, so that no images or view state stay without a document.
    let absent = |path: &PathBuf| fs::symlink_metadata(path).is_err();
    let mut files: Vec<PathBuf> = image_names(&copy)
        .iter()
        .filter_map(|name| image_path(to, name).ok())
        .collect();
    files.push(view_state_path(folder_of(to), copy.id()));
    files.retain(absent);
    let mut folders = vec![images_dir(to), folder_of(to).join(VIEW_DIR)];
    folders.retain(absent);
    let done = (|| {
        // The document goes last, so that a copy that is there is complete.
        copy_images(&copy, from, to)?;
        // View state is a convenience: a missing or damaged one is not copied.
        if let Ok(Some(view)) = load_view_state(folder_of(from), original.id()) {
            save_view_state(folder_of(to), &view.with_fresh_ids(&fresh))?;
        }
        save_document(&copy, to)
    })();
    match done {
        Ok(()) => Ok(copy),
        Err(error) => {
            // Best effort: the error to report is the one from copying. A folder that has
            // got something else in it meanwhile stays, since only an empty one is removed.
            for file in &files {
                let _ = fs::remove_file(file);
            }
            for folder in &folders {
                let _ = fs::remove_dir(folder);
            }
            Err(error)
        }
    }
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

/// A step of [`save_two`], named for [`save_two_checked`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SaveStep {
    /// Writing the new content of the first file beside it.
    WriteFirst,
    /// Writing the new content of the second file beside it.
    WriteSecond,
    /// Keeping the old content of the first file under another name.
    KeepFirst,
    /// Putting the new first file in the place of the old one.
    ReplaceFirst,
    /// Putting the new second file in the place of the old one.
    ReplaceSecond,
    /// Putting the old first file back, after the second could not be replaced.
    RestoreFirst,
}

fn backup_path(path: &Path) -> Result<PathBuf, Error> {
    match path.file_name() {
        Some(name) => {
            let mut backup = std::ffi::OsString::from(".");
            backup.push(name);
            backup.push(".bak");
            Ok(path.with_file_name(backup))
        }
        None => invalid(format!("{} is not a file name", path.display())),
    }
}

/// Saves two documents that belong together (the two sides of a move of cells between
/// documents) so that, as far as two files allow, either both files are the new ones or
/// both are as they were (decision A89):
///
/// 1. both are written out beside their files (`.<name>.tmp`); if that fails, nothing has
///    changed;
/// 2. the old first file is kept as `.<name>.bak` (a second name for the same file where
///    the file system can do that, else a copy);
/// 3. the new first file replaces the old one, then the new second file the old second;
/// 4. if the second cannot be replaced, the old first file is put back from the backup,
///    and the error is that of step 3: both files are as they were;
/// 5. if that fails too, the error is [`Error::HalfSaved`], which says which file is in
///    which state and where the old content of the first is.
///
/// Each of the two replacements is atomic; between them, for an instant, the first file is
/// new and the second old, and a crash right then leaves that with the backup beside it.
/// Links are treated as by [`write_atomic`]. An invalid document is not written.
pub fn save_two(first: (&Document, &Path), second: (&Document, &Path)) -> Result<(), Error> {
    save_two_checked(first, second, &mut |_| Ok(()))
}

/// [`save_two`], calling `before` ahead of every step it takes; an error from `before` is
/// taken as that step failing. For tests of what happens when a step fails, and for an
/// interface that shows progress.
pub fn save_two_checked(
    (first, first_path): (&Document, &Path),
    (second, second_path): (&Document, &Path),
    before: &mut dyn FnMut(SaveStep) -> std::io::Result<()>,
) -> Result<(), Error> {
    let (first_text, second_text) = (
        crate::format::to_string(first)?,
        crate::format::to_string(second)?,
    );
    let (first_target, second_target) = (real_path(first_path)?, real_path(second_path)?);
    if first_target == second_target {
        return invalid("the two documents are to be saved as the same file");
    }
    let backup = backup_path(&first_target)?;
    let mut write = |step, target: &Path, text: &str| {
        before(step)?;
        write_temp(target, &mut |file| file.write_all(text.as_bytes()))
    };
    let first_temp =
        write(SaveStep::WriteFirst, &first_target, &first_text).map_err(io(first_path))?;
    let second_temp = match write(SaveStep::WriteSecond, &second_target, &second_text) {
        Ok(temp) => temp,
        Err(error) => {
            let _ = fs::remove_file(&first_temp);
            return Err(io(second_path)(error));
        }
    };
    let give_up = |error: std::io::Error, path: &Path| {
        let _ = fs::remove_file(&first_temp);
        let _ = fs::remove_file(&second_temp);
        Err(io(path)(error))
    };
    // The old first file under a second name, unless there is none yet.
    let had_first = fs::symlink_metadata(&first_target).is_ok();
    let kept = before(SaveStep::KeepFirst).and_then(|()| {
        match fs::remove_file(&backup) {
            Err(error) if error.kind() != ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        if !had_first {
            return Ok(());
        }
        // Neither follows a link at the backup's name: both fail if anything is there.
        fs::hard_link(&first_target, &backup).or_else(|_| {
            let old = fs::read(&first_target)?;
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&backup)?;
            file.write_all(&old)?;
            file.sync_all()
        })
    });
    if let Err(error) = kept {
        let _ = fs::remove_file(&backup);
        return give_up(error, first_path);
    }
    let replaced =
        before(SaveStep::ReplaceFirst).and_then(|()| fs::rename(&first_temp, &first_target));
    if let Err(error) = replaced {
        let _ = fs::remove_file(&backup);
        return give_up(error, first_path);
    }
    let replaced =
        before(SaveStep::ReplaceSecond).and_then(|()| fs::rename(&second_temp, &second_target));
    let Err(source) = replaced else {
        let _ = fs::remove_file(&backup);
        sync_folder(&first_target);
        sync_folder(&second_target);
        return Ok(());
    };
    let _ = fs::remove_file(&second_temp);
    // The second file is as it was: so must the first be.
    let restored = before(SaveStep::RestoreFirst).and_then(|()| {
        if had_first {
            fs::rename(&backup, &first_target)
        } else {
            fs::remove_file(&first_target)
        }
    });
    match restored {
        Ok(()) => {
            sync_folder(&first_target);
            Err(io(second_path)(source))
        }
        Err(restore) => Err(Error::HalfSaved {
            saved: first_path.to_path_buf(),
            not_saved: second_path.to_path_buf(),
            backup: had_first.then_some(backup),
            source,
            restore,
        }),
    }
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
    own_dir(&dir)?;
    let target = image_path(doc_path, &name)?;
    copy_own(source, &target)?;
    Ok(ImageRef::new(name))
}

/// Writes `bytes` as the image file `name` of the document saved at `doc_path`, making the
/// image folder if it is not there. A link at the image's name is replaced by the file,
/// and an image folder that is a link is refused (decision A87).
pub fn write_image(doc_path: &Path, name: &str, bytes: &[u8]) -> Result<(), Error> {
    let target = image_path(doc_path, name)?;
    own_dir(&images_dir(doc_path))?;
    write_own(&target, bytes)
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
        // Also checks the name, which could come from a document built in memory.
        let source = image_path(from_doc_path, &name)?;
        if from == to || !source.is_file() {
            continue;
        }
        own_dir(&to)?;
        copy_own(&source, &image_path(to_doc_path, &name)?)?;
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
    own_dir(&folder.join(VIEW_DIR))?;
    write_own(&path, text.as_bytes())
}

/// What [`carry_images`] did with the image files that a clip's cells show.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Carried {
    /// The files that were copied into the target's image folder, by the name they have
    /// there.
    pub copied: Vec<String>,
    /// The names for which the target's folder already had a file with the same bytes.
    pub reused: Vec<String>,
    /// The images that have another name in the target (old name, new name), because its
    /// folder had another file under the old one. The clip was changed to say so.
    pub renamed: Vec<(String, String)>,
}

impl Carried {
    /// Removes the files that [`carry_images`] copied into the image folder of the
    /// document at `to` again: for when what was to follow (pasting, saving) failed.
    pub fn take_back(&self, to: &Path) {
        for name in &self.copied {
            if let Ok(path) = image_path(to, name) {
                // A link is removed itself; what it leads to is not touched.
                let _ = fs::remove_file(path);
            }
        }
    }
}

/// Whether two files have the same bytes.
fn same_bytes(a: &Path, b: &Path) -> std::io::Result<bool> {
    if fs::metadata(a)?.len() != fs::metadata(b)?.len() {
        return Ok(false);
    }
    let (mut a, mut b) = (
        BufReader::new(File::open(a)?),
        BufReader::new(File::open(b)?),
    );
    let (mut x, mut y) = ([0u8; 8192], [0u8; 8192]);
    loop {
        let n = a.read(&mut x)?;
        if n == 0 {
            return Ok(true);
        }
        let other = y.get_mut(..n).unwrap_or_default();
        b.read_exact(other)?;
        if x.get(..n) != Some(&*other) {
            return Ok(false);
        }
    }
}

/// The name of a regular file in an image folder, if it is one: not a link, not a folder.
fn plain_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|found| found.is_file())
}

/// Brings the image files that the cells of `clip` show from the image folder of the
/// document saved at `from` to that of the document saved at `to`, before the clip is
/// pasted or moved there (decision A88):
///
/// - a file that the target's folder does not have is copied;
/// - one that it has with the same bytes is used as it is;
/// - where it has another file of that name, the image is copied under a new name (an ID
///   from `ids`, with the old extension), and the cells of the clip are changed to show
///   that name, so that no cell of either document shows another picture than before.
///
/// Fails, with nothing copied, if a cell shows an image whose file is missing at the
/// source (or is a link or a folder there): a copy would show nothing, and after a move
/// the picture would be lost for good. Nothing is written through a link in the target's
/// folder. If what follows fails, [`Carried::take_back`] removes the copies again.
pub fn carry_images(
    clip: &mut crate::edit::Clip,
    from: &Path,
    to: &Path,
    ids: &mut dyn IdSource,
) -> Result<Carried, Error> {
    let mut carried = Carried::default();
    let names = clip.image_names();
    let (from_dir, to_dir) = (images_dir(from), images_dir(to));
    if names.is_empty() || from_dir == to_dir {
        return Ok(carried);
    }
    // First see that everything is there: nothing is copied if anything is missing.
    for name in &names {
        let source = image_path(from, name)?;
        if !plain_file(&source) {
            return Err(Error::NotFound(format!(
                "the image file {} that a cell shows; nothing was copied or changed",
                source.display()
            )));
        }
    }
    let done = (|| {
        own_dir(&to_dir)?;
        for name in &names {
            let source = image_path(from, name)?;
            let target = image_path(to, name)?;
            if fs::symlink_metadata(&target).is_err() {
                copy_own(&source, &target)?;
                carried.copied.push(name.clone());
            } else if plain_file(&target) && same_bytes(&source, &target).map_err(io(&target))? {
                carried.reused.push(name.clone());
            } else {
                // Another picture, or something that is no file: a name of its own, unless
                // an earlier copy of this picture got one already.
                let extension = Path::new(name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!(".{e}"))
                    .unwrap_or_default();
                let known = fs::read_dir(&to_dir)
                    .map_err(io(&to_dir))?
                    .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                    .filter(|other| other.ends_with(&extension) && is_valid_image_name(other))
                    .find(|other| {
                        let other = to_dir.join(other);
                        plain_file(&other) && same_bytes(&source, &other).unwrap_or(false)
                    });
                let new = match known {
                    Some(known) => {
                        carried.reused.push(known.clone());
                        known
                    }
                    None => {
                        let new = format!("{}{extension}", ids.next_id());
                        copy_own(&source, &image_path(to, &new)?)?;
                        carried.copied.push(new.clone());
                        new
                    }
                };
                clip.rename_image(name, &new)?;
                carried.renamed.push((name.clone(), new));
            }
        }
        Ok(())
    })();
    match done {
        Ok(()) => Ok(carried),
        Err(error) => {
            carried.take_back(to);
            Err(error)
        }
    }
}

/// Removes from the image folder of the document saved at `doc_path` those of the files
/// `names` that no cell of `doc` shows: the images of cells that moved to another
/// document, after both documents were saved. Returns the names removed. Only files of
/// that folder are touched; a link there is removed itself, never what it leads to.
pub fn remove_unused_images(
    doc: &Document,
    doc_path: &Path,
    names: &[String],
) -> Result<Vec<String>, Error> {
    let used = image_names(doc);
    let mut removed = Vec::new();
    for name in names.iter().filter(|name| !used.contains(*name)) {
        let path = image_path(doc_path, name)?;
        match fs::remove_file(&path) {
            Ok(()) => removed.push(name.clone()),
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(io(&path)(error)),
        }
    }
    Ok(removed)
}

/// What [`transfer_cell`] did.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Transferred {
    /// The number of cells that were copied or moved, the top cell included.
    pub cells: usize,
    /// What became of the image files.
    pub images: Carried,
    /// After a move: the image files removed from the source's folder, which no cell
    /// there shows any more.
    pub removed_images: Vec<String>,
    /// After a move: the folds that went with the cells.
    pub folds: usize,
}

/// Copies (or with `moving` moves) the cell `cell` of the document saved at `from`, with
/// everything in it, to the cell `place` of the document saved at `to`, and saves what
/// changed: the whole of [`crate::links::copy_between`] or
/// [`crate::links::move_between`] for two files (decision A88). With the cells go their
/// image files ([`carry_images`]), the tags they have, and on a move the folds of this
/// device's view state. A move saves both documents with [`save_two`], so that the cells
/// are not left in both files or in neither, and then removes from the source's image
/// folder the files that nothing there shows any more.
///
/// If anything fails before the documents are saved, the image files that were copied
/// are removed again and both documents are as they were. The view state and the
/// removal of images come after the documents; a failure there is reported, and leaves a
/// fold or an unused image file behind, nothing worse.
pub fn transfer_cell(
    from: &Path,
    cell: crate::id::CellId,
    to: &Path,
    place: crate::id::CellId,
    moving: bool,
    ids: &mut dyn IdSource,
) -> Result<Transferred, Error> {
    use crate::edit::{Clip, Editor};
    let source = load_document(from)?;
    let target = load_document(to)?;
    let mut clip = Clip::of(&source, cell)?;
    let moved: Vec<crate::id::CellId> = clip.cell_ids().collect();
    let names = clip.image_names();
    let mut target = Editor::new(target)?;
    let images = carry_images(&mut clip, from, to, ids)?;
    let saved = (|| {
        if !moving {
            crate::links::paste_clip(&source, clip, &mut target, place, ids)?;
            save_document(target.document(), to)?;
            return Ok(None);
        }
        let mut source = Editor::new(source)?;
        crate::links::move_clip(&mut source, clip, &mut target, place, None, ids)?;
        // The target first: should the worst happen (see `save_two`), the cells are in
        // both files, not in neither.
        save_two((target.document(), to), (source.document(), from))?;
        Ok(Some(source))
    })();
    let source = match saved {
        Ok(source) => source,
        Err(error) => {
            images.take_back(to);
            return Err(error);
        }
    };
    let mut done = Transferred {
        cells: moved.len(),
        images,
        ..Transferred::default()
    };
    let Some(source) = source else {
        return Ok(done);
    };
    done.removed_images = remove_unused_images(source.document(), from, &names)?;
    // The folds go with the cells, if both documents are in one folder's view states or
    // not: each has its own file.
    let (from_dir, to_dir) = (folder_of(from), folder_of(to));
    if let Some(mut old) = load_view_state(from_dir, source.document().id())? {
        let id = target.document().id();
        let mut new = load_view_state(to_dir, id)?.unwrap_or_else(|| ViewState::new(id));
        done.folds = new.take_folds(&mut old, moved);
        old.retain_existing(source.document());
        if done.folds > 0 {
            save_view_state(to_dir, &new)?;
        }
        save_view_state(from_dir, &old)?;
    }
    Ok(done)
}
