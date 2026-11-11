//! Editing: the only way to change a document (decisions A9, A10, A31-A39).
//!
//! An [`Editor`] owns a document. [`Editor::apply`] carries out a [`Command`], which names
//! the cells it works on by ID; [`Editor::undo`] and [`Editor::redo`] walk the history. Each
//! of the three returns [`Changes`]: exactly which cells, grids and tags are different
//! afterwards; [`Editor::end_group`] returns them for a group of commands. Commands, changes and errors are plain data with no references into the
//! document, so they can cross a boundary to an interface written in another language.
//!
//! Inside, a command is carried out as a list of small steps, each of which knows how to
//! undo itself exactly (`step.rs`). The history keeps those steps, not copies of the
//! document. A command that fails has its steps undone before the error is returned, so the
//! document is exactly as it was.
//!
//! **Adding a command** (what milestone 3b does): add a variant to [`Command`], and in
//! `command.rs` a function that checks the request against the document and then calls
//! `Tx::step` for each step. Undo, redo, grouping, the change report and roll-back on
//! failure then work without more code. No new kind of step should be needed: `SetGrid`
//! rearranges existing cells in any way, `AddCells` and `RemoveCells` make and drop cells.
//!
//! The editor also keeps what commands build within what can be saved (`size.rs`,
//! [`Editor::size_bound`]), bounds the memory of its history, and treats the ID of a cell
//! that undo or redo can bring back as taken (decisions A43, A46, A48).

mod command;
mod paste;
mod restructure;
mod size;
mod step;

use std::collections::HashSet;

use crate::error::{refused, Error};
use crate::id::{CellId, IdSource};
use crate::limits::MAX_FILE_BYTES;
use crate::model::{
    Cell, CellKind, Color, Document, DrawStyle, ImageRef, LinkTarget, Style, TextAlign, VertAlign,
};

use step::{ChangeSet, Step};

pub use paste::Clip;

/// The smallest width of a column, as in TreeSheets.
pub const MIN_COL_WIDTH: u32 = 5;

/// The smallest relative size (the biggest text) that [`Command::AdjustRelSize`] goes to, as
/// TreeSheets' text size actions.
pub const MIN_REL_SIZE: i32 = -32;
/// The largest relative size (the smallest text) that [`Command::AdjustRelSize`] goes to.
pub const MAX_REL_SIZE: i32 = 8;

/// A property of a cell with a new value, for [`Command::SetProps`].
#[derive(Clone, PartialEq, Debug)]
pub enum Prop {
    /// The background colour. Stamps the last-edit time, as TreeSheets does, if the
    /// colour is different after the command.
    CellColor(Color),
    /// The colour of the text. As in TreeSheets this also removes the colours of the
    /// cell's runs, so that the whole text has the new colour. Stamps the last-edit time
    /// if any of that is different after the command.
    TextColor(Color),
    /// The text size relative to the default; positive is smaller.
    RelSize(i32),
    /// Horizontal alignment.
    TextAlign(TextAlign),
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
    /// The display scale of the cell's image; the cell must have one.
    ImageScale(f64),
    /// When the text was last edited, or `None` for unknown.
    LastEdit(Option<i64>),
    /// The border colour of the cell's grid; the cell must have one.
    BorderColor(Color),
    /// The outer spacing of the cell's grid; the cell must have one.
    OuterSpacing(u32),
    /// Whether the cell's grid is below its text (instead of beside it); the cell must
    /// have a grid.
    Vertical(bool),
}

