//! What each [`Command`] does: checks against the document, then steps.
//!
//! A function here may return an error at any point; the editor undoes the steps already
//! applied. Semantics follow TreeSheets (`src/text.h`, `src/textruns.h`, `src/grid.h`,
//! `src/cell.h`); where they differ on purpose, `docs/DECISIONS.md` section B says so.

use std::collections::{HashMap, HashSet};

use crate::error::{refused, Error};
use crate::id::{CellId, IdSource};
use crate::limits::{MAX_CELLS, MAX_DEPTH, MAX_TEXT_CODE_POINTS};
use crate::model::{
    code_points, is_valid_image_name, Cell, CellKind, Document, Grid, LinkTarget, Run, Style, Tag,
};
use crate::text;

use super::size::CELL_BYTES;
use super::step::{Axis, Field, Patch, Step};
use super::{restructure, Command, Prop, Tx, MIN_COL_WIDTH};

pub(super) fn cell(doc: &Document, id: CellId) -> Result<&Cell, Error> {
    doc.cell(id)
        .ok_or_else(|| Error::NotFound(format!("cell {id}")))
}

pub(super) fn grid(doc: &Document, id: CellId) -> Result<&Grid, Error> {
    cell(doc, id)?
        .grid()
        .ok_or_else(|| Error::Refused(format!("cell {id} has no grid")))
}

pub(super) fn set(tx: &mut Tx, cell: CellId, field: Field) -> Result<(), Error> {
    tx.step(Step::Set { cell, field })
}

/// Stamps the last-edit time, if the caller gave one.
pub(super) fn stamp(tx: &mut Tx, id: CellId, now: Option<i64>) -> Result<(), Error> {
    if now.is_some() && cell(tx.doc(), id)?.last_edit != now {
        set(tx, id, Field::LastEdit(now))?;
    }
    Ok(())
}

pub(super) fn len32(text: &str) -> Result<u32, Error> {
    u32::try_from(code_points(text)).map_err(|_| Error::Limit("text too long".into()))
}

/// Checks that `start..start + len` is inside the text of `c` and cuts no user-perceived
/// character; returns the end of the range.
pub(super) fn check_range(c: &Cell, start: u32, len: u32) -> Result<u32, Error> {
    let id = c.id();
    let end = start.checked_add(len);
    let inside = |pos: u32| text::is_boundary(&c.text, pos);
    match end.and_then(|end| Some((inside(start)?, inside(end)?, end))) {
        None => refused(format!(
            "cell {id}: {start}, length {len} is outside the text"
        )),
        Some((true, true, end)) => Ok(end),
        Some(_) => refused(format!(
            "cell {id}: {start}, length {len} would cut a character made of several code points"
        )),
    }
}

/// The steps that put new runs and links in place, where they differ from the old.
pub(super) fn set_runs_links(
    tx: &mut Tx,
    id: CellId,
    runs: Option<Vec<Run>>,
    links: Option<Vec<crate::model::Link>>,
) -> Result<(), Error> {
    if let Some(runs) = runs.filter(|runs| cell(tx.doc(), id).is_ok_and(|c| c.runs != *runs)) {
        set(tx, id, Field::Runs(runs))?;
    }
    if let Some(links) = links.filter(|links| cell(tx.doc(), id).is_ok_and(|c| c.links != *links)) {
        set(tx, id, Field::Links(links))?;
    }
    Ok(())
}

pub(super) fn replace_text(
    tx: &mut Tx,
    id: CellId,
    start: u32,
    len: u32,
    new: &str,
    now: Option<i64>,
) -> Result<(), Error> {
    let c = cell(tx.doc(), id)?;
    check_range(c, start, len)?;
    let ins = text::clean(new);
    let count = len32(&ins)?;
    if len == 0 && count == 0 {
        return Ok(());
    }
    let after = code_points(&c.text) - len as usize + count as usize;
    if after > MAX_TEXT_CODE_POINTS {
        return Err(Error::Limit(format!(
            "cell {id}: text longer than {MAX_TEXT_CODE_POINTS} code points"
        )));
    }
    let (mut runs, mut links) = (c.runs.clone(), c.links.clone());
    text::runs_replace(&mut runs, start, len, count, c.style);
    text::links_replace(&mut links, start, len, count);
    let old = text::byte_offset(&c.text, start)
        .zip(text::byte_offset(&c.text, start.saturating_add(len)))
        .and_then(|(from, to)| c.text.get(from..to));
    if old == Some(ins.as_str()) && runs == c.runs && links == c.links {
        // Replacing text by the same text: nothing to do, nothing to undo.
        return Ok(());
    }
    // Only the runs and links that the edit touches go into the history, not all of them.
    let shift = i64::from(count) - i64::from(len);
    let runs = Patch::between(&c.runs, &runs, shift);
    let links = Patch::between(&c.links, &links, shift);
    tx.step(Step::Splice {
        cell: id,
        at: start,
        del: len,
        ins,
    })?;
    if let Some(patch) = runs {
        tx.step(Step::PatchRuns { cell: id, patch })?;
    }
    if let Some(patch) = links {
        tx.step(Step::PatchLinks { cell: id, patch })?;
    }
    stamp(tx, id, now)
}

