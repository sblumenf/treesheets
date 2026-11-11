//! Shared by the random-sequence tests of commands (here and in `wombat-cts`, which runs
//! them on imported TreeSheets documents): random commands for whatever document is at
//! hand, and the checks that every command, undo and redo must pass.
#![allow(
    dead_code,
    clippy::manual_is_multiple_of,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeSet;

use proptest::prelude::*;
use unicode_segmentation::UnicodeSegmentation;
use wombat_core::text::floor_boundary;
use wombat_core::{
    format, Cell, CellId, CellKind, Changes, Color, Command, Document, DrawStyle, Editor, Error,
    Id, ImageRef, LinkTarget, Prop, SeededIds, Style, TextAlign, VertAlign,
};

/// User-perceived characters of one to five code points each: letters, an accent written
/// both ways, emoji with a joiner, a skin tone and a variation selector, two flags, Hangul
/// written both ways. Any sequence of them segments back into exactly them.
pub const CLUSTERS: &[&str] = &[
    "a",
    "b",
    " ",
    "é",
    "e\u{301}",
    "日",
    "\u{1F600}",
    "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    "\u{1F44D}\u{1F3FD}",
    "\u{2764}\u{FE0F}",
    "\u{1F1E9}\u{1F1EA}",
    "\u{1F1EB}\u{1F1F7}",
    "\u{D55C}",
    "\u{1100}\u{1161}\u{11A8}",
    // The to-do checkboxes that `ToggleCheck` writes.
    "\u{2610}",
    "\u{2611}",
];

/// A random command before it is fitted to a document: a kind and raw numbers.
#[derive(Clone, Debug)]
pub struct Op {
    pub kind: u8,
    pub n: [u32; 5],
    pub clusters: Vec<usize>,
    /// Use text positions as they come, also inside a character (to be refused).
    pub raw: bool,
    pub color: (u8, u8, u8),
}

pub const KINDS: u8 = 38;

pub fn op() -> impl Strategy<Value = Op> {
    (
        0..KINDS,
        any::<[u32; 5]>(),
        proptest::collection::vec(0..CLUSTERS.len(), 0..4),
        proptest::bool::weighted(0.1),
        any::<(u8, u8, u8)>(),
    )
        .prop_map(|(kind, n, clusters, raw, color)| Op {
            kind,
            n,
            clusters,
            raw,
            color,
        })
}

pub fn text_of(clusters: &[usize]) -> String {
    clusters.iter().map(|c| CLUSTERS[*c]).collect()
}

fn pick<T: Copy>(items: &[T], n: u32) -> Option<T> {
    (!items.is_empty()).then(|| items[n as usize % items.len()])
}

fn style(n: u32) -> Style {
    Style {
        bold: n & 1 != 0,
        italic: n & 2 != 0,
        fixed: n & 4 != 0,
        underline: n & 8 != 0,
        strike: n & 16 != 0,
    }
}

