//! The `.wombat` document format: JSON text, specified in `docs/FILE_FORMAT.md`.
//!
//! [`to_string`] always gives the same bytes for the same document, and [`from_slice`] never
//! panics, hangs or allocates without bound, whatever it is given (decisions A7 and A12).

use std::collections::{BTreeMap, HashMap};

use serde_json::Value;

use crate::error::{invalid, Error};
use crate::id::{CellId, DocId};
use crate::json::{
    into_extra, into_object, parse, take_array, take_bool, take_color, take_header, take_i32,
    take_i64, take_id, take_string, take_u32, Fields, Kind, Map,
};
use crate::limits::{MAX_CELLS, MAX_DEPTH, MAX_FILE_BYTES};
use crate::model::{
    Cell, CellKind, Color, Document, DrawStyle, Grid, ImageRef, Link, LinkTarget, Run, Style, Tag,
    TextAlign, VertAlign,
};

/// The value of the `format` field.
pub const FORMAT_NAME: &str = "wombat";
/// The major format version this core writes, and the newest it reads.
pub const FORMAT_VERSION: u32 = 1;
/// The minor format version this core writes.
pub const FORMAT_MINOR: u32 = 0;

const KIND: Kind = Kind {
    format: FORMAT_NAME,
    version: FORMAT_VERSION,
    id_key: "id",
};

pub(crate) const DOCUMENT_KEYS: &[&str] = &["format", "version", "minor", "id", "tags", "root"];
pub(crate) const TAG_KEYS: &[&str] = &["color", "text_color"];
pub(crate) const STYLE_KEYS: [&str; 5] = ["bold", "italic", "fixed", "underline", "strike"];
pub(crate) const CELL_KEYS: &[&str] = &[
    "id",
    "text",
    "kind",
    "color",
    "text_color",
    "draw",
    "align",
    "valign",
    "size",
    "bold",
    "italic",
    "fixed",
    "underline",
    "strike",
    "runs",
    "links",
    "note",
    "edited",
    "image",
    "grid",
];
pub(crate) const RUN_KEYS: &[&str] = &[
    "start",
    "len",
    "bold",
    "italic",
    "fixed",
    "underline",
    "strike",
    "color",
];
pub(crate) const LINK_KEYS: &[&str] = &["start", "len", "url", "cell", "doc"];
pub(crate) const IMAGE_KEYS: &[&str] = &["file", "scale"];
pub(crate) const GRID_KEYS: &[&str] = &[
    "xs", "ys", "border", "spacing", "vertical", "widths", "cells",
];

/// Refuses text that has grown too long to be loaded again (decision A22).
pub(crate) fn check_size(out: &str) -> Result<(), Error> {
    if out.len() as u64 > MAX_FILE_BYTES {
        return Err(Error::Limit(format!(
            "the file would be larger than {MAX_FILE_BYTES} bytes"
        )));
    }
    Ok(())
}

/// Writes a document as `.wombat` text. Fails if the document is not valid or the text would
/// be longer than [`MAX_FILE_BYTES`]; whatever it returns, [`from_slice`] reads back.
pub fn to_string(doc: &Document) -> Result<String, Error> {
    doc.validate()?;
    let mut out = String::new();
    out.push_str("{\n");
    let mut head = Fields::new(&mut out);
    head.string("format", FORMAT_NAME);
    head.display("version", FORMAT_VERSION);
    head.display(
        "minor",
        crate::json::minor_to_write(doc.minor, FORMAT_MINOR),
    );
    head.quoted("id", doc.id);
    out.push_str(",\n");
    if !doc.tags.is_empty() {
        out.push_str("\"tags\": {");
        for (i, (name, tag)) in doc.tags.iter().enumerate() {
            out.push_str(if i == 0 { "\n " } else { ",\n " });
            crate::json::write_string(&mut out, name);
            out.push_str(": {");
            let mut f = Fields::new(&mut out);
            if tag.cell_color != Color::WHITE {
                f.quoted("color", tag.cell_color);
            }
            if tag.text_color != Color::BLACK {
                f.quoted("text_color", tag.text_color);
            }
            f.extra(&tag.extra)?;
            out.push('}');
        }
        out.push_str("\n},\n");
    }
    for (key, value) in &doc.extra {
        crate::json::write_string(&mut out, key);
        out.push_str(": ");
        crate::json::write_value(&mut out, value)?;
        out.push_str(",\n");
    }
    out.push_str("\"root\": ");
    write_cell(doc, doc.root, 0, &mut out)?;
    out.push_str("\n}\n");
    check_size(&out)?;
    Ok(out)
}

fn write_style(f: &mut Fields<'_>, style: Style) {
    let Style {
        bold,
        italic,
        fixed,
        underline,
        strike,
    } = style;
    for (key, on) in STYLE_KEYS
        .iter()
        .zip([bold, italic, fixed, underline, strike])
    {
        if on {
            f.display(key, true);
        }
    }
}

