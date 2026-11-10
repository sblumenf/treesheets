//! The `.cts` reader proper: header, images, the compressed stream, the cell tree and the tags.
//!
//! It follows TreeSheets' own loader (`System::LoadDB`, `Cell::LoadWhich`, `Cell::LoadGrid`,
//! `Grid::LoadContents`, `Text::Load` in `../../../../src/`), which is the truth where the
//! written specification is vague. Nothing here recurses: the tree is read with an explicit
//! stack, so nesting depth costs heap, not stack.

use std::collections::BTreeMap;
use wombat_core::construct::Construct;

use wombat_core::limits::MAX_TEXT_CODE_POINTS;
use wombat_core::model::{code_points, Extra};
use wombat_core::{
    CellId, CellKind, Color, Document, DrawStyle, IdSource, ImageRef, Link, LinkTarget, Run, Style,
    Tag, TextAlign, VertAlign,
};

use crate::reader::Reader;
use crate::{CtsError, Image, ImageKind, MAX_CTS_FILE_BYTES, MAX_IMAGES, NEWEST_VERSION};

/// TreeSheets refuses a grid with more cells than this (`g_max_grid_cells`).
const MAX_GRID_CELLS: i64 = 4 * 1024 * 1024;
/// The bit of the "cell contents" byte that marks the selected cell.
const SELECTION_MASK: u8 = 0x80;
/// The bit of a run's colour that says the run has a colour of its own.
const RUN_HAS_COLOR: u32 = 0x100_0000;
/// The width of the columns of a grid in a file too old to say (before version 13).
const OLD_COLUMN_WIDTH: u32 = 80;

/// An internal link as the file has it: a range and the text of the cell it goes to.
pub(crate) struct TextLink {
    pub(crate) start: u32,
    pub(crate) len: u32,
    pub(crate) target: String,
}

/// What the file says, before links are resolved and the view state is worked out.
pub(crate) struct Parsed {
    pub(crate) version: u8,
    pub(crate) document: Document,
    pub(crate) images: Vec<Image>,
    /// The cells whose grid is folded.
    pub(crate) folded: Vec<CellId>,
    /// The cell marked as the first selected one, and the size of the selection.
    pub(crate) selected: Option<CellId>,
    pub(crate) selection_size: (u8, u8),
    pub(crate) zoom_level: u8,
    pub(crate) autoexport: u8,
    /// Per cell, its internal links, still by target text.
    pub(crate) text_links: Vec<(CellId, Vec<TextLink>)>,
    pub(crate) undecodable: usize,
}

fn corrupt<T>(message: impl Into<String>) -> Result<T, CtsError> {
    Err(CtsError::Corrupt(message.into()))
}

/// A TreeSheets colour number (`0xBBGGRR`, red in the lowest byte) as a Wombat colour.
pub(crate) fn color(number: u32) -> Color {
    let [r, g, b, _] = number.to_le_bytes();
    Color::rgb(r, g, b)
}

fn style(bits: u32) -> Style {
    Style {
        bold: bits & 1 != 0,
        italic: bits & 2 != 0,
        fixed: bits & 4 != 0,
        underline: bits & 8 != 0,
        strike: bits & 16 != 0,
    }
}

/// What TreeSheets does to text as it loads it: control characters become spaces, one for
/// one, so positions in the text stay the same.
fn filter_controls(text: String) -> String {
    if text.chars().any(|c| c < ' ') {
        text.chars()
            .map(|c| if c < ' ' { ' ' } else { c })
            .collect()
    } else {
        text
    }
}

