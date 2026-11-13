//! Reading for an interface that draws nested grids: the stage and what is below it.
//!
//! An interface draws one grid at full size, **the stage**, and each grid nested in it in
//! the richest form that still fits: cell by cell ("Full"), as the texts of its first cells
//! ("Preview"), as a miniature of colours without text ("Shape"), or as a count ("Badge").
//! Which form fits is the interface's to decide, since only it can measure; what each form
//! is made of is decided here, so that no interface makes up its own rules for what a cell
//! shows, which colours it is drawn in, or where a text may be cut.
//!
//! [`Editor::stage_view`] gives, in one answer and in a flat form (no type holds itself):
//!
//! - the **trail**: the cells from the root down to the stage cell;
//! - **grids**: the stage cell's grid, or a window of it, and every nested grid that the
//!   request wants cell by cell;
//! - **cells**: every cell of those grids, with all that is needed to draw it;
//! - **summaries**: for every one of those cells that has a grid, the size of that grid,
//!   the texts of its top-left cells and its shape. The count a Badge shows is the
//!   summary's `xs` times `ys`, the direct cells of the grid; nothing counts the cells
//!   further down.
//!
//! The work done and the memory used are in proportion to what is returned, never to the
//! document or to a number in the request: a window of 40 rows of a grid of 100,000 rows
//! looks at 40 rows.
//!
//! [`Editor::cell_look`] gives one cell with its whole text, which is what an editor opens
//! on.

use unicode_segmentation::GraphemeCursor;

use crate::error::Error;
use crate::eval::Calc;
use crate::id::CellId;
use crate::model::{
    Cell, CellKind, Color, Document, DrawStyle, Extra, Grid, Link, Run, Style, TextAlign, VertAlign,
};
use crate::text::normalize_runs;

use super::Editor;

/// What an interface wants of the stage and of what is nested below it: the request of
/// [`Editor::stage_view`].
///
/// Every number is the caller's and any value is safe: a number larger than what the
/// document has means "all of it", and zero means "none".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StageAsk {
    /// The first column of the stage grid to give, counted from 0.
    pub x0: u32,
    /// The first row of the stage grid to give, counted from 0.
    pub y0: u32,
    /// How many columns of the stage grid to give, from `x0` on.
    pub cols: u32,
    /// How many rows of the stage grid to give, from `y0` on. The window (`x0`, `y0`,
    /// `cols`, `rows`) is cut to the grid: `0, 0, u32::MAX, u32::MAX` is all of it, and a
    /// window that lies outside the grid gives no cells. Only the stage grid has a window;
    /// a nested grid is given whole or not at all.
    pub rows: u32,
    /// How many levels of nested grids may be given cell by cell, counted from the stage
    /// grid: 0 none, 1 the grids of the stage grid's cells, 2 also the grids of their
    /// cells, and so on.
    pub full_depth: u32,
    /// A nested grid with more cells than this (columns times rows) is not given cell by
    /// cell, and neither is anything inside it.
    pub full_cells: u32,
    /// The columns of the top-left block whose texts a summary gives.
    pub preview_cols: u32,
    /// The rows of that block.
    pub preview_rows: u32,
    /// The columns of the top-left block that a summary's shape covers.
    pub shape_cols: u32,
    /// The rows of that block.
    pub shape_rows: u32,
    /// The most code points of a text in [`CellLook`] (the text and the result).
    pub text_cut: u32,
    /// The most code points of a text in a summary's preview and in the trail.
    pub preview_cut: u32,
}

impl Default for StageAsk {
    /// Values to start from on a phone: the whole stage grid, nested grids of up to 24
    /// cells one level down cell by cell, previews of 3 by 4 cells of 40 code points,
    /// shapes of 12 by 20 cells, texts of up to 2,000 code points.
    fn default() -> StageAsk {
        StageAsk {
            x0: 0,
            y0: 0,
            cols: u32::MAX,
            rows: u32::MAX,
            full_depth: 1,
            full_cells: 24,
            preview_cols: 3,
            preview_rows: 4,
            shape_cols: 12,
            shape_rows: 20,
            text_cut: 2000,
            preview_cut: 40,
        }
    }
}

