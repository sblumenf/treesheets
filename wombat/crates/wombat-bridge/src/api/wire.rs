//! The serialised form of commands, change reports and errors (decision A123).
//!
//! These types mirror the core's ([`wombat_core::Command`] and what it holds) as plain
//! data that both sides of the bridge can build: IDs are their canonical text, unknown
//! fields are JSON text, a clip is the text of a document. `convert.rs` turns them into
//! the core's types and back, checking on the way in. With `serde` they are also JSON
//! (`record.rs`), for recording commands and playing them back.
//!
//! One command of the core is one variant here with the same name and fields; see the
//! core for what each does.

use serde::{Deserialize, Serialize};

/// A colour: red, green and blue.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Rgb {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
}

/// Text style switches.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
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

/// What a cell is to the formula system.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum CellKind {
    /// Plain data.
    Data,
    /// An operation.
    Code,
    /// Assigns its value to the variable named by its text.
    VarAssign,
    /// Reads the variable named by its text.
    VarRead,
    /// A horizontal view.
    ViewH,
    /// A vertical view.
    ViewV,
    /// A formula; the cell's text is the formula itself.
    Formula,
}

/// How the grid of a cell is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DrawStyle {
    /// A grid with borders.
    Grid,
    /// Bubbles in a hierarchy.
    Bubbles,
    /// A hierarchy joined by lines.
    Lines,
}

/// Horizontal alignment of a cell's content.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum HorizAlign {
    /// Right for right-to-left text, left otherwise.
    Auto,
    /// Left.
    Left,
    /// Centred.
    Center,
    /// Right.
    Right,
}

/// Vertical alignment of a cell's content.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum VertAlign {
    /// Decided by the draw style.
    Auto,
    /// Top.
    Top,
    /// Middle.
    Middle,
    /// Bottom.
    Bottom,
}

/// A reference to an image file kept beside the document.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ImageRef {
    /// The file name inside the document's image folder.
    pub file: String,
    /// The display scale; 1.0 is the default. Must be a finite number.
    pub scale: f64,
    /// Fields the core does not recognise, as the text of a JSON object; empty for none.
    pub extra: String,
}

/// What a link points at.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LinkTarget {
    /// A URL with a scheme, or else a file path.
    External {
        /// The URL or path.
        target: String,
    },
    /// A cell.
    Cell {
        /// The ID of the document the cell is in, or `None` for the document that holds
        /// the link.
        doc: Option<String>,
        /// The ID of the cell.
        cell: String,
    },
    /// A kind of target this core does not know.
    Unknown,
}

/// A property of a cell with a new value, for [`Command::SetProps`].
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Prop {
    /// The background colour.
    CellColor(Rgb),
    /// The colour of the text.
    TextColor(Rgb),
    /// The text size relative to the default; positive is smaller.
    RelSize(i32),
    /// Horizontal alignment.
    TextAlign(HorizAlign),
    /// Vertical alignment.
    VertAlign(VertAlign),
    /// What the cell is to the formula system.
    Kind(CellKind),
    /// The note.
    Note(String),
    /// How the cell's grid is drawn.
    DrawStyle(DrawStyle),
    /// The image shown in the cell, or `None` for no image.
    Image(Option<ImageRef>),
    /// The display scale of the cell's image, a finite number.
    ImageScale(f64),
    /// When the text was last edited (milliseconds since 1970 UTC), or `None`.
    LastEdit(Option<i64>),
    /// The border colour of the cell's grid.
    BorderColor(Rgb),
    /// The outer spacing of the cell's grid.
    OuterSpacing(u32),
    /// Whether the cell's grid is below its text instead of beside it.
    Vertical(bool),
}

/// A cell with everything inside it, to paste or to move into a document. It is made by
/// the core and only carried here: `document` is the text of a Wombat document whose root
/// is the top cell, which the core reads and checks like any file.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ClipData {
    /// The ID of the document the cells came from.
    pub source: String,
    /// The ID of the cell whose grid the top cell was in, or `None` if it was the root.
    pub parent: Option<String>,
    /// The cells, as the text of a document.
    pub document: String,
}

