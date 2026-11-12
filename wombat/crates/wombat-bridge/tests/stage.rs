//! The stage read: every field of what goes out against a value written by hand, with a
//! different value in every field, so that two fields swapped or one dropped fails; and
//! every number of the request at both ends of its range.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use wombat_bridge::api::wire as w;
use wombat_bridge::api::wire::ErrorKind;
use wombat_bridge::convert::{look_out, stage_ask_in, stage_out};
use wombat_core::model::Extra;
use wombat_core::{
    CellId, CellKind, CellLook, Color, CutText, DocId, DrawStyle, GridPart, GridSummary, Id, Link,
    LinkTarget, Run, StageAsk, StageView, Style, TextAlign, TrailStep, VertAlign,
};

/// The cell with the number `n`, as the core has it.
fn c(n: u128) -> CellId {
    CellId(Id::from_u128(n))
}

/// The ID with the number `n` (below 100) as it is carried.
fn t(n: u128) -> String {
    format!("00000000-0000-0000-0000-0000000000{n:02x}")
}

fn rgb(r: i64, g: i64, b: i64) -> w::Rgb {
    w::Rgb { r, g, b }
}

fn cut(text: &str, cut: bool) -> CutText {
    CutText {
        text: text.into(),
        cut,
    }
}

fn wcut(text: &str, cut: bool) -> w::CutText {
    w::CutText {
        text: text.into(),
        cut,
    }
}

/// A cell in which every field has a value that no other field of it has.
fn a_look() -> (CellLook, w::CellLook) {
    let core = CellLook {
        id: c(1),
        text: cut("the text", true),
        kind: CellKind::Formula,
        result: Some(cut("the result", false)),
        failed: true,
        cell_color: Color::rgb(1, 2, 3),
        text_color: Color::rgb(4, 5, 6),
        is_tag: false,
        style: Style {
            bold: true,
            italic: false,
            fixed: true,
            underline: false,
            strike: false,
        },
        rel_size: -7,
        text_align: TextAlign::Center,
        vert_align: VertAlign::Bottom,
        runs: vec![
            Run {
                start: 8,
                len: 9,
                style: Style {
                    bold: false,
                    italic: true,
                    fixed: false,
                    underline: false,
                    strike: true,
                },
                color: Some(Color::rgb(10, 11, 12)),
                extra: Extra::new(),
            },
            Run {
                start: 20,
                len: 21,
                style: Style {
                    underline: true,
                    ..Style::default()
                },
                color: None,
                extra: Extra::new(),
            },
        ],
        links: vec![
            Link {
                start: 13,
                len: 14,
                target: LinkTarget::External("https://example.org".into()),
                extra: Extra::new(),
            },
            Link {
                start: 30,
                len: 31,
                target: LinkTarget::Cell {
                    doc: Some(DocId(Id::from_u128(2))),
                    cell: c(3),
                },
                extra: Extra::new(),
            },
            Link {
                start: 70,
                len: 71,
                target: LinkTarget::Unknown,
                extra: Extra::new(),
            },
        ],
        image: Some("picture.png".into()),
        has_note: true,
        has_grid: false,
        summary: Some(15),
        full: Some(16),
    };
    let wire = w::CellLook {
        id: t(1),
        text: "the text".into(),
        text_cut: true,
        kind: w::CellKind::Formula,
        result: Some("the result".into()),
        result_cut: false,
        failed: true,
        cell_color: rgb(1, 2, 3),
        text_color: rgb(4, 5, 6),
        is_tag: false,
        style: w::Style {
            bold: true,
            italic: false,
            fixed: true,
            underline: false,
            strike: false,
        },
        rel_size: -7,
        text_align: w::HorizAlign::Center,
        vert_align: w::VertAlign::Bottom,
        runs: vec![
            w::RunInfo {
                start: 8,
                len: 9,
                style: w::Style {
                    bold: false,
                    italic: true,
                    fixed: false,
                    underline: false,
                    strike: true,
                },
                color: Some(rgb(10, 11, 12)),
            },
            w::RunInfo {
                start: 20,
                len: 21,
                style: w::Style {
                    underline: true,
                    ..w::Style::default()
                },
                color: None,
            },
        ],
        links: vec![
            w::LinkInfo {
                start: 13,
                len: 14,
                target: w::LinkTarget::External {
                    target: "https://example.org".into(),
                },
            },
            w::LinkInfo {
                start: 30,
                len: 31,
                target: w::LinkTarget::Cell {
                    doc: Some(t(2)),
                    cell: t(3),
                },
            },
            w::LinkInfo {
                start: 70,
                len: 71,
                target: w::LinkTarget::Unknown,
            },
        ],
        image: Some("picture.png".into()),
        has_note: true,
        has_grid: false,
        summary: Some(15),
        full: Some(16),
    };
    (core, wire)
}

