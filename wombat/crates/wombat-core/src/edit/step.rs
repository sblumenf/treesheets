//! Steps: the small reversible changes that every command is made of (decision A9).
//!
//! Applying a step changes the document and returns the step that undoes it exactly. A step
//! holds only what it needs for that: a text step the text it puts in, a property step one
//! value, a step that takes cells out of the document those cells. A step either applies
//! completely or returns an error with the document untouched.
//!
//! Every step also says by how much it changes the bound on the size of the saved file
//! (`size.rs`), from the parts it touches alone.
//!
//! Between the steps of one command the document may break the model's rules (cells that no
//! grid lists yet, for example); it is valid again when the command is complete.

use std::collections::{BTreeMap, BTreeSet};
use std::mem;

use crate::error::{refused, Error};
use crate::id::CellId;
use crate::model::{
    Cell, CellKind, Color, Document, DrawStyle, Extra, Grid, ImageRef, Link, Run, Style, Tag,
    TextAlign, VertAlign,
};
use crate::text::byte_offset;

use super::{size, Changes};

/// Rows or columns of a grid.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Axis {
    /// Rows.
    Rows,
    /// Columns.
    Columns,
}

/// One field of a cell (or of its grid) with a value for it.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Field {
    Kind(CellKind),
    CellColor(Color),
    TextColor(Color),
    DrawStyle(DrawStyle),
    Note(String),
    TextAlign(TextAlign),
    VertAlign(VertAlign),
    RelSize(i32),
    Style(Style),
    Runs(Vec<Run>),
    Links(Vec<Link>),
    LastEdit(Option<i64>),
    Image(Option<ImageRef>),
    Extra(Extra),
    BorderColor(Color),
    OuterSpacing(u32),
    Vertical(bool),
    ColWidths(Vec<u32>),
}

impl Field {
    /// Puts the value into the cell and returns the field with the value that was there.
    fn swap(self, cell: &mut Cell) -> Result<Field, Error> {
        let id = cell.id;
        let no_grid = || Error::Refused(format!("cell {id} has no grid"));
        Ok(match self {
            Field::Kind(v) => Field::Kind(mem::replace(&mut cell.kind, v)),
            Field::CellColor(v) => Field::CellColor(mem::replace(&mut cell.cell_color, v)),
            Field::TextColor(v) => Field::TextColor(mem::replace(&mut cell.text_color, v)),
            Field::DrawStyle(v) => Field::DrawStyle(mem::replace(&mut cell.draw_style, v)),
            Field::Note(v) => Field::Note(mem::replace(&mut cell.note, v)),
            Field::TextAlign(v) => Field::TextAlign(mem::replace(&mut cell.text_align, v)),
            Field::VertAlign(v) => Field::VertAlign(mem::replace(&mut cell.vert_align, v)),
            Field::RelSize(v) => Field::RelSize(mem::replace(&mut cell.rel_size, v)),
            Field::Style(v) => Field::Style(mem::replace(&mut cell.style, v)),
            Field::Runs(v) => Field::Runs(mem::replace(&mut cell.runs, v)),
            Field::Links(v) => Field::Links(mem::replace(&mut cell.links, v)),
            Field::LastEdit(v) => Field::LastEdit(mem::replace(&mut cell.last_edit, v)),
            Field::Image(v) => Field::Image(mem::replace(&mut cell.image, v)),
            Field::Extra(v) => Field::Extra(mem::replace(&mut cell.extra, v)),
            Field::BorderColor(v) => {
                let grid = cell.grid.as_mut().ok_or_else(no_grid)?;
                Field::BorderColor(mem::replace(&mut grid.border_color, v))
            }
            Field::OuterSpacing(v) => {
                let grid = cell.grid.as_mut().ok_or_else(no_grid)?;
                Field::OuterSpacing(mem::replace(&mut grid.outer_spacing, v))
            }
            Field::Vertical(v) => {
                let grid = cell.grid.as_mut().ok_or_else(no_grid)?;
                Field::Vertical(mem::replace(&mut grid.vertical, v))
            }
            Field::ColWidths(v) => {
                let grid = cell.grid.as_mut().ok_or_else(no_grid)?;
                if v.len() != grid.xs as usize {
                    return refused(format!("cell {id}: one column width per column is needed"));
                }
                Field::ColWidths(mem::replace(&mut grid.col_widths, v))
            }
        })
    }
}

