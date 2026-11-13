//! View state: which grids are folded, the selection, the level that is shown (the zoom) and
//! what each level remembers (decision A8).
//!
//! View state is not content. It is kept in its own small JSON file, apart from the document
//! (see [`crate::storage`] for where), and refers to cells by ID. It may be out of date with
//! the document, so it can name cells that no longer exist; [`ViewState::retain_existing`]
//! drops those.

use std::collections::BTreeSet;

use crate::edit::Changes;
use crate::error::{invalid, Error};
use crate::id::{CellId, DocId};
use crate::json::{
    into_extra, into_object, parse, take_array, take_header, take_id, take_u32, Fields, Kind,
};
use crate::limits::{MAX_CELLS, MAX_DEPTH};
use crate::model::{check_extra, Document, Extra, FreshIds};

/// The value of the `format` field of a view-state file.
pub const VIEW_FORMAT_NAME: &str = "wombat-view";
/// The major version of the view-state format this core writes and reads.
pub const VIEW_FORMAT_VERSION: u32 = 1;
/// The minor version of the view-state format this core writes.
pub const VIEW_FORMAT_MINOR: u32 = 1;
/// The most levels a view state remembers. A screen shows one level at a time, so this is
/// how many places of one document a device can come back to as they were left.
pub const MAX_LEVELS: usize = 256;

/// A selection: a rectangle of cells in one grid, or a text cursor in one cell.
#[derive(Clone, PartialEq, Debug)]
pub struct Selection {
    /// The top-left cell of the selection.
    pub cell: CellId,
    /// The number of columns selected, from `cell` rightwards. As in TreeSheets, 0 means an
    /// insertion point before the cell rather than the cell itself.
    pub xs: u32,
    /// The number of rows selected, from `cell` downwards; 0 as for `xs`.
    pub ys: u32,
    /// The text cursor inside `cell` when its text is being edited: start and end of the
    /// selected text in code points (equal for a plain cursor).
    pub cursor: Option<(u32, u32)>,
    /// Unrecognised fields.
    pub extra: Extra,
}

/// What one level remembers on this device: where it was scrolled to and what was selected
/// there. A level is a cell whose grid fills the screen.
#[derive(Clone, PartialEq, Debug)]
pub struct Level {
    /// The cell whose grid is the level.
    pub cell: CellId,
    /// The column of the grid at the left edge of what was last seen. A column and a row
    /// rather than a distance, which would mean nothing on a screen of another size or
    /// after the text size changed.
    pub x: u32,
    /// The row of the grid at the top edge of what was last seen.
    pub y: u32,
    /// The cell that was selected there, if one was.
    pub selected: Option<CellId>,
    /// Unrecognised fields.
    pub extra: Extra,
}

impl Level {
    /// The level of `cell` as it is before anyone has been there: at the top-left, nothing
    /// selected.
    pub fn new(cell: CellId) -> Level {
        Level {
            cell,
            x: 0,
            y: 0,
            selected: None,
            extra: Extra::new(),
        }
    }
}

/// The view state of one document on one device. Two are equal when they are written as
/// the same file.
#[derive(Clone, Debug)]
pub struct ViewState {
    /// The document this belongs to.
    pub doc: DocId,
    /// The cells whose grid is folded (hidden).
    pub folded: BTreeSet<CellId>,
    /// The selection as TreeSheets has it, one for the document: a rectangle of cells, or
    /// the text cursor while a cell is edited. What a screen that shows one level at a time
    /// has selected in each level is in `levels`.
    pub selection: Option<Selection>,
    /// The level that is shown: the cell that is zoomed into and shown as if it were the
    /// root. `None` is the root itself. It may name a cell that is gone or has no grid;
    /// [`ViewState::stage`] says what to show then.
    pub zoom: Option<CellId>,
    /// What each level remembers, the one remembered last at the end; at most
    /// [`MAX_LEVELS`], no cell twice. The level that is shown may be among them: it is
    /// remembered like any other when it is left or when the app is put away.
    pub levels: Vec<Level>,
    /// Unrecognised fields.
    pub extra: Extra,
    minor: u32,
    /// The cell `zoom` and the cells above it, nearest first, as they were when the
    /// document was last looked at. Once the zoom cell is removed the document no longer
    /// says where it was, and this does. Not saved: a file read later is followed from the
    /// document as it is then.
    line: Vec<CellId>,
}

