//! `Command::Import`: a table of texts read from a file becomes the grid of a cell
//! (milestone 7). An ordinary command: steps, undo, change report, limits.

use crate::convert::Table;
use crate::error::Error;
use crate::id::CellId;
use crate::limits::MAX_DEPTH;
use crate::model::{Cell, CellKind, Grid};

use super::command::{cell, check_room, depth, descendants, fresh_grid, new_cell, Fresh};
use super::step::Step;
use super::Tx;

/// Gives `id` the table as its grid, in place of the grid it may have. The new cells take
/// the looks of `id`, as the cells of a new grid do.
pub(super) fn import(
    tx: &mut Tx,
    id: CellId,
    table: &Table,
    now: Option<i64>,
    fresh: &mut Fresh,
) -> Result<(), Error> {
    let doc = tx.doc();
    let owner = cell(doc, id)?;
    let replaced = descendants(doc, id);
    check_room(tx, table.count.saturating_sub(replaced.len()))?;
    if depth(doc, id)?.saturating_add(table.height) > MAX_DEPTH {
        return Err(Error::Limit(format!(
            "grids nested deeper than {MAX_DEPTH}"
        )));
    }
    let mut cells: Vec<Cell> = Vec::with_capacity(table.count);
    // Each table still to make, with the cell it goes into; `None` is the top one.
    let mut todo: Vec<(&Table, Option<usize>)> = vec![(table, None)];
    let mut top = None;
    while let Some((table, place)) = todo.pop() {
        let parent = match place {
            Some(at) => cells.get(at).map_or(id, Cell::id),
            None => id,
        };
        let mut ids = Vec::with_capacity(table.cells.len());
        for entry in &table.cells {
            let mut new = new_cell(fresh.next(doc)?, parent, owner, now);
            new.text.clone_from(&entry.text);
            if entry.formula {
                new.kind = CellKind::Formula;
            }
            ids.push(new.id());
            if let Some(inner) = &entry.grid {
                todo.push((inner, Some(cells.len())));
            }
            cells.push(new);
        }
        // Every grid of the import looks like a new grid of the cell would.
        let mut grid = fresh_grid(doc, id, table.xs, table.ys, ids);
        match place.and_then(|at| cells.get_mut(at)) {
            Some(inside) => {
                grid.col_widths = vec![Grid::DEFAULT_COL_WIDTH; table.xs as usize];
                inside.grid = Some(grid);
            }
            None => top = Some(grid),
        }
    }
    let had_grid = owner.grid().is_some();
    tx.step(Step::AddCells(cells))?;
    tx.step(Step::SetGrid {
        cell: id,
        grid: top,
    })?;
    if had_grid {
        tx.step(Step::RemoveCells(replaced))?;
    }
    Ok(())
}
