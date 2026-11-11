//! Exports that give one text: a grid as a table, a grid flattened with everything nested
//! in it, and TreeSheets' indented text.

use super::delimited::Writer;
use super::{too_large, ExportOptions, Source, MAX_EXPORT_BYTES};
use crate::error::Error;
use crate::id::CellId;
use crate::limits::MAX_DEPTH;
use crate::model::{Cell, Document, Grid};

pub(super) fn cell_of(doc: &Document, id: CellId) -> Result<&Cell, Error> {
    doc.cell(id)
        .ok_or_else(|| Error::NotFound(format!("cell {id}")))
}

pub(super) fn grid_of(doc: &Document, id: CellId) -> Result<&Grid, Error> {
    cell_of(doc, id)?
        .grid()
        .ok_or_else(|| Error::Refused(format!("cell {id} has no grid to export")))
}

/// Writes what one cell exports as: its text, and with the notes option its note.
pub(super) fn cell_fields(
    out: &mut Writer,
    source: Source,
    id: CellId,
    options: &ExportOptions,
) -> Result<(), Error> {
    out.field(source.text(id, options.formulas))?;
    if options.notes {
        out.field(source.doc.cell(id).map_or("", |c| c.note.as_str()))?;
    }
    Ok(())
}

/// The grid of `cell` as delimited text: one line per row, one field per cell, as
/// TreeSheets' "CSV (flat)" and "TSV (flat)" exports. Of a cell that has a grid of its own
/// only the text is exported; what is nested is left out. Every row and column is there,
/// the empty ones too. Refused for a cell without a grid.
pub fn grid_csv(source: Source, cell: CellId, options: &ExportOptions) -> Result<String, Error> {
    let grid = grid_of(source.doc, cell)?;
    let mut out = Writer::new(options);
    for row in grid.cells().chunks(grid.xs().max(1) as usize) {
        for id in row {
            cell_fields(&mut out, source, *id, options)?;
        }
        out.end_row();
    }
    Ok(out.finish())
}

/// The widest grid at each level of nesting below `top`: level 0 is the grid of `top`.
fn level_widths(doc: &Document, top: CellId) -> Result<Vec<usize>, Error> {
    let mut widths: Vec<usize> = Vec::new();
    let mut stack = vec![(top, 0usize)];
    while let Some((owner, level)) = stack.pop() {
        let Some(grid) = doc.cell(owner).and_then(Cell::grid) else {
            continue;
        };
        if level > MAX_DEPTH as usize {
            return Err(Error::Limit(format!(
                "grids nested deeper than {MAX_DEPTH}"
            )));
        }
        if widths.len() <= level {
            widths.resize(level.saturating_add(1), 0);
        }
        if let Some(width) = widths.get_mut(level) {
            *width = (*width).max(grid.xs() as usize);
        }
        let below = level.saturating_add(1);
        stack.extend(grid.cells().iter().map(|c| (*c, below)));
    }
    Ok(widths)
}

struct Flat<'a> {
    source: Source<'a>,
    options: &'a ExportOptions,
    widths: Vec<usize>,
    out: Writer<'a>,
}

impl Flat<'_> {
    /// Writes the rows of the grid of `owner`, each after the rows in `trail` that it is
    /// inside of (one per level above).
    fn rows(&mut self, owner: CellId, trail: &mut Vec<Vec<CellId>>) -> Result<(), Error> {
        let Some(grid) = self.source.doc.cell(owner).and_then(Cell::grid) else {
            return Ok(());
        };
        for row in grid.cells().chunks(grid.xs().max(1) as usize) {
            trail.push(row.to_vec());
            let nested: Vec<CellId> = row
                .iter()
                .copied()
                .filter(|id| self.has_grid(*id))
                .collect();
            if nested.is_empty() {
                self.line(trail)?;
            }
            for id in nested {
                self.rows(id, trail)?;
            }
            trail.pop();
        }
        Ok(())
    }

    fn has_grid(&self, id: CellId) -> bool {
        self.source.doc.cell(id).is_some_and(|c| c.grid().is_some())
    }

    /// One line: the cells of each level, padded to the width of the level, then empty
    /// fields for the levels that this row is not nested as deep as.
    fn line(&mut self, trail: &[Vec<CellId>]) -> Result<(), Error> {
        let per_cell = if self.options.notes { 2 } else { 1 };
        for (level, width) in self.widths.iter().enumerate() {
            let cells = trail.get(level).map_or(&[][..], Vec::as_slice);
            for id in cells {
                cell_fields(&mut self.out, self.source, *id, self.options)?;
            }
            let missing = width.saturating_sub(cells.len()).saturating_mul(per_cell);
            for _ in 0..missing {
                self.out.raw("")?;
            }
        }
        self.out.end_row();
        Ok(())
    }
}