impl PartialEq for ViewState {
    fn eq(&self, other: &ViewState) -> bool {
        // Written out so that a new field cannot be forgotten here.
        let ViewState {
            doc,
            folded,
            selection,
            zoom,
            levels,
            extra,
            minor,
            line: _,
        } = self;
        (doc, folded, selection, zoom, levels, extra, minor)
            == (
                &other.doc,
                &other.folded,
                &other.selection,
                &other.zoom,
                &other.levels,
                &other.extra,
                &other.minor,
            )
    }
}

const KIND: Kind = Kind {
    format: VIEW_FORMAT_NAME,
    version: VIEW_FORMAT_VERSION,
    id_key: "doc",
};

const VIEW_KEYS: &[&str] = &[
    "format",
    "version",
    "minor",
    "doc",
    "selection",
    "zoom",
    "levels",
    "folded",
];
const SELECTION_KEYS: &[&str] = &["cell", "xs", "ys", "cursor"];
const LEVEL_KEYS: &[&str] = &["cell", "x", "y", "selected"];

/// `cell` and the cells above it in `doc`, nearest first.
fn line_of(doc: &Document, cell: CellId) -> Vec<CellId> {
    let mut line = Vec::new();
    let mut at = doc.cell(cell);
    // No cell is deeper than MAX_DEPTH, so this is the whole way up.
    for _ in 0..=MAX_DEPTH {
        let Some(cell) = at else { break };
        line.push(cell.id());
        at = cell.parent().and_then(|parent| doc.cell(parent));
    }
    line
}

impl ViewState {
    /// An empty view state: nothing folded, no selection, not zoomed, nothing remembered.
    pub fn new(doc: DocId) -> ViewState {
        ViewState {
            doc,
            folded: BTreeSet::new(),
            selection: None,
            zoom: None,
            levels: Vec::new(),
            extra: Extra::new(),
            minor: VIEW_FORMAT_MINOR,
            line: Vec::new(),
        }
    }

    /// The level to show: the cell whose grid fills the screen. That is the zoom cell if
    /// it is in `doc` and has a grid; otherwise the nearest cell above it that has one
    /// (above where it last was, if it is gone and this view state followed the document
    /// until then); otherwise the root, which is the one level that may have no grid.
    /// Always a cell of `doc`, so a view state that is out of date is never an error: the
    /// screen opens further up. It changes nothing: `zoom` is as it was.
    pub fn stage(&self, doc: &Document) -> CellId {
        let known = |id: &CellId| doc.cell(*id).is_some();
        let at = self
            .zoom
            .filter(known)
            .or_else(|| match self.line.split_first() {
                Some((first, above)) if Some(*first) == self.zoom => {
                    above.iter().copied().find(known)
                }
                _ => None,
            });
        match at.and_then(|id| doc.cell(id)) {
            Some(cell) if cell.grid().is_some() => cell.id(),
            // One step is the whole way: a cell is in the grid of the cell above it.
            Some(cell) => cell.parent().filter(known).unwrap_or_else(|| doc.root()),
            None => doc.root(),
        }
    }

    /// Makes `cell` the level that is shown. Like folding it is view state: not a command
    /// and not in the undo history. Refused for a cell without a grid, except the root,
    /// which is where a new document starts. The level that is left is not remembered by
    /// this: call [`ViewState::remember`] with it first.
    pub fn go(&mut self, doc: &Document, cell: CellId) -> Result<(), Error> {
        match doc.cell(cell) {
            None => Err(Error::NotFound(format!("cell {cell}"))),
            Some(c) if c.grid().is_none() && cell != doc.root() => {
                Err(Error::Refused(format!("cell {cell} has no grid to show")))
            }
            Some(_) => {
                self.zoom = (cell != doc.root()).then_some(cell);
                self.line = self.zoom.map_or_else(Vec::new, |zoom| line_of(doc, zoom));
                Ok(())
            }
        }
    }