/// A text that may have been cut short.
///
/// A text is only ever cut between two user-perceived characters (extended grapheme
/// clusters, as [`crate::text::floor_boundary`] finds them), so what is left is the longest
/// such beginning of the whole text that has at most the asked number of code points. It
/// can be shorter than asked, and is empty if the first character alone is too long.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CutText {
    /// The beginning of the text that was kept; all of it if `cut` is false.
    pub text: String,
    /// Whether anything was left out.
    pub cut: bool,
}

/// What [`Editor::stage_view`] gives. See the fields for the order of everything, which is
/// fully determined by the document and the request.
#[derive(Clone, PartialEq, Debug)]
pub struct StageView {
    /// The cells from the root to the stage cell, the root first and the stage cell last.
    /// Never empty; for the root it is the root alone.
    pub trail: Vec<TrailStep>,
    /// The grids given cell by cell. `grids[0]` is the stage cell's own grid (its window);
    /// if the stage cell has no grid, `grids` is empty. After it come the nested grids
    /// given whole, in the order in which the cells that own them appear in `cells`; so
    /// all grids of one level come before any grid of the next.
    pub grids: Vec<GridPart>,
    /// The cells of all `grids`: those of `grids[0]` first, then those of `grids[1]`, and
    /// so on, each grid's cells row by row. [`GridPart::first`] says where a grid's cells
    /// start.
    pub cells: Vec<CellLook>,
    /// One for each cell in `cells` that has a grid, in the order of those cells, whether
    /// or not that grid is also in `grids`.
    pub summaries: Vec<GridSummary>,
}

/// One cell on the way from the root to the stage: what a title bar and a trail bar show.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TrailStep {
    /// The cell.
    pub id: CellId,
    /// What the cell shows (the result of a formula cell, the text of any other) on one
    /// line: cut to [`StageAsk::preview_cut`], and then every line break, and any other
    /// character below U+0020, replaced by a space.
    pub shown: CutText,
    /// The background colour the cell is drawn in: its tag's if it is a tag (see
    /// [`CellLook::is_tag`]), else its own.
    pub color: Color,
}

/// A grid whose cells are in [`StageView::cells`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GridPart {
    /// The cell whose grid this is.
    pub owner: CellId,
    /// How far below the stage: 0 for the stage cell's grid, 1 for the grids of its cells,
    /// and so on.
    pub level: u32,
    /// The columns of the whole grid.
    pub xs: u32,
    /// The rows of the whole grid.
    pub ys: u32,
    /// The first column given. For the stage grid this is [`StageAsk::x0`], or `xs` if
    /// that is past the end; for a nested grid 0.
    pub x0: u32,
    /// The first row given, likewise.
    pub y0: u32,
    /// How many columns are given: as many as asked for, or as there are from `x0` on, if
    /// those are fewer; for a nested grid all `xs`.
    pub cols: u32,
    /// How many rows are given, likewise.
    pub rows: u32,
    /// Where the cells of this grid start in [`StageView::cells`]. `cols` times `rows`
    /// cells follow, row by row: the cell in column `x0 + i` of row `y0 + j` is at
    /// `first + j * cols + i`.
    pub first: u32,
    /// The width at which text wraps (TreeSheets' unit; the default is 80) for each column
    /// that is given: `cols` entries, the first for column `x0`.
    pub col_widths: Vec<u32>,
    /// The colour of the grid's border.
    pub border_color: Color,
    /// The space around the grid.
    pub outer_spacing: u32,
    /// Whether the grid is below the owner's text (the default) instead of beside it.
    pub vertical: bool,
    /// How the owner wants its grid drawn.
    pub draw_style: DrawStyle,
}

