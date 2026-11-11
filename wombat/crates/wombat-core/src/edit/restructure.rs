//! The restructuring commands (milestone 3b): TreeSheets' sort, transpose, hierarchify,
//! flatten, hierarchy swap, wrap, collapse and split, and the actions that walk a selection
//! (tags, relative sizes and widths, checkboxes).
//!
//! Each follows what TreeSheets does to the document (`Document::Action` in
//! `src/document.h`, `src/grid.h`, `src/selection.h`), and takes as parameters what
//! TreeSheets takes from the selection. They are made of the same steps as every other
//! command. The three that rearrange a whole subtree (hierarchify, flatten, hierarchy swap)
//! work on a private copy of that subtree, a [`Scratch`], in the way TreeSheets works on its
//! own tree, and then turn the difference into steps.
//!
//! Which cell keeps its ID is part of each command's meaning, because links point at IDs
//! (milestone 4): a cell that survives keeps its ID wherever it goes; see each function.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use crate::error::{refused, Error};
use crate::id::CellId;
use crate::limits::{MAX_DEPTH, MAX_TEXT_CODE_POINTS};
use crate::model::{code_points, Cell, CellKind, Document, Extra, Grid, Tag};
use crate::text;

use super::command::{
    cell, check_range, check_room, clear_cells, clear_own, col_width_of, delete_cells, depth,
    descendants, fresh_grid, grid, has_content, height, len32, new_cell, replace_text, set,
    set_tag, stamp, sync_cell, Fresh,
};
use super::step::{Field, Step};
use super::{Tx, MAX_REL_SIZE, MIN_COL_WIDTH, MIN_REL_SIZE};

fn too_deep() -> Error {
    Error::Limit(format!("grids nested deeper than {MAX_DEPTH}"))
}

fn broken(what: &str) -> Error {
    Error::Refused(format!("the document is not as expected: {what}"))
}

/// Checks that the block of `xs` by `ys` cells at `x`, `y` is inside the grid and not empty.
fn check_block(id: CellId, g: &Grid, x: u32, y: u32, xs: u32, ys: u32) -> Result<(), Error> {
    let inside = |from: u32, size: u32, all: u32| {
        size > 0 && from.checked_add(size).is_some_and(|end| end <= all)
    };
    if !inside(x, xs, g.xs()) || !inside(y, ys, g.ys()) {
        return refused(format!(
            "cell {id}: no cells {xs} by {ys} at {x},{y} in a grid of {} by {}",
            g.xs(),
            g.ys()
        ));
    }
    Ok(())
}

/// The cells of a block, row by row.
fn block(g: &Grid, x: u32, y: u32, xs: u32, ys: u32) -> Vec<CellId> {
    let mut out = Vec::new();
    for cy in y..y.saturating_add(ys) {
        for cx in x..x.saturating_add(xs) {
            out.extend(g.at(cx, cy));
        }
    }
    out
}

// ---- Sort ---------------------------------------------------------------------------------

/// One character as TreeSheets compares it when sorting (`wxString::CmpNoCase`): its
/// lower-case form where that is one character.
fn fold(c: char) -> char {
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(one), None) => one,
        _ => c,
    }
}

fn cmp_no_case(a: &str, b: &str) -> Ordering {
    a.chars().map(fold).cmp(b.chars().map(fold))
}

/// TreeSheets' `Grid::Sort`: reorders `count` whole rows from `row` on by the text of their
/// cells in `column`, and where those are equal by the following columns in turn (wrapping
/// round to the first). Texts compare as text without regard to case, so "10" sorts before
/// "9". Rows that are equal in every column keep their order, also when descending. Every
/// cell keeps its ID.
pub(super) fn sort_rows(
    tx: &mut Tx,
    id: CellId,
    column: u32,
    row: u32,
    count: u32,
    descending: bool,
) -> Result<(), Error> {
    let doc = tx.doc();
    let g = grid(doc, id)?;
    if column >= g.xs() || row.checked_add(count).is_none_or(|end| end > g.ys()) {
        return refused(format!(
            "cell {id}: cannot sort {count} rows from {row} by column {column} in a grid of {} \
             by {}",
            g.xs(),
            g.ys()
        ));
    }
    if count < 2 {
        return Ok(());
    }
    let width = g.xs() as usize;
    let (from, to) = (row as usize * width, (row + count) as usize * width);
    let rows: Vec<&[CellId]> = g
        .cells()
        .get(from..to)
        .unwrap_or_default()
        .chunks(width)
        .collect();
    // The texts of each row in the order they are compared.
    let keys: Vec<Vec<&str>> = rows
        .iter()
        .map(|row| {
            (0..width)
                .filter_map(|k| row.get((k + column as usize) % width))
                .map(|c| doc.cell(*c).map_or("", |c| c.text.as_str()))
                .collect()
        })
        .collect();
    let compare = |a: &usize, b: &usize| {
        let (Some(ka), Some(kb)) = (keys.get(*a), keys.get(*b)) else {
            return Ordering::Equal;
        };
        let order = ka
            .iter()
            .zip(kb)
            .map(|(x, y)| cmp_no_case(x, y))
            .find(|o| o.is_ne())
            .unwrap_or(Ordering::Equal);
        if descending {
            order.reverse()
        } else {
            order
        }
    };
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(compare);
    let mut cells = Vec::with_capacity(g.cells().len());
    cells.extend_from_slice(g.cells().get(..from).unwrap_or_default());
    for n in order {
        cells.extend_from_slice(rows.get(n).copied().unwrap_or_default());
    }
    cells.extend_from_slice(g.cells().get(to..).unwrap_or_default());
    if cells == g.cells() {
        return Ok(());
    }
    let mut sorted = g.clone();
    sorted.cells = cells;
    tx.step(Step::SetGrid {
        cell: id,
        grid: Some(sorted),
    })
}

// ---- Transpose ----------------------------------------------------------------------------

