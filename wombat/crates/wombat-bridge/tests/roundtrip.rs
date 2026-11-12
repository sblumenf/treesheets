//! The serialised form: every command of the core survives being carried and written as
//! JSON text unchanged, and so do change reports and errors.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::strategy::{Union, ValueTree};
use proptest::test_runner::TestRunner;
use serde_json::{json, Value};
use wombat_bridge::api::wire::{self, BridgeError, ErrorKind, Report};
use wombat_bridge::convert::{clip_out, command_in, command_out};
use wombat_bridge::record::{command_from_json, command_to_json, report_from_json, report_to_json};
use wombat_core::convert::{Entry, Table};
use wombat_core::model::Extra;
use wombat_core::{
    CellId, CellKind, Clip, Color, Command, DocId, Document, DrawStyle, Editor, Id, ImageRef,
    LinkTarget, Prop, SeededIds, Style, TextAlign, VertAlign,
};

/// The number of commands the core has. The match in [`index`] does not compile when one
/// is added; then this, [`index`] and [`command`] need the new one.
const COMMANDS: usize = 45;

/// Which command a command is, from 0.
fn index(command: &Command) -> usize {
    match command {
        Command::InsertText { .. } => 0,
        Command::DeleteText { .. } => 1,
        Command::ReplaceText { .. } => 2,
        Command::SetText { .. } => 3,
        Command::SetRangeStyle { .. } => 4,
        Command::SetRangeColor { .. } => 5,
        Command::SetLink { .. } => 6,
        Command::SetProps { .. } => 7,
        Command::SetStyle { .. } => 8,
        Command::SetColumnWidth { .. } => 9,
        Command::CreateGrid { .. } => 10,
        Command::RemoveGrid { .. } => 11,
        Command::InsertRows { .. } => 12,
        Command::InsertColumns { .. } => 13,
        Command::DeleteRows { .. } => 14,
        Command::DeleteColumns { .. } => 15,
        Command::ClearCells { .. } => 16,
        Command::DeleteCells { .. } => 17,
        Command::MoveRows { .. } => 18,
        Command::MoveColumns { .. } => 19,
        Command::SwapCells { .. } => 20,
        Command::MoveCell { .. } => 21,
        Command::CopyCell { .. } => 22,
        Command::SetTag { .. } => 23,
        Command::RemoveTag { .. } => 24,
        Command::SortRows { .. } => 25,
        Command::Transpose { .. } => 26,
        Command::HierarchySwap { .. } => 27,
        Command::Hierarchify { .. } => 28,
        Command::Flatten { .. } => 29,
        Command::Wrap { .. } => 30,
        Command::Collapse { .. } => 31,
        Command::SplitCell { .. } => 32,
        Command::TagCells { .. } => 33,
        Command::UntagCells { .. } => 34,
        Command::AdjustRelSize { .. } => 35,
        Command::AdjustColumnWidths { .. } => 36,
        Command::Run => 37,
        Command::EnterText { .. } => 38,
        Command::ToggleCheck { .. } => 39,
        Command::PasteCells { .. } => 40,
        Command::AdoptCells { .. } => 41,
        Command::ReleaseCell { .. } => 42,
        Command::Import { .. } => 43,
        Command::RetargetLinks { .. } => 44,
    }
}

/// Every kind of error. The match in [`every_kind_is_listed`] does not compile when one is
/// added; then this needs the new one.
const KINDS: &[ErrorKind] = &[
    ErrorKind::Io,
    ErrorKind::Syntax,
    ErrorKind::WrongFormat,
    ErrorKind::NewerVersion,
    ErrorKind::Invalid,
    ErrorKind::BadRequest,
    ErrorKind::Refused,
    ErrorKind::Limit,
    ErrorKind::NotFound,
    ErrorKind::AlreadyExists,
    ErrorKind::HalfSaved,
    ErrorKind::ChangedOnDisk,
    ErrorKind::AlreadyOpen,
    ErrorKind::Closed,
    ErrorKind::Internal,
];

#[test]
fn every_kind_is_listed() {
    for kind in KINDS {
        match kind {
            ErrorKind::Io
            | ErrorKind::Syntax
            | ErrorKind::WrongFormat
            | ErrorKind::NewerVersion
            | ErrorKind::Invalid
            | ErrorKind::BadRequest
            | ErrorKind::Refused
            | ErrorKind::Limit
            | ErrorKind::NotFound
            | ErrorKind::AlreadyExists
            | ErrorKind::HalfSaved
            | ErrorKind::ChangedOnDisk
            | ErrorKind::AlreadyOpen
            | ErrorKind::Closed
            | ErrorKind::Internal => {}
        }
    }
    let distinct: BTreeSet<String> = KINDS.iter().map(|kind| format!("{kind:?}")).collect();
    assert_eq!(distinct.len(), KINDS.len());
}

fn cell() -> impl Strategy<Value = CellId> {
    any::<u128>().prop_map(|bits| CellId(Id::from_u128(bits)))
}

