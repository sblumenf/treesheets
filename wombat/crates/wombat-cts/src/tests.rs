//! Unit tests on hand-built `.cts` files. [`Cts`] writes a file of any format version the way
//! TreeSheets' `Save` functions would have (as far as their version-conditional loaders say),
//! so every version's layout can be tried, also the ones no example file uses.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use wombat_core::limits::MAX_DEPTH;
use wombat_core::{
    CellId, CellKind, Color, Document, DrawStyle, LinkTarget, SeededIds, Style, TextAlign,
    VertAlign,
};

use crate::{import, import_bounded, CtsError, ImageKind, Imported, UNRESOLVED_TARGET_FIELD};

/// A run as the file has it: start, length, style bits, colour.
type RawRun = (i32, i32, u32, u32);
/// A link as the file has it: start, length, internal, target.
type RawLink = (i32, i32, bool, &'static str);

#[derive(Clone, Debug, Default)]
struct C {
    kind: u8,
    bg: u32,
    fg: u32,
    draw: u8,
    note: &'static str,
    align: u8,
    valign: u8,
    selected: bool,
    /// `None`: the cell has no text record at all.
    text: Option<String>,
    rel_size: i32,
    image: i32,
    style: u32,
    last_edit: i64,
    runs: Vec<RawRun>,
    links: Vec<RawLink>,
    grid: Option<G>,
}

#[derive(Clone, Debug)]
struct G {
    xs: u32,
    border: u32,
    spacing: i32,
    vertical: bool,
    folded: bool,
    widths: Vec<i32>,
    cells: Vec<C>,
}

/// A cell with text.
fn t(text: &str) -> C {
    C {
        bg: 0xff_ffff,
        image: -1,
        text: Some(text.to_string()),
        ..C::default()
    }
}

/// A cell with neither text nor grid.
fn empty() -> C {
    C {
        text: None,
        ..t("")
    }
}

impl C {
    /// Gives the cell a grid of `xs` columns.
    fn grid(mut self, xs: u32, cells: Vec<C>) -> C {
        assert_eq!(cells.len() % xs as usize, 0);
        self.grid = Some(G {
            xs,
            border: 0xa0_a0a0,
            spacing: 3,
            vertical: true,
            folded: false,
            widths: vec![80; xs as usize],
            cells,
        });
        self
    }

    fn g(&mut self) -> &mut G {
        self.grid.as_mut().unwrap()
    }

    fn link(mut self, target: &'static str) -> C {
        let len = self.text.as_ref().unwrap().chars().count() as i32;
        self.links = vec![(0, len, true, target)];
        self
    }
}

/// A whole file.
struct Cts {
    version: u8,
    encryption: u8,
    selection: (u8, u8),
    zoom: u8,
    autoexport: u8,
    /// Type letter, scale, data.
    images: Vec<(u8, f64, Vec<u8>)>,
    root: C,
    tags: Vec<(&'static str, u32, u32)>,
    /// Whether the empty name that ends the tags is written.
    tags_end: bool,
}

fn cts(version: u8, root: C) -> Cts {
    Cts {
        version,
        encryption: 0,
        selection: (1, 1),
        zoom: 0,
        autoexport: 0,
        images: Vec::new(),
        root,
        tags: Vec::new(),
        tags_end: true,
    }
}

fn put_str(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
}

/// A number as `wxDataOutputStream::WriteDouble` writes it (80-bit extended).
fn put_extended(out: &mut Vec<u8>, value: f64) {
    if value == 0.0 {
        out.extend_from_slice(&[0; 10]);
        return;
    }
    if !value.is_finite() {
        out.extend_from_slice(&[0x7f, 0xff, 0x80, 0, 0, 0, 0, 0, 0, 0]);
        return;
    }
    let bits = value.abs().to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023 + 16383;
    let mantissa = (1u64 << 63) | ((bits & ((1 << 52) - 1)) << 11);
    let sign = if value < 0.0 { 0x8000 } else { 0 };
    out.extend_from_slice(&(exponent as u16 | sign).to_be_bytes());
    out.extend_from_slice(&mantissa.to_be_bytes());
}

impl Cts {
    /// The header up to and including the `D`.
    fn header(&self) -> Vec<u8> {
        let v = self.version;
        let mut out = b"TSFF".to_vec();
        out.push(v);
        if v >= 30 {
            out.push(self.encryption);
        }
        if v >= 21 {
            out.extend_from_slice(&[self.selection.0, self.selection.1]);
        }
        if v >= 23 {
            out.push(self.zoom);
        }
        if v >= 30 {
            out.push(self.autoexport);
        }
        for (kind, scale, data) in &self.images {
            out.push(*kind);
            if v < 9 {
                put_str(&mut out, "old name");
            }
            if v >= 19 {
                put_extended(&mut out, *scale);
            }
            if v >= 22 {
                out.extend_from_slice(&(data.len() as i64).to_le_bytes());
            }
            out.extend_from_slice(data);
        }
        out.push(b'D');
        out
    }

    /// The uncompressed cells and tags.
    fn stream(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.put_cell(&mut out, &self.root);
        if self.version >= 11 {
            for (name, bg, fg) in &self.tags {
                put_str(&mut out, name);
                if self.version >= 24 {
                    out.extend_from_slice(&bg.to_le_bytes());
                    out.extend_from_slice(&fg.to_le_bytes());
                }
            }
            if self.tags_end {
                put_str(&mut out, "");
            }
        }
        out
    }

    fn put_cell(&self, out: &mut Vec<u8>, c: &C) {
        let v = self.version;
        out.push(c.kind);
        if v >= 8 {
            out.extend_from_slice(&c.bg.to_le_bytes());
            out.extend_from_slice(&c.fg.to_le_bytes());
        }
        if v >= 15 {
            out.push(c.draw);
        }
        if v >= 25 {
            put_str(out, c.note);
        }
        if v >= 28 {
            out.push(c.align);
        }
        if v >= 29 {
            out.push(c.valign);
        }
        let contents = match (&c.text, &c.grid) {
            (Some(_), None) => 0,
            (None, Some(_)) => 1,
            (Some(_), Some(_)) => 2,
            (None, None) => 3,
        };
        out.push(contents | if c.selected { 0x80 } else { 0 });
        if let Some(text) = &c.text {
            put_str(out, text);
            if v <= 11 {
                out.extend_from_slice(&7u32.to_le_bytes());
            }
            out.extend_from_slice(&c.rel_size.to_le_bytes());
            out.extend_from_slice(&c.image.to_le_bytes());
            if v >= 7 {
                out.extend_from_slice(&c.style.to_le_bytes());
            }
            if v >= 14 {
                out.extend_from_slice(&c.last_edit.to_le_bytes());
            }
            if v >= 27 {
                out.extend_from_slice(&(c.runs.len() as u32).to_le_bytes());
                for (start, len, bits, col) in &c.runs {
                    out.extend_from_slice(&start.to_le_bytes());
                    out.extend_from_slice(&len.to_le_bytes());
                    out.extend_from_slice(&bits.to_le_bytes());
                    out.extend_from_slice(&col.to_le_bytes());
                }
            }
            if v >= 31 {
                out.extend_from_slice(&(c.links.len() as u32).to_le_bytes());
                for (start, len, internal, target) in &c.links {
                    out.extend_from_slice(&start.to_le_bytes());
                    out.extend_from_slice(&len.to_le_bytes());
                    out.push(u8::from(*internal));
                    put_str(out, target);
                }
            }
        }
        if let Some(g) = &c.grid {
            out.extend_from_slice(&g.xs.to_le_bytes());
            out.extend_from_slice(&(g.cells.len() as u32 / g.xs).to_le_bytes());
            if v >= 10 {
                out.extend_from_slice(&g.border.to_le_bytes());
                out.extend_from_slice(&g.spacing.to_le_bytes());
            }
            if v >= 11 {
                out.push(u8::from(g.vertical));
            }
            if v >= 16 {
                out.push(u8::from(g.folded));
            }
            if v >= 13 {
                for width in &g.widths {
                    out.extend_from_slice(&width.to_le_bytes());
                }
            }
            for cell in &g.cells {
                self.put_cell(out, cell);
            }
        }
    }

    fn bytes(&self) -> Vec<u8> {
        with_stream(&self.header(), &self.stream())
    }

    fn import(&self) -> Result<Imported, CtsError> {
        import(&self.bytes(), &mut SeededIds::from_u64(1))
    }
}

fn with_stream(header: &[u8], stream: &[u8]) -> Vec<u8> {
    let mut out = header.to_vec();
    out.extend_from_slice(&miniz_oxide::deflate::compress_to_vec_zlib(stream, 6));
    out
}

/// The cell at a path of (column, row) steps from the root.
fn at(doc: &Document, path: &[(u32, u32)]) -> CellId {
    path.iter()
        .fold(doc.root(), |cell, (x, y)| doc.child(cell, *x, *y).unwrap())
}

fn text_at<'a>(doc: &'a Document, path: &[(u32, u32)]) -> &'a str {
    &doc.cell(at(doc, path)).unwrap().text
}