pub(super) fn set_text(tx: &mut Tx, id: CellId, new: &str, now: Option<i64>) -> Result<(), Error> {
    let c = cell(tx.doc(), id)?;
    let ins = text::clean(new);
    if code_points(&ins) > MAX_TEXT_CODE_POINTS {
        return Err(Error::Limit(format!(
            "cell {id}: text longer than {MAX_TEXT_CODE_POINTS} code points"
        )));
    }
    if c.text == ins && c.runs.is_empty() && c.links.is_empty() {
        return Ok(());
    }
    let del = len32(&c.text)?;
    tx.step(Step::Splice {
        cell: id,
        at: 0,
        del,
        ins,
    })?;
    set_runs_links(tx, id, Some(Vec::new()), Some(Vec::new()))?;
    stamp(tx, id, now)
}

fn with_bits(style: Style, set: Style, clear: Style) -> Style {
    let bit = |old: bool, on: bool, off: bool| (old || on) && !off;
    Style {
        bold: bit(style.bold, set.bold, clear.bold),
        italic: bit(style.italic, set.italic, clear.italic),
        fixed: bit(style.fixed, set.fixed, clear.fixed),
        underline: bit(style.underline, set.underline, clear.underline),
        strike: bit(style.strike, set.strike, clear.strike),
    }
}

fn modify_runs(
    tx: &mut Tx,
    id: CellId,
    start: u32,
    len: u32,
    change: impl Fn(&mut Run),
) -> Result<(), Error> {
    let c = cell(tx.doc(), id)?;
    let end = check_range(c, start, len)?;
    let mut runs = c.runs.clone();
    text::runs_modify(&mut runs, start, end, c.style, change);
    set_runs_links(tx, id, Some(runs), None)
}

fn set_link(
    tx: &mut Tx,
    id: CellId,
    start: u32,
    len: u32,
    target: Option<LinkTarget>,
) -> Result<(), Error> {
    let c = cell(tx.doc(), id)?;
    let end = check_range(c, start, len)?;
    match &target {
        Some(LinkTarget::External(url)) if url.is_empty() => {
            return refused("a link needs a target that is not empty");
        }
        Some(LinkTarget::Unknown) => {
            return refused("a link needs a target: an external one or a cell");
        }
        _ => {}
    }
    // One form for a link to a cell of this document: without the document (A47).
    let own = tx.doc().id();
    let target = target.map(|target| match target {
        LinkTarget::Cell { doc, cell } => LinkTarget::Cell {
            doc: doc.filter(|d| *d != own),
            cell,
        },
        other => other,
    });
    let mut links = c.links.clone();
    text::links_set(&mut links, start, end, target);
    set_runs_links(tx, id, None, Some(links))
}

/// The properties of a cell as they are after `props`, in a cell that has no text and
/// whose grid has no cells (they are left out so that a long text or a large grid costs
/// nothing here). The last-edit time is stamped if a colour really differs at the end.
fn with_props(c: &Cell, props: &[Prop], now: Option<i64>) -> Result<Cell, Error> {
    let id = c.id();
    let mut want = Cell::new(id, c.parent());
    want.kind = c.kind;
    want.cell_color = c.cell_color;
    want.text_color = c.text_color;
    want.draw_style = c.draw_style;
    want.note.clone_from(&c.note);
    want.text_align = c.text_align;
    want.vert_align = c.vert_align;
    want.rel_size = c.rel_size;
    want.style = c.style;
    want.runs.clone_from(&c.runs);
    want.links.clone_from(&c.links);
    want.last_edit = c.last_edit;
    want.image.clone_from(&c.image);
    want.extra.clone_from(&c.extra);
    want.grid = c.grid().map(|g| {
        let mut bare = Grid::new(g.xs(), g.ys(), Vec::new());
        bare.border_color = g.border_color;
        bare.outer_spacing = g.outer_spacing;
        bare.vertical = g.vertical;
        bare.col_widths.clone_from(&g.col_widths);
        bare.extra.clone_from(&g.extra);
        bare
    });
    let no_grid = || Error::Refused(format!("cell {id} has no grid"));
    // Whether a time set with `Prop::LastEdit` comes after the last colour, and so wins.
    let mut timed = false;
    for prop in props {
        match prop {
            Prop::CellColor(v) => {
                want.cell_color = *v;
                timed = false;
            }
            Prop::TextColor(v) => {
                want.text_color = *v;
                want.runs = text::runs_without_colors(&want.runs, want.style);
                timed = false;
            }
            Prop::RelSize(v) => want.rel_size = *v,
            Prop::TextAlign(v) => want.text_align = *v,
            Prop::VertAlign(v) => want.vert_align = *v,
            Prop::Kind(v) => want.kind = *v,
            Prop::DrawStyle(v) => want.draw_style = *v,
            Prop::LastEdit(v) => {
                want.last_edit = *v;
                timed = true;
            }
            Prop::Note(v) => {
                if code_points(v) > MAX_TEXT_CODE_POINTS {
                    return Err(Error::Limit(format!(
                        "cell {id}: note longer than {MAX_TEXT_CODE_POINTS} code points"
                    )));
                }
                want.note.clone_from(v);
            }
            Prop::Image(v) => {
                if let Some(image) = v {
                    check_image(id, &image.file, image.scale)?;
                    crate::model::check_extra(
                        &image.extra,
                        crate::format::IMAGE_KEYS,
                        &format!("the image of cell {id}"),
                    )?;
                }
                want.image.clone_from(v);
            }
            Prop::ImageScale(v) => {
                let Some(image) = &mut want.image else {
                    return refused(format!("cell {id} has no image"));
                };
                check_image(id, &image.file, *v)?;
                image.scale = *v;
            }
            Prop::BorderColor(v) => want.grid.as_mut().ok_or_else(no_grid)?.border_color = *v,
            Prop::OuterSpacing(v) => want.grid.as_mut().ok_or_else(no_grid)?.outer_spacing = *v,
            Prop::Vertical(v) => want.grid.as_mut().ok_or_else(no_grid)?.vertical = *v,
        }
    }
    let recoloured =
        (want.cell_color, want.text_color) != (c.cell_color, c.text_color) || want.runs != c.runs;
    if recoloured && !timed && now.is_some() {
        want.last_edit = now;
    }
    Ok(want)
}

