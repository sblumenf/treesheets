//! The serialised form of commands, change reports and errors (decision A123).
//!
//! These types mirror the core's ([`wombat_core::Command`] and what it holds) as plain
//! data that both sides of the bridge can build: IDs are their canonical text, unknown
//! fields are JSON text, a clip is the text of a document. `convert.rs` turns them into
//! the core's types and back, checking on the way in. With `serde` they are also JSON
//! (`record.rs`), for recording commands and playing them back.
//!
//! One command of the core is one variant here with the same name and fields. The doc
//! comments here become the documentation of the generated Dart classes and are all that
//! a Dart developer sees, so they say what the core's own comments say
//! (`wombat-core/src/edit/mod.rs`): keep the two alike when either changes. The
//! generator does not carry the comment of an enum that holds data (`Command`, `Prop`,
//! `LinkTarget`) nor this one, only those of their variants and fields and of the other
//! types; so a rule that a Dart developer must know is said on a variant, on a field or
//! on [`Report`] as well.
//!
//! Text crosses as Unicode. A Dart string is made of UTF-16 units and may hold half of a
//! surrogate pair on its own, which is no character; such a half arrives here as U+FFFD,
//! the replacement character, with no error, because Dart puts it there when it encodes
//! the string, before Rust sees anything. That goes for every string: cell text, notes,
//! tag names, link targets and paths (an ID with one in it is no ID and is refused).
//!
//! As JSON the form is strict (decision A136): an object must have every field of its
//! type and no other. A field that may be none is written `null`; leaving it out is an
//! error, so that a misspelt or forgotten field is never read as "none". The attributes
//! for this (`deny_unknown_fields`, `present`) concern JSON only: between Dart and Rust
//! the generated code writes every field in order, and has no names to get wrong. They
//! are not all of it: `serde` would still read an object from a list of its values, and
//! `record.rs`, the one reader of the JSON form, refuses any form that is not written.
//!
//! Whole numbers that Dart sends are 64 bits wide here, as they are in Dart, though the
//! core takes fewer: a narrower number would lose its high bits on the way without
//! anyone being told (decision A134). `convert.rs` refuses what is out of range.

use serde::{Deserialize, Serialize};

use crate::record::present;

/// A colour: red, green and blue, each from 0 to 255. Any other number is refused.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rgb {
    /// Red.
    pub r: i64,
    /// Green.
    pub g: i64,
    /// Blue.
    pub b: i64,
}

/// Text style switches.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct ImageRef {
    /// The file name inside the document's image folder: 1 to 255 of the characters
    /// `A-Z a-z 0-9 . _ -`, not starting or ending with a dot, and not a device name of
    /// Windows (`CON`, `NUL`, `COM1` and the like).
    pub file: String,
    /// The display scale; 1.0 is the default. A finite number above 0.
    pub scale: f64,
    /// Fields the core does not recognise, as the text of a JSON object; empty for none.
    /// Text that is no JSON object is refused, and so is a field that the core does know.
    pub extra: String,
}

/// What a link points at.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum LinkTarget {
    /// A URL with a scheme, or else a file path, absolute or relative to the document.
    External {
        /// The URL or path; not empty.
        target: String,
    },
    /// A cell.
    Cell {
        /// The ID of the document the cell is in, or `None` for the document that holds
        /// the link. The ID of that document itself is read as `None`, so that a link to
        /// one cell has one form.
        #[serde(deserialize_with = "present")]
        doc: Option<String>,
        /// The ID of the cell.
        cell: String,
    },
    /// A kind of target this core does not know. It can be read, in a link that a file
    /// has; [`Command::SetLink`] refuses to make one.
    Unknown,
}

/// A property of a cell with a new value, for [`Command::SetProps`]. A property of a
/// grid or of an image on a cell that has none makes the whole command fail.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Prop {
    /// The background colour. Stamps the last-edit time, as TreeSheets does, if the
    /// colour is different after the command.
    CellColor(Rgb),
    /// The colour of the text. As in TreeSheets this also removes the colours of the
    /// cell's runs, so that the whole text has the new colour. Stamps the last-edit time
    /// if any of that is different after the command.
    TextColor(Rgb),
    /// The text size relative to the default; positive is smaller. From -2147483648 to
    /// 2147483647.
    RelSize(i64),
    /// Horizontal alignment.
    TextAlign(HorizAlign),
    /// Vertical alignment.
    VertAlign(VertAlign),
    /// What the cell is to the formula system.
    Kind(CellKind),
    /// The note: any text, line breaks included.
    Note(String),
    /// How the cell's grid is drawn.
    DrawStyle(DrawStyle),
    /// The image shown in the cell, or `None` for no image.
    Image(Option<ImageRef>),
    /// The display scale of the cell's image, a finite number above 0; the cell must
    /// have an image.
    ImageScale(f64),
    /// When the text was last edited (milliseconds since 1970 UTC), or `None` for
    /// unknown. Any 64-bit number is one.
    LastEdit(Option<i64>),
    /// The border colour of the cell's grid; the cell must have one.
    BorderColor(Rgb),
    /// The outer spacing of the cell's grid, from 0 to 4294967295; the cell must have
    /// one.
    OuterSpacing(i64),
    /// Whether the cell's grid is below its text instead of beside it; the cell must
    /// have a grid.
    Vertical(bool),
}