/// Writes one cell on one line, followed by the cells of its grid, one per line, indented by
/// one more space than the cell itself. `indent` is also the depth, which `validate` bounded.
fn write_cell(doc: &Document, id: CellId, indent: usize, out: &mut String) -> Result<(), Error> {
    let Some(cell) = doc.cells.get(&id) else {
        return invalid(format!("cell {id} does not exist"));
    };
    out.push('{');
    let mut f = Fields::new(out);
    f.quoted("id", id);
    if !cell.text.is_empty() {
        f.string("text", &cell.text);
    }
    if cell.kind != CellKind::Data {
        f.string("kind", cell.kind.as_str());
    }
    if cell.cell_color != Color::WHITE {
        f.quoted("color", cell.cell_color);
    }
    if cell.text_color != Color::BLACK {
        f.quoted("text_color", cell.text_color);
    }
    if cell.draw_style != DrawStyle::Grid {
        f.string("draw", cell.draw_style.as_str());
    }
    if cell.text_align != TextAlign::Auto {
        f.string("align", cell.text_align.as_str());
    }
    if cell.vert_align != VertAlign::Auto {
        f.string("valign", cell.vert_align.as_str());
    }
    if cell.rel_size != 0 {
        f.display("size", cell.rel_size);
    }
    write_style(&mut f, cell.style);
    if !cell.runs.is_empty() {
        f.key("runs");
        f.out.push('[');
        for (i, run) in cell.runs.iter().enumerate() {
            f.out.push_str(if i == 0 { "{" } else { ", {" });
            let mut r = Fields::new(f.out);
            r.display("start", run.start);
            r.display("len", run.len);
            write_style(&mut r, run.style);
            if let Some(color) = run.color {
                r.quoted("color", color);
            }
            r.extra(&run.extra)?;
            f.out.push('}');
        }
        f.out.push(']');
    }
    if !cell.links.is_empty() {
        f.key("links");
        f.out.push('[');
        for (i, link) in cell.links.iter().enumerate() {
            f.out.push_str(if i == 0 { "{" } else { ", {" });
            let mut l = Fields::new(f.out);
            l.display("start", link.start);
            l.display("len", link.len);
            match &link.target {
                LinkTarget::External(url) => l.string("url", url),
                LinkTarget::Cell { doc, cell } => {
                    l.quoted("cell", cell);
                    if let Some(doc) = doc {
                        l.quoted("doc", doc);
                    }
                }
                LinkTarget::Unknown => {}
            }
            l.extra(&link.extra)?;
            f.out.push('}');
        }
        f.out.push(']');
    }
    if !cell.note.is_empty() {
        f.string("note", &cell.note);
    }
    if let Some(edited) = cell.last_edit {
        f.display("edited", edited);
    }
    if let Some(image) = &cell.image {
        f.key("image");
        f.out.push('{');
        let mut m = Fields::new(f.out);
        m.string("file", &image.file);
        if image.scale != 1.0 {
            m.key("scale");
            crate::json::write_value(m.out, &Value::from(image.scale))?;
        }
        m.extra(&image.extra)?;
        f.out.push('}');
    }
    f.extra(&cell.extra)?;
    if let Some(grid) = &cell.grid {
        f.key("grid");
        f.out.push('{');
        let mut g = Fields::new(f.out);
        g.display("xs", grid.xs);
        g.display("ys", grid.ys);
        if grid.border_color != Color::BORDER {
            g.quoted("border", grid.border_color);
        }
        if grid.outer_spacing != Grid::DEFAULT_OUTER_SPACING {
            g.display("spacing", grid.outer_spacing);
        }
        if !grid.vertical {
            g.display("vertical", false);
        }
        if grid
            .col_widths
            .iter()
            .any(|w| *w != Grid::DEFAULT_COL_WIDTH)
        {
            g.key("widths");
            let widths: Vec<String> = grid.col_widths.iter().map(u32::to_string).collect();
            g.out.push('[');
            g.out.push_str(&widths.join(", "));
            g.out.push(']');
        }
        g.extra(&grid.extra)?;
        g.key("cells");
        out.push('[');
        let child_indent = indent.saturating_add(1);
        for (i, child) in grid.cells.iter().enumerate() {
            out.push_str(if i == 0 { "\n" } else { ",\n" });
            out.extend(std::iter::repeat_n(' ', child_indent));
            write_cell(doc, *child, child_indent, out)?;
        }
        out.push('\n');
        out.extend(std::iter::repeat_n(' ', indent));
        out.push_str("]}");
    }
    out.push('}');
    // After every cell, so that a document far over the limit stops early.
    check_size(out)
}

/// Reads only the header of `.wombat` text and returns the document's ID, without building
/// the document and without reading past the header, so the rest of the file is not checked:
/// a file that is damaged further on still gives its ID here and an error when loaded. The
/// caller bounds how much `reader` gives.
pub fn read_id(reader: impl std::io::Read) -> Result<DocId, Error> {
    crate::json::read_id(reader, KIND).map(DocId)
}