/// The same cell with every switch the other way and nothing where something may be
/// missing: a switch that is carried as "always on", or as another one, fails here or
/// there.
fn another_look() -> (CellLook, w::CellLook) {
    let (mut core, mut wire) = a_look();
    core.text = cut("whole", false);
    (wire.text, wire.text_cut) = ("whole".into(), false);
    core.result = Some(cut("part", true));
    (wire.result, wire.result_cut) = (Some("part".into()), true);
    (core.failed, wire.failed) = (false, false);
    (core.is_tag, wire.is_tag) = (true, true);
    (core.has_note, wire.has_note) = (false, false);
    (core.has_grid, wire.has_grid) = (true, true);
    (core.image, wire.image) = (None, None);
    (core.summary, wire.summary) = (None, None);
    (core.full, wire.full) = (None, None);
    (core.kind, wire.kind) = (CellKind::Data, w::CellKind::Data);
    (core.runs, wire.runs) = (Vec::new(), Vec::new());
    (core.links, wire.links) = (Vec::new(), Vec::new());
    (core, wire)
}

#[test]
fn every_field_of_a_cell_is_the_field_of_that_name() {
    let (core, wire) = a_look();
    assert_eq!(look_out(core), wire);
    let (core, wire) = another_look();
    assert_eq!(look_out(core), wire);
    // A cell that is no formula cell has no result, and then none was cut.
    let (mut core, mut wire) = a_look();
    core.result = None;
    (wire.result, wire.result_cut) = (None, false);
    assert_eq!(look_out(core), wire);
}

#[test]
fn every_field_of_the_stage_is_the_field_of_that_name() {
    let (look, wire_look) = a_look();
    let (other, wire_other) = another_look();
    let core = StageView {
        trail: vec![
            TrailStep {
                id: c(20),
                shown: cut("root", false),
                color: Color::rgb(21, 22, 23),
            },
            TrailStep {
                id: c(24),
                shown: cut("stage", true),
                color: Color::rgb(25, 26, 27),
            },
        ],
        grids: vec![
            GridPart {
                owner: c(30),
                level: 31,
                xs: 32,
                ys: 33,
                x0: 34,
                y0: 35,
                cols: 36,
                rows: 37,
                first: 38,
                col_widths: vec![39, 40, 41],
                border_color: Color::rgb(42, 43, 44),
                outer_spacing: 45,
                vertical: true,
                draw_style: DrawStyle::Lines,
            },
            GridPart {
                owner: c(46),
                level: 0,
                xs: 1,
                ys: 2,
                x0: 3,
                y0: 4,
                cols: 5,
                rows: 6,
                first: 7,
                col_widths: Vec::new(),
                border_color: Color::rgb(8, 9, 10),
                outer_spacing: 11,
                vertical: false,
                draw_style: DrawStyle::Bubbles,
            },
        ],
        cells: vec![look, other],
        summaries: vec![
            GridSummary {
                owner: c(50),
                xs: 51,
                ys: 52,
                border_color: Color::rgb(53, 54, 55),
                preview_cols: 56,
                preview_rows: 57,
                preview: vec![cut("first", false), cut("second", true)],
                shape_cols: 58,
                shape_rows: 59,
                shape: vec![60, 61, 62, 63, 0, 255, 128, 1],
            },
            GridSummary {
                owner: c(64),
                xs: 65,
                ys: 66,
                border_color: Color::rgb(67, 68, 69),
                preview_cols: 70,
                preview_rows: 71,
                preview: Vec::new(),
                shape_cols: 72,
                shape_rows: 73,
                shape: Vec::new(),
            },
        ],
    };
    let wire = w::StageInfo {
        trail: vec![
            w::TrailStep {
                id: t(20),
                shown: wcut("root", false),
                color: rgb(21, 22, 23),
            },
            w::TrailStep {
                id: t(24),
                shown: wcut("stage", true),
                color: rgb(25, 26, 27),
            },
        ],
        grids: vec![
            w::GridPart {
                owner: t(30),
                level: 31,
                xs: 32,
                ys: 33,
                x0: 34,
                y0: 35,
                cols: 36,
                rows: 37,
                first: 38,
                col_widths: vec![39, 40, 41],
                border_color: rgb(42, 43, 44),
                outer_spacing: 45,
                vertical: true,
                draw_style: w::DrawStyle::Lines,
            },
            w::GridPart {
                owner: t(46),
                level: 0,
                xs: 1,
                ys: 2,
                x0: 3,
                y0: 4,
                cols: 5,
                rows: 6,
                first: 7,
                col_widths: Vec::new(),
                border_color: rgb(8, 9, 10),
                outer_spacing: 11,
                vertical: false,
                draw_style: w::DrawStyle::Bubbles,
            },
        ],
        cells: vec![wire_look, wire_other],
        summaries: vec![
            w::GridSummary {
                owner: t(50),
                xs: 51,
                ys: 52,
                border_color: rgb(53, 54, 55),
                preview_cols: 56,
                preview_rows: 57,
                preview: vec![wcut("first", false), wcut("second", true)],
                shape_cols: 58,
                shape_rows: 59,
                shape: vec![60, 61, 62, 63, 0, 255, 128, 1],
            },
            w::GridSummary {
                owner: t(64),
                xs: 65,
                ys: 66,
                border_color: rgb(67, 68, 69),
                preview_cols: 70,
                preview_rows: 71,
                preview: Vec::new(),
                shape_cols: 72,
                shape_rows: 73,
                shape: Vec::new(),
            },
        ],
    };
    assert_eq!(stage_out(core), wire);
}