/// Sets properties of one cell: the steps for what differs once all of `props` are
/// applied, so that a property set and set back is no change (decision A44).
fn set_props(tx: &mut Tx, id: CellId, props: &[Prop], now: Option<i64>) -> Result<(), Error> {
    let want = with_props(cell(tx.doc(), id)?, props, now)?;
    sync_props(tx, id, &want)?;
    if let Some(want) = &want.grid {
        sync_grid_props(tx, id, want)?;
    }
    Ok(())
}

fn check_image(id: CellId, file: &str, scale: f64) -> Result<(), Error> {
    if !is_valid_image_name(file) {
        return refused(format!(
            "cell {id}: {file:?} is not a valid image file name"
        ));
    }
    if !(scale.is_finite() && scale > 0.0) {
        return refused(format!(
            "cell {id}: the image scale is not a positive number"
        ));
    }
    Ok(())
}

fn set_style(tx: &mut Tx, id: CellId, on: Style, off: Style) -> Result<(), Error> {
    let c = cell(tx.doc(), id)?;
    let base = with_bits(c.style, on, off);
    let mut runs = c.runs.clone();
    for run in &mut runs {
        run.style = with_bits(run.style, on, off);
    }
    text::normalize_runs(&mut runs, base);
    if c.style != base {
        set(tx, id, Field::Style(base))?;
    }
    set_runs_links(tx, id, Some(runs), None)
}

/// The cells inside `id` (not `id` itself), in document order.
pub(super) fn descendants(doc: &Document, id: CellId) -> Vec<CellId> {
    let mut out = Vec::new();
    let mut stack: Vec<CellId> = Vec::new();
    let push = |stack: &mut Vec<CellId>, id: CellId| {
        if let Some(grid) = doc.cell(id).and_then(Cell::grid) {
            stack.extend(grid.cells().iter().rev());
        }
    };
    push(&mut stack, id);
    while let Some(next) = stack.pop() {
        out.push(next);
        push(&mut stack, next);
    }
    out
}

/// How many levels of grids are inside `id`: 0 for a cell without a grid.
pub(super) fn height(doc: &Document, id: CellId) -> u32 {
    let mut deepest = 0;
    let mut stack = vec![(id, 0u32)];
    while let Some((next, level)) = stack.pop() {
        deepest = deepest.max(level);
        if let Some(grid) = doc.cell(next).and_then(Cell::grid) {
            stack.extend(grid.cells().iter().map(|c| (*c, level.saturating_add(1))));
        }
    }
    deepest
}

pub(super) fn depth(doc: &Document, id: CellId) -> Result<u32, Error> {
    doc.depth(id)
        .ok_or_else(|| Error::NotFound(format!("cell {id}")))
}

/// Whether `inner` is `outer` or inside it.
fn is_inside(doc: &Document, inner: CellId, outer: CellId) -> bool {
    let mut at = Some(inner);
    while let Some(id) = at {
        if id == outer {
            return true;
        }
        at = doc.cell(id).and_then(Cell::parent);
    }
    false
}

pub(super) fn check_depth(doc: &Document, place: CellId, content: CellId) -> Result<(), Error> {
    if depth(doc, place)?.saturating_add(height(doc, content)) > MAX_DEPTH {
        return Err(Error::Limit(format!(
            "grids nested deeper than {MAX_DEPTH}"
        )));
    }
    Ok(())
}

/// Checks, before they are made, that `more` cells can be added: within the limit on
/// cells, and not more than a file can hold (whether the document as a whole can still be
/// saved is checked when the command is complete).
pub(super) fn check_room(tx: &Tx, more: usize) -> Result<(), Error> {
    match tx.doc().cell_count().checked_add(more) {
        Some(total) if total <= MAX_CELLS => {}
        _ => return Err(Error::Limit(format!("more than {MAX_CELLS} cells"))),
    }
    if (more as u64).saturating_mul(CELL_BYTES) > tx.size_limit {
        return Err(Error::Limit(format!(
            "{more} more cells would make the document too large to save (over {} bytes)",
            tx.size_limit
        )));
    }
    Ok(())
}

