//! Cells that go from one document to another (decisions A50 and A51).
//!
//! A [`Clip`] is a cell with everything inside it, taken out of a document as a plain value.
//! Three commands use it: `PasteCells` makes a cell a copy of it (new IDs inside, as
//! `CopyCell` does within a document), `AdoptCells` puts the very cells into another
//! document, IDs and all, and `ReleaseCell` is the other half of that move: the cell leaves
//! its document and an empty one stands in its place. Each is an ordinary command of one
//! document, with ordinary steps, undo and change report.
//!
//! Links in the cells that arrive are rewritten so that they still point at the same
//! cells: see [`relink`].

use std::collections::{HashMap, HashSet};

use crate::error::{invalid, refused, Error};
use crate::id::{CellId, DocId};
use crate::limits::MAX_DEPTH;
use crate::model::{validate_cell, Cell, Document, LinkTarget};

use super::command::{
    cell, check_room, depth, descendants, grid, has_content, new_cell, sync_cell, Fresh,
};
use super::step::Step;
use super::Tx;

/// A cell with everything inside it, copied out of a document: what a clipboard holds. It
/// remembers which document it came from, so that its links can be kept pointing at the
/// same cells wherever it is pasted. Made by [`Clip::of`], used by
/// [`super::Command::PasteCells`] and [`super::Command::AdoptCells`].
#[derive(Clone, PartialEq, Debug)]
pub struct Clip {
    doc: DocId,
    /// The top cell first, then the cells inside it in document order.
    cells: Vec<Cell>,
    /// How many levels of grids are inside the top cell.
    height: u32,
}

impl Clip {
    /// Copies `cell` and everything inside it out of `doc`, which is not changed. Fails if
    /// the cell does not exist or if what is inside it breaks the model's rules (which
    /// cannot happen for a document that was loaded or is being edited).
    pub fn of(doc: &Document, cell: CellId) -> Result<Clip, Error> {
        let mut cells: Vec<Cell> = Vec::new();
        let mut seen = HashSet::new();
        let mut height = 0;
        let mut stack = vec![(cell, None::<CellId>, 0u32)];
        while let Some((id, parent, level)) = stack.pop() {
            let Some(found) = doc.cell(id) else {
                return Err(Error::NotFound(format!("cell {id}")));
            };
            if !seen.insert(id) {
                return invalid(format!("cell {id} is in a grid more than once"));
            }
            validate_cell(found)?;
            let mut copy = found.clone();
            // The top cell keeps the parent it had, to say where it was; the others must
            // be the children of the cells that list them.
            if let Some(parent) = parent {
                copy.parent = Some(parent);
            }
            height = height.max(level);
            if let Some(grid) = found.grid() {
                let below = level.saturating_add(1);
                stack.extend(grid.cells().iter().rev().map(|c| (*c, Some(id), below)));
            }
            cells.push(copy);
        }
        Ok(Clip {
            doc: doc.id(),
            cells,
            height,
        })
    }

    /// The document the cells were copied from.
    pub fn source(&self) -> DocId {
        self.doc
    }

    /// The ID that the top cell had in the document it came from.
    pub fn top(&self) -> Option<CellId> {
        self.cells.first().map(Cell::id)
    }

    /// The IDs that the cells had in the document they came from, the top cell first.
    pub fn cell_ids(&self) -> impl Iterator<Item = CellId> + '_ {
        self.cells.iter().map(Cell::id)
    }

    /// The number of cells, the top cell included.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

/// Rewrites the links of a cell that came from the document `from` for the document `own`
/// it arrives in, so that each still points at the cell it pointed at:
///
/// - a link to a cell that arrives with it follows that cell (to its new ID, if it got one);
/// - a link to another cell of the document it came from now names that document;
/// - a link to a cell of the document it arrives in no longer names a document (A47);
/// - links to cells of other documents, and external links, stay as they are.
fn relink(c: &mut Cell, from: DocId, own: DocId, arriving: impl Fn(CellId) -> Option<CellId>) {
    for link in &mut c.links {
        let LinkTarget::Cell { doc, cell } = &mut link.target else {
            continue;
        };
        match *doc {
            None => match arriving(*cell) {
                Some(new) => *cell = new,
                None if from != own => *doc = Some(from),
                None => {}
            },
            Some(named) if named == own => *doc = None,
            Some(_) => {}
        }
    }
}

fn too_deep() -> Error {
    Error::Limit(format!("grids nested deeper than {MAX_DEPTH}"))
}