/// Fits a random command to the document: cells are picked from those it has, positions
/// from inside the texts, so that most commands can be carried out. Some still cannot, on
/// purpose: those must be refused and change nothing.
pub fn command(doc: &Document, op: &Op) -> Command {
    let all = doc.cells_in_order();
    let cell_of = |id: &CellId| doc.cell(*id).unwrap();
    let with_grid: Vec<CellId> = all
        .iter()
        .copied()
        .filter(|c| cell_of(c).grid().is_some())
        .collect();
    let leaves: Vec<CellId> = all
        .iter()
        .copied()
        .filter(|c| cell_of(c).grid().is_none())
        .collect();
    let [n0, n1, n2, n3, n4] = op.n;
    let cell = pick(&all, n0).unwrap();
    // Mostly a cell that has text, so that text commands have something to work on.
    let wordy: Vec<CellId> = all
        .iter()
        .copied()
        .filter(|c| !cell_of(c).text.is_empty())
        .collect();
    let text_cell = if n4 % 4 == 0 {
        cell
    } else {
        pick(&wordy, n0).unwrap_or(cell)
    };
    let text = &cell_of(&text_cell).text;
    let len = text.chars().count() as u32;
    let position = |n: u32| {
        let at = n % (len + 2);
        if op.raw {
            at
        } else {
            floor_boundary(text, at)
        }
    };
    let (start, end) = {
        let (a, b) = (position(n1), position(n2));
        (a.min(b), a.max(b))
    };
    let color = Color::rgb(op.color.0, op.color.1, op.color.2);
    // Moving lines needs two of them, wrapping and collapsing are mostly about several
    // cells: for those, a grid that has them, if there is one.
    let sized = |fits: &dyn Fn(u32, u32) -> bool| -> Vec<CellId> {
        let fits = |c: &&CellId| cell_of(c).grid().is_some_and(|g| fits(g.xs(), g.ys()));
        with_grid.iter().filter(fits).copied().collect()
    };
    let roomy = match op.kind {
        18 => sized(&|_, ys| ys > 1),
        19 => sized(&|xs, _| xs > 1),
        26 | 30 => sized(&|xs, ys| xs * ys > 1),
        // Collapsing needs a text in the block as well.
        31 => sized(&|xs, ys| xs * ys > 1)
            .into_iter()
            .filter(|c| {
                let grid = cell_of(c).grid().unwrap();
                grid.cells().iter().any(|k| !cell_of(k).text.is_empty())
            })
            .collect(),
        _ => Vec::new(),
    };
    let grid_cell = pick(&roomy, n0).or(pick(&with_grid, n0)).unwrap_or(cell);
    let (xs, ys) = cell_of(&grid_cell)
        .grid()
        .map_or((1, 1), |g| (g.xs(), g.ys()));
    // Now and then numbers that may be out of range, to be refused.
    let sloppy = u32::from(n4 % 6 == 0);
    let several = || {
        let count = 1 + n4 % 3;
        (0..count)
            .map(|i| pick(&all, n0.wrapping_add(i * 7)).unwrap())
            .collect::<Vec<_>>()
    };
    let new_text = || {
        let mut text = text_of(&op.clusters);
        if n4 % 9 == 0 {
            text.push('\n');
        }
        text
    };
    match op.kind {
        0 => Command::InsertText {
            cell: text_cell,
            at: start,
            text: new_text(),
        },
        1 => Command::DeleteText {
            cell: text_cell,
            start,
            len: end - start,
        },
        2 => Command::ReplaceText {
            cell: text_cell,
            start,
            len: end - start,
            text: new_text(),
        },
        3 => Command::SetText {
            cell,
            text: new_text(),
        },
        4 => Command::SetRangeStyle {
            cell: text_cell,
            start,
            len: end - start,
            set: style(n3),
            clear: style(n4),
        },
        5 => Command::SetRangeColor {
            cell: text_cell,
            start,
            len: end - start,
            color: (n3 % 4 != 0).then_some(color),
        },
        6 => Command::SetLink {
            cell: text_cell,
            start,
            len: end - start,
            target: match n3 % 4 {
                0 => None,
                1 => Some(LinkTarget::External(format!(
                    "https://example.org/{}",
                    n4 % 3
                ))),
                2 => Some(LinkTarget::Cell {
                    doc: None,
                    cell: pick(&all, n4).unwrap(),
                }),
                _ => Some(LinkTarget::Cell {
                    doc: Some(doc.id()),
                    cell: pick(&all, n4).unwrap(),
                }),
            },
        },
        7 => {
            let prop = match n1 % 14 {
                0 => Prop::CellColor(color),
                1 => Prop::TextColor(color),
                2 => Prop::RelSize(n2 as i32 % 5 - 2),
                3 => Prop::TextAlign(TextAlign::ALL[n2 as usize % 4]),
                4 => Prop::VertAlign(VertAlign::ALL[n2 as usize % 4]),
                5 => Prop::Kind(CellKind::ALL[n2 as usize % 7]),
                6 => Prop::Note(if n2 % 3 == 0 {
                    String::new()
                } else {
                    format!("note\n{}", n2 % 4)
                }),
                7 => Prop::DrawStyle(DrawStyle::ALL[n2 as usize % 3]),
                8 => Prop::Image((n2 % 3 != 0).then(|| ImageRef::new(format!("i{}.png", n2 % 3)))),
                9 => Prop::ImageScale(f64::from(n2 % 4) * 0.5),
                10 => Prop::LastEdit((n2 % 3 != 0).then_some(i64::from(n2))),
                11 => Prop::BorderColor(color),
                12 => Prop::OuterSpacing(n2 % 6),
                _ => Prop::Vertical(n2 % 2 == 0),
            };
            // Grid properties mostly on cells that have a grid.
            let cells = if n1 % 14 >= 11 && n4 % 5 != 0 {
                vec![grid_cell]
            } else {
                several()
            };
            Command::SetProps {
                cells,
                props: vec![prop],
            }
        }
        8 => Command::SetStyle {
            cells: several(),
            set: style(n1),
            clear: style(n2),
        },
        9 => Command::SetColumnWidth {
            cell: grid_cell,
            column: n1 % (xs + 1),
            width: n2 % 200,
        },
        10 => Command::CreateGrid {
            cell: pick(&leaves, n0).unwrap_or(cell),
            xs: n1 % 4,
            ys: n2 % 4,
        },
        11 => Command::RemoveGrid { cell: grid_cell },
        12 => Command::InsertRows {
            cell: grid_cell,
            at: n1 % (ys + 2),
            count: n2 % 3,
        },
        13 => Command::InsertColumns {
            cell: grid_cell,
            at: n1 % (xs + 2),
            count: n2 % 3,
        },
        14 => Command::DeleteRows {
            cell: grid_cell,
            at: n1 % (ys + sloppy),
            count: 1 + n2 % 2,
        },
        15 => Command::DeleteColumns {
            cell: grid_cell,
            at: n1 % (xs + sloppy),
            count: 1 + n2 % 2,
        },
        16 => Command::ClearCells { cells: several() },
        17 => Command::DeleteCells {
            cell: grid_cell,
            x: n1 % (xs + sloppy),
            y: n2 % (ys + sloppy),
            xs: (1 - sloppy) + n3 % (xs - n1 % xs + sloppy),
            ys: (1 - sloppy) + n4 / 8 % (ys - n2 % ys + sloppy),
        },
        18 | 19 => {
            // Lines that exist, to a place that exists, unless sloppy.
            let along = if op.kind == 18 { ys } else { xs };
            let from = n1 % (along + sloppy);
            let count = 1 + n2 % (along - from.min(along - 1)).min(2) + sloppy * (n2 % 2);
            let places = along.saturating_sub(count) + 1 + sloppy;
            // Mostly to another place than where they are.
            let to = if places > 1 && n4 % 7 != 0 {
                (from + 1 + n3 % (places - 1)) % places
            } else {
                n3 % places
            };
            if op.kind == 18 {
                Command::MoveRows {
                    cell: grid_cell,
                    from,
                    count,
                    to,
                }
            } else {
                Command::MoveColumns {
                    cell: grid_cell,
                    from,
                    count,
                    to,
                }
            }
        }
        20 => Command::SwapCells {
            a: cell,
            b: pick(&all, n1).unwrap(),
        },
        21 => {
            let empty: Vec<CellId> = leaves
                .iter()
                .copied()
                .filter(|c| cell_of(c).text.is_empty() && cell_of(c).image.is_none())
                .collect();
            Command::MoveCell {
                from: cell,
                to: if n2 % 5 == 0 {
                    pick(&all, n1).unwrap()
                } else {
                    pick(&empty, n1).unwrap_or(cell)
                },
            }
        }
        22 => Command::CopyCell {
            // Whole subtrees only while the document is small, or it grows without end.
            from: if all.len() > 400 {
                pick(&leaves, n0).unwrap_or(cell)
            } else {
                cell
            },
            to: pick(&all, n1).unwrap(),
        },
        23 => Command::SetTag {
            name: match n1 % 4 {
                0 => String::new(),
                n => format!("tag{n}"),
            },
            cell_color: color,
            text_color: Color::rgb(op.color.2, op.color.1, op.color.0),
        },
        24 => Command::RemoveTag {
            name: format!("tag{}", n1 % 4),
        },
        25 => {
            // Mostly by a column that has a text in it, or there is nothing to sort by.
            let in_rows: Vec<CellId> = wordy
                .iter()
                .copied()
                .filter(|c| {
                    let parent = cell_of(c).parent().map(|p| cell_of(&p));
                    parent.and_then(Cell::grid).is_some_and(|g| g.ys() > 1)
                })
                .collect();
            let keyed = pick(&in_rows, n1).and_then(|c| {
                let parent = cell_of(&c).parent()?;
                let grid = cell_of(&parent).grid()?;
                let index = grid.cells().iter().position(|k| *k == c)?;
                Some((parent, index as u32 % grid.xs(), grid.ys()))
            });
            let (owner, column, rows) = match keyed {
                Some(keyed) if n4 % 5 != 0 => keyed,
                _ => (grid_cell, n1 % (xs + sloppy), ys),
            };
            let row = if n3 % 4 == 0 { n2 % rows } else { 0 };
            Command::SortRows {
                cell: owner,
                column,
                row,
                count: rows - row + sloppy,
                descending: n2 % 2 == 0,
            }
        }
        26 => Command::Transpose {
            cells: if n4 % 4 == 0 {
                several()
            } else {
                vec![grid_cell]
            },
        },
        27 => {
            // Mostly a cell that can be swapped: with text, two levels down, in lines.
            let line = |c: &Cell| c.grid().is_some_and(|g| g.xs() == 1 || g.ys() == 1);
            let fit: Vec<CellId> = wordy
                .iter()
                .copied()
                .filter(|c| {
                    let parent = cell_of(c).parent().map(|p| cell_of(&p));
                    let base = parent.and_then(Cell::parent).map(|p| cell_of(&p));
                    parent.is_some_and(line) && base.is_some_and(line)
                })
                .collect();
            Command::HierarchySwap {
                cell: if n4 % 5 == 0 {
                    cell
                } else {
                    pick(&fit, n1).unwrap_or(cell)
                },
            }
        }
        28 => {
            let is_table = |c: &CellId| {
                let grid = cell_of(c).grid().unwrap();
                grid.cells().iter().all(|k| cell_of(k).grid().is_none())
            };
            let tables: Vec<CellId> = with_grid.iter().copied().filter(is_table).collect();
            // Mostly a table with something to do: several columns.
            let wide: Vec<CellId> = tables
                .iter()
                .copied()
                .filter(|c| cell_of(c).grid().unwrap().xs() > 1)
                .collect();
            Command::Hierarchify {
                cell: if n4 % 5 == 0 {
                    cell
                } else {
                    pick(&wide, n1).or(pick(&tables, n1)).unwrap_or(cell)
                },
            }
        }
        29 => Command::Flatten {
            // A flattened hierarchy can be many times its size: only while small.
            cell: if all.len() > 300 || n4 % 7 == 0 {
                cell
            } else {
                grid_cell
            },
        },
        30 | 31 => {
            let (mut x, mut y) = (n1 % (xs + sloppy), n2 % (ys + sloppy));
            let (mut bxs, mut bys) = (
                (1 - sloppy) + n3 % (xs - n1 % xs + sloppy),
                (1 - sloppy) + n4 / 8 % (ys - n2 % ys + sloppy),
            );
            // Collapsing takes two cells or more: mostly up to the end of the grid.
            if op.kind == 31 && sloppy == 0 && n4 % 4 != 0 {
                if n4 % 3 != 0 {
                    (x, y) = (0, 0);
                }
                (bxs, bys) = (xs - x, ys - y);
            }
            if op.kind == 30 {
                Command::Wrap {
                    cell: grid_cell,
                    x,
                    y,
                    xs: bxs,
                    ys: bys,
                }
            } else {
                Command::Collapse {
                    cell: grid_cell,
                    x,
                    y,
                    xs: bxs,
                    ys: bys,
                }
            }
        }
        32 => Command::SplitCell {
            cell: text_cell,
            at: start,
        },
        33 => Command::TagCells { cells: several() },
        34 => {
            let tagged: Vec<CellId> = all
                .iter()
                .copied()
                .filter(|c| doc.tags().contains_key(&cell_of(c).text))
                .collect();
            let mut cells = several();
            cells.extend(pick(&tagged, n1));
            Command::UntagCells { cells }
        }
        35 => Command::AdjustRelSize {
            cells: several(),
            delta: n1 as i32 % 5 - 2,
            recursive: n2 % 2 == 0,
        },
        36 => {
            let column = n1 % (xs + sloppy);
            Command::AdjustColumnWidths {
                cell: grid_cell,
                column,
                count: 1 + n2 % (xs - column.min(xs - 1)) + sloppy,
                delta: (n3 as i32 % 7 - 3) * 5,
                recursive: n4 % 2 == 0,
            }
        }
        _ => Command::ToggleCheck { cells: several() },
    }
}