fn cells() -> impl Strategy<Value = Vec<CellId>> {
    proptest::collection::vec(cell(), 0..4)
}

fn doc() -> impl Strategy<Value = Option<DocId>> {
    proptest::option::of(any::<u128>().prop_map(|bits| DocId(Id::from_u128(bits))))
}

/// Any text at all: what a command holds is carried as it is, and judged when applied.
fn text() -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>(), 0..6).prop_map(String::from_iter)
}

fn color() -> impl Strategy<Value = Color> {
    any::<(u8, u8, u8)>().prop_map(|(r, g, b)| Color::rgb(r, g, b))
}

fn style() -> impl Strategy<Value = Style> {
    any::<[bool; 5]>().prop_map(|[bold, italic, fixed, underline, strike]| Style {
        bold,
        italic,
        fixed,
        underline,
        strike,
    })
}

fn finite() -> impl Strategy<Value = f64> {
    any::<f64>().prop_filter("finite", |n| n.is_finite())
}

fn value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::from),
        any::<i64>().prop_map(Value::from),
        finite().prop_map(Value::from),
        text().prop_map(Value::from),
    ];
    leaf.prop_recursive(3, 12, 3, |inner| {
        prop_oneof![
            proptest::collection::vec(inner.clone(), 0..3).prop_map(Value::from),
            proptest::collection::btree_map(text(), inner, 0..3)
                .prop_map(|map| Value::Object(map.into_iter().collect())),
        ]
    })
}

fn extra() -> impl Strategy<Value = Extra> {
    proptest::collection::btree_map(text(), value(), 0..3)
}

fn image() -> impl Strategy<Value = ImageRef> {
    (text(), finite(), extra()).prop_map(|(file, scale, extra)| ImageRef { file, scale, extra })
}

fn target() -> impl Strategy<Value = LinkTarget> {
    prop_oneof![
        text().prop_map(LinkTarget::External),
        (doc(), cell()).prop_map(|(doc, cell)| LinkTarget::Cell { doc, cell }),
        Just(LinkTarget::Unknown),
    ]
}

fn pick<T: Copy + std::fmt::Debug + 'static>(all: &'static [T]) -> impl Strategy<Value = T> {
    proptest::sample::select(all)
}

fn prop() -> impl Strategy<Value = Prop> {
    prop_oneof![
        color().prop_map(Prop::CellColor),
        color().prop_map(Prop::TextColor),
        any::<i32>().prop_map(Prop::RelSize),
        pick(TextAlign::ALL).prop_map(Prop::TextAlign),
        pick(VertAlign::ALL).prop_map(Prop::VertAlign),
        pick(CellKind::ALL).prop_map(Prop::Kind),
        text().prop_map(Prop::Note),
        pick(DrawStyle::ALL).prop_map(Prop::DrawStyle),
        proptest::option::of(image()).prop_map(Prop::Image),
        finite().prop_map(Prop::ImageScale),
        any::<Option<i64>>().prop_map(Prop::LastEdit),
        color().prop_map(Prop::BorderColor),
        any::<u32>().prop_map(Prop::OuterSpacing),
        any::<bool>().prop_map(Prop::Vertical),
    ]
}

/// A clip of a cell of a small document that has what cells can have: text, a grid inside
/// a grid, styles, links within the clip, out of it and to another document, a note, an
/// image, a formula.
fn clip() -> impl Strategy<Value = Clip> {
    (any::<u64>(), any::<usize>(), text(), image(), doc()).prop_map(
        |(seed, pick, note, image, other)| {
            let mut ids = SeededIds::from_u64(seed);
            let doc = Document::new(&mut ids);
            let root = doc.root();
            let mut editor = Editor::new(doc).unwrap();
            let mut apply = |editor: &mut Editor, command| {
                // What a random note or image name makes impossible is left out.
                let _ = editor.apply(command, Some(5), &mut ids);
            };
            let grid = |cell, xs, ys| Command::CreateGrid { cell, xs, ys };
            apply(&mut editor, grid(root, 2, 2));
            let at = |editor: &Editor, x, y| editor.document().child(root, x, y).unwrap();
            let (a, b, c) = (at(&editor, 0, 0), at(&editor, 1, 0), at(&editor, 0, 1));
            apply(&mut editor, grid(a, 1, 2));
            let inner = editor.document().child(a, 0, 0).unwrap();
            for (cell, text) in [(a, "about"), (inner, "inside"), (b, "=1+2"), (c, "links")] {
                let text = text.to_string();
                apply(&mut editor, Command::EnterText { cell, text });
            }
            let link = |start, target| Command::SetLink {
                cell: c,
                start,
                len: 1,
                target: Some(target),
            };
            let to = |doc, cell| LinkTarget::Cell { doc, cell };
            apply(&mut editor, link(0, to(None, inner)));
            apply(&mut editor, link(1, to(None, b)));
            apply(&mut editor, link(2, to(other, a)));
            apply(&mut editor, link(3, LinkTarget::External("x:y".into())));
            let bold = Style {
                bold: true,
                ..Style::default()
            };
            apply(
                &mut editor,
                Command::SetRangeStyle {
                    cell: a,
                    start: 1,
                    len: 2,
                    set: bold,
                    clear: Style::default(),
                },
            );
            // The name of an image file has rules; its scale and unknown fields are free.
            let image = ImageRef {
                file: "a.png".into(),
                ..image
            };
            for props in [vec![Prop::Note(note)], vec![Prop::Image(Some(image))]] {
                let cells = vec![a, inner];
                apply(&mut editor, Command::SetProps { cells, props });
            }
            let all = editor.document().cells_in_order();
            Clip::of(editor.document(), all[pick % all.len()]).unwrap()
        },
    )
}