/// A cell with everything inside it, to paste or to move into a document. It is made by
/// the core and only carried here: `document` is the text of a Wombat document whose root
/// is the top cell, which the core reads and checks like any file. The ID of that document
/// is the ID of the document the cells came from; it is said nowhere else.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClipData {
    /// The ID of the cell whose grid the top cell was in, or `None` if it was the root.
    /// It is not one of the cells carried.
    #[serde(deserialize_with = "present")]
    pub parent: Option<String>,
    /// The cells, as the text of a document that has the ID of the document they came
    /// from.
    pub document: String,
}

/// The size of the grid inside one cell of a [`TableData`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridSize {
    /// The number of columns, at least 1.
    pub xs: i64,
    /// The number of cells, whole rows of them and at least one. Cells of grids further
    /// inside are not counted.
    pub cells: i64,
}

/// One cell of a [`TableData`].
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryData {
    /// The text. Characters that cell text cannot hold become spaces.
    pub text: String,
    /// Whether the cell becomes a formula cell; only a text that is a formula does.
    pub formula: bool,
    /// The size of the grid inside, or `None` for a cell without one. The cells of that
    /// grid are the next `cells` cells of the list that are not inside one of them.
    #[serde(deserialize_with = "present")]
    pub grid: Option<GridSize>,
}

/// A grid of texts to import, which may have grids inside its cells.
///
/// It is one flat list, not a table within a table: each cell is followed at once by the
/// cells of the grid inside it, row by row, and each of those by the cells of its own
/// grid. A table of two cells `a` and `b`, where `a` holds a grid of `c` and `d`, is the
/// list `a` (with a grid of 2 cells), `c`, `d`, `b`. The form is flat so that a table
/// nested however deep is read and refused without the depth costing anything: nothing
/// that handles it calls itself (decision A135).
///
/// A list that is no table is refused with [`ErrorKind::BadRequest`]: a grid, the
/// outermost or one inside, with no column, no cell or a row that is not whole, a grid
/// that says it has more cells than follow, a number out of range. A table that is one,
/// but nested deeper than a document may be, with more cells or with a longer text, is
/// refused with [`ErrorKind::Limit`]. A table is never [`ErrorKind::Invalid`].
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableData {
    /// The number of columns of the outermost grid, from 1 to 4294967295.
    pub xs: i64,
    /// The cells in the order described. Those that are in no grid inside are the
    /// outermost grid: at least one, and whole rows of `xs`.
    pub cells: Vec<EntryData>,
}