    /// Remembers `level` in place of what was remembered of its cell before, as the one
    /// remembered last. When there are more than [`MAX_LEVELS`], those remembered longest
    /// ago are forgotten. Refused for a cell without a grid; a position beyond the grid
    /// becomes its last column or row.
    pub fn remember(&mut self, doc: &Document, mut level: Level) -> Result<(), Error> {
        let Some(cell) = doc.cell(level.cell) else {
            return Err(Error::NotFound(format!("cell {}", level.cell)));
        };
        let Some(grid) = cell.grid() else {
            return Err(Error::Refused(format!(
                "cell {} has no grid to remember",
                level.cell
            )));
        };
        if let Some(selected) = level.selected.filter(|id| doc.cell(*id).is_none()) {
            return Err(Error::NotFound(format!("cell {selected}")));
        }
        level.x = level.x.min(grid.xs().saturating_sub(1));
        level.y = level.y.min(grid.ys().saturating_sub(1));
        self.levels.retain(|l| l.cell != level.cell);
        self.levels.push(level);
        let over = self.levels.len().saturating_sub(MAX_LEVELS);
        self.levels.drain(..over);
        Ok(())
    }

    /// What is remembered of the level `cell`, if anything.
    pub fn level(&self, cell: CellId) -> Option<&Level> {
        self.levels.iter().find(|l| l.cell == cell)
    }

    /// Brings the levels and the zoom in line with `doc` as it is now. What a level
    /// remembers goes when its cell is gone or has no grid (as a fold does); its selected
    /// cell is forgotten when that is gone; its position is kept inside the grid. A zoom
    /// cell that is gone gives way to the level that [`ViewState::stage`] shows instead.
    /// The cost is bounded by [`MAX_LEVELS`], whatever the size of the document. Returns
    /// whether anything that is written to the file changed.
    fn settle(&mut self, doc: &Document) -> bool {
        let mut changed = false;
        self.levels.retain_mut(|level| {
            let Some(grid) = doc.cell(level.cell).and_then(|cell| cell.grid()) else {
                changed = true;
                return false;
            };
            let kept = (
                level.x.min(grid.xs().saturating_sub(1)),
                level.y.min(grid.ys().saturating_sub(1)),
                level.selected.filter(|id| doc.cell(*id).is_some()),
            );
            changed |= kept != (level.x, level.y, level.selected);
            (level.x, level.y, level.selected) = kept;
            true
        });
        if self.zoom.is_some_and(|zoom| doc.cell(zoom).is_none()) {
            let stage = self.stage(doc);
            self.zoom = (stage != doc.root()).then_some(stage);
            changed = true;
        }
        self.line = self.zoom.map_or_else(Vec::new, |zoom| line_of(doc, zoom));
        changed
    }

    /// After cells moved from the document of `source` into this one's, keeping their IDs
    /// ([`crate::links::move_between`]): the folds of the cells `moved` go with them, from
    /// `source` to here. A copy does not take folds along: its cells are new ones
    /// (decision A88). Returns how many folds moved.
    pub fn take_folds(
        &mut self,
        source: &mut ViewState,
        moved: impl IntoIterator<Item = CellId>,
    ) -> usize {
        let mut count = 0usize;
        for cell in moved {
            if source.folded.remove(&cell) {
                self.folded.insert(cell);
                count = count.saturating_add(1);
            }
        }
        count
    }

    /// Forgets everything that refers to a cell that is not in `doc`, and what is
    /// remembered of a level that has no grid any more. A zoom cell that is not in `doc`
    /// gives way as [`ViewState::stage`] says: to the root, unless this view state
    /// followed the document until the cell went.
    pub fn retain_existing(&mut self, doc: &Document) {
        self.folded.retain(|id| doc.cell(*id).is_some());
        if self
            .selection
            .as_ref()
            .is_some_and(|s| doc.cell(s.cell).is_none())
        {
            self.selection = None;
        }
        self.settle(doc);
    }

