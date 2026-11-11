//! Two cheap estimates that the editor keeps up to date from the steps it applies
//! (decisions A43 and A46): an upper bound on the size of the saved file, and the memory
//! that a step in the history holds.
//!
//! The bound is a sum of one number per cell, tag and document, each a function of that
//! thing alone, so a step changes the sum by the difference of the parts it touches and
//! nothing ever needs the whole document. It is never below what [`mod@crate::format`] writes:
//! every part is counted as written or longer (indentation as the deepest allowed, numbers
//! of runs and links with all their digits), and a part is counted in the same way whether
//! a whole cell is counted or one field of it changes.

use std::mem::size_of;

use serde_json::Value;

use crate::limits::MAX_DEPTH;
use crate::model::{
    Cell, CellKind, Color, Document, DrawStyle, Extra, Grid, ImageRef, Link, LinkTarget, Run,
    Style, Tag, TextAlign, VertAlign,
};

use super::step::{Field, Step};

/// A cell with nothing but its ID: the separator and line end, the deepest indentation, and
/// `{"id": "` with the 36 characters of the ID and `"}`. No cell adds less to the bound.
pub(crate) const CELL_BYTES: u64 = 2 + (MAX_DEPTH as u64 + 1) + 46;
/// What a grid adds besides its properties: `, "grid": {"xs": `, `, "ys": `, two numbers,
/// `, "cells": [`, and the line with `]}`.
const GRID_BYTES: u64 = 96;
/// The lines of a document around its cells, tags and unknown fields.
const DOCUMENT_BYTES: u64 = 192;
/// The longest a number is written: `-9223372036854775808`.
const NUMBER: u64 = 20;

/// The length of `text` inside a JSON string, as `json::write_string` writes it.
fn escaped(text: &str) -> u64 {
    let escapes: u64 = text
        .bytes()
        .map(|b| match b {
            b'"' | b'\\' | b'\n' | b'\r' | b'\t' => 1,
            0..=0x1f => 5,
            _ => 0,
        })
        .sum();
    (text.len() as u64).saturating_add(escapes)
}

/// The length of `text` as a JSON string, with its quotes.
fn string(text: &str) -> u64 {
    escaped(text).saturating_add(2)
}

/// `, "name": `.
fn key(name: &str) -> u64 {
    name.len() as u64 + 6
}

fn named(name: &str, value: &str) -> u64 {
    key(name) + value.len() as u64 + 2
}

fn color(name: &str, color: Color, default: Color) -> u64 {
    if color == default {
        0
    } else {
        key(name) + 9
    }
}

fn style(style: Style) -> u64 {
    let Style {
        bold,
        italic,
        fixed,
        underline,
        strike,
    } = style;
    crate::format::STYLE_KEYS
        .iter()
        .zip([bold, italic, fixed, underline, strike])
        .map(|(name, on)| if on { key(name) + 4 } else { 0 })
        .sum()
}

fn extra(extra: &Extra) -> u64 {
    let value = |v: &Value| serde_json::to_string(v).map_or(0, |text| text.len() as u64);
    extra
        .iter()
        .map(|(name, v)| string(name).saturating_add(4).saturating_add(value(v)))
        .fold(0, u64::saturating_add)
}

fn text(text: &str) -> u64 {
    text_frame(text.is_empty()).saturating_add(escaped(text))
}

fn note(note: &str) -> u64 {
    if note.is_empty() {
        0
    } else {
        key("note") + string(note)
    }
}

pub(crate) fn runs(runs: &[Run]) -> u64 {
    if runs.is_empty() {
        return 0;
    }
    let one = |run: &Run| {
        4 + key("start")
            + key("len")
            + 2 * NUMBER
            + style(run.style)
            + run.color.map_or(0, |_| key("color") + 9)
            + extra(&run.extra)
    };
    runs.iter()
        .map(one)
        .fold(key("runs") + 2, u64::saturating_add)
}