/// A change to a document: [`wombat_core::Command`] with IDs as text.
///
/// - **Cells** are named by their ID, as the 36 characters that [`CellInfo::id`] gives
///   (lower case, with hyphens). Any other text is refused ([`ErrorKind::BadRequest`]),
///   and an ID that no cell of the document has is [`ErrorKind::NotFound`].
/// - **Positions and lengths in text** are in Unicode code points, not in the UTF-16
///   units that Dart strings count in (most emoji are one code point and two units), and
///   must not cut a user-perceived character (an extended grapheme cluster: a letter with
///   its combining accents, a flag, a family emoji). A position or range that would, or
///   that is outside the text, is refused ([`ErrorKind::Refused`]); it is never moved.
/// - **Cell text holds no control characters** (below U+0020: no line breaks, no tabs).
///   The commands that take text turn each into a space. Notes may hold any text.
/// - **Rows and columns** are counted from 0.
/// - **Whole numbers** are positions, counts and sizes from 0 to 4294967295, but for each
///   `delta`, which is from -2147483648 to 2147483647, and for the parts of a colour,
///   from 0 to 255. A number outside its range is refused ([`ErrorKind::BadRequest`]); it
///   is never cut to fit. (A number inside its range can still be one that the command
///   has no use for, such as a row that the grid does not have: that is
///   [`ErrorKind::Refused`].)
/// - **A command that asks for nothing new** (setting a colour a cell already has,
///   inserting empty text) succeeds with an empty [`Report`], changes nothing and leaves
///   nothing to undo.
/// - **A command that fails changes nothing**: the document is exactly as it was.
/// - A text or a note is at most 16,777,216 code points, grids nest at most 32 deep, and
///   a document has at most 700,000 cells; past any of these is [`ErrorKind::Limit`].
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Command {
    /// Inserts text at a position. Stamps the last-edit time.
    InsertText {
        /// The cell.
        cell: String,
        /// Where to insert: a position in code points, not inside a user-perceived
        /// character.
        at: i64,
        /// What to insert; control characters become spaces. Half of a
        /// surrogate pair on its own, which a Dart string can hold, arrives as U+FFFD.
        text: String,
    },
    /// Deletes a range of text. Stamps the last-edit time.
    DeleteText {
        /// The cell.
        cell: String,
        /// The first code point to delete. Positions count code points, and a range
        /// must not cut a user-perceived character.
        start: i64,
        /// How many code points to delete.
        len: i64,
    },
    /// Replaces a range of text; the new text takes the style and link of the first
    /// replaced character. Stamps the last-edit time.
    ReplaceText {
        /// The cell.
        cell: String,
        /// The first code point to replace. Positions count code points, and a range
        /// must not cut a user-perceived character.
        start: i64,
        /// How many code points to replace.
        len: i64,
        /// The new text; control characters become spaces. Half of a
        /// surrogate pair on its own, which a Dart string can hold, arrives as U+FFFD.
        text: String,
    },
    /// Replaces the whole text and, as TreeSheets does, drops all runs and links. Stamps the
    /// last-edit time.
    SetText {
        /// The cell.
        cell: String,
        /// The new text; control characters become spaces. Half of a
        /// surrogate pair on its own, which a Dart string can hold, arrives as U+FFFD.
        text: String,
    },
    /// Switches style bits on and off for a range of text: the bits of `set` on, then the
    /// bits of `clear` off, the others as they are.
    SetRangeStyle {
        /// The cell.
        cell: String,
        /// The first code point. Positions count code points, and a range
        /// must not cut a user-perceived character.
        start: i64,
        /// How many code points.
        len: i64,
        /// The bits to switch on.
        set: Style,
        /// The bits to switch off.
        clear: Style,
    },
    /// Gives a range of text a colour, or with `None` the cell's text colour again.
    SetRangeColor {
        /// The cell.
        cell: String,
        /// The first code point. Positions count code points, and a range
        /// must not cut a user-perceived character.
        start: i64,
        /// How many code points.
        len: i64,
        /// The colour.
        #[serde(deserialize_with = "present")]
        color: Option<Rgb>,
    },
    /// Makes a range of text a link to `target`, replacing any links it overlaps; `None`
    /// removes the links from the range.
    SetLink {
        /// The cell.
        cell: String,
        /// The first code point. Positions count code points, and a range
        /// must not cut a user-perceived character.
        start: i64,
        /// How many code points.
        len: i64,
        /// Where the link goes: an external target or a cell. For a cell of this
        /// document, the document's own ID is dropped from the target.
        #[serde(deserialize_with = "present")]
        target: Option<LinkTarget>,
    },
    /// Sets properties of one or more cells, as one undo unit. If any of it cannot be done
    /// (a grid property on a cell without a grid, say), nothing is. Of several values for
    /// one property the last counts, and only what differs in the end is a change.
    SetProps {
        /// The cells.
        cells: Vec<String>,
        /// The properties with their new values.
        props: Vec<Prop>,
    },
    /// Switches style bits on and off for whole cells: on the base style and, as TreeSheets
    /// does, on every run. The bits of `set` on, then the bits of `clear` off.
    SetStyle {
        /// The cells.
        cells: Vec<String>,
        /// The bits to switch on.
        set: Style,
        /// The bits to switch off.
        clear: Style,
    },
    /// Sets the width of one column of a cell's grid (at least 5; smaller
    /// values become that).
    SetColumnWidth {
        /// The cell that has the grid.
        cell: String,
        /// The column.
        column: i64,
        /// The width.
        width: i64,
    },
    /// Gives a cell that has no grid a grid of new empty cells. As in TreeSheets they take
    /// the cell's colours, style, alignments, draw style and relative size, and the grid
    /// takes the border colour and spacing of the grid the cell is in.
    CreateGrid {
        /// The cell.
        cell: String,
        /// The number of columns.
        xs: i64,
        /// The number of rows.
        ys: i64,
    },
    /// Removes the grid of a cell with everything in it. Refused for the root cell.
    RemoveGrid {
        /// The cell.
        cell: String,
    },
    /// Inserts empty rows. As in TreeSheets each new cell takes the colours, style,
    /// alignments, draw style and relative size of the cell above it (below it, for rows
    /// inserted at the top).
    InsertRows {
        /// The cell that has the grid.
        cell: String,
        /// The row before which to insert; the number of rows appends.
        at: i64,
        /// How many rows.
        count: i64,
    },
    /// Inserts empty columns; like [`Command::InsertRows`], taking after the cell to the
    /// left (to the right, for columns inserted at the left edge).
    InsertColumns {
        /// The cell that has the grid.
        cell: String,
        /// The column before which to insert; the number of columns appends.
        at: i64,
        /// How many columns.
        count: i64,
    },
    /// Deletes rows with everything in them. Deleting every row removes the grid, as in
    /// TreeSheets; for the root cell that is refused.
    DeleteRows {
        /// The cell that has the grid.
        cell: String,
        /// The first row.
        at: i64,
        /// How many rows.
        count: i64,
    },
    /// Deletes columns; see [`Command::DeleteRows`].
    DeleteColumns {
        /// The cell that has the grid.
        cell: String,
        /// The first column.
        at: i64,
        /// How many columns.
        count: i64,
    },
    /// Empties cells, as TreeSheets' `Cell::Clear`: text, runs, links, image and grid go,
    /// a formula cell becomes a data cell; colours, style, note and the rest stay. Refused
    /// if the root cell is among them.
    ClearCells {
        /// The cells.
        cells: Vec<String>,
    },
    /// What the Delete key does to a selection of cells in TreeSheets: empties them, and
    /// if that leaves the selected rows (or columns) of the grid entirely empty, deletes
    /// those rows (or columns); if that would be every one, removes the grid from its cell
    /// (except for the root, whose grid stays, emptied).
    DeleteCells {
        /// The cell that has the grid.
        cell: String,
        /// The first selected column.
        x: i64,
        /// The first selected row.
        y: i64,
        /// The number of selected columns, at least 1.
        xs: i64,
        /// The number of selected rows, at least 1.
        ys: i64,
    },
    /// Moves rows within their grid.
    MoveRows {
        /// The cell that has the grid.
        cell: String,
        /// The first row to move.
        from: i64,
        /// How many rows.
        count: i64,
        /// The index the first moved row has afterwards.
        to: i64,
    },
    /// Moves columns within their grid, with their widths.
    MoveColumns {
        /// The cell that has the grid.
        cell: String,
        /// The first column to move.
        from: i64,
        /// How many columns.
        count: i64,
        /// The index the first moved column has afterwards.
        to: i64,
    },
    /// Makes two cells trade places, with everything in them; they may be in different
    /// grids, but neither may be inside the other.
    SwapCells {
        /// One cell.
        a: String,
        /// The other.
        b: String,
    },
    /// Moves a cell, with its text, properties, grid and ID, to where the empty cell `to`
    /// is (which may be in another grid). The empty cell takes its old place. Refused if
    /// `to` is not empty or is inside `from`.
    MoveCell {
        /// The cell to move.
        from: String,
        /// An empty cell (no text, image or grid) whose place it takes.
        to: String,
    },
    /// The paste operation: makes `to` a copy of `from` with everything inside it. `to`
    /// keeps its ID and place; what it held is lost; the cells of the copied grid get new
    /// IDs, and links from inside the copy to cells inside `from` follow to the copies.
    /// Refused onto the root cell.
    CopyCell {
        /// The cell to copy.
        from: String,
        /// The cell that becomes the copy.
        to: String,
    },
    /// Adds a tag to the document, or changes its colours. A cell has a tag when its
    /// text is the tag's name.
    SetTag {
        /// The name; not empty, no control characters.
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
    /// TreeSheets' Sort: reorders whole rows of a grid by the text of their cells in one
    /// column; rows equal there are ordered by the next columns in turn, wrapping round to
    /// the first, and rows equal in all columns keep their order (also when descending).
    /// Texts compare as text without regard to case: "10" comes before "9". Fewer than two
    /// rows is not an error and changes nothing. Cells keep their IDs.
    SortRows {
        /// The cell that has the grid.
        cell: String,
        /// The column to sort by first.
        column: i64,
        /// The first row that takes part.
        row: i64,
        /// How many rows take part.
        count: i64,
        /// From high to low instead of from low to high.
        descending: bool,
    },
    /// TreeSheets' Transpose: in the grid of every listed cell that has one, rows become
    /// columns. Column widths stay with their position; new columns take the width of the
    /// last old one. Cells keep their IDs.
    Transpose {
        /// The cells whose grids are transposed; those without a grid are passed over.
        cells: Vec<String>,
    },
    /// TreeSheets' Hierarchy Swap: every cell with the text of `cell` found below the grid
    /// two levels above it moves up into that grid and takes copies of the cells it was
    /// inside of with it, as a chain inside itself; cells with the same text merge into
    /// one, and parents left empty disappear. Refused unless the cell has text, a cell two
    /// levels up exists, and both the grid it is in and the grid it moves into have one
    /// column or one row. The swapped cells keep their IDs; the copy of a parent takes the
    /// parent's ID if the parent disappears, and is a new cell otherwise.
    HierarchySwap {
        /// The cell whose text is swapped upwards.
        cell: String,
    },
    /// TreeSheets' Hierarchify: turns the grid of `cell`, a table whose cells have no grids
    /// themselves, into a hierarchy: rows that start with the same text are merged under
    /// the first cell with that text, which gets a grid of the remaining columns, and so
    /// on for those. All cells keep their IDs; the first cells of repeated rows are removed.
    Hierarchify {
        /// The cell that has the table.
        cell: String,
    },
    /// TreeSheets' Flatten: replaces the grid of `cell` by a table with one row per cell
    /// inside it that has no grid, and one column per level: the cells it was inside of,
    /// then the cell itself, then empty cells. Only text (with styles, links, size, image)
    /// is carried over. Every cell keeps its ID where it first stands; repeats of a parent
    /// on later rows and the empty cells are new.
    Flatten {
        /// The cell whose grid is flattened.
        cell: String,
    },
    /// TreeSheets' Wrap in new parent: a block of cells moves into the grid of a new empty
    /// cell that takes the place of the block's first cell; rows and columns that this
    /// leaves empty are deleted. The cells of the block keep their IDs.
    Wrap {
        /// The cell that has the grid.
        cell: String,
        /// The first column of the block.
        x: i64,
        /// The first row of the block.
        y: i64,
        /// The number of columns, at least 1.
        xs: i64,
        /// The number of rows, at least 1.
        ys: i64,
    },
    /// TreeSheets' Collapse Cells: the texts of a block of two or more cells, and of the
    /// cells inside them, are appended to the block's first cell, each after a space; the
    /// other cells are emptied, and rows or columns left empty are deleted.
    Collapse {
        /// The cell that has the grid.
        cell: String,
        /// The first column of the block.
        x: i64,
        /// The first row of the block.
        y: i64,
        /// The number of columns.
        xs: i64,
        /// The number of rows.
        ys: i64,
    },
    /// TreeSheets' Split cell at the cursor: the text before and after `at` become the two
    /// cells of a new grid inside the cell, which is emptied. A grid the cell already has
    /// is kept beside them. The cell keeps its ID; the two parts are new cells.
    SplitCell {
        /// The cell.
        cell: String,
        /// Where to cut the text: a position in code points, not inside a user-perceived
        /// character.
        at: i64,
    },
    /// TreeSheets' Add Cell Text as Tag: the text of each listed cell that has text
    /// becomes a tag with the cell's colours.
    TagCells {
        /// The cells.
        cells: Vec<String>,
    },
    /// TreeSheets' Remove Cell Text from Tags: removes the tags named by the texts of the
    /// listed cells; texts that are no tags are passed over.
    UntagCells {
        /// The cells.
        cells: Vec<String>,
    },
    /// TreeSheets' Increase / Decrease text size: adds `delta` to the relative size of the
    /// cells (negative makes the text bigger), keeping each between -32 and 8.
    AdjustRelSize {
        /// The cells.
        cells: Vec<String>,
        /// What to add; TreeSheets adds 1 or -1.
        delta: i64,
        /// Also the cells inside the listed ones.
        recursive: bool,
    },
    /// TreeSheets' Increase / Decrease column width: adds `delta` to the widths of columns,
    /// none becoming smaller than 5.
    AdjustColumnWidths {
        /// The cell that has the grid.
        cell: String,
        /// The first column.
        column: i64,
        /// How many columns.
        count: i64,
        /// What to add; TreeSheets adds 5 or -5.
        delta: i64,
        /// Also every column of every grid inside the cells of those columns.
        recursive: bool,
    },
    /// TreeSheets' Program > Run: works the program of the document out (its operation,
    /// variable and view cells) and overwrites every view cell with the value that
    /// reached it. A view cell keeps its ID; the cells of a grid it gets are new. The
    /// variables the run assigned are kept for the formulas. Refused if the program nests
    /// too deeply or takes too much work.
    Run,
    /// Typing into a cell has ended with this text (also TreeSheets' formula bar): sets
    /// the text like `SetText`, and a data or formula cell becomes a formula cell if the
    /// text starts with `=` and has more than that, and a data cell if not. Cells of other
    /// kinds keep their kind. A text that the cell has already (typed with the text
    /// commands) keeps its styles and links, unless the cell is now a formula cell.
    EnterText {
        /// The cell.
        cell: String,
        /// The text typed; control characters become spaces, and half of a surrogate
        /// pair on its own arrives as U+FFFD.
        text: String,
    },
    /// TreeSheets' Toggle to-do checkbox: cells without a checkbox (U+2610 or U+2611 at
    /// the start of the text) get an open one; if all have one they become ticked; if all
    /// are ticked the checkboxes are removed. Stamps the last-edit time.
    ToggleCheck {
        /// The cells.
        cells: Vec<String>,
    },
    /// The paste operation for cells that may come from another document: makes `to` a
    /// copy of the top cell of `clip` with everything inside it, as [`Command::CopyCell`]
    /// does within a document. `to` keeps its ID and place; the cells inside get new IDs.
    /// Links in the copy keep pointing at what they pointed at: at the copies, for cells
    /// that were copied with them, and otherwise at the original cells, naming the
    /// document those are in when it is not this one. Refused onto the root cell.
    PasteCells {
        /// The cell that becomes the copy.
        to: String,
        /// What to paste.
        clip: ClipData,
    },
    /// One half of moving cells between documents: the cells of `clip`, which was taken
    /// from another document, come into this one with the IDs they have, in the place of
    /// the empty cell `to`, which is removed. Refused if `to` is not empty or is the root,
    /// or if one of the IDs is in use in this document. Cells that left this document
    /// before and are still in its history can come back (decision A84). The other half
    /// is [`Command::ReleaseCell`] in the document they came from.
    AdoptCells {
        /// An empty cell (no text, image or grid) whose place the top cell takes.
        to: String,
        /// The cells that move in.
        clip: ClipData,
    },
    /// The other half of moving cells between documents: `cell` and everything inside it
    /// leave this document, and a new empty cell with the same looks takes the place.
    /// Unlike [`Command::ClearCells`] the ID of `cell` goes with it. Refused for the root.
    ReleaseCell {
        /// The cell that leaves.
        cell: String,
    },
    /// Import: the table becomes the grid of `cell`; the cell keeps its text. The cells are new, with new IDs, and look like the cells of a new grid of
    /// `cell` would. Refused if the document would have too many cells or nest too deep.
    ///
    /// A grid that `cell` already has is replaced with everything in it. If any cell in it
    /// has content (text, a note, an image or a grid), that is done only with `replace`,
    /// and refused without. The cells that went are in [`Report::removed`].
    Import {
        /// The cell that gets the grid.
        cell: String,
        /// What to import.
        table: TableData,
        /// Whether a grid with content may be replaced.
        replace: bool,
    },
    /// Corrects links after cells went to another document: every link of this document
    /// to one of `cells` that names the document `from` (`None`: no document, the cells
    /// were its own) names `to` instead (`None`, or this document: no document). Nothing
    /// else of a link changes: its range, its unknown fields and its neighbours stay
    /// (decision A85). Reads every cell of the document.
    RetargetLinks {
        /// The cells that the links point at.
        cells: Vec<String>,
        /// The document that the links to correct name.
        #[serde(deserialize_with = "present")]
        from: Option<String>,
        /// The document that they are to name.
        #[serde(deserialize_with = "present")]
        to: Option<String>,
    },
}

/// What a command, an undo or a redo changed: the core's change report, and with it the
/// formula cells that show something else now. Everything not listed is exactly as it
/// was. All are IDs of cells, but `tags`. The first five lists are sorted and have no
/// duplicates. A report with every list empty means that nothing changed: a command that
/// asks for nothing new (setting a colour a cell already has, inserting empty text)
/// succeeds with such a report and leaves nothing to undo.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    /// Cells that existed before and still exist, whose own content changed: text, runs,
    /// links, any property, or a property of their grid (border, spacing, column widths;
    /// so a cell whose grid gained, lost or reordered columns is here as well as in
    /// `grids`, because its list of widths is different).
    pub changed: Vec<String>,
    /// Cells that did not exist before.
    pub created: Vec<String>,
    /// Cells that no longer exist.
    pub removed: Vec<String>,
    /// Cells that existed before and still exist, whose grid appeared, disappeared,
    /// changed size or has a different list of cells. A cell that moved is found here
    /// through the cell whose grid it left and the cell whose grid it entered.
    pub grids: Vec<String>,
    /// Names of document tags that were added, removed or changed.
    pub tags: Vec<String>,
    /// Formula cells whose result changed, in document order, though they may not be
    /// changed themselves: an interface that redraws only what a report names needs them.
    pub recalculated: Vec<String>,
}