    /// Folds or unfolds the grid of `cell`. Folding is view state, not content (decision
    /// A8): it is not a command, is not in the undo history and does not change the
    /// document. Returns whether anything changed; refused for a cell without a grid.
    pub fn set_folded(
        &mut self,
        doc: &Document,
        cell: CellId,
        folded: bool,
    ) -> Result<bool, Error> {
        match doc.cell(cell) {
            None => Err(Error::NotFound(format!("cell {cell}"))),
            Some(c) if c.grid().is_none() => {
                Err(Error::Refused(format!("cell {cell} has no grid to fold")))
            }
            Some(_) if folded => Ok(self.folded.insert(cell)),
            Some(_) => Ok(self.folded.remove(&cell)),
        }
    }

    /// Whether the grid of `cell` is folded.
    pub fn is_folded(&self, cell: CellId) -> bool {
        self.folded.contains(&cell)
    }

    /// Keeps the view state in step with the document after a command, an undo or a redo:
    /// call it with the document as it is now and the [`Changes`] that were returned.
    /// Whatever refers to a removed cell is dropped (fold, selection, what a level
    /// remembers, the cell selected in a level), and so are the fold and what is remembered
    /// of a cell that lost its grid. When the zoom cell is removed, the zoom moves to the
    /// nearest cell that was above it, still exists and has a grid, so that the screen
    /// goes up as few levels as it must; a zoom cell that only lost its grid stays, and
    /// [`ViewState::stage`] shows the level above it for as long as that lasts. Nothing is
    /// restored when an undo brings a cell or grid back: it returns unfolded, with nothing
    /// remembered, and the zoom stays where it went (decision A38). The cost depends on
    /// the size of the change and on [`MAX_LEVELS`], not on the size of the document.
    /// Returns whether the view state is another one now, so that whoever keeps its file
    /// knows when to write it.
    pub fn apply_changes(&mut self, doc: &Document, changes: &Changes) -> bool {
        let mut changed = false;
        for id in &changes.removed {
            changed |= self.folded.remove(id);
            if self.selection.as_ref().is_some_and(|s| s.cell == *id) {
                self.selection = None;
                changed = true;
            }
        }
        for id in &changes.grids {
            if doc.cell(*id).is_none_or(|c| c.grid().is_none()) {
                changed |= self.folded.remove(id);
            }
        }
        // In this order: the levels are settled whatever happened to the folds.
        self.settle(doc) || changed
    }

    /// The view state of the copy of the document that [`Document::with_fresh_ids`] made:
    /// the same folds, selection, zoom and levels, on the copy's cells.
    pub fn with_fresh_ids(&self, fresh: &FreshIds) -> ViewState {
        ViewState {
            doc: fresh.to,
            folded: self.folded.iter().filter_map(|c| fresh.cell(*c)).collect(),
            selection: self.selection.as_ref().and_then(|selection| {
                Some(Selection {
                    cell: fresh.cell(selection.cell)?,
                    ..selection.clone()
                })
            }),
            zoom: self.zoom.and_then(|cell| fresh.cell(cell)),
            levels: self
                .levels
                .iter()
                .filter_map(|level| {
                    Some(Level {
                        cell: fresh.cell(level.cell)?,
                        selected: level.selected.and_then(|cell| fresh.cell(cell)),
                        ..level.clone()
                    })
                })
                .collect(),
            extra: self.extra.clone(),
            minor: self.minor,
            line: self.line.iter().filter_map(|c| fresh.cell(*c)).collect(),
        }
    }