/// The grid of `cell` and every grid nested in it as one table, without changing the
/// document: one line per innermost row, that is per row none of whose cells has a grid.
/// Each line starts with the rows it is inside of, outermost first, so a document of
/// projects with tasks inside gives lines of project, task.
///
/// The table is rectangular whatever the nesting looks like:
///
/// - Every level of nesting has its own block of columns, as wide as the widest grid at
///   that level; rows of narrower grids are padded with empty fields on the right.
/// - A row that is not nested as deep as others has empty fields in the deeper blocks.
/// - A row with a nested grid gives no line of its own: its cells, the text of the cell
///   that has the grid included, are repeated on every line that comes from that grid.
/// - A row with several nested grids gives the lines of each grid in turn, all after the
///   same outer row.
/// - Empty cells are empty fields; rows that are entirely empty are kept.
///
/// Where every grid has one column this is the table that TreeSheets' Flatten (and
/// [`crate::Command::Flatten`]) makes. Refused for a cell without a grid, and if the text
/// would be larger than [`MAX_EXPORT_BYTES`].
pub fn flat_csv(source: Source, cell: CellId, options: &ExportOptions) -> Result<String, Error> {
    grid_of(source.doc, cell)?;
    let mut flat = Flat {
        source,
        options,
        widths: level_widths(source.doc, cell)?,
        out: Writer::new(options),
    };
    flat.rows(cell, &mut Vec::new())?;
    Ok(flat.out.finish())
}

/// TreeSheets' indented text export, byte for byte: one line per cell with what it shows,
/// the cells of a grid row by row, two more spaces of indentation per level. A cell with
/// an image has one space after its text. Lines end in CR LF with `crlf`, as TreeSheets
/// writes them on Linux and Windows, and in LF without, as it does on macOS.
///
/// With `own_line` the text starts with the line of `cell` itself, not indented, and its
/// grid follows at two spaces: what File > Export > Indented text writes for the whole
/// document. Without, it is what TreeSheets' script function `ts.get_subtree_text` gives:
/// the grid alone, starting at two spaces, or for a cell without a grid its text with no
/// line end.
pub fn outline(source: Source, cell: CellId, own_line: bool, crlf: bool) -> Result<String, Error> {
    let top = cell_of(source.doc, cell)?;
    if !own_line && top.grid().is_none() {
        return Ok(source.text(cell, false).to_string());
    }
    let mut out = String::new();
    // Each cell still to write, with its indentation; the last is next.
    let mut stack: Vec<(CellId, usize)> = Vec::new();
    if own_line {
        stack.push((cell, 0));
    } else if let Some(grid) = top.grid() {
        stack.extend(grid.cells().iter().rev().map(|c| (*c, 2)));
    }
    while let Some((id, indent)) = stack.pop() {
        let Some(c) = source.doc.cell(id) else {
            continue;
        };
        out.extend(std::iter::repeat_n(' ', indent));
        out.push_str(source.text(id, false));
        if c.image.is_some() {
            out.push(' ');
        }
        out.push_str(if crlf { "\r\n" } else { "\n" });
        if out.len() > MAX_EXPORT_BYTES {
            return Err(too_large());
        }
        if let Some(grid) = c.grid() {
            let inner = indent.saturating_add(2);
            stack.extend(grid.cells().iter().rev().map(|c| (*c, inner)));
        }
    }
    Ok(out)
}