/// A change to a document. Positions and lengths in text are in Unicode code points and
/// must not cut a user-perceived character (see [`crate::text`]); rows and columns are
/// counted from 0. A command that asks for nothing new (setting a colour a cell already
/// has, inserting empty text) succeeds, changes nothing and leaves no entry in the history.
#[derive(Clone, PartialEq, Debug)]
pub enum Command {
    /// Inserts text at a position. Stamps the last-edit time.
    InsertText {
        /// The cell.
        cell: CellId,
        /// Where to insert.
        at: u32,
        /// What to insert; control characters become spaces.
        text: String,
    },
    /// Deletes a range of text. Stamps the last-edit time.
    DeleteText {
        /// The cell.
        cell: CellId,
        /// The first code point to delete.
        start: u32,
        /// How many code points to delete.
        len: u32,
    },
    /// Replaces a range of text; the new text takes the style and link of the first
    /// replaced character. Stamps the last-edit time.
    ReplaceText {
        /// The cell.
        cell: CellId,
        /// The first code point to replace.
        start: u32,
        /// How many code points to replace.
        len: u32,
        /// The new text; control characters become spaces.
        text: String,
    },
    /// Replaces the whole text and, as TreeSheets does, drops all runs and links. Stamps the
    /// last-edit time.
    SetText {
        /// The cell.
        cell: CellId,
        /// The new text; control characters become spaces.
        text: String,
    },
    /// Switches style bits on and off for a range of text: the bits of `set` on, then the
    /// bits of `clear` off, the others as they are.
    SetRangeStyle {
        /// The cell.
        cell: CellId,
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
        cell: CellId,
        /// The first code point.
        start: u32,
        /// How many code points.
        len: u32,
        /// The colour.
        color: Option<Color>,
    },
    /// Makes a range of text a link to `target`, replacing any links it overlaps; `None`
    /// removes the links from the range.
    SetLink {
        /// The cell.
        cell: CellId,
        /// The first code point.
        start: u32,
        /// How many code points.
        len: u32,
        /// Where the link goes: an external target or a cell. For a cell of this
        /// document, the document's own ID is dropped from the target.
        target: Option<LinkTarget>,
    },
    /// Sets properties of one or more cells, as one undo unit. If any of it cannot be done
    /// (a grid property on a cell without a grid, say), nothing is. Of several values for
    /// one property the last counts, and only what differs in the end is a change.
    SetProps {
        /// The cells.
        cells: Vec<CellId>,
        /// The properties with their new values.
        props: Vec<Prop>,
    },
    /// Switches style bits on and off for whole cells: on the base style and, as TreeSheets
    /// does, on every run. The bits of `set` on, then the bits of `clear` off.
    SetStyle {
        /// The cells.
        cells: Vec<CellId>,
        /// The bits to switch on.
        set: Style,
        /// The bits to switch off.
        clear: Style,
    },
    /// Sets the width of one column of a cell's grid (at least [`MIN_COL_WIDTH`]; smaller
    /// values become that).
    SetColumnWidth {
        /// The cell that has the grid.
        cell: CellId,
        /// The column.
        column: u32,
        /// The width.
        width: u32,
    },
    /// Gives a cell that has no grid a grid of new empty cells. As in TreeSheets they take
    /// the cell's colours, style, alignments, draw style and relative size, and the grid
    /// takes the border colour and spacing of the grid the cell is in.
    CreateGrid {
        /// The cell.
        cell: CellId,
        /// The number of columns.
        xs: u32,
        /// The number of rows.
        ys: u32,
    },
    /// Removes the grid of a cell with everything in it. Refused for the root cell.
    RemoveGrid {
        /// The cell.
        cell: CellId,
    },
    /// Inserts empty rows. As in TreeSheets each new cell takes the colours, style,
    /// alignments, draw style and relative size of the cell above it (below it, for rows
    /// inserted at the top).
    InsertRows {
        /// The cell that has the grid.
        cell: CellId,
        /// The row before which to insert; the number of rows appends.
        at: u32,
        /// How many rows.
        count: u32,
    },
    /// Inserts empty columns; like [`Command::InsertRows`], taking after the cell to the
    /// left (to the right, for columns inserted at the left edge).
    InsertColumns {
        /// The cell that has the grid.
        cell: CellId,
        /// The column before which to insert; the number of columns appends.
        at: u32,
        /// How many columns.
        count: u32,
    },
    /// Deletes rows with everything in them. Deleting every row removes the grid, as in
    /// TreeSheets; for the root cell that is refused.
    DeleteRows {
        /// The cell that has the grid.
        cell: CellId,
        /// The first row.
        at: u32,
        /// How many rows.
        count: u32,
    },
    /// Deletes columns; see [`Command::DeleteRows`].
    DeleteColumns {
        /// The cell that has the grid.
        cell: CellId,
        /// The first column.
        at: u32,
        /// How many columns.
        count: u32,
    },
    /// Empties cells, as TreeSheets' `Cell::Clear`: text, runs, links, image and grid go,
    /// a formula cell becomes a data cell; colours, style, note and the rest stay. Refused
    /// if the root cell is among them.
    ClearCells {
        /// The cells.
        cells: Vec<CellId>,
    },
    /// What the Delete key does to a selection of cells in TreeSheets: empties them, and
    /// if that leaves the selected rows (or columns) of the grid entirely empty, deletes
    /// those rows (or columns); if that would be every one, removes the grid from its cell
    /// (except for the root, whose grid stays, emptied).
    DeleteCells {
        /// The cell that has the grid.
        cell: CellId,
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
        cell: CellId,
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
        cell: CellId,
        /// The first column to move.
        from: u32,
        /// How many columns.
        count: u32,
        /// The index the first moved column has afterwards.
        to: u32,
    },
    /// Makes two cells trade places, with everything in them; they may be in different
    /// grids, but neither may be inside the other.
    SwapCells {
        /// One cell.
        a: CellId,
        /// The other.
        b: CellId,
    },
    /// Moves a cell, with its text, properties, grid and ID, to where the empty cell `to`
    /// is (which may be in another grid). The empty cell takes its old place. Refused if
    /// `to` is not empty or is inside `from`.
    MoveCell {
        /// The cell to move.
        from: CellId,
        /// An empty cell (no text, image or grid) whose place it takes.
        to: CellId,
    },
    /// The paste operation: makes `to` a copy of `from` with everything inside it. `to`
    /// keeps its ID and place; what it held is lost; the cells of the copied grid get new
    /// IDs, and links from inside the copy to cells inside `from` follow to the copies.
    /// Refused onto the root cell.
    CopyCell {
        /// The cell to copy.
        from: CellId,
        /// The cell that becomes the copy.
        to: CellId,
    },
    /// Adds a tag to the document, or changes its colours. A cell has a tag when its
    /// text is the tag's name.
    SetTag {
        /// The name; not empty, no control characters.
        name: String,
        /// The background colour of cells with the tag.
        cell_color: Color,
        /// The text colour of cells with the tag.
        text_color: Color,
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
        cell: CellId,
        /// The column to sort by first.
        column: u32,
        /// The first row that takes part.
        row: u32,
        /// How many rows take part.
        count: u32,
        /// From high to low instead of from low to high.
        descending: bool,
    },
    /// TreeSheets' Transpose: in the grid of every listed cell that has one, rows become
    /// columns. Column widths stay with their position; new columns take the width of the
    /// last old one. Cells keep their IDs.
    Transpose {
        /// The cells whose grids are transposed; those without a grid are passed over.
        cells: Vec<CellId>,
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
        cell: CellId,
    },
    /// TreeSheets' Hierarchify: turns the grid of `cell`, a table whose cells have no grids
    /// themselves, into a hierarchy: rows that start with the same text are merged under
    /// the first cell with that text, which gets a grid of the remaining columns, and so
    /// on for those. All cells keep their IDs; the first cells of repeated rows are removed.
    Hierarchify {
        /// The cell that has the table.
        cell: CellId,
    },
    /// TreeSheets' Flatten: replaces the grid of `cell` by a table with one row per cell
    /// inside it that has no grid, and one column per level: the cells it was inside of,
    /// then the cell itself, then empty cells. Only text (with styles, links, size, image)
    /// is carried over. Every cell keeps its ID where it first stands; repeats of a parent
    /// on later rows and the empty cells are new.
    Flatten {
        /// The cell whose grid is flattened.
        cell: CellId,
    },
    /// TreeSheets' Wrap in new parent: a block of cells moves into the grid of a new empty
    /// cell that takes the place of the block's first cell; rows and columns that this
    /// leaves empty are deleted. The cells of the block keep their IDs.
    Wrap {
        /// The cell that has the grid.
        cell: CellId,
        /// The first column of the block.
        x: u32,
        /// The first row of the block.
        y: u32,
        /// The number of columns, at least 1.
        xs: u32,
        /// The number of rows, at least 1.
        ys: u32,
    },
    /// TreeSheets' Collapse Cells: the texts of a block of two or more cells, and of the
    /// cells inside them, are appended to the block's first cell, each after a space; the
    /// other cells are emptied, and rows or columns left empty are deleted.
    Collapse {
        /// The cell that has the grid.
        cell: CellId,
        /// The first column of the block.
        x: u32,
        /// The first row of the block.
        y: u32,
        /// The number of columns.
        xs: u32,
        /// The number of rows.
        ys: u32,
    },
    /// TreeSheets' Split cell at the cursor: the text before and after `at` become the two
    /// cells of a new grid inside the cell, which is emptied. A grid the cell already has
    /// is kept beside them. The cell keeps its ID; the two parts are new cells.
    SplitCell {
        /// The cell.
        cell: CellId,
        /// Where to cut the text.
        at: u32,
    },
    /// TreeSheets' Add Cell Text as Tag: the text of each listed cell that has text
    /// becomes a tag with the cell's colours.
    TagCells {
        /// The cells.
        cells: Vec<CellId>,
    },
    /// TreeSheets' Remove Cell Text from Tags: removes the tags named by the texts of the
    /// listed cells; texts that are no tags are passed over.
    UntagCells {
        /// The cells.
        cells: Vec<CellId>,
    },
    /// TreeSheets' Increase / Decrease text size: adds `delta` to the relative size of the
    /// cells (negative makes the text bigger), keeping each between [`MIN_REL_SIZE`] and
    /// [`MAX_REL_SIZE`].
    AdjustRelSize {
        /// The cells.
        cells: Vec<CellId>,
        /// What to add; TreeSheets adds 1 or -1.
        delta: i32,
        /// Also the cells inside the listed ones.
        recursive: bool,
    },
    /// TreeSheets' Increase / Decrease column width: adds `delta` to the widths of columns,
    /// none becoming smaller than [`MIN_COL_WIDTH`].
    AdjustColumnWidths {
        /// The cell that has the grid.
        cell: CellId,
        /// The first column.
        column: u32,
        /// How many columns.
        count: u32,
        /// What to add; TreeSheets adds 5 or -5.
        delta: i32,
        /// Also every column of every grid inside the cells of those columns.
        recursive: bool,
    },
    /// TreeSheets' Toggle to-do checkbox: cells without a checkbox (U+2610 or U+2611 at
    /// the start of the text) get an open one; if all have one they become ticked; if all
    /// are ticked the checkboxes are removed. Stamps the last-edit time.
    ToggleCheck {
        /// The cells.
        cells: Vec<CellId>,
    },
    /// The paste operation for cells that may come from another document: makes `to` a
    /// copy of the top cell of `clip` with everything inside it, as [`Command::CopyCell`]
    /// does within a document. `to` keeps its ID and place; the cells inside get new IDs.
    /// Links in the copy keep pointing at what they pointed at: at the copies, for cells
    /// that were copied with them, and otherwise at the original cells, naming the
    /// document those are in when it is not this one. Refused onto the root cell.
    PasteCells {
        /// The cell that becomes the copy.
        to: CellId,
        /// What to paste.
        clip: Clip,
    },
    /// One half of moving cells between documents: the cells of `clip`, which was taken
    /// from another document, come into this one with the IDs they have, in the place of
    /// the empty cell `to`, which is removed. Refused if `to` is not empty or is the root,
    /// or if one of the IDs is in use in this document or its history. The other half is
    /// [`Command::ReleaseCell`] in the document they came from;
    /// [`crate::links::move_between`] does both.
    AdoptCells {
        /// An empty cell (no text, image or grid) whose place the top cell takes.
        to: CellId,
        /// The cells that move in.
        clip: Clip,
    },
    /// The other half of moving cells between documents: `cell` and everything inside it
    /// leave this document, and a new empty cell with the same looks takes the place.
    /// Unlike [`Command::ClearCells`] the ID of `cell` goes with it. Refused for the root.
    ReleaseCell {
        /// The cell that leaves.
        cell: CellId,
    },
}