/// TreeSheets' `Grid::Transpose` for the grid of every listed cell that has one: rows
/// become columns. As there, the columns keep the widths of the first columns, and columns
/// beyond those take the width of the last column the grid had. Every cell keeps its ID.
pub(super) fn transpose(tx: &mut Tx, cells: &[CellId]) -> Result<(), Error> {
    for id in cells {
        cell(tx.doc(), *id)?;
    }
    let mut seen = HashSet::new();
    for id in cells {
        let Some(g) = cell(tx.doc(), *id)?.grid() else {
            continue;
        };
        if !seen.insert(*id) {
            continue;
        }
        let (xs, ys) = (g.xs() as usize, g.ys() as usize);
        let mut turned = g.clone();
        turned.cells.clear();
        for x in 0..xs {
            for y in 0..ys {
                turned.cells.extend(g.cells().get(y * xs + x));
            }
        }
        turned.xs = g.ys();
        turned.ys = g.xs();
        let last = g
            .col_widths
            .last()
            .copied()
            .unwrap_or(Grid::DEFAULT_COL_WIDTH);
        turned.col_widths.resize(ys, last);
        if turned != *g {
            tx.step(Step::SetGrid {
                cell: *id,
                grid: Some(turned),
            })?;
        }
    }
    Ok(())
}

// ---- A private copy of a subtree ------------------------------------------------------------

/// A copy of one cell and everything inside it, which a command rearranges freely: cells
/// change grids, are added, changed and left out. [`Scratch::commit`] then makes the
/// document equal to the copy with steps.
struct Scratch {
    cells: HashMap<CellId, Cell>,
    /// The cells that were copied from the document.
    loaded: Vec<CellId>,
}

impl Scratch {
    fn load(doc: &Document, top: CellId) -> Result<Scratch, Error> {
        let mut loaded = vec![top];
        loaded.extend(descendants(doc, top));
        let mut cells = HashMap::with_capacity(loaded.len());
        for id in &loaded {
            cells.insert(*id, cell(doc, *id)?.clone());
        }
        Ok(Scratch { cells, loaded })
    }

    fn get(&self, id: CellId) -> Result<&Cell, Error> {
        self.cells
            .get(&id)
            .ok_or_else(|| broken("a cell is missing"))
    }

    fn get_mut(&mut self, id: CellId) -> Result<&mut Cell, Error> {
        self.cells
            .get_mut(&id)
            .ok_or_else(|| broken("a cell is missing"))
    }

    fn insert(&mut self, cell: Cell) {
        self.cells.insert(cell.id, cell);
    }

    /// The cells of the grid of `id`; none if it has no grid.
    fn kids(&self, id: CellId) -> Vec<CellId> {
        let grid = self.cells.get(&id).and_then(|c| c.grid.as_ref());
        grid.map(|g| g.cells.clone()).unwrap_or_default()
    }

    /// Makes `id` the parent of the cells of its grid.
    fn adopt(&mut self, id: CellId) -> Result<(), Error> {
        for kid in self.kids(id) {
            self.get_mut(kid)?.parent = Some(id);
        }
        Ok(())
    }

    /// `top` and the cells inside it as the copy has them now, in document order.
    fn reachable(&self, top: CellId) -> Vec<CellId> {
        let mut order = Vec::new();
        let mut stack = vec![top];
        while let Some(id) = stack.pop() {
            // More cells than exist would be a cell listed twice.
            if order.len() > self.cells.len() {
                break;
            }
            order.push(id);
            stack.extend(self.kids(id).iter().rev());
        }
        order
    }

    /// The width of the column that `id` is in (see `col_width_of`); its parent may be in
    /// the copy or, for the top of the copy, only in the document.
    fn col_width_of(&self, doc: &Document, id: CellId) -> u32 {
        let width = || {
            let parent = self.cells.get(&id)?.parent?;
            let grid = self
                .cells
                .get(&parent)
                .or_else(|| doc.cell(parent))?
                .grid()?;
            let index = grid.cells().iter().position(|c| *c == id)?;
            grid.col_widths
                .get(index % grid.xs().max(1) as usize)
                .copied()
        };
        width().unwrap_or(Grid::DEFAULT_COL_WIDTH)
    }

    /// Gives the cell `from` the ID `to`, which no cell of the copy's tree has.
    fn rename(&mut self, from: CellId, to: CellId) -> Result<(), Error> {
        let mut moved = self
            .cells
            .remove(&from)
            .ok_or_else(|| broken("a cell is missing"))?;
        moved.id = to;
        if let Some(grid) = moved
            .parent
            .and_then(|p| self.cells.get_mut(&p))
            .and_then(|p| p.grid.as_mut())
        {
            for slot in &mut grid.cells {
                if *slot == from {
                    *slot = to;
                }
            }
        }
        self.insert(moved);
        self.adopt(to)
    }

