//! Read-only importer of TreeSheets `.cts` files (decision B6): a testing tool that turns a
//! file the real TreeSheets wrote into a Wombat [`Document`], so that the core can be checked
//! against the real app on real documents.
//!
//! It reads every format version up to 31 (the examples go back to version 6) as TreeSheets'
//! own loader does, including what that loader repairs or ignores. What differs from the
//! file, on purpose:
//!
//! - **Links by text become links by ID** (decision B1), resolved once with TreeSheets' rule.
//!   A link whose target does not exist becomes a link with [`LinkTarget::Unknown`] and the
//!   target text in its `cts_target` field, so nothing is lost (decision B14).
//! - **Fold state, selection and zoom** go to a [`ViewState`] (decision A8).
//! - **Images** are returned as bytes with a file name; [`Imported::save`] writes them into the
//!   document's image folder. The display scale is copied to every cell showing the image.
//! - **A formula cell has its formula as text.** What it shows is worked out by the core
//!   (`wombat_core::eval`), here only to find the targets of links, which go by what cells
//!   show (decision A66).
//!
//! Arbitrary bytes are safe to pass in: every count and length is checked against the data
//! that is really there, the decompressed size is bounded, and nothing recurses.

mod links;
mod parse;
mod reader;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;
use wombat_core::construct::Construct;

use wombat_core::model::Extra;
use wombat_core::{
    storage, CellId, Document, Error, IdSource, Link, LinkTarget, Selection, ViewState,
};

/// The newest `.cts` format version this importer reads.
pub const NEWEST_VERSION: u8 = 31;
/// The largest `.cts` file that is read. It is not the limit on `.wombat` files, which is
/// lower: a `.cts` file carries its images inside it and a `.wombat` file does not.
pub const MAX_CTS_FILE_BYTES: u64 = 256 * 1024 * 1024;
/// The most bytes the compressed part of a file may decompress to.
pub const MAX_DECOMPRESSED_BYTES: usize = 256 * 1024 * 1024;
/// The most images one file may hold.
pub const MAX_IMAGES: usize = 65_536;
/// The field of a link that holds the target text of a `.cts` link to a cell that does not
/// exist.
pub const UNRESOLVED_TARGET_FIELD: &str = "cts_target";

/// Why a `.cts` file could not be imported.
#[derive(Debug)]
pub enum CtsError {
    /// The data does not start with the `.cts` signature.
    NotCts,
    /// The file was written by a newer TreeSheets (the format version is given).
    NewerVersion(u8),
    /// The file is password protected.
    Encrypted,
    /// The file is damaged or truncated (says how).
    Corrupt(String),
    /// The file is over a limit of the importer or of the Wombat core.
    Limit(String),
    /// The core refused the document, or a file could not be read or written.
    Core(Error),
}

impl fmt::Display for CtsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CtsError::NotCts => write!(f, "not a TreeSheets file"),
            CtsError::NewerVersion(found) => write!(
                f,
                "the file has TreeSheets format version {found}, but only versions up to \
                 {NEWEST_VERSION} are supported"
            ),
            CtsError::Encrypted => write!(f, "password-protected files are not supported"),
            CtsError::Corrupt(m) => write!(f, "damaged TreeSheets file: {m}"),
            CtsError::Limit(m) => write!(f, "over a limit: {m}"),
            CtsError::Core(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for CtsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CtsError::Core(error) => Some(error),
            _ => None,
        }
    }
}

impl From<Error> for CtsError {
    fn from(error: Error) -> CtsError {
        match error {
            Error::Limit(m) => CtsError::Limit(m),
            other => CtsError::Core(other),
        }
    }
}

/// The format of an image in a `.cts` file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImageKind {
    /// PNG (`I` in the file).
    Png,
    /// JPEG (`J` in the file).
    Jpeg,
}

impl ImageKind {
    /// The file extension for images of this format.
    pub fn extension(self) -> &'static str {
        match self {
            ImageKind::Png => "png",
            ImageKind::Jpeg => "jpg",
        }
    }
}

/// One image of a `.cts` file.
#[derive(Clone, PartialEq, Debug)]
pub struct Image {
    /// The file name the document's cells refer to it by.
    pub file: String,
    /// The format.
    pub kind: ImageKind,
    /// TreeSheets' display scale, limited to 1/16..100 as TreeSheets does (larger is drawn
    /// smaller); also on every [`wombat_core::ImageRef`] that shows the image.
    pub scale: f64,
    /// The bytes of the image file, as they are in the `.cts`, not decoded or checked.
    pub data: Vec<u8>,
    /// Whether any cell shows the image. Unused images are not written by [`Imported::save`].
    pub used: bool,
}