/// A request with a different number in every field.
fn an_ask() -> w::StageAsk {
    w::StageAsk {
        x0: 1,
        y0: 2,
        cols: 3,
        rows: 4,
        full_depth: 5,
        full_cells: 6,
        preview_cols: 7,
        preview_rows: 8,
        shape_cols: 9,
        shape_rows: 10,
        text_cut: 11,
        preview_cut: 12,
    }
}

#[test]
fn every_number_of_a_request_is_the_number_of_that_name() {
    let expected = StageAsk {
        x0: 1,
        y0: 2,
        cols: 3,
        rows: 4,
        full_depth: 5,
        full_cells: 6,
        preview_cols: 7,
        preview_rows: 8,
        shape_cols: 9,
        shape_rows: 10,
        text_cut: 11,
        preview_cut: 12,
    };
    assert_eq!(stage_ask_in(an_ask()).unwrap(), expected);
}

/// Each number of a request by its name, to set it and to read what the core got.
type Number = (
    &'static str,
    fn(&mut w::StageAsk) -> &mut i64,
    fn(&StageAsk) -> u32,
);

const NUMBERS: [Number; 12] = [
    ("x0", |a| &mut a.x0, |a| a.x0),
    ("y0", |a| &mut a.y0, |a| a.y0),
    ("cols", |a| &mut a.cols, |a| a.cols),
    ("rows", |a| &mut a.rows, |a| a.rows),
    ("full_depth", |a| &mut a.full_depth, |a| a.full_depth),
    ("full_cells", |a| &mut a.full_cells, |a| a.full_cells),
    ("preview_cols", |a| &mut a.preview_cols, |a| a.preview_cols),
    ("preview_rows", |a| &mut a.preview_rows, |a| a.preview_rows),
    ("shape_cols", |a| &mut a.shape_cols, |a| a.shape_cols),
    ("shape_rows", |a| &mut a.shape_rows, |a| a.shape_rows),
    ("text_cut", |a| &mut a.text_cut, |a| a.text_cut),
    ("preview_cut", |a| &mut a.preview_cut, |a| a.preview_cut),
];

/// A number that the core cannot take is refused, never cut: 4294967296 would arrive as
/// 0, and "all rows" would be none.
#[test]
fn every_number_of_a_request_is_checked_at_both_ends() {
    for (name, number, got) in NUMBERS {
        for (given, taken) in [(0, 0), (i64::from(u32::MAX), u32::MAX)] {
            let mut ask = an_ask();
            *number(&mut ask) = given;
            let core = stage_ask_in(ask).unwrap_or_else(|e| panic!("{name} {given}: {e}"));
            assert_eq!(got(&core), taken, "{name}");
        }
        for given in [-1, i64::from(u32::MAX) + 1, i64::MIN, i64::MAX] {
            let mut ask = an_ask();
            *number(&mut ask) = given;
            let error = stage_ask_in(ask).expect_err(name);
            assert_eq!(error.kind, ErrorKind::BadRequest, "{name} {given}");
            assert!(
                error.message.contains(&given.to_string()),
                "{name}: {error}"
            );
        }
    }
}