/// A PNG file as far as its chunk structure goes (which is all the importer looks at).
fn png(extra_chunk: &[u8]) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    for (name, data) in [
        (b"IHDR", &[0u8; 13][..]),
        (b"IDAT", extra_chunk),
        (b"IEND", &[]),
    ] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        out.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------------------------

#[test]
fn password_protected_files_are_refused_with_a_clear_message() {
    for version in [30, 31] {
        let mut file = cts(version, t("").grid(1, vec![t("a")]));
        file.encryption = 1;
        let error = file.import().unwrap_err();
        assert!(matches!(error, CtsError::Encrypted));
        assert_eq!(
            error.to_string(),
            "password-protected files are not supported"
        );
    }
    // A real encrypted file has a different body; the header alone is enough to refuse it.
    let error = import(b"TSFF\x1f\x01", &mut SeededIds::from_u64(1)).unwrap_err();
    assert!(matches!(error, CtsError::Encrypted));
    // An unknown method is damage, as in TreeSheets.
    let mut file = cts(31, t("").grid(1, vec![t("a")]));
    file.encryption = 2;
    assert!(matches!(file.import(), Err(CtsError::Corrupt(_))));
    // Before version 30 there is no such byte: nothing is mistaken for it.
    let mut file = cts(29, t("").grid(1, vec![t("a")]));
    file.encryption = 1;
    file.import().unwrap();
}

#[test]
fn files_newer_than_version_31_are_refused() {
    for version in [32u8, 33, 100, 255] {
        let mut bytes = cts(31, t("").grid(1, vec![t("a")])).bytes();
        bytes[4] = version;
        let error = import(&bytes, &mut SeededIds::from_u64(1)).unwrap_err();
        assert!(matches!(error, CtsError::NewerVersion(v) if v == version));
        assert!(error.to_string().contains(&format!("version {version}")));
    }
}

#[test]
fn other_data_is_not_a_treesheets_file() {
    for bytes in [
        &b""[..],
        b"TSF",
        b"TSFG\x1f",
        b"{\"format\": \"wombat\"}",
        b"\0\0\0\0\0",
    ] {
        let error = import(bytes, &mut SeededIds::from_u64(1)).unwrap_err();
        assert!(matches!(error, CtsError::NotCts), "{bytes:?}");
    }
    // The signature alone, an unknown block, and a compressed part that is not zlib.
    for bytes in [
        &b"TSFF"[..],
        b"TSFF\x06X",
        b"TSFF\x06Dnot zlib",
        b"TSFF\x06D",
    ] {
        let error = import(bytes, &mut SeededIds::from_u64(1)).unwrap_err();
        assert!(matches!(error, CtsError::Corrupt(_)), "{bytes:?}");
    }
}

#[test]
fn the_root_must_have_a_grid_as_in_treesheets() {
    assert!(matches!(
        cts(31, t("alone")).import(),
        Err(CtsError::Corrupt(_))
    ));
    assert!(matches!(
        cts(31, empty()).import(),
        Err(CtsError::Corrupt(_))
    ));
}

// ---------------------------------------------------------------------------------------------
// Versions
// ---------------------------------------------------------------------------------------------

/// A document that sets every property away from its default.
fn everything() -> Cts {
    let mut inner = t("in\tner").grid(2, vec![t("x"), empty()]);
    inner.g().folded = true;
    inner.g().widths = vec![40, 3];
    inner.g().border = 0x33_2211;
    inner.g().spacing = 7;
    inner.g().vertical = false;
    let styled = C {
        kind: 1,
        bg: 0x10_2030,
        fg: 0x0a_0b0c,
        draw: 2,
        note: "a\nnote",
        align: 2,
        valign: 3,
        selected: true,
        rel_size: -2,
        image: 0,
        style: 1 | 16,
        last_edit: 1_700_000_000_123,
        runs: vec![(0, 2, 2, 0x100_00ff)],
        links: vec![(2, 3, false, "https://example.org"), (5, 1, true, "x")],
        ..t("styled")
    };
    let mut root = t("root").grid(2, vec![styled, inner]);
    root.g().widths = vec![100, 120];
    let mut file = cts(31, root);
    file.selection = (2, 1);
    file.autoexport = 2 | 4;
    file.images = vec![(b'I', 3.0, png(b"data"))];
    file.tags = vec![("x", 0x00_ff00, 0x11_2233)];
    file
}