/// What a command, an undo or a redo changed. Everything not listed is exactly as it was.
/// Each list is sorted by ID and has no duplicates.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Changes {
    /// Cells that existed before and still exist, whose own content changed: text, runs,
    /// links, any property, or a property of their grid (border, spacing, column widths;
    /// so a cell whose grid gained, lost or reordered columns is here as well as in
    /// `grids`, because its list of widths is different).
    pub changed: Vec<CellId>,
    /// Cells that did not exist before.
    pub created: Vec<CellId>,
    /// Cells that no longer exist.
    pub removed: Vec<CellId>,
    /// Cells that existed before and still exist, whose grid appeared, disappeared, changed
    /// size or has a different list of cells. A cell that moved is found here through the
    /// cell whose grid it left and the cell whose grid it entered.
    pub grids: Vec<CellId>,
    /// Names of document tags that were added, removed or changed.
    pub tags: Vec<String>,
}

impl Changes {
    /// Whether nothing changed.
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty()
            && self.created.is_empty()
            && self.removed.is_empty()
            && self.grids.is_empty()
            && self.tags.is_empty()
    }
}

/// A command in progress: applies steps and remembers how to undo them.
pub(crate) struct Tx<'a> {
    doc: &'a mut Document,
    /// The undoing step of each step applied, in the order applied.
    inverses: Vec<Step>,
    seen: ChangeSet,
    /// The IDs of the cells that the history holds.
    held: &'a HashSet<CellId>,
    /// The largest file that the document may become.
    size_limit: u64,
}

