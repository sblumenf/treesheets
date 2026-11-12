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

/// The command that each kind of [`Op`] becomes.
pub const NAMES: [&str; KINDS as usize] = [
    "InsertText",
    "DeleteText",
    "ReplaceText",
    "SetText",
    "SetRangeStyle",
    "SetRangeColor",
    "SetLink",
    "SetProps",
    "SetStyle",
    "SetColumnWidth",
    "CreateGrid",
    "RemoveGrid",
    "InsertRows",
    "InsertColumns",
    "DeleteRows",
    "DeleteColumns",
    "ClearCells",
    "DeleteCells",
    "MoveRows",
    "MoveColumns",
    "SwapCells",
    "MoveCell",
    "CopyCell",
    "SetTag",
    "RemoveTag",
    "SortRows",
    "Transpose",
    "HierarchySwap",
    "Hierarchify",
    "Flatten",
    "Wrap",
    "Collapse",
    "SplitCell",
    "TagCells",
    "UntagCells",
    "AdjustRelSize",
    "AdjustColumnWidths",
    "ToggleCheck",
];

pub fn op() -> impl Strategy<Value = Op> {
    (
        0..KINDS,
        any::<[u32; 5]>(),
        proptest::collection::vec(0..CLUSTERS.len(), 0..4),
        proptest::bool::weighted(0.15),
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
    let text_cell = if n4 % 7 == 0 {
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
    // Deleting, styling, colouring and linking are about a range that is not empty: the
    // end moves on to the next character where there is one.
    // (Or the start moves back, at the end of the text.)
    let wants_range = end == start && !op.raw && matches!(op.kind, 1 | 4 | 5 | 6);
    let end = if wants_range {
        (start + 1..=len)
            .find(|at| floor_boundary(text, *at) == *at)
            .unwrap_or(end)
    } else {
        end
    };
    let start = if wants_range && end == start && start > 0 {
        floor_boundary(text, start - 1)
    } else {
        start
    };
    let root = doc.root();
    // Whether `inner` is `outer` or inside it.
    let within = |inner: CellId, outer: CellId| {
        let mut at = Some(inner);
        while let Some(id) = at {
            if id == outer {
                return true;
            }
            at = cell_of(&id).parent();
        }
        false
    };
    let filled = |c: &CellId| {
        let c = cell_of(c);
        !c.text.is_empty() || c.image.is_some() || c.grid().is_some()
    };
    // The cells of the block `xs` by `ys` at `x`, `y` of a grid.
    let block_of = |g: &wombat_core::Grid, x: u32, y: u32, xs: u32, ys: u32| -> Vec<CellId> {
        (y..y + ys)
            .flat_map(|cy| (x..x + xs).filter_map(move |cx| g.at(cx, cy)))
            .collect()
    };
    let color = Color::rgb(op.color.0, op.color.1, op.color.2);
    // Moving lines needs two of them, wrapping and collapsing are mostly about several
    // cells: for those, a grid that has them, if there is one.
    let sized = |fits: &dyn Fn(u32, u32) -> bool| -> Vec<CellId> {
        let fits = |c: &&CellId| cell_of(c).grid().is_some_and(|g| fits(g.xs(), g.ys()));
        with_grid.iter().filter(fits).copied().collect()
    };
    let roomy = match op.kind {
        // Not the root's grid, which cannot be removed (nor all its rows or columns).
        11 | 14 | 15 => with_grid.iter().copied().filter(|c| *c != root).collect(),
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
    let sloppy = u32::from(n4 % 8 == 0);
    let several = || {
        let count = 1 + n4 % 3;
        (0..count)
            .map(|i| pick(&all, n0.wrapping_add(i * 7)).unwrap())
            .collect::<Vec<_>>()
    };
    let below_root: Vec<CellId> = all.iter().copied().filter(|c| *c != root).collect();
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
            // Mostly at least one bit on, and none switched on and off at once.
            set: style(n3 | (1 << (n3 % 5))),
            clear: style(if n4 % 4 == 0 {
                n4
            } else {
                n4 & !(n3 | (1 << (n3 % 5)))
            }),
        },
        5 => Command::SetRangeColor {
            cell: text_cell,
            start,
            len: end - start,
            color: (n3 % 6 != 0).then_some(color),
        },
        6 => Command::SetLink {
            cell: text_cell,
            start,
            len: end - start,
            target: match n3 % 7 {
                0 => None,
                1 | 4 => Some(LinkTarget::External(format!(
                    "https://example.org/{}",
                    n4 % 3
                ))),
                2 | 5 => Some(LinkTarget::Cell {
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
            // Now and then a second property, or the first one set and set back to what
            // the cell has (which must come out as no change at all).
            let has = cell_of(&cells[0]);
            let back = match &prop {
                Prop::CellColor(_) => Some(Prop::CellColor(has.cell_color)),
                Prop::TextColor(_) => Some(Prop::TextColor(has.text_color)),
                Prop::RelSize(_) => Some(Prop::RelSize(has.rel_size)),
                Prop::TextAlign(_) => Some(Prop::TextAlign(has.text_align)),
                Prop::Note(_) => Some(Prop::Note(has.note.clone())),
                Prop::Kind(_) => Some(Prop::Kind(has.kind)),
                _ => None,
            };
            let props = match (n3 % 8, back) {
                (0, Some(back)) => vec![prop, back],
                (1, _) => vec![prop, Prop::RelSize(n3 as i32 / 8 % 5 - 2)],
                (2, _) => vec![Prop::LastEdit(Some(i64::from(n3))), prop],
                _ => vec![prop],
            };
            Command::SetProps { cells, props }
        }
        8 => Command::SetStyle {
            cells: several(),
            set: style(n1),
            clear: style(n2),
        },
        9 => Command::SetColumnWidth {
            cell: grid_cell,
            column: n1 % (xs + sloppy),
            width: n2 % 200,
        },
        10 => Command::CreateGrid {
            cell: if n4 % 9 == 0 {
                cell
            } else {
                pick(&leaves, n0).unwrap_or(cell)
            },
            xs: (1 - sloppy) + n1 % 3,
            ys: (1 - sloppy) + n2 % 3,
        },
        11 => Command::RemoveGrid { cell: grid_cell },
        12 => Command::InsertRows {
            cell: grid_cell,
            at: n1 % (ys + 1 + sloppy),
            count: u32::from(n4 % 10 != 0) + n2 % 2,
        },
        13 => Command::InsertColumns {
            cell: grid_cell,
            at: n1 % (xs + 1 + sloppy),
            count: u32::from(n4 % 10 != 0) + n2 % 2,
        },
        14 | 15 => {
            // Lines that exist, unless sloppy; all of them takes the grid away.
            let along = if op.kind == 14 { ys } else { xs };
            let at = n1 % (along + sloppy);
            let count = 1 + n2 % (along - at.min(along - 1)).min(2) + sloppy;
            if op.kind == 14 {
                Command::DeleteRows {
                    cell: grid_cell,
                    at,
                    count,
                }
            } else {
                Command::DeleteColumns {
                    cell: grid_cell,
                    at,
                    count,
                }
            }
        }
        16 => {
            // Mostly cells that have something to lose, and not the root, which is refused.
            let full: Vec<CellId> = below_root.iter().copied().filter(filled).collect();
            let mut cells: Vec<CellId> = (0..1 + n4 % 3)
                .filter_map(|i| pick(&full, n0.wrapping_add(i * 7)))
                .collect();
            if n4 % 9 == 0 || cells.is_empty() {
                cells = several();
            }
            Command::ClearCells { cells }
        }
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
        20 => {
            // Mostly two cells that can trade places: not the root, neither in the other.
            let a = pick(&below_root, n0).unwrap_or(cell);
            let apart: Vec<CellId> = below_root
                .iter()
                .copied()
                .filter(|b| !within(*b, a) && !within(a, *b))
                .collect();
            if n4 % 9 == 0 {
                Command::SwapCells {
                    a: cell,
                    b: pick(&all, n1).unwrap(),
                }
            } else {
                Command::SwapCells {
                    a,
                    b: pick(&apart, n1).unwrap_or(a),
                }
            }
        }
        21 => {
            // Mostly a cell with something in it, to an empty cell that is not inside it.
            let full: Vec<CellId> = below_root.iter().copied().filter(filled).collect();
            let from = pick(&full, n0).unwrap_or(cell);
            let empty: Vec<CellId> = below_root
                .iter()
                .copied()
                .filter(|c| !filled(c) && !within(*c, from))
                .collect();
            Command::MoveCell {
                from,
                to: if n2 % 9 == 0 {
                    pick(&all, n1).unwrap()
                } else {
                    pick(&empty, n1).unwrap_or(from)
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
            // Not often onto the root, which is refused.
            to: if n4 % 9 == 0 {
                pick(&all, n1).unwrap()
            } else {
                pick(&below_root, n1).unwrap_or(cell)
            },
        },
        23 => Command::SetTag {
            // Now and then the text of a cell, so that cells have tags.
            name: match n1 % 6 {
                0 => String::new(),
                2.. => pick(&wordy, n2).map_or("tag1".into(), |c| cell_of(&c).text.clone()),
                n => format!("tag{n}"),
            },
            cell_color: color,
            text_color: Color::rgb(op.color.2, op.color.1, op.color.0),
        },
        24 => {
            // Mostly a tag that the document has.
            let names: Vec<&String> = doc.tags().keys().collect();
            Command::RemoveTag {
                name: match pick(&names, n1) {
                    Some(name) if n4 % 3 != 0 => name.clone(),
                    _ => format!("tag{}", n1 % 4),
                },
            }
        }
        25 => {
            // Mostly a sort that has something to do: a grid and a column in which two
            // rows next to each other differ, in the direction that turns those two round.
            let lower = |c: CellId| cell_of(&c).text.to_lowercase();
            let mut unsorted: Vec<(CellId, u32, bool)> = Vec::new();
            for owner in &with_grid {
                let grid = cell_of(owner).grid().unwrap();
                for column in 0..grid.xs() {
                    let key = |row: u32| grid.at(column, row).map(lower);
                    if let Some(row) = (1..grid.ys()).find(|row| key(row - 1) != key(*row)) {
                        unsorted.push((*owner, column, key(row - 1) < key(row)));
                    }
                }
            }
            match pick(&unsorted, n1) {
                Some((owner, column, descending)) if n4 % 7 != 0 => Command::SortRows {
                    cell: owner,
                    column,
                    row: 0,
                    count: cell_of(&owner).grid().unwrap().ys(),
                    descending,
                },
                _ => {
                    let row = if n3 % 4 == 0 { n2 % ys } else { 0 };
                    Command::SortRows {
                        cell: grid_cell,
                        column: n1 % (xs + sloppy),
                        row,
                        count: ys - row + sloppy,
                        descending: n2 % 2 == 0,
                    }
                }
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
                cell: if n4 % 12 == 0 {
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
            // Mostly a table with something to do: several columns, or rows that start
            // with the same text.
            let wide: Vec<CellId> = tables
                .iter()
                .copied()
                .filter(|c| {
                    let grid = cell_of(c).grid().unwrap();
                    let firsts: BTreeSet<&str> = (0..grid.ys())
                        .filter_map(|y| grid.at(0, y))
                        .map(|k| cell_of(&k).text.as_str())
                        .collect();
                    grid.xs() > 1 || firsts.len() < grid.ys() as usize
                })
                .collect();
            Command::Hierarchify {
                cell: if n4 % 9 == 0 {
                    cell
                } else {
                    pick(&wide, n1).or(pick(&tables, n1)).unwrap_or(cell)
                },
            }
        }
        29 => {
            // Mostly a grid with grids inside it; a table is flat already.
            let nested: Vec<CellId> = with_grid
                .iter()
                .copied()
                .filter(|c| {
                    let grid = cell_of(c).grid().unwrap();
                    grid.cells().iter().any(|k| cell_of(k).grid().is_some())
                })
                .collect();
            Command::Flatten {
                // A flattened hierarchy can be many times its size: only while small.
                cell: if all.len() > 300 || n4 % 9 == 0 {
                    cell
                } else {
                    pick(&nested, n1).unwrap_or(grid_cell)
                },
            }
        }
        30 | 31 => {
            let (mut x, mut y) = (n1 % (xs + sloppy), n2 % (ys + sloppy));
            let (mut bxs, mut bys) = (
                (1 - sloppy) + n3 % (xs - n1 % xs + sloppy),
                (1 - sloppy) + n4 / 8 % (ys - n2 % ys + sloppy),
            );
            // Collapsing takes two cells or more with something in them: mostly up to the
            // end of the grid, and from its start if that is not enough.
            if op.kind == 31 && sloppy == 0 && n4 % 9 != 0 {
                (bxs, bys) = (xs - x, ys - y);
                let poor = |x, y, bxs, bys| {
                    let grid = cell_of(&grid_cell).grid();
                    let block = grid
                        .map(|g| block_of(g, x, y, bxs, bys))
                        .unwrap_or_default();
                    block.len() < 2 || !block.iter().any(filled)
                };
                if poor(x, y, bxs, bys) {
                    (x, y, bxs, bys) = (0, 0, xs, ys);
                }
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
            if n4 % 9 != 0 && !tagged.is_empty() {
                cells.retain(|c| tagged.contains(c));
            }
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
/// made of whole characters from [`CLUSTERS`], or of the characters of the error values
/// that a formula cell can show (`#NAME?` and the like, all capitals and signs, none of
/// them part of a longer character): flatten, collapse and hierarchy swap put what a
/// formula cell shows into the text of a data cell (decision A83).
pub fn check_document(doc: &Document, whole_clusters: bool) {
    doc.validate().unwrap();
    if whole_clusters {
        for id in doc.cells_in_order() {
            let text = &doc.cell(id).unwrap().text;
            for cluster in text.graphemes(true) {
                assert!(
                    CLUSTERS.contains(&cluster)
                        || (cluster.len() == 1 && "#NAME?VLU!DI/0ROCYT".contains(cluster)),
                    "{text:?} has the piece {cluster:?}"
                );
            }
        }
    }
}

/// Checks the editor's bound on the size of the saved file: it is what a fresh count of
/// the whole document gives (the running sum has not drifted), and the file is not longer.
/// And the other things the editor counts as it goes (the cell IDs that its histories
/// hold, the memory of the undo history) against a fresh count of those.
pub fn check_bound(editor: &Editor, what: &str) {
    if let Err(wrong) = editor.audit() {
        panic!("{wrong}: {what}");
    }
    let doc = editor.document();
    let fresh = wombat_core::edit::size_bound(doc);
    assert_eq!(editor.size_bound(), fresh, "the size bound drifted: {what}");
    let bytes = format::to_string(doc).unwrap().len() as u64;
    assert!(bytes <= fresh, "{bytes} bytes saved, bound {fresh}: {what}");
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
            let after = editor.document();
            check_report(&before, after, changes, &what);
            check_document(after, whole_clusters);
            // A command that leaves the document as it was reports nothing and leaves no
            // unit in the history, whatever it did on the way (decision A44).
            if changes.is_empty() || *after == before {
                assert!(changes.is_empty(), "nothing differs, but reported: {what}");
                assert_eq!((editor.undo_len(), editor.redo_len()), history, "{what}");
            }
            // No command takes the root's grid away (decision B33).
            let rooted = |doc: &Document| doc.cell(doc.root()).unwrap().grid().is_some();
            assert!(
                !rooted(&before) || rooted(after),
                "the root lost its grid: {what}"
            );
            check_bound(editor, &what);
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
        check_bound(&editor, "undo");
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
        check_bound(&editor, "redo");
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
    /// A text edit that joins the unit before it if that is typing in the same cell.
    Type(Op),
    /// Several commands as one undo unit, the second and third of them in a group inside
    /// the group.
    Group(Vec<Op>),
    /// The same, but the group is cancelled and not ended: nothing of it may be left.
    Cancelled(Vec<Op>),
    Undo,
    Redo,
    /// Bounds the history to so many units.
    Limit(usize),
}

pub fn act() -> impl Strategy<Value = Act> {
    prop_oneof![
        10 => op().prop_map(Act::Apply),
        4 => op().prop_map(Act::Type),
        3 => proptest::collection::vec(op(), 0..5).prop_map(Act::Group),
        3 => proptest::collection::vec(op(), 0..5).prop_map(Act::Cancelled),
        6 => Just(Act::Undo),
        4 => Just(Act::Redo),
        1 => (0..6usize).prop_map(Act::Limit),
    ]
}

/// Every formula cell with what it shows.
fn results(editor: &Editor) -> Vec<(CellId, String)> {
    let results = editor.calc().results();
    results
        .into_iter()
        .map(|(id, text)| (id, text.to_string()))
        .collect()
}

/// The cell whose text a command edits, if it is one of the three typing commands.
fn typed_cell(command: &Command) -> Option<CellId> {
    match command {
        Command::InsertText { cell, .. }
        | Command::DeleteText { cell, .. }
        | Command::ReplaceText { cell, .. } => Some(*cell),
        _ => None,
    }
}

/// Commands, typing, groups (also nested, ended or cancelled), undo, redo and history
/// limits in any order, against a model that keeps a copy of the document for every unit
/// of the history: each undo and redo must land on the copy, a group reports what it
/// changed as a whole, and a cancelled group leaves the document, both histories and
/// everything the editor counts as they were.
pub fn run_walk(original: Document, acts: &[Act], whole_clusters: bool) {
    let mut ids = SeededIds::from_u64(77);
    let mut editor = Editor::new(original.clone()).unwrap();
    // Per unit: the document before it, and the cell if the unit is typing in one cell.
    type Units = Vec<(Document, Option<CellId>)>;
    let (mut past, mut future): (Units, Units) = (Vec::new(), Vec::new());
    let mut limit = usize::MAX;
    for act in acts {
        let before = editor.document().clone();
        match act {
            Act::Apply(op) => {
                let typed = typed_cell(&command(&before, op));
                if apply_checked(&mut editor, op, &mut ids, whole_clusters)
                    .is_ok_and(|c| !c.is_empty())
                {
                    past.push((before, typed));
                    future.clear();
                }
            }
            Act::Type(op) => {
                // The same checks as for `apply`, on one of the three text commands.
                let op = Op {
                    kind: op.kind % 3,
                    ..op.clone()
                };
                let command = command(&before, &op);
                let typed = typed_cell(&command);
                let now = Some(1_700_000_000_000 + i64::from(op.n[4] % 1000));
                match editor.apply_typing(command.clone(), now, &mut ids) {
                    Ok(changes) => {
                        let what = format!("typing {command:?}");
                        check_report(&before, editor.document(), &changes, &what);
                        check_bound(&editor, &what);
                        if !changes.is_empty() {
                            let joins = past.last().is_some_and(|(_, cell)| *cell == typed);
                            if !joins {
                                past.push((before, typed));
                            }
                            future.clear();
                        }
                    }
                    Err(_) => assert_eq!(*editor.document(), before, "refused typing"),
                }
            }
            Act::Group(ops) | Act::Cancelled(ops) => {
                let counted = (editor.history_bytes(), editor.size_bound());
                let shown = results(&editor);
                editor.begin_group();
                let mut any = false;
                for (n, op) in ops.iter().enumerate() {
                    if n == 1 {
                        editor.begin_group();
                    }
                    any |= apply_checked(&mut editor, op, &mut ids, whole_clusters)
                        .is_ok_and(|c| !c.is_empty());
                    assert!(
                        editor.undo().is_err() && editor.redo().is_err(),
                        "undo and redo inside a group must be refused"
                    );
                    // The histories wait for the end of the group, what could be redone
                    // too.
                    assert_eq!(
                        (editor.undo_len(), editor.redo_len()),
                        (past.len(), future.len()),
                        "inside a group"
                    );
                    if n == 2 || (n == 1 && ops.len() == 2) {
                        assert!(editor.end_group().is_empty(), "an inner group reports");
                    }
                }
                if matches!(act, Act::Cancelled(_)) {
                    // Also with the inner group still open (four commands or more).
                    let inside = editor.document().clone();
                    let changes = editor.cancel_group().unwrap();
                    check_report_loose(&inside, editor.document(), &changes);
                    assert_eq!(changes.is_empty(), !any, "the report of a cancel");
                    assert_eq!(*editor.document(), before, "cancel");
                    assert!(!editor.in_group());
                    assert_eq!((editor.history_bytes(), editor.size_bound()), counted);
                    assert_eq!(results(&editor), shown, "results after a cancel");
                    check_bound(&editor, "cancel");
                    // Nothing joins the model: `past` and `future` are as they were.
                } else {
                    let changes = editor.end_group();
                    check_report_loose(&before, editor.document(), &changes);
                    if any {
                        past.push((before, None));
                        future.clear();
                    } else {
                        assert!(changes.is_empty(), "a group that did nothing reports");
                    }
                    check_bound(&editor, "end of a group");
                }
            }
            Act::Undo => match (editor.undo().unwrap(), past.pop()) {
                (Some(changes), Some((expected, typed))) => {
                    assert_eq!(*editor.document(), expected, "undo");
                    check_report_loose(&before, editor.document(), &changes);
                    check_bound(&editor, "undo");
                    future.push((before, typed));
                }
                (None, None) => {}
                (got, _) => panic!("undo gave {got:?}, the model disagrees"),
            },
            Act::Redo => match (editor.redo().unwrap(), future.pop()) {
                (Some(changes), Some((expected, typed))) => {
                    assert_eq!(*editor.document(), expected, "redo");
                    check_report_loose(&before, editor.document(), &changes);
                    check_bound(&editor, "redo");
                    past.push((before, typed));
                }
                (None, None) => {}
                (got, _) => panic!("redo gave {got:?}, the model disagrees"),
            },
            Act::Limit(units) => {
                limit = *units;
                editor.set_history_limit(Some(limit));
                check_bound(&editor, "a limit on the history");
            }
        }
        // The oldest units are forgotten; what they did stays.
        let over = past.len().saturating_sub(limit);
        past.drain(..over);
        assert_eq!(
            (editor.undo_len(), editor.redo_len()),
            (past.len(), future.len())
        );
        check_document(editor.document(), whole_clusters);
    }
    // Undoing all there is leads to the document before the oldest unit that was kept.
    let oldest = past
        .first()
        .map_or(editor.document().clone(), |(doc, _)| doc.clone());
    while editor.undo().unwrap().is_some() {}
    assert_eq!(*editor.document(), oldest);
    if limit == usize::MAX {
        assert_eq!(oldest, original);
    }
    assert_eq!(saved(editor.document()), saved(&oldest));
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