/// Reads a whole file.
pub(crate) fn parse(
    bytes: &[u8],
    ids: &mut dyn IdSource,
    max_decompressed: usize,
) -> Result<Parsed, CtsError> {
    if u64::try_from(bytes.len()).map_or(true, |len| len > MAX_CTS_FILE_BYTES) {
        return Err(CtsError::Limit(format!(
            "the file is larger than {MAX_CTS_FILE_BYTES} bytes"
        )));
    }
    let mut r = Reader::new(bytes);
    if r.take(4).ok() != Some(b"TSFF") {
        return Err(CtsError::NotCts);
    }
    let version = r.u8()?;
    if version > NEWEST_VERSION {
        return Err(CtsError::NewerVersion(version));
    }
    if version >= 30 {
        match r.u8()? {
            0 => {}
            1 => return Err(CtsError::Encrypted),
            other => return corrupt(format!("unknown encryption method {other}")),
        }
    }
    let selection_size = if version >= 21 {
        (r.u8()?, r.u8()?)
    } else {
        (1, 1)
    };
    let zoom_level = if version >= 23 { r.u8()? } else { 0 };
    let autoexport = if version >= 30 { r.u8()? } else { 0 };

    let mut images = Vec::new();
    loop {
        let kind = match r.u8()? {
            b'I' => ImageKind::Png,
            b'J' => ImageKind::Jpeg,
            b'D' => break,
            other => return corrupt(format!("unknown block type {other:#04x} in the header")),
        };
        if images.len() >= MAX_IMAGES {
            return Err(CtsError::Limit(format!("more than {MAX_IMAGES} images")));
        }
        images.push(read_image(&mut r, version, kind, ids)?);
    }

    let stream =
        miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(r.rest(), max_decompressed)
            .map_err(|error| match error.status {
                miniz_oxide::inflate::TINFLStatus::HasMoreOutput => CtsError::Limit(format!(
                    "the cells take more than {max_decompressed} bytes uncompressed"
                )),
                _ => CtsError::Corrupt("the compressed cell data is damaged".into()),
            })?;
    let undecodable = r.undecodable;
    let mut tree = Tree {
        r: Reader::new(&stream),
        version,
        document: Document::new(ids),
        ids,
        images: &images,
        folded: Vec::new(),
        selected: None,
        text_links: Vec::new(),
    };
    tree.read()?;
    let tags = tree.read_tags()?;
    let Tree {
        r,
        mut document,
        folded,
        selected,
        text_links,
        ..
    } = tree;
    *document.tags_mut() = tags;
    let undecodable = undecodable.saturating_add(r.undecodable);
    Ok(Parsed {
        version,
        document,
        images,
        folded,
        selected,
        selection_size,
        zoom_level,
        autoexport,
        text_links,
        undecodable,
    })
}

fn read_image(
    r: &mut Reader,
    version: u8,
    kind: ImageKind,
    ids: &mut dyn IdSource,
) -> Result<Image, CtsError> {
    if version < 9 {
        // A name that TreeSheets itself reads and ignores.
        r.string()?;
    }
    let scale = if version >= 19 { r.extended()? } else { 1.0 };
    // As TreeSheets: "a scale from a file can be anything".
    let scale = if scale.is_finite() {
        scale.clamp(1.0 / 16.0, 100.0)
    } else {
        1.0
    };
    let data = if version >= 22 {
        match usize::try_from(r.i64()?) {
            Ok(len) if len <= r.remaining() => r.take(len)?,
            _ => return corrupt("an image is longer than the file"),
        }
    } else {
        // No length: the image ends where its own format says it does.
        let len = match kind {
            ImageKind::Png => png_len(r.rest()),
            ImageKind::Jpeg => jpeg_len(r.rest()),
        };
        let Some(len) = len else {
            return corrupt("an image stored without a length is damaged");
        };
        if kind == ImageKind::Png {
            r.big_endian = true;
        }
        r.take(len)?
    };
    Ok(Image {
        file: format!("{}.{}", ids.next_id(), kind.extension()),
        kind,
        scale,
        data: data.to_vec(),
        used: false,
    })
}