/// One cell with all that is needed to draw it.
#[derive(Clone, PartialEq, Debug)]
pub struct CellLook {
    /// The cell.
    pub id: CellId,
    /// The text, cut to [`StageAsk::text_cut`]; for a formula cell, the formula. It may
    /// hold line breaks (U+000A).
    pub text: CutText,
    /// What the cell is to the formula system.
    pub kind: CellKind,
    /// What a formula cell shows instead of its text, cut to [`StageAsk::text_cut`];
    /// `None` for every other kind.
    pub result: Option<CutText>,
    /// Whether `result` is an error value such as `#NAME?`.
    pub failed: bool,
    /// The background colour to draw: the tag's if the cell is a tag, else the cell's own.
    pub cell_color: Color,
    /// The colour of the text outside of coloured runs: the tag's if the cell is a tag,
    /// else the cell's own.
    pub text_color: Color,
    /// Whether the cell is a tag: its whole text (not cut; for a formula cell, the
    /// formula) is the name of a tag of the document. As in TreeSheets, a tag is drawn in
    /// the tag's two colours and in no other: `cell_color` and `text_color` are then the
    /// tag's, and no run has a colour.
    pub is_tag: bool,
    /// The base style of the text.
    pub style: Style,
    /// The text size relative to the default; as in TreeSheets, positive is smaller.
    pub rel_size: i32,
    /// Horizontal alignment.
    pub text_align: TextAlign,
    /// Vertical alignment.
    pub vert_align: VertAlign,
    /// The ranges of `text` styled differently from `style`, in code points: sorted, not
    /// overlapping, and inside the text as it is given here (a run that the cut goes
    /// through ends at the cut; one beyond it is left out). Each has its complete style.
    /// Neighbours of the same style and colour are one run, and no run equals `style`
    /// without a colour. For a tag no run has a colour. `extra` is always empty.
    pub runs: Vec<Run>,
    /// The ranges of `text` that are links, in code points: sorted, not overlapping, and
    /// cut like the runs. `extra` is always empty.
    pub links: Vec<Link>,
    /// The file name of the cell's image in the document's image folder, if it has one.
    pub image: Option<String>,
    /// Whether the cell has a note.
    pub has_note: bool,
    /// Whether the cell has a grid.
    pub has_grid: bool,
    /// The cell's entry in [`StageView::summaries`]: there is one exactly if the cell has a
    /// grid. Always `None` from [`Editor::cell_look`].
    pub summary: Option<u32>,
    /// The cell's grid in [`StageView::grids`], if it is given cell by cell. Always `None`
    /// from [`Editor::cell_look`].
    pub full: Option<u32>,
}

/// A grid in reduced form: what a Preview, a Shape and a Badge are drawn from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GridSummary {
    /// The cell whose grid this is.
    pub owner: CellId,
    /// The columns of the grid.
    pub xs: u32,
    /// The rows of the grid. `xs` times `ys` is the count a Badge shows.
    pub ys: u32,
    /// The colour of the grid's border.
    pub border_color: Color,
    /// The columns of the preview block: [`StageAsk::preview_cols`], or `xs` if that is
    /// fewer.
    pub preview_cols: u32,
    /// The rows of the preview block: [`StageAsk::preview_rows`], or `ys` if that is fewer.
    pub preview_rows: u32,
    /// What the cells of the grid's top-left block of `preview_cols` by `preview_rows`
    /// show, row by row (`preview_cols` times `preview_rows` entries), each on one line:
    /// cut to [`StageAsk::preview_cut`], and then every line break, and any other
    /// character below U+0020, replaced by a space.
    pub preview: Vec<CutText>,
    /// The columns of the shape block: [`StageAsk::shape_cols`], or `xs` if that is fewer.
    pub shape_cols: u32,
    /// The rows of the shape block: [`StageAsk::shape_rows`], or `ys` if that is fewer.
    pub shape_rows: u32,
    /// The grid's top-left block of `shape_cols` by `shape_rows` cells, row by row, four
    /// bytes a cell: the red, green and blue of the background colour the cell is drawn in
    /// (its tag's if it is a tag), then how much it holds: 255 if it has a grid; else the
    /// number of code points it shows, but at most 254; so 0 is a cell that shows nothing.
    pub shape: Vec<u8>,
}

