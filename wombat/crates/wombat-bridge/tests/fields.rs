//! Every field of every command lands in the field of the same name: each of the 45
//! commands is written out here twice by hand, as it is carried and as the core has it,
//! with a different value in every field, and each direction of `convert.rs` is compared
//! with the other hand-written one on its own.
//!
//! The tests that send a command out and read it back in (`roundtrip.rs`, `randseq.rs`)
//! cannot see two fields that are swapped the same way in both directions, such as the
//! rows to move and the place to move them to.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::collections::BTreeSet;

use common::{index, COMMANDS};
use wombat_bridge::api::wire as w;
use wombat_bridge::convert::{command_in, command_out};
use wombat_core::construct::Construct;
use wombat_core::convert::{Entry, Table};
use wombat_core::model::Extra;
use wombat_core::{
    CellId, CellKind, Clip, Color, Command as C, DocId, Document, DrawStyle, Id, ImageRef,
    LinkTarget, Prop, SeededIds, Style, TextAlign, VertAlign,
};

/// The cell with the number `n`, as the core has it.
fn c(n: u128) -> CellId {
    CellId(Id::from_u128(n))
}

/// The ID with the number `n` (below 10) as it is carried, for a cell or a document.
fn t(n: u128) -> String {
    format!("00000000-0000-0000-0000-00000000000{n}")
}

fn d(n: u128) -> DocId {
    DocId(Id::from_u128(n))
}

fn rgb(r: i64, g: i64, b: i64) -> w::Rgb {
    w::Rgb { r, g, b }
}

/// Four styles in which each of the five switches is on in a different set of them, so
/// that no two switches can stand in for each other.
fn styles() -> [(w::Style, Style); 4] {
    let off = (w::Style::default(), Style::default());
    let mut all = [off, off, off, off];
    all[0].0.bold = true;
    all[0].1.bold = true;
    all[1].0.italic = true;
    all[1].1.italic = true;
    all[2].0.fixed = true;
    all[2].1.fixed = true;
    all[3].0.underline = true;
    all[3].1.underline = true;
    all[0].0.strike = true;
    all[0].1.strike = true;
    all[1].0.strike = true;
    all[1].1.strike = true;
    all
}

/// Every property, and every value of those that have a few.
fn props() -> (Vec<w::Prop>, Vec<Prop>) {
    let extra: Extra = [("crop".to_string(), serde_json::json!([1, 2]))].into();
    let carried = vec![
        w::Prop::CellColor(rgb(1, 2, 3)),
        w::Prop::TextColor(rgb(4, 5, 6)),
        w::Prop::RelSize(-7),
        w::Prop::TextAlign(w::HorizAlign::Auto),
        w::Prop::TextAlign(w::HorizAlign::Left),
        w::Prop::TextAlign(w::HorizAlign::Center),
        w::Prop::TextAlign(w::HorizAlign::Right),
        w::Prop::VertAlign(w::VertAlign::Auto),
        w::Prop::VertAlign(w::VertAlign::Top),
        w::Prop::VertAlign(w::VertAlign::Middle),
        w::Prop::VertAlign(w::VertAlign::Bottom),
        w::Prop::Kind(w::CellKind::Data),
        w::Prop::Kind(w::CellKind::Code),
        w::Prop::Kind(w::CellKind::VarAssign),
        w::Prop::Kind(w::CellKind::VarRead),
        w::Prop::Kind(w::CellKind::ViewH),
        w::Prop::Kind(w::CellKind::ViewV),
        w::Prop::Kind(w::CellKind::Formula),
        w::Prop::Note("note\nof two lines".into()),
        w::Prop::DrawStyle(w::DrawStyle::Grid),
        w::Prop::DrawStyle(w::DrawStyle::Bubbles),
        w::Prop::DrawStyle(w::DrawStyle::Lines),
        w::Prop::Image(Some(w::ImageRef {
            file: "a.png".into(),
            scale: 1.5,
            extra: r#"{"crop":[1,2]}"#.into(),
        })),
        w::Prop::Image(Some(w::ImageRef {
            file: "b.png".into(),
            scale: 0.25,
            extra: String::new(),
        })),
        w::Prop::Image(None),
        w::Prop::ImageScale(2.5),
        w::Prop::LastEdit(Some(-8)),
        w::Prop::LastEdit(None),
        w::Prop::BorderColor(rgb(9, 10, 11)),
        w::Prop::OuterSpacing(12),
        w::Prop::Vertical(true),
        w::Prop::Vertical(false),
    ];
    let core = vec![
        Prop::CellColor(Color::rgb(1, 2, 3)),
        Prop::TextColor(Color::rgb(4, 5, 6)),
        Prop::RelSize(-7),
        Prop::TextAlign(TextAlign::Auto),
        Prop::TextAlign(TextAlign::Left),
        Prop::TextAlign(TextAlign::Center),
        Prop::TextAlign(TextAlign::Right),
        Prop::VertAlign(VertAlign::Auto),
        Prop::VertAlign(VertAlign::Top),
        Prop::VertAlign(VertAlign::Middle),
        Prop::VertAlign(VertAlign::Bottom),
        Prop::Kind(CellKind::Data),
        Prop::Kind(CellKind::Code),
        Prop::Kind(CellKind::VarAssign),
        Prop::Kind(CellKind::VarRead),
        Prop::Kind(CellKind::ViewH),
        Prop::Kind(CellKind::ViewV),
        Prop::Kind(CellKind::Formula),
        Prop::Note("note\nof two lines".into()),
        Prop::DrawStyle(DrawStyle::Grid),
        Prop::DrawStyle(DrawStyle::Bubbles),
        Prop::DrawStyle(DrawStyle::Lines),
        Prop::Image(Some(ImageRef {
            file: "a.png".into(),
            scale: 1.5,
            extra,
        })),
        Prop::Image(Some(ImageRef {
            file: "b.png".into(),
            scale: 0.25,
            extra: Extra::new(),
        })),
        Prop::Image(None),
        Prop::ImageScale(2.5),
        Prop::LastEdit(Some(-8)),
        Prop::LastEdit(None),
        Prop::BorderColor(Color::rgb(9, 10, 11)),
        Prop::OuterSpacing(12),
        Prop::Vertical(true),
        Prop::Vertical(false),
    ];
    (carried, core)
}