    /// Makes the document's `top` and everything inside it equal to the copy: adds the
    /// cells that are new, changes those that differ, gives every cell the grid it has in
    /// the copy, and removes the cells of the document that the copy no longer reaches.
    fn commit(mut self, tx: &mut Tx, top: CellId) -> Result<(), Error> {
        let order = self.reachable(top);
        let kept: HashSet<CellId> = order.iter().copied().collect();
        if kept.len() != order.len() {
            return Err(broken("a cell would be in two grids"));
        }
        // Parents follow from the grids; and how deep does the tree go now?
        let mut levels: HashMap<CellId, u32> = HashMap::with_capacity(order.len());
        levels.insert(top, 0);
        let mut deepest = 0u32;
        for id in &order {
            let level = levels.get(id).copied().unwrap_or(0);
            deepest = deepest.max(level);
            for kid in self.kids(*id) {
                self.get_mut(kid)?.parent = Some(*id);
                levels.insert(kid, level.saturating_add(1));
            }
        }
        let doc = tx.doc();
        if depth(doc, top)?.saturating_add(deepest) > MAX_DEPTH {
            return Err(too_deep());
        }
        let removed: Vec<CellId> = self
            .loaded
            .iter()
            .copied()
            .filter(|id| !kept.contains(id))
            .collect();
        let mut added = Vec::new();
        for id in &order {
            if doc.cell(*id).is_none() {
                // Without its grid: a grid makes its cells children only when it is set.
                let mut new = self.get(*id)?.clone();
                new.grid = None;
                added.push(new);
            }
        }
        check_room(doc, added.len().saturating_sub(removed.len()))?;
        let new: HashSet<CellId> = added.iter().map(Cell::id).collect();
        if !added.is_empty() {
            tx.step(Step::AddCells(added))?;
        }
        for id in &order {
            if !new.contains(id) {
                sync_cell(tx, *id, self.get(*id)?)?;
            }
        }
        for id in &order {
            let want = self.get_mut(*id)?.grid.take();
            let have = cell(tx.doc(), *id)?.grid().cloned();
            match (have, want) {
                (None, None) => {}
                (Some(have), Some(want))
                    if (have.xs, have.ys) == (want.xs, want.ys) && have.cells == want.cells =>
                {
                    // The same cells in the same places: only properties can differ.
                    if have.border_color != want.border_color {
                        set(tx, *id, Field::BorderColor(want.border_color))?;
                    }
                    if have.outer_spacing != want.outer_spacing {
                        set(tx, *id, Field::OuterSpacing(want.outer_spacing))?;
                    }
                    if have.vertical != want.vertical {
                        set(tx, *id, Field::Vertical(want.vertical))?;
                    }
                    if have.col_widths != want.col_widths {
                        set(tx, *id, Field::ColWidths(want.col_widths))?;
                    }
                }
                (_, want) => tx.step(Step::SetGrid {
                    cell: *id,
                    grid: want,
                })?,
            }
        }
        if !removed.is_empty() {
            tx.step(Step::RemoveCells(removed))?;
        }
        Ok(())
    }
}

// ---- Hierarchify --------------------------------------------------------------------------

/// TreeSheets' `Grid::Hierarchify`: turns a table (a grid none of whose cells has a grid)
/// into a hierarchy. The rows are grouped by the text of their first cell: the first row
/// with a text keeps its first cell, which gets a grid holding the rest of that row and of
/// every later row that starts with the same text, one row each; the first cells of those
/// later rows are dropped. The table is left with one column, and the same is done to
/// every new grid that has more than one column.
///
/// IDs: every cell keeps its ID except the dropped first cells of repeated rows; no cell is
/// created.
pub(super) fn hierarchify(tx: &mut Tx, id: CellId) -> Result<(), Error> {
    let doc = tx.doc();
    let table = grid(doc, id)?;
    let nested = |c: &CellId| doc.cell(*c).is_some_and(|c| c.grid().is_some());
    if table.cells().iter().any(nested) {
        return refused(format!(
            "cell {id}: the grid is not a table: its cells must not have grids of their own"
        ));
    }
    if depth(doc, id)?.saturating_add(table.xs()) > MAX_DEPTH {
        return Err(too_deep());
    }
    let mut s = Scratch::load(doc, id)?;
    let mut work = vec![id];
    while let Some(owner) = work.pop() {
        let Some(g) = s.get(owner)?.grid.clone() else {
            continue;
        };
        let width = g.xs as usize;
        let rest_widths = g.col_widths.get(1..).unwrap_or_default();
        let mut kept: Vec<CellId> = Vec::new();
        let mut first_with: HashMap<String, CellId> = HashMap::new();
        for row in g.cells.chunks(width.max(1)) {
            let Some((first, rest)) = row.split_first() else {
                continue;
            };
            let text = s.get(*first)?.text.clone();
            match first_with.get(&text) {
                Some(earlier) => {
                    if let Some(sub) = s.get_mut(*earlier)?.grid.as_mut() {
                        sub.cells.extend_from_slice(rest);
                        sub.ys = sub.ys.saturating_add(1);
                    }
                }
                None => {
                    kept.push(*first);
                    first_with.insert(text, *first);
                    if !rest.is_empty() {
                        let mut sub = Grid::new(g.xs.saturating_sub(1), 1, rest.to_vec());
                        sub.col_widths = rest_widths.to_vec();
                        sub.vertical = g.vertical;
                        s.get_mut(*first)?.grid = Some(sub);
                    }
                }
            }
        }
        let mut column = g.clone();
        column.xs = 1;
        column.ys = u32::try_from(kept.len()).map_err(|_| broken("too many rows"))?;
        column.col_widths.truncate(1);
        if width > 2 {
            work.extend(&kept);
        }
        column.cells = kept;
        s.get_mut(owner)?.grid = Some(column);
    }
    s.commit(tx, id)
}

// ---- Flatten ------------------------------------------------------------------------------