/// The longest beginning of `text` of at most `max` code points that ends between two
/// user-perceived characters, and whether that is less than all of it. Agrees with
/// [`crate::text::floor_boundary`], but looks only at the place of the cut, not at every
/// character before it: every preview text of a large grid is cut.
fn prefix(text: &str, max: u32) -> (&str, bool) {
    let max = max as usize;
    // A code point is at least one byte.
    if text.len() <= max {
        return (text, false);
    }
    let Some((at, _)) = text.char_indices().nth(max) else {
        return (text, false);
    };
    let whole = |at: usize| GraphemeCursor::new(at, text.len(), true).is_boundary(text, 0);
    let at = match whole(at) {
        Ok(true) => at,
        _ => GraphemeCursor::new(at, text.len(), true)
            .prev_boundary(text, 0)
            .ok()
            .flatten()
            .unwrap_or(0),
    };
    (text.get(..at).unwrap_or(""), true)
}

fn cut(text: &str, max: u32) -> CutText {
    let (head, cut) = prefix(text, max);
    CutText {
        text: head.to_string(),
        cut,
    }
}

/// `text` cut and on one line. The cut comes first, so that the result is as long as
/// the beginning of the text it stands for.
fn one_line(text: &str, max: u32) -> CutText {
    let (head, cut) = prefix(text, max);
    let space = |c: char| if c < ' ' { ' ' } else { c };
    CutText {
        text: head.chars().map(space).collect(),
        cut,
    }
}

fn code_points(text: &str) -> u32 {
    u32::try_from(text.chars().count()).unwrap_or(u32::MAX)
}

/// The part of `start`, `count` that lies in `0..size`.
fn clip(start: u32, count: u32, size: u32) -> (u32, u32) {
    let start = start.min(size);
    (start, count.min(size.saturating_sub(start)))
}

/// The cells of a block of `grid`, row by row. The block must lie inside the grid.
fn block(grid: &Grid, x0: u32, y0: u32, cols: u32, rows: u32) -> impl Iterator<Item = CellId> + '_ {
    // Skipping rows of a slice, and cells of a row, does not walk over what is skipped.
    grid.cells()
        .chunks((grid.xs() as usize).max(1))
        .skip(y0 as usize)
        .take(rows as usize)
        .flat_map(move |row| row.iter().skip(x0 as usize).take(cols as usize).copied())
}

fn area(cols: u32, rows: u32) -> usize {
    (cols as usize).saturating_mul(rows as usize)
}

fn index(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// One reading of the document. Every cell is fetched through [`Walk::cell`], which counts,
/// so that a test can hold the work against what was returned.
struct Walk<'a> {
    doc: &'a Document,
    calc: &'a Calc,
    looked: u64,
}

impl<'a> Walk<'a> {
    fn new(editor: &'a Editor) -> Walk<'a> {
        Walk {
            doc: &editor.doc,
            calc: &editor.calc,
            looked: 0,
        }
    }

    fn cell(&mut self, id: CellId) -> Result<&'a Cell, Error> {
        self.looked = self.looked.saturating_add(1);
        self.doc
            .cell(id)
            .ok_or_else(|| Error::NotFound(format!("cell {id}")))
    }