/// Reads `.wombat` text. Any input gives either a valid document or an error.
pub fn from_slice(bytes: &[u8]) -> Result<Document, Error> {
    let mut map = into_object(parse(bytes, KIND)?, "the file")?;
    let minor = take_header(&mut map, KIND)?;
    let Some(id) = take_id(&mut map, "id", "the document")? else {
        return invalid("the document has no \"id\"");
    };
    let mut tags = BTreeMap::new();
    if let Some(value) = map.remove("tags") {
        for (name, value) in into_object(value, "\"tags\"")? {
            let place = format!("tag {name:?}");
            let mut tag = into_object(value, &place)?;
            let cell_color = take_color(&mut tag, "color", &place)?.unwrap_or(Color::WHITE);
            let text_color = take_color(&mut tag, "text_color", &place)?.unwrap_or(Color::BLACK);
            tags.insert(
                name,
                Tag {
                    cell_color,
                    text_color,
                    extra: into_extra(tag),
                },
            );
        }
    }
    let Some(root_value) = map.remove("root") else {
        return invalid("the document has no \"root\" cell");
    };
    let mut cells = HashMap::new();
    let root = read_cell(root_value, None, 0, DocId(id), &mut cells)?;
    let doc = Document {
        id: DocId(id),
        minor,
        root,
        cells,
        tags,
        extra: into_extra(map),
    };
    doc.validate()?;
    Ok(doc)
}

fn read_style(map: &mut Map, place: &str) -> Result<Style, Error> {
    Ok(Style {
        bold: take_bool(map, "bold", place)?,
        italic: take_bool(map, "italic", place)?,
        fixed: take_bool(map, "fixed", place)?,
        underline: take_bool(map, "underline", place)?,
        strike: take_bool(map, "strike", place)?,
    })
}

fn read_name<T>(
    map: &mut Map,
    key: &str,
    place: &str,
    parse: fn(&str) -> Option<T>,
) -> Result<Option<T>, Error> {
    match take_string(map, key, place)? {
        None => Ok(None),
        Some(name) => match parse(&name) {
            Some(value) => Ok(Some(value)),
            None => invalid(format!("{place}: {key:?} has the unknown value {name:?}")),
        },
    }
}

fn read_range(map: &mut Map, place: &str) -> Result<(u32, u32), Error> {
    match (take_u32(map, "start", place)?, take_u32(map, "len", place)?) {
        (Some(start), Some(len)) => Ok((start, len)),
        _ => invalid(format!("{place} needs \"start\" and \"len\"")),
    }
}