#[test]
fn every_version_reads_exactly_the_fields_it_has() {
    for version in 1..=31u8 {
        let mut file = everything();
        file.version = version;
        let imported = file
            .import()
            .unwrap_or_else(|e| panic!("version {version}: {e}"));
        let since = |v: u8| version >= v;
        let doc = &imported.document;
        doc.validate().unwrap();
        assert_eq!(imported.version, version);
        assert_eq!(doc.cell_count(), 5);

        let root = doc.cell(doc.root()).unwrap();
        assert_eq!(root.text, "root");
        let grid = root.grid().unwrap();
        assert_eq!((grid.xs(), grid.ys()), (2, 1));
        let widths = if since(13) {
            vec![100, 120]
        } else {
            vec![80, 80]
        };
        assert_eq!(grid.col_widths, widths, "version {version}");

        let styled = doc.cell(at(doc, &[(0, 0)])).unwrap();
        assert_eq!(styled.text, "styled");
        assert_eq!(styled.kind, CellKind::Code);
        let (bg, fg) = if since(8) {
            (Color::rgb(0x30, 0x20, 0x10), Color::rgb(0x0c, 0x0b, 0x0a))
        } else {
            (Color::WHITE, Color::BLACK)
        };
        assert_eq!(
            (styled.cell_color, styled.text_color),
            (bg, fg),
            "version {version}"
        );
        let draw = if since(15) {
            DrawStyle::Lines
        } else {
            DrawStyle::Grid
        };
        assert_eq!(styled.draw_style, draw);
        assert_eq!(styled.note, if since(25) { "a\nnote" } else { "" });
        let align = if since(28) {
            TextAlign::Center
        } else {
            TextAlign::Auto
        };
        assert_eq!(styled.text_align, align);
        let valign = if since(29) {
            VertAlign::Bottom
        } else {
            VertAlign::Auto
        };
        assert_eq!(styled.vert_align, valign);
        assert_eq!(styled.rel_size, -2);
        let bold_strike = Style {
            bold: true,
            strike: true,
            ..Style::default()
        };
        assert_eq!(
            styled.style,
            if since(7) {
                bold_strike
            } else {
                Style::default()
            }
        );
        assert_eq!(styled.last_edit, since(14).then_some(1_700_000_000_123));
        if since(27) {
            assert_eq!(styled.runs.len(), 1);
            let run = &styled.runs[0];
            assert_eq!((run.start, run.len), (0, 2));
            assert!(run.style.italic && !run.style.bold);
            assert_eq!(run.color, Some(Color::rgb(0xff, 0, 0)));
        } else {
            assert!(styled.runs.is_empty());
        }
        if since(31) {
            assert_eq!(styled.links.len(), 2);
            assert_eq!((styled.links[0].start, styled.links[0].len), (2, 3));
            assert_eq!(
                styled.links[0].target,
                LinkTarget::External("https://example.org".into())
            );
            let target = LinkTarget::Cell {
                doc: None,
                cell: at(doc, &[(1, 0), (0, 0)]),
            };
            assert_eq!(styled.links[1].target, target);
        } else {
            assert!(styled.links.is_empty());
        }
        let image = styled.image.as_ref().unwrap();
        assert_eq!(image.file, imported.images[0].file);
        assert_eq!(image.scale, if since(19) { 3.0 } else { 1.0 });
        assert_eq!(imported.images[0].data, png(b"data"));
        assert!(imported.images[0].used);

        // Control characters become spaces in text, but stay in notes.
        let inner = doc.cell(at(doc, &[(1, 0)])).unwrap();
        assert_eq!(inner.text, "in ner");
        let grid = inner.grid().unwrap();
        assert_eq!((grid.xs(), grid.ys()), (2, 1));
        let border = if since(10) {
            Color::rgb(0x11, 0x22, 0x33)
        } else {
            Color::BORDER
        };
        assert_eq!(grid.border_color, border);
        assert_eq!(grid.outer_spacing, if since(10) { 7 } else { 3 });
        assert_eq!(grid.vertical, !since(11), "version {version}");
        // A column narrower than TreeSheets' minimum of 5 is widened to it.
        let widths = if since(13) { vec![40, 5] } else { vec![80, 80] };
        assert_eq!(grid.col_widths, widths);
        assert_eq!(imported.view.folded.contains(&inner.id()), since(16));
        assert_eq!(text_at(doc, &[(1, 0), (0, 0)]), "x");
        let bare = doc.cell(at(doc, &[(1, 0), (1, 0)])).unwrap();
        assert_eq!((bare.text.as_str(), bare.last_edit), ("", None));

        let selection = imported.view.selection.as_ref().unwrap();
        assert_eq!(selection.cell, styled.id());
        let size = if since(21) { (2, 1) } else { (1, 1) };
        assert_eq!((selection.xs, selection.ys), size);
        assert_eq!(imported.view.zoom, None);
        let autoexport = if since(30) { (2, true) } else { (0, false) };
        assert_eq!(
            (imported.autoexport_html, imported.autoexport_pdf),
            autoexport
        );

        if since(11) {
            assert_eq!(doc.tags().len(), 1);
            let tag = &doc.tags()["x"];
            let colors = if since(24) {
                (Color::rgb(0, 0xff, 0), Color::rgb(0x33, 0x22, 0x11))
            } else {
                (Color::WHITE, Color::rgb(0, 0, 0xff))
            };
            assert_eq!((tag.cell_color, tag.text_color), colors);
        } else {
            assert!(doc.tags().is_empty());
        }
    }
}

#[test]
fn a_file_one_version_older_than_a_field_does_not_read_it_as_something_else() {
    // The same bytes read as the next version are damaged or different, never equal: this
    // catches a version check that is off by one in a way the test above could not, because
    // there the writer and the reader could agree on the same mistake.
    for version in [7u8, 8, 10, 11, 13, 14, 15, 16, 25, 27, 28, 29, 31] {
        let mut file = everything();
        file.version = version - 1;
        let older = file.import().unwrap();
        let mut bytes = file.bytes();
        bytes[4] = version;
        let as_newer = import(&bytes, &mut SeededIds::from_u64(1));
        assert!(
            as_newer.map_or(true, |newer| newer != older),
            "version {version} reads like {}",
            version - 1
        );
    }
}

#[test]
fn tags_may_end_with_the_file_instead_of_an_empty_name() {
    // TS/examples/contrib/help/"...3 posted-ko.cts" (version 24) ends like this.
    let mut file = cts(24, t("").grid(1, vec![t("a")]));
    file.tags = vec![("a", 0xff_0000, 0), ("b\nc", 1, 2)];
    file.tags_end = false;
    let doc = file.import().unwrap().document;
    assert_eq!(doc.tags().keys().collect::<Vec<_>>(), ["a", "b c"]);
    assert_eq!(doc.tags()["a"].cell_color, Color::rgb(0, 0, 0xff));
    // A repeated name keeps the last colours, as TreeSheets' map does.
    let mut file = cts(31, t("").grid(1, vec![t("a")]));
    file.tags = vec![("a", 1, 1), ("a", 2, 2)];
    assert_eq!(
        file.import().unwrap().document.tags()["a"].text_color,
        Color::rgb(2, 0, 0)
    );
}

#[test]
fn cells_without_text_take_their_parents_size() {
    // TreeSheets creates each cell with its parent's relative size (the Cell constructor in
    // cell.h), and only a text record in the file replaces it.
    let parent = C {
        rel_size: 3,
        ..t("a")
    };
    let inner = empty().grid(1, vec![empty()]);
    let root = t("").grid(1, vec![parent.grid(1, vec![empty(), t("own"), inner])]);
    for version in [6u8, 31] {
        let doc = cts(version, root.clone()).import().unwrap().document;
        let size = |path: &[(u32, u32)]| doc.cell(at(&doc, path)).unwrap().rel_size;
        assert_eq!(size(&[]), 0);
        assert_eq!(size(&[(0, 0)]), 3);
        assert_eq!(size(&[(0, 0), (0, 0)]), 3);
        assert_eq!(size(&[(0, 0), (0, 1)]), 0);
        assert_eq!(size(&[(0, 0), (0, 2)]), 3);
        assert_eq!(size(&[(0, 0), (0, 2), (0, 0)]), 3);
    }
}

#[test]
fn a_formula_is_kept_as_text_with_its_equals_sign() {
    let formula = |text: &str| C {
        kind: 6,
        runs: vec![(0, 1, 1, 0)],
        links: vec![(1, 2, false, "x:y")],
        ..t(text)
    };
    let root = t("").grid(1, vec![formula("=SUM(a)"), formula("SUM(a)")]);
    let doc = cts(31, root).import().unwrap().document;
    for (y, start) in [(0, 0), (1, 1)] {
        let cell = doc.cell(at(&doc, &[(0, y)])).unwrap();
        assert_eq!(cell.kind, CellKind::Formula);
        assert_eq!(cell.text, "=SUM(a)");
        // Where the sign was added, runs and links move along with the text.
        assert_eq!(
            (cell.runs[0].start, cell.links[0].start),
            (start, start + 1)
        );
    }
}

#[test]
fn unknown_cell_types_draw_styles_and_contents_are_damage() {
    let bad = [C { kind: 7, ..t("a") }, C { draw: 3, ..t("a") }];
    for cell in bad {
        let file = cts(31, t("").grid(1, vec![cell]));
        assert!(matches!(file.import(), Err(CtsError::Corrupt(_))));
    }
    // Contents 4 (with and without the selection bit).
    for contents in [4u8, 0x84, 0x7f] {
        let file = cts(6, t("").grid(1, vec![empty()]));
        let mut stream = file.stream();
        let last = stream.len() - 1;
        stream[last] = contents;
        let bytes = with_stream(&file.header(), &stream);
        let result = import(&bytes, &mut SeededIds::from_u64(1));
        assert!(matches!(result, Err(CtsError::Corrupt(_))));
    }
    // Alignments out of range are automatic, as in TreeSheets.
    let cell = C {
        align: 9,
        valign: 200,
        ..t("a")
    };
    let doc = cts(31, t("").grid(1, vec![cell]))
        .import()
        .unwrap()
        .document;
    let cell = doc.cell(at(&doc, &[(0, 0)])).unwrap();
    assert_eq!(
        (cell.text_align, cell.vert_align),
        (TextAlign::Auto, VertAlign::Auto)
    );
}

// ---------------------------------------------------------------------------------------------
// Runs and links in the text
// ---------------------------------------------------------------------------------------------