/// TreeSheets' `Grid::Flatten`: replaces the grid of a cell by a flat table with one row
/// per cell inside it that has no grid (a leaf), in document order, and one column per
/// level: the leaf stands in the column of its level, and to its left stand the cells it
/// was inside of, outermost first. As in TreeSheets a cell of the table has only the text
/// of the cell it comes from, with its styles, links, relative size, image and edit time;
/// colours, note, alignments and kind are not carried over, cells right of a leaf are
/// empty, and the table has the default column widths, border and spacing.
///
/// IDs: every cell that was inside keeps its ID: a leaf on its row, a cell that had a grid
/// on the first row it stands on. Its repeats on the following rows and the empty cells are
/// new.
pub(super) fn flatten(
    tx: &mut Tx,
    id: CellId,
    now: Option<i64>,
    fresh: &mut Fresh,
) -> Result<(), Error> {
    let doc = tx.doc();
    let old = grid(doc, id)?;
    // First the size, so that a result beyond the limits costs nothing.
    let (mut leaves, mut levels) = (0usize, 0u32);
    let mut stack: Vec<(CellId, u32)> = old.cells().iter().map(|c| (*c, 1)).collect();
    let mut inside = 0usize;
    while let Some((next, level)) = stack.pop() {
        inside = inside.saturating_add(1);
        levels = levels.max(level);
        match doc.cell(next).and_then(Cell::grid) {
            Some(g) => stack.extend(g.cells().iter().map(|c| (*c, level.saturating_add(1)))),
            None => leaves = leaves.saturating_add(1),
        }
    }
    let total = leaves
        .checked_mul(levels as usize)
        .ok_or_else(|| Error::Limit("a table that large".into()))?;
    check_room(doc, total.saturating_sub(inside))?;
    let rows = u32::try_from(leaves).map_err(|_| Error::Limit("a table that large".into()))?;

    let mut s = Scratch::load(doc, id)?;
    let mut used: HashSet<CellId> = HashSet::with_capacity(inside);
    let mut cells = Vec::with_capacity(total);
    // Walks the tree with the path of cells from the top grid down to where it is.
    let mut path: Vec<CellId> = Vec::new();
    let mut walk: Vec<(CellId, usize)> = vec![(id, 0)];
    while let Some((owner, next)) = walk.last_mut() {
        let kid = doc
            .cell(*owner)
            .and_then(Cell::grid)
            .and_then(|g| g.cells().get(*next))
            .copied();
        let Some(kid) = kid else {
            walk.pop();
            path.pop();
            continue;
        };
        *next = next.saturating_add(1);
        path.push(kid);
        if doc.cell(kid).is_some_and(|c| c.grid().is_some()) {
            walk.push((kid, 0));
            continue;
        }
        // A leaf: one row of the table.
        for column in 0..levels as usize {
            let new = match path.get(column) {
                Some(from) => {
                    let source = cell(doc, *from)?;
                    let first_use = used.insert(*from);
                    let mut new =
                        Cell::new(if first_use { *from } else { fresh.next(doc)? }, Some(id));
                    new.text.clone_from(&source.text);
                    new.runs.clone_from(&source.runs);
                    new.links.clone_from(&source.links);
                    new.rel_size = source.rel_size;
                    new.style = source.style;
                    new.image.clone_from(&source.image);
                    new.last_edit = source.last_edit;
                    if first_use {
                        new.extra.clone_from(&source.extra);
                    }
                    new
                }
                None => {
                    let mut new = Cell::new(fresh.next(doc)?, Some(id));
                    new.last_edit = now;
                    new
                }
            };
            cells.push(new.id);
            s.insert(new);
        }
        path.pop();
    }
    let mut table = Grid::new(levels, rows, cells);
    table.vertical = old.vertical;
    table.extra.clone_from(&old.extra);
    s.get_mut(id)?.grid = Some(table);
    s.commit(tx, id)
}

// ---- Hierarchy swap -----------------------------------------------------------------------

fn is_blank(c: &Cell) -> bool {
    c.text.is_empty() && c.image.is_none() && c.grid.is_none() && c.note.is_empty()
}

/// The parts of a note that are separated by empty lines.
fn paragraphs(note: &str) -> Vec<&str> {
    note.split("\n\n").filter(|part| !part.is_empty()).collect()
}

/// What taking a cell out of its grid led to.
enum Taken {
    /// The grid had only that cell and is gone; its cell is to be taken out next.
    Emptied(CellId),
    /// The only cell of the base grid was taken out: its place is free.
    Hole,
    /// A row or column was deleted, or an empty cell left behind.
    Done,
}

/// The state of one hierarchy swap.
struct Swap<'a, 'b> {
    s: Scratch,
    doc: &'a Document,
    now: Option<i64>,
    fresh: &'a mut Fresh<'b>,
    /// Whether a grid that changed during the swap counts as a row (see [`Swap::shaped`]).
    rows: HashMap<CellId, bool>,
}

impl Swap<'_, '_> {
    /// TreeSheets' `Grid::SetOrient`: a grid is a row if it is wider than high, a column
    /// if it is higher than wide, and if it is square what it was before. Called when the
    /// grid of `owner`, which was `xs` by `ys`, has become `g`; returns whether it is a
    /// row now. (Before the swap a square grid counts as a column, as after loading a file.)
    fn shaped(&mut self, owner: CellId, (xs, ys): (u32, u32), g: &Grid) -> bool {
        let row = self.rows.entry(owner).or_insert(xs > ys);
        if g.xs != g.ys {
            *row = g.xs > g.ys;
        }
        *row
    }

    /// The first cell with the text `tag` inside the grid of `owner`, in document order.
    fn find_exact(&self, owner: CellId, tag: &str) -> Option<CellId> {
        let mut stack = self.s.kids(owner);
        stack.reverse();
        while let Some(id) = stack.pop() {
            let found = self.s.cells.get(&id)?;
            if found.text == tag {
                return Some(id);
            }
            if let Some(g) = &found.grid {
                stack.extend(g.cells.iter().rev());
            }
        }
        None
    }

    /// TreeSheets' `Grid::DeleteTagParent`: takes `tag` out of the grid it is in. The last
    /// cell of a grid takes the grid with it (and then its cell is next, unless that is
    /// `base`). Otherwise its row or column goes: in a grid of one column or row always,
    /// in a wider one only if nothing else is in it, and else an empty cell takes its place.
    fn take_out(&mut self, tag: CellId, base: CellId) -> Result<Taken, Error> {
        let owner = self
            .s
            .get(tag)?
            .parent
            .ok_or_else(|| broken("a cell has no parent"))?;
        let g = self.s.get(owner)?.grid.clone();
        let g = g.ok_or_else(|| broken("a parent has no grid"))?;
        let index = g.cells.iter().position(|c| *c == tag);
        let index = index.ok_or_else(|| broken("a cell is not in its grid"))?;
        let (xs, ys) = (g.xs as usize, g.ys as usize);
        if g.cells.len() == 1 {
            if owner == base {
                return Ok(Taken::Hole);
            }
            self.s.get_mut(owner)?.grid = None;
            return Ok(Taken::Emptied(owner));
        }
        let (fx, fy) = (index % xs.max(1), index / xs.max(1));
        let blank = |x: usize, y: usize| {
            let there = g.cells.get(y * xs + x).and_then(|c| self.s.cells.get(c));
            (x, y) == (fx, fy) || there.is_some_and(is_blank)
        };
        let row_empty = (0..xs).all(|x| blank(x, fy));
        let column_empty = (0..ys).all(|y| blank(fx, y));
        let mut left = g.clone();
        let keep = |keep: &dyn Fn(usize) -> bool| -> Vec<CellId> {
            let cells = g.cells.iter().enumerate();
            cells.filter(|(i, _)| keep(*i)).map(|(_, c)| *c).collect()
        };
        if ys > 1 && (xs == 1 || row_empty) {
            left.cells = keep(&|i| i / xs != fy);
            left.ys = g.ys.saturating_sub(1);
        } else if ys == 1 || column_empty {
            left.cells = keep(&|i| i % xs != fx);
            left.xs = g.xs.saturating_sub(1);
            if fx < left.col_widths.len() {
                left.col_widths.remove(fx);
            }
        } else {
            let mut empty = Cell::new(self.fresh.next(self.doc)?, Some(owner));
            empty.rel_size = self.s.get(owner)?.rel_size;
            empty.last_edit = self.now;
            if let Some(slot) = left.cells.get_mut(index) {
                *slot = empty.id;
            }
            self.s.insert(empty);
        }
        self.shaped(owner, (g.xs, g.ys), &left);
        self.s.get_mut(owner)?.grid = Some(left);
        Ok(Taken::Done)
    }