    /// What the cell shows. The results are those of the formula cells and of no other
    /// cell, as in `read.rs`.
    fn shown(&self, cell: &'a Cell) -> &'a str {
        match self.calc.result(cell.id) {
            Some((text, _)) => text,
            None => &cell.text,
        }
    }

    /// The colours the cell is drawn in, and whether they are a tag's (TreeSheets
    /// `Cell::Render` and `Text::Render`: a cell whose text is a tag's name takes both
    /// colours from the tag).
    fn colors(&self, cell: &Cell) -> (Color, Color, bool) {
        match self.doc.tags().get(&cell.text) {
            Some(tag) => (tag.cell_color, tag.text_color, true),
            None => (cell.cell_color, cell.text_color, false),
        }
    }

    fn look(&self, cell: &Cell, max: u32) -> CellLook {
        let (head, was_cut) = prefix(&cell.text, max);
        let (cell_color, text_color, is_tag) = self.colors(cell);
        let result = self.calc.result(cell.id);
        let (mut runs, mut links) = (Vec::new(), Vec::new());
        if !cell.runs.is_empty() || !cell.links.is_empty() {
            let end = if was_cut { code_points(head) } else { u32::MAX };
            let inside = |start: u32, len: u32| len.min(end.saturating_sub(start));
            // A run beyond the cut is left with no length, and normalising drops it.
            runs = cell
                .runs
                .iter()
                .map(|run| Run {
                    start: run.start,
                    len: inside(run.start, run.len),
                    style: run.style,
                    color: if is_tag { None } else { run.color },
                    extra: Extra::new(),
                })
                .collect();
            // Without its colour, or its unknown fields, a run may no longer differ from
            // its neighbour or from the cell.
            normalize_runs(&mut runs, cell.style);
            links = cell
                .links
                .iter()
                .filter(|link| link.start < end)
                .map(|link| Link {
                    start: link.start,
                    len: inside(link.start, link.len),
                    target: link.target.clone(),
                    extra: Extra::new(),
                })
                .collect();
        }
        CellLook {
            id: cell.id,
            text: CutText {
                text: head.to_string(),
                cut: was_cut,
            },
            kind: cell.kind,
            result: result.map(|(text, _)| cut(text, max)),
            failed: result.is_some_and(|(_, failed)| failed),
            cell_color,
            text_color,
            is_tag,
            style: cell.style,
            rel_size: cell.rel_size,
            text_align: cell.text_align,
            vert_align: cell.vert_align,
            runs,
            links,
            image: cell.image.as_ref().map(|image| image.file.clone()),
            has_note: !cell.note.is_empty(),
            has_grid: cell.grid().is_some(),
            summary: None,
            full: None,
        }
    }

    /// The cells from the root to `stage`.
    fn trail(&mut self, stage: CellId, ask: &StageAsk) -> Result<Vec<TrailStep>, Error> {
        let mut trail = Vec::new();
        let mut next = Some(stage);
        while let Some(id) = next {
            // A valid document has no cycle of parents; this keeps a broken one from looping.
            if trail.len() > self.doc.cell_count() {
                break;
            }
            let cell = self.cell(id)?;
            trail.push(TrailStep {
                id,
                shown: one_line(self.shown(cell), ask.preview_cut),
                color: self.colors(cell).0,
            });
            next = cell.parent();
        }
        trail.reverse();
        Ok(trail)
    }

    fn summary(
        &mut self,
        owner: CellId,
        grid: &Grid,
        ask: &StageAsk,
    ) -> Result<GridSummary, Error> {
        let (xs, ys) = (grid.xs(), grid.ys());
        let (preview_cols, preview_rows) = (ask.preview_cols.min(xs), ask.preview_rows.min(ys));
        // The sizes are cut to the grid, so what is reserved is what will be filled.
        let mut preview = Vec::with_capacity(area(preview_cols, preview_rows));
        for id in block(grid, 0, 0, preview_cols, preview_rows) {
            let cell = self.cell(id)?;
            preview.push(one_line(self.shown(cell), ask.preview_cut));
        }
        let (shape_cols, shape_rows) = (ask.shape_cols.min(xs), ask.shape_rows.min(ys));
        let mut shape = Vec::with_capacity(area(shape_cols, shape_rows).saturating_mul(4));
        for id in block(grid, 0, 0, shape_cols, shape_rows) {
            let cell = self.cell(id)?;
            let color = self.colors(cell).0;
            let holds = if cell.grid().is_some() {
                255
            } else {
                // Counting stops at 254, however long the text is.
                self.shown(cell).chars().take(254).count() as u8
            };
            shape.extend([color.r, color.g, color.b, holds]);
        }
        Ok(GridSummary {
            owner,
            xs,
            ys,
            border_color: grid.border_color,
            preview_cols,
            preview_rows,
            preview,
            shape_cols,
            shape_rows,
            shape,
        })
    }

    fn stage_view(&mut self, stage: CellId, ask: &StageAsk) -> Result<StageView, Error> {
        let mut view = StageView {
            trail: self.trail(stage, ask)?,
            grids: Vec::new(),
            cells: Vec::new(),
            summaries: Vec::new(),
        };
        // `view.grids` is also the list of grids still to give: each is entered when the
        // cell that owns it is met, with the place its cells will have, and its cells are
        // added when its turn comes. No recursion, whatever the depth.
        let mut planned = 0u32;
        let mut part = |owner: &Cell, grid: &Grid, level: u32, window: (u32, u32, u32, u32)| {
            let (x0, cols) = clip(window.0, window.2, grid.xs());
            let (y0, rows) = clip(window.1, window.3, grid.ys());
            let first = planned;
            planned = planned.saturating_add(index(area(cols, rows)));
            let widths = grid.col_widths.iter().skip(x0 as usize);
            GridPart {
                owner: owner.id,
                level,
                xs: grid.xs(),
                ys: grid.ys(),
                x0,
                y0,
                cols,
                rows,
                first,
                col_widths: widths.take(cols as usize).copied().collect(),
                border_color: grid.border_color,
                outer_spacing: grid.outer_spacing,
                vertical: grid.vertical,
                draw_style: owner.draw_style,
            }
        };
        let top = self.cell(stage)?;
        if let Some(grid) = top.grid() {
            view.grids
                .push(part(top, grid, 0, (ask.x0, ask.y0, ask.cols, ask.rows)));
        }
        let mut turn = 0;
        while let Some(given) = view.grids.get(turn) {
            // A valid document is a tree; this keeps a broken one from looping.
            if view.cells.len() > self.doc.cell_count() {
                break;
            }
            let (owner, level) = (given.owner, given.level);
            let (x0, y0, cols, rows) = (given.x0, given.y0, given.cols, given.rows);
            turn += 1;
            let Some(grid) = self.cell(owner)?.grid() else {
                continue;
            };
            view.cells.reserve(area(cols, rows));
            for id in block(grid, x0, y0, cols, rows) {
                let cell = self.cell(id)?;
                let mut look = self.look(cell, ask.text_cut);
                if let Some(inner) = cell.grid() {
                    look.summary = Some(index(view.summaries.len()));
                    view.summaries.push(self.summary(id, inner, ask)?);
                    let count = u64::from(inner.xs()).saturating_mul(u64::from(inner.ys()));
                    if level < ask.full_depth && count <= u64::from(ask.full_cells) {
                        look.full = Some(index(view.grids.len()));
                        let whole = (0, 0, u32::MAX, u32::MAX);
                        view.grids
                            .push(part(cell, inner, level.saturating_add(1), whole));
                    }
                }
                view.cells.push(look);
            }
        }
        Ok(view)
    }
}

impl Editor {
    /// The stage cell `stage` with what is nested below it, as far as `ask` says: the
    /// trail from the root, the window of the stage cell's grid cell by cell, the nested
    /// grids that are wanted cell by cell, and a summary of every grid met on the way. See
    /// [`StageView`] for what is where and in which order, and [`StageAsk`] for the request.
    ///
    /// A nested grid is given cell by cell if the grid it is in is given, it is at most
    /// [`StageAsk::full_depth`] levels below the stage grid, and it has at most
    /// [`StageAsk::full_cells`] cells. A summary is given for the grid of every cell that
    /// is given, also when that grid is given cell by cell as well: an interface that
    /// finds no room for the whole grid then has the lesser forms at hand.
    ///
    /// This only reads. It fails only if there is no cell `stage`, with
    /// [`Error::NotFound`]. A stage cell without a grid gives its trail and nothing else.
    /// Any numbers in `ask` are safe. The same document and request always give the same
    /// answer, and the work is in proportion to the answer: the cells, preview texts and
    /// shape cells returned, and the trail.
    pub fn stage_view(&self, stage: CellId, ask: &StageAsk) -> Result<StageView, Error> {
        Walk::new(self).stage_view(stage, ask)
    }