/// The content of a cell that is its own: everything except where it is and which cells
/// its grid has.
fn content_differs(a: &Cell, b: &Cell) -> bool {
    let own = |c: &Cell| {
        (
            (
                c.text.clone(),
                c.kind,
                c.cell_color,
                c.text_color,
                c.draw_style,
                c.note.clone(),
            ),
            (
                c.text_align,
                c.vert_align,
                c.rel_size,
                c.style,
                c.runs.clone(),
                c.links.clone(),
            ),
            (c.last_edit, c.image.clone(), c.extra.clone()),
        )
    };
    let grid_properties = match (a.grid(), b.grid()) {
        (Some(x), Some(y)) => {
            (
                x.border_color,
                x.outer_spacing,
                x.vertical,
                &x.col_widths,
                &x.extra,
            ) != (
                y.border_color,
                y.outer_spacing,
                y.vertical,
                &y.col_widths,
                &y.extra,
            )
        }
        _ => false,
    };
    own(a) != own(b) || grid_properties
}

fn shape_differs(a: &Cell, b: &Cell) -> bool {
    let shape = |c: &Cell| c.grid().map(|g| (g.xs(), g.ys(), g.cells().to_vec()));
    shape(a) != shape(b)
}

/// Checks a change report against the documents before and after: it must name every
/// cell, grid and tag that differs (complete), and nothing that does not (tight).
pub fn check_report(before: &Document, after: &Document, changes: &Changes, what: &str) {
    let ids = |doc: &Document| doc.cells_in_order().into_iter().collect::<BTreeSet<_>>();
    let (old, new) = (ids(before), ids(after));
    let created: Vec<CellId> = new.difference(&old).copied().collect();
    let removed: Vec<CellId> = old.difference(&new).copied().collect();
    assert_eq!(changes.created, created, "{what}: created cells");
    assert_eq!(changes.removed, removed, "{what}: removed cells");
    let (mut changed, mut grids) = (Vec::new(), Vec::new());
    let reported: BTreeSet<CellId> = changes
        .grids
        .iter()
        .chain(&created)
        .chain(&removed)
        .copied()
        .collect();
    for id in old.intersection(&new) {
        let (a, b) = (before.cell(*id).unwrap(), after.cell(*id).unwrap());
        if content_differs(a, b) {
            changed.push(*id);
        }
        if shape_differs(a, b) {
            grids.push(*id);
        }
        // A cell that moved is reported through the grids it left and entered.
        if a.parent() != b.parent() {
            for parent in [a.parent(), b.parent()].into_iter().flatten() {
                assert!(
                    reported.contains(&parent),
                    "{what}: {id} moved, {parent} not reported"
                );
            }
        }
    }
    assert_eq!(changes.changed, changed, "{what}: changed cells");
    assert_eq!(changes.grids, grids, "{what}: changed grids");
    let names: BTreeSet<&String> = before.tags().keys().chain(after.tags().keys()).collect();
    let tags: Vec<String> = names
        .into_iter()
        .filter(|name| before.tags().get(*name) != after.tags().get(*name))
        .cloned()
        .collect();
    assert_eq!(changes.tags, tags, "{what}: changed tags");
    assert_eq!(
        (before.id(), before.root(), before.extra()),
        (after.id(), after.root(), after.extra())
    );
}