    /// TreeSheets' `Grid::Add`: puts `f` into the grid of `owner` in a new column at the
    /// end if the grid is a row, else in a new row; in a grid of several rows and
    /// columns the other new cells are empty and take after their neighbours.
    fn add(&mut self, owner: CellId, f: CellId) -> Result<(), Error> {
        let g = self.s.get(owner)?.grid.clone();
        let mut g = g.ok_or_else(|| broken("a parent has no grid"))?;
        let width = (g.xs as usize).max(1);
        let old = std::mem::take(&mut g.cells);
        let before = (g.xs, g.ys);
        let as_row = self.shaped(owner, before, &g);
        let mut like_cell = |s: &mut Scratch, like: Option<&CellId>| -> Result<CellId, Error> {
            let like = like.ok_or_else(|| broken("a grid is not full"))?;
            let new = new_cell(self.fresh.next(self.doc)?, owner, s.get(*like)?, self.now);
            let id = new.id;
            s.insert(new);
            Ok(id)
        };
        if as_row {
            for (y, row) in old.chunks(width).enumerate() {
                g.cells.extend_from_slice(row);
                g.cells.push(if y == 0 {
                    f
                } else {
                    like_cell(&mut self.s, row.last())?
                });
            }
            g.xs = g.xs.saturating_add(1);
            g.col_widths.push(self.s.col_width_of(self.doc, owner));
        } else {
            let last = old.len().saturating_sub(width);
            g.cells.push(f);
            for above in old.get(last..).unwrap_or_default().iter().skip(1) {
                g.cells.push(like_cell(&mut self.s, Some(above))?);
            }
            let mut cells = old;
            cells.append(&mut g.cells);
            g.cells = cells;
            g.ys = g.ys.saturating_add(1);
        }
        self.shaped(owner, before, &g);
        self.s.get_mut(owner)?.grid = Some(g);
        self.s.get_mut(f)?.parent = Some(owner);
        Ok(())
    }

    /// TreeSheets' `Grid::MergeTagCell`: puts `f` into the grid of `owner`. If a cell with
    /// the same text is there, `f` is merged into it and dropped: the paragraphs of its
    /// note that the other lacks are appended, and the cells of its grid are merged into
    /// the other's grid in the same way (or the grid moves over, if the other has none).
    /// Otherwise `f` is added at the end.
    fn merge(&mut self, owner: CellId, f: CellId) -> Result<(), Error> {
        let text = self.s.get(f)?.text.clone();
        let kids = self.s.kids(owner);
        let same = |c: &&CellId| self.s.cells.get(c).is_some_and(|c| c.text == text);
        let Some(into) = kids.iter().find(same).copied() else {
            return self.add(owner, f);
        };
        let from = self.s.get(f)?.note.clone();
        let note = &mut self.s.get_mut(into)?.note;
        let mut parts: Vec<String> = paragraphs(note).into_iter().map(String::from).collect();
        for part in paragraphs(&from) {
            if !parts.iter().any(|p| p == part) {
                if !note.is_empty() {
                    note.push_str("\n\n");
                }
                note.push_str(part);
                parts.push(part.to_string());
            }
        }
        if code_points(note) > MAX_TEXT_CODE_POINTS {
            return Err(Error::Limit(format!(
                "a note longer than {MAX_TEXT_CODE_POINTS} code points"
            )));
        }
        if let Some(moving) = self.s.get_mut(f)?.grid.take() {
            if self.s.get(into)?.grid.is_some() {
                for kid in moving.cells {
                    self.merge(into, kid)?;
                }
            } else {
                self.s.get_mut(into)?.grid = Some(moving);
                self.s.adopt(into)?;
            }
        }
        Ok(())
    }
}