/// A clip of a small document, with the cell it was in.
fn clip() -> (w::ClipData, Clip) {
    let mut ids = SeededIds::from_u64(3);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), 2, 1, &mut ids).unwrap();
    let carried = w::ClipData {
        parent: Some(t(9)),
        document: wombat_core::format::to_string(&doc).unwrap(),
    };
    (carried, Clip::from_document(&doc, Some(c(9))).unwrap())
}

/// A table of two columns and two rows whose first cell holds a column of two, the
/// second of them a formula.
fn table() -> (w::TableData, Table) {
    let entry = |text: &str, formula, grid: Option<(i64, i64)>| w::EntryData {
        text: text.into(),
        formula,
        grid: grid.map(|(xs, cells)| w::GridSize { xs, cells }),
    };
    let carried = w::TableData {
        xs: 2,
        cells: vec![
            entry("a", false, Some((1, 2))),
            entry("e", false, None),
            entry("=1+1", true, None),
            entry("b", false, None),
            entry("c", false, None),
            entry("d", false, None),
        ],
    };
    let inside = vec![Entry::new("e"), Entry::new("=1+1").as_formula()];
    let cells = vec![
        Entry::new("a").with_grid(Table::new(1, inside).unwrap()),
        Entry::new("b"),
        Entry::new("c"),
        Entry::new("d"),
    ];
    (carried, Table::new(2, cells).unwrap())
}