/// Checks what must hold for every document a command leaves behind: it is valid (which
/// covers runs and links being sorted, apart and inside the text, and no control characters)
/// and it saves and loads back equal. With `whole_clusters`, also that every text is still
/// made of whole characters from [`CLUSTERS`].
pub fn check_document(doc: &Document, whole_clusters: bool) {
    doc.validate().unwrap();
    if whole_clusters {
        for id in doc.cells_in_order() {
            let text = &doc.cell(id).unwrap().text;
            for cluster in text.graphemes(true) {
                assert!(
                    CLUSTERS.contains(&cluster),
                    "{text:?} has the piece {cluster:?}"
                );
            }
        }
    }
}

pub fn saved(doc: &Document) -> String {
    let text = format::to_string(doc).unwrap();
    assert_eq!(
        format::from_slice(text.as_bytes()).unwrap(),
        *doc,
        "save then load"
    );
    text
}

/// Applies a command with every check: valid afterwards; if refused, nothing changed; if
/// carried out, the report is complete and tight. Returns whether it was carried out.
pub fn apply_checked(
    editor: &mut Editor,
    op: &Op,
    ids: &mut SeededIds,
    whole_clusters: bool,
) -> Result<Changes, Error> {
    let before = editor.document().clone();
    let history = (editor.undo_len(), editor.redo_len());
    let command = command(&before, op);
    let now = (op.n[4] % 3 != 0).then_some(1_700_000_000_000 + i64::from(op.n[4] % 1000));
    let result = editor.apply(command.clone(), now, ids);
    let what = format!("{command:?}");
    match &result {
        Ok(changes) => {
            check_report(&before, editor.document(), changes, &what);
            check_document(editor.document(), whole_clusters);
            if changes.is_empty() {
                assert_eq!((editor.undo_len(), editor.redo_len()), history, "{what}");
            }
        }
        Err(_) => {
            assert_eq!(
                *editor.document(),
                before,
                "refused, but changed something: {what}"
            );
            assert_eq!((editor.undo_len(), editor.redo_len()), history, "{what}");
        }
    }
    result
}