pub(crate) fn links(links: &[Link]) -> u64 {
    if links.is_empty() {
        return 0;
    }
    let one = |link: &Link| {
        let target = match &link.target {
            LinkTarget::External(url) => key("url").saturating_add(string(url)),
            LinkTarget::Cell { doc, .. } => key("cell") + 38 + doc.map_or(0, |_| key("doc") + 38),
            LinkTarget::Unknown => 0,
        };
        (4 + key("start") + key("len") + 2 * NUMBER)
            .saturating_add(target)
            .saturating_add(extra(&link.extra))
    };
    links
        .iter()
        .map(one)
        .fold(key("links") + 2, u64::saturating_add)
}

fn image(image: Option<&ImageRef>) -> u64 {
    image.map_or(0, |image| {
        // A scale is a number of at most 24 characters.
        (key("image") + 2 + key("file") + key("scale") + 24)
            .saturating_add(string(&image.file))
            .saturating_add(extra(&image.extra))
    })
}

fn widths(widths: &[u32]) -> u64 {
    if widths.iter().all(|w| *w == Grid::DEFAULT_COL_WIDTH) {
        0
    } else {
        key("widths") + 2 + 12 * widths.len() as u64
    }
}

fn spacing(spacing: u32) -> u64 {
    if spacing == Grid::DEFAULT_OUTER_SPACING {
        0
    } else {
        key("spacing") + NUMBER
    }
}

fn vertical(vertical: bool) -> u64 {
    if vertical {
        0
    } else {
        key("vertical") + 5
    }
}

pub(crate) fn grid(grid: Option<&Grid>) -> u64 {
    grid.map_or(0, |g| {
        (GRID_BYTES
            + color("border", g.border_color, Color::BORDER)
            + spacing(g.outer_spacing)
            + vertical(g.vertical))
        .saturating_add(widths(&g.col_widths))
        .saturating_add(extra(&g.extra))
    })
}

/// What the columns of a grid add to the bound; changes when columns come and go.
pub(crate) fn grid_widths(grid: &Grid) -> u64 {
    widths(&grid.col_widths)
}

fn kind(kind: CellKind) -> u64 {
    if kind == CellKind::Data {
        0
    } else {
        named("kind", kind.as_str())
    }
}

fn last_edit(time: Option<i64>) -> u64 {
    time.map_or(0, |_| key("edited") + NUMBER)
}

fn rel_size(size: i32) -> u64 {
    if size == 0 {
        0
    } else {
        key("size") + NUMBER
    }
}

fn draw_style(draw: DrawStyle) -> u64 {
    if draw == DrawStyle::Grid {
        0
    } else {
        named("draw", draw.as_str())
    }
}

fn text_align(align: TextAlign) -> u64 {
    if align == TextAlign::Auto {
        0
    } else {
        named("align", align.as_str())
    }
}

fn vert_align(align: VertAlign) -> u64 {
    if align == VertAlign::Auto {
        0
    } else {
        named("valign", align.as_str())
    }
}

/// What one field with this value adds to the bound of its cell.
pub(crate) fn field(field: &Field) -> u64 {
    match field {
        Field::Kind(v) => kind(*v),
        Field::CellColor(v) => color("color", *v, Color::WHITE),
        Field::TextColor(v) => color("text_color", *v, Color::BLACK),
        Field::DrawStyle(v) => draw_style(*v),
        Field::Note(v) => note(v),
        Field::TextAlign(v) => text_align(*v),
        Field::VertAlign(v) => vert_align(*v),
        Field::RelSize(v) => rel_size(*v),
        Field::Style(v) => style(*v),
        Field::Runs(v) => runs(v),
        Field::Links(v) => links(v),
        Field::LastEdit(v) => last_edit(*v),
        Field::Image(v) => image(v.as_ref()),
        Field::Extra(v) => extra(v),
        Field::BorderColor(v) => color("border", *v, Color::BORDER),
        Field::OuterSpacing(v) => spacing(*v),
        Field::Vertical(v) => vertical(*v),
        Field::ColWidths(v) => widths(v),
    }
}