/// The length of the PNG file at the start of `data`: the signature, then chunks up to and
/// including `IEND`.
fn png_len(data: &[u8]) -> Option<usize> {
    if data.get(..8)? != [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'] {
        return None;
    }
    let mut pos = 8usize;
    loop {
        let len =
            u32::from_be_bytes(<[u8; 4]>::try_from(data.get(pos..pos.checked_add(4)?)?).ok()?);
        if len > 0x7fff_ffff {
            return None;
        }
        let name_at = pos.checked_add(4)?;
        let name = data.get(name_at..name_at.checked_add(4)?)?;
        // Length, name, data, checksum.
        pos = pos
            .checked_add(12)?
            .checked_add(usize::try_from(len).ok()?)?;
        if pos > data.len() {
            return None;
        }
        if name == b"IEND" {
            return Some(pos);
        }
    }
}

/// The length of the JPEG file at the start of `data`: from the start marker up to and
/// including the end marker, stepping over segments and over the compressed scans.
fn jpeg_len(data: &[u8]) -> Option<usize> {
    if data.get(..2)? != [0xff, 0xd8] {
        return None;
    }
    let mut pos = 2usize;
    loop {
        // Find the next marker: 0xff, then a byte that is neither stuffing (0) nor fill (0xff).
        while *data.get(pos)? != 0xff {
            pos = pos.checked_add(1)?;
        }
        let marker = *data.get(pos.checked_add(1)?)?;
        match marker {
            0xff => pos = pos.checked_add(1)?,
            0x00 | 0x01 | 0xd0..=0xd8 => pos = pos.checked_add(2)?,
            0xd9 => return pos.checked_add(2),
            _ => {
                let at = pos.checked_add(2)?;
                let len = u16::from_be_bytes(
                    <[u8; 2]>::try_from(data.get(at..at.checked_add(2)?)?).ok()?,
                );
                if len < 2 {
                    return None;
                }
                pos = at.checked_add(usize::from(len))?;
            }
        }
    }
}

/// The smallest number of bytes one cell takes in a file of this version: a cell with neither
/// text nor grid and an empty note. It bounds how many cells a grid can really have.
fn min_cell_bytes(version: u8) -> usize {
    let at_least = |v: u8, bytes: usize| if version >= v { bytes } else { 0 };
    2 + at_least(8, 8) + at_least(15, 1) + at_least(25, 4) + at_least(28, 1) + at_least(29, 1)
}

struct Tree<'a> {
    r: Reader<'a>,
    version: u8,
    document: Document,
    ids: &'a mut dyn IdSource,
    images: &'a [Image],
    folded: Vec<CellId>,
    selected: Option<CellId>,
    text_links: Vec<(CellId, Vec<TextLink>)>,
}

/// The cells of one grid that are still to be read.
struct Pending {
    cells: Vec<CellId>,
    next: usize,
    /// The relative size of the cell that has the grid, which its cells start with.
    rel_size: i32,
}