/// TreeSheets' Hierarchy Swap (`A_HSWAP`, `Grid::HierarchySwap`): turns "parents holding
/// this text" into "this text holding those parents". `id` is the selected cell; the swap
/// works on the grid two levels up (the grid that the cell's parent is in) and on the text
/// of the cell. Every cell with that text anywhere below that grid's cells is taken out of
/// where it is, gets a copy of each cell it was inside of as a chain of grids of one cell
/// (outermost first, the innermost holding what the cell held), and is put directly into
/// that grid, merged with a cell of the same text if one is there already. A parent left
/// without cells disappears.
///
/// IDs: the swapped cells keep their IDs, as do the cells they hold; when two cells with
/// the same text merge, the one that was put there first keeps its ID and the other's is
/// gone. The copy of a parent is a new cell, unless the parent itself disappears: then the
/// copy takes its ID, so that the parent in effect moves.
pub(super) fn hierarchy_swap(
    tx: &mut Tx,
    id: CellId,
    now: Option<i64>,
    fresh: &mut Fresh,
) -> Result<(), Error> {
    let doc = tx.doc();
    let selected = cell(doc, id)?;
    let parent = selected.parent().and_then(|p| doc.cell(p));
    let base = parent.and_then(Cell::parent).and_then(|p| doc.cell(p));
    let (Some(parent), Some(base)) = (parent, base) else {
        return refused(format!(
            "cell {id} cannot move up in the hierarchy: it has no cell two levels above it"
        ));
    };
    if selected.text.is_empty() {
        return refused("only a cell that has text can be swapped");
    }
    let line = |c: &Cell| c.grid().is_some_and(|g| g.xs() == 1 || g.ys() == 1);
    if !line(base) {
        return refused("the cell can only move into a grid of one column or one row");
    }
    if !line(parent) {
        return refused("the cell can only move out of a grid of one column or one row");
    }
    let (tag, base) = (selected.text.clone(), base.id());
    let mut swap = Swap {
        s: Scratch::load(doc, base)?,
        doc,
        now,
        fresh,
        rows: HashMap::new(),
    };
    // Which new cell is a copy of which parent.
    let mut copies: Vec<(CellId, CellId)> = Vec::new();
    let mut done = false;
    let mut rounds = swap.s.cells.len().saturating_add(2);
    while !done {
        rounds = rounds
            .checked_sub(1)
            .ok_or_else(|| broken("the swap does not end"))?;
        let tops = swap.s.kids(base);
        let Some(f) = tops.iter().find_map(|c| swap.find_exact(*c, &tag)) else {
            break;
        };
        // A copy of every cell it is inside of goes inside it.
        let mut above = swap.s.get(f)?.parent;
        while let Some(p) = above.filter(|p| *p != base) {
            let original = swap.s.get(p)?;
            // A parent with the same text would be found and swapped again, forever.
            done |= original.text == tag;
            let vertical = original.grid.as_ref().is_none_or(|g| g.vertical);
            above = original.parent;
            let mut copy = original.clone();
            copy.id = swap.fresh.next(doc)?;
            copy.parent = Some(f);
            copy.kind = CellKind::Data;
            copy.extra = Extra::new();
            copy.grid = swap.s.get_mut(f)?.grid.take();
            let mut chain = Grid::new(1, 1, vec![copy.id]);
            chain.vertical = copy.grid.as_ref().map_or(vertical, |g| g.vertical);
            if let Some(g) = &mut copy.grid {
                g.vertical = vertical;
            }
            copies.push((copy.id, p));
            let copy_id = copy.id;
            swap.s.insert(copy);
            swap.s.adopt(copy_id)?;
            swap.s.get_mut(f)?.grid = Some(chain);
        }
        // Out of where it is, with every parent that this leaves empty.
        let mut out = f;
        let hole = loop {
            match swap.take_out(out, base)? {
                Taken::Emptied(next) => out = next,
                Taken::Hole => break true,
                Taken::Done => break false,
            }
        };
        if hole {
            if let Some(slot) = swap.s.get_mut(base)?.grid.as_mut() {
                slot.cells = vec![f];
            }
            swap.s.get_mut(f)?.parent = Some(base);
        } else {
            swap.merge(base, f)?;
        }
    }
    let mut s = swap.s;
    let mut there: HashSet<CellId> = s.reachable(base).into_iter().collect();
    for (copy, original) in copies {
        if there.contains(&copy) && !there.contains(&original) {
            s.rename(copy, original)?;
            there.remove(&copy);
            there.insert(original);
        }
    }
    s.commit(tx, base)
}

// ---- Wrap, collapse, split ------------------------------------------------------------------

/// TreeSheets' `Selection::Wrap`: a block of cells of the grid of `id` moves into the grid
/// of a new cell, which takes the place of the block's first cell. The new cell is empty
/// and takes after that first cell (colours, style, alignments, size); its grid has the
/// widths of the block's columns and the border and spacing of the grid around. The other
/// places of the block are left empty, and as with the Delete key their columns, and then
/// their rows, are deleted where that leaves whole columns or rows of the grid empty.
///
/// IDs: the cells of the block keep their IDs inside the new cell. The new cell is new,
/// and so are the empty cells that stay where cells of the block were.
pub(super) fn wrap(
    tx: &mut Tx,
    id: CellId,
    (x, y, xs, ys): (u32, u32, u32, u32),
    now: Option<i64>,
    fresh: &mut Fresh,
) -> Result<(), Error> {
    let doc = tx.doc();
    let g = grid(doc, id)?;
    check_block(id, g, x, y, xs, ys)?;
    let selected = block(g, x, y, xs, ys);
    let tallest = selected.iter().map(|c| height(doc, *c)).max().unwrap_or(0);
    if depth(doc, id)?.saturating_add(2).saturating_add(tallest) > MAX_DEPTH {
        return Err(too_deep());
    }
    check_room(doc, selected.len())?;
    let first = selected.first().and_then(|c| doc.cell(*c));
    let first = first.ok_or_else(|| broken("a grid is not full"))?;
    let wrapper = new_cell(fresh.next(doc)?, id, first, now);
    let wrapper_id = wrapper.id;
    let mut outer = g.clone();
    let mut added = vec![wrapper];
    let places = (y..y + ys).flat_map(|cy| (x..x + xs).map(move |cx| (cx, cy)));
    for (n, (cx, cy)) in places.enumerate() {
        let place = cy as usize * g.xs() as usize + cx as usize;
        let slot = outer.cells.get_mut(place);
        let slot = slot.ok_or_else(|| broken("a grid is not full"))?;
        if n == 0 {
            *slot = wrapper_id;
            continue;
        }
        // What `Cell::Clear` leaves of a cell: its looks and its note.
        let mut left = cell(doc, *slot)?.clone();
        left.id = fresh.next(doc)?;
        left.grid = None;
        left.text.clear();
        left.runs.clear();
        left.links.clear();
        left.image = None;
        left.extra = Extra::new();
        if left.kind == CellKind::Formula {
            left.kind = CellKind::Data;
        }
        *slot = left.id;
        added.push(left);
    }
    let mut inner = Grid::new(xs, ys, selected);
    let widths = g.col_widths.iter().skip(x as usize).take(xs as usize);
    inner.col_widths = widths.copied().collect();
    inner.border_color = g.border_color;
    inner.outer_spacing = g.outer_spacing;
    inner.vertical = g.vertical;
    tx.step(Step::AddCells(added))?;
    tx.step(Step::SetGrid {
        cell: id,
        grid: Some(outer),
    })?;
    tx.step(Step::SetGrid {
        cell: wrapper_id,
        grid: Some(inner),
    })?;
    if xs > 1 {
        delete_cells(tx, id, x + 1, y, xs - 1, ys)?;
    }
    if ys > 1 {
        delete_cells(tx, id, x, y + 1, 1, ys - 1)?;
    }
    Ok(())
}

