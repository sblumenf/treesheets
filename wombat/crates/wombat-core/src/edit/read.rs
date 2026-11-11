//! Reading for an interface: one cell and the cells of its grid, with what each shows.
//!
//! An interface draws one grid at a time and needs, for each cell in it, the text, what a
//! formula cell shows instead, and whether there is a grid to go into. [`Editor::grid_view`]
//! gives that as plain data, so that no interface has to put it together from the document
//! and the results itself.

use crate::error::Error;
use crate::id::CellId;
use crate::model::{Cell, CellKind};

use super::Editor;

/// One cell as an interface lists it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CellView {
    /// The cell.
    pub id: CellId,
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

/// A cell and the cells of its grid: what [`Editor::grid_view`] gives.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GridView {
    /// The cell itself.
    pub cell: CellView,
    /// The number of columns, 0 for a cell without a grid.
    pub xs: u32,
    /// The number of rows, 0 for a cell without a grid.
    pub ys: u32,
    /// The cells of the grid row by row; empty for a cell without a grid.
    pub cells: Vec<CellView>,
}

impl Editor {
    fn cell_view(&self, cell: &Cell) -> CellView {
        let result = match cell.kind {
            CellKind::Formula => self.calc.result(cell.id),
            _ => None,
        };
        CellView {
            id: cell.id,
            text: cell.text.clone(),
            kind: cell.kind,
            result: result.map(|(text, _)| text.to_string()),
            failed: result.is_some_and(|(_, failed)| failed),
            has_grid: cell.grid.is_some(),
        }
    }

    /// The cell `id` and the cells of its grid, each with what it shows. A cell without a
    /// grid gives a view of 0 by 0 cells. Fails only if there is no such cell.
    pub fn grid_view(&self, id: CellId) -> Result<GridView, Error> {
        let missing = |id: CellId| Error::NotFound(format!("cell {id}"));
        let cell = self.doc.cell(id).ok_or_else(|| missing(id))?;
        let (mut xs, mut ys, mut cells) = (0, 0, Vec::new());
        if let Some(grid) = cell.grid() {
            (xs, ys) = (grid.xs(), grid.ys());
            cells.reserve(grid.cells().len());
            for child in grid.cells() {
                let child = self.doc.cell(*child).ok_or_else(|| missing(*child))?;
                cells.push(self.cell_view(child));
            }
        }
        Ok(GridView {
            cell: self.cell_view(cell),
            xs,
            ys,
            cells,
        })
    }
}