impl Tx<'_> {
    pub(crate) fn doc(&self) -> &Document {
        self.doc
    }

    /// Applies one step.
    pub(crate) fn step(&mut self, step: Step) -> Result<(), Error> {
        let inverse = step.apply(self.doc, &mut self.seen)?;
        self.inverses.push(inverse);
        Ok(())
    }

    /// Undoes every step applied so far.
    fn roll_back(self) -> Result<(), Error> {
        roll_back(self.doc, self.inverses)
    }
}

fn roll_back(doc: &mut Document, inverses: Vec<Step>) -> Result<(), Error> {
    let mut seen = ChangeSet::default();
    for step in inverses.into_iter().rev() {
        step.apply(doc, &mut seen)?;
    }
    Ok(())
}

/// One entry of the history: the steps that undo (or redo) one command or one group.
#[derive(Clone, Debug)]
struct Unit {
    /// In the order to apply them.
    steps: Vec<Step>,
    /// The cell, if the unit is nothing but edits of that cell's text.
    typing: Option<CellId>,
    /// About how much memory the steps take.
    cost: usize,
}

impl Unit {
    fn new(steps: Vec<Step>, typing: Option<CellId>) -> Unit {
        let cost = steps.iter().map(size::held).fold(0, usize::saturating_add);
        Unit {
            steps,
            typing,
            cost,
        }
    }
}