/// Hands out IDs for new cells, refusing a source that repeats itself.
pub(super) struct Fresh<'a> {
    ids: &'a mut dyn IdSource,
    given: HashSet<CellId>,
    /// The IDs of cells that are not in the document but that undo or redo can bring back.
    held: &'a HashSet<CellId>,
}

impl Fresh<'_> {
    pub(super) fn next(&mut self, doc: &Document) -> Result<CellId, Error> {
        let id = CellId(self.ids.next_id());
        if id.0 == doc.id().0 || doc.cell(id).is_some() || !self.given.insert(id) {
            return refused(format!("the ID source repeated the ID {id}"));
        }
        if self.held.contains(&id) {
            return refused(format!(
                "the ID source gave the ID {id} of a cell that undo or redo can bring back"
            ));
        }
        Ok(id)
    }
}

/// A new empty cell that takes after `like`, as TreeSheets' new cells do.
pub(super) fn new_cell(id: CellId, parent: CellId, like: &Cell, now: Option<i64>) -> Cell {
    let mut c = Cell::new(id, Some(parent));
    c.cell_color = like.cell_color;
    c.text_color = like.text_color;
    c.draw_style = like.draw_style;
    c.text_align = like.text_align;
    c.vert_align = like.vert_align;
    c.style = like.style;
    c.rel_size = like.rel_size;
    c.last_edit = now;
    c
}

/// The width of the column that `id` is in, which TreeSheets gives to the columns of a
/// grid made or widened inside `id`.
pub(super) fn col_width_of(doc: &Document, id: CellId) -> u32 {
    let width = || {
        let parent = doc.cell(doc.cell(id)?.parent()?)?.grid()?;
        let index = parent.cells().iter().position(|c| *c == id)?;
        parent
            .col_widths
            .get(index % parent.xs().max(1) as usize)
            .copied()
    };
    width().unwrap_or(Grid::DEFAULT_COL_WIDTH)
}

/// A grid for the cell `id`, which has none, as TreeSheets' `Cell::AddGrid` makes it:
/// columns as wide as the column the cell is in, border colour and spacing (and here the
/// vertical flag, decision B24) of the grid around the cell.
pub(super) fn fresh_grid(doc: &Document, id: CellId, xs: u32, ys: u32, cells: Vec<CellId>) -> Grid {
    let mut new = Grid::new(xs, ys, cells);
    new.col_widths = vec![col_width_of(doc, id); xs as usize];
    let around = doc
        .cell(id)
        .and_then(Cell::parent)
        .and_then(|p| doc.cell(p))
        .and_then(Cell::grid);
    if let Some(around) = around {
        new.border_color = around.border_color;
        new.outer_spacing = around.outer_spacing;
        new.vertical = around.vertical;
    }
    new
}

fn create_grid(
    tx: &mut Tx,
    id: CellId,
    xs: u32,
    ys: u32,
    now: Option<i64>,
    fresh: &mut Fresh,
) -> Result<(), Error> {
    let doc = tx.doc();
    let owner = cell(doc, id)?;
    if owner.grid().is_some() {
        return refused(format!("cell {id} already has a grid"));
    }
    if xs == 0 || ys == 0 {
        return refused("a grid has at least one column and one row");
    }
    if depth(doc, id)? >= MAX_DEPTH {
        return Err(Error::Limit(format!(
            "grids nested deeper than {MAX_DEPTH}"
        )));
    }
    let count = (xs as usize)
        .checked_mul(ys as usize)
        .ok_or_else(|| Error::Limit(format!("more than {MAX_CELLS} cells")))?;
    check_room(tx, count)?;
    let mut cells = Vec::with_capacity(count);
    for _ in 0..count {
        cells.push(new_cell(fresh.next(doc)?, id, owner, now));
    }
    let new = fresh_grid(doc, id, xs, ys, cells.iter().map(Cell::id).collect());
    tx.step(Step::AddCells(cells))?;
    tx.step(Step::SetGrid {
        cell: id,
        grid: Some(new),
    })
}

/// The root is not a cell that can be selected in TreeSheets, so nothing there clears it,
/// pastes over it or deletes its grid; here those are refused (decision B33).
fn not_the_root(doc: &Document, id: CellId, what: &str) -> Result<(), Error> {
    if id == doc.root() {
        return refused(format!("the root cell cannot be {what}"));
    }
    Ok(())
}

fn remove_grid(tx: &mut Tx, id: CellId) -> Result<(), Error> {
    grid(tx.doc(), id)?;
    not_the_root(tx.doc(), id, "left without its grid")?;
    let inside = descendants(tx.doc(), id);
    tx.step(Step::SetGrid {
        cell: id,
        grid: None,
    })?;
    tx.step(Step::RemoveCells(inside))
}

