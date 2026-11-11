//! The in-memory document.
//!
//! A document is a tree: a root cell; a cell has text and optionally a grid; a grid is `xs` by
//! `ys` cells in row-major order. Cells live in one map from [`CellId`] to [`Cell`], and each
//! cell knows its parent, so finding a cell or its parent by ID does not depend on the size or
//! depth of the document. A grid holds the IDs of its cells, not the cells themselves.
//!
//! A document is **changed** only through the commands of [`crate::edit`] (decision A9). The
//! one other way to write to a document is the **construction layer**, the trait
//! [`crate::construct::Construct`]: unrestricted, unrecorded access for code that builds a
//! document before anyone edits it (the file reader, the `.cts` importer, tests). Its methods
//! are only there for code that imports the trait, and an [`crate::edit::Editor`] owns its
//! document and lends it out read-only, so nothing can reach the construction layer of a
//! document that has an undo history (decision A31).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use serde_json::Value;

use crate::error::{invalid, Error};
use crate::id::{CellId, DocId, IdSource};
use crate::limits::{
    MAX_CELLS, MAX_DEPTH, MAX_IMAGE_NAME_BYTES, MAX_TEXT_CODE_POINTS, MAX_UNKNOWN_DEPTH,
};

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
        /// The document the cell is in, or `None` for the document that holds the link;
        /// never the ID of that document itself, so that a link to one cell has one form
        /// (decision A47).
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
    /// The text. For a formula cell this is the formula. No characters below U+0020
    /// (decision A32).
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

    /// The grid, to change its properties (not its size or cells). Reachable only through
    /// the construction layer ([`crate::construct::Construct`]).
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
    pub(crate) tags: BTreeMap<String, Tag>,
    pub(crate) extra: Extra,
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

    /// The tags of the document, by name. A cell has a tag when its text is the tag's name.
    pub fn tags(&self) -> &BTreeMap<String, Tag> {
        &self.tags
    }

    /// Unrecognised document-level fields.
    pub fn extra(&self) -> &Extra {
        &self.extra
    }

    pub(crate) fn cell_mut(&mut self, id: CellId) -> Option<&mut Cell> {
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

    pub(crate) fn add_grid(
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
            if !crate::text::is_clean(name) {
                return invalid(format!(
                    "tag {name:?} has a control character (below U+0020) in its name"
                ));
            }
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
        // Counting is not enough: one cell listed twice can hide another that is left out.
        let mut seen = HashSet::with_capacity(self.cells.len());
        let mut stack = vec![(self.root, 0u32)];
        while let Some((id, depth)) = stack.pop() {
            if !seen.insert(id) {
                return invalid(format!("cell {id} is in a grid more than once"));
            }
            let Some(cell) = self.cells.get(&id) else {
                return invalid(format!("cell {id} is in a grid but does not exist"));
            };
            if cell.id != id || id.0 == self.id.0 {
                return invalid(format!("cell {id} has a wrong or duplicate ID"));
            }
            validate_cell(cell)?;
            let own = |link: &Link| matches!(link.target, LinkTarget::Cell { doc: Some(doc), .. } if doc == self.id);
            if cell.links.iter().any(own) {
                return invalid(format!(
                    "cell {id}: a link to a cell of this document names the document"
                ));
            }
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
        if seen.len() != self.cells.len() {
            return invalid("some cells are not reachable from the root");
        }
        Ok(())
    }

    /// Makes a copy of the document that shares no ID with it: the copy has a new document
    /// ID and a new ID for every cell, taken from `ids` in document order. Links to cells of
    /// this document (which do not name the document, decision A47) follow the copy's
    /// cells; links to cells of other documents, and links to cells that are not in this
    /// document, are unchanged. IDs inside unknown fields are not recognised and stay as they are. The
    /// returned [`FreshIds`] says which ID became which, for the view state
    /// ([`crate::ViewState::with_fresh_ids`]) and anything else keyed by cell ID.
    pub fn with_fresh_ids(&self, ids: &mut dyn IdSource) -> Result<(Document, FreshIds), Error> {
        self.validate()?;
        let mut used = HashSet::with_capacity(self.cells.len().saturating_add(1));
        let mut fresh_id = || {
            let id = ids.next_id();
            // Not an ID of the original either, or the two documents would still collide.
            if id == self.id.0 || self.cells.contains_key(&CellId(id)) || !used.insert(id) {
                return invalid(format!("the ID source repeated the ID {id}"));
            }
            Ok(id)
        };
        let mut fresh = FreshIds {
            from: self.id,
            to: DocId(fresh_id()?),
            cells: HashMap::with_capacity(self.cells.len()),
        };
        for old in self.cells_in_order() {
            fresh.cells.insert(old, CellId(fresh_id()?));
        }
        // Every cell is in the map, because the document is valid.
        let renew = |old: CellId| {
            fresh
                .cell(old)
                .ok_or_else(|| Error::Invalid(format!("cell {old} is not in the document")))
        };
        let mut cells = HashMap::with_capacity(self.cells.len());
        for (old, cell) in &self.cells {
            let mut cell = cell.clone();
            cell.id = renew(*old)?;
            cell.parent = cell.parent.map(renew).transpose()?;
            if let Some(grid) = &mut cell.grid {
                for child in &mut grid.cells {
                    *child = renew(*child)?;
                }
            }
            for link in &mut cell.links {
                if let LinkTarget::Cell {
                    doc: None,
                    cell: target,
                } = &mut link.target
                {
                    if let Some(new) = fresh.cell(*target) {
                        *target = new;
                    }
                }
            }
            cells.insert(cell.id, cell);
        }
        let copy = Document {
            id: fresh.to,
            minor: self.minor,
            root: renew(self.root)?,
            cells,
            tags: self.tags.clone(),
            extra: self.extra.clone(),
        };
        Ok((copy, fresh))
    }
}

/// Which IDs [`Document::with_fresh_ids`] replaced by which.
#[derive(Clone, Debug)]
pub struct FreshIds {
    /// The ID of the original document.
    pub from: DocId,
    /// The ID of the copy.
    pub to: DocId,
    cells: HashMap<CellId, CellId>,
}

impl FreshIds {
    /// The cell of the copy that corresponds to the cell `old` of the original; `None` if
    /// the original has no such cell.
    pub fn cell(&self, old: CellId) -> Option<CellId> {
        self.cells.get(&old).copied()
    }
}

/// The number of Unicode code points in `text`; positions in text are counted in these (A11).
pub fn code_points(text: &str) -> usize {
    text.chars().count()
}

/// Whether `value` nests arrays and objects at most `depth` deep. It never recurses deeper
/// than that itself, however deep the value is.
fn nests_at_most(value: &Value, depth: u32) -> bool {
    let inside = |inner: &Value| nests_at_most(inner, depth.saturating_sub(1));
    match value {
        Value::Array(items) => depth > 0 && items.iter().all(inside),
        Value::Object(map) => depth > 0 && map.values().all(inside),
        _ => true,
    }
}

/// Checks the unknown fields of one object: none has the name of a known field, and none
/// nests deeper than a file may (decision A22: what is saved must load).
pub(crate) fn check_extra(extra: &Extra, known: &[&str], place: &str) -> Result<(), Error> {
    for (key, value) in extra {
        if known.contains(&key.as_str()) {
            return invalid(format!(
                "{place} has the known field {key:?} as an unknown field"
            ));
        }
        if !nests_at_most(value, MAX_UNKNOWN_DEPTH) {
            return Err(Error::Limit(format!(
                "{place}: the unknown field {key:?} nests arrays and objects deeper than \
                 {MAX_UNKNOWN_DEPTH}"
            )));
        }
    }
    Ok(())
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
/// `A-Z a-z 0-9 . _ -`, not starting or ending with a dot, and not a device name of Windows
/// (`CON`, `PRN`, `AUX`, `NUL`, `COM0`-`COM9`, `LPT0`-`LPT9` in any case, alone or before
/// the first dot). This keeps names usable on every file system and makes it impossible for
/// a document to point outside its image folder.
pub fn is_valid_image_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name);
    let is = |device: &str| stem.eq_ignore_ascii_case(device);
    let device = is("CON")
        || is("PRN")
        || is("AUX")
        || is("NUL")
        || (stem.len() == 4
            && stem.get(..3).is_some_and(|port| {
                port.eq_ignore_ascii_case("COM") || port.eq_ignore_ascii_case("LPT")
            })
            && stem.ends_with(|c: char| c.is_ascii_digit()));
    !name.is_empty()
        && name.len() <= MAX_IMAGE_NAME_BYTES
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !device
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
    if !crate::text::is_clean(&cell.text) {
        return invalid(format!(
            "{place}: the text has a control character (below U+0020)"
        ));
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
    use crate::id::{Id, SeededIds};

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
    fn a_cell_listed_twice_while_another_is_left_out_is_invalid() {
        let (mut doc, _) = doc_with_grid();
        let root = doc.root();
        let cells = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
        // The counts still match: six cells are listed and six exist besides the root.
        doc.cells
            .get_mut(&root)
            .unwrap()
            .grid
            .as_mut()
            .unwrap()
            .cells[1] = cells[0];
        let error = doc.validate().unwrap_err();
        assert!(error.to_string().contains("more than once"), "{error}");
        assert!(crate::format::to_string(&doc).is_err());

        // An orphan that names a parent which does not list it.
        let (mut doc, mut ids) = doc_with_grid();
        let orphan = CellId(ids.next_id());
        doc.cells
            .insert(orphan, Cell::new(orphan, Some(doc.root())));
        let error = doc.validate().unwrap_err();
        assert!(error.to_string().contains("not reachable"), "{error}");

        // Two cells that list each other and are not under the root.
        let (mut doc, mut ids) = doc_with_grid();
        let (a, b) = (CellId(ids.next_id()), CellId(ids.next_id()));
        for (id, other) in [(a, b), (b, a)] {
            let mut cell = Cell::new(id, Some(other));
            cell.grid = Some(Grid::new(1, 1, vec![other]));
            doc.cells.insert(id, cell);
        }
        assert!(doc.validate().is_err());
    }

    fn link(target: LinkTarget) -> Link {
        Link {
            start: 0,
            len: 1,
            target,
            extra: Extra::new(),
        }
    }

    #[test]
    fn a_copy_with_fresh_ids_shares_no_id_and_keeps_its_own_links() {
        let (mut doc, mut ids) = doc_with_grid();
        let root = doc.root();
        let cells = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
        doc.add_grid(cells[4], 1, 2, &mut ids).unwrap();
        let inner = doc.child(cells[4], 0, 1).unwrap();
        let elsewhere = DocId(ids.next_id());
        let gone = CellId(ids.next_id());
        let targets = [
            LinkTarget::Cell {
                doc: None,
                cell: inner,
            },
            LinkTarget::Cell {
                doc: None,
                cell: root,
            },
            // Into another document, even to a cell with an ID this document also has.
            LinkTarget::Cell {
                doc: Some(elsewhere),
                cell: inner,
            },
            // To a cell this document does not have (deleted, perhaps).
            LinkTarget::Cell {
                doc: None,
                cell: gone,
            },
            LinkTarget::External("https://example.org".into()),
            LinkTarget::Unknown,
        ];
        // A link to an own cell that names the own document is not a valid form (A47).
        {
            let mut named = doc.clone();
            let own = named.id;
            let cell = named.cell_mut(cells[0]).unwrap();
            cell.text = "0".into();
            cell.links = vec![link(LinkTarget::Cell {
                doc: Some(own),
                cell: root,
            })];
            assert!(matches!(named.validate(), Err(Error::Invalid(_))));
        }
        {
            let cell = doc.cell_mut(cells[0]).unwrap();
            cell.text = "012345".into();
            cell.links = targets.iter().cloned().map(link).collect();
            for (n, link) in cell.links.iter_mut().enumerate() {
                link.start = n as u32;
            }
            cell.extra.insert("zz".into(), Value::from(1));
        }
        doc.cell_mut(inner).unwrap().text = "inner".into();
        doc.tags.insert("tag".into(), Tag::default());
        doc.extra.insert("zz".into(), Value::from(2));
        doc.validate().unwrap();
        let before = doc.clone();

        let (copy, fresh) = doc.with_fresh_ids(&mut ids).unwrap();
        assert_eq!(doc, before);
        copy.validate().unwrap();
        assert_eq!((fresh.from, fresh.to), (doc.id(), copy.id()));
        assert_ne!(copy.id(), doc.id());
        let (old, new) = (doc.cells_in_order(), copy.cells_in_order());
        assert_eq!(old.len(), new.len());
        for (old, new) in old.iter().zip(&new) {
            assert_eq!(fresh.cell(*old), Some(*new));
            assert!(doc.cell(*new).is_none() && new.0 != doc.id().0);
            assert!(copy.cell(*old).is_none());
            // Everything but the IDs is the same.
            let (a, b) = (doc.cell(*old).unwrap(), copy.cell(*new).unwrap());
            assert_eq!(
                (&a.text, &a.extra, a.links.len()),
                (&b.text, &b.extra, b.links.len())
            );
            assert_eq!(a.parent().and_then(|p| fresh.cell(p)), b.parent());
            assert_eq!(
                a.grid().map(|g| (g.xs(), g.ys())),
                b.grid().map(|g| (g.xs(), g.ys()))
            );
        }
        assert_eq!(fresh.cell(gone), None);
        assert_eq!((&copy.tags, &copy.extra), (&doc.tags, &doc.extra));
        let links: Vec<LinkTarget> = copy
            .cell(fresh.cell(cells[0]).unwrap())
            .unwrap()
            .links
            .iter()
            .map(|l| l.target.clone())
            .collect();
        let mut expected = targets.to_vec();
        expected[0] = LinkTarget::Cell {
            doc: None,
            cell: fresh.cell(inner).unwrap(),
        };
        expected[1] = LinkTarget::Cell {
            doc: None,
            cell: copy.root(),
        };
        assert_eq!(links, expected);

        // The same source of IDs gives the same copy; saving does not depend on map order.
        let again = |seed| {
            doc.with_fresh_ids(&mut SeededIds::from_u64(seed))
                .unwrap()
                .0
        };
        assert_eq!(again(7), again(7));
        assert_ne!(again(7).id(), again(8).id());

        // A source that repeats itself, or gives an ID of the original, is refused.
        struct Fixed(Vec<Id>);
        impl IdSource for Fixed {
            fn next_id(&mut self) -> Id {
                self.0.pop().unwrap_or(Id::from_u128(0))
            }
        }
        let n = doc.cell_count() + 1;
        let distinct: Vec<Id> = (1..=n as u128).map(Id::from_u128).collect();
        assert!(doc.with_fresh_ids(&mut Fixed(distinct.clone())).is_ok());
        assert!(doc.with_fresh_ids(&mut Fixed(Vec::new())).is_err());
        for taken in [doc.id().0, inner.0] {
            let mut with_old = distinct.clone();
            with_old[0] = taken;
            assert!(doc.with_fresh_ids(&mut Fixed(with_old)).is_err());
        }
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