/// What a piece of a cell's text adds to the bound: a text is counted as its pieces and
/// its frame, so that a change of a long text is counted from the piece that changes.
pub(crate) fn text_piece(piece: &str) -> u64 {
    escaped(piece)
}

/// What a text adds to the bound besides its pieces: nothing if it is empty.
pub(crate) fn text_frame(empty: bool) -> u64 {
    if empty {
        0
    } else {
        key("text") + 2
    }
}

/// What one cell adds to the bound, its grid's properties included, the cells inside not.
pub(crate) fn cell(c: &Cell) -> u64 {
    [
        CELL_BYTES,
        text(&c.text),
        kind(c.kind),
        color("color", c.cell_color, Color::WHITE),
        color("text_color", c.text_color, Color::BLACK),
        draw_style(c.draw_style),
        note(&c.note),
        text_align(c.text_align),
        vert_align(c.vert_align),
        rel_size(c.rel_size),
        style(c.style),
        runs(&c.runs),
        links(&c.links),
        last_edit(c.last_edit),
        image(c.image.as_ref()),
        extra(&c.extra),
        grid(c.grid.as_ref()),
    ]
    .into_iter()
    .fold(0, u64::saturating_add)
}

/// What one tag adds to the bound.
pub(crate) fn tag(name: &str, tag: Option<&Tag>) -> u64 {
    tag.map_or(0, |tag| {
        string(name)
            .saturating_add(64)
            .saturating_add(extra(&tag.extra))
    })
}

/// The bound for a whole document: no file that [`crate::format::to_string`] writes for it
/// is longer.
pub(crate) fn document(doc: &Document) -> u64 {
    let cells = doc.cells.values().map(cell);
    let tags = doc.tags.iter().map(|(name, t)| tag(name, Some(t)));
    cells
        .chain(tags)
        .fold(DOCUMENT_BYTES, u64::saturating_add)
        .saturating_add(extra(&doc.extra))
}

fn held_cell(c: &Cell) -> usize {
    let links: usize = c
        .links
        .iter()
        .map(|link| match &link.target {
            LinkTarget::External(url) => url.len(),
            _ => 0,
        })
        .sum();
    size_of::<Cell>()
        + c.text.len()
        + c.note.len()
        + c.runs.len() * size_of::<Run>()
        + c.links.len() * size_of::<Link>()
        + links
        + c.grid.as_ref().map_or(0, held_grid)
}

fn held_grid(grid: &Grid) -> usize {
    grid.cells.len() * size_of::<crate::id::CellId>() + grid.col_widths.len() * size_of::<u32>()
}

/// About how many bytes of memory a step in the history takes. Unknown fields are not
/// looked into.
pub(crate) fn held(step: &Step) -> usize {
    let id = size_of::<crate::id::CellId>();
    let more = match step {
        Step::Splice { ins, .. } => ins.len(),
        Step::Set { field, .. } => match field {
            Field::Note(note) => note.len(),
            Field::Runs(runs) => runs.len() * size_of::<Run>(),
            Field::Links(links) => links.len() * size_of::<Link>(),
            Field::Image(Some(image)) => image.file.len(),
            Field::ColWidths(widths) => widths.len() * size_of::<u32>(),
            _ => 0,
        },
        Step::PatchRuns { patch, .. } => patch.ins.len() * size_of::<Run>(),
        Step::PatchLinks { patch, .. } => patch.ins.len() * size_of::<Link>(),
        Step::Tag { name, .. } => name.len() + size_of::<Tag>(),
        Step::AddCells(cells) => cells.iter().map(held_cell).sum(),
        Step::RemoveCells(ids) => ids.len() * id,
        Step::InsertLines { ids, widths, .. } => ids.len() * id + widths.len() * size_of::<u32>(),
        Step::SetGrid { grid, .. } => grid.as_ref().map_or(0, held_grid),
        Step::Reparented { step, parents } => held(step) + parents.len() * (2 * id + 1),
        Step::RemoveLines { .. } | Step::MoveLines { .. } | Step::Swap { .. } => 0,
    };
    size_of::<Step>() + more
}