fn insert_lines(
    tx: &mut Tx,
    id: CellId,
    axis: Axis,
    at: u32,
    count: u32,
    now: Option<i64>,
    fresh: &mut Fresh,
) -> Result<(), Error> {
    let doc = tx.doc();
    let g = grid(doc, id)?;
    let (along, across) = match axis {
        Axis::Rows => (g.ys(), g.xs()),
        Axis::Columns => (g.xs(), g.ys()),
    };
    if count == 0 {
        return Ok(());
    }
    if at > along {
        return refused(format!("cell {id}: cannot insert at {at} of {along}"));
    }
    let total = (count as usize)
        .checked_mul(across as usize)
        .filter(|_| along.checked_add(count).is_some())
        .ok_or_else(|| Error::Limit(format!("more than {MAX_CELLS} cells")))?;
    check_room(tx, total)?;
    // New cells take after the line before them, or after the first line when inserted
    // in front of it.
    let source = at.saturating_sub(1);
    let like = |x: u32, y: u32| {
        g.at(x, y)
            .and_then(|c| doc.cell(c))
            .ok_or_else(|| Error::Refused(format!("cell {id}: the grid is damaged")))
    };
    let mut cells = Vec::with_capacity(total);
    match axis {
        Axis::Rows => {
            for _ in 0..count {
                for x in 0..across {
                    cells.push(new_cell(fresh.next(doc)?, id, like(x, source)?, now));
                }
            }
        }
        Axis::Columns => {
            for y in 0..across {
                for _ in 0..count {
                    cells.push(new_cell(fresh.next(doc)?, id, like(source, y)?, now));
                }
            }
        }
    }
    let widths = match axis {
        Axis::Rows => Vec::new(),
        Axis::Columns => vec![col_width_of(doc, id); count as usize],
    };
    let ids = cells.iter().map(Cell::id).collect();
    tx.step(Step::AddCells(cells))?;
    tx.step(Step::InsertLines {
        owner: id,
        axis,
        at,
        ids,
        widths,
    })
}

/// The cells of `count` rows or columns from `at` on, row by row.
fn line_cells(g: &Grid, axis: Axis, at: u32, count: u32) -> Vec<CellId> {
    let end = at.saturating_add(count);
    let mut out = Vec::new();
    for y in 0..g.ys() {
        for x in 0..g.xs() {
            let index = match axis {
                Axis::Rows => y,
                Axis::Columns => x,
            };
            if index >= at && index < end {
                out.extend(g.at(x, y));
            }
        }
    }
    out
}

fn delete_lines(tx: &mut Tx, id: CellId, axis: Axis, at: u32, count: u32) -> Result<(), Error> {
    let doc = tx.doc();
    let g = grid(doc, id)?;
    let along = match axis {
        Axis::Rows => g.ys(),
        Axis::Columns => g.xs(),
    };
    if count == 0 {
        return Ok(());
    }
    if at.checked_add(count).is_none_or(|end| end > along) {
        return refused(format!(
            "cell {id}: cannot delete {count} from {at} of {along}"
        ));
    }
    if count == along {
        return remove_grid(tx, id);
    }
    let mut gone = Vec::new();
    for line in line_cells(g, axis, at, count) {
        gone.push(line);
        gone.extend(descendants(doc, line));
    }
    tx.step(Step::RemoveLines {
        owner: id,
        axis,
        at,
        count,
    })?;
    tx.step(Step::RemoveCells(gone))
}

/// TreeSheets' `Cell::Clear`.
pub(super) fn clear_cell(tx: &mut Tx, id: CellId) -> Result<(), Error> {
    if cell(tx.doc(), id)?.grid().is_some() {
        remove_grid(tx, id)?;
    }
    clear_own(tx, id)
}

/// `Cell::Clear` without the grid: text, runs, links and image go, a formula cell becomes a
/// data cell.
pub(super) fn clear_own(tx: &mut Tx, id: CellId) -> Result<(), Error> {
    let c = cell(tx.doc(), id)?;
    let (has_image, has_text) = (c.image.is_some(), !c.text.is_empty());
    let del = len32(&c.text)?;
    if c.kind == CellKind::Formula {
        set(tx, id, Field::Kind(CellKind::Data))?;
    }
    if has_text {
        tx.step(Step::Splice {
            cell: id,
            at: 0,
            del,
            ins: String::new(),
        })?;
    }
    set_runs_links(tx, id, Some(Vec::new()), Some(Vec::new()))?;
    if has_image {
        set(tx, id, Field::Image(None))?;
    }
    Ok(())
}

pub(super) fn clear_cells(tx: &mut Tx, cells: &[CellId]) -> Result<(), Error> {
    for id in cells {
        cell(tx.doc(), *id)?;
        not_the_root(tx.doc(), *id, "cleared")?;
    }
    for id in cells {
        // A cell named after a cell it is inside of is gone with that cell's grid.
        if tx.doc().cell(*id).is_some() {
            clear_cell(tx, *id)?;
        }
    }
    Ok(())
}

pub(super) fn has_content(c: &Cell) -> bool {
    !c.text.is_empty() || c.image.is_some() || c.grid().is_some()
}