/// Each command as it is carried and as the core has it. Some are there more than once,
/// for a field that may be none or of several kinds.
fn rows() -> Vec<(w::Command, C)> {
    let [(set_w, set), (clear_w, clear), (on_w, on), (off_w, off)] = styles();
    let (props_w, props) = props();
    let (clip_w, clip) = clip();
    let (table_w, table) = table();
    let link = |target_w, target| {
        let carried = w::Command::SetLink {
            cell: t(1),
            start: 2,
            len: 3,
            target: target_w,
        };
        let core = C::SetLink {
            cell: c(1),
            start: 2,
            len: 3,
            target,
        };
        (carried, core)
    };
    let cell_w = |doc, cell| w::LinkTarget::Cell { doc, cell };
    let two = || (vec![t(1), t(2)], vec![c(1), c(2)]);
    vec![
        (
            w::Command::InsertText {
                cell: t(1),
                at: 2,
                text: "three".into(),
            },
            C::InsertText {
                cell: c(1),
                at: 2,
                text: "three".into(),
            },
        ),
        (
            w::Command::DeleteText {
                cell: t(1),
                start: 2,
                len: 3,
            },
            C::DeleteText {
                cell: c(1),
                start: 2,
                len: 3,
            },
        ),
        (
            w::Command::ReplaceText {
                cell: t(1),
                start: 2,
                len: 3,
                text: "four".into(),
            },
            C::ReplaceText {
                cell: c(1),
                start: 2,
                len: 3,
                text: "four".into(),
            },
        ),
        (
            w::Command::SetText {
                cell: t(1),
                text: "two".into(),
            },
            C::SetText {
                cell: c(1),
                text: "two".into(),
            },
        ),
        (
            w::Command::SetRangeStyle {
                cell: t(1),
                start: 2,
                len: 3,
                set: set_w,
                clear: clear_w,
            },
            C::SetRangeStyle {
                cell: c(1),
                start: 2,
                len: 3,
                set,
                clear,
            },
        ),
        (
            w::Command::SetRangeColor {
                cell: t(1),
                start: 2,
                len: 3,
                color: Some(rgb(4, 5, 6)),
            },
            C::SetRangeColor {
                cell: c(1),
                start: 2,
                len: 3,
                color: Some(Color::rgb(4, 5, 6)),
            },
        ),
        (
            w::Command::SetRangeColor {
                cell: t(1),
                start: 2,
                len: 3,
                color: None,
            },
            C::SetRangeColor {
                cell: c(1),
                start: 2,
                len: 3,
                color: None,
            },
        ),
        link(
            Some(w::LinkTarget::External {
                target: "https://example.org".into(),
            }),
            Some(LinkTarget::External("https://example.org".into())),
        ),
        link(
            Some(cell_w(Some(t(4)), t(5))),
            Some(LinkTarget::Cell {
                doc: Some(d(4)),
                cell: c(5),
            }),
        ),
        link(
            Some(cell_w(None, t(5))),
            Some(LinkTarget::Cell {
                doc: None,
                cell: c(5),
            }),
        ),
        link(Some(w::LinkTarget::Unknown), Some(LinkTarget::Unknown)),
        link(None, None),
        (
            w::Command::SetProps {
                cells: two().0,
                props: props_w,
            },
            C::SetProps {
                cells: two().1,
                props,
            },
        ),
        (
            w::Command::SetStyle {
                cells: two().0,
                set: on_w,
                clear: off_w,
            },
            C::SetStyle {
                cells: two().1,
                set: on,
                clear: off,
            },
        ),
        (
            w::Command::SetColumnWidth {
                cell: t(1),
                column: 2,
                width: 3,
            },
            C::SetColumnWidth {
                cell: c(1),
                column: 2,
                width: 3,
            },
        ),
        (
            w::Command::CreateGrid {
                cell: t(1),
                xs: 2,
                ys: 3,
            },
            C::CreateGrid {
                cell: c(1),
                xs: 2,
                ys: 3,
            },
        ),
        (
            w::Command::RemoveGrid { cell: t(1) },
            C::RemoveGrid { cell: c(1) },
        ),
        (
            w::Command::InsertRows {
                cell: t(1),
                at: 2,
                count: 3,
            },
            C::InsertRows {
                cell: c(1),
                at: 2,
                count: 3,
            },
        ),
        (
            w::Command::InsertColumns {
                cell: t(1),
                at: 2,
                count: 3,
            },
            C::InsertColumns {
                cell: c(1),
                at: 2,
                count: 3,
            },
        ),
        (
            w::Command::DeleteRows {
                cell: t(1),
                at: 2,
                count: 3,
            },
            C::DeleteRows {
                cell: c(1),
                at: 2,
                count: 3,
            },
        ),
        (
            w::Command::DeleteColumns {
                cell: t(1),
                at: 2,
                count: 3,
            },
            C::DeleteColumns {
                cell: c(1),
                at: 2,
                count: 3,
            },
        ),
        (
            w::Command::ClearCells { cells: two().0 },
            C::ClearCells { cells: two().1 },
        ),
        (
            w::Command::DeleteCells {
                cell: t(1),
                x: 2,
                y: 3,
                xs: 4,
                ys: 5,
            },
            C::DeleteCells {
                cell: c(1),
                x: 2,
                y: 3,
                xs: 4,
                ys: 5,
            },
        ),
        (
            w::Command::MoveRows {
                cell: t(1),
                from: 2,
                count: 3,
                to: 4,
            },
            C::MoveRows {
                cell: c(1),
                from: 2,
                count: 3,
                to: 4,
            },
        ),
        (
            w::Command::MoveColumns {
                cell: t(1),
                from: 2,
                count: 3,
                to: 4,
            },
            C::MoveColumns {
                cell: c(1),
                from: 2,
                count: 3,
                to: 4,
            },
        ),
        (
            w::Command::SwapCells { a: t(1), b: t(2) },
            C::SwapCells { a: c(1), b: c(2) },
        ),
        (
            w::Command::MoveCell {
                from: t(1),
                to: t(2),
            },
            C::MoveCell {
                from: c(1),
                to: c(2),
            },
        ),
        (
            w::Command::CopyCell {
                from: t(1),
                to: t(2),
            },
            C::CopyCell {
                from: c(1),
                to: c(2),
            },
        ),
        (
            w::Command::SetTag {
                name: "tag".into(),
                cell_color: rgb(1, 2, 3),
                text_color: rgb(4, 5, 6),
            },
            C::SetTag {
                name: "tag".into(),
                cell_color: Color::rgb(1, 2, 3),
                text_color: Color::rgb(4, 5, 6),
            },
        ),
        (
            w::Command::RemoveTag { name: "tag".into() },
            C::RemoveTag { name: "tag".into() },
        ),
        (
            w::Command::SortRows {
                cell: t(1),
                column: 2,
                row: 3,
                count: 4,
                descending: true,
            },
            C::SortRows {
                cell: c(1),
                column: 2,
                row: 3,
                count: 4,
                descending: true,
            },
        ),
        (
            w::Command::Transpose { cells: two().0 },
            C::Transpose { cells: two().1 },
        ),
        (
            w::Command::HierarchySwap { cell: t(1) },
            C::HierarchySwap { cell: c(1) },
        ),
        (
            w::Command::Hierarchify { cell: t(1) },
            C::Hierarchify { cell: c(1) },
        ),
        (
            w::Command::Flatten { cell: t(1) },
            C::Flatten { cell: c(1) },
        ),
        (
            w::Command::Wrap {
                cell: t(1),
                x: 2,
                y: 3,
                xs: 4,
                ys: 5,
            },
            C::Wrap {
                cell: c(1),
                x: 2,
                y: 3,
                xs: 4,
                ys: 5,
            },
        ),
        (
            w::Command::Collapse {
                cell: t(1),
                x: 2,
                y: 3,
                xs: 4,
                ys: 5,
            },
            C::Collapse {
                cell: c(1),
                x: 2,
                y: 3,
                xs: 4,
                ys: 5,
            },
        ),
        (
            w::Command::SplitCell { cell: t(1), at: 2 },
            C::SplitCell { cell: c(1), at: 2 },
        ),
        (
            w::Command::TagCells { cells: two().0 },
            C::TagCells { cells: two().1 },
        ),
        (
            w::Command::UntagCells { cells: two().0 },
            C::UntagCells { cells: two().1 },
        ),
        (
            w::Command::AdjustRelSize {
                cells: two().0,
                delta: -3,
                recursive: true,
            },
            C::AdjustRelSize {
                cells: two().1,
                delta: -3,
                recursive: true,
            },
        ),
        (
            w::Command::AdjustColumnWidths {
                cell: t(1),
                column: 2,
                count: 3,
                delta: -4,
                recursive: true,
            },
            C::AdjustColumnWidths {
                cell: c(1),
                column: 2,
                count: 3,
                delta: -4,
                recursive: true,
            },
        ),
        (w::Command::Run, C::Run),
        (
            w::Command::EnterText {
                cell: t(1),
                text: "=2".into(),
            },
            C::EnterText {
                cell: c(1),
                text: "=2".into(),
            },
        ),
        (
            w::Command::ToggleCheck { cells: two().0 },
            C::ToggleCheck { cells: two().1 },
        ),
        (
            w::Command::PasteCells {
                to: t(1),
                clip: clip_w.clone(),
            },
            C::PasteCells {
                to: c(1),
                clip: clip.clone(),
            },
        ),
        (
            w::Command::AdoptCells {
                to: t(1),
                clip: clip_w,
            },
            C::AdoptCells { to: c(1), clip },
        ),
        (
            w::Command::ReleaseCell { cell: t(1) },
            C::ReleaseCell { cell: c(1) },
        ),
        (
            w::Command::Import {
                cell: t(1),
                table: table_w,
                replace: true,
            },
            C::Import {
                cell: c(1),
                table,
                replace: true,
            },
        ),
        (
            w::Command::RetargetLinks {
                cells: two().0,
                from: Some(t(3)),
                to: Some(t(4)),
            },
            C::RetargetLinks {
                cells: two().1,
                from: Some(d(3)),
                to: Some(d(4)),
            },
        ),
        (
            w::Command::RetargetLinks {
                cells: two().0,
                from: None,
                to: Some(t(4)),
            },
            C::RetargetLinks {
                cells: two().1,
                from: None,
                to: Some(d(4)),
            },
        ),
        (
            w::Command::RetargetLinks {
                cells: two().0,
                from: Some(t(3)),
                to: None,
            },
            C::RetargetLinks {
                cells: two().1,
                from: Some(d(3)),
                to: None,
            },
        ),
    ]
}

#[test]
fn every_command_is_in_the_table() {
    let listed: BTreeSet<usize> = rows().iter().map(|(_, core)| index(core)).collect();
    assert_eq!(listed, (0..COMMANDS).collect());
}

#[test]
fn a_carried_command_becomes_the_command_of_the_core_field_by_field() {
    for (carried, core) in rows() {
        assert_eq!(command_in(carried).unwrap(), core);
    }
}

#[test]
fn a_command_of_the_core_is_carried_field_by_field() {
    for (carried, core) in rows() {
        assert_eq!(command_out(&core).unwrap(), carried);
    }
}