    /// One cell with its whole text, runs and links, nothing cut: what an editor opens on.
    /// The colours are those the cell is drawn in, as in [`Editor::stage_view`].
    /// `has_grid` says whether it has a grid; `summary` and `full` are `None`. Fails only
    /// if there is no such cell, with [`Error::NotFound`].
    pub fn cell_look(&self, id: CellId) -> Result<CellLook, Error> {
        let mut walk = Walk::new(self);
        let cell = walk.cell(id)?;
        // No text is as long as this (`MAX_TEXT_CODE_POINTS`), so nothing is cut.
        Ok(walk.look(cell, u32::MAX))
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
    use crate::construct::Construct;
    use crate::id::SeededIds;
    use crate::text::floor_boundary;

    /// The texts that `text.rs` tests its boundaries with, and some longer ones.
    const TEXTS: &[&str] = &[
        "",
        "abc",
        "e\u{301}x",
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}a",
        "\u{1F44D}\u{1F3FD}",
        "\u{1F1E9}\u{1F1EA}\u{1F1EB}\u{1F1F7}\u{1F1E9}\u{1F1EA}",
        "\u{1100}\u{1161}\u{11A8}\u{D55C}",
        "\u{2764}\u{FE0F}!",
        "a\nb\r\nc",
        "日本語 e\u{301}\u{301}\u{301} 😀\u{200D}",
    ];