#[test]
fn runs_are_repaired_the_way_treesheets_repairs_them() {
    // 6 code points, one of them outside the BMP and one of two bytes.
    let text = "a\u{1F600}b\u{e9}cd";
    type Ranges = Vec<(u32, u32)>;
    let cases: Vec<(Vec<RawRun>, Ranges)> = vec![
        (vec![(0, 2, 1, 0), (2, 4, 2, 0)], vec![(0, 2), (2, 4)]),
        // Overlap: the second starts where the first ends.
        (vec![(0, 3, 1, 0), (1, 4, 2, 0)], vec![(0, 3), (3, 3)]),
        // Too long, negative start, negative and zero length, wholly outside.
        (vec![(4, 100, 1, 0)], vec![(4, 2)]),
        (vec![(-5, 2, 1, 0)], vec![(0, 2)]),
        (
            vec![(1, -1, 1, 0), (2, 0, 1, 0), (6, 1, 1, 0), (9, 9, 1, 0)],
            vec![],
        ),
        // Out of order: the later one is pushed behind the earlier one.
        (vec![(3, 2, 1, 0), (0, 2, 2, 0)], vec![(3, 2), (5, 1)]),
        // Same style as the cell and no colour: not a run. Neighbours alike: one run.
        (
            vec![
                (0, 2, 0, 0),
                (2, 1, 1, 0),
                (3, 1, 1, 0),
                (4, 1, 1, 0x100_0000),
            ],
            vec![(2, 2), (4, 1)],
        ),
        (
            vec![(i32::MAX, i32::MAX, 1, 0), (i32::MIN, i32::MIN, 1, 0)],
            vec![],
        ),
    ];
    for (runs, expected) in cases {
        let cell = C {
            runs: runs.clone(),
            ..t(text)
        };
        let doc = cts(31, t("").grid(1, vec![cell]))
            .import()
            .unwrap()
            .document;
        let cell = doc.cell(at(&doc, &[(0, 0)])).unwrap();
        let got: Vec<(u32, u32)> = cell.runs.iter().map(|r| (r.start, r.len)).collect();
        assert_eq!(got, expected, "{runs:?}");
    }
    // A colour of its own (bit 24) is kept even if the style is the cell's; without the bit
    // the colour bytes mean nothing.
    let cell = C {
        style: 2,
        runs: vec![(0, 1, 2, 0x1ab_cdef), (1, 1, 3, 0x0ab_cdef)],
        ..t(text)
    };
    let doc = cts(27, t("").grid(1, vec![cell]))
        .import()
        .unwrap()
        .document;
    let runs = &doc.cell(at(&doc, &[(0, 0)])).unwrap().runs;
    assert_eq!(runs[0].color, Some(Color::rgb(0xef, 0xcd, 0xab)));
    assert!(runs[0].style.italic && !runs[0].style.bold);
    assert_eq!(runs[1].color, None);
    assert!(runs[1].style.italic && runs[1].style.bold);
}

#[test]
fn links_are_repaired_likewise_and_empty_targets_dropped() {
    let cell = C {
        links: vec![
            (0, 2, false, "https://a.example"),
            (1, 2, false, "file.txt"),
            (3, 1, false, ""),
            (3, 100, true, "nowhere"),
            (0, 1, false, "late"),
        ],
        ..t("abcdef")
    };
    let imported = cts(31, t("").grid(1, vec![cell])).import().unwrap();
    let doc = &imported.document;
    let links = &doc.cell(at(doc, &[(0, 0)])).unwrap().links;
    let ranges: Vec<(u32, u32)> = links.iter().map(|l| (l.start, l.len)).collect();
    // A link that starts too early is moved behind the one before, keeping its length.
    assert_eq!(ranges, [(0, 2), (2, 2), (4, 2)]);
    assert_eq!(links[1].target, LinkTarget::External("file.txt".into()));
    assert_eq!(links[2].target, LinkTarget::Unknown);
    assert_eq!(links[2].extra[UNRESOLVED_TARGET_FIELD], "nowhere");
}

#[test]
fn more_runs_or_links_than_characters_is_damage() {
    for (runs, links) in [(3usize, 0usize), (0, 3)] {
        let cell = C {
            runs: vec![(0, 1, 1, 0); runs],
            links: vec![(0, 1, false, "x:"); links],
            ..t("ab")
        };
        let file = cts(31, t("").grid(1, vec![cell]));
        assert!(matches!(file.import(), Err(CtsError::Corrupt(_))));
    }
}

// ---------------------------------------------------------------------------------------------
// Links between cells (decision B1)
// ---------------------------------------------------------------------------------------------

/// Imports a version 31 file and returns, for the cell at `from`, where its first link goes.
fn link_from(root: &C, from: &[(u32, u32)]) -> (Imported, Option<CellId>) {
    let imported = cts(31, root.clone()).import().unwrap();
    let cell = imported
        .document
        .cell(at(&imported.document, from))
        .unwrap();
    let target = match &cell.links[0].target {
        LinkTarget::Cell { doc: None, cell } => Some(*cell),
        LinkTarget::Unknown => None,
        other => panic!("{other:?}"),
    };
    (imported, target)
}

#[test]
fn a_link_goes_to_a_cell_with_the_text_in_the_same_grid() {
    let root = t("").grid(2, vec![t("go").link("T"), t("x"), t("T"), t("T")]);
    let (imported, target) = link_from(&root, &[(0, 0)]);
    // The first one row by row.
    assert_eq!(target, Some(at(&imported.document, &[(0, 1)])));
    assert!(imported.unresolved_links.is_empty());
}

#[test]
fn a_link_goes_into_nested_grids_breadth_first() {
    // "T" two levels down in the first column, one level down in the second: the shallower
    // one wins although the deeper one comes first in the file.
    let deep = t("p").grid(1, vec![t("q").grid(1, vec![t("T")])]);
    let shallow = t("r").grid(1, vec![t("T")]);
    let root = t("").grid(3, vec![deep, shallow, t("go").link("T")]);
    let (imported, target) = link_from(&root, &[(2, 0)]);
    assert_eq!(target, Some(at(&imported.document, &[(1, 0), (0, 0)])));
}

#[test]
fn a_link_goes_to_the_enclosing_grids_when_its_own_has_no_match() {
    // The linking cell is two levels down; "T" is beside its grandparent.
    let nest = t("a").grid(1, vec![t("b").grid(1, vec![t("go").link("T")])]);
    let root = t("").grid(2, vec![nest, t("T")]);
    let (imported, target) = link_from(&root, &[(0, 0), (0, 0), (0, 0)]);
    assert_eq!(target, Some(at(&imported.document, &[(1, 0)])));
    // The cell that holds the linking cell's grid is itself the first candidate.
    let nest = t("a").grid(1, vec![t("T").grid(2, vec![t("go").link("T"), t("x")])]);
    let root = t("").grid(2, vec![nest, t("T")]);
    let (imported, target) = link_from(&root, &[(0, 0), (0, 0), (0, 0)]);
    assert_eq!(target, Some(at(&imported.document, &[(0, 0), (0, 0)])));
}

#[test]
fn of_duplicates_the_nearest_grid_wins_then_the_shallowest_then_the_first() {
    // Candidates: far (beside the grandparent), deep (two below a sibling), near (one below
    // a sibling, later in the file than deep).
    let deep = t("d").grid(1, vec![t("e").grid(1, vec![t("T")])]);
    let near = t("n").grid(1, vec![t("T")]);
    let own = t("own").grid(3, vec![t("go").link("T"), deep, near]);
    let root = t("").grid(2, vec![t("a").grid(1, vec![own]), t("T")]);
    let from = [(0, 0), (0, 0), (0, 0)];
    let (imported, target) = link_from(&root, &from);
    let doc = &imported.document;
    assert_eq!(target, Some(at(doc, &[(0, 0), (0, 0), (2, 0), (0, 0)])));

    // Without the near one, the deep one in the same grid still beats the far one.
    let deep = t("d").grid(1, vec![t("e").grid(1, vec![t("T")])]);
    let own = t("own").grid(2, vec![t("go").link("T"), deep]);
    let root = t("").grid(2, vec![t("a").grid(1, vec![own]), t("T")]);
    let (imported, target) = link_from(&root, &from);
    let doc = &imported.document;
    assert_eq!(
        target,
        Some(at(doc, &[(0, 0), (0, 0), (1, 0), (0, 0), (0, 0)]))
    );
}