/// What went wrong, for an interface to tell apart without reading the message.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ErrorKind {
    /// A file could not be read or written.
    Io,
    /// The input is not JSON text.
    Syntax,
    /// The input is not the expected kind of Wombat file.
    WrongFormat,
    /// The file was written by a newer Wombat than this one reads.
    NewerVersion,
    /// The document breaks a rule of the format or of the model: a file, or the document
    /// that carries a clip, is not what the core accepts. Nothing else is: a command with
    /// something wrong in it is [`ErrorKind::BadRequest`], [`ErrorKind::Refused`] or
    /// [`ErrorKind::Limit`].
    Invalid,
    /// What was sent over the bridge is not a request at all, and the bridge refused it
    /// before the core saw it: text that is no ID, a number out of range or not finite,
    /// unknown fields that are no JSON object, a list that is no table (a grid without
    /// a column or a cell, with a row that is not whole, or short of cells), JSON text
    /// that is no command. This is a mistake of the caller, not of the user or of a
    /// file; nothing changed.
    BadRequest,
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
    /// The file of an open document is no longer the one that was read or last written:
    /// another program changed or replaced it. Nothing was written; `path` is the file.
    ChangedOnDisk,
    /// A document of this app has the file open already, and no file is open twice;
    /// `path` is the file as it was asked for.
    AlreadyOpen,
    /// The call was made on a document that was closed; nothing was done.
    Closed,
    /// A fault in the program itself: an earlier call that changes the document did not
    /// finish (it panicked), so the document may be half changed. Every later call on it
    /// is refused with this and nothing more is saved from it; its file is as it was
    /// last saved. The handle can only be disposed of, and the file opened again.
    Internal,
}