/// A table with tables inside its cells, up to three levels.
fn table() -> impl Strategy<Value = Table> {
    let entry = |(text, formula): (String, bool)| {
        let entry = Entry::new(&text);
        if formula {
            entry.as_formula()
        } else {
            entry
        }
    };
    let text = prop_oneof![text(), Just("=1+1".to_string())];
    let leaf = proptest::collection::vec((text, any::<bool>()), 1..5).prop_map(move |texts| {
        let cells: Vec<Entry> = texts.into_iter().map(entry).collect();
        Table::new(1, cells).unwrap()
    });
    leaf.prop_recursive(2, 24, 4, |inner| {
        let cells = proptest::collection::vec(proptest::option::of(inner), 1..5);
        (1..3u32, cells).prop_map(|(xs, grids)| {
            let cell = |(n, grid): (usize, Option<Table>)| {
                let cell = Entry::new(&n.to_string());
                match grid {
                    Some(grid) => cell.with_grid(grid),
                    None => cell,
                }
            };
            let cells: Vec<Entry> = grids.into_iter().enumerate().map(cell).collect();
            let xs = if cells.len().is_multiple_of(xs as usize) {
                xs
            } else {
                1
            };
            Table::new(xs, cells).unwrap()
        })
    })
}

/// Any command: every one of the 45, with any values in its fields.
fn command() -> impl Strategy<Value = Command> {
    let n = any::<u32>;
    let block = || (cell(), n(), n(), n(), n());
    Union::new(vec![
        (cell(), n(), text())
            .prop_map(|(cell, at, text)| Command::InsertText { cell, at, text })
            .boxed(),
        (cell(), n(), n())
            .prop_map(|(cell, start, len)| Command::DeleteText { cell, start, len })
            .boxed(),
        (cell(), n(), n(), text())
            .prop_map(|(cell, start, len, text)| Command::ReplaceText {
                cell,
                start,
                len,
                text,
            })
            .boxed(),
        (cell(), text())
            .prop_map(|(cell, text)| Command::SetText { cell, text })
            .boxed(),
        (cell(), n(), n(), style(), style())
            .prop_map(|(cell, start, len, set, clear)| Command::SetRangeStyle {
                cell,
                start,
                len,
                set,
                clear,
            })
            .boxed(),
        (cell(), n(), n(), proptest::option::of(color()))
            .prop_map(|(cell, start, len, color)| Command::SetRangeColor {
                cell,
                start,
                len,
                color,
            })
            .boxed(),
        (cell(), n(), n(), proptest::option::of(target()))
            .prop_map(|(cell, start, len, target)| Command::SetLink {
                cell,
                start,
                len,
                target,
            })
            .boxed(),
        (cells(), proptest::collection::vec(prop(), 0..4))
            .prop_map(|(cells, props)| Command::SetProps { cells, props })
            .boxed(),
        (cells(), style(), style())
            .prop_map(|(cells, set, clear)| Command::SetStyle { cells, set, clear })
            .boxed(),
        (cell(), n(), n())
            .prop_map(|(cell, column, width)| Command::SetColumnWidth {
                cell,
                column,
                width,
            })
            .boxed(),
        (cell(), n(), n())
            .prop_map(|(cell, xs, ys)| Command::CreateGrid { cell, xs, ys })
            .boxed(),
        cell().prop_map(|cell| Command::RemoveGrid { cell }).boxed(),
        (cell(), n(), n())
            .prop_map(|(cell, at, count)| Command::InsertRows { cell, at, count })
            .boxed(),
        (cell(), n(), n())
            .prop_map(|(cell, at, count)| Command::InsertColumns { cell, at, count })
            .boxed(),
        (cell(), n(), n())
            .prop_map(|(cell, at, count)| Command::DeleteRows { cell, at, count })
            .boxed(),
        (cell(), n(), n())
            .prop_map(|(cell, at, count)| Command::DeleteColumns { cell, at, count })
            .boxed(),
        cells()
            .prop_map(|cells| Command::ClearCells { cells })
            .boxed(),
        block()
            .prop_map(|(cell, x, y, xs, ys)| Command::DeleteCells { cell, x, y, xs, ys })
            .boxed(),
        (cell(), n(), n(), n())
            .prop_map(|(cell, from, count, to)| Command::MoveRows {
                cell,
                from,
                count,
                to,
            })
            .boxed(),
        (cell(), n(), n(), n())
            .prop_map(|(cell, from, count, to)| Command::MoveColumns {
                cell,
                from,
                count,
                to,
            })
            .boxed(),
        (cell(), cell())
            .prop_map(|(a, b)| Command::SwapCells { a, b })
            .boxed(),
        (cell(), cell())
            .prop_map(|(from, to)| Command::MoveCell { from, to })
            .boxed(),
        (cell(), cell())
            .prop_map(|(from, to)| Command::CopyCell { from, to })
            .boxed(),
        (text(), color(), color())
            .prop_map(|(name, cell_color, text_color)| Command::SetTag {
                name,
                cell_color,
                text_color,
            })
            .boxed(),
        text().prop_map(|name| Command::RemoveTag { name }).boxed(),
        (cell(), n(), n(), n(), any::<bool>())
            .prop_map(|(cell, column, row, count, descending)| Command::SortRows {
                cell,
                column,
                row,
                count,
                descending,
            })
            .boxed(),
        cells()
            .prop_map(|cells| Command::Transpose { cells })
            .boxed(),
        cell()
            .prop_map(|cell| Command::HierarchySwap { cell })
            .boxed(),
        cell()
            .prop_map(|cell| Command::Hierarchify { cell })
            .boxed(),
        cell().prop_map(|cell| Command::Flatten { cell }).boxed(),
        block()
            .prop_map(|(cell, x, y, xs, ys)| Command::Wrap { cell, x, y, xs, ys })
            .boxed(),
        block()
            .prop_map(|(cell, x, y, xs, ys)| Command::Collapse { cell, x, y, xs, ys })
            .boxed(),
        (cell(), n())
            .prop_map(|(cell, at)| Command::SplitCell { cell, at })
            .boxed(),
        cells()
            .prop_map(|cells| Command::TagCells { cells })
            .boxed(),
        cells()
            .prop_map(|cells| Command::UntagCells { cells })
            .boxed(),
        (cells(), any::<i32>(), any::<bool>())
            .prop_map(|(cells, delta, recursive)| Command::AdjustRelSize {
                cells,
                delta,
                recursive,
            })
            .boxed(),
        (cell(), n(), n(), any::<i32>(), any::<bool>())
            .prop_map(
                |(cell, column, count, delta, recursive)| Command::AdjustColumnWidths {
                    cell,
                    column,
                    count,
                    delta,
                    recursive,
                },
            )
            .boxed(),
        Just(Command::Run).boxed(),
        (cell(), text())
            .prop_map(|(cell, text)| Command::EnterText { cell, text })
            .boxed(),
        cells()
            .prop_map(|cells| Command::ToggleCheck { cells })
            .boxed(),
        (cell(), clip())
            .prop_map(|(to, clip)| Command::PasteCells { to, clip })
            .boxed(),
        (cell(), clip())
            .prop_map(|(to, clip)| Command::AdoptCells { to, clip })
            .boxed(),
        cell()
            .prop_map(|cell| Command::ReleaseCell { cell })
            .boxed(),
        (cell(), table(), any::<bool>())
            .prop_map(|(cell, table, replace)| Command::Import {
                cell,
                table,
                replace,
            })
            .boxed(),
        (cells(), doc(), doc())
            .prop_map(|(cells, from, to)| Command::RetargetLinks { cells, from, to })
            .boxed(),
    ])
}