/// A link to a cell by text for which TreeSheets' rule finds no cell.
#[derive(Clone, PartialEq, Debug)]
pub struct UnresolvedLink {
    /// The cell that has the link.
    pub cell: CellId,
    /// The text the link wants its target to have.
    pub target: String,
}

/// The result of importing a `.cts` file.
#[derive(Clone, PartialEq, Debug)]
pub struct Imported {
    /// The document, with new IDs. It passes [`Document::validate`].
    pub document: Document,
    /// What the file says about folded grids, the selection and the zoom.
    pub view: ViewState,
    /// The images of the file, in file order.
    pub images: Vec<Image>,
    /// The `.cts` format version of the file.
    pub version: u8,
    /// The file's "export along with each save" setting for HTML: 0 none, 1 with images, 2
    /// without; 3 is not defined. Wombat has no such feature, so it is only reported here.
    pub autoexport_html: u8,
    /// The same setting for PDF.
    pub autoexport_pdf: bool,
    /// The internal links without a target, in document order (see [`UNRESOLVED_TARGET_FIELD`]).
    pub unresolved_links: Vec<UnresolvedLink>,
    /// How many strings in the file were not valid UTF-8. TreeSheets reads such a string as
    /// empty, and so does the importer; this says that it happened.
    pub undecodable_strings: usize,
    /// The depth of the deepest cell (the root is 0).
    pub max_depth: u32,
}

/// Imports the bytes of a `.cts` file. Every cell gets a new ID from `ids`, so the result is
/// the same whenever `ids` starts from the same seed.
pub fn import(bytes: &[u8], ids: &mut dyn IdSource) -> Result<Imported, CtsError> {
    import_bounded(bytes, ids, MAX_DECOMPRESSED_BYTES)
}

fn import_bounded(
    bytes: &[u8],
    ids: &mut dyn IdSource,
    max_decompressed: usize,
) -> Result<Imported, CtsError> {
    let parse::Parsed {
        version,
        mut document,
        mut images,
        folded,
        selected,
        selection_size,
        zoom_level,
        autoexport,
        text_links,
        undecodable,
    } = parse::parse(bytes, ids, max_decompressed)?;

    // Links by text to links by ID.
    let mut resolved: Vec<(CellId, Vec<Link>)> = Vec::new();
    let mut unresolved_links = Vec::new();
    let max_depth;
    {
        let wanted: BTreeSet<&str> = text_links
            .iter()
            .flat_map(|(_, links)| links.iter().map(|link| link.target.as_str()))
            .collect();
        let wanted: Vec<&str> = wanted.into_iter().collect();
        // What formula cells show, if there is any link to find a target for: only then
        // is the work of calculating needed.
        let calc = (!wanted.is_empty()).then(|| wombat_core::eval::Calc::new(&document));
        let resolver = links::Resolver::new(&document, &wanted, calc.as_ref());
        max_depth = resolver.max_depth();
        for (cell, links) in &text_links {
            let mut made = Vec::with_capacity(links.len());
            for link in links {
                let mut extra = Extra::new();
                let target = match resolver.resolve(*cell, &link.target) {
                    Some(to) => LinkTarget::Cell {
                        doc: None,
                        cell: to,
                    },
                    None => {
                        extra.insert(
                            UNRESOLVED_TARGET_FIELD.to_string(),
                            link.target.clone().into(),
                        );
                        unresolved_links.push(UnresolvedLink {
                            cell: *cell,
                            target: link.target.clone(),
                        });
                        LinkTarget::Unknown
                    }
                };
                made.push(Link {
                    start: link.start,
                    len: link.len,
                    target,
                    extra,
                });
            }
            resolved.push((*cell, made));
        }
    }
    for (id, mut made) in resolved {
        if let Some(cell) = document.cell_mut(id) {
            // The cell's external links are there already; both lists are sorted and they do
            // not overlap, so sorting by start merges them.
            cell.links.append(&mut made);
            cell.links.sort_by_key(|link| link.start);
        }
    }

    let used: BTreeSet<String> = storage::image_names(&document).into_iter().collect();
    for image in &mut images {
        image.used = used.contains(&image.file);
    }

    let mut view = ViewState::new(document.id());
    view.folded = folded.into_iter().collect();
    let (selection, zoom) = initial_view(&document, selected, selection_size, zoom_level);
    view.selection = selection;
    view.zoom = zoom;

    document.validate()?;
    Ok(Imported {
        document,
        view,
        images,
        version,
        autoexport_html: autoexport & 3,
        autoexport_pdf: autoexport & 4 != 0,
        unresolved_links,
        undecodable_strings: undecodable,
        max_depth,
    })
}