/// An error, as a value an interface can show.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeError {
    /// What went wrong.
    pub kind: ErrorKind,
    /// The whole story in English, as the core tells it.
    pub message: String,
    /// The file concerned, if it is about one.
    #[serde(deserialize_with = "present")]
    pub path: Option<String>,
}

/// One cell as an interface lists it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellInfo {
    /// The ID of the cell.
    pub id: String,
    /// The text; for a formula cell, the formula.
    pub text: String,
    /// What the cell is to the formula system.
    pub kind: CellKind,
    /// What a formula cell shows instead of its text; `None` for every other kind.
    #[serde(deserialize_with = "present")]
    pub result: Option<String>,
    /// Whether `result` is an error value such as `#NAME?`.
    pub failed: bool,
    /// Whether the cell has a grid.
    pub has_grid: bool,
}

/// A cell and the cells of its grid.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// What an interface wants of the stage and of what is nested below it: the request of
/// `OpenDocument::stage`.
///
/// Every number must be from 0 to 4294967295; any other is refused
/// ([`ErrorKind::BadRequest`]), never cut. Within that any value is safe: a number larger
/// than what the document has means "all of it", and zero means "none". The core's
/// starting values for a phone are: the whole stage grid (`x0` 0, `y0` 0, `cols` and
/// `rows` 4294967295), `full_depth` 1, `full_cells` 24, previews of 3 by 4 cells cut at 40
/// code points, shapes of 12 by 20 cells, `text_cut` 2000.
///
/// A read is not a command and is never recorded, so this and the types of the answer
/// have no JSON form.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StageAsk {
    /// The first column of the stage grid to give, counted from 0.
    pub x0: i64,
    /// The first row of the stage grid to give, counted from 0.
    pub y0: i64,
    /// How many columns of the stage grid to give, from `x0` on.
    pub cols: i64,
    /// How many rows of the stage grid to give, from `y0` on. The window (`x0`, `y0`,
    /// `cols`, `rows`) is cut to the grid, and a window that lies outside the grid gives
    /// no cells. Only the stage grid has a window; a nested grid is given whole or not at
    /// all.
    pub rows: i64,
    /// How many levels of nested grids may be given cell by cell, counted from the stage
    /// grid: 0 none, 1 the grids of the stage grid's cells, 2 also the grids of their
    /// cells, and so on.
    pub full_depth: i64,
    /// A nested grid with more cells than this (columns times rows) is not given cell by
    /// cell, and neither is anything inside it.
    pub full_cells: i64,
    /// The columns of the top-left block whose texts a summary gives.
    pub preview_cols: i64,
    /// The rows of that block.
    pub preview_rows: i64,
    /// The columns of the top-left block that a summary's shape covers.
    pub shape_cols: i64,
    /// The rows of that block.
    pub shape_rows: i64,
    /// The most code points of the text and of the result of a [`CellLook`].
    pub text_cut: i64,
    /// The most code points of a text in a summary's preview and in the trail.
    pub preview_cut: i64,
}