#[test]
fn a_link_never_goes_to_its_own_cell_or_to_the_root() {
    // The linking cell has the wanted text itself: the next one is taken.
    let root = t("T").grid(3, vec![t("T").link("T"), t("x"), t("T")]);
    let (imported, target) = link_from(&root, &[(0, 0)]);
    assert_eq!(target, Some(at(&imported.document, &[(2, 0)])));
    // Only the linking cell and the root have it: no target.
    let root = t("T").grid(2, vec![t("T").link("T"), t("x")]);
    let (imported, target) = link_from(&root, &[(0, 0)]);
    assert_eq!(target, None);
    assert_eq!(imported.unresolved_links.len(), 1);
    // A link in the root cell itself has no grid around it to search.
    let root = t("root").link("x").grid(1, vec![t("x")]);
    assert_eq!(link_from(&root, &[]).1, None);
}

#[test]
fn a_link_without_a_target_keeps_its_text_and_range() {
    let cell = C {
        links: vec![(1, 2, true, "no such cell")],
        ..t("abcd")
    };
    let imported = cts(31, t("").grid(2, vec![cell, t("other")]))
        .import()
        .unwrap();
    let doc = &imported.document;
    let id = at(doc, &[(0, 0)]);
    let link = &doc.cell(id).unwrap().links[0];
    assert_eq!((link.start, link.len), (1, 2));
    assert_eq!(link.target, LinkTarget::Unknown);
    assert_eq!(link.extra[UNRESOLVED_TARGET_FIELD], "no such cell");
    assert_eq!(imported.unresolved_links.len(), 1);
    assert_eq!(imported.unresolved_links[0].cell, id);
    assert_eq!(imported.unresolved_links[0].target, "no such cell");
    // It survives the .wombat format.
    let text = wombat_core::format::to_string(doc).unwrap();
    assert!(
        text.contains("\"cts_target\": \"no such cell\"")
            || text.contains("\"cts_target\":\"no such cell\"")
    );
    assert_eq!(
        &wombat_core::format::from_slice(text.as_bytes()).unwrap(),
        doc
    );
}

#[test]
fn text_is_matched_exactly_and_formula_cells_are_never_targets() {
    let formula = C { kind: 6, ..t("=T") };
    let cells = vec![
        t("go").link("=T"),
        t("t"),
        t("T "),
        formula,
        t("go2").link("T"),
    ];
    let root = t("").grid(5, cells);
    let (imported, target) = link_from(&root, &[(0, 0)]);
    // In TreeSheets a formula cell's text is its result, which the importer does not have.
    assert_eq!(target, None);
    assert_eq!(link_from(&root, &[(4, 0)]).1, None);
    assert_eq!(imported.unresolved_links.len(), 2);
}

#[test]
fn internal_and_external_links_of_one_cell_stay_in_order() {
    let cell = C {
        links: vec![
            (0, 1, true, "T"),
            (1, 1, false, "x:1"),
            (2, 1, true, "U"),
            (3, 1, false, "x:2"),
        ],
        ..t("abcd")
    };
    let doc = cts(31, t("").grid(2, vec![cell, t("T")]))
        .import()
        .unwrap()
        .document;
    let links = &doc.cell(at(&doc, &[(0, 0)])).unwrap().links;
    let starts: Vec<u32> = links.iter().map(|l| l.start).collect();
    assert_eq!(starts, [0, 1, 2, 3]);
    assert!(matches!(links[0].target, LinkTarget::Cell { .. }));
    assert!(matches!(links[2].target, LinkTarget::Unknown));
}

#[test]
fn many_links_to_a_far_target_do_not_take_quadratic_time() {
    // 60,000 cells that all link to the text of the very last one, with as many duplicates
    // of another text in between. Searching the tree once per link would be 3.6 thousand
    // million steps.
    let count = 60_000;
    let mut cells = vec![t("T").link("U"); count - 1];
    cells.push(t("U"));
    let started = std::time::Instant::now();
    let imported = cts(31, t("").grid(1, cells)).import().unwrap();
    assert!(started.elapsed().as_secs() < 30, "{:?}", started.elapsed());
    let doc = &imported.document;
    let last = at(doc, &[(0, count as u32 - 1)]);
    let grid = doc.cell(doc.root()).unwrap().grid().unwrap();
    for id in grid.cells().iter().take(count - 1) {
        let target = &doc.cell(*id).unwrap().links[0].target;
        assert_eq!(
            target,
            &LinkTarget::Cell {
                doc: None,
                cell: last
            }
        );
    }
    assert!(imported.unresolved_links.is_empty());
}

/// TreeSheets' search written out as it is in `evaluator.h` (`ForNearest` with `all`), slow
/// and obvious, to check the indexed search against.
fn slow_link_target(doc: &Document, from: CellId, name: &str) -> Option<CellId> {
    let mut scope = doc.cell(from)?.parent();
    while let Some(top) = scope {
        let mut level = vec![top];
        while !level.is_empty() {
            let mut next = Vec::new();
            for id in level {
                let cell = doc.cell(id)?;
                if id != from
                    && cell.parent().is_some()
                    && cell.kind != CellKind::Formula
                    && cell.text == name
                {
                    return Some(id);
                }
                if let Some(grid) = cell.grid() {
                    next.extend_from_slice(grid.cells());
                }
            }
            level = next;
        }
        scope = doc.cell(top)?.parent();
    }
    None
}