/// The whole life of a document under random commands: every command checked; then undo
/// everything, back to the original document and the original bytes; then redo everything,
/// forward to the same final document, bytes and IDs. Returns (carried out, refused).
pub fn run_sequence(original: Document, ops: &[Op], whole_clusters: bool) -> (usize, usize) {
    let original_text = saved(&original);
    let mut ids = SeededIds::from_u64(99);
    let mut editor = Editor::new(original.clone()).unwrap();
    let (mut done, mut refused) = (0, 0);
    let mut trail = vec![original.clone()];
    for op in ops {
        match apply_checked(&mut editor, op, &mut ids, whole_clusters) {
            Ok(changes) if changes.is_empty() => {}
            Ok(_) => {
                done += 1;
                trail.push(editor.document().clone());
            }
            Err(_) => refused += 1,
        }
    }
    assert_eq!(editor.undo_len(), done);
    let last = editor.document().clone();
    let last_text = saved(&last);
    // Undo everything, checking each stop on the way and each report.
    for expected in trail.iter().rev().skip(1) {
        let before = editor.document().clone();
        let changes = editor.undo().unwrap().unwrap();
        assert_eq!(editor.document(), expected, "undo");
        check_report(&before, editor.document(), &changes, "undo");
    }
    assert_eq!(editor.undo().unwrap(), None);
    assert_eq!(*editor.document(), original);
    assert_eq!(
        saved(editor.document()),
        original_text,
        "undoing everything gives the same bytes"
    );
    for expected in trail.iter().skip(1) {
        let before = editor.document().clone();
        let changes = editor.redo().unwrap().unwrap();
        assert_eq!(editor.document(), expected, "redo");
        check_report(&before, editor.document(), &changes, "redo");
    }
    assert_eq!(editor.redo().unwrap(), None);
    assert_eq!(*editor.document(), last);
    assert_eq!(
        saved(editor.document()),
        last_text,
        "redoing everything gives the same bytes"
    );
    (done, refused)
}