impl Tree<'_> {
    /// Reads the root cell and everything in it.
    fn read(&mut self) -> Result<(), CtsError> {
        let root = self.document.root();
        let mut stack = Vec::new();
        stack.extend(self.read_cell(root, 0)?);
        // TreeSheets: "the rest of the program assumes the root cell has a grid".
        if stack.is_empty() {
            return corrupt("the root cell has no grid");
        }
        while let Some(top) = stack.last_mut() {
            let Some(&id) = top.cells.get(top.next) else {
                stack.pop();
                continue;
            };
            top.next = top.next.saturating_add(1);
            let rel_size = top.rel_size;
            stack.extend(self.read_cell(id, rel_size)?);
        }
        Ok(())
    }

    /// Reads one cell into the (already made, empty) cell `id`. If it has a grid, makes the
    /// grid and returns its cells, which follow in the file.
    fn read_cell(&mut self, id: CellId, inherited_size: i32) -> Result<Option<Pending>, CtsError> {
        let v = self.version;
        let kind = match self.r.u8()? {
            0 => CellKind::Data,
            1 => CellKind::Code,
            2 => CellKind::VarAssign,
            3 => CellKind::ViewH,
            4 => CellKind::VarRead,
            5 => CellKind::ViewV,
            6 => CellKind::Formula,
            other => return corrupt(format!("unknown cell type {other}")),
        };
        let (cell_color, text_color) = if v >= 8 {
            (color(self.r.u32()?), color(self.r.u32()?))
        } else {
            (Color::WHITE, Color::BLACK)
        };
        let draw_style = match if v >= 15 { self.r.u8()? } else { 0 } {
            0 => DrawStyle::Grid,
            1 => DrawStyle::Bubbles,
            2 => DrawStyle::Lines,
            other => return corrupt(format!("unknown draw style {other}")),
        };
        let note = if v >= 25 {
            self.r.string()?
        } else {
            String::new()
        };
        if code_points(&note) > MAX_TEXT_CODE_POINTS {
            return Err(CtsError::Limit("a note is too long".into()));
        }
        // Out-of-range alignments are automatic, as in TreeSheets.
        let text_align = match if v >= 28 { self.r.u8()? } else { 0 } {
            1 => TextAlign::Left,
            2 => TextAlign::Center,
            3 => TextAlign::Right,
            _ => TextAlign::Auto,
        };
        let vert_align = match if v >= 29 { self.r.u8()? } else { 0 } {
            1 => VertAlign::Top,
            2 => VertAlign::Middle,
            3 => VertAlign::Bottom,
            _ => VertAlign::Auto,
        };
        let mut contents = self.r.u8()?;
        if contents & SELECTION_MASK != 0 {
            // If several cells are marked, TreeSheets ends up with the last one.
            self.selected = Some(id);
            contents &= !SELECTION_MASK;
        }
        let (has_text, has_grid) = match contents {
            0 => (true, false),
            1 => (false, true),
            2 => (true, true),
            3 => (false, false),
            other => return corrupt(format!("unknown cell contents {other}")),
        };
        let text = if has_text {
            Some(self.read_text(kind)?)
        } else {
            None
        };
        let rel_size = text.as_ref().map_or(inherited_size, |t| t.rel_size);

        let Some(cell) = self.document.cell_mut(id) else {
            return corrupt("internal error: a cell is missing");
        };
        cell.kind = kind;
        cell.cell_color = cell_color;
        cell.text_color = text_color;
        cell.draw_style = draw_style;
        cell.note = note;
        cell.text_align = text_align;
        cell.vert_align = vert_align;
        // A cell without text keeps the size it was created with, which is its parent's.
        cell.rel_size = rel_size;
        if let Some(text) = text {
            cell.text = text.text;
            cell.style = style(text.style_bits);
            cell.last_edit = text.last_edit;
            cell.runs = text.runs;
            cell.links = text.links;
            cell.image = text.image;
            if !text.text_links.is_empty() {
                self.text_links.push((id, text.text_links));
            }
        }
        if !has_grid {
            return Ok(None);
        }
        let cells = self.read_grid(id)?;
        Ok(Some(Pending {
            cells,
            next: 0,
            rel_size,
        }))
    }

    /// Reads the grid of cell `id` up to where its cells start, and returns the new cells.
    fn read_grid(&mut self, id: CellId) -> Result<Vec<CellId>, CtsError> {
        let v = self.version;
        let (xs, ys) = (self.r.i32()?, self.r.i32()?);
        let count = i64::from(xs).saturating_mul(i64::from(ys));
        let (Ok(xs), Ok(ys)) = (u32::try_from(xs), u32::try_from(ys)) else {
            return corrupt("a grid has a negative size");
        };
        if xs < 1 || ys < 1 {
            return corrupt("a grid has no cells");
        }
        if count > MAX_GRID_CELLS {
            return Err(CtsError::Limit(format!(
                "a grid has {count} cells; TreeSheets allows {MAX_GRID_CELLS}"
            )));
        }
        // Before any cell is made: there must be at least enough data left for that many.
        let needed = usize::try_from(count)
            .ok()
            .and_then(|n| n.checked_mul(min_cell_bytes(v)));
        if needed.is_none_or(|needed| needed > self.r.remaining()) {
            return corrupt(format!(
                "a {xs} by {ys} grid cannot fit in the rest of the file"
            ));
        }
        self.document.add_grid(id, xs, ys, self.ids)?;

        let mut border = Color::BORDER;
        let mut spacing = wombat_core::Grid::DEFAULT_OUTER_SPACING;
        let mut vertical = None;
        let mut folded = false;
        let mut widths = None;
        if v >= 10 {
            border = color(self.r.u32()?);
            spacing = u32::try_from(self.r.i32()?.clamp(0, 32)).unwrap_or(0);
        }
        if v >= 11 {
            vertical = Some(self.r.u8()? != 0);
        }
        if v >= 16 {
            folded = self.r.u8()? != 0;
        }
        if v >= 13 {
            let mut read = Vec::new();
            for _ in 0..xs {
                read.push(u32::try_from(self.r.i32()?.max(5)).unwrap_or(5));
            }
            widths = Some(read);
        }
        if folded {
            self.folded.push(id);
        }
        let Some(cell) = self.document.cell_mut(id) else {
            return corrupt("internal error: a cell is missing");
        };
        if folded && (16..=17).contains(&v) {
            // "Before v18, folding would use the image slot."
            cell.image = None;
        }
        let Some(grid) = cell.grid_mut() else {
            return corrupt("internal error: a grid is missing");
        };
        grid.border_color = border;
        grid.outer_spacing = spacing;
        // Before version 11 every grid is below its cell's text, TreeSheets' default.
        grid.vertical = vertical.unwrap_or(true);
        grid.col_widths = widths.unwrap_or_else(|| vec![OLD_COLUMN_WIDTH; grid.col_widths.len()]);
        Ok(grid.cells().to_vec())
    }

    fn read_text(&mut self, kind: CellKind) -> Result<TextPart, CtsError> {
        let v = self.version;
        let mut text = filter_controls(self.r.string()?);
        let len = code_points(&text);
        if len > MAX_TEXT_CODE_POINTS {
            return Err(CtsError::Limit("a text is too long".into()));
        }
        let len = i64::try_from(len).unwrap_or(i64::MAX);
        if v <= 11 {
            self.r.u32()?; // A number of lines that TreeSheets ignores.
        }
        let rel_size = self.r.i32()?;
        let image_index = self.r.i32()?;
        let style_bits = if v >= 7 { self.r.u32()? } else { 0 };
        let last_edit = if v >= 14 { Some(self.r.i64()?) } else { None };
        let image = usize::try_from(image_index)
            .ok()
            .and_then(|index| self.images.get(index))
            .map(|image| ImageRef {
                scale: image.scale,
                ..ImageRef::new(image.file.clone())
            });

        let mut runs = Vec::new();
        let mut links = Vec::new();
        let mut text_links = Vec::new();
        if v >= 27 {
            let count = self.r.u32()?;
            if i64::from(count) > len {
                return corrupt("a cell has more runs than characters");
            }
            let mut raw = Vec::new();
            let mut pos = 0i64;
            for _ in 0..count {
                let start = i64::from(self.r.i32()?);
                let length = i64::from(self.r.i32()?);
                let bits = self.r.u32()?;
                let col = self.r.u32()?;
                // As TreeSheets: keep runs sorted, disjoint and inside the text.
                let start = start.max(pos);
                let length = length.min(len.saturating_sub(start));
                if length > 0 {
                    pos = start.saturating_add(length);
                    raw.push((
                        start,
                        length,
                        bits,
                        (col & RUN_HAS_COLOR != 0).then(|| color(col)),
                    ));
                }
            }
            runs = normalize_runs(raw, style_bits);
        }
        if v >= 31 {
            let count = self.r.u32()?;
            if i64::from(count) > len {
                return corrupt("a cell has more links than characters");
            }
            let mut pos = 0i64;
            for _ in 0..count {
                let start = i64::from(self.r.i32()?);
                let length = i64::from(self.r.i32()?);
                let internal = self.r.u8()? != 0;
                let target = self.r.string()?;
                let start = start.max(pos);
                let length = length.min(len.saturating_sub(start));
                if length <= 0 || target.is_empty() {
                    continue;
                }
                pos = start.saturating_add(length);
                let (Ok(start), Ok(length)) = (u32::try_from(start), u32::try_from(length)) else {
                    continue;
                };
                if internal {
                    text_links.push(TextLink {
                        start,
                        len: length,
                        target,
                    });
                } else {
                    links.push(Link {
                        start,
                        len: length,
                        target: LinkTarget::External(target),
                        extra: Extra::new(),
                    });
                }
            }
        }
        // TreeSheets keeps the formula of a formula cell with a leading "=".
        if kind == CellKind::Formula && !text.starts_with('=') {
            text.insert(0, '=');
            for run in &mut runs {
                run.start = run.start.saturating_add(1);
            }
            for link in &mut links {
                link.start = link.start.saturating_add(1);
            }
            for link in &mut text_links {
                link.start = link.start.saturating_add(1);
            }
        }
        Ok(TextPart {
            text,
            rel_size,
            style_bits,
            last_edit,
            image,
            runs,
            links,
            text_links,
        })
    }

    /// Reads the tags that follow the cells.
    fn read_tags(&mut self) -> Result<BTreeMap<String, Tag>, CtsError> {
        let mut tags = BTreeMap::new();
        if self.version < 11 {
            return Ok(tags);
        }
        loop {
            // Some old files end after the last tag, without the empty name that closes the
            // list (an example of version 24 does); TreeSheets reads the missing name as empty.
            if self.r.remaining() < 4 {
                return Ok(tags);
            }
            let name = filter_controls(self.r.string()?);
            if name.is_empty() {
                return Ok(tags);
            }
            let (cell_color, text_color) = if self.version >= 24 {
                (color(self.r.u32()?), color(self.r.u32()?))
            } else {
                // TreeSheets' defaults: white, and 0xFF0000 as blue-green-red, which is blue.
                (Color::WHITE, Color::rgb(0, 0, 0xff))
            };
            tags.insert(
                name,
                Tag {
                    cell_color,
                    text_color,
                    extra: Extra::new(),
                },
            );
        }
    }
}