    /// Checks what [`ViewState::from_json`] checks, so that what is written can be read.
    fn validate(&self) -> Result<(), Error> {
        check_extra(&self.extra, VIEW_KEYS, "the view state")?;
        if let Some(selection) = &self.selection {
            check_extra(&selection.extra, SELECTION_KEYS, "the selection")?;
            if selection.cursor.is_some_and(|(start, end)| start > end) {
                return invalid("the cursor is not [start, end]");
            }
        }
        if self.folded.len() > MAX_CELLS {
            return Err(Error::Limit(format!("more than {MAX_CELLS} folded cells")));
        }
        if self.levels.len() > MAX_LEVELS {
            return Err(Error::Limit(format!("more than {MAX_LEVELS} levels")));
        }
        let mut cells = BTreeSet::new();
        for level in &self.levels {
            check_extra(&level.extra, LEVEL_KEYS, "a level")?;
            if !cells.insert(level.cell) {
                return invalid(format!("the level {} is there twice", level.cell));
            }
        }
        Ok(())
    }

    /// Writes the view state as text; the same state always gives the same bytes. Whatever
    /// it returns, [`ViewState::from_json`] reads back.
    pub fn to_json(&self) -> Result<String, Error> {
        self.validate()?;
        let mut out = String::from("{\n");
        let mut head = Fields::new(&mut out);
        head.string("format", VIEW_FORMAT_NAME);
        head.display("version", VIEW_FORMAT_VERSION);
        head.display(
            "minor",
            crate::json::minor_to_write(self.minor, VIEW_FORMAT_MINOR),
        );
        head.quoted("doc", self.doc);
        if let Some(selection) = &self.selection {
            out.push_str(",\n\"selection\": {");
            let mut f = Fields::new(&mut out);
            f.quoted("cell", selection.cell);
            f.display("xs", selection.xs);
            f.display("ys", selection.ys);
            if let Some((start, end)) = selection.cursor {
                f.display("cursor", format_args!("[{start}, {end}]"));
            }
            f.extra(&selection.extra)?;
            out.push('}');
        }
        if let Some(zoom) = self.zoom {
            out.push_str(&format!(",\n\"zoom\": \"{zoom}\""));
        }
        if !self.levels.is_empty() {
            out.push_str(",\n\"levels\": [");
            for (i, level) in self.levels.iter().enumerate() {
                out.push_str(if i == 0 { "\n {" } else { ",\n {" });
                let mut f = Fields::new(&mut out);
                f.quoted("cell", level.cell);
                if level.x != 0 {
                    f.display("x", level.x);
                }
                if level.y != 0 {
                    f.display("y", level.y);
                }
                if let Some(selected) = level.selected {
                    f.quoted("selected", selected);
                }
                f.extra(&level.extra)?;
                out.push('}');
            }
            out.push_str("\n]");
        }
        for (key, value) in &self.extra {
            out.push_str(",\n");
            crate::json::write_string(&mut out, key);
            out.push_str(": ");
            crate::json::write_value(&mut out, value)?;
        }
        if !self.folded.is_empty() {
            out.push_str(",\n\"folded\": [");
            for (i, id) in self.folded.iter().enumerate() {
                out.push_str(if i == 0 { "\n " } else { ",\n " });
                out.push_str(&format!("\"{id}\""));
            }
            out.push_str("\n]");
        }
        out.push_str("\n}\n");
        crate::format::check_size(&out)?;
        Ok(out)
    }