/// One move in a random walk through the history.
#[derive(Clone, Debug)]
pub enum Act {
    Apply(Op),
    /// Several commands as one undo unit.
    Group(Vec<Op>),
    Undo,
    Redo,
}

pub fn act() -> impl Strategy<Value = Act> {
    prop_oneof![
        5 => op().prop_map(Act::Apply),
        1 => proptest::collection::vec(op(), 0..4).prop_map(Act::Group),
        3 => Just(Act::Undo),
        2 => Just(Act::Redo),
    ]
}

/// Commands, groups, undo and redo in any order, against a model that keeps a copy of the
/// document for every unit of the history: each undo and redo must land on the copy.
pub fn run_walk(original: Document, acts: &[Act], whole_clusters: bool) {
    let mut ids = SeededIds::from_u64(77);
    let mut editor = Editor::new(original.clone()).unwrap();
    let (mut past, mut future): (Vec<Document>, Vec<Document>) = (Vec::new(), Vec::new());
    for act in acts {
        let before = editor.document().clone();
        match act {
            Act::Apply(op) => {
                if apply_checked(&mut editor, op, &mut ids, whole_clusters)
                    .is_ok_and(|c| !c.is_empty())
                {
                    past.push(before);
                    future.clear();
                }
            }
            Act::Group(ops) => {
                editor.begin_group();
                let mut any = false;
                for op in ops {
                    any |= apply_checked(&mut editor, op, &mut ids, whole_clusters)
                        .is_ok_and(|c| !c.is_empty());
                    assert!(
                        editor.undo().is_err(),
                        "undo inside a group must be refused"
                    );
                }
                editor.end_group();
                if any {
                    past.push(before);
                    future.clear();
                }
            }
            Act::Undo => match (editor.undo().unwrap(), past.pop()) {
                (Some(changes), Some(expected)) => {
                    assert_eq!(*editor.document(), expected, "undo");
                    check_report_loose(&before, editor.document(), &changes);
                    future.push(before);
                }
                (None, None) => {}
                (got, _) => panic!("undo gave {got:?}, the model disagrees"),
            },
            Act::Redo => match (editor.redo().unwrap(), future.pop()) {
                (Some(changes), Some(expected)) => {
                    assert_eq!(*editor.document(), expected, "redo");
                    check_report_loose(&before, editor.document(), &changes);
                    past.push(before);
                }
                (None, None) => {}
                (got, _) => panic!("redo gave {got:?}, the model disagrees"),
            },
        }
        assert_eq!(
            (editor.undo_len(), editor.redo_len()),
            (past.len(), future.len())
        );
        check_document(editor.document(), whole_clusters);
    }
    while editor.undo().unwrap().is_some() {}
    assert_eq!(*editor.document(), original);
    assert_eq!(saved(editor.document()), saved(&original));
}