/// One cell of a [`TableData`].
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct EntryData {
    /// The text.
    pub text: String,
    /// Whether the cell becomes a formula cell; only a text that is a formula does.
    pub formula: bool,
    /// The grid inside.
    pub grid: Option<TableData>,
}

/// A grid of texts to import.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TableData {
    /// The number of columns.
    pub xs: u32,
    /// The cells row by row: whole rows, at least one cell.
    pub cells: Vec<EntryData>,
}

/// A change to a document: [`wombat_core::Command`] with IDs as text. Positions and
/// lengths in text are in Unicode code points; rows and columns are counted from 0.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Command {
    /// Inserts text at a position.
    InsertText {
        /// The cell.
        cell: String,
        /// Where to insert.
        at: u32,
        /// What to insert.
        text: String,
    },
    /// Deletes a range of text.
    DeleteText {
        /// The cell.
        cell: String,
        /// The first code point to delete.
        start: u32,
        /// How many code points to delete.
        len: u32,
    },
    /// Replaces a range of text.
    ReplaceText {
        /// The cell.
        cell: String,
        /// The first code point to replace.
        start: u32,
        /// How many code points to replace.
        len: u32,
        /// The new text.
        text: String,
    },
    /// Replaces the whole text and drops all runs and links.
    SetText {
        /// The cell.
        cell: String,
        /// The new text.
        text: String,
    },
    /// Switches style bits on and off for a range of text.
    SetRangeStyle {
        /// The cell.
        cell: String,
        /// The first code point.
        start: u32,
        /// How many code points.
        len: u32,
        /// The bits to switch on.
        set: Style,
        /// The bits to switch off.
        clear: Style,
    },
    /// Gives a range of text a colour, or with `None` the cell's text colour again.
    SetRangeColor {
        /// The cell.
        cell: String,
        /// The first code point.
        start: u32,
        /// How many code points.
        len: u32,
        /// The colour.
        color: Option<Rgb>,
    },
    /// Makes a range of text a link, or with `None` removes the links from it.
    SetLink {
        /// The cell.
        cell: String,
        /// The first code point.
        start: u32,
        /// How many code points.
        len: u32,
        /// Where the link goes.
        target: Option<LinkTarget>,
    },
    /// Sets properties of one or more cells, as one undo unit.
    SetProps {
        /// The cells.
        cells: Vec<String>,
        /// The properties with their new values.
        props: Vec<Prop>,
    },
    /// Switches style bits on and off for whole cells.
    SetStyle {
        /// The cells.
        cells: Vec<String>,
        /// The bits to switch on.
        set: Style,
        /// The bits to switch off.
        clear: Style,
    },
    /// Sets the width of one column of a cell's grid.
    SetColumnWidth {
        /// The cell that has the grid.
        cell: String,
        /// The column.
        column: u32,
        /// The width.
        width: u32,
    },
    /// Gives a cell that has no grid a grid of new empty cells.
    CreateGrid {
        /// The cell.
        cell: String,
        /// The number of columns.
        xs: u32,
        /// The number of rows.
        ys: u32,
    },
    /// Removes the grid of a cell with everything in it.
    RemoveGrid {
        /// The cell.
        cell: String,
    },
    /// Inserts empty rows.
    InsertRows {
        /// The cell that has the grid.
        cell: String,
        /// The row before which to insert; the number of rows appends.
        at: u32,
        /// How many rows.
        count: u32,
    },
    /// Inserts empty columns.
    InsertColumns {
        /// The cell that has the grid.
        cell: String,
        /// The column before which to insert; the number of columns appends.
        at: u32,
        /// How many columns.
        count: u32,
    },
    /// Deletes rows with everything in them.
    DeleteRows {
        /// The cell that has the grid.
        cell: String,
        /// The first row.
        at: u32,
        /// How many rows.
        count: u32,
    },
    /// Deletes columns with everything in them.
    DeleteColumns {
        /// The cell that has the grid.
        cell: String,
        /// The first column.
        at: u32,
        /// How many columns.
        count: u32,
    },
    /// Empties cells: text, runs, links, image and grid go, the looks stay.
    ClearCells {
        /// The cells.
        cells: Vec<String>,
    },
    /// What the Delete key does to a block of cells: empties them, and deletes the rows
    /// or columns that this leaves empty.
    DeleteCells {
        /// The cell that has the grid.
        cell: String,
        /// The first selected column.
        x: u32,
        /// The first selected row.
        y: u32,
        /// The number of selected columns, at least 1.
        xs: u32,
        /// The number of selected rows, at least 1.
        ys: u32,
    },
    /// Moves rows within their grid.
    MoveRows {
        /// The cell that has the grid.
        cell: String,
        /// The first row to move.
        from: u32,
        /// How many rows.
        count: u32,
        /// The index the first moved row has afterwards.
        to: u32,
    },
    /// Moves columns within their grid, with their widths.
    MoveColumns {
        /// The cell that has the grid.
        cell: String,
        /// The first column to move.
        from: u32,
        /// How many columns.
        count: u32,
        /// The index the first moved column has afterwards.
        to: u32,
    },
    /// Makes two cells trade places, with everything in them.
    SwapCells {
        /// One cell.
        a: String,
        /// The other.
        b: String,
    },
    /// Moves a cell to where an empty cell is.
    MoveCell {
        /// The cell to move.
        from: String,
        /// An empty cell whose place it takes.
        to: String,
    },
    /// Makes `to` a copy of `from` with everything inside it.
    CopyCell {
        /// The cell to copy.
        from: String,
        /// The cell that becomes the copy.
        to: String,
    },
    /// Adds a tag to the document, or changes its colours.
    SetTag {
        /// The name.
        name: String,
        /// The background colour of cells with the tag.
        cell_color: Rgb,
        /// The text colour of cells with the tag.
        text_color: Rgb,
    },
    /// Removes a tag from the document.
    RemoveTag {
        /// The name.
        name: String,
    },
    /// Sorts whole rows of a grid by the text of their cells in one column.
    SortRows {
        /// The cell that has the grid.
        cell: String,
        /// The column to sort by first.
        column: u32,
        /// The first row that takes part.
        row: u32,
        /// How many rows take part.
        count: u32,
        /// From high to low instead of from low to high.
        descending: bool,
    },
    /// Rows become columns in the grid of every listed cell that has one.
    Transpose {
        /// The cells whose grids are transposed.
        cells: Vec<String>,
    },
    /// The cells with the text of `cell` move up one level and take their parents inside.
    HierarchySwap {
        /// The cell whose text is swapped upwards.
        cell: String,
    },
    /// Turns the grid of `cell`, a table, into a hierarchy.
    Hierarchify {
        /// The cell that has the table.
        cell: String,
    },
    /// Turns the grid of `cell`, a hierarchy, into a table.
    Flatten {
        /// The cell whose grid is flattened.
        cell: String,
    },
    /// Moves a block of cells into the grid of a new cell in its place.
    Wrap {
        /// The cell that has the grid.
        cell: String,
        /// The first column of the block.
        x: u32,
        /// The first row of the block.
        y: u32,
        /// The number of columns, at least 1.
        xs: u32,
        /// The number of rows, at least 1.
        ys: u32,
    },
    /// Joins the texts of a block of cells into its first cell.
    Collapse {
        /// The cell that has the grid.
        cell: String,
        /// The first column of the block.
        x: u32,
        /// The first row of the block.
        y: u32,
        /// The number of columns.
        xs: u32,
        /// The number of rows.
        ys: u32,
    },
    /// Cuts the text of a cell into the two cells of a new grid inside it.
    SplitCell {
        /// The cell.
        cell: String,
        /// Where to cut the text.
        at: u32,
    },
    /// The text of each listed cell that has text becomes a tag with the cell's colours.
    TagCells {
        /// The cells.
        cells: Vec<String>,
    },
    /// Removes the tags named by the texts of the listed cells.
    UntagCells {
        /// The cells.
        cells: Vec<String>,
    },
    /// Adds `delta` to the relative size of the cells (negative makes the text bigger).
    AdjustRelSize {
        /// The cells.
        cells: Vec<String>,
        /// What to add.
        delta: i32,
        /// Also the cells inside the listed ones.
        recursive: bool,
    },
    /// Adds `delta` to the widths of columns.
    AdjustColumnWidths {
        /// The cell that has the grid.
        cell: String,
        /// The first column.
        column: u32,
        /// How many columns.
        count: u32,
        /// What to add.
        delta: i32,
        /// Also every column of every grid inside the cells of those columns.
        recursive: bool,
    },
    /// Runs the program of the document and fills its view cells.
    Run,
    /// Typing into a cell has ended with this text: sets it, and makes the cell a formula
    /// cell if the text is a formula and a data cell if not.
    EnterText {
        /// The cell.
        cell: String,
        /// The text typed.
        text: String,
    },
    /// Adds to-do checkboxes to cells, ticks them, or removes them.
    ToggleCheck {
        /// The cells.
        cells: Vec<String>,
    },
    /// Makes `to` a copy of the top cell of `clip`, which may come from another document.
    PasteCells {
        /// The cell that becomes the copy.
        to: String,
        /// What to paste.
        clip: ClipData,
    },
    /// The cells of `clip`, taken from another document, come into this one with the IDs
    /// they have, in the place of the empty cell `to`.
    AdoptCells {
        /// An empty cell whose place the top cell takes.
        to: String,
        /// The cells that move in.
        clip: ClipData,
    },
    /// `cell` and everything inside it leave this document.
    ReleaseCell {
        /// The cell that leaves.
        cell: String,
    },
    /// The table becomes the grid of `cell`.
    Import {
        /// The cell that gets the grid.
        cell: String,
        /// What to import.
        table: TableData,
        /// Whether a grid with content may be replaced.
        replace: bool,
    },
    /// Corrects links after cells went to another document.
    RetargetLinks {
        /// The cells that the links point at.
        cells: Vec<String>,
        /// The ID of the document that the links to correct name.
        from: Option<String>,
        /// The ID of the document that they are to name.
        to: Option<String>,
    },
}