/// TreeSheets' Collapse Cells (`A_COLLAPSE`): the texts of a block of at least two cells,
/// and of all cells inside them, are appended to the text of the block's first cell, each
/// after a space, in document order. The other cells of the block are emptied, and as with
/// the Delete key rows or columns left empty are deleted (looking, as TreeSheets does, only
/// at the block without its first row and first column). The first cell keeps its grid,
/// though the texts in it were appended too. Styles and links of the appended texts are
/// lost; the first cell's stay where they are.
pub(super) fn collapse(
    tx: &mut Tx,
    id: CellId,
    (x, y, xs, ys): (u32, u32, u32, u32),
) -> Result<(), Error> {
    let doc = tx.doc();
    let g = grid(doc, id)?;
    check_block(id, g, x, y, xs, ys)?;
    let selected = block(g, x, y, xs, ys);
    let Some((first, others)) = selected.split_first() else {
        return Err(broken("a grid is not full"));
    };
    if others.is_empty() {
        return refused("more than one cell must be selected to collapse");
    }
    let mut more = String::new();
    for top in &selected {
        let inside = descendants(doc, *top);
        for from in std::iter::once(top).chain(&inside) {
            let text = &cell(doc, *from)?.text;
            if from != first && !text.is_empty() {
                more.push(' ');
                more.push_str(text);
            }
        }
    }
    let target = cell(doc, *first)?;
    if !has_content(target) && more.is_empty() {
        return refused("there is no content to collapse");
    }
    let end = len32(&target.text)?;
    if code_points(&target.text).saturating_add(code_points(&more)) > MAX_TEXT_CODE_POINTS {
        return Err(Error::Limit(format!(
            "cell {first}: text longer than {MAX_TEXT_CODE_POINTS} code points"
        )));
    }
    if !more.is_empty() {
        tx.step(Step::Splice {
            cell: *first,
            at: end,
            del: 0,
            ins: more,
        })?;
    }
    clear_cells(tx, others)?;
    let (dx, dy) = (u32::from(xs > 1), u32::from(ys > 1));
    delete_cells(tx, id, x + dx, y + dy, xs - dx, ys - dy)
}

/// TreeSheets' Split Cell (`A_SPLITCELL`): the text of a cell is cut at `at` and the two
/// parts become the cells of a new grid of two columns in the cell, which itself is
/// emptied (it keeps its colours, note and so on). Each part has the styles and links of
/// its text and the cell's looks; the image goes with the first part. If the cell already
/// had a grid, the cell gets a grid of two cells instead: the first holds the two parts,
/// the second the old grid.
///
/// IDs: the cell keeps its ID and its place, and so do the cells of its old grid; the parts
/// and the two holders are new cells.
pub(super) fn split_cell(
    tx: &mut Tx,
    id: CellId,
    at: u32,
    now: Option<i64>,
    fresh: &mut Fresh,
) -> Result<(), Error> {
    let doc = tx.doc();
    let whole = cell(doc, id)?;
    check_range(whole, at, 0)?;
    let len = len32(&whole.text)?;
    let existing = whole.grid().cloned();
    let levels = if existing.is_some() {
        height(doc, id).saturating_add(1).max(2)
    } else {
        1
    };
    if depth(doc, id)?.saturating_add(levels) > MAX_DEPTH {
        return Err(too_deep());
    }
    check_room(doc, 4)?;
    let cut = text::byte_offset(&whole.text, at).unwrap_or(whole.text.len());
    let (before, after) = whole.text.split_at(cut);
    let mut part = |from: u32, to: u32, text: &str| -> Result<Cell, Error> {
        let mut part = new_cell(fresh.next(doc)?, id, whole, whole.last_edit);
        part.text = text.to_string();
        part.runs.clone_from(&whole.runs);
        part.links.clone_from(&whole.links);
        for (pos, n) in [(to, len - to), (0, from)] {
            text::runs_replace(&mut part.runs, pos, n, 0, whole.style);
            text::links_replace(&mut part.links, pos, n, 0);
        }
        Ok(part)
    };
    let mut first = part(0, at, before)?;
    let second = part(at, len, after)?;
    first.image.clone_from(&whole.image);
    let pair = vec![first.id, second.id];
    let width = col_width_of(doc, id);
    let outer = |cells: Vec<CellId>| fresh_grid(doc, id, 2, 1, cells);
    let Some(existing) = existing else {
        let halves = outer(pair);
        clear_own(tx, id)?;
        tx.step(Step::AddCells(vec![first, second]))?;
        return tx.step(Step::SetGrid {
            cell: id,
            grid: Some(halves),
        });
    };
    let holder = |fresh: &mut Fresh| Ok::<Cell, Error>(new_cell(fresh.next(doc)?, id, whole, now));
    let (target, rest) = (holder(fresh)?, holder(fresh)?);
    let (target_id, rest_id) = (target.id, rest.id);
    let top = outer(vec![target_id, rest_id]);
    let mut halves = Grid::new(2, 1, pair);
    halves.col_widths = vec![width; 2];
    halves.border_color = top.border_color;
    halves.outer_spacing = top.outer_spacing;
    halves.vertical = top.vertical;
    first.parent = Some(target_id);
    let mut second = second;
    second.parent = Some(target_id);
    clear_own(tx, id)?;
    tx.step(Step::AddCells(vec![target, rest, first, second]))?;
    tx.step(Step::SetGrid {
        cell: id,
        grid: Some(top),
    })?;
    tx.step(Step::SetGrid {
        cell: rest_id,
        grid: Some(existing),
    })?;
    tx.step(Step::SetGrid {
        cell: target_id,
        grid: Some(halves),
    })
}

// ---- Tags, sizes, widths, checkboxes --------------------------------------------------------