/// A random tree whose cells have one of three texts and link to one of four.
fn arb_tree(depth: u32) -> BoxedStrategy<C> {
    let leaf = (0usize..3, 0usize..4, any::<bool>()).prop_map(|(text, target, formula)| C {
        kind: if formula { 6 } else { 0 },
        ..t(["A", "B", "C"][text]).link(["A", "B", "C", "D"][target])
    });
    if depth == 0 {
        return leaf.boxed();
    }
    (
        leaf,
        1u32..3,
        prop::collection::vec(arb_tree(depth - 1), 0..7),
    )
        .prop_map(|(cell, xs, mut cells)| {
            cells.truncate(cells.len() / xs as usize * xs as usize);
            if cells.is_empty() {
                cell
            } else {
                cell.grid(xs, cells)
            }
        })
        .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn the_indexed_link_search_agrees_with_treesheets_search(tree in arb_tree(4)) {
        let root = C { kind: 0, ..tree };
        prop_assume!(root.grid.is_some());
        let imported = cts(31, root).import().unwrap();
        let doc = &imported.document;
        for id in doc.cells_in_order() {
            let cell = doc.cell(id).unwrap();
            // A formula's text got an "=" in front, so its link is one further on; the text
            // the link asks for is whichever of A to D the cell was given.
            for link in &cell.links {
                let wanted = match &link.target {
                    LinkTarget::Cell { cell: to, .. } => doc.cell(*to).unwrap().text.clone(),
                    LinkTarget::Unknown => {
                        link.extra[UNRESOLVED_TARGET_FIELD].as_str().unwrap().to_string()
                    }
                    other => panic!("{other:?}"),
                };
                let expected = slow_link_target(doc, id, &wanted);
                let got = match &link.target {
                    LinkTarget::Cell { cell: to, .. } => Some(*to),
                    _ => None,
                };
                prop_assert_eq!(got, expected);
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// View state
// ---------------------------------------------------------------------------------------------

#[test]
fn the_selection_is_the_marked_cell_and_a_size_that_fits() {
    let mark = |text: &str| C {
        selected: true,
        ..t(text)
    };
    let root = t("").grid(3, vec![t("a"), mark("b"), t("c"), t("d"), t("e"), t("f")]);
    for (size, expected) in [
        ((1, 1), (1, 1)),
        ((2, 2), (2, 2)),
        ((9, 9), (2, 2)),
        ((0, 0), (1, 1)),
    ] {
        let mut file = cts(31, root.clone());
        file.selection = size;
        let imported = file.import().unwrap();
        let selection = imported.view.selection.unwrap();
        assert_eq!(selection.cell, at(&imported.document, &[(1, 0)]));
        assert_eq!((selection.xs, selection.ys), expected, "{size:?}");
        assert_eq!(selection.cursor, None);
    }
    // No mark, or the root marked: the first cell of the root's grid, alone.
    for root in [
        t("").grid(2, vec![t("a"), t("b")]),
        mark("").grid(2, vec![t("a"), t("b")]),
    ] {
        let mut file = cts(31, root);
        file.selection = (2, 1);
        let imported = file.import().unwrap();
        let selection = imported.view.selection.unwrap();
        assert_eq!(selection.cell, at(&imported.document, &[(0, 0)]));
        assert_eq!((selection.xs, selection.ys), (1, 1));
    }
    // Several marks: the last one, as TreeSheets' loader ends up with.
    let root = t("").grid(2, vec![mark("a"), mark("b")]);
    let imported = cts(31, root).import().unwrap();
    let selection = imported.view.selection.unwrap();
    assert_eq!(selection.cell, at(&imported.document, &[(1, 0)]));
}

#[test]
fn zoom_is_the_cell_at_the_saved_depth_on_the_way_to_the_selection() {
    let mark = |text: &str| C {
        selected: true,
        ..t(text)
    };
    // root > a > b > (c, d): c is marked and has no grid.
    let tree = |marked_has_grid: bool| {
        let c = if marked_has_grid {
            mark("c").grid(2, vec![t("x"), t("y")])
        } else {
            mark("c")
        };
        t("").grid(
            1,
            vec![t("a").grid(1, vec![t("b").grid(2, vec![c, t("d")])])],
        )
    };
    let a = [(0, 0)];
    let b = [(0, 0), (0, 0)];
    let c = [(0, 0), (0, 0), (0, 0)];
    for (level, expected) in [
        (0u8, None),
        (1, Some(&a[..])),
        (2, Some(&b[..])),
        (3, Some(&b[..])),
        (200, Some(&b[..])),
    ] {
        let mut file = cts(31, tree(false));
        file.zoom = level;
        let imported = file.import().unwrap();
        let doc = &imported.document;
        assert_eq!(
            imported.view.zoom,
            expected.map(|path| at(doc, path)),
            "level {level}"
        );
        assert_eq!(imported.view.selection.unwrap().cell, at(doc, &c));
    }
    // A selected cell with a grid can itself be zoomed into; the selection then moves to
    // all of its cells.
    let mut file = cts(31, tree(true));
    file.zoom = 3;
    let imported = file.import().unwrap();
    let doc = &imported.document;
    assert_eq!(imported.view.zoom, Some(at(doc, &c)));
    let selection = imported.view.selection.unwrap();
    assert_eq!(selection.cell, at(doc, &[(0, 0), (0, 0), (0, 0), (0, 0)]));
    assert_eq!((selection.xs, selection.ys), (2, 1));
    // ... but not if more than one cell is selected.
    let mut file = cts(31, tree(true));
    file.zoom = 3;
    file.selection = (2, 1);
    let imported = file.import().unwrap();
    assert_eq!(imported.view.zoom, Some(at(&imported.document, &b)));
    // Versions before 23 have no zoom level.
    let mut file = cts(22, tree(false));
    file.zoom = 2;
    assert_eq!(file.import().unwrap().view.zoom, None);
    // A selection in the root's own grid cannot be zoomed.
    let mut file = cts(31, t("").grid(1, vec![mark("a")]));
    file.zoom = 1;
    assert_eq!(file.import().unwrap().view.zoom, None);
}

// ---------------------------------------------------------------------------------------------
// Images
// ---------------------------------------------------------------------------------------------

fn with_image(index: i32) -> C {
    C {
        image: index,
        ..t("pic")
    }
}

#[test]
fn images_get_names_and_cells_refer_to_them() {
    let root = t("").grid(
        4,
        vec![with_image(1), with_image(1), with_image(-1), with_image(5)],
    );
    let mut file = cts(31, root);
    file.images = vec![
        (b'I', 1.0, png(b"one")),
        (b'J', 2.5, vec![0xff, 0xd8, 0xff, 0xd9]),
        (b'I', 1.0, vec![]),
    ];
    let imported = file.import().unwrap();
    let doc = &imported.document;
    let names: Vec<&str> = imported.images.iter().map(|i| i.file.as_str()).collect();
    assert!(names[0].ends_with(".png") && names[1].ends_with(".jpg"));
    assert!(names
        .iter()
        .all(|name| wombat_core::model::is_valid_image_name(name)));
    assert_eq!(imported.images[1].kind, ImageKind::Jpeg);
    assert_eq!(imported.images[1].data, [0xff, 0xd8, 0xff, 0xd9]);
    let used: Vec<bool> = imported.images.iter().map(|i| i.used).collect();
    assert_eq!(used, [false, true, false]);
    for x in 0..2 {
        let image = doc.cell(at(doc, &[(x, 0)])).unwrap().image.clone().unwrap();
        assert_eq!((image.file.as_str(), image.scale), (names[1], 2.5));
    }
    // No image, and an index that is not in the file (TreeSheets shows none either).
    assert!(doc.cell(at(doc, &[(2, 0)])).unwrap().image.is_none());
    assert!(doc.cell(at(doc, &[(3, 0)])).unwrap().image.is_none());
}

#[test]
fn the_display_scale_is_limited_as_treesheets_limits_it() {
    let cases = [
        (1.0, 1.0),
        (0.5, 0.5),
        (100.0, 100.0),
        (1000.0, 100.0),
        (0.01, 1.0 / 16.0),
        (0.0, 1.0 / 16.0),
        (-3.0, 1.0 / 16.0),
        (f64::INFINITY, 1.0),
    ];
    for (scale, expected) in cases {
        let mut file = cts(31, t("").grid(1, vec![with_image(0)]));
        file.images = vec![(b'I', scale, png(b""))];
        let imported = file.import().unwrap();
        assert_eq!(imported.images[0].scale, expected, "{scale}");
        imported.document.validate().unwrap();
    }
}

#[test]
fn old_images_have_no_length_and_end_where_their_format_says() {
    let root = t("").grid(2, vec![with_image(0), with_image(1)]);
    for version in [9u8, 17, 19, 21] {
        let mut file = cts(version, root.clone());
        file.images = vec![
            (b'I', 2.0, png(b"first")),
            (b'I', 2.0, png(b"the second one")),
        ];
        let imported = file.import().unwrap();
        assert_eq!(imported.images[0].data, png(b"first"), "version {version}");
        assert_eq!(imported.images[1].data, png(b"the second one"));
        assert_eq!(
            imported.images[1].scale,
            if version >= 19 { 2.0 } else { 1.0 }
        );
        assert!(imported.images.iter().all(|i| i.used));
    }
    // A JPEG: markers, a segment, a scan with a stuffed 0xff and a restart marker, the end.
    let jpeg = vec![
        0xff, 0xd8, 0xff, 0xe0, 0, 4, 1, 2, 0xff, 0xda, 0, 3, 9, 0x12, 0xff, 0x00, 0x34, 0xff,
        0xd0, 0x56, 0xff, 0xd9,
    ];
    let mut file = cts(19, root.clone());
    file.images = vec![(b'J', 1.0, jpeg.clone()), (b'I', 1.0, png(b"after"))];
    let imported = file.import().unwrap();
    assert_eq!(imported.images[0].data, jpeg);
    assert_eq!(imported.images[1].data, png(b"after"));
    // Damaged ones are refused, not read past.
    for bad in [
        b"\x89PNG\r\n\x1a\n".to_vec(),
        b"not a png at all".to_vec(),
        vec![0xff, 0xd8, 0xff],
        {
            let mut cut = png(b"x");
            cut.truncate(cut.len() - 5);
            cut
        },
    ] {
        for kind in *b"IJ" {
            let mut file = cts(19, root.clone());
            file.images = vec![(kind, 1.0, bad.clone())];
            assert!(matches!(file.import(), Err(CtsError::Corrupt(_))));
        }
    }
}

#[test]
fn an_image_longer_than_the_file_is_damage_not_an_allocation() {
    let file = cts(31, t("").grid(1, vec![t("a")]));
    for len in [1i64 << 40, i64::MAX, -1, i64::MIN, 5] {
        let mut bytes = b"TSFF\x1f\0\x01\x01\0\0I".to_vec();
        put_extended(&mut bytes, 1.0);
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes.extend_from_slice(b"abc");
        let _ = &file;
        let result = import(&bytes, &mut SeededIds::from_u64(1));
        assert!(matches!(result, Err(CtsError::Corrupt(_))), "{len}");
    }
}

#[test]
fn a_fold_of_versions_16_and_17_takes_the_cells_image_away() {
    // "Before v18, folding would use the image slot" (Grid::LoadContents).
    for (version, folded, kept) in [
        (15u8, true, true),
        (16, true, false),
        (17, true, false),
        (17, false, true),
        (18, true, true),
    ] {
        let mut cell = with_image(0).grid(1, vec![t("in")]);
        cell.g().folded = folded;
        let mut file = cts(version, t("").grid(1, vec![cell]));
        file.images = vec![(b'I', 1.0, png(b""))];
        let imported = file.import().unwrap();
        let doc = &imported.document;
        let has = doc.cell(at(doc, &[(0, 0)])).unwrap().image.is_some();
        assert_eq!(has, kept, "version {version}, folded {folded}");
        assert_eq!(imported.images[0].used, kept);
    }
}

#[test]
fn saving_writes_the_document_its_used_images_and_the_view() {
    let dir = std::env::temp_dir().join(format!("wombat-cts-unit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut folded = with_image(1).grid(1, vec![t("in")]);
    folded.g().folded = true;
    let mut file = cts(31, t("").grid(1, vec![folded]));
    file.images = vec![(b'I', 1.0, png(b"unused")), (b'I', 2.0, png(b"used"))];
    let imported = file.import().unwrap();
    let path = dir.join("out.wombat");
    imported.save(&path).unwrap();
    let doc = wombat_core::storage::load_document(&path).unwrap();
    assert_eq!(doc, imported.document);
    let images = wombat_core::storage::images_dir(&path);
    let files: Vec<_> = std::fs::read_dir(&images)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(files, [std::ffi::OsString::from(&imported.images[1].file)]);
    assert_eq!(
        std::fs::read(images.join(&imported.images[1].file)).unwrap(),
        png(b"used")
    );
    let view = wombat_core::storage::load_view_state(&dir, doc.id())
        .unwrap()
        .unwrap();
    assert_eq!(view, imported.view);
    assert_eq!(view.folded.len(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---------------------------------------------------------------------------------------------
// Limits (decision A12)
// ---------------------------------------------------------------------------------------------

/// A version 6 file whose stream is `stream`.
fn raw(stream: &[u8]) -> Vec<u8> {
    with_stream(b"TSFF\x06D", stream)
}

#[test]
fn a_grid_must_fit_in_the_data_that_is_left() {
    // The root (data, grid only) claiming xs by ys cells, followed by almost nothing.
    for (xs, ys) in [
        (2000u32, 2000u32),
        (4 * 1024 * 1024, 1),
        (u32::MAX, u32::MAX),
        (0x7fff_ffff, 2),
        (0, 5),
        (5, 0),
        (1, 0x8000_0000),
        (65536, 65536),
    ] {
        let mut stream = vec![0u8, 1];
        stream.extend_from_slice(&xs.to_le_bytes());
        stream.extend_from_slice(&ys.to_le_bytes());
        stream.extend_from_slice(&[0, 3, 0, 3]);
        let started = std::time::Instant::now();
        let result = import(&raw(&stream), &mut SeededIds::from_u64(1));
        assert!(
            matches!(result, Err(CtsError::Corrupt(_) | CtsError::Limit(_))),
            "{xs} x {ys}"
        );
        assert!(started.elapsed().as_secs() < 5);
    }
    // The same grid sizes are fine when the cells are there.
    let mut stream = vec![0u8, 1];
    stream.extend_from_slice(&300u32.to_le_bytes());
    stream.extend_from_slice(&200u32.to_le_bytes());
    stream.extend_from_slice(&[0, 3].repeat(60_000));
    let imported = import(&raw(&stream), &mut SeededIds::from_u64(1)).unwrap();
    assert_eq!(imported.document.cell_count(), 60_001);
}

#[test]
fn nesting_is_limited_to_what_wombat_holds() {
    let nested = |levels: u32| {
        let mut stream = Vec::new();
        for _ in 0..levels {
            stream.extend_from_slice(&[0, 1, 1, 0, 0, 0, 1, 0, 0, 0]);
        }
        stream.extend_from_slice(&[0, 3]);
        raw(&stream)
    };
    let imported = import(&nested(MAX_DEPTH), &mut SeededIds::from_u64(1)).unwrap();
    assert_eq!(imported.max_depth, MAX_DEPTH);
    imported.document.validate().unwrap();
    for levels in [MAX_DEPTH + 1, 500, 200_000] {
        let error = import(&nested(levels), &mut SeededIds::from_u64(1)).unwrap_err();
        assert!(matches!(error, CtsError::Limit(_)), "{levels}: {error}");
        assert!(error.to_string().contains("nested deeper than 32"));
    }
}

#[test]
fn the_decompressed_size_is_bounded() {
    // 4 MiB of zeros compress to a few kilobytes.
    let bytes = raw(&vec![0u8; 4 * 1024 * 1024]);
    assert!(bytes.len() < 10_000);
    let result = import_bounded(&bytes, &mut SeededIds::from_u64(1), 1024 * 1024);
    assert!(matches!(result, Err(CtsError::Limit(_))));
    // Within the bound it is read (and found to be damaged: cell type 0, contents 0, ...).
    let result = import_bounded(&bytes, &mut SeededIds::from_u64(1), 8 * 1024 * 1024);
    assert!(matches!(result, Err(CtsError::Corrupt(_))));
}

#[test]
fn a_truncated_or_altered_compressed_part_is_damage() {
    let bytes = cts(
        31,
        t("").grid(1, vec![t("some text to compress, some text to compress")]),
    )
    .bytes();
    for cut in 1..20 {
        let result = import(&bytes[..bytes.len() - cut], &mut SeededIds::from_u64(1));
        assert!(matches!(result, Err(CtsError::Corrupt(_))), "cut {cut}");
    }
    // The checksum at the end is checked.
    let mut altered = bytes.clone();
    let last = altered.len() - 1;
    altered[last] ^= 1;
    assert!(matches!(
        import(&altered, &mut SeededIds::from_u64(1)),
        Err(CtsError::Corrupt(_))
    ));
    // Bytes after the compressed part are ignored, as TreeSheets ignores them.
    let mut longer = bytes.clone();
    longer.extend_from_slice(b"trailing");
    import(&longer, &mut SeededIds::from_u64(1)).unwrap();
}

#[test]
fn text_that_is_not_utf8_reads_as_empty_and_is_reported() {
    let file = cts(31, t("").grid(1, vec![t("ab")]));
    let mut stream = file.stream();
    let at = stream.windows(2).position(|w| w == b"ab").unwrap();
    stream[at] = 0xff;
    let imported = import(
        &with_stream(&file.header(), &stream),
        &mut SeededIds::from_u64(1),
    )
    .unwrap();
    assert_eq!(text_at(&imported.document, &[(0, 0)]), "");
    assert_eq!(imported.undecodable_strings, 1);
    assert_eq!(file.import().unwrap().undecodable_strings, 0);
}

#[test]
fn import_is_repeatable_with_the_same_seed_and_new_with_another() {
    let file = everything();
    let a = file.import().unwrap();
    assert_eq!(a, file.import().unwrap());
    let b = import(&file.bytes(), &mut SeededIds::from_u64(2)).unwrap();
    assert_ne!(a.document.id(), b.document.id());
    assert_ne!(a.document.root(), b.document.root());
    assert_ne!(a.images[0].file, b.images[0].file);
}

#[test]
fn errors_say_what_is_wrong() {
    assert_eq!(CtsError::NotCts.to_string(), "not a TreeSheets file");
    assert!(CtsError::NewerVersion(40)
        .to_string()
        .contains("only versions up to 31"));
    assert!(CtsError::Corrupt("x".into())
        .to_string()
        .contains("damaged"));
}

// ---------------------------------------------------------------------------------------------
// The "quirk" files: hand-built files for what no example exercises, which the real
// TreeSheets then read (tests/quirks/README.md). `cargo test -p wombat-cts -- --ignored
// write_quirk_files` writes them again, byte for byte.
// ---------------------------------------------------------------------------------------------

/// The PNG in the referee's fixture, so that TreeSheets has a real image to show.
fn real_png() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../referee/fixtures/features.cts"
    );
    let file = std::fs::read(path).unwrap();
    // Version 31: signature, version, encryption, selection, zoom, export, then the image:
    // 'I', a 10-byte scale, an 8-byte length.
    assert_eq!(file[10], b'I');
    let len = u64::from_le_bytes(file[21..29].try_into().unwrap()) as usize;
    file[29..29 + len].to_vec()
}

fn quirk_files() -> Vec<(&'static str, Vec<u8>)> {
    let mark = |text: &str| C {
        selected: true,
        ..t(text)
    };
    let mut files = Vec::new();

    // Version 31: sizes inherited, runs and links repaired, odd values.
    let text = "a\u{1F600}b\u{e9}cd";
    let mut cells = vec![
        C {
            rel_size: 3,
            ..t("big parent")
        }
        .grid(
            1,
            vec![
                empty(),
                C {
                    rel_size: -1,
                    ..t("own size")
                },
                empty().grid(1, vec![empty()]),
            ],
        ),
        C {
            runs: vec![(0, 3, 1, 0), (1, 4, 2, 0)],
            ..t(text)
        },
        C {
            runs: vec![(4, 100, 1, 0)],
            ..t(text)
        },
        C {
            runs: vec![(-5, 2, 1, 0)],
            ..t(text)
        },
        C {
            runs: vec![(1, -1, 1, 0), (2, 0, 1, 0), (6, 1, 1, 0), (9, 9, 1, 0)],
            ..t(text)
        },
        C {
            runs: vec![(3, 2, 1, 0), (0, 2, 2, 0)],
            ..t(text)
        },
        C {
            style: 2,
            runs: vec![
                (0, 1, 2, 0x1ab_cdef),
                (1, 1, 3, 0x0ab_cdef),
                (2, 2, 2, 0),
                (4, 1, 0, 0x100_0000),
            ],
            ..t(text)
        },
        C {
            links: vec![
                (0, 2, false, "https://a.example"),
                (1, 2, false, "file.txt"),
                (3, 1, false, ""),
                (3, 100, true, "nowhere"),
                (0, 1, false, "late"),
            ],
            ..t("abcdef")
        },
        C {
            kind: 6,
            ..t("1+2")
        },
        t("ZZ not UTF-8"),
        C {
            align: 9,
            valign: 200,
            ..t("odd alignment")
        },
        C {
            image: 5,
            ..t("image 5 of 1")
        },
        C {
            image: 0,
            ..t("image 0")
        },
        C {
            last_edit: 1_600_000_000_000,
            ..t("")
        },
        C {
            last_edit: 0,
            ..t("edited at zero")
        },
        C {
            last_edit: -5,
            ..t("edited before 1970")
        },
        C {
            note: "note\twith\ncontrols",
            ..t("text\twith\ncontrols\u{1}")
        },
        t("ta g"),
        mark("first mark"),
        mark("last mark"),
        t("go").link("ta g"),
    ];
    let mut narrow = t("narrow").grid(3, vec![t("x"), t("y"), t("z")]);
    narrow.g().widths = vec![3, -7, 6];
    narrow.g().spacing = 99;
    narrow.g().border = 0x12_3456;
    narrow.g().vertical = false;
    cells.push(narrow);
    cells.push(t("=starts with an equals sign but is data"));
    cells.push(empty());
    let mut file = cts(
        31,
        C {
            bg: 0xcc_dce2,
            ..empty()
        }
        .grid(4, cells),
    );
    file.selection = (9, 9);
    file.images = vec![(b'I', 1000.0, real_png())];
    file.tags = vec![("ta\tg", 0x00_ff00, 0xff_0000), ("unused", 1, 2)];
    file.tags_end = false;
    let mut stream = file.stream();
    let at = stream.windows(2).position(|w| w == b"ZZ").unwrap();
    stream[at] = 0xff;
    stream[at + 1] = 0xfe;
    files.push(("quirks-31", with_stream(&file.header(), &stream)));

    // Zoom: root > a > b > (c, d), c marked.
    let tree = |c: C| {
        t("").grid(
            1,
            vec![t("a").grid(1, vec![t("b").grid(2, vec![c, t("d")])])],
        )
    };
    let with_grid = || mark("c").grid(2, vec![t("x"), t("y")]);
    for (name, c, zoom, selection) in [
        ("zoom-2-leaf", mark("c"), 2, (1, 1)),
        ("zoom-1-leaf", mark("c"), 1, (1, 1)),
        ("zoom-9-leaf", mark("c"), 9, (1, 1)),
        ("zoom-3-grid", with_grid(), 3, (1, 1)),
        ("zoom-3-grid-two-selected", with_grid(), 3, (2, 1)),
        ("zoom-0-size-0", mark("c"), 0, (0, 0)),
    ] {
        let mut file = cts(31, tree(c));
        file.zoom = zoom;
        file.selection = selection;
        files.push((name, file.bytes()));
    }
    let mut file = cts(31, mark("").grid(2, vec![t("a"), t("b")]));
    file.selection = (2, 1);
    file.zoom = 1;
    files.push(("root-marked", file.bytes()));

    // Version 17: images without a length, a fold that takes the image away.
    let mut folded = C {
        image: 0,
        ..t("folded with image")
    }
    .grid(1, vec![t("in")]);
    folded.g().folded = true;
    let open = C {
        image: 1,
        ..t("open with image")
    }
    .grid(1, vec![t("in")]);
    let mut file = cts(
        17,
        t("").grid(
            2,
            vec![
                folded,
                open,
                C {
                    image: 0,
                    ..t("image")
                },
                empty(),
            ],
        ),
    );
    file.images = vec![(b'I', 1.0, real_png()), (b'I', 1.0, real_png())];
    files.push(("quirks-17", file.bytes()));

    // Version 8: an image with a name in front; version 6: the least there is.
    let mut file = cts(
        8,
        t("").grid(
            2,
            vec![
                C {
                    image: 0,
                    style: 3,
                    ..t("image")
                },
                empty(),
            ],
        ),
    );
    file.images = vec![(b'I', 1.0, real_png())];
    files.push(("quirks-8", file.bytes()));
    let root = t("root").grid(
        2,
        vec![
            C {
                style: 3,
                rel_size: 2,
                ..t("six")
            }
            .grid(1, vec![empty()]),
            empty(),
        ],
    );
    files.push(("quirks-6", cts(6, root).bytes()));
    files
}

#[test]
#[ignore = "writes tests/quirks/*.cts; run by hand, then regenerate their golden data"]
fn write_quirk_files() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/quirks");
    for (name, bytes) in quirk_files() {
        std::fs::write(format!("{dir}/{name}.cts"), bytes).unwrap();
    }
}

#[test]
fn the_quirk_files_in_the_repository_are_the_ones_this_code_builds() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/quirks");
    for (name, bytes) in quirk_files() {
        let on_disk = std::fs::read(format!("{dir}/{name}.cts")).unwrap();
        assert!(
            on_disk == bytes,
            "{name}.cts differs: see tests/quirks/README.md"
        );
    }
}