/// A run or a link: a range of text that starts somewhere.
pub(crate) trait Ranged: PartialEq {
    fn start(&self) -> u32;
    fn start_mut(&mut self) -> &mut u32;
    /// Whether `self`, moved by `shift` code points, is `other`.
    fn is_shifted(&self, other: &Self, shift: i64) -> bool;
}

impl Ranged for Run {
    fn start(&self) -> u32 {
        self.start
    }

    fn start_mut(&mut self) -> &mut u32 {
        &mut self.start
    }

    fn is_shifted(&self, other: &Run, shift: i64) -> bool {
        i64::from(self.start).checked_add(shift) == Some(i64::from(other.start))
            && (self.len, self.style, self.color) == (other.len, other.style, other.color)
            && self.extra == other.extra
    }
}

impl Ranged for Link {
    fn start(&self) -> u32 {
        self.start
    }

    fn start_mut(&mut self) -> &mut u32 {
        &mut self.start
    }

    fn is_shifted(&self, other: &Link, shift: i64) -> bool {
        i64::from(self.start).checked_add(shift) == Some(i64::from(other.start))
            && self.len == other.len
            && self.target == other.target
            && self.extra == other.extra
    }
}

/// A change to a cell's list of runs or links as a text edit makes it: the ranges that the
/// edit touches are replaced, and those after it move with the text. It holds only the
/// touched ranges, however many there are after them (decision A46).
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Patch<T> {
    /// The index of the first range that is replaced.
    pub(crate) at: usize,
    /// How many ranges are replaced.
    pub(crate) del: usize,
    /// The ranges that take their place.
    pub(crate) ins: Vec<T>,
    /// By how many code points the ranges after them move.
    pub(crate) shift: i64,
}

impl<T: Ranged + Clone> Patch<T> {
    /// The patch that turns `old` into `new`, where the ranges that follow the changed
    /// ones have moved by `shift`; `None` if nothing differs.
    pub(crate) fn between(old: &[T], new: &[T], shift: i64) -> Option<Patch<T>> {
        if old == new {
            return None;
        }
        let same = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        let (old_rest, new_rest) = (old.get(same..)?, new.get(same..)?);
        let moved = old_rest
            .iter()
            .rev()
            .zip(new_rest.iter().rev())
            .take_while(|(a, b)| a.is_shifted(b, shift))
            .count();
        Some(Patch {
            at: same,
            del: old_rest.len().checked_sub(moved)?,
            ins: new_rest.get(..new_rest.len().checked_sub(moved)?)?.to_vec(),
            // With no range after the changed ones there is nothing to move, also not back.
            shift: if moved == 0 { 0 } else { shift },
        })
    }

    /// Applies the patch to `list` and returns the patch that undoes it; `None`, with the
    /// list untouched, if it does not fit.
    fn apply(self, list: &mut Vec<T>) -> Option<Patch<T>> {
        let end = self.at.checked_add(self.del).filter(|e| *e <= list.len())?;
        let shift = self.shift;
        let moved = |start: u32| u32::try_from(i64::from(start).checked_add(shift)?).ok();
        if list.get(end..)?.iter().any(|r| moved(r.start()).is_none()) {
            return None;
        }
        let count = self.ins.len();
        let removed: Vec<T> = list.splice(self.at..end, self.ins).collect();
        for range in list.iter_mut().skip(self.at.saturating_add(count)) {
            let start = range.start_mut();
            *start = moved(*start).unwrap_or(*start);
        }
        Some(Patch {
            at: self.at,
            del: count,
            ins: removed,
            shift: shift.checked_neg()?,
        })
    }
}