/// A text that may have been cut short.
///
/// A text is only ever cut between two user-perceived characters (a letter with its
/// accents, a flag, an emoji made of several), so what is left is the longest such
/// beginning of the whole text that has at most the asked number of code points. It can
/// be shorter than asked, and is empty if the first character alone is too long.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CutText {
    /// The beginning of the text that was kept; all of it if `cut` is false.
    pub text: String,
    /// Whether anything was left out.
    pub cut: bool,
}

/// The stage and what is nested below it, in a flat form: no entry holds another, they
/// name each other by their place in the lists. The order of everything is fully
/// determined by the document and the request, so two answers can be compared entry by
/// entry to find what to redraw.
///
/// Only Rust sends this and the types in it, so their numbers are as wide as the core's.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StageInfo {
    /// The cells from the root to the stage cell, the root first and the stage cell last.
    /// Never empty; for the root it is the root alone.
    pub trail: Vec<TrailStep>,
    /// The grids given cell by cell. `grids[0]` is the stage cell's own grid (its window);
    /// if the stage cell has no grid, `grids` is empty. After it come the nested grids
    /// given whole, in the order in which the cells that own them appear in `cells`; so
    /// all grids of one level come before any grid of the next.
    pub grids: Vec<GridPart>,
    /// The cells of all `grids`: those of `grids[0]` first, then those of `grids[1]`, and
    /// so on, each grid's cells row by row. `GridPart.first` says where a grid's cells
    /// start.
    pub cells: Vec<CellLook>,
    /// One for each cell in `cells` that has a grid, in the order of those cells, whether
    /// or not that grid is also in `grids`.
    pub summaries: Vec<GridSummary>,
}

