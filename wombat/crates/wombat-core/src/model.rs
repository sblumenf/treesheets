//! The in-memory document.
//!
//! A document is a tree: a root cell; a cell has text and optionally a grid; a grid is `xs` by
//! `ys` cells in row-major order. Cells live in one map from [`CellId`] to [`Cell`], and each
//! cell knows its parent, so finding a cell or its parent by ID does not depend on the size or
//! depth of the document. A grid holds the IDs of its cells, not the cells themselves.
//!
//! The functions here that change a document ([`Document::add_grid`], [`Document::cell_mut`])
//! are the **low-level layer**. From milestone 3 on, documents change only through commands
//! built on top of it (decision A9); until then tests, the command-line tool and the importer
//! use it directly to construct documents.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use serde_json::Value;

use crate::error::{invalid, Error};
use crate::id::{CellId, DocId, IdSource};
use crate::limits::{MAX_CELLS, MAX_DEPTH, MAX_IMAGE_NAME_BYTES, MAX_TEXT_CODE_POINTS};

/// Fields the core does not recognise, kept so that they are written back (decision A7).
pub type Extra = BTreeMap<String, Value>;

/// An opaque colour: red, green and blue.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
}

impl Color {
    /// White, the default cell colour.
    pub const WHITE: Color = Color::rgb(0xff, 0xff, 0xff);
    /// Black, the default text colour.
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    /// Grey, the default grid border colour.
    pub const BORDER: Color = Color::rgb(0xa0, 0xa0, 0xa0);

    /// Makes a colour from its three parts.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }

    /// Reads `#rrggbb` (hexadecimal digits in either case).
    pub fn parse(text: &str) -> Option<Color> {
        let hex = text.strip_prefix('#')?;
        if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let n = u32::from_str_radix(hex, 16).ok()?;
        Some(Color::rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

macro_rules! named_enum {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident = $text:literal,)+ }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
        pub enum $name {
            $($(#[$vdoc])* $variant,)+
        }

        impl $name {
            /// Every value, the default first.
            pub const ALL: &'static [$name] = &[$($name::$variant,)+];

            /// The name used in files.
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $text,)+ }
            }

            /// Reads the name used in files.
            pub fn parse(text: &str) -> Option<$name> {
                match text { $($text => Some($name::$variant),)+ _ => None }
            }
        }
    };
}

named_enum!(
    /// What a cell is to the formula system (TreeSheets' cell types).
    CellKind {
        /// Plain data.
        #[default]
        Data = "data",
        /// An operation.
        Code = "code",
        /// Assigns its value to the variable named by its text.
        VarAssign = "var_assign",
        /// Reads the variable named by its text.
        VarRead = "var_read",
        /// A horizontal view.
        ViewH = "view_h",
        /// A vertical view.
        ViewV = "view_v",
        /// A formula; the cell's text is the formula itself.
        Formula = "formula",
    }
);

named_enum!(
    /// How the grid of a cell is drawn.
    DrawStyle {
        /// A grid with borders.
        #[default]
        Grid = "grid",
        /// Bubbles in a hierarchy.
        Bubbles = "bubbles",
        /// A hierarchy joined by lines.
        Lines = "lines",
    }
);

named_enum!(
    /// Horizontal alignment of a cell's content.
    TextAlign {
        /// Right for right-to-left text, left otherwise.
        #[default]
        Auto = "auto",
        /// Left.
        Left = "left",
        /// Centred.
        Center = "center",
        /// Right.
        Right = "right",
    }
);

named_enum!(
    /// Vertical alignment of a cell's content.
    VertAlign {
        /// Decided by the draw style, as TreeSheets does.
        #[default]
        Auto = "auto",
        /// Top.
        Top = "top",
        /// Middle.
        Middle = "middle",
        /// Bottom.
        Bottom = "bottom",
    }
);

/// Text style switches. On a cell they are the base style of its text; on a run they are the
/// complete style of the run (not relative to the base).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Style {
    /// Bold.
    pub bold: bool,
    /// Italic.
    pub italic: bool,
    /// Fixed-width font.
    pub fixed: bool,
    /// Underlined.
    pub underline: bool,
    /// Struck through.
    pub strike: bool,
}

