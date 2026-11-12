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

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::error::{invalid, refused, Error};
use crate::id::{CellId, DocId};
use crate::limits::MAX_DEPTH;
use crate::model::{validate_cell, Cell, Document, Extra, LinkTarget};

use super::command::{
    cell, check_room, depth, descendants, grid, has_content, new_cell, set_runs_links, sync_cell,
    Fresh,
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

    /// The same cells as a document of their own: the top cell is its root, and its ID is
    /// that of the document the cells came from, so that the links inside read as they did
    /// there. With [`Clip::from_document`] this is how a clip leaves the program and comes
    /// back (as the text that [`crate::format::to_string`] writes, say): no other way in
    /// exists, so that a clip always holds what [`Clip::of`] would have made.
    pub fn as_document(&self) -> Result<Document, Error> {
        let Some(top) = self.top() else {
            return invalid("a clip without cells");
        };
        let mut cells: HashMap<CellId, Cell> =
            self.cells.iter().map(|c| (c.id, c.clone())).collect();
        if let Some(root) = cells.get_mut(&top) {
            root.parent = None;
        }
        let doc = Document {
            id: self.doc,
            minor: 0,
            root: top,
            cells,
            tags: BTreeMap::new(),
            extra: Extra::new(),
        };
        // `Clip::of` checks cell by cell. A document under construction is not checked at
        // all (`crate::construct`), and a clip of one can break the rules that only a
        // whole document has: a cell with the ID of the document, a link that names its
        // own document.
        doc.validate()?;
        Ok(doc)
    }

    /// The clip that [`Clip::as_document`] made `doc` of: the root of `doc` and everything
    /// inside it, as cells that came from the document whose ID `doc` has, where the top
    /// cell was in the grid of `parent` (`None`: it was the root). The source is said in
    /// one place only, the ID of `doc`, so that it cannot be said twice and differently.
    /// Checked as [`Clip::of`] checks, and refused if a cell or `parent` has the ID of the
    /// document, which no cell of a document can have, or if `parent` is among the cells.
    pub fn from_document(doc: &Document, parent: Option<CellId>) -> Result<Clip, Error> {
        let mut clip = Clip::of(doc, doc.root())?;
        let source = clip.doc;
        if clip.cell_ids().any(|id| id.0 == source.0) || parent.is_some_and(|p| p.0 == source.0) {
            return invalid(format!("a cell has the ID of the document {source}"));
        }
        if parent.is_some_and(|p| doc.cell(p).is_some()) {
            return invalid("the cell that the top cell was in is among the cells");
        }
        if let Some(top) = clip.cells.first_mut() {
            top.parent = parent;
        }
        Ok(clip)
    }

    /// The document the cells were copied from.
    pub fn source(&self) -> DocId {
        self.doc
    }

    /// The cell whose grid the top cell was in, or `None` if it was the root.
    pub fn parent(&self) -> Option<CellId> {
        self.cells.first().and_then(Cell::parent)
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

    /// The texts of the cells, the top cell first.
    pub fn texts(&self) -> impl Iterator<Item = &str> + '_ {
        self.cells.iter().map(|c| c.text.as_str())
    }

    /// The distinct names of the image files that the cells show, sorted.
    pub fn image_names(&self) -> Vec<String> {
        let names: std::collections::BTreeSet<&str> = self
            .cells
            .iter()
            .filter_map(|c| c.image.as_ref())
            .map(|image| image.file.as_str())
            .collect();
        names.into_iter().map(String::from).collect()
    }

    /// Makes every cell that shows the image file `from` show the file `to` instead: for
    /// an image that has another name in the image folder of the document the cells go
    /// to (see [`crate::storage::carry_images`]). Fails if `to` is no name for an image.
    pub fn rename_image(&mut self, from: &str, to: &str) -> Result<(), Error> {
        if !crate::model::is_valid_image_name(to) {
            return invalid(format!("{to:?} is not a valid image file name"));
        }
        for image in self.cells.iter_mut().filter_map(|c| c.image.as_mut()) {
            if image.file == from {
                image.file = to.to_string();
            }
        }
        Ok(())
    }

    /// Whether these are the cell `top` of `doc` and the cells inside it as they are now,
    /// but for the names of image files.
    pub(crate) fn is_of(&self, doc: &Document, top: CellId) -> bool {
        let Ok(mut now) = Clip::of(doc, top) else {
            return false;
        };
        let mut own = self.clone();
        for image in own.cells.iter_mut().chain(&mut now.cells) {
            if let Some(image) = image.image.as_mut() {
                image.file.clear();
            }
        }
        own == now
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
        // An ID that the history of this document holds is that of this very cell, which
        // left before and comes back (IDs are never given twice, A5 and A48). The unit
        // that holds it is older than this command, so undo reaches it only after this
        // command is undone and the cell is gone again: never twice (decision A84).
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

/// `Command::RetargetLinks`: the links to `cells` that name `from` name `to`, in place.
pub(super) fn retarget(
    tx: &mut Tx,
    cells: &[CellId],
    from: Option<DocId>,
    to: Option<DocId>,
) -> Result<(), Error> {
    let doc = tx.doc();
    // A link to a cell of this document names no document (A47).
    let own = doc.id();
    let (from, to) = (from.filter(|d| *d != own), to.filter(|d| *d != own));
    if from == to {
        return Ok(());
    }
    let wanted: HashSet<CellId> = cells.iter().copied().collect();
    let hit = |target: &LinkTarget| match target {
        LinkTarget::Cell { doc, cell } => *doc == from && wanted.contains(cell),
        _ => false,
    };
    let mut changed = Vec::new();
    for id in doc.cells_in_order() {
        let found = cell(doc, id)?;
        if !found.links.iter().any(|link| hit(&link.target)) {
            continue;
        }
        let mut links = found.links.clone();
        for link in &mut links {
            if hit(&link.target) {
                if let LinkTarget::Cell { doc, .. } = &mut link.target {
                    *doc = to;
                }
            }
        }
        changed.push((id, links));
    }
    for (id, links) in changed {
        set_runs_links(tx, id, None, Some(links))?;
    }
    Ok(())
}