/// The IDs of the cells that the steps hold: cells that are not in the document but come
/// back when the steps are applied.
fn held_ids(steps: &[Step]) -> impl Iterator<Item = CellId> + '_ {
    fn cells(step: &Step) -> &[Cell] {
        match step {
            Step::AddCells(cells) => cells,
            _ => &[],
        }
    }
    steps.iter().flat_map(cells).map(Cell::id)
}

/// The number of units the undo history keeps unless [`Editor::set_history_limit`] says
/// otherwise.
pub const DEFAULT_HISTORY_UNITS: usize = 1000;
/// About how many bytes of memory the undo history takes at most unless
/// [`Editor::set_history_budget`] says otherwise.
pub const DEFAULT_HISTORY_BYTES: usize = 64 * 1024 * 1024;

/// A document with its undo and redo history. See the module documentation.
#[derive(Clone, Debug)]
pub struct Editor {
    doc: Document,
    /// Each entry undoes one unit.
    undo: Vec<Unit>,
    /// Each entry redoes one unit.
    redo: Vec<Unit>,
    /// The undo entries of the commands of the open group, oldest first.
    group: Vec<Vec<Step>>,
    /// What the commands of the open group touched.
    group_seen: ChangeSet,
    group_depth: u32,
    limit: Option<usize>,
    budget: Option<usize>,
    /// The sum of the costs of the undo entries.
    undo_cost: usize,
    /// The IDs of the cells held by `undo`, `redo` and `group`.
    held: HashSet<CellId>,
    /// The bound on the size of the saved file (`size.rs`).
    size: u64,
    size_limit: u64,
    /// Whether the document as it is now can be saved within the limit, if that is known
    /// from having tried. Only looked at while `size` is over the limit.
    fits: Option<bool>,
}