fn report() -> impl Strategy<Value = Report> {
    let ids = || proptest::collection::vec(cell().prop_map(|id| id.to_string()), 0..4);
    let tags = proptest::collection::vec(text(), 0..3);
    (ids(), ids(), ids(), ids(), tags, ids()).prop_map(
        |(changed, created, removed, grids, tags, recalculated)| Report {
            changed,
            created,
            removed,
            grids,
            tags,
            recalculated,
        },
    )
}

#[test]
fn the_generator_makes_every_command() {
    let mut runner = TestRunner::deterministic();
    let mut seen = BTreeSet::new();
    for _ in 0..3000 {
        seen.insert(index(&command().new_tree(&mut runner).unwrap().current()));
    }
    assert_eq!(seen, (0..COMMANDS).collect());
}

#[test]
fn the_form_of_a_command_as_json_is_as_documented() {
    let cell = CellId(Id::from_u128(1));
    let set = Command::SetText {
        cell,
        text: "new".into(),
    };
    let text = command_to_json(&set).unwrap();
    let expected = json!({"SetText": {
        "cell": "00000000-0000-0000-0000-000000000001", "text": "new"}});
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), expected);
    assert_eq!(command_to_json(&Command::Run).unwrap(), "\"Run\"");
    let props = Command::SetProps {
        cells: vec![cell],
        props: vec![Prop::CellColor(Color::rgb(1, 2, 3)), Prop::Vertical(true)],
    };
    let text = command_to_json(&props).unwrap();
    let expected = json!({"SetProps": {
        "cells": ["00000000-0000-0000-0000-000000000001"],
        "props": [{"CellColor": {"r": 1, "g": 2, "b": 3}}, {"Vertical": true}]}});
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), expected);
}