/// A range of a cell's text that is styled differently from the base style.
#[derive(Clone, PartialEq, Debug)]
pub struct Run {
    /// Position of the first code point.
    pub start: u32,
    /// Number of code points, at least 1.
    pub len: u32,
    /// The complete style of the range.
    pub style: Style,
    /// The colour of the range, or `None` for the cell's text colour.
    pub color: Option<Color>,
    /// Unrecognised fields.
    pub extra: Extra,
}

/// What a link points at (decision B1).
#[derive(Clone, PartialEq, Debug)]
pub enum LinkTarget {
    /// A URL with a scheme, or else a file path, absolute or relative to the document.
    External(String),
    /// A cell, by its permanent ID.
    Cell {
        /// The document the cell is in, or `None` for the document that holds the link.
        doc: Option<DocId>,
        /// The cell.
        cell: CellId,
    },
    /// A kind of target this core does not know; what it is is in the link's `extra`.
    Unknown,
}

/// A range of a cell's text that is a link, independent of the runs it may span.
#[derive(Clone, PartialEq, Debug)]
pub struct Link {
    /// Position of the first code point.
    pub start: u32,
    /// Number of code points, at least 1.
    pub len: u32,
    /// Where the link goes.
    pub target: LinkTarget,
    /// Unrecognised fields.
    pub extra: Extra,
}

/// A reference to an image file kept beside the document (see [`crate::storage`]).
#[derive(Clone, PartialEq, Debug)]
pub struct ImageRef {
    /// The file name inside the document's image folder.
    pub file: String,
    /// The display scale; 1.0 is the default.
    pub scale: f64,
    /// Unrecognised fields.
    pub extra: Extra,
}

impl ImageRef {
    /// A reference to `file` at the default scale.
    pub fn new(file: impl Into<String>) -> ImageRef {
        ImageRef {
            file: file.into(),
            scale: 1.0,
            extra: Extra::new(),
        }
    }
}

/// A grid of cells inside a cell. Its size and cells change only through [`Document`].
#[derive(Clone, PartialEq, Debug)]
pub struct Grid {
    pub(crate) xs: u32,
    pub(crate) ys: u32,
    pub(crate) cells: Vec<CellId>,
    /// The colour of the border.
    pub border_color: Color,
    /// Space around the grid; the default is [`Grid::DEFAULT_OUTER_SPACING`].
    pub outer_spacing: u32,
    /// Whether the grid is below the cell's text (the default) instead of beside it.
    pub vertical: bool,
    /// The width at which text wraps, per column; `xs` entries, by default
    /// [`Grid::DEFAULT_COL_WIDTH`].
    pub col_widths: Vec<u32>,
    /// Unrecognised fields.
    pub extra: Extra,
}

impl Grid {
    /// The default of [`Grid::outer_spacing`].
    pub const DEFAULT_OUTER_SPACING: u32 = 3;
    /// The default of each entry of [`Grid::col_widths`].
    pub const DEFAULT_COL_WIDTH: u32 = 80;

    pub(crate) fn new(xs: u32, ys: u32, cells: Vec<CellId>) -> Grid {
        Grid {
            xs,
            ys,
            cells,
            border_color: Color::BORDER,
            outer_spacing: Grid::DEFAULT_OUTER_SPACING,
            vertical: true,
            col_widths: vec![Grid::DEFAULT_COL_WIDTH; xs as usize],
            extra: Extra::new(),
        }
    }

    /// The number of columns.
    pub fn xs(&self) -> u32 {
        self.xs
    }

    /// The number of rows.
    pub fn ys(&self) -> u32 {
        self.ys
    }

    /// The cells, row by row.
    pub fn cells(&self) -> &[CellId] {
        &self.cells
    }

    /// The cell in column `x` of row `y`, both counted from 0.
    pub fn at(&self, x: u32, y: u32) -> Option<CellId> {
        if x >= self.xs || y >= self.ys {
            return None;
        }
        let index = (y as usize)
            .checked_mul(self.xs as usize)?
            .checked_add(x as usize)?;
        self.cells.get(index).copied()
    }
}