/// An upper bound on the size of the file that saving `doc` would write, in bytes, found
/// without writing it: see [`Editor::size_bound`], which is this number kept up to date.
pub fn size_bound(doc: &Document) -> u64 {
    size::document(doc)
}

fn too_large(limit: u64) -> Error {
    Error::Limit(format!(
        "the document would be too large to save (over {limit} bytes)"
    ))
}

impl Editor {
    /// Starts editing a document, with an empty history. Refuses a document that breaks
    /// the model's rules, so that every command starts from a valid one.
    pub fn new(doc: Document) -> Result<Editor, Error> {
        doc.validate()?;
        Ok(Editor {
            size: size::document(&doc),
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
            group: Vec::new(),
            group_seen: ChangeSet::default(),
            group_depth: 0,
            limit: Some(DEFAULT_HISTORY_UNITS),
            budget: Some(DEFAULT_HISTORY_BYTES),
            undo_cost: 0,
            held: HashSet::new(),
            size_limit: MAX_FILE_BYTES,
            fits: None,
        })
    }

    /// The document as it is now, to read.
    pub fn document(&self) -> &Document {
        &self.doc
    }

    /// Ends editing and gives the document back; the history is dropped.
    pub fn into_document(self) -> Document {
        self.doc
    }

    /// Carries out a command. `now` is the time for commands that stamp a cell's last-edit
    /// time (milliseconds since 1970 UTC), or `None` to stamp nothing; `ids` gives the IDs
    /// of cells the command creates. On an error the document and the history are exactly
    /// as before. A command that changes something becomes one unit of the undo history
    /// (or part of the open group) and empties the redo history.
    ///
    /// A command that would make the document too large to save is refused with
    /// [`Error::Limit`]: see [`Editor::size_bound`].
    pub fn apply(
        &mut self,
        command: Command,
        now: Option<i64>,
        ids: &mut dyn IdSource,
    ) -> Result<Changes, Error> {
        self.run(command, now, ids, false)
    }

    /// As [`Editor::apply`], for typing: if `command` is an edit of one cell's text
    /// ([`Command::InsertText`], [`Command::DeleteText`], [`Command::ReplaceText`]) and
    /// the newest unit of the undo history is nothing but edits of the text of the same
    /// cell, the command joins that unit instead of becoming a unit of its own, so that one
    /// undo takes back the whole run of typing. Otherwise, and inside a group, it is
    /// exactly `apply`. The caller decides when typing continues (the same cell, no pause,
    /// no cursor movement in between): the core has no clock and no cursor.
    pub fn apply_typing(
        &mut self,
        command: Command,
        now: Option<i64>,
        ids: &mut dyn IdSource,
    ) -> Result<Changes, Error> {
        self.run(command, now, ids, true)
    }

    fn run(
        &mut self,
        command: Command,
        now: Option<i64>,
        ids: &mut dyn IdSource,
        join: bool,
    ) -> Result<Changes, Error> {
        let typing = match &command {
            Command::InsertText { cell, .. }
            | Command::DeleteText { cell, .. }
            | Command::ReplaceText { cell, .. } => Some(*cell),
            _ => None,
        };
        // Over the limit by the bound, the document may still fit: then it has to be
        // written out to know (see `size_bound`).
        let fitted = self.size > self.size_limit && self.fits();
        let mut tx = Tx {
            doc: &mut self.doc,
            inverses: Vec::new(),
            seen: ChangeSet::default(),
            held: &self.held,
            size_limit: self.size_limit,
        };
        if let Err(error) = command::run(&mut tx, command, now, ids) {
            tx.roll_back()?;
            return Err(error);
        }
        let Tx { inverses, seen, .. } = tx;
        if inverses.is_empty() {
            return Ok(Changes::default());
        }
        let size = self.size.saturating_add_signed(seen.bytes);
        if size > self.size_limit {
            let grew = size > self.size;
            if grew || (fitted && !self.written_fits()) {
                roll_back(&mut self.doc, inverses)?;
                return Err(too_large(self.size_limit));
            }
        }
        self.size = size;
        // While the bound is over the limit: a document that fitted was just written out
        // and still fits; one that did not is not looked at again until an undo or redo.
        self.fits = Some(fitted);
        let steps: Vec<Step> = inverses.into_iter().rev().collect();
        self.held.extend(held_ids(&steps));
        self.forget_redo();
        if self.group_depth > 0 {
            self.group.push(steps);
            self.group_seen.absorb(seen.clone());
        } else {
            match self.undo.last_mut() {
                Some(last) if join && typing.is_some() && last.typing == typing => {
                    let mut unit = Unit::new(steps, typing);
                    self.undo_cost = self.undo_cost.saturating_add(unit.cost);
                    last.cost = last.cost.saturating_add(unit.cost);
                    // To undo: the newest command first.
                    unit.steps.append(&mut last.steps);
                    last.steps = unit.steps;
                    self.trim();
                }
                _ => self.push_undo(Unit::new(steps, typing)),
            }
        }
        Ok(seen.finish(&self.doc))
    }