/// TreeSheets' `Grid::MultiCellDeleteSub`.
pub(super) fn delete_cells(
    tx: &mut Tx,
    id: CellId,
    x: u32,
    y: u32,
    xs: u32,
    ys: u32,
) -> Result<(), Error> {
    let g = grid(tx.doc(), id)?;
    let (gxs, gys) = (g.xs(), g.ys());
    let inside = |from: u32, size: u32, all: u32| {
        size > 0 && from.checked_add(size).is_some_and(|end| end <= all)
    };
    if !inside(x, xs, gxs) || !inside(y, ys, gys) {
        return refused(format!(
            "cell {id}: no cells {xs} by {ys} at {x},{y} in a grid of {gxs} by {gys}"
        ));
    }
    let mut selected = Vec::new();
    for cy in y..y + ys {
        for cx in x..x + xs {
            selected.extend(g.at(cx, cy));
        }
    }
    clear_cells(tx, &selected)?;
    let doc = tx.doc();
    let g = grid(doc, id)?;
    let (mut rows_empty, mut columns_empty) = (true, true);
    for cy in 0..gys {
        for cx in 0..gxs {
            if g.at(cx, cy)
                .and_then(|c| doc.cell(c))
                .is_some_and(has_content)
            {
                if cy >= y && cy < y + ys {
                    rows_empty = false;
                }
                if cx >= x && cx < x + xs {
                    columns_empty = false;
                }
            }
        }
    }
    // If both are possible, prefer the direction that does not delete the whole grid.
    if rows_empty && columns_empty && (xs == gxs) != (ys == gys) {
        rows_empty = xs == gxs;
        columns_empty = !rows_empty;
    }
    let (axis, at, count, all) = if rows_empty && (!columns_empty || xs >= ys) {
        (Axis::Rows, y, ys, gys)
    } else if columns_empty {
        (Axis::Columns, x, xs, gxs)
    } else {
        return Ok(());
    };
    if count == all && id == doc.root() {
        // TreeSheets leaves the root its grid, emptied.
        return Ok(());
    }
    delete_lines(tx, id, axis, at, count)
}

fn move_lines(
    tx: &mut Tx,
    id: CellId,
    axis: Axis,
    from: u32,
    count: u32,
    to: u32,
) -> Result<(), Error> {
    let g = grid(tx.doc(), id)?;
    let along = match axis {
        Axis::Rows => g.ys(),
        Axis::Columns => g.xs(),
    };
    let fits = |start: u32| start.checked_add(count).is_some_and(|end| end <= along);
    if !fits(from) || !fits(to) {
        return refused(format!(
            "cell {id}: cannot move {count} from {from} to {to} of {along}"
        ));
    }
    if count == 0 || from == to {
        return Ok(());
    }
    tx.step(Step::MoveLines {
        owner: id,
        axis,
        from,
        count,
        to,
    })
}

fn swap_cells(tx: &mut Tx, a: CellId, b: CellId) -> Result<(), Error> {
    let doc = tx.doc();
    let (ca, cb) = (cell(doc, a)?, cell(doc, b)?);
    if a == b {
        return Ok(());
    }
    if ca.parent().is_none() || cb.parent().is_none() {
        return refused("the root cell cannot be moved");
    }
    if is_inside(doc, a, b) || is_inside(doc, b, a) {
        return refused(format!(
            "cells {a} and {b}: one is inside the other, so they cannot trade places"
        ));
    }
    check_depth(doc, a, b)?;
    check_depth(doc, b, a)?;
    tx.step(Step::Swap { a, b })
}

fn move_cell(tx: &mut Tx, from: CellId, to: CellId) -> Result<(), Error> {
    let doc = tx.doc();
    cell(doc, from)?;
    if has_content(cell(doc, to)?) {
        return refused(format!("cell {to} is not empty"));
    }
    if from == to {
        return refused("a cell cannot be moved onto itself");
    }
    if is_inside(doc, to, from) {
        return refused(format!(
            "cell {to} is inside cell {from}, which cannot be moved into itself"
        ));
    }
    swap_cells(tx, from, to)
}

/// The steps that make the own content of the cell `id` (its text and every property, but
/// not its grid or its place) equal to that of `like`.
pub(super) fn sync_cell(tx: &mut Tx, id: CellId, like: &Cell) -> Result<(), Error> {
    let target = cell(tx.doc(), id)?;
    if target.text != like.text {
        let del = len32(&target.text)?;
        tx.step(Step::Splice {
            cell: id,
            at: 0,
            del,
            ins: like.text.clone(),
        })?;
    }
    sync_props(tx, id, like)
}

/// The steps that make every property of the cell `id` equal to that of `like`: all of its
/// own content but the text.
fn sync_props(tx: &mut Tx, id: CellId, like: &Cell) -> Result<(), Error> {
    let target = cell(tx.doc(), id)?;
    let fields = [
        (target.kind != like.kind).then_some(Field::Kind(like.kind)),
        (target.cell_color != like.cell_color).then_some(Field::CellColor(like.cell_color)),
        (target.text_color != like.text_color).then_some(Field::TextColor(like.text_color)),
        (target.draw_style != like.draw_style).then_some(Field::DrawStyle(like.draw_style)),
        (target.note != like.note).then(|| Field::Note(like.note.clone())),
        (target.text_align != like.text_align).then_some(Field::TextAlign(like.text_align)),
        (target.vert_align != like.vert_align).then_some(Field::VertAlign(like.vert_align)),
        (target.rel_size != like.rel_size).then_some(Field::RelSize(like.rel_size)),
        (target.style != like.style).then_some(Field::Style(like.style)),
        (target.runs != like.runs).then(|| Field::Runs(like.runs.clone())),
        (target.links != like.links).then(|| Field::Links(like.links.clone())),
        (target.last_edit != like.last_edit).then_some(Field::LastEdit(like.last_edit)),
        (target.image != like.image).then(|| Field::Image(like.image.clone())),
        (target.extra != like.extra).then(|| Field::Extra(like.extra.clone())),
    ];
    for field in fields.into_iter().flatten() {
        set(tx, id, field)?;
    }
    Ok(())
}