    #[test]
    fn a_cut_is_where_the_rules_of_text_put_it() {
        for text in TEXTS {
            let len = text.chars().count() as u32;
            for max in (0..=len + 2).chain([u32::MAX]) {
                let (head, was_cut) = prefix(text, max);
                let kept = floor_boundary(text, max);
                assert!(text.starts_with(head), "{text:?} at {max}");
                assert_eq!(head.chars().count() as u32, kept, "{text:?} at {max}");
                assert_eq!(was_cut, max < len, "{text:?} at {max}");
            }
        }
    }

    #[test]
    fn a_preview_text_is_one_line_as_long_as_what_it_stands_for() {
        let line = one_line("ab\ncd\te\u{1f}f", u32::MAX);
        assert_eq!((line.text.as_str(), line.cut), ("ab cd e f", false));
        // The break is a character like any other to the cut, which comes first.
        let line = one_line("ab\ncd", 3);
        assert_eq!((line.text.as_str(), line.cut), ("ab ", true));
        assert_eq!(one_line("ab\ncd", 2).text, "ab");
        // A cell's own text keeps its breaks.
        assert_eq!(cut("ab\ncd", 4).text, "ab\nc");
    }

    #[test]
    fn a_window_is_cut_to_the_grid() {
        assert_eq!(clip(0, u32::MAX, 7), (0, 7));
        assert_eq!(clip(2, 3, 7), (2, 3));
        assert_eq!(clip(5, 3, 7), (5, 2));
        assert_eq!(clip(7, 3, 7), (7, 0));
        assert_eq!(clip(u32::MAX, u32::MAX, 7), (7, 0));
        assert_eq!(clip(3, 0, 7), (3, 0));
        assert_eq!(clip(0, 5, 0), (0, 0));
    }

    /// A line break in a text, put there behind the editor's back so that this holds
    /// whether or not commands let one in: the cell keeps it, a preview and the trail show
    /// a space.
    #[test]
    fn a_line_break_is_kept_in_the_cell_and_a_space_in_a_preview_and_the_trail() {
        let mut ids = SeededIds::from_u64(40);
        let mut doc = Document::new(&mut ids);
        let root = doc.root();
        doc.add_grid(root, 1, 1, &mut ids).unwrap();
        let holder = doc.child(root, 0, 0).unwrap();
        doc.add_grid(holder, 1, 1, &mut ids).unwrap();
        let inner = doc.child(holder, 0, 0).unwrap();
        let mut editor = Editor::new(doc).unwrap();
        for id in [holder, inner] {
            editor.doc.cell_mut(id).unwrap().text = "one\ntwo".into();
        }
        let ask = StageAsk::default();
        let view = editor.stage_view(root, &ask).unwrap();
        assert_eq!(view.cells[0].text.text, "one\ntwo");
        assert_eq!(view.summaries[0].preview[0].text, "one two");
        assert_eq!(view.summaries[0].shape[3], 7);
        let view = editor.stage_view(holder, &ask).unwrap();
        assert_eq!(view.trail[1].shown.text, "one two");
        assert_eq!(view.cells[0].text.text, "one\ntwo");
        assert_eq!(editor.cell_look(inner).unwrap().text.text, "one\ntwo");
    }