    /// Whether the document as it is can be saved within the limit, by writing it out.
    fn written_fits(&self) -> bool {
        crate::format::to_string(&self.doc).is_ok_and(|text| text.len() as u64 <= self.size_limit)
    }

    fn fits(&mut self) -> bool {
        let fits = self.fits.unwrap_or_else(|| self.written_fits());
        self.fits = Some(fits);
        fits
    }

    fn forget(held: &mut HashSet<CellId>, steps: &[Step]) {
        for id in held_ids(steps) {
            held.remove(&id);
        }
    }

    fn forget_redo(&mut self) {
        for unit in self.redo.drain(..) {
            Editor::forget(&mut self.held, &unit.steps);
        }
    }

    fn push_undo(&mut self, unit: Unit) {
        self.undo_cost = self.undo_cost.saturating_add(unit.cost);
        self.undo.push(unit);
        self.trim();
    }

    /// Forgets the oldest units while the history is over its bounds. The newest unit
    /// stays whatever it takes, unless the history is to keep no units at all.
    fn trim(&mut self) {
        let mut over = self
            .undo
            .len()
            .saturating_sub(self.limit.unwrap_or(usize::MAX));
        let mut cost: usize = self.undo.iter().skip(over).map(|unit| unit.cost).sum();
        while let Some(budget) = self.budget {
            let Some(oldest) = self.undo.get(over) else {
                break;
            };
            if cost <= budget || over.saturating_add(1) >= self.undo.len() {
                break;
            }
            cost = cost.saturating_sub(oldest.cost);
            over = over.saturating_add(1);
        }
        for unit in self.undo.drain(..over) {
            Editor::forget(&mut self.held, &unit.steps);
        }
        self.undo_cost = cost;
    }

    /// Applies the steps of one history entry and returns the entry that reverses it.
    fn play(&mut self, unit: Unit) -> Result<(Unit, Changes), Error> {
        Editor::forget(&mut self.held, &unit.steps);
        let mut tx = Tx {
            doc: &mut self.doc,
            inverses: Vec::with_capacity(unit.steps.len()),
            seen: ChangeSet::default(),
            held: &self.held,
            size_limit: self.size_limit,
        };
        for step in unit.steps {
            if let Err(error) = tx.step(step) {
                // Cannot happen while the history matches the document; if it does, at
                // least leave the document as it was.
                tx.roll_back()?;
                return Err(error);
            }
        }
        let Tx { inverses, seen, .. } = tx;
        // Undo and redo are never refused for size: they lead to a document that was.
        self.size = self.size.saturating_add_signed(seen.bytes);
        self.fits = None;
        let steps: Vec<Step> = inverses.into_iter().rev().collect();
        self.held.extend(held_ids(&steps));
        Ok((Unit::new(steps, unit.typing), seen.finish(&self.doc)))
    }

    /// Undoes the latest unit of the history: one command, or one group. Returns what that
    /// changed, or `None` if there is nothing to undo. Refused while a group is open.
    pub fn undo(&mut self) -> Result<Option<Changes>, Error> {
        if self.group_depth > 0 {
            return refused("undo while a group of commands is open");
        }
        let Some(unit) = self.undo.pop() else {
            return Ok(None);
        };
        self.undo_cost = self.undo_cost.saturating_sub(unit.cost);
        let (reverse, changes) = self.play(unit)?;
        self.redo.push(reverse);
        Ok(Some(changes))
    }