/// A small reversible change. See the module documentation.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Step {
    /// Replaces `del` code points of a cell's text from `at` on by `ins`. Runs and links
    /// are not touched: they are separate [`Step::Set`] steps.
    Splice {
        cell: CellId,
        at: u32,
        del: u32,
        ins: String,
    },
    /// Sets one field of a cell.
    Set { cell: CellId, field: Field },
    /// Changes the runs of a cell as an edit of its text does.
    PatchRuns { cell: CellId, patch: Patch<Run> },
    /// Changes the links of a cell as an edit of its text does.
    PatchLinks { cell: CellId, patch: Patch<Link> },
    /// Sets (or with `None` removes) a tag of the document.
    Tag { name: String, tag: Option<Tag> },
    /// Puts cells into the document that no grid lists yet. Each cell says which cell is
    /// its parent and which cells its grid has.
    AddCells(Vec<Cell>),
    /// Takes cells out of the document. They must not be listed by a grid that stays.
    RemoveCells(Vec<CellId>),
    /// Inserts whole rows or columns of existing cells into the grid of `owner` at `at`.
    /// `ids` are the cells row by row; `widths` the widths of inserted columns.
    InsertLines {
        owner: CellId,
        axis: Axis,
        at: u32,
        ids: Vec<CellId>,
        widths: Vec<u32>,
    },
    /// Takes `count` rows or columns out of the grid of `owner`, leaving at least one. The
    /// cells stay in the document, unlisted.
    RemoveLines {
        owner: CellId,
        axis: Axis,
        at: u32,
        count: u32,
    },
    /// Moves `count` rows or columns starting at `from` so that they start at `to`.
    MoveLines {
        owner: CellId,
        axis: Axis,
        from: u32,
        count: u32,
        to: u32,
    },
    /// Replaces the whole grid of a cell: its shape, the list of its cells and its
    /// properties. The cells it lists must be in the document and become children of
    /// `cell`; the cells of the old grid stay in the document, unlisted. This is the general
    /// step for restructuring: any rearrangement of existing cells is a few of these.
    SetGrid { cell: CellId, grid: Option<Grid> },
    /// Makes two cells trade places, each going where the other was (possibly in another
    /// grid), with everything inside them.
    Swap { a: CellId, b: CellId },
    /// Applies `step` and then gives the listed cells the parents they are listed with.
    /// Never asked for by a command: it is the inverse of a [`Step::SetGrid`] or
    /// [`Step::InsertLines`] that took cells which were children of other cells, and gives
    /// them back.
    Reparented {
        step: Box<Step>,
        parents: Vec<(CellId, Option<CellId>)>,
    },
}

/// What the steps applied so far have touched.
#[derive(Clone, Default, Debug)]
pub(crate) struct ChangeSet {
    /// By how much the steps changed the bound on the size of the saved file (`size.rs`).
    pub(crate) bytes: i64,
    changed: BTreeSet<CellId>,
    created: BTreeSet<CellId>,
    removed: BTreeSet<CellId>,
    grids: BTreeSet<CellId>,
    tags: BTreeSet<String>,
    /// The properties of the grid a cell had before the first step that replaced it.
    first_grids: BTreeMap<CellId, Option<GridProperties>>,
}

/// What a grid has besides its shape and cells.
type GridProperties = (Color, u32, bool, Vec<u32>, Extra);

fn grid_properties(grid: &Grid) -> GridProperties {
    (
        grid.border_color,
        grid.outer_spacing,
        grid.vertical,
        grid.col_widths.clone(),
        grid.extra.clone(),
    )
}

impl ChangeSet {
    fn grew(&mut self, bytes: u64) {
        self.bytes = self.bytes.saturating_add_unsigned(bytes);
    }

    fn shrank(&mut self, bytes: u64) {
        self.bytes = self.bytes.saturating_sub_unsigned(bytes);
    }