    /// A root of 10 columns by `rows` rows, every cell holding a grid of 6 by 6, in which
    /// the first cell of each row holds a grid of 2 by 2.
    fn tall(rows: u32) -> Editor {
        let mut ids = SeededIds::from_u64(41);
        let mut doc = Document::new(&mut ids);
        let root = doc.root();
        doc.add_grid(root, 10, rows, &mut ids).unwrap();
        for cell in doc.cell(root).unwrap().grid().unwrap().cells().to_vec() {
            doc.add_grid(cell, 6, 6, &mut ids).unwrap();
            let inner = doc.cell(cell).unwrap().grid().unwrap().cells().to_vec();
            for (n, leaf) in inner.iter().enumerate() {
                Construct::cell_mut(&mut doc, *leaf).unwrap().text = format!("leaf {n}");
                if n % 6 == 0 {
                    doc.add_grid(*leaf, 2, 2, &mut ids).unwrap();
                }
            }
        }
        Editor::new(doc).unwrap()
    }

    /// The cells a reading may fetch: each step of the trail, the stage cell, the owner of
    /// each grid given, each cell given, and each cell behind a preview text or a shape
    /// cell.
    fn allowed(view: &StageView) -> u64 {
        let summaries: usize = view
            .summaries
            .iter()
            .map(|s| s.preview.len() + s.shape.len() / 4)
            .sum();
        (view.trail.len() + 1 + view.grids.len() + view.cells.len() + summaries) as u64
    }

    /// The instrument for "no work for what is not returned": every cell the reading
    /// fetches from the document is counted, and the count is held against the answer. It
    /// does not depend on the machine or its load.
    #[test]
    fn a_reading_fetches_only_the_cells_it_gives() {
        let editor = tall(400);
        let root = editor.document().root();
        assert_eq!(editor.document().cell_count(), 1 + 4000 * (1 + 36 + 6 * 4));
        let asks = [
            StageAsk {
                y0: 200,
                rows: 40,
                ..StageAsk::default()
            },
            StageAsk {
                x0: 3,
                cols: 2,
                y0: 390,
                rows: 40,
                full_depth: 2,
                full_cells: 36,
                ..StageAsk::default()
            },
            StageAsk {
                rows: 0,
                ..StageAsk::default()
            },
            StageAsk {
                y0: 5,
                rows: 1,
                cols: 1,
                full_depth: 0,
                preview_cols: 0,
                shape_rows: 0,
                ..StageAsk::default()
            },
        ];
        let mut fetched = Vec::new();
        for ask in &asks {
            let mut walk = Walk::new(&editor);
            let view = walk.stage_view(root, ask).unwrap();
            assert_eq!(walk.looked, allowed(&view), "{ask:?}");
            fetched.push((view.cells.len(), walk.looked));
        }
        // 400 cells, each with a preview of 3 by 4 and a shape of 6 by 6.
        assert_eq!(fetched[0], (400, 1 + 1 + 1 + 400 + 400 * (12 + 36)));
        // 20 cells and their grids of 36 cells, and the 6 grids of 4 cells in each.
        assert_eq!(fetched[1].0, 20 + 20 * 36 + 20 * 6 * 4);
        assert_eq!(fetched[2], (0, 3));
        assert_eq!(fetched[3], (1, 4));
        // A stage further down fetches its trail and no more of what is above it.
        let doc = editor.document();
        let inner = doc.child(doc.child(root, 9, 399).unwrap(), 0, 5).unwrap();
        let mut walk = Walk::new(&editor);
        let view = walk.stage_view(inner, &StageAsk::default()).unwrap();
        assert_eq!((view.trail.len(), view.cells.len()), (3, 4));
        assert_eq!(walk.looked, 3 + 1 + 1 + 4);
    }
}