/// Where the cell is in the grid it is in: the cell that has that grid, column and row.
fn position(document: &Document, id: CellId) -> Option<(CellId, u32, u32)> {
    let parent = document.cell(id)?.parent()?;
    let grid = document.cell(parent)?.grid()?;
    let index = u32::try_from(grid.cells().iter().position(|c| *c == id)?).ok()?;
    Some((
        parent,
        index.checked_rem(grid.xs())?,
        index.checked_div(grid.xs())?,
    ))
}

/// The selection and zoom TreeSheets shows after loading (`Document::InitCellSelect`, then
/// `Document::Zoom`), from the marked cell, the selection size and the zoom depth in the file.
fn initial_view(
    document: &Document,
    marked: Option<CellId>,
    size: (u8, u8),
    zoom_level: u8,
) -> (Option<Selection>, Option<CellId>) {
    let select = |cell, xs, ys| Selection {
        cell,
        xs,
        ys,
        cursor: None,
        extra: Extra::new(),
    };
    let root = document.root();
    // Without a marked cell, or with the root marked: the first cell of the root's grid.
    let (cell, xs, ys) = match marked.and_then(|id| Some((id, position(document, id)?))) {
        Some((id, (parent, x, y))) => {
            let room = |size: u8, all: u32, at: u32| {
                u32::from(size).clamp(1, all.saturating_sub(at).max(1))
            };
            let grid = document.cell(parent).and_then(|cell| cell.grid());
            let (all_x, all_y) = grid.map_or((1, 1), |grid| (grid.xs(), grid.ys()));
            (id, room(size.0, all_x, x), room(size.1, all_y, y))
        }
        None => match document.child(root, 0, 0) {
            Some(first) => (first, 1, 1),
            None => return (None, None),
        },
    };
    let mut selection = select(cell, xs, ys);
    if zoom_level == 0 {
        return (Some(selection), None);
    }
    // Zoom into the selected cell if it is one cell with a grid, else into the cell around
    // it; but no deeper than the level in the file, counted from the root.
    let single = xs == 1 && ys == 1;
    let has_grid = |id| document.cell(id).is_some_and(|cell| cell.grid().is_some());
    let mut shown = if single && has_grid(cell) {
        cell
    } else {
        document.cell(cell).and_then(|c| c.parent()).unwrap_or(root)
    };
    let mut depth = document.depth(shown).unwrap_or(0);
    while depth > u32::from(zoom_level) {
        match document.cell(shown).and_then(|c| c.parent()) {
            Some(parent) => shown = parent,
            None => break,
        }
        depth = depth.saturating_sub(1);
    }
    if shown == root {
        return (Some(selection), None);
    }
    // "We can't have the drawroot selected, so we must move the selection to the children."
    if single && shown == cell {
        if let (Some(first), Some(grid)) = (
            document.child(shown, 0, 0),
            document.cell(shown).and_then(|c| c.grid()),
        ) {
            selection = select(first, grid.xs(), grid.ys());
        }
    }
    (Some(selection), Some(shown))
}

/// Reads and imports a `.cts` file.
pub fn import_file(path: &Path, ids: &mut dyn IdSource) -> Result<Imported, CtsError> {
    import(&storage::read_up_to(path, MAX_CTS_FILE_BYTES)?, ids)
}

impl Imported {
    /// Saves the result as the Wombat document `path`: the `.wombat` file, the images that
    /// cells show in its image folder, and the view state beside it.
    pub fn save(&self, path: &Path) -> Result<(), CtsError> {
        storage::save_document(&self.document, path)?;
        let folder = storage::images_dir(path);
        for image in self.images.iter().filter(|image| image.used) {
            std::fs::create_dir_all(&folder).map_err(|source| Error::Io {
                path: folder.clone(),
                source,
            })?;
            storage::write_atomic(&storage::image_path(path, &image.file)?, &image.data)?;
        }
        let folder = path.parent().unwrap_or(Path::new(""));
        storage::save_view_state(folder, &self.view)?;
        Ok(())
    }
}