/// The message of what the bridge itself refused, which has a kind of its own.
fn bad_request<T: std::fmt::Debug>(result: Result<T, BridgeError>) -> String {
    let error = result.unwrap_err();
    assert_eq!(error.kind, ErrorKind::BadRequest, "{error:?}");
    assert_eq!(error.path, None);
    error.message
}

/// The message of what the core refused as breaking a rule of documents.
fn invalid<T: std::fmt::Debug>(result: Result<T, BridgeError>) -> String {
    let error = result.unwrap_err();
    assert_eq!(error.kind, ErrorKind::Invalid, "{error:?}");
    error.message
}

#[test]
fn what_cannot_be_a_command_is_refused_as_a_value() {
    bad_request(command_from_json("{"));
    bad_request(command_from_json("\"NoSuchCommand\""));
    bad_request(command_from_json(r#"{"RemoveGrid": {"cell": "1"}}"#));
    // Only the canonical text of an ID is one: lower case, with its hyphens.
    let upper = r#"{"RemoveGrid": {"cell": "00000000-0000-0000-0000-00000000000A"}}"#;
    assert!(bad_request(command_from_json(upper)).contains("is not an ID"));
    let cell = "00000000-0000-0000-0000-000000000001".to_string();
    for scale in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let props = vec![wire::Prop::ImageScale(scale)];
        let cells = vec![cell.clone()];
        bad_request(command_in(wire::Command::SetProps { cells, props }));
        let nan = Command::SetProps {
            cells: Vec::new(),
            props: vec![Prop::ImageScale(scale)],
        };
        bad_request(command_out(&nan));
    }
    let image = |extra: &str| wire::Command::SetProps {
        cells: vec![cell.clone()],
        props: vec![wire::Prop::Image(Some(wire::ImageRef {
            file: "a.png".into(),
            scale: 1.0,
            extra: extra.into(),
        }))],
    };
    command_in(image("")).unwrap();
    command_in(image(r#"{"crop": [1, 2]}"#)).unwrap();
    bad_request(command_in(image("[1]")));
    bad_request(command_in(image("{")));
    let table = |xs, n| wire::Command::Import {
        cell: cell.clone(),
        table: wire::TableData {
            xs,
            cells: vec![
                wire::EntryData {
                    text: "x".into(),
                    formula: false,
                    grid: None,
                };
                n
            ],
        },
        replace: false,
    };
    command_in(table(2, 4)).unwrap();
    // A table is the core's to judge, as a document is.
    invalid(command_in(table(2, 3)));
    invalid(command_in(table(0, 3)));
    invalid(command_in(table(1, 0)));
}

/// Dart's whole numbers have 64 bits and cross as they are (decision A134). What does not
/// fit the core's number is refused; it is not cut to fit, which made row 2^32 row 0.
#[test]
fn numbers_out_of_range_are_refused() {
    let cell = || "00000000-0000-0000-0000-000000000001".to_string();
    let rows = |at, count| {
        command_in(wire::Command::InsertRows {
            cell: cell(),
            at,
            count,
        })
    };
    let (two_32, two_31) = (1i64 << 32, 1i64 << 31);
    let rgb = |r, g, b| wire::Rgb { r, g, b };
    let props = |prop| {
        command_in(wire::Command::SetProps {
            cells: vec![cell()],
            props: vec![prop],
        })
    };
    let size = |delta| {
        command_in(wire::Command::AdjustRelSize {
            cells: vec![cell()],
            delta,
            recursive: false,
        })
    };
    let widths = |delta| {
        command_in(wire::Command::AdjustColumnWidths {
            cell: cell(),
            column: 0,
            count: 1,
            delta,
            recursive: false,
        })
    };
    let table = |xs| {
        command_in(wire::Command::Import {
            cell: cell(),
            table: wire::TableData {
                xs,
                cells: vec![wire::EntryData {
                    text: "x".into(),
                    formula: false,
                    grid: None,
                }],
            },
            replace: false,
        })
    };

    // Positions, counts and sizes: 0 to 2^32 - 1.
    rows(0, two_32 - 1).unwrap();
    rows(two_32 - 1, 0).unwrap();
    let message = bad_request(rows(two_32, 1));
    assert!(message.contains("4294967296"), "{message}");
    bad_request(rows(0, two_32 + 1));
    bad_request(rows(-1, 1));
    bad_request(rows(0, -1));
    bad_request(rows(i64::MAX, 1));
    bad_request(rows(i64::MIN, 1));
    props(wire::Prop::OuterSpacing(two_32 - 1)).unwrap();
    bad_request(props(wire::Prop::OuterSpacing(two_32)));
    bad_request(props(wire::Prop::OuterSpacing(-1)));
    table(1).unwrap();
    bad_request(table(two_32 + 1));
    bad_request(table(-1));

    // Differences and the relative size: -2^31 to 2^31 - 1.
    for delta in [-two_31, -1, 0, two_31 - 1] {
        size(delta).unwrap();
        widths(delta).unwrap();
        props(wire::Prop::RelSize(delta)).unwrap();
    }
    for delta in [two_31, -two_31 - 1, two_32 + 3, i64::MAX, i64::MIN] {
        bad_request(size(delta));
        bad_request(widths(delta));
        bad_request(props(wire::Prop::RelSize(delta)));
    }

    // The parts of a colour: 0 to 255, each of the three.
    props(wire::Prop::CellColor(rgb(0, 255, 0))).unwrap();
    for part in [256, -1, two_32] {
        for color in [rgb(part, 0, 0), rgb(0, part, 0), rgb(0, 0, part)] {
            bad_request(props(wire::Prop::CellColor(color)));
            bad_request(props(wire::Prop::TextColor(color)));
            bad_request(props(wire::Prop::BorderColor(color)));
            bad_request(command_in(wire::Command::SetRangeColor {
                cell: cell(),
                start: 0,
                len: 1,
                color: Some(color),
            }));
            for (cell_color, text_color) in [(color, rgb(0, 0, 0)), (rgb(0, 0, 0), color)] {
                bad_request(command_in(wire::Command::SetTag {
                    name: "t".into(),
                    cell_color,
                    text_color,
                }));
            }
        }
    }

    // The time of the last edit is a 64-bit number in the core too: all of it is one.
    props(wire::Prop::LastEdit(Some(i64::MIN))).unwrap();
    // As JSON the same holds, and a number that 64 bits do not hold is no command.
    let json = |at: &str| {
        let text = format!(r#"{{"SplitCell": {{"cell": "{}", "at": {at}}}}}"#, cell());
        command_from_json(&text)
    };
    json("4294967295").unwrap();
    bad_request(json("4294967296"));
    bad_request(json("-1"));
    bad_request(json("18446744073709551616"));
    bad_request(json("1.5"));
}

/// Puts `number` in the place of whole number `which` of a command as JSON, counting
/// from 0 in the order of the text; says whether there was one. The time of the last
/// edit is left out: it has no range to be out of.
fn replace_number(value: &mut Value, which: &mut usize, number: i64) -> bool {
    match value {
        Value::Number(n) if n.is_i64() || n.is_u64() => {
            if *which == 0 {
                *value = Value::from(number);
                return true;
            }
            *which -= 1;
            false
        }
        Value::Array(items) => items
            .iter_mut()
            .any(|item| replace_number(item, which, number)),
        Value::Object(map) => map
            .iter_mut()
            .any(|(key, item)| key != "LastEdit" && replace_number(item, which, number)),
        _ => false,
    }
}

/// The JSON form is strict (decision A136): a command says everything, and nothing else.
/// A field that may be none is written `null`; leaving it out is not a way to say none.
#[test]
fn json_that_leaves_a_field_out_or_adds_one_is_refused() {
    let id = "00000000-0000-0000-0000-000000000001";
    let read = |text: &str| command_from_json(&text.replace("ID", id));
    let cell = CellId(Id::from_u128(1));

    // `null` is none; no field is nothing.
    let color = read(r#"{"SetRangeColor": {"cell": "ID", "start": 0, "len": 1, "color": null}}"#);
    let expected = Command::SetRangeColor {
        cell,
        start: 0,
        len: 1,
        color: None,
    };
    assert_eq!(color.unwrap(), expected);
    let message = bad_request(read(
        r#"{"SetRangeColor": {"cell": "ID", "start": 0, "len": 1}}"#,
    ));
    assert!(message.contains("missing field `color`"), "{message}");
    read(r#"{"SetLink": {"cell": "ID", "start": 0, "len": 1, "target": null}}"#).unwrap();
    bad_request(read(r#"{"SetLink": {"cell": "ID", "start": 0, "len": 1}}"#));
    read(r#"{"RetargetLinks": {"cells": [], "from": null, "to": "ID"}}"#).unwrap();
    bad_request(read(r#"{"RetargetLinks": {"cells": [], "to": "ID"}}"#));
    bad_request(read(r#"{"RetargetLinks": {"cells": [], "from": "ID"}}"#));
    let link = |target: &str| {
        let text = r#"{"SetLink": {"cell": "ID", "start": 0, "len": 1, "target": TARGET}}"#;
        read(&text.replace("TARGET", target))
    };
    link(r#"{"Cell": {"doc": null, "cell": "ID"}}"#).unwrap();
    bad_request(link(r#"{"Cell": {"cell": "ID"}}"#));
    let entry = |entry: &str| {
        let text = r#"{"Import": {"cell": "ID", "replace": false,
            "table": {"xs": 1, "cells": [ENTRY]}}}"#;
        read(&text.replace("ENTRY", entry))
    };
    entry(r#"{"text": "x", "formula": false, "grid": null}"#).unwrap();
    bad_request(entry(r#"{"text": "x", "formula": false}"#));
    let props = |prop: &str| {
        read(&format!(
            r#"{{"SetProps": {{"cells": [], "props": [{prop}]}}}}"#
        ))
    };
    props(r#"{"Image": null}"#).unwrap();
    props(r#"{"LastEdit": null}"#).unwrap();
    bad_request(props(r#""Image""#));
    bad_request(props(r#""LastEdit""#));
    let paste = |clip: &str| {
        read(&format!(
            r#"{{"PasteCells": {{"to": "ID", "clip": {clip}}}}}"#
        ))
    };
    let doc = Document::new(&mut SeededIds::from_u64(8));
    let carried = clip_out(&Clip::of(&doc, doc.root()).unwrap()).unwrap();
    let document = serde_json::to_string(&carried.document).unwrap();
    paste(&format!(r#"{{"parent": null, "document": {document}}}"#)).unwrap();
    bad_request(paste(&format!(r#"{{"document": {document}}}"#)));
    // The field of a clip that is no more (decision A132) is as unknown as any.
    bad_request(paste(&format!(
        r#"{{"source": "ID", "parent": null, "document": {document}}}"#
    )));

    // A field that the command does not have: a misspelt one, most likely.
    read(r#"{"SetText": {"cell": "ID", "text": "a"}}"#).unwrap();
    let message = bad_request(read(
        r#"{"SetText": {"cell": "ID", "text": "a", "txet": "b"}}"#,
    ));
    assert!(message.contains("unknown field `txet`"), "{message}");
    bad_request(read(
        r#"{"SetText": {"cell": "ID", "text": "a", "text": "b"}}"#,
    ));
    bad_request(read(
        r#"{"SetText": {"cell": "ID", "text": "a"}, "Run": null}"#,
    ));
    bad_request(props(r#"{"CellColor": {"r": 1, "g": 2, "b": 3, "a": 4}}"#));
    bad_request(link(r#"{"External": {"target": "x:y", "title": "t"}}"#));
    bad_request(entry(
        r#"{"text": "x", "formula": false, "grid": null, "note": ""}"#,
    ));

    // A command without fields is its name. `{"Run": null}` says the same and is read
    // too: that is how `serde` reads any variant without fields, and nothing is lost or
    // guessed by it. Anything else in that place is refused.
    assert_eq!(read(r#""Run""#).unwrap(), Command::Run);
    assert_eq!(read(r#"{"Run": null}"#).unwrap(), Command::Run);
    bad_request(read(r#"{"Run": {}}"#));
    bad_request(read(r#"{"Run": 0}"#));
    bad_request(read(r#"{"Run": []}"#));
    bad_request(read(r#"["Run"]"#));

    // A report and an error are as strict.
    let full = r#"{"changed": [], "created": [], "removed": [], "grids": [], "tags": [],
        "recalculated": []}"#;
    assert_eq!(report_from_json(full).unwrap(), Report::default());
    bad_request(report_from_json(&full.replace(r#""tags": [],"#, "")));
    bad_request(report_from_json(&full.replace('{', r#"{"moved": [],"#)));
    let error = |text: &str| serde_json::from_str::<BridgeError>(text);
    error(r#"{"kind": "Io", "message": "m", "path": null}"#).unwrap();
    error(r#"{"kind": "Io", "message": "m"}"#).unwrap_err();
    error(r#"{"kind": "Io", "message": "m", "path": null, "code": 2}"#).unwrap_err();
}

/// Takes field `which` out of the JSON objects of a command, counting through all of
/// them in the order of the text, or with `add` puts a field in that no object has;
/// says whether there was such an object.
fn tamper(value: &mut Value, which: &mut usize, add: bool) -> bool {
    match value {
        Value::Array(items) => items.iter_mut().any(|item| tamper(item, which, add)),
        Value::Object(map) => {
            if *which < map.len() {
                if add {
                    map.insert("zz".into(), Value::Null);
                } else if let Some(key) = map.keys().nth(*which).cloned() {
                    map.remove(&key);
                }
                return true;
            }
            *which -= map.len();
            map.values_mut().any(|item| tamper(item, which, add))
        }
        _ => false,
    }
}

#[test]
fn a_clip_that_was_tampered_with_is_refused() {
    let mut ids = SeededIds::from_u64(3);
    let doc = Document::new(&mut ids);
    let root = doc.root();
    let mut editor = Editor::new(doc).unwrap();
    let grid = Command::CreateGrid {
        cell: root,
        xs: 1,
        ys: 2,
    };
    editor.apply(grid, None, &mut ids).unwrap();
    let good = clip_out(&Clip::of(editor.document(), root).unwrap()).unwrap();
    let paste = |clip: wire::ClipData| {
        command_in(wire::Command::PasteCells {
            to: root.to_string(),
            clip,
        })
    };
    paste(good.clone()).unwrap();
    // A grid that says it has more columns than it has cells for.
    let mut bad = good.clone();
    bad.document = bad.document.replacen("\"xs\": 1", "\"xs\": 2", 1);
    assert_ne!(bad.document, good.document);
    assert!(paste(bad).is_err());
    // Not a document at all.
    let mut bad = good.clone();
    bad.document = "{}".into();
    assert!(paste(bad).is_err());
    // A document whose ID, the source of the cells, is the ID of the top cell.
    let mut bad = good.clone();
    let id = editor.document().id().to_string();
    bad.document = bad.document.replacen(&id, &root.to_string(), 1);
    assert_ne!(bad.document, good.document);
    assert!(paste(bad).is_err());
    // The cell that the top cell was in with the ID of the document.
    let mut bad = good.clone();
    bad.parent = Some(id);
    invalid(paste(bad));
    // A parent that is no ID is the bridge's to refuse.
    let mut bad = good.clone();
    bad.parent = Some("above".into());
    bad_request(paste(bad));
    // The top cell inside one of its own cells.
    let mut bad = good.clone();
    bad.parent = Some(editor.document().child(root, 0, 1).unwrap().to_string());
    invalid(paste(bad));
}

/// A clip says once where its cells came from: by the ID of the document it is carried as.
#[test]
fn the_source_of_a_carried_clip_is_the_id_of_its_document() {
    let mut ids = SeededIds::from_u64(4);
    let doc = Document::new(&mut ids);
    let (id, root) = (doc.id(), doc.root());
    let good = clip_out(&Clip::of(&doc, root).unwrap()).unwrap();
    let source = |clip: wire::ClipData| {
        let paste = wire::Command::PasteCells {
            to: root.to_string(),
            clip,
        };
        match command_in(paste).unwrap() {
            Command::PasteCells { clip, .. } => clip.source(),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(source(good.clone()), id);
    // The same cells carried as a document with another ID came from that document.
    let other = DocId(Id::from_u128(77));
    let mut carried = good.clone();
    carried.document = good.document.replace(&id.to_string(), &other.to_string());
    assert_ne!(carried.document, good.document);
    assert_eq!(source(carried), other);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// Every command comes back unchanged from the serialised form, and from that as JSON.
    #[test]
    fn commands_survive_the_serialised_form(command in command()) {
        let carried = command_out(&command).unwrap();
        prop_assert_eq!(&command_in(carried.clone()).unwrap(), &command);
        let text = serde_json::to_string(&carried).unwrap();
        prop_assert_eq!(&serde_json::from_str::<wire::Command>(&text).unwrap(), &carried);
        let text = command_to_json(&command).unwrap();
        prop_assert_eq!(&command_from_json(&text).unwrap(), &command);
    }

    /// Every whole number of every command is checked: with any one of them out of every
    /// range, the command is refused.
    #[test]
    fn every_number_of_a_command_is_checked(command in command(), which in 0..12usize, high: bool) {
        let number = if high { 1i64 << 32 } else { -(1i64 << 31) - 1 };
        let mut value = serde_json::to_value(command_out(&command).unwrap()).unwrap();
        if replace_number(&mut value, &mut which.clone(), number) {
            let carried: wire::Command = serde_json::from_value(value).unwrap();
            let error = command_in(carried).unwrap_err();
            prop_assert_eq!(error.kind, ErrorKind::BadRequest);
            prop_assert!(error.message.contains(&number.to_string()));
        }
    }

    /// No field of any command can be left out of its JSON, and none can be added.
    #[test]
    fn every_field_of_a_command_is_needed_and_no_other_is_read(
        command in command(), which in 0..24usize, add: bool
    ) {
        let mut value = serde_json::to_value(command_out(&command).unwrap()).unwrap();
        if tamper(&mut value, &mut which.clone(), add) {
            let error = command_from_json(&value.to_string()).unwrap_err();
            prop_assert_eq!(error.kind, ErrorKind::BadRequest, "{}", value);
        }
    }

    #[test]
    fn reports_survive_json(report in report()) {
        let text = report_to_json(&report).unwrap();
        prop_assert_eq!(report_from_json(&text).unwrap(), report);
    }

    #[test]
    fn errors_survive_json(kind in pick(KINDS), message in text(), path in proptest::option::of(text())) {
        let error = BridgeError { kind, message, path };
        let text = serde_json::to_string(&error).unwrap();
        prop_assert_eq!(serde_json::from_str::<BridgeError>(&text).unwrap(), error);
    }

    /// Text that is no command is an error value, whatever it is.
    #[test]
    fn any_text_is_a_command_or_an_error(text in ".{0,40}") {
        let _ = command_from_json(&text);
    }
}