    /// Reads view-state text. Any input gives either a view state or an error.
    pub fn from_json(bytes: &[u8]) -> Result<ViewState, Error> {
        let place = "the view state";
        let mut map = into_object(parse(bytes, KIND)?, place)?;
        let minor = take_header(&mut map, KIND)?;
        let Some(doc) = take_id(&mut map, "doc", place)?.map(DocId) else {
            return invalid("the view state has no \"doc\"");
        };
        let mut view = ViewState::new(doc);
        // The number that will be written, so that a file of an older version, once read,
        // is the same view state as one made here.
        view.minor = crate::json::minor_to_write(minor, VIEW_FORMAT_MINOR);
        if let Some(value) = map.remove("selection") {
            let place = "the selection";
            let mut sel = into_object(value, place)?;
            let (Some(cell), Some(xs), Some(ys)) = (
                take_id(&mut sel, "cell", place)?.map(CellId),
                take_u32(&mut sel, "xs", place)?,
                take_u32(&mut sel, "ys", place)?,
            ) else {
                return invalid("the selection needs \"cell\", \"xs\" and \"ys\"");
            };
            let cursor = match take_array(&mut sel, "cursor", place)? {
                None => None,
                Some(pair) => {
                    let position =
                        |i: usize| pair.get(i)?.as_u64().and_then(|n| u32::try_from(n).ok());
                    match (pair.len(), position(0), position(1)) {
                        (2, Some(start), Some(end)) if start <= end => Some((start, end)),
                        _ => return invalid("the cursor is not [start, end]"),
                    }
                }
            };
            view.selection = Some(Selection {
                cell,
                xs,
                ys,
                cursor,
                extra: into_extra(sel),
            });
        }
        view.zoom = take_id(&mut map, "zoom", place)?.map(CellId);
        if let Some(levels) = take_array(&mut map, "levels", place)? {
            // How many there may be, and none twice, is checked with the rest at the end.
            let place = "a level";
            for value in levels {
                let mut level = into_object(value, place)?;
                let Some(cell) = take_id(&mut level, "cell", place)?.map(CellId) else {
                    return invalid("a level needs \"cell\"");
                };
                view.levels.push(Level {
                    cell,
                    x: take_u32(&mut level, "x", place)?.unwrap_or(0),
                    y: take_u32(&mut level, "y", place)?.unwrap_or(0),
                    selected: take_id(&mut level, "selected", place)?.map(CellId),
                    extra: into_extra(level),
                });
            }
        }
        if let Some(folded) = take_array(&mut map, "folded", place)? {
            if folded.len() > MAX_CELLS {
                return Err(Error::Limit(format!("more than {MAX_CELLS} folded cells")));
            }
            for value in folded {
                match value.as_str().and_then(CellId::parse) {
                    Some(id) => view.folded.insert(id),
                    None => return invalid("\"folded\" has an entry that is not an ID"),
                };
            }
        }
        view.extra = into_extra(map);
        view.validate()?;
        Ok(view)
    }
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
    use crate::id::{IdSource, SeededIds};
    use serde_json::json;

    fn sample() -> (Document, ViewState) {
        let mut ids = SeededIds::from_u64(3);
        let mut doc = Document::new(&mut ids);
        doc.add_grid(doc.root(), 2, 2, &mut ids).unwrap();
        let a = doc.child(doc.root(), 0, 0).unwrap();
        let b = doc.child(doc.root(), 1, 1).unwrap();
        let mut view = ViewState::new(doc.id());
        view.folded.insert(doc.root());
        view.folded.insert(b);
        view.selection = Some(Selection {
            cell: a,
            xs: 2,
            ys: 1,
            cursor: Some((1, 4)),
            extra: Extra::new(),
        });
        view.zoom = Some(b);
        (doc, view)
    }