    /// Adds what a later command touched, for the report of a group of commands.
    pub(crate) fn absorb(&mut self, later: ChangeSet) {
        self.changed.extend(later.changed);
        self.created.extend(later.created);
        self.removed.extend(later.removed);
        self.grids.extend(later.grids);
        self.tags.extend(later.tags);
        for (id, grid) in later.first_grids {
            self.first_grids.entry(id).or_insert(grid);
        }
    }

    /// The report: what differs between the document before the first step and `doc` now.
    pub(crate) fn finish(mut self, doc: &Document) -> Changes {
        // A cell both created and removed either never existed for the outside (made and
        // dropped again) or was taken out and put back; the latter may differ in any way.
        let both: Vec<CellId> = self.created.intersection(&self.removed).copied().collect();
        for id in both {
            self.created.remove(&id);
            self.removed.remove(&id);
            if doc.cell(id).is_some() {
                self.changed.insert(id);
                self.grids.insert(id);
            }
        }
        for (id, before) in &self.first_grids {
            let now = doc.cell(*id).and_then(|c| c.grid.as_ref());
            if let (Some(before), Some(now)) = (before, now) {
                if *before != grid_properties(now) {
                    self.changed.insert(*id);
                }
            }
        }
        let alive = |id: &CellId| !self.created.contains(id) && !self.removed.contains(id);
        Changes {
            changed: self.changed.iter().copied().filter(alive).collect(),
            grids: self.grids.iter().copied().filter(alive).collect(),
            created: self.created.iter().copied().collect(),
            removed: self.removed.iter().copied().collect(),
            tags: self.tags.into_iter().collect(),
        }
    }
}

fn missing(id: CellId) -> Error {
    Error::NotFound(format!("cell {id}"))
}

fn grid_of(doc: &Document, owner: CellId) -> Result<&Grid, Error> {
    doc.cells
        .get(&owner)
        .ok_or_else(|| missing(owner))?
        .grid
        .as_ref()
        .ok_or_else(|| Error::Refused(format!("cell {owner} has no grid")))
}

fn grid_of_mut(doc: &mut Document, owner: CellId) -> Result<&mut Grid, Error> {
    doc.cells
        .get_mut(&owner)
        .ok_or_else(|| missing(owner))?
        .grid
        .as_mut()
        .ok_or_else(|| Error::Refused(format!("cell {owner} has no grid")))
}

/// Makes `parent` the parent of `ids`; returns the cells that had another, with it.
fn set_parents(
    doc: &mut Document,
    ids: &[CellId],
    parent: CellId,
) -> Result<Vec<(CellId, Option<CellId>)>, Error> {
    if let Some(id) = ids.iter().find(|id| !doc.cells.contains_key(id)) {
        return Err(missing(*id));
    }
    let mut previous = Vec::new();
    for id in ids {
        if let Some(cell) = doc.cells.get_mut(id) {
            if cell.parent != Some(parent) {
                previous.push((*id, cell.parent.replace(parent)));
            }
        }
    }
    Ok(previous)
}

/// The inverse `step`, which also gives back the cells that the step it undoes took.
fn giving_back(step: Step, parents: Vec<(CellId, Option<CellId>)>) -> Step {
    if parents.is_empty() {
        return step;
    }
    Step::Reparented {
        step: Box::new(step),
        parents,
    }
}

/// The cells of a grid as rows.
fn rows(grid: &Grid) -> impl Iterator<Item = &[CellId]> {
    grid.cells.chunks(grid.xs.max(1) as usize)
}