/// One cell on the way from the root to the stage: what a title bar and a trail bar show.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TrailStep {
    /// The ID of the cell.
    pub id: String,
    /// What the cell shows (the result of a formula cell, the text of any other) on one
    /// line: cut to `StageAsk.preview_cut`, and then every line break, and any other
    /// character below U+0020, replaced by a space.
    pub shown: CutText,
    /// The background colour the cell is drawn in: its tag's if it is a tag (see
    /// `CellLook.is_tag`), else its own.
    pub color: Rgb,
}

/// A grid whose cells are in `StageInfo.cells`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GridPart {
    /// The ID of the cell whose grid this is.
    pub owner: String,
    /// How far below the stage: 0 for the stage cell's grid, 1 for the grids of its cells,
    /// and so on.
    pub level: u32,
    /// The columns of the whole grid.
    pub xs: u32,
    /// The rows of the whole grid.
    pub ys: u32,
    /// The first column given. For the stage grid this is `StageAsk.x0`, or `xs` if that
    /// is past the end; for a nested grid 0.
    pub x0: u32,
    /// The first row given, likewise.
    pub y0: u32,
    /// How many columns are given: as many as asked for, or as there are from `x0` on, if
    /// those are fewer; for a nested grid all `xs`.
    pub cols: u32,
    /// How many rows are given, likewise.
    pub rows: u32,
    /// Where the cells of this grid start in `StageInfo.cells`. `cols` times `rows` cells
    /// follow, row by row: the cell in column `x0 + i` of row `y0 + j` is at
    /// `first + j * cols + i`.
    pub first: u32,
    /// The width at which text wraps (TreeSheets' unit; the default is 80) for each column
    /// that is given: `cols` entries, the first for column `x0`.
    pub col_widths: Vec<u32>,
    /// The colour of the grid's border.
    pub border_color: Rgb,
    /// The space around the grid.
    pub outer_spacing: u32,
    /// Whether the grid is below the owner's text (the default) instead of beside it.
    pub vertical: bool,
    /// How the owner wants its grid drawn.
    pub draw_style: DrawStyle,
}