/// `Command::PasteCells`: as `copy_cell`, from a clip.
pub(super) fn paste(tx: &mut Tx, to: CellId, clip: Clip, fresh: &mut Fresh) -> Result<(), Error> {
    let doc = tx.doc();
    cell(doc, to)?;
    if to == doc.root() {
        return refused("the root cell cannot be pasted over");
    }
    let replaced = descendants(doc, to);
    let more = clip.cells.len().saturating_sub(1);
    check_room(tx, more.saturating_sub(replaced.len()))?;
    if depth(doc, to)?.saturating_add(clip.height) > MAX_DEPTH {
        return Err(too_deep());
    }
    let own = doc.id();
    // Which cell of the clip becomes which cell of the document.
    let mut renamed: HashMap<CellId, CellId> = HashMap::with_capacity(clip.cells.len());
    for (n, c) in clip.cells.iter().enumerate() {
        let new = if n == 0 { to } else { fresh.next(doc)? };
        renamed.insert(c.id, new);
    }
    let rename = |id: CellId| renamed.get(&id).copied();
    let mut cells = clip.cells;
    for c in &mut cells {
        relink(c, clip.doc, own, rename);
        c.parent = c.parent.and_then(rename);
        if let Some(new) = rename(c.id) {
            c.id = new;
        }
        if let Some(grid) = &mut c.grid {
            for child in &mut grid.cells {
                if let Some(new) = rename(*child) {
                    *child = new;
                }
            }
        }
    }
    let mut inside = cells.split_off(1.min(cells.len()));
    let Some(top) = cells.pop() else {
        return refused("nothing to paste");
    };
    let had_grid = cell(tx.doc(), to)?.grid().is_some();
    sync_cell(tx, to, &top)?;
    if top.grid.is_some() {
        tx.step(Step::AddCells(std::mem::take(&mut inside)))?;
    }
    if had_grid || top.grid.is_some() {
        tx.step(Step::SetGrid {
            cell: to,
            grid: top.grid,
        })?;
        tx.step(Step::RemoveCells(replaced))?;
    }
    Ok(())
}

/// The grid of `parent` with `new` in the place of `old`.
fn with_replaced(
    doc: &Document,
    parent: CellId,
    old: CellId,
    new: CellId,
) -> Result<crate::model::Grid, Error> {
    let mut g = grid(doc, parent)?.clone();
    match g.cells.iter_mut().find(|c| **c == old) {
        Some(slot) => *slot = new,
        None => return invalid(format!("cell {old} is not in the grid of its parent")),
    }
    Ok(g)
}

/// `Command::AdoptCells`: the cells of the clip come into the document as they are.
pub(super) fn adopt(tx: &mut Tx, to: CellId, clip: Clip) -> Result<(), Error> {
    let doc = tx.doc();
    let own = doc.id();
    let place = cell(doc, to)?;
    if clip.doc == own {
        return refused("the cells are of this document; move them within it with MoveCell");
    }
    let Some(parent) = place.parent() else {
        return refused("the root cell cannot be replaced");
    };
    if has_content(place) {
        return refused(format!("cell {to} is not empty"));
    }
    for c in &clip.cells {
        let id = c.id;
        if id.0 == own.0 || doc.cell(id).is_some() {
            return refused(format!("the ID {id} is already in use in this document"));
        }
        if tx.held.contains(&id) {
            return refused(format!(
                "the ID {id} is that of a cell that undo or redo can bring back"
            ));
        }
    }
    check_room(tx, clip.cells.len().saturating_sub(1))?;
    if depth(doc, to)?.saturating_add(clip.height) > MAX_DEPTH {
        return Err(too_deep());
    }
    let Some(top) = clip.top() else {
        return refused("nothing to move in");
    };
    let new_grid = with_replaced(doc, parent, to, top)?;
    let arriving: HashSet<CellId> = clip.cell_ids().collect();
    let mut cells = clip.cells;
    for c in &mut cells {
        relink(c, clip.doc, own, |id| arriving.get(&id).copied());
    }
    if let Some(first) = cells.first_mut() {
        first.parent = Some(parent);
    }
    tx.step(Step::AddCells(cells))?;
    tx.step(Step::SetGrid {
        cell: parent,
        grid: Some(new_grid),
    })?;
    tx.step(Step::RemoveCells(vec![to]))
}

/// `Command::ReleaseCell`: the cell and all in it go, a new empty cell takes its place.
pub(super) fn release(
    tx: &mut Tx,
    id: CellId,
    now: Option<i64>,
    fresh: &mut Fresh,
) -> Result<(), Error> {
    let doc = tx.doc();
    let old = cell(doc, id)?;
    let Some(parent) = old.parent() else {
        return refused("the root cell cannot leave its document");
    };
    let new = fresh.next(doc)?;
    let empty = new_cell(new, parent, old, now);
    let new_grid = with_replaced(doc, parent, id, new)?;
    let mut gone = descendants(doc, id);
    gone.push(id);
    tx.step(Step::AddCells(vec![empty]))?;
    tx.step(Step::SetGrid {
        cell: parent,
        grid: Some(new_grid),
    })?;
    tx.step(Step::RemoveCells(gone))
}