/// A cell: text with its formatting, and optionally a grid.
#[derive(Clone, PartialEq, Debug)]
pub struct Cell {
    pub(crate) id: CellId,
    pub(crate) parent: Option<CellId>,
    pub(crate) grid: Option<Grid>,
    /// The text. For a formula cell this is the formula.
    pub text: String,
    /// What the cell is to the formula system.
    pub kind: CellKind,
    /// The background colour.
    pub cell_color: Color,
    /// The colour of the text outside of coloured runs.
    pub text_color: Color,
    /// How the cell's grid is drawn.
    pub draw_style: DrawStyle,
    /// A note attached to the cell.
    pub note: String,
    /// Horizontal alignment.
    pub text_align: TextAlign,
    /// Vertical alignment.
    pub vert_align: VertAlign,
    /// Text size relative to the default; as in TreeSheets, positive is smaller.
    pub rel_size: i32,
    /// The base style of the text.
    pub style: Style,
    /// Rich text: sorted, not overlapping, inside the text.
    pub runs: Vec<Run>,
    /// Links: sorted, not overlapping, inside the text.
    pub links: Vec<Link>,
    /// When the text was last edited, in milliseconds since 1970-01-01 00:00 UTC.
    pub last_edit: Option<i64>,
    /// An image shown in the cell.
    pub image: Option<ImageRef>,
    /// Unrecognised fields.
    pub extra: Extra,
}

impl Cell {
    pub(crate) fn new(id: CellId, parent: Option<CellId>) -> Cell {
        Cell {
            id,
            parent,
            grid: None,
            text: String::new(),
            kind: CellKind::Data,
            cell_color: Color::WHITE,
            text_color: Color::BLACK,
            draw_style: DrawStyle::Grid,
            note: String::new(),
            text_align: TextAlign::Auto,
            vert_align: VertAlign::Auto,
            rel_size: 0,
            style: Style::default(),
            runs: Vec::new(),
            links: Vec::new(),
            last_edit: None,
            image: None,
            extra: Extra::new(),
        }
    }

    /// The permanent ID.
    pub fn id(&self) -> CellId {
        self.id
    }

    /// The cell whose grid this cell is in, or `None` for the root.
    pub fn parent(&self) -> Option<CellId> {
        self.parent
    }

    /// The grid, if the cell has one.
    pub fn grid(&self) -> Option<&Grid> {
        self.grid.as_ref()
    }

    /// The grid, to change its properties (not its size or cells).
    pub fn grid_mut(&mut self) -> Option<&mut Grid> {
        self.grid.as_mut()
    }
}

/// The colours of a document-level tag.
#[derive(Clone, PartialEq, Debug)]
pub struct Tag {
    /// The background colour of cells with this tag.
    pub cell_color: Color,
    /// The text colour of cells with this tag.
    pub text_color: Color,
    /// Unrecognised fields.
    pub extra: Extra,
}

impl Default for Tag {
    fn default() -> Tag {
        Tag {
            cell_color: Color::WHITE,
            text_color: Color::BLACK,
            extra: Extra::new(),
        }
    }
}

/// One document.
#[derive(Clone, PartialEq, Debug)]
pub struct Document {
    pub(crate) id: DocId,
    pub(crate) minor: u32,
    pub(crate) root: CellId,
    pub(crate) cells: HashMap<CellId, Cell>,
    /// The tags of the document, by name. A cell has a tag when its text is the tag's name.
    pub tags: BTreeMap<String, Tag>,
    /// Unrecognised document-level fields.
    pub extra: Extra,
}

impl Document {
    /// Makes a document with a new ID and an empty root cell.
    pub fn new(ids: &mut dyn IdSource) -> Document {
        let id = DocId(ids.next_id());
        let root = CellId(ids.next_id());
        let mut cells = HashMap::new();
        cells.insert(root, Cell::new(root, None));
        Document {
            id,
            minor: 0,
            root,
            cells,
            tags: BTreeMap::new(),
            extra: Extra::new(),
        }
    }

    /// The permanent ID of the document.
    pub fn id(&self) -> DocId {
        self.id
    }

    /// The root cell.
    pub fn root(&self) -> CellId {
        self.root
    }

    /// The number of cells, the root included.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// The cell with this ID.
    pub fn cell(&self, id: CellId) -> Option<&Cell> {
        self.cells.get(&id)
    }

    /// Low-level layer: the cell with this ID, to set its text and properties. Call
    /// [`Document::validate`] afterwards; saving does so and refuses an invalid document.
    pub fn cell_mut(&mut self, id: CellId) -> Option<&mut Cell> {
        self.cells.get_mut(&id)
    }

    /// The depth of a cell: 0 for the root, 1 for the cells of its grid, and so on.
    pub fn depth(&self, id: CellId) -> Option<u32> {
        let mut cell = self.cells.get(&id)?;
        let mut depth = 0u32;
        while let Some(parent) = cell.parent {
            cell = self.cells.get(&parent)?;
            depth = depth.checked_add(1)?;
            // A parent chain longer than the cell count would be a cycle.
            if depth as usize > self.cells.len() {
                return None;
            }
        }
        Some(depth)
    }