/// Reads a cell and, recursively, its grid. The recursion is at most `MAX_DEPTH` deep.
fn read_cell(
    value: Value,
    parent: Option<CellId>,
    depth: u32,
    doc: DocId,
    cells: &mut HashMap<CellId, Cell>,
) -> Result<CellId, Error> {
    let mut map = into_object(value, "a cell")?;
    let Some(id) = take_id(&mut map, "id", "a cell")?.map(CellId) else {
        return invalid("a cell has no \"id\"");
    };
    if id.0 == doc.0 || cells.contains_key(&id) {
        return invalid(format!("the ID {id} is used more than once"));
    }
    if cells.len() >= MAX_CELLS {
        return Err(Error::Limit(format!("more than {MAX_CELLS} cells")));
    }
    let place = format!("cell {id}");
    let mut cell = Cell::new(id, parent);
    cell.text = take_string(&mut map, "text", &place)?.unwrap_or_default();
    cell.kind = read_name(&mut map, "kind", &place, CellKind::parse)?.unwrap_or_default();
    cell.cell_color = take_color(&mut map, "color", &place)?.unwrap_or(Color::WHITE);
    cell.text_color = take_color(&mut map, "text_color", &place)?.unwrap_or(Color::BLACK);
    cell.draw_style = read_name(&mut map, "draw", &place, DrawStyle::parse)?.unwrap_or_default();
    cell.text_align = read_name(&mut map, "align", &place, TextAlign::parse)?.unwrap_or_default();
    cell.vert_align = read_name(&mut map, "valign", &place, VertAlign::parse)?.unwrap_or_default();
    cell.rel_size = take_i32(&mut map, "size", &place)?.unwrap_or(0);
    cell.style = read_style(&mut map, &place)?;
    for value in take_array(&mut map, "runs", &place)?.unwrap_or_default() {
        let place = format!("a run of {place}");
        let mut run = into_object(value, &place)?;
        let (start, len) = read_range(&mut run, &place)?;
        let style = read_style(&mut run, &place)?;
        let color = take_color(&mut run, "color", &place)?;
        cell.runs.push(Run {
            start,
            len,
            style,
            color,
            extra: into_extra(run),
        });
    }
    for value in take_array(&mut map, "links", &place)?.unwrap_or_default() {
        let place = format!("a link of {place}");
        let mut link = into_object(value, &place)?;
        let (start, len) = read_range(&mut link, &place)?;
        let url = take_string(&mut link, "url", &place)?;
        let target_cell = take_id(&mut link, "cell", &place)?.map(CellId);
        let target_doc = take_id(&mut link, "doc", &place)?.map(DocId);
        let target = match (url, target_cell, target_doc) {
            (Some(url), None, None) => LinkTarget::External(url),
            (None, Some(cell), doc) => LinkTarget::Cell { doc, cell },
            (None, None, None) => LinkTarget::Unknown,
            _ => return invalid(format!("{place} has a contradictory target")),
        };
        cell.links.push(Link {
            start,
            len,
            target,
            extra: into_extra(link),
        });
    }
    cell.note = take_string(&mut map, "note", &place)?.unwrap_or_default();
    cell.last_edit = take_i64(&mut map, "edited", &place)?;
    if let Some(value) = map.remove("image") {
        let place = format!("the image of {place}");
        let mut image = into_object(value, &place)?;
        let Some(file) = take_string(&mut image, "file", &place)? else {
            return invalid(format!("{place} has no \"file\""));
        };
        let scale = match image.remove("scale") {
            None => 1.0,
            Some(value) => match value.as_f64() {
                Some(scale) => scale,
                None => return invalid(format!("{place}: \"scale\" is not a number")),
            },
        };
        cell.image = Some(ImageRef {
            file,
            scale,
            extra: into_extra(image),
        });
    }
    let grid_value = map.remove("grid");
    cell.extra = into_extra(map);
    // The cell goes in before its children, so that a child repeating its ID is caught.
    cells.insert(id, cell);
    if let Some(value) = grid_value {
        if depth >= MAX_DEPTH {
            return Err(Error::Limit(format!(
                "grids nested deeper than {MAX_DEPTH}"
            )));
        }
        let place = format!("the grid of {place}");
        let mut map = into_object(value, &place)?;
        let (Some(xs), Some(ys), Some(children)) = (
            take_u32(&mut map, "xs", &place)?,
            take_u32(&mut map, "ys", &place)?,
            take_array(&mut map, "cells", &place)?,
        ) else {
            return invalid(format!("{place} needs \"xs\", \"ys\" and \"cells\""));
        };
        if xs == 0 || ys == 0 || u64::from(xs) * u64::from(ys) != children.len() as u64 {
            return invalid(format!(
                "{place}: {xs} by {ys} is not {} cells",
                children.len()
            ));
        }
        // Nothing is allocated from xs and ys before this point: the cells really are there.
        let mut grid = Grid::new(xs, ys, Vec::with_capacity(children.len()));
        grid.border_color = take_color(&mut map, "border", &place)?.unwrap_or(Color::BORDER);
        grid.outer_spacing =
            take_u32(&mut map, "spacing", &place)?.unwrap_or(Grid::DEFAULT_OUTER_SPACING);
        grid.vertical = match map.remove("vertical") {
            None => true,
            Some(Value::Bool(b)) => b,
            Some(_) => return invalid(format!("{place}: \"vertical\" is not true or false")),
        };
        if let Some(widths) = take_array(&mut map, "widths", &place)? {
            grid.col_widths = widths
                .iter()
                .map(|w| w.as_u64().and_then(|w| u32::try_from(w).ok()))
                .collect::<Option<Vec<u32>>>()
                .filter(|w| w.len() == xs as usize)
                .ok_or_else(|| {
                    Error::Invalid(format!("{place}: \"widths\" is not {xs} whole numbers"))
                })?;
        }
        grid.extra = into_extra(map);
        for child in children {
            grid.cells.push(read_cell(
                child,
                Some(id),
                depth.saturating_add(1),
                doc,
                cells,
            )?);
        }
        if let Some(cell) = cells.get_mut(&id) {
            cell.grid = Some(grid);
        }
    }
    Ok(id)
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
    use crate::id::{Id, IdSource, SeededIds};
    use crate::model::Extra;
    use serde_json::json;

    /// IDs 1, 2, 3, ... so that expected file text is readable.
    struct Counter(u128);
    impl IdSource for Counter {
        fn next_id(&mut self) -> Id {
            self.0 += 1;
            Id::from_u128(self.0)
        }
    }

    fn id(n: u128) -> String {
        Id::from_u128(n).to_string()
    }

    #[test]
    fn empty_document_text() {
        let doc = Document::new(&mut Counter(0));
        let text = to_string(&doc).unwrap();
        assert_eq!(
            text,
            format!(
                "{{\n\"format\": \"wombat\", \"version\": 1, \"minor\": 0, \"id\": \"{}\",\n\
                 \"root\": {{\"id\": \"{}\"}}\n}}\n",
                id(1),
                id(2)
            )
        );
        assert_eq!(from_slice(text.as_bytes()).unwrap(), doc);
    }

    /// A document that uses every field of the model.
    fn full_document() -> Document {
        let mut ids = Counter(0);
        let mut doc = Document::new(&mut ids);
        let root = doc.root();
        doc.add_grid(root, 2, 2, &mut ids).unwrap();
        doc.tags.insert(
            "todo".into(),
            Tag {
                text_color: Color::rgb(255, 0, 0),
                ..Tag::default()
            },
        );
        doc.tags.insert(
            "done".into(),
            Tag {
                cell_color: Color::rgb(0, 128, 0),
                ..Tag::default()
            },
        );
        let a = doc.child(root, 0, 0).unwrap();
        let b = doc.child(root, 1, 0).unwrap();
        let c = doc.child(root, 0, 1).unwrap();
        {
            let cell = doc.cell_mut(root).unwrap();
            cell.text = "Plan \"A\" \\ \u{1F600}".into();
            // Control characters may only be in notes (decision A32).
            cell.note = "two\n\tlines \u{1}".into();
            cell.draw_style = DrawStyle::Lines;
            let grid = cell.grid_mut().unwrap();
            grid.border_color = Color::rgb(1, 2, 3);
            grid.outer_spacing = 0;
            grid.vertical = false;
            grid.col_widths = vec![80, 20];
        }
        {
            let cell = doc.cell_mut(a).unwrap();
            cell.text = "bold and link".into();
            cell.kind = CellKind::Code;
            cell.cell_color = Color::rgb(0xff, 0xee, 0xdd);
            cell.text_color = Color::rgb(0, 0, 0x80);
            cell.text_align = TextAlign::Center;
            cell.vert_align = VertAlign::Bottom;
            cell.rel_size = -2;
            cell.style = Style {
                italic: true,
                fixed: true,
                ..Style::default()
            };
            cell.note = "a note".into();
            cell.last_edit = Some(1_731_234_567_890);
            cell.runs = vec![
                Run {
                    start: 0,
                    len: 4,
                    style: Style {
                        bold: true,
                        underline: true,
                        strike: true,
                        ..Style::default()
                    },
                    color: None,
                    extra: Extra::new(),
                },
                Run {
                    start: 9,
                    len: 4,
                    style: Style::default(),
                    color: Some(Color::rgb(0, 0, 255)),
                    extra: Extra::new(),
                },
            ];
            cell.links = vec![
                Link {
                    start: 0,
                    len: 4,
                    target: LinkTarget::External("https://example.org/?a=\"b\"".into()),
                    extra: Extra::new(),
                },
                Link {
                    start: 5,
                    len: 3,
                    target: LinkTarget::Cell { doc: None, cell: b },
                    extra: Extra::new(),
                },
                Link {
                    start: 9,
                    len: 4,
                    target: LinkTarget::Cell {
                        doc: Some(DocId(Id::from_u128(77))),
                        cell: c,
                    },
                    extra: Extra::new(),
                },
            ];
        }
        {
            let cell = doc.cell_mut(b).unwrap();
            cell.kind = CellKind::Formula;
            cell.text = "=SUM(Expenses)".into();
            cell.image = Some(ImageRef {
                scale: 0.5,
                ..ImageRef::new("cat.png")
            });
        }
        doc.cell_mut(c).unwrap().image = Some(ImageRef::new("dog.jpg"));
        doc.add_grid(c, 1, 1, &mut ids).unwrap();
        doc
    }

    #[test]
    fn full_document_text_and_round_trip() {
        let doc = full_document();
        let text = to_string(&doc).unwrap();
        let expected = r##"{
"format": "wombat", "version": 1, "minor": 0, "id": "ID1",
"tags": {
 "done": {"color": "#008000"},
 "todo": {"text_color": "#ff0000"}
},
"root": {"id": "ID2", "text": "Plan \"A\" \\ 😀", "draw": "lines", "note": "two\n\tlines \u0001", "grid": {"xs": 2, "ys": 2, "border": "#010203", "spacing": 0, "vertical": false, "widths": [80, 20], "cells": [
 {"id": "ID3", "text": "bold and link", "kind": "code", "color": "#ffeedd", "text_color": "#000080", "align": "center", "valign": "bottom", "size": -2, "italic": true, "fixed": true, "runs": [{"start": 0, "len": 4, "bold": true, "underline": true, "strike": true}, {"start": 9, "len": 4, "color": "#0000ff"}], "links": [{"start": 0, "len": 4, "url": "https://example.org/?a=\"b\""}, {"start": 5, "len": 3, "cell": "ID4"}, {"start": 9, "len": 4, "cell": "ID5", "doc": "ID77"}], "note": "a note", "edited": 1731234567890},
 {"id": "ID4", "text": "=SUM(Expenses)", "kind": "formula", "image": {"file": "cat.png", "scale": 0.5}},
 {"id": "ID5", "image": {"file": "dog.jpg"}, "grid": {"xs": 1, "ys": 1, "cells": [
  {"id": "ID7"}
 ]}},
 {"id": "ID6"}
]}}
}
"##;
        let mut expected = expected.to_string();
        for n in [77u128, 1, 2, 3, 4, 5, 6, 7] {
            expected = expected.replace(&format!("\"ID{n}\""), &format!("\"{}\"", id(n)));
        }
        assert_eq!(text, expected);
        let back = from_slice(text.as_bytes()).unwrap();
        assert_eq!(back, doc);
        assert_eq!(to_string(&back).unwrap(), text);
        // It is ordinary JSON to any other reader.
        let value: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value["root"]["grid"]["cells"][2]["grid"]["cells"][0]["id"],
            json!(id(7))
        );
    }

    /// A canonical file with fields no core knows, at every level.
    fn file_with_unknown_fields() -> String {
        let text = r##"{
"format": "wombat", "version": 1, "minor": 4, "id": "ID1",
"tags": {
 "t": {"color": "#000001", "shape": "round"}
},
"author": {"n":[1,2.5,-3,1e+300,null,true],"name":"x"},
"zebra": "last",
"root": {"id": "ID2", "text": "abc", "runs": [{"start": 0, "len": 1, "bold": true, "font": "serif"}], "links": [{"start": 1, "len": 1, "url": "u", "rel": "nofollow"}, {"start": 2, "len": 1, "anchor": {"page":3}}], "image": {"file": "a.png", "crop": [0,0,5,5]}, "date": "2026-01-01", "typed": {"as":"date"}, "grid": {"xs": 1, "ys": 1, "frozen": 1, "cells": [
 {"id": "ID3", "x": 0.1}
]}}
}
"##;
        let mut text = text.to_string();
        for n in [1u128, 2, 3] {
            text = text.replace(&format!("\"ID{n}\""), &format!("\"{}\"", id(n)));
        }
        text
    }

    #[test]
    fn unknown_fields_are_kept_at_every_level_and_written_back() {
        let text = file_with_unknown_fields();
        let doc = from_slice(text.as_bytes()).unwrap();
        assert_eq!(to_string(&doc).unwrap(), text);
        let root = doc.cell(doc.root()).unwrap();
        assert_eq!(doc.extra["zebra"], json!("last"));
        assert_eq!(doc.tags["t"].extra["shape"], json!("round"));
        assert_eq!(root.extra["typed"], json!({"as": "date"}));
        assert_eq!(root.runs[0].extra["font"], json!("serif"));
        assert_eq!(root.links[0].extra["rel"], json!("nofollow"));
        assert_eq!(root.links[1].target, LinkTarget::Unknown);
        assert_eq!(root.links[1].extra["anchor"], json!({"page": 3}));
        assert_eq!(
            root.image.as_ref().unwrap().extra["crop"],
            json!([0, 0, 5, 5])
        );
        assert_eq!(root.grid().unwrap().extra["frozen"], json!(1));
        let child = doc.child(doc.root(), 0, 0).unwrap();
        assert_eq!(doc.cell(child).unwrap().extra["x"], json!(0.1));
    }

    /// The valid document as a JSON value, to break in one place at a time.
    fn broken(change: impl FnOnce(&mut Value)) -> Result<Document, Error> {
        let mut value: Value = serde_json::from_str(&to_string(&full_document()).unwrap()).unwrap();
        change(&mut value);
        from_slice(value.to_string().as_bytes())
    }

    #[test]
    fn unchanged_value_loads() {
        assert_eq!(broken(|_| {}).unwrap(), full_document());
    }

    #[test]
    fn newer_major_version_is_refused_before_anything_else_is_read() {
        let err = from_slice(br#"{"format": "wombat", "version": 2, "root": 5}"#).unwrap_err();
        assert!(matches!(
            err,
            Error::NewerVersion {
                found: 2,
                supported: 1
            }
        ));
        assert!(err.to_string().contains("newer Wombat"));
        assert!(matches!(
            broken(|v| v["version"] = json!(u64::MAX)),
            Err(Error::NewerVersion { .. })
        ));
        assert!(matches!(
            broken(|v| v["version"] = json!(0)),
            Err(Error::Invalid(_))
        ));
        for bad in [json!("1"), json!(1.5), json!(-1), json!(null)] {
            assert!(matches!(
                broken(|v| v["version"] = bad),
                Err(Error::WrongFormat(_))
            ));
        }
    }

    #[test]
    fn newer_minor_version_loads_and_is_kept() {
        let doc = broken(|v| v["minor"] = json!(9)).unwrap();
        assert!(to_string(&doc)
            .unwrap()
            .contains("\"version\": 1, \"minor\": 9,"));
        assert!(broken(|v| v["minor"] = json!(-1)).is_err());
        let no_minor = broken(|v| {
            v.as_object_mut().unwrap().remove("minor");
        });
        assert_eq!(no_minor.unwrap(), full_document());
    }

    #[test]
    fn other_json_is_not_a_wombat_file() {
        for text in [
            "{}",
            r#"{"format": "wombat-view", "version": 1}"#,
            r#"{"format": 1}"#,
        ] {
            assert!(
                matches!(from_slice(text.as_bytes()), Err(Error::WrongFormat(_))),
                "{text}"
            );
        }
        for text in ["[]", "1", "null", "\"wombat\""] {
            assert!(
                matches!(from_slice(text.as_bytes()), Err(Error::Invalid(_))),
                "{text}"
            );
        }
        for text in [
            "",
            "{",
            "TSFF\u{1f}",
            "{\"format\": \"wombat\"} x",
            "\u{feff}{}",
        ] {
            assert!(
                matches!(from_slice(text.as_bytes()), Err(Error::Syntax(_))),
                "{text}"
            );
        }
        assert!(matches!(
            from_slice(&[0xff, 0xfe, 0x00]),
            Err(Error::Syntax(_))
        ));
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let cells = |v: &mut Value| v["root"]["grid"]["cells"].clone();
        // Two cells in one grid, a cell and its parent, a cell and a deeper cell, and a cell
        // and the document.
        let cases: [fn(&mut Value); 4] = [
            |v| v["root"]["grid"]["cells"][3]["id"] = v["root"]["grid"]["cells"][0]["id"].clone(),
            |v| v["root"]["grid"]["cells"][0]["id"] = v["root"]["id"].clone(),
            |v| {
                v["root"]["grid"]["cells"][2]["grid"]["cells"][0]["id"] =
                    v["root"]["grid"]["cells"][1]["id"].clone()
            },
            |v| v["root"]["grid"]["cells"][3]["id"] = v["id"].clone(),
        ];
        for case in cases {
            let err = broken(|v| {
                let _ = cells(v);
                case(v)
            })
            .unwrap_err();
            assert!(
                matches!(&err, Error::Invalid(m) if m.contains("more than once")),
                "{err}"
            );
        }
    }

    #[test]
    fn grid_shape_must_match_its_cells() {
        let cases: [fn(&mut Value); 9] = [
            |v| v["root"]["grid"]["xs"] = json!(3),
            |v| v["root"]["grid"]["ys"] = json!(0),
            |v| v["root"]["grid"]["xs"] = json!(4_294_967_295u64),
            |v| v["root"]["grid"]["xs"] = json!(4_294_967_296u64),
            |v| v["root"]["grid"]["xs"] = json!(2.0),
            |v| {
                v["root"]["grid"]["cells"].as_array_mut().unwrap().pop();
            },
            |v| {
                v["root"]["grid"].as_object_mut().unwrap().remove("cells");
            },
            |v| v["root"]["grid"]["widths"] = json!([80]),
            |v| v["root"]["grid"]["widths"] = json!([80, -1]),
        ];
        for (i, case) in cases.into_iter().enumerate() {
            assert!(matches!(broken(case), Err(Error::Invalid(_))), "case {i}");
        }
        // A huge claimed size with no cells is refused without allocating for it.
        let huge = format!(
            r#"{{"format":"wombat","version":1,"id":"{}","root":{{"id":"{}","grid":{{"xs":4294967295,"ys":4294967295,"cells":[]}}}}}}"#,
            id(1),
            id(2)
        );
        assert!(matches!(
            from_slice(huge.as_bytes()),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn runs_and_links_must_fit_the_text() {
        let cases: [fn(&mut Value); 9] = [
            |v| v["root"]["grid"]["cells"][0]["runs"][1]["len"] = json!(5),
            |v| v["root"]["grid"]["cells"][0]["runs"][1]["start"] = json!(3),
            |v| v["root"]["grid"]["cells"][0]["runs"][0]["len"] = json!(0),
            |v| v["root"]["grid"]["cells"][0]["runs"][0]["start"] = json!(4_294_967_295u64),
            |v| v["root"]["grid"]["cells"][0]["links"][1]["start"] = json!(3),
            |v| v["root"]["grid"]["cells"][0]["links"][2]["len"] = json!(5),
            |v| v["root"]["grid"]["cells"][0]["links"][0]["url"] = json!(""),
            |v| v["root"]["grid"]["cells"][0]["links"][0]["cell"] = json!(id(4)),
            |v| v["root"]["grid"]["cells"][0]["text"] = json!("short"),
        ];
        for (i, case) in cases.into_iter().enumerate() {
            assert!(matches!(broken(case), Err(Error::Invalid(_))), "case {i}");
        }
        // Positions are code points: 13 code points here, though 16 UTF-16 units.
        let emoji = "\u{1F600}old and\u{1F600}li\u{1F600}k";
        assert!(broken(|v| v["root"]["grid"]["cells"][0]["text"] = json!(emoji)).is_ok());
        assert!(broken(|v| v["root"]["grid"]["cells"][0]["text"] = json!(&emoji[4..])).is_err());
    }

    #[test]
    fn wrong_types_and_values_are_refused() {
        let cases: [fn(&mut Value); 16] = [
            |v| v["id"] = json!("1"),
            |v| v["id"] = json!(id(1).to_uppercase().replace('0', "A")),
            |v| {
                v.as_object_mut().unwrap().remove("id");
            },
            |v| {
                v.as_object_mut().unwrap().remove("root");
            },
            |v| v["root"] = json!([]),
            |v| v["tags"] = json!([]),
            |v| v["tags"]["todo"]["color"] = json!("red"),
            |v| v["root"]["text"] = json!(5),
            |v| v["root"]["draw"] = json!("spiral"),
            |v| v["root"]["grid"]["cells"][0]["kind"] = json!("date"),
            |v| v["root"]["grid"]["cells"][0]["bold"] = json!(1),
            |v| v["root"]["grid"]["cells"][0]["size"] = json!(2_147_483_648i64),
            |v| v["root"]["grid"]["cells"][0]["edited"] = json!(1.5),
            |v| v["root"]["grid"]["cells"][1]["image"]["file"] = json!("../../etc/passwd"),
            |v| v["root"]["grid"]["cells"][1]["image"]["scale"] = json!(0),
            |v| v["root"]["grid"]["cells"][0]["links"][1]["doc"] = json!(null),
        ];
        for (i, case) in cases.into_iter().enumerate() {
            assert!(matches!(broken(case), Err(Error::Invalid(_))), "case {i}");
        }
    }

    #[test]
    fn defaults_written_out_are_accepted_and_dropped_on_save() {
        let doc = broken(|v| {
            let cell = &mut v["root"]["grid"]["cells"][3];
            cell["text"] = json!("");
            cell["kind"] = json!("data");
            cell["color"] = json!("#FFFFFF");
            cell["bold"] = json!(false);
            cell["runs"] = json!([]);
            cell["size"] = json!(0);
            v["root"]["grid"]["cells"][2]["grid"]["vertical"] = json!(true);
            v["root"]["grid"]["cells"][2]["grid"]["widths"] = json!([80]);
            v["root"]["grid"]["cells"][2]["image"]["scale"] = json!(1);
        })
        .unwrap();
        assert_eq!(doc, full_document());
    }

    fn nested(levels: u32) -> String {
        let mut text = format!(
            r#"{{"format":"wombat","version":1,"id":"{}","root":"#,
            id(1)
        );
        for n in 0..levels {
            text.push_str(&format!(
                r#"{{"id":"{}","grid":{{"xs":1,"ys":1,"cells":["#,
                id(100 + n as u128)
            ));
        }
        text.push_str(&format!(r#"{{"id":"{}"}}"#, id(99)));
        text.push_str(&"]}}".repeat(levels as usize));
        text.push('}');
        text
    }

    #[test]
    fn nesting_depth_is_bounded() {
        let deepest = from_slice(nested(MAX_DEPTH).as_bytes()).unwrap();
        assert_eq!(deepest.cell_count(), MAX_DEPTH as usize + 1);
        let leaf = CellId(Id::from_u128(99));
        assert_eq!(deepest.depth(leaf), Some(MAX_DEPTH));
        assert_eq!(
            from_slice(to_string(&deepest).unwrap().as_bytes()).unwrap(),
            deepest
        );
        assert!(matches!(
            from_slice(nested(MAX_DEPTH + 1).as_bytes()),
            Err(Error::Limit(_))
        ));
        // Far deeper than the JSON parser accepts: an error, not a stack overflow.
        assert!(matches!(
            from_slice(nested(100_000).as_bytes()),
            Err(Error::Syntax(_))
        ));
        assert!(from_slice("[".repeat(1_000_000).as_bytes()).is_err());
        // Unknown fields may be nested too, up to the parser's limit.
        let mut deep_extra = String::from(r#"{"format":"wombat","version":1,"zz":"#);
        deep_extra.push_str(&"[".repeat(1_000));
        assert!(from_slice(deep_extra.as_bytes()).is_err());
    }

    #[test]
    fn saving_is_deterministic_and_refuses_invalid_documents() {
        let doc = full_document();
        assert_eq!(to_string(&doc).unwrap(), to_string(&doc.clone()).unwrap());
        // Built in another order, with the same IDs: the same bytes.
        let reloaded = from_slice(to_string(&doc).unwrap().as_bytes()).unwrap();
        assert_eq!(to_string(&reloaded).unwrap(), to_string(&doc).unwrap());
        let mut bad = doc;
        let root = bad.root();
        bad.cell_mut(root).unwrap().runs.push(Run {
            start: 500,
            len: 1,
            style: Style::default(),
            color: None,
            extra: Extra::new(),
        });
        assert!(matches!(to_string(&bad), Err(Error::Invalid(_))));
    }

    #[test]
    fn seeded_ids_give_real_uuids_in_files() {
        let doc = Document::new(&mut SeededIds::from_u64(42));
        let text = to_string(&doc).unwrap();
        assert!(text.contains(&format!("\"id\": \"{}\"", doc.id())));
        assert_eq!(doc.id().to_string().len(), 36);
    }
}