/// The listed cells, each once, and with `recursive` the cells inside them.
fn with_inside(doc: &Document, cells: &[CellId], recursive: bool) -> Result<Vec<CellId>, Error> {
    let mut seen = HashSet::new();
    let mut all = Vec::new();
    for id in cells {
        cell(doc, *id)?;
        if seen.insert(*id) {
            all.push(*id);
        }
        if recursive {
            all.extend(
                descendants(doc, *id)
                    .into_iter()
                    .filter(|c| seen.insert(*c)),
            );
        }
    }
    Ok(all)
}

/// TreeSheets' Add Cell Text as Tag (`A_TAGADD`): the text of every listed cell that has
/// text becomes a tag of the document, with the cell's colours (an existing tag of that
/// name takes them).
pub(super) fn tag_cells(tx: &mut Tx, cells: &[CellId]) -> Result<(), Error> {
    for id in with_inside(tx.doc(), cells, false)? {
        let c = cell(tx.doc(), id)?;
        if c.text.is_empty() {
            continue;
        }
        let old = tx.doc().tags().get(&c.text);
        let tag = Tag {
            cell_color: c.cell_color,
            text_color: c.text_color,
            extra: old.map(|tag| tag.extra.clone()).unwrap_or_default(),
        };
        set_tag(tx, c.text.clone(), Some(tag))?;
    }
    Ok(())
}

/// TreeSheets' Remove Cell Text from Tags (`A_TAGREMOVE`): the tags named by the texts of
/// the listed cells are removed; a text that is no tag is passed over.
pub(super) fn untag_cells(tx: &mut Tx, cells: &[CellId]) -> Result<(), Error> {
    for id in with_inside(tx.doc(), cells, false)? {
        let name = cell(tx.doc(), id)?.text.clone();
        if tx.doc().tags().contains_key(&name) {
            set_tag(tx, name, None)?;
        }
    }
    Ok(())
}

/// TreeSheets' text size actions (`Text::RelSize`): adds `delta` to the relative size of
/// the listed cells and, with `recursive`, of the cells inside them, keeping each between
/// [`MIN_REL_SIZE`] and [`MAX_REL_SIZE`].
pub(super) fn adjust_rel_size(
    tx: &mut Tx,
    cells: &[CellId],
    delta: i32,
    recursive: bool,
) -> Result<(), Error> {
    for id in with_inside(tx.doc(), cells, recursive)? {
        let old = cell(tx.doc(), id)?.rel_size;
        let new = old.saturating_add(delta).clamp(MIN_REL_SIZE, MAX_REL_SIZE);
        if new != old && delta != 0 {
            set(tx, id, Field::RelSize(new))?;
        }
    }
    Ok(())
}

/// TreeSheets' column width actions (`Grid::ResizeColWidths`): adds `delta` to the widths of
/// `count` columns from `column` on, none becoming smaller than [`MIN_COL_WIDTH`], and with
/// `recursive` to every column of every grid inside the cells of those columns.
pub(super) fn adjust_column_widths(
    tx: &mut Tx,
    id: CellId,
    column: u32,
    count: u32,
    delta: i32,
    recursive: bool,
) -> Result<(), Error> {
    let g = grid(tx.doc(), id)?;
    if column.checked_add(count).is_none_or(|end| end > g.xs()) {
        return refused(format!(
            "cell {id}: no {count} columns from {column} in a grid of {} columns",
            g.xs()
        ));
    }
    if count == 0 || delta == 0 {
        return Ok(());
    }
    let mut work = vec![(id, column as usize, count as usize)];
    while let Some((owner, from, n)) = work.pop() {
        let g = grid(tx.doc(), owner)?;
        let mut widths = g.col_widths.clone();
        for width in widths.iter_mut().skip(from).take(n) {
            let wide = i64::from(*width).saturating_add(i64::from(delta));
            *width = u32::try_from(wide.max(i64::from(MIN_COL_WIDTH))).unwrap_or(u32::MAX);
        }
        if recursive {
            let xs = g.xs() as usize;
            for (index, kid) in g.cells().iter().enumerate() {
                let inside = tx.doc().cell(*kid).and_then(Cell::grid);
                if let Some(inside) = inside.filter(|_| (from..from + n).contains(&(index % xs))) {
                    work.push((*kid, 0, inside.xs() as usize));
                }
            }
        }
        if widths != g.col_widths {
            set(tx, owner, Field::ColWidths(widths))?;
        }
    }
    Ok(())
}

/// The state of the to-do checkbox that a text starts with: 0 none, 1 open, 2 ticked.
fn check_box(text: &str) -> u8 {
    match text.chars().next() {
        Some('\u{2610}') => 1,
        Some('\u{2611}') => 2,
        _ => 0,
    }
}

/// TreeSheets' Toggle to-do checkbox (`A_TOGGLECHECK`). A checkbox is the character U+2610
/// (open) or U+2611 (ticked) at the start of the text. If not every listed cell has one,
/// those without get an open one and a space; if all have one and not all are ticked, all
/// become ticked; if all are ticked, the checkboxes (and the space after them) are removed.
/// Stamps the last-edit time of the cells it works on.
pub(super) fn toggle_check(tx: &mut Tx, cells: &[CellId], now: Option<i64>) -> Result<(), Error> {
    let cells = with_inside(tx.doc(), cells, false)?;
    let state = |tx: &Tx, id: &CellId| tx.doc().cell(*id).map_or(0, |c| check_box(&c.text));
    let boxes = cells.iter().all(|id| state(tx, id) != 0);
    let ticked = cells.iter().all(|id| state(tx, id) == 2);
    for id in cells {
        let text = &cell(tx.doc(), id)?.text;
        if !boxes {
            if check_box(text) != 0 {
                continue;
            }
            replace_text(tx, id, 0, 0, "\u{2610} ", now)?;
        } else if ticked {
            let len = if text.chars().nth(1) == Some(' ') {
                2
            } else {
                1
            };
            replace_text(tx, id, 0, len, "", now)?;
        } else {
            replace_text(tx, id, 0, 1, "\u{2611}", now)?;
        }
        stamp(tx, id, now)?;
    }
    Ok(())
}