    /// Low-level layer: gives a cell that has no grid a grid of `xs` by `ys` new empty cells.
    pub fn add_grid(
        &mut self,
        cell: CellId,
        xs: u32,
        ys: u32,
        ids: &mut dyn IdSource,
    ) -> Result<(), Error> {
        let Some(depth) = self.depth(cell) else {
            return Err(Error::NotFound(format!("cell {cell}")));
        };
        if self.cells.get(&cell).is_some_and(|c| c.grid.is_some()) {
            return invalid(format!("cell {cell} already has a grid"));
        }
        if xs == 0 || ys == 0 {
            return invalid("a grid has at least one column and one row");
        }
        if depth >= MAX_DEPTH {
            return Err(Error::Limit(format!(
                "grids nested deeper than {MAX_DEPTH}"
            )));
        }
        let count = (xs as usize)
            .checked_mul(ys as usize)
            .filter(|n| {
                self.cells
                    .len()
                    .checked_add(*n)
                    .is_some_and(|total| total <= MAX_CELLS)
            })
            .ok_or_else(|| Error::Limit(format!("more than {MAX_CELLS} cells")))?;
        let mut children = Vec::with_capacity(count);
        for _ in 0..count {
            let id = CellId(ids.next_id());
            if id.0 == self.id.0 || self.cells.contains_key(&id) {
                return invalid(format!("the ID source repeated the ID {id}"));
            }
            self.cells.insert(id, Cell::new(id, Some(cell)));
            children.push(id);
        }
        if let Some(c) = self.cells.get_mut(&cell) {
            c.grid = Some(Grid::new(xs, ys, children));
        }
        Ok(())
    }

    /// The cell at column `x`, row `y` of the grid of `cell`.
    pub fn child(&self, cell: CellId, x: u32, y: u32) -> Option<CellId> {
        self.cells.get(&cell)?.grid.as_ref()?.at(x, y)
    }

    /// All cells in document order: a cell, then the cells of its grid row by row, each
    /// followed by its own grid's cells.
    pub fn cells_in_order(&self) -> Vec<CellId> {
        let mut order = Vec::with_capacity(self.cells.len());
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            // A valid document never revisits a cell; this keeps a broken one from looping.
            if order.len() >= self.cells.len() {
                break;
            }
            order.push(id);
            if let Some(grid) = self.cells.get(&id).and_then(|c| c.grid.as_ref()) {
                stack.extend(grid.cells.iter().rev());
            }
        }
        order
    }

    /// Checks every rule of the model: the tree is consistent, and each cell's runs, links,
    /// image and grid properties are well-formed. Loading and saving both call this.
    pub fn validate(&self) -> Result<(), Error> {
        check_extra(&self.extra, crate::format::DOCUMENT_KEYS, "the document")?;
        for (name, tag) in &self.tags {
            check_extra(
                &tag.extra,
                crate::format::TAG_KEYS,
                &format!("tag {name:?}"),
            )?;
        }
        if self.cells.len() > MAX_CELLS {
            return Err(Error::Limit(format!("more than {MAX_CELLS} cells")));
        }
        match self.cells.get(&self.root) {
            Some(root) if root.parent.is_none() => {}
            _ => return invalid("the root cell is missing or has a parent"),
        }
        // Walk the tree: every cell must be reached exactly once, under the parent it names.
        let mut seen = 0usize;
        let mut stack = vec![(self.root, 0u32)];
        while let Some((id, depth)) = stack.pop() {
            seen = seen.saturating_add(1);
            if seen > self.cells.len() {
                return invalid("a cell is in more than one grid");
            }
            let Some(cell) = self.cells.get(&id) else {
                return invalid(format!("cell {id} is in a grid but does not exist"));
            };
            if cell.id != id || id.0 == self.id.0 {
                return invalid(format!("cell {id} has a wrong or duplicate ID"));
            }
            validate_cell(cell)?;
            if let Some(grid) = &cell.grid {
                if depth >= MAX_DEPTH {
                    return Err(Error::Limit(format!(
                        "grids nested deeper than {MAX_DEPTH}"
                    )));
                }
                for child in &grid.cells {
                    if self.cells.get(child).map(|c| c.parent) != Some(Some(id)) {
                        return invalid(format!("cell {child} is not a child of {id}"));
                    }
                    stack.push((*child, depth.saturating_add(1)));
                }
            }
        }
        if seen != self.cells.len() {
            return invalid("some cells are not reachable from the root");
        }
        Ok(())
    }
}