/// The steps that make the properties of the grid of `id` (not its shape or cells) equal
/// to those of `want`.
pub(super) fn sync_grid_props(tx: &mut Tx, id: CellId, want: &Grid) -> Result<(), Error> {
    let have = grid(tx.doc(), id)?;
    let fields = [
        (have.border_color != want.border_color).then_some(Field::BorderColor(want.border_color)),
        (have.outer_spacing != want.outer_spacing)
            .then_some(Field::OuterSpacing(want.outer_spacing)),
        (have.vertical != want.vertical).then_some(Field::Vertical(want.vertical)),
        (have.col_widths != want.col_widths).then(|| Field::ColWidths(want.col_widths.clone())),
    ];
    for field in fields.into_iter().flatten() {
        set(tx, id, field)?;
    }
    Ok(())
}

fn copy_cell(tx: &mut Tx, from: CellId, to: CellId, fresh: &mut Fresh) -> Result<(), Error> {
    let doc = tx.doc();
    let source = cell(doc, from)?.clone();
    cell(doc, to)?;
    if from == to {
        return refused("a cell cannot be copied onto itself");
    }
    not_the_root(doc, to, "pasted over")?;
    let inside = descendants(doc, from);
    let replaced = descendants(doc, to).len();
    if doc
        .cell_count()
        .saturating_sub(replaced)
        .saturating_add(inside.len())
        > MAX_CELLS
    {
        return Err(Error::Limit(format!("more than {MAX_CELLS} cells")));
    }
    check_depth(doc, to, from)?;
    // Which cell of the original becomes which cell of the copy.
    let mut renamed: HashMap<CellId, CellId> = HashMap::with_capacity(inside.len() + 1);
    renamed.insert(from, to);
    for old in &inside {
        renamed.insert(*old, fresh.next(doc)?);
    }
    let rename = |c: &mut Cell| {
        for link in &mut c.links {
            if let LinkTarget::Cell { doc: None, cell } = &mut link.target {
                if let Some(new) = renamed.get(cell) {
                    *cell = *new;
                }
            }
        }
        if let Some(grid) = &mut c.grid {
            for child in &mut grid.cells {
                if let Some(new) = renamed.get(child) {
                    *child = *new;
                }
            }
        }
    };
    let mut copies = Vec::with_capacity(inside.len());
    for old in &inside {
        let mut copy = cell(doc, *old)?.clone();
        if let (Some(id), Some(parent)) =
            (renamed.get(old), copy.parent.and_then(|p| renamed.get(&p)))
        {
            copy.id = *id;
            copy.parent = Some(*parent);
        }
        rename(&mut copy);
        copies.push(copy);
    }
    let mut top = source;
    rename(&mut top);

    let had_grid = cell(tx.doc(), to)?.grid().is_some();
    let replaced = descendants(tx.doc(), to);
    sync_cell(tx, to, &top)?;
    if top.grid.is_some() {
        tx.step(Step::AddCells(copies))?;
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

pub(super) fn set_tag(tx: &mut Tx, name: String, tag: Option<Tag>) -> Result<(), Error> {
    let old = tx.doc().tags().get(&name);
    match &tag {
        None if old.is_none() => {
            return Err(Error::NotFound(format!("tag {name:?}")));
        }
        Some(_) if name.is_empty() || !text::is_clean(&name) => {
            return refused("a tag's name is not empty and has no control characters");
        }
        Some(_) if code_points(&name) > MAX_TEXT_CODE_POINTS => {
            return Err(Error::Limit("a tag name that long".into()));
        }
        Some(new) if old == Some(new) => return Ok(()),
        _ => {}
    }
    tx.step(Step::Tag { name, tag })
}

/// Carries out `command` as steps of `tx`.
pub(super) fn run(
    tx: &mut Tx,
    command: Command,
    now: Option<i64>,
    ids: &mut dyn IdSource,
) -> Result<(), Error> {
    let mut fresh = Fresh {
        ids,
        given: HashSet::new(),
        held: tx.held,
    };
    match command {
        Command::InsertText { cell, at, text } => replace_text(tx, cell, at, 0, &text, now),
        Command::DeleteText { cell, start, len } => replace_text(tx, cell, start, len, "", now),
        Command::ReplaceText {
            cell,
            start,
            len,
            text,
        } => replace_text(tx, cell, start, len, &text, now),
        Command::SetText { cell, text } => set_text(tx, cell, &text, now),
        Command::Run => super::program::run_program(tx, now, &mut fresh),
        Command::EnterText { cell, text } => super::program::enter_text(tx, cell, &text, now),
        Command::SetRangeStyle {
            cell,
            start,
            len,
            set,
            clear,
        } => modify_runs(tx, cell, start, len, |run| {
            run.style = with_bits(run.style, set, clear);
        }),
        Command::SetRangeColor {
            cell,
            start,
            len,
            color,
        } => modify_runs(tx, cell, start, len, text::set_run_color(color)),
        Command::SetLink {
            cell,
            start,
            len,
            target,
        } => set_link(tx, cell, start, len, target),
        Command::SetProps { cells, props } => cells
            .iter()
            .try_for_each(|id| set_props(tx, *id, &props, now)),
        Command::SetStyle { cells, set, clear } => cells
            .iter()
            .try_for_each(|id| set_style(tx, *id, set, clear)),
        Command::SetColumnWidth {
            cell,
            column,
            width,
        } => {
            let g = grid(tx.doc(), cell)?;
            let width = width.max(MIN_COL_WIDTH);
            let mut widths = g.col_widths.clone();
            match widths.get_mut(column as usize) {
                None => refused(format!("cell {cell}: the grid has no column {column}")),
                Some(old) if *old == width => Ok(()),
                Some(old) => {
                    *old = width;
                    set(tx, cell, Field::ColWidths(widths))
                }
            }
        }
        Command::CreateGrid { cell, xs, ys } => create_grid(tx, cell, xs, ys, now, &mut fresh),
        Command::RemoveGrid { cell } => remove_grid(tx, cell),
        Command::InsertRows { cell, at, count } => {
            insert_lines(tx, cell, Axis::Rows, at, count, now, &mut fresh)
        }
        Command::InsertColumns { cell, at, count } => {
            insert_lines(tx, cell, Axis::Columns, at, count, now, &mut fresh)
        }
        Command::DeleteRows { cell, at, count } => delete_lines(tx, cell, Axis::Rows, at, count),
        Command::DeleteColumns { cell, at, count } => {
            delete_lines(tx, cell, Axis::Columns, at, count)
        }
        Command::ClearCells { cells } => clear_cells(tx, &cells),
        Command::DeleteCells { cell, x, y, xs, ys } => delete_cells(tx, cell, x, y, xs, ys),
        Command::MoveRows {
            cell,
            from,
            count,
            to,
        } => move_lines(tx, cell, Axis::Rows, from, count, to),
        Command::MoveColumns {
            cell,
            from,
            count,
            to,
        } => move_lines(tx, cell, Axis::Columns, from, count, to),
        Command::SwapCells { a, b } => swap_cells(tx, a, b),
        Command::MoveCell { from, to } => move_cell(tx, from, to),
        Command::CopyCell { from, to } => copy_cell(tx, from, to, &mut fresh),
        Command::SetTag {
            name,
            cell_color,
            text_color,
        } => {
            let extra = tx
                .doc()
                .tags()
                .get(&name)
                .map(|tag| tag.extra.clone())
                .unwrap_or_default();
            let tag = Tag {
                cell_color,
                text_color,
                extra,
            };
            set_tag(tx, name, Some(tag))
        }
        Command::RemoveTag { name } => set_tag(tx, name, None),
        Command::SortRows {
            cell,
            column,
            row,
            count,
            descending,
        } => restructure::sort_rows(tx, cell, column, row, count, descending),
        Command::Transpose { cells } => restructure::transpose(tx, &cells),
        Command::HierarchySwap { cell } => restructure::hierarchy_swap(tx, cell, now, &mut fresh),
        Command::Hierarchify { cell } => restructure::hierarchify(tx, cell),
        Command::Flatten { cell } => restructure::flatten(tx, cell, now, &mut fresh),
        Command::Wrap { cell, x, y, xs, ys } => {
            restructure::wrap(tx, cell, (x, y, xs, ys), now, &mut fresh)
        }
        Command::Collapse { cell, x, y, xs, ys } => restructure::collapse(tx, cell, (x, y, xs, ys)),
        Command::SplitCell { cell, at } => restructure::split_cell(tx, cell, at, now, &mut fresh),
        Command::TagCells { cells } => restructure::tag_cells(tx, &cells),
        Command::UntagCells { cells } => restructure::untag_cells(tx, &cells),
        Command::AdjustRelSize {
            cells,
            delta,
            recursive,
        } => restructure::adjust_rel_size(tx, &cells, delta, recursive),
        Command::AdjustColumnWidths {
            cell,
            column,
            count,
            delta,
            recursive,
        } => restructure::adjust_column_widths(tx, cell, column, count, delta, recursive),
        Command::ToggleCheck { cells } => restructure::toggle_check(tx, &cells, now),
        Command::PasteCells { to, clip } => super::paste::paste(tx, to, clip, &mut fresh),
        Command::AdoptCells { to, clip } => super::paste::adopt(tx, to, clip),
        Command::ReleaseCell { cell } => super::paste::release(tx, cell, now, &mut fresh),
        Command::Import { cell, table } => super::import::import(tx, cell, &table, now, &mut fresh),
    }
}