/// The text part of a cell.
struct TextPart {
    text: String,
    rel_size: i32,
    style_bits: u32,
    last_edit: Option<i64>,
    image: Option<ImageRef>,
    runs: Vec<Run>,
    links: Vec<Link>,
    text_links: Vec<TextLink>,
}

/// TreeSheets' `TextRuns::Normalize`: drops runs that do not differ from the cell's style and
/// merges neighbours that are styled the same. Runs are `(start, length, style bits, colour)`.
fn normalize_runs(raw: Vec<(i64, i64, u32, Option<Color>)>, base_bits: u32) -> Vec<Run> {
    let mut merged: Vec<(i64, i64, u32, Option<Color>)> = Vec::new();
    for run in raw {
        let (start, length, bits, col) = run;
        if bits == base_bits && col.is_none() {
            continue;
        }
        match merged.last_mut() {
            Some(last)
                if last.0.saturating_add(last.1) == start && last.2 == bits && last.3 == col =>
            {
                last.1 = last.1.saturating_add(length);
            }
            _ => merged.push(run),
        }
    }
    merged
        .into_iter()
        .filter_map(|(start, length, bits, col)| {
            Some(Run {
                start: u32::try_from(start).ok()?,
                len: u32::try_from(length).ok()?,
                style: style(bits),
                color: col,
                extra: Extra::new(),
            })
        })
        .collect()
}