/// A range of a cell's text that is styled differently from the rest.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RunInfo {
    /// The position of the first character of the range, counted in Unicode code points
    /// from 0, not in the UTF-16 units of a Dart string: turn it into an offset of the
    /// string before building spans (count the code points of the string up to it).
    pub start: u32,
    /// The length of the range in code points, at least 1.
    pub len: u32,
    /// The complete style of the range (not a difference from the cell's).
    pub style: Style,
    /// The colour of the range, or `None` for the cell's text colour.
    pub color: Option<Rgb>,
}

/// A range of a cell's text that is a link.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LinkInfo {
    /// The position of the first character of the range, counted in Unicode code points
    /// from 0, not in the UTF-16 units of a Dart string.
    pub start: u32,
    /// The length of the range in code points, at least 1.
    pub len: u32,
    /// Where the link goes. A file may hold a link of a kind this core does not know: it
    /// is given as `LinkTarget.unknown`.
    pub target: LinkTarget,
}

/// One cell with all that is needed to draw it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CellLook {
    /// The ID of the cell.
    pub id: String,
    /// The text, cut to `StageAsk.text_cut` as [`CutText`] says (from
    /// `OpenDocument::cell` it is whole); for a formula cell, the formula. It may hold
    /// line breaks (U+000A).
    pub text: String,
    /// Whether anything of the text was left out.
    pub text_cut: bool,
    /// What the cell is to the formula system.
    pub kind: CellKind,
    /// What a formula cell shows instead of its text, cut like the text; `None` for every
    /// other kind.
    pub result: Option<String>,
    /// Whether anything of the result was left out.
    pub result_cut: bool,
    /// Whether `result` is an error value such as `#NAME?`.
    pub failed: bool,
    /// The background colour to draw: the tag's if the cell is a tag, else the cell's own.
    pub cell_color: Rgb,
    /// The colour of the text outside of coloured runs: the tag's if the cell is a tag,
    /// else the cell's own.
    pub text_color: Rgb,
    /// Whether the cell is a tag: its whole text (not cut; for a formula cell, the
    /// formula) is the name of a tag of the document. As in TreeSheets, a tag is drawn in
    /// the tag's two colours and in no other: `cell_color` and `text_color` are then the
    /// tag's, and no run has a colour.
    pub is_tag: bool,
    /// The base style of the text.
    pub style: Style,
    /// The text size relative to the default; as in TreeSheets, positive is smaller.
    pub rel_size: i32,
    /// Horizontal alignment.
    pub text_align: HorizAlign,
    /// Vertical alignment.
    pub vert_align: VertAlign,
    /// The ranges of `text` styled differently from `style`: sorted, not overlapping, and
    /// inside the text as it is given here (a run that the cut goes through ends at the
    /// cut; one beyond it is left out). Neighbours of the same style and colour are one
    /// run, and no run equals `style` without a colour. Positions are in code points.
    pub runs: Vec<RunInfo>,
    /// The ranges of `text` that are links: sorted, not overlapping, and cut like the
    /// runs. Positions are in code points.
    pub links: Vec<LinkInfo>,
    /// The file name of the cell's image in the document's image folder, if it has one.
    pub image: Option<String>,
    /// Whether the cell has a note.
    pub has_note: bool,
    /// Whether the cell has a grid.
    pub has_grid: bool,
    /// The place of the cell's entry in `StageInfo.summaries`: there is one exactly if
    /// the cell has a grid. Always `None` from `OpenDocument::cell`.
    pub summary: Option<u32>,
    /// The place of the cell's grid in `StageInfo.grids`, if it is given cell by cell.
    /// Always `None` from `OpenDocument::cell`.
    pub full: Option<u32>,
}

/// A grid in reduced form: what a Preview, a Shape and a Badge are drawn from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GridSummary {
    /// The ID of the cell whose grid this is.
    pub owner: String,
    /// The columns of the grid.
    pub xs: u32,
    /// The rows of the grid. `xs` times `ys` is the count a Badge shows: the cells of
    /// this grid, without those further down.
    pub ys: u32,
    /// The colour of the grid's border.
    pub border_color: Rgb,
    /// The columns of the preview block: `StageAsk.preview_cols`, or `xs` if that is
    /// fewer.
    pub preview_cols: u32,
    /// The rows of the preview block: `StageAsk.preview_rows`, or `ys` if that is fewer.
    pub preview_rows: u32,
    /// What the cells of the grid's top-left block of `preview_cols` by `preview_rows`
    /// show, row by row (`preview_cols` times `preview_rows` entries), each on one line:
    /// cut to `StageAsk.preview_cut`, and then every line break, and any other character
    /// below U+0020, replaced by a space.
    pub preview: Vec<CutText>,
    /// The columns of the shape block: `StageAsk.shape_cols`, or `xs` if that is fewer.
    pub shape_cols: u32,
    /// The rows of the shape block: `StageAsk.shape_rows`, or `ys` if that is fewer.
    pub shape_rows: u32,
    /// The grid's top-left block of `shape_cols` by `shape_rows` cells, row by row, four
    /// bytes a cell: the red, green and blue of the background colour the cell is drawn in
    /// (its tag's if it is a tag), then how much it holds: 255 if it has a grid; else the
    /// number of code points it shows, but at most 254; so 0 is a cell that shows nothing.
    /// In Dart this is a `Uint8List`, one block of memory however large the shape.
    pub shape: Vec<u8>,
}