    /// Redoes the unit that was undone last, giving created cells the IDs they had.
    /// Returns what that changed, or `None` if there is nothing to redo.
    pub fn redo(&mut self) -> Result<Option<Changes>, Error> {
        if self.group_depth > 0 {
            return refused("redo while a group of commands is open");
        }
        let Some(unit) = self.redo.pop() else {
            return Ok(None);
        };
        let (reverse, changes) = self.play(unit)?;
        self.push_undo(reverse);
        Ok(Some(changes))
    }

    /// Opens a group: the commands applied until the matching [`Editor::end_group`] are
    /// undone and redone as one unit. Groups may be nested; only the outermost counts.
    pub fn begin_group(&mut self) {
        self.group_depth = self.group_depth.saturating_add(1);
    }

    /// Closes a group. Closing the outermost group returns what the group changed as a
    /// whole, between the document before its first command and the document now: a cell
    /// made and removed again inside the group is not named. (A cell that was changed and
    /// changed back may be.) Closing an inner group, or closing without an open group,
    /// does nothing and returns an empty report.
    pub fn end_group(&mut self) -> Changes {
        if self.group_depth == 0 {
            return Changes::default();
        }
        self.group_depth -= 1;
        if self.group_depth > 0 {
            return Changes::default();
        }
        if !self.group.is_empty() {
            // To undo the group, undo its commands from the newest to the oldest.
            let steps = self.group.drain(..).rev().flatten().collect();
            self.push_undo(Unit::new(steps, None));
        }
        std::mem::take(&mut self.group_seen).finish(&self.doc)
    }

    /// The number of units that can be undone.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// The number of units that can be redone.
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Bounds the undo history to the `limit` newest units (`None`: no bound; the default
    /// is [`DEFAULT_HISTORY_UNITS`]). Older units are forgotten: their changes stay, but
    /// cannot be undone.
    pub fn set_history_limit(&mut self, limit: Option<usize>) {
        self.limit = limit;
        self.trim();
    }

    /// Bounds the memory of the undo history to about `bytes` (`None`: no bound; the
    /// default is [`DEFAULT_HISTORY_BYTES`]): the oldest units are forgotten while the
    /// rest takes more, but the newest unit always stays, so that the last command can be
    /// undone whatever it removed.
    pub fn set_history_budget(&mut self, bytes: Option<usize>) {
        self.budget = bytes;
        self.trim();
    }

    /// About how many bytes of memory the undo history takes now: the steps and what they
    /// hold (removed cells, replaced text), without what unknown fields hold.
    pub fn history_bytes(&self) -> usize {
        self.undo_cost
    }

    /// Whether a group of commands is open ([`Editor::begin_group`]).
    pub fn in_group(&self) -> bool {
        self.group_depth > 0
    }

    /// Forgets what could be redone; the document and the undo history stay as they are.
    pub fn clear_redo(&mut self) {
        self.forget_redo();
    }

    /// Forgets the whole history; the document stays as it is.
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.undo_cost = 0;
        self.held.clear();
        self.held
            .extend(self.group.iter().flat_map(|steps| held_ids(steps)));
    }

    /// An upper bound on the size of the file that saving the document as it is now would
    /// write, in bytes. The editor keeps it up to date from what each command changes, at
    /// no cost that depends on the size of the document, and refuses with [`Error::Limit`]
    /// a command that would take it over the limit ([`MAX_FILE_BYTES`], or what
    /// [`Editor::set_size_limit`] says). So a document that commands built can always be
    /// saved and loaded again.
    ///
    /// The bound counts every cell as if it were nested as deep as allowed, so it can be
    /// over the limit for a document that was loaded from a file close to the limit. Such
    /// a document can still be edited: commands that do not raise the bound are carried
    /// out, each after writing the document out once to see that it still fits (which
    /// takes time in proportion to the document), until the bound is under the limit
    /// again. Undo and redo are never refused.
    pub fn size_bound(&self) -> u64 {
        self.size
    }

    /// Lowers the size that commands may take the saved document to; values over
    /// [`MAX_FILE_BYTES`] mean that. For an interface with little memory, and for tests.
    pub fn set_size_limit(&mut self, bytes: u64) {
        self.size_limit = bytes.min(MAX_FILE_BYTES);
        self.fits = None;
    }
}

#[cfg(test)]
mod tests;