/// As [`check_report`], but for a group of commands, whose report may name a cell that
/// was changed and changed back: complete, not necessarily tight.
pub fn check_report_loose(before: &Document, after: &Document, changes: &Changes) {
    let ids = |doc: &Document| doc.cells_in_order().into_iter().collect::<BTreeSet<_>>();
    let (old, new) = (ids(before), ids(after));
    assert_eq!(
        changes.created,
        new.difference(&old).copied().collect::<Vec<_>>()
    );
    assert_eq!(
        changes.removed,
        old.difference(&new).copied().collect::<Vec<_>>()
    );
    for id in old.intersection(&new) {
        let (a, b) = (before.cell(*id).unwrap(), after.cell(*id).unwrap());
        assert!(
            !content_differs(a, b) || changes.changed.contains(id),
            "{id} changed, not reported"
        );
        assert!(
            !shape_differs(a, b) || changes.grids.contains(id),
            "grid of {id} changed, not reported"
        );
    }
    for name in before.tags().keys().chain(after.tags().keys()) {
        let same = before.tags().get(name) == after.tags().get(name);
        assert!(
            same || changes.tags.contains(name),
            "tag {name} changed, not reported"
        );
    }
}

/// An unused ID, for tests that need a cell that does not exist.
pub fn no_such_cell() -> CellId {
    CellId(Id::from_u128(0xdead))
}