impl Step {
    /// Applies the step and returns the step that undoes it. On an error the document is
    /// unchanged.
    pub(crate) fn apply(self, doc: &mut Document, seen: &mut ChangeSet) -> Result<Step, Error> {
        match self {
            Step::Splice { cell, at, del, ins } => {
                let c = doc.cells.get_mut(&cell).ok_or_else(|| missing(cell))?;
                let range = byte_offset(&c.text, at).zip(
                    at.checked_add(del)
                        .and_then(|end| byte_offset(&c.text, end)),
                );
                let Some((from, to)) = range else {
                    return refused(format!("cell {cell}: the range is outside the text"));
                };
                let removed = c.text.get(from..to).unwrap_or_default().to_owned();
                let inserted = u32::try_from(ins.chars().count())
                    .map_err(|_| Error::Limit("text too long".into()))?;
                let was_empty = c.text.is_empty();
                c.text.replace_range(from..to, &ins);
                seen.grew(size::text_piece(&ins) + size::text_frame(c.text.is_empty()));
                seen.shrank(size::text_piece(&removed) + size::text_frame(was_empty));
                seen.changed.insert(cell);
                Ok(Step::Splice {
                    cell,
                    at,
                    del: inserted,
                    ins: removed,
                })
            }
            Step::Set { cell, field } => {
                let c = doc.cells.get_mut(&cell).ok_or_else(|| missing(cell))?;
                let new = size::field(&field);
                let field = field.swap(c)?;
                seen.grew(new);
                seen.shrank(size::field(&field));
                seen.changed.insert(cell);
                Ok(Step::Set { cell, field })
            }
            Step::PatchRuns { cell, patch } => {
                let c = doc.cells.get_mut(&cell).ok_or_else(|| missing(cell))?;
                let old = size::runs(&c.runs);
                let Some(patch) = patch.apply(&mut c.runs) else {
                    return refused(format!("cell {cell}: runs that do not fit"));
                };
                seen.grew(size::runs(&c.runs));
                seen.shrank(old);
                seen.changed.insert(cell);
                Ok(Step::PatchRuns { cell, patch })
            }
            Step::PatchLinks { cell, patch } => {
                let c = doc.cells.get_mut(&cell).ok_or_else(|| missing(cell))?;
                let old = size::links(&c.links);
                let Some(patch) = patch.apply(&mut c.links) else {
                    return refused(format!("cell {cell}: links that do not fit"));
                };
                seen.grew(size::links(&c.links));
                seen.shrank(old);
                seen.changed.insert(cell);
                Ok(Step::PatchLinks { cell, patch })
            }
            Step::Tag { name, tag } => {
                seen.grew(size::tag(&name, tag.as_ref()));
                let old = match tag {
                    Some(tag) => doc.tags.insert(name.clone(), tag),
                    None => doc.tags.remove(&name),
                };
                seen.shrank(size::tag(&name, old.as_ref()));
                seen.tags.insert(name.clone());
                Ok(Step::Tag { name, tag: old })
            }
            Step::AddCells(cells) => {
                let mut ids = BTreeSet::new();
                for cell in &cells {
                    let id = cell.id;
                    if id.0 == doc.id.0 || doc.cells.contains_key(&id) || !ids.insert(id) {
                        return refused(format!("the ID {id} is already in use"));
                    }
                }
                let ids = cells.iter().map(|c| c.id).collect();
                for cell in cells {
                    seen.grew(size::cell(&cell));
                    seen.created.insert(cell.id);
                    doc.cells.insert(cell.id, cell);
                }
                Ok(Step::RemoveCells(ids))
            }
            Step::RemoveCells(ids) => {
                if let Some(id) = ids
                    .iter()
                    .find(|id| **id == doc.root || !doc.cells.contains_key(id))
                {
                    return refused(format!("cell {id} cannot be removed"));
                }
                let mut cells = Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(cell) = doc.cells.remove(&id) {
                        seen.shrank(size::cell(&cell));
                        seen.removed.insert(id);
                        cells.push(cell);
                    }
                }
                Ok(Step::AddCells(cells))
            }
            Step::InsertLines {
                owner,
                axis,
                at,
                ids,
                widths,
            } => {
                let grid = grid_of(doc, owner)?;
                let (xs, ys) = (grid.xs as usize, grid.ys as usize);
                let (across, along) = match axis {
                    Axis::Rows => (xs, ys),
                    Axis::Columns => (ys, xs),
                };
                let count = ids.len() / across.max(1);
                let widths_fit = match axis {
                    Axis::Rows => widths.is_empty(),
                    Axis::Columns => widths.len() == count,
                };
                let count32 = u32::try_from(count).ok();
                let new_len = count32.and_then(|n| n.checked_add(u32::try_from(along).ok()?));
                let (Some(count32), Some(new_len)) = (count32, new_len) else {
                    return Err(Error::Limit("a grid that large".into()));
                };
                if count == 0 || count * across != ids.len() || at as usize > along || !widths_fit {
                    return refused(format!("cell {owner}: rows or columns that do not fit"));
                }
                let previous = set_parents(doc, &ids, owner)?;
                let grid = grid_of_mut(doc, owner)?;
                match axis {
                    Axis::Rows => {
                        let at = (at as usize) * xs;
                        grid.cells.splice(at..at, ids);
                        grid.ys = new_len;
                    }
                    Axis::Columns => {
                        let mut cells = Vec::with_capacity(grid.cells.len() + ids.len());
                        for (row, new) in rows(grid).zip(ids.chunks(count)) {
                            let (left, right) = row.split_at(at as usize);
                            cells.extend_from_slice(left);
                            cells.extend_from_slice(new);
                            cells.extend_from_slice(right);
                        }
                        grid.cells = cells;
                        grid.xs = new_len;
                        let at = at as usize;
                        seen.shrank(size::grid_widths(grid));
                        grid.col_widths.splice(at..at, widths);
                        seen.grew(size::grid_widths(grid));
                        // The widths are a property of the cell, and now differ.
                        seen.changed.insert(owner);
                    }
                }
                seen.grids.insert(owner);
                let inverse = Step::RemoveLines {
                    owner,
                    axis,
                    at,
                    count: count32,
                };
                Ok(giving_back(inverse, previous))
            }
            Step::RemoveLines {
                owner,
                axis,
                at,
                count,
            } => {
                let grid = grid_of_mut(doc, owner)?;
                let along = match axis {
                    Axis::Rows => grid.ys,
                    Axis::Columns => grid.xs,
                };
                let end = at.checked_add(count).filter(|end| *end <= along);
                let Some(end) = end.filter(|_| count > 0 && count < along) else {
                    return refused(format!("cell {owner}: no such rows or columns to remove"));
                };
                let (ids, widths) = match axis {
                    Axis::Rows => {
                        let xs = grid.xs as usize;
                        let ids = grid
                            .cells
                            .drain((at as usize) * xs..(end as usize) * xs)
                            .collect();
                        grid.ys = along - count;
                        (ids, Vec::new())
                    }
                    Axis::Columns => {
                        let inside = |x: usize| x >= at as usize && x < end as usize;
                        let mut ids = Vec::new();
                        let mut cells = Vec::new();
                        for row in rows(grid) {
                            for (x, id) in row.iter().enumerate() {
                                if inside(x) { &mut ids } else { &mut cells }.push(*id);
                            }
                        }
                        grid.cells = cells;
                        grid.xs = along - count;
                        seen.shrank(size::grid_widths(grid));
                        let widths = grid.col_widths.drain(at as usize..end as usize).collect();
                        seen.grew(size::grid_widths(grid));
                        seen.changed.insert(owner);
                        (ids, widths)
                    }
                };
                seen.grids.insert(owner);
                Ok(Step::InsertLines {
                    owner,
                    axis,
                    at,
                    ids,
                    widths,
                })
            }
            Step::MoveLines {
                owner,
                axis,
                from,
                count,
                to,
            } => {
                let grid = grid_of_mut(doc, owner)?;
                let along = match axis {
                    Axis::Rows => grid.ys,
                    Axis::Columns => grid.xs,
                };
                let fits = |start: u32| start.checked_add(count).is_some_and(|e| e <= along);
                if count == 0 || !fits(from) || !fits(to) {
                    return refused(format!("cell {owner}: no such rows or columns to move"));
                }
                // The new order of the lines: the old index of each.
                let mut order: Vec<usize> = (0..along as usize)
                    .filter(|i| *i < from as usize || *i >= (from + count) as usize)
                    .collect();
                let moved = from as usize..(from + count) as usize;
                order.splice(to as usize..to as usize, moved);
                let xs = grid.xs as usize;
                let mut cells = Vec::with_capacity(grid.cells.len());
                match axis {
                    Axis::Rows => {
                        for line in &order {
                            let row = grid.cells.get(line * xs..(line + 1) * xs);
                            cells.extend_from_slice(row.unwrap_or_default());
                        }
                    }
                    Axis::Columns => {
                        for row in rows(grid) {
                            cells.extend(order.iter().filter_map(|x| row.get(*x)));
                        }
                        let widths: Vec<u32> = order
                            .iter()
                            .filter_map(|x| grid.col_widths.get(*x).copied())
                            .collect();
                        if widths != grid.col_widths {
                            grid.col_widths = widths;
                            seen.changed.insert(owner);
                        }
                    }
                }
                grid.cells = cells;
                seen.grids.insert(owner);
                Ok(Step::MoveLines {
                    owner,
                    axis,
                    from: to,
                    count,
                    to: from,
                })
            }
            Step::SetGrid { cell, grid } => {
                if !doc.cells.contains_key(&cell) {
                    return Err(missing(cell));
                }
                let mut previous = Vec::new();
                if let Some(grid) = &grid {
                    let count = (grid.xs as usize).checked_mul(grid.ys as usize);
                    if grid.xs == 0
                        || grid.ys == 0
                        || count != Some(grid.cells.len())
                        || grid.col_widths.len() != grid.xs as usize
                    {
                        return refused(format!("cell {cell}: a grid of the wrong size"));
                    }
                    previous = set_parents(doc, &grid.cells, cell)?;
                }
                let c = doc.cells.get_mut(&cell).ok_or_else(|| missing(cell))?;
                seen.grew(size::grid(grid.as_ref()));
                let old = mem::replace(&mut c.grid, grid);
                seen.shrank(size::grid(old.as_ref()));
                // Whether the properties of the grid changed is known only at the end: the
                // grid may be taken away by one step and another put there by a later one.
                seen.first_grids
                    .entry(cell)
                    .or_insert_with(|| old.as_ref().map(grid_properties));
                seen.grids.insert(cell);
                Ok(giving_back(Step::SetGrid { cell, grid: old }, previous))
            }
            Step::Swap { a, b } => {
                let parent = |id: CellId| {
                    doc.cells
                        .get(&id)
                        .ok_or_else(|| missing(id))?
                        .parent
                        .ok_or_else(|| Error::Refused("the root cell cannot be moved".into()))
                };
                let (pa, pb) = (parent(a)?, parent(b)?);
                let place = |owner: CellId, id: CellId| {
                    grid_of(doc, owner)?
                        .cells
                        .iter()
                        .position(|c| *c == id)
                        .ok_or_else(|| Error::Refused(format!("cell {id} is not in its grid")))
                };
                let (ia, ib) = (place(pa, a)?, place(pb, b)?);
                if let Some(slot) = grid_of_mut(doc, pa)?.cells.get_mut(ia) {
                    *slot = b;
                }
                if let Some(slot) = grid_of_mut(doc, pb)?.cells.get_mut(ib) {
                    *slot = a;
                }
                if let Some(cell) = doc.cells.get_mut(&a) {
                    cell.parent = Some(pb);
                }
                if let Some(cell) = doc.cells.get_mut(&b) {
                    cell.parent = Some(pa);
                }
                seen.grids.insert(pa);
                seen.grids.insert(pb);
                Ok(Step::Swap { a, b })
            }
            Step::Reparented { step, parents } => {
                if let Some((id, _)) = parents.iter().find(|(id, _)| !doc.cells.contains_key(id)) {
                    return Err(missing(*id));
                }
                // Undoing this takes the cells again, which is what the inner step's
                // inverse does: the cells are in the grid or lines it puts back.
                let inverse = step.apply(doc, seen)?;
                for (id, parent) in parents {
                    if let Some(cell) = doc.cells.get_mut(&id) {
                        cell.parent = parent;
                    }
                }
                Ok(inverse)
            }
        }
    }
}