/// What a command, an undo or a redo changed: the core's change report, and with it the
/// formula cells that show something else now. Everything not listed is exactly as it
/// was. All are IDs of cells, but `tags`.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Report {
    /// Cells whose own content changed: text, runs, links, any property.
    pub changed: Vec<String>,
    /// Cells that did not exist before.
    pub created: Vec<String>,
    /// Cells that no longer exist.
    pub removed: Vec<String>,
    /// Cells whose grid appeared, disappeared, changed size or has other cells.
    pub grids: Vec<String>,
    /// Names of document tags that were added, removed or changed.
    pub tags: Vec<String>,
    /// Formula cells whose result changed, though they may not be changed themselves.
    pub recalculated: Vec<String>,
}

/// What went wrong, for an interface to tell apart without reading the message.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ErrorKind {
    /// A file could not be read or written.
    Io,
    /// The input is not JSON text.
    Syntax,
    /// The input is not the expected kind of Wombat file.
    WrongFormat,
    /// The file was written by a newer Wombat than this one reads.
    NewerVersion,
    /// The document, or what was sent over the bridge, breaks a rule.
    Invalid,
    /// The command cannot be carried out on the document as it is; nothing changed.
    Refused,
    /// A size, count or depth is over a limit.
    Limit,
    /// There is no such cell or document.
    NotFound,
    /// The file is there already.
    AlreadyExists,
    /// Two files were to be saved together and only one was.
    HalfSaved,
}

/// An error, as a value an interface can show.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BridgeError {
    /// What went wrong.
    pub kind: ErrorKind,
    /// The whole story in English, as the core tells it.
    pub message: String,
    /// The file concerned, if it is about one.
    pub path: Option<String>,
}

/// One cell as an interface lists it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CellInfo {
    /// The ID of the cell.
    pub id: String,
    /// The text; for a formula cell, the formula.
    pub text: String,
    /// What the cell is to the formula system.
    pub kind: CellKind,
    /// What a formula cell shows instead of its text; `None` for every other kind.
    pub result: Option<String>,
    /// Whether `result` is an error value such as `#NAME?`.
    pub failed: bool,
    /// Whether the cell has a grid.
    pub has_grid: bool,
}

/// A cell and the cells of its grid.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct GridInfo {
    /// The cell itself.
    pub cell: CellInfo,
    /// The number of columns, 0 for a cell without a grid.
    pub xs: u32,
    /// The number of rows, 0 for a cell without a grid.
    pub ys: u32,
    /// The cells of the grid row by row; empty for a cell without a grid.
    pub cells: Vec<CellInfo>,
}