    #[test]
    fn what_cannot_be_read_back_is_not_written() {
        let (_, good) = sample();
        let mut view = good.clone();
        view.selection.as_mut().unwrap().cursor = Some((4, 1));
        assert!(matches!(view.to_json(), Err(Error::Invalid(_))));
        let mut view = good.clone();
        view.extra.insert("zoom".into(), json!(1));
        assert!(matches!(view.to_json(), Err(Error::Invalid(_))));
        let mut view = good.clone();
        let extra = &mut view.selection.as_mut().unwrap().extra;
        extra.insert("cell".into(), json!(1));
        assert!(matches!(view.to_json(), Err(Error::Invalid(_))));
        // Keys may not repeat, as in a document.
        let text = good.to_json().unwrap();
        let twice = text.replacen("\"xs\": 2", "\"xs\": 2, \"xs\": 2", 1);
        assert_ne!(twice, text);
        assert!(matches!(
            ViewState::from_json(twice.as_bytes()),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn view_state_follows_a_copy_with_fresh_ids() {
        let (doc, mut view) = sample();
        let gone = CellId(SeededIds::from_u64(99).next_id());
        view.folded.insert(gone);
        view.extra.insert("scale".into(), json!(1.5));
        let (copy, fresh) = doc.with_fresh_ids(&mut SeededIds::from_u64(4)).unwrap();
        let moved = view.with_fresh_ids(&fresh);
        assert_eq!(moved.doc, copy.id());
        let a = copy.child(copy.root(), 0, 0).unwrap();
        let b = copy.child(copy.root(), 1, 1).unwrap();
        // The fold of a cell that the document does not have is dropped.
        assert_eq!(moved.folded, [copy.root(), b].into_iter().collect());
        let selection = moved.selection.as_ref().unwrap();
        assert_eq!((selection.cell, selection.xs, selection.ys), (a, 2, 1));
        assert_eq!(selection.cursor, Some((1, 4)));
        assert_eq!((moved.zoom, &moved.extra), (Some(b), &view.extra));
        // A selection or zoom on a cell that is gone is dropped too.
        view.selection.as_mut().unwrap().cell = gone;
        view.zoom = Some(gone);
        let moved = view.with_fresh_ids(&fresh);
        assert_eq!((moved.selection, moved.zoom), (None, None));
    }

    #[test]
    fn view_state_round_trips_and_is_deterministic() {
        let (_, mut view) = sample();
        view.extra.insert("scale".into(), json!(1.5));
        view.selection
            .as_mut()
            .unwrap()
            .extra
            .insert("later".into(), json!({"a": [1]}));
        let text = view.to_json().unwrap();
        let back = ViewState::from_json(text.as_bytes()).unwrap();
        assert_eq!(back, view);
        assert_eq!(back.to_json().unwrap(), text);
        assert!(text.starts_with("{\n\"format\": \"wombat-view\", \"version\": 1, \"minor\": 1, "));
    }

    #[test]
    fn empty_view_state_is_small() {
        let view = ViewState::new(DocId(crate::Id::from_u128(1)));
        let text = view.to_json().unwrap();
        assert_eq!(
            text,
            "{\n\"format\": \"wombat-view\", \"version\": 1, \"minor\": 1, \
             \"doc\": \"00000000-0000-0000-0000-000000000001\"\n}\n"
        );
        assert_eq!(ViewState::from_json(text.as_bytes()).unwrap(), view);
    }

    #[test]
    fn stale_references_are_dropped() {
        let (doc, mut view) = sample();
        let mut ids = SeededIds::from_u64(99);
        let gone = CellId(ids.next_id());
        view.folded.insert(gone);
        view.retain_existing(&doc);
        assert_eq!(view.folded.len(), 2);
        assert!(view.selection.is_some() && view.zoom.is_some());
        view.zoom = Some(gone);
        view.selection.as_mut().unwrap().cell = gone;
        view.retain_existing(&doc);
        assert!(view.selection.is_none() && view.zoom.is_none());
    }

    #[test]
    fn bad_view_state_is_refused() {
        let (_, view) = sample();
        let good: serde_json::Value = serde_json::from_str(&view.to_json().unwrap()).unwrap();
        let with = |key: &str, value: serde_json::Value| {
            let mut v = good.clone();
            v[key] = value;
            ViewState::from_json(v.to_string().as_bytes())
        };
        assert!(matches!(
            with("format", json!("wombat")),
            Err(Error::WrongFormat(_))
        ));
        assert!(matches!(
            with("version", json!(2)),
            Err(Error::NewerVersion { found: 2, .. })
        ));
        assert!(matches!(with("doc", json!(5)), Err(Error::Invalid(_))));
        assert!(matches!(
            with("folded", json!(["x"])),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(with("zoom", json!("x")), Err(Error::Invalid(_))));
        assert!(matches!(
            with("selection", json!({"cell": 1})),
            Err(Error::Invalid(_))
        ));
        let mut bad_cursor = good.clone();
        bad_cursor["selection"]["cursor"] = json!([4, 1]);
        assert!(ViewState::from_json(bad_cursor.to_string().as_bytes()).is_err());
        assert!(with("minor", json!(7)).is_ok());
        assert!(matches!(ViewState::from_json(b"[1"), Err(Error::Syntax(_))));
        let mut view = view;
        view.extra.insert("doc".into(), json!(1));
        assert!(view.to_json().is_err());
    }
}