/// The number of Unicode code points in `text`; positions in text are counted in these (A11).
pub fn code_points(text: &str) -> usize {
    text.chars().count()
}

fn check_extra(extra: &Extra, known: &[&str], place: &str) -> Result<(), Error> {
    match extra.keys().find(|k| known.contains(&k.as_str())) {
        Some(key) => invalid(format!(
            "{place} has the known field {key:?} as an unknown field"
        )),
        None => Ok(()),
    }
}

/// Checks that ranges are sorted, not empty, not overlapping and inside a text of `len` code
/// points.
fn check_ranges(
    ranges: impl Iterator<Item = (u32, u32)>,
    len: usize,
    what: &str,
    id: CellId,
) -> Result<(), Error> {
    let mut pos = 0u64;
    for (start, count) in ranges {
        let end = u64::from(start).saturating_add(u64::from(count));
        if count == 0 || u64::from(start) < pos || end > len as u64 {
            return invalid(format!(
                "cell {id}: {what} at {start}, length {count} is empty, out of order, \
                 overlapping or outside the text ({len} code points)"
            ));
        }
        pos = end;
    }
    Ok(())
}

/// Whether `name` is acceptable as the file name of an image: 1 to 255 of the characters
/// `A-Z a-z 0-9 . _ -`, not starting with a dot. This keeps names valid on every file system
/// and makes it impossible for a document to point outside its image folder.
pub fn is_valid_image_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_IMAGE_NAME_BYTES
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn validate_cell(cell: &Cell) -> Result<(), Error> {
    let id = cell.id;
    let place = format!("cell {id}");
    check_extra(&cell.extra, crate::format::CELL_KEYS, &place)?;
    let len = code_points(&cell.text);
    if len > MAX_TEXT_CODE_POINTS || code_points(&cell.note) > MAX_TEXT_CODE_POINTS {
        return Err(Error::Limit(format!(
            "{place}: text or note longer than {MAX_TEXT_CODE_POINTS} code points"
        )));
    }
    check_ranges(cell.runs.iter().map(|r| (r.start, r.len)), len, "run", id)?;
    check_ranges(cell.links.iter().map(|l| (l.start, l.len)), len, "link", id)?;
    for run in &cell.runs {
        check_extra(
            &run.extra,
            crate::format::RUN_KEYS,
            &format!("a run of {place}"),
        )?;
    }
    for link in &cell.links {
        check_extra(
            &link.extra,
            crate::format::LINK_KEYS,
            &format!("a link of {place}"),
        )?;
        if matches!(&link.target, LinkTarget::External(target) if target.is_empty()) {
            return invalid(format!("{place}: a link has an empty target"));
        }
    }
    if let Some(image) = &cell.image {
        check_extra(
            &image.extra,
            crate::format::IMAGE_KEYS,
            &format!("the image of {place}"),
        )?;
        if !is_valid_image_name(&image.file) {
            return invalid(format!(
                "{place}: {:?} is not a valid image file name",
                image.file
            ));
        }
        if !(image.scale.is_finite() && image.scale > 0.0) {
            return invalid(format!("{place}: the image scale is not a positive number"));
        }
    }
    if let Some(grid) = &cell.grid {
        check_extra(
            &grid.extra,
            crate::format::GRID_KEYS,
            &format!("the grid of {place}"),
        )?;
        let count = (grid.xs as usize).checked_mul(grid.ys as usize);
        if grid.xs == 0 || grid.ys == 0 || count != Some(grid.cells.len()) {
            return invalid(format!(
                "{place}: a {} by {} grid cannot have {} cells",
                grid.xs,
                grid.ys,
                grid.cells.len()
            ));
        }
        if grid.col_widths.len() != grid.xs as usize {
            return invalid(format!(
                "{place}: the grid needs one column width per column"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::id::SeededIds;

    fn doc_with_grid() -> (Document, SeededIds) {
        let mut ids = SeededIds::from_u64(1);
        let mut doc = Document::new(&mut ids);
        doc.add_grid(doc.root(), 3, 2, &mut ids).unwrap();
        (doc, ids)
    }

    #[test]
    fn new_document_is_one_empty_valid_cell() {
        let doc = Document::new(&mut SeededIds::from_u64(1));
        assert_eq!(doc.cell_count(), 1);
        let root = doc.cell(doc.root()).unwrap();
        assert_eq!(
            (root.text.as_str(), root.parent(), root.grid().is_none()),
            ("", None, true)
        );
        assert_eq!(doc.depth(doc.root()), Some(0));
        doc.validate().unwrap();
    }

    #[test]
    fn add_grid_makes_row_major_children_that_know_their_parent() {
        let (mut doc, mut ids) = doc_with_grid();
        assert_eq!(doc.cell_count(), 7);
        let grid = doc.cell(doc.root()).unwrap().grid().unwrap().clone();
        assert_eq!((grid.xs(), grid.ys(), grid.cells().len()), (3, 2, 6));
        assert_eq!(grid.at(2, 1), Some(grid.cells()[5]));
        assert_eq!(grid.at(1, 0), Some(grid.cells()[1]));
        assert_eq!(grid.at(3, 0), None);
        assert_eq!(grid.at(0, 2), None);
        assert_eq!(grid.col_widths, vec![Grid::DEFAULT_COL_WIDTH; 3]);
        let inner = doc.child(doc.root(), 1, 1).unwrap();
        assert_eq!(doc.cell(inner).unwrap().parent(), Some(doc.root()));
        doc.add_grid(inner, 1, 1, &mut ids).unwrap();
        let leaf = doc.child(inner, 0, 0).unwrap();
        assert_eq!(doc.depth(leaf), Some(2));
        assert_eq!(doc.cells_in_order().len(), 8);
        assert_eq!(doc.cells_in_order()[5], inner);
        assert_eq!(doc.cells_in_order()[6], leaf);
        doc.validate().unwrap();
    }

    #[test]
    fn add_grid_refuses_bad_requests() {
        let (mut doc, mut ids) = doc_with_grid();
        let root = doc.root();
        assert!(matches!(
            doc.add_grid(root, 1, 1, &mut ids),
            Err(Error::Invalid(_))
        ));
        let cell = doc.child(root, 0, 0).unwrap();
        assert!(matches!(
            doc.add_grid(cell, 0, 1, &mut ids),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            doc.add_grid(cell, 1, 0, &mut ids),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            doc.add_grid(cell, u32::MAX, u32::MAX, &mut ids),
            Err(Error::Limit(_))
        ));
        assert!(matches!(
            doc.add_grid(cell, 5_000, 5_000, &mut ids),
            Err(Error::Limit(_))
        ));
        let missing = CellId(ids.next_id());
        assert!(matches!(
            doc.add_grid(missing, 1, 1, &mut ids),
            Err(Error::NotFound(_))
        ));
        assert_eq!(doc.cell_count(), 7);
        doc.validate().unwrap();
    }

    #[test]
    fn nesting_is_limited() {
        let mut ids = SeededIds::from_u64(1);
        let mut doc = Document::new(&mut ids);
        let mut cell = doc.root();
        for _ in 0..MAX_DEPTH {
            doc.add_grid(cell, 1, 1, &mut ids).unwrap();
            cell = doc.child(cell, 0, 0).unwrap();
        }
        assert_eq!(doc.depth(cell), Some(MAX_DEPTH));
        assert!(matches!(
            doc.add_grid(cell, 1, 1, &mut ids),
            Err(Error::Limit(_))
        ));
        doc.validate().unwrap();
    }

    #[test]
    fn a_repeating_id_source_is_refused() {
        struct Stuck;
        impl IdSource for Stuck {
            fn next_id(&mut self) -> crate::Id {
                crate::Id::from_u128(5)
            }
        }
        let mut doc = Document::new(&mut SeededIds::from_u64(1));
        assert!(matches!(
            doc.add_grid(doc.root(), 2, 1, &mut Stuck),
            Err(Error::Invalid(_))
        ));
    }

    fn run(start: u32, len: u32) -> Run {
        Run {
            start,
            len,
            style: Style::default(),
            color: None,
            extra: Extra::new(),
        }
    }

    #[test]
    fn runs_and_links_are_counted_in_code_points() {
        let mut doc = Document::new(&mut SeededIds::from_u64(1));
        let root = doc.root();
        // 4 code points, 13 bytes, 6 UTF-16 units.
        doc.cell_mut(root).unwrap().text = "a\u{1F600}é\u{1F600}".into();
        doc.cell_mut(root).unwrap().runs = vec![run(0, 1), run(1, 3)];
        doc.validate().unwrap();
        doc.cell_mut(root).unwrap().runs = vec![run(1, 4)];
        assert!(matches!(doc.validate(), Err(Error::Invalid(_))));
        for bad in [
            vec![run(0, 0)],
            vec![run(2, 1), run(0, 1)],
            vec![run(0, 2), run(1, 1)],
            vec![run(4, 1)],
            vec![run(u32::MAX, u32::MAX)],
        ] {
            doc.cell_mut(root).unwrap().runs = bad;
            assert!(matches!(doc.validate(), Err(Error::Invalid(_))));
        }
        doc.cell_mut(root).unwrap().runs.clear();
        let link = |start, len| Link {
            start,
            len,
            target: LinkTarget::External("https://example.org".into()),
            extra: Extra::new(),
        };
        doc.cell_mut(root).unwrap().links = vec![link(0, 4)];
        doc.validate().unwrap();
        doc.cell_mut(root).unwrap().links = vec![link(0, 3), link(2, 2)];
        assert!(matches!(doc.validate(), Err(Error::Invalid(_))));
        doc.cell_mut(root).unwrap().links = vec![Link {
            target: LinkTarget::External(String::new()),
            ..link(0, 1)
        }];
        assert!(matches!(doc.validate(), Err(Error::Invalid(_))));
    }

    #[test]
    fn image_references_are_checked() {
        let mut doc = Document::new(&mut SeededIds::from_u64(1));
        let root = doc.root();
        doc.cell_mut(root).unwrap().image = Some(ImageRef::new("photo-1.png"));
        doc.validate().unwrap();
        for bad in [
            "",
            "../x.png",
            "a/b.png",
            "a\\b.png",
            ".hidden",
            "sp ace.png",
            "é.png",
        ] {
            doc.cell_mut(root).unwrap().image = Some(ImageRef::new(bad));
            assert!(matches!(doc.validate(), Err(Error::Invalid(_))), "{bad}");
        }
        doc.cell_mut(root).unwrap().image = Some(ImageRef::new("x".repeat(256)));
        assert!(doc.validate().is_err());
        for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            doc.cell_mut(root).unwrap().image = Some(ImageRef {
                scale,
                ..ImageRef::new("a.png")
            });
            assert!(matches!(doc.validate(), Err(Error::Invalid(_))), "{scale}");
        }
    }

    #[test]
    fn grid_properties_and_extras_are_checked() {
        let (mut doc, _) = doc_with_grid();
        let root = doc.root();
        doc.cell_mut(root)
            .unwrap()
            .grid_mut()
            .unwrap()
            .col_widths
            .pop();
        assert!(matches!(doc.validate(), Err(Error::Invalid(_))));
        doc.cell_mut(root)
            .unwrap()
            .grid_mut()
            .unwrap()
            .col_widths
            .push(10);
        doc.validate().unwrap();
        doc.cell_mut(root)
            .unwrap()
            .extra
            .insert("text".into(), Value::Null);
        assert!(matches!(doc.validate(), Err(Error::Invalid(_))));
        doc.cell_mut(root).unwrap().extra.clear();
        doc.extra.insert("root".into(), Value::Null);
        assert!(matches!(doc.validate(), Err(Error::Invalid(_))));
    }

    #[test]
    fn colours_parse_and_print() {
        assert_eq!(Color::parse("#A0b1C2"), Some(Color::rgb(0xa0, 0xb1, 0xc2)));
        assert_eq!(Color::rgb(0xa0, 0xb1, 0xc2).to_string(), "#a0b1c2");
        for bad in [
            "", "#", "a0b1c2", "#a0b1c", "#a0b1c2d", "#+0b1c2", "#gggggg", "#ééé",
        ] {
            assert_eq!(Color::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn enum_names_round_trip() {
        for kind in CellKind::ALL {
            assert_eq!(CellKind::parse(kind.as_str()), Some(*kind));
        }
        assert_eq!(CellKind::ALL.len(), 7);
        assert_eq!(CellKind::parse("date"), None);
        assert_eq!(DrawStyle::ALL.len(), 3);
        assert_eq!(TextAlign::parse("center"), Some(TextAlign::Center));
        assert_eq!(VertAlign::parse("middle"), Some(VertAlign::Middle));
    }
}
