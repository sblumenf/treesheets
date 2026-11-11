//! Random sequences of commands on random documents (the same tests run on imported
//! TreeSheets documents in `wombat-cts/tests/commands.rs`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use proptest::prelude::*;
use support::{act, op, run_sequence, run_walk, text_of, Op, CLUSTERS};
use wombat_core::construct::Construct;
use wombat_core::{CellId, Command, Document, Editor, SeededIds, Tag};

/// The shape and texts of a random document: per cell its text and the size of its grid.
#[derive(Clone, Debug)]
struct Spec {
    seed: u64,
    cells: Vec<(Vec<usize>, u32, u32)>,
    decorate: Vec<Op>,
}

fn spec() -> impl Strategy<Value = Spec> {
    let cell = (
        proptest::collection::vec(0..CLUSTERS.len(), 0..5),
        0..4u32,
        0..4u32,
    );
    (
        any::<u64>(),
        proptest::collection::vec(cell, 1..20),
        proptest::collection::vec(op(), 0..12),
    )
        .prop_map(|(seed, cells, decorate)| Spec {
            seed,
            cells,
            decorate,
        })
}

/// Builds the document: the specs fill cells in document order, giving every third cell or
/// so a grid, up to three levels; then styles, colours, links and properties are put on
/// with commands, so that they are valid whatever the texts are.
fn document(spec: &Spec) -> Document {
    let mut ids = SeededIds::from_u64(spec.seed);
    let mut doc = Document::new(&mut ids);
    let mut queue: Vec<CellId> = vec![doc.root()];
    for (next, (clusters, xs, ys)) in spec.cells.iter().enumerate() {
        let Some(cell) = queue.get(next).copied() else {
            break;
        };
        doc.cell_mut(cell).unwrap().text = text_of(clusters);
        // The root nearly always has a grid with several cells, half the time a single
        // column: most commands need a grid to work on, and many need rows.
        let (xs, ys) = match (next, spec.seed % 8) {
            (0, 0) => (*xs, *ys),
            (0, 1..=4) => (1, (*ys).max(2)),
            (0, _) => ((*xs).max(2), (*ys).max(2)),
            _ => (*xs, *ys),
        };
        // A good part of the grids are single columns, as hierarchies are.
        let xs = if (spec.seed as usize).wrapping_add(next) % 3 == 0 {
            xs.min(1)
        } else {
            xs
        };
        if xs > 0 && ys > 0 && doc.depth(cell).unwrap() < 3 {
            doc.add_grid(cell, xs, ys, &mut ids).unwrap();
            queue.extend(doc.cell(cell).unwrap().grid().unwrap().cells());
        }
    }
    // The cells that the specs did not reach get short texts, many of them equal, as
    // sorting, hierarchify and hierarchy swap have little to do on empty cells.
    for (n, cell) in queue.iter().enumerate().skip(spec.cells.len()) {
        // One of them a letter with an accent of its own, which no command may cut.
        let pick = [Some(0), Some(1), Some(4), Some(1), None, None];
        if let Some(cluster) = pick[(spec.seed as usize).wrapping_add(n * 7) % pick.len()] {
            doc.cell_mut(*cell).unwrap().text = CLUSTERS[cluster].to_string();
        }
    }
    // Most documents also get what the restructuring commands work on, in the first cell
    // below the root that is still empty-handed: a small hierarchy of single columns with
    // texts that repeat, and in it a table whose rows start alike.
    let spare: Vec<CellId> = doc
        .cells_in_order()
        .into_iter()
        .filter(|c| *c != doc.root() && doc.cell(*c).unwrap().grid().is_none())
        .collect();
    let mut fill = |doc: &mut Document, cell: CellId, xs: u32, clusters: &[usize]| {
        doc.add_grid(cell, xs, clusters.len() as u32 / xs, &mut ids)
            .unwrap();
        let cells = doc.cell(cell).unwrap().grid().unwrap().cells().to_vec();
        for (id, cluster) in cells.iter().zip(clusters) {
            doc.cell_mut(*id).unwrap().text = CLUSTERS[*cluster].to_string();
        }
        cells
    };
    if spec.seed % 8 != 7 {
        if let Some(top) = spare.first() {
            let kids = fill(&mut doc, *top, 1, &[0, 1, 0]);
            let inner = fill(&mut doc, kids[0], 1, &[1, 3]);
            fill(&mut doc, inner[1], 1, &[0, 1]);
            fill(&mut doc, kids[1], 3, &[0, 1, 5, 0, 3, 1, 1, 1, 2]);
        }
    }
    doc.tags_mut().insert("tag1".into(), Tag::default());
    // And a tag that many cells have.
    doc.tags_mut().insert(CLUSTERS[1].into(), Tag::default());
    let mut editor = Editor::new(doc).unwrap();
    for op in &spec.decorate {
        // Only what decorates: text ranges, properties, cell styles, widths.
        if (4..10).contains(&op.kind) {
            let command = support::command(editor.document(), op);
            let _ = editor.apply(command, Some(1_600_000_000_000), &mut ids);
        }
    }
    let mut doc = editor.into_document();
    // One document in three has fields that this version does not know, at every level:
    // they must survive whatever leaves their owner alone, and saving must count them.
    if spec.seed.is_multiple_of(3) {
        let unknown = || serde_json::json!({"list": [1, "two\n", null], "deep": {"x": 1.5}});
        doc.extra_mut().insert("x_document".into(), unknown());
        for tag in doc.tags_mut().values_mut() {
            tag.extra.insert("x_tag".into(), unknown());
        }
        for (n, id) in doc.cells_in_order().into_iter().enumerate() {
            if n % 4 != 0 {
                continue;
            }
            let cell = doc.cell_mut(id).unwrap();
            cell.extra.insert("x_cell".into(), unknown());
            for run in &mut cell.runs {
                run.extra.insert("x_run".into(), unknown());
            }
            for link in &mut cell.links {
                link.extra.insert("x_link".into(), unknown());
            }
            if let Some(image) = &mut cell.image {
                image.extra.insert("x_image".into(), unknown());
            }
            if let Some(grid) = cell.grid_mut() {
                grid.extra.insert("x_grid".into(), unknown());
            }
        }
    }
    doc
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// (a) valid after every command; (b) undoing everything gives the original document
    /// and bytes; (c) redoing everything gives the final document, bytes and IDs; (e) a
    /// refused command changes nothing; (f) every report is complete and tight; (g) texts
    /// stay made of whole characters, runs and links stay well-formed; (h) every state
    /// saves and loads back equal.
    #[test]
    fn any_sequence_of_commands_can_be_undone_and_redone_exactly(
        spec in spec(),
        ops in proptest::collection::vec(op(), 0..60),
    ) {
        run_sequence(document(&spec), &ops, true);
    }

    /// (d) commands, groups, undo and redo in any order.
    #[test]
    fn undo_and_redo_in_any_order_follow_the_history(
        spec in spec(),
        acts in proptest::collection::vec(act(), 0..60),
    ) {
        run_walk(document(&spec), &acts, true);
    }
}

/// The generator is only worth something if most of its commands are carried out and some
/// are refused, for every kind of command. Prints how often each kind was carried out, did
/// nothing or was refused, and fails if a kind falls below what it reached when this was
/// last tuned, so that a change to the generator or to a command that makes one rare shows.
#[test]
fn the_random_commands_reach_every_kind_both_ways() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    use std::collections::BTreeMap;
    let mut runner = TestRunner::deterministic();
    let kinds = support::KINDS as usize;
    let (mut carried_out, mut refused, mut idle) = (
        vec![0usize; kinds],
        vec![0usize; kinds],
        vec![0usize; kinds],
    );
    let mut reasons: Vec<BTreeMap<String, usize>> = vec![BTreeMap::new(); kinds];
    for _ in 0..300 {
        let spec = spec().new_tree(&mut runner).unwrap().current();
        let ops = proptest::collection::vec(op(), 30)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        let mut editor = Editor::new(document(&spec)).unwrap();
        let mut ids = SeededIds::from_u64(5);
        for op in &ops {
            match support::apply_checked(&mut editor, op, &mut ids, true) {
                Ok(changes) if !changes.is_empty() => carried_out[op.kind as usize] += 1,
                Ok(_) => idle[op.kind as usize] += 1,
                Err(error) => {
                    refused[op.kind as usize] += 1;
                    // The reason without the IDs and numbers in it.
                    let reason: String = error
                        .to_string()
                        .split(' ')
                        .filter(|word| !word.contains(|c: char| c.is_ascii_digit()))
                        .collect::<Vec<_>>()
                        .join(" ");
                    *reasons[op.kind as usize].entry(reason).or_default() += 1;
                }
            }
        }
    }
    println!(
        "{:<20}{:>8}{:>8}{:>8}{:>6}",
        "command", "done", "nothing", "refused", "done"
    );
    let mut low = Vec::new();
    for kind in 0..kinds {
        let tries = carried_out[kind] + idle[kind] + refused[kind];
        let rate = carried_out[kind] * 100 / tries.max(1);
        println!(
            "{:<20}{:>8}{:>8}{:>8}{:>5}%",
            support::NAMES[kind],
            carried_out[kind],
            idle[kind],
            refused[kind],
            rate
        );
        if rate < MIN_RATE {
            low.push(format!(
                "{} {rate}%: {:?}",
                support::NAMES[kind],
                reasons[kind]
            ));
        }
        // Setting a whole text, styling whole cells, transposing, tagging, resizing and
        // ticking cannot fail on an existing cell.
        if refused[kind] < 5 && ![3, 8, 26, 33, 34, 35, 37].contains(&kind) {
            low.push(format!(
                "{} was refused {} times",
                support::NAMES[kind],
                refused[kind]
            ));
        }
    }
    assert!(
        low.is_empty(),
        "carried out too rarely:\n{}",
        low.join("\n")
    );
}

/// Every kind of command is carried out in at least this share of its tries, in per cent.
const MIN_RATE: usize = 60;

/// Whatever commands are accepted near the size limit, the document saves within it and
/// loads back equal; a refusal for size changes nothing; undo and redo are never refused.
/// The limit is set just above the document, in three ways: above the editor's bound (the
/// ordinary case), between the saved size and the bound (a document loaded from a file
/// close to the limit: the bound cannot tell, so the editor writes it out), and below the
/// saved size (a document that was built too large: it can still be made smaller).
#[test]
fn what_commands_build_near_the_size_limit_can_be_saved() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    use wombat_core::{format, Error};
    let mut runner = TestRunner::deterministic();
    let (mut accepted, mut too_large) = ([0usize; 3], [0usize; 3]);
    for round in 0..240 {
        let spec = spec().new_tree(&mut runner).unwrap().current();
        let ops = proptest::collection::vec(op(), 40)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        let original = document(&spec);
        let size = |doc: &Document| format::to_string(doc).unwrap().len() as u64;
        let mut editor = Editor::new(original.clone()).unwrap();
        let (mode, room) = (round % 3, spec.seed % 600);
        let limit = match mode {
            0 => editor.size_bound() + room,
            // With hardly any room, so that a cell moving one level down is too much.
            1 => size(&original) + (room % 16).min(editor.size_bound() - size(&original) - 1),
            _ => size(&original).saturating_sub(1 + room),
        };
        editor.set_size_limit(limit);
        let mut ids = SeededIds::from_u64(11);
        let mut fits = mode < 2;
        let mut states = vec![original.clone()];
        for op in &ops {
            let bound = editor.size_bound();
            match support::apply_checked(&mut editor, op, &mut ids, true) {
                Ok(changes) if changes.is_empty() => {}
                Ok(_) => {
                    accepted[mode] += 1;
                    // From the moment the bound is under the limit the document fits, and
                    // one that fitted goes on fitting.
                    fits |= editor.size_bound() <= limit;
                    let now = size(editor.document());
                    assert!(!fits || now <= limit, "{now} bytes saved, limit {limit}");
                    assert!(
                        editor.size_bound() <= limit.max(bound),
                        "the bound grew over the limit"
                    );
                    states.push(editor.document().clone());
                }
                Err(Error::Limit(reason)) if reason.contains("too large to save") => {
                    too_large[mode] += 1;
                }
                Err(_) => {}
            }
        }
        // Back to the start and forward again, whatever the limit.
        for expected in states.iter().rev().skip(1) {
            editor.undo().unwrap().unwrap();
            assert_eq!(editor.document(), expected);
        }
        assert_eq!(editor.undo().unwrap(), None);
        for expected in states.iter().skip(1) {
            editor.redo().unwrap().unwrap();
            assert_eq!(editor.document(), expected);
        }
        let text = format::to_string(editor.document()).unwrap();
        assert_eq!(
            format::from_slice(text.as_bytes()).unwrap(),
            *editor.document()
        );
    }
    println!("near the size limit, by mode: {accepted:?} carried out, {too_large:?} refused as too large");
    for mode in 0..3 {
        assert!(accepted[mode] > 300 && too_large[mode] > 100, "mode {mode}");
    }
}

#[test]
fn a_command_on_a_cell_that_does_not_exist_is_refused() {
    let mut ids = SeededIds::from_u64(1);
    let doc = Document::new(&mut ids);
    let before = doc.clone();
    let mut editor = Editor::new(doc).unwrap();
    let cell = support::no_such_cell();
    let commands = [
        Command::SetText {
            cell,
            text: "x".into(),
        },
        Command::CreateGrid { cell, xs: 1, ys: 1 },
        Command::RemoveGrid { cell },
        Command::InsertRows {
            cell,
            at: 0,
            count: 1,
        },
        Command::DeleteColumns {
            cell,
            at: 0,
            count: 1,
        },
        Command::ClearCells { cells: vec![cell] },
        Command::DeleteCells {
            cell,
            x: 0,
            y: 0,
            xs: 1,
            ys: 1,
        },
        Command::MoveRows {
            cell,
            from: 0,
            count: 1,
            to: 0,
        },
        Command::SwapCells {
            a: cell,
            b: before.root(),
        },
        Command::MoveCell {
            from: before.root(),
            to: cell,
        },
        Command::CopyCell {
            from: cell,
            to: before.root(),
        },
        Command::SetColumnWidth {
            cell,
            column: 0,
            width: 10,
        },
    ];
    for command in commands {
        assert!(
            editor.apply(command.clone(), None, &mut ids).is_err(),
            "{command:?}"
        );
        assert_eq!(*editor.document(), before);
    }
    assert_eq!(editor.undo_len(), 0);
}

/// The cell of the document that the number picks among those with a grid, if any.
fn grid_cell(doc: &Document, n: u32) -> Option<CellId> {
    let with_grid: Vec<CellId> = doc
        .cells_in_order()
        .into_iter()
        .filter(|c| doc.cell(*c).unwrap().grid().is_some())
        .collect();
    (!with_grid.is_empty()).then(|| with_grid[n as usize % with_grid.len()])
}

/// A text as sorting compares it, written out again here: lower case, a character at a
/// time, where that is one character.
fn folded(text: &str) -> Vec<char> {
    text.chars()
        .map(|c| {
            let mut lower = c.to_lowercase();
            match (lower.next(), lower.next()) {
                (Some(one), None) => one,
                _ => c,
            }
        })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    /// Transposing twice puts every cell back where it was; only column widths can
    /// differ, and not if the grid had one width for all columns.
    #[test]
    fn transposing_twice_gives_the_grid_back(spec in spec(), n in any::<u32>()) {
        let doc = document(&spec);
        let Some(cell) = grid_cell(&doc, n) else { return Ok(()); };
        let mut ids = SeededIds::from_u64(1);
        let mut editor = Editor::new(doc.clone()).unwrap();
        let before = doc.cell(cell).unwrap().grid().unwrap().clone();
        for _ in 0..2 {
            editor.apply(Command::Transpose { cells: vec![cell] }, None, &mut ids).unwrap();
        }
        let after = editor.document().cell(cell).unwrap().grid().unwrap().clone();
        prop_assert_eq!((after.xs(), after.ys(), after.cells()), (before.xs(), before.ys(), before.cells()));
        let kept = before.xs().min(before.ys()) as usize;
        prop_assert_eq!(&after.col_widths[..kept], &before.col_widths[..kept]);
        prop_assert!(after.col_widths[kept..].iter().all(|w| *w == before.col_widths[kept - 1]));
        if before.col_widths.iter().all(|w| *w == before.col_widths[0]) {
            prop_assert_eq!(editor.document(), &doc);
        }
    }

    /// Sorting reorders whole rows and nothing else; the rows come out in order; rows that
    /// compare equal keep the order they had; sorting again changes nothing.
    #[test]
    fn sorting_is_a_stable_permutation_of_rows(
        spec in spec(),
        n in any::<[u32; 4]>(),
        descending in any::<bool>(),
    ) {
        let doc = document(&spec);
        let Some(cell) = grid_cell(&doc, n[0]) else { return Ok(()); };
        let grid = doc.cell(cell).unwrap().grid().unwrap().clone();
        let (xs, ys) = (grid.xs(), grid.ys());
        let (column, row) = (n[1] % xs, n[2] % ys);
        let count = 1 + n[3] % (ys - row);
        let command = Command::SortRows { cell, column, row, count, descending };
        let mut ids = SeededIds::from_u64(1);
        let mut editor = Editor::new(doc.clone()).unwrap();
        editor.apply(command.clone(), None, &mut ids).unwrap();
        let sorted = editor.document().cell(cell).unwrap().grid().unwrap().clone();
        let rows = |g: &wombat_core::Grid| -> Vec<Vec<CellId>> {
            g.cells().chunks(xs as usize).map(<[CellId]>::to_vec).collect()
        };
        let (old, new) = (rows(&grid), rows(&sorted));
        let (from, to) = (row as usize, (row + count) as usize);
        // Outside the rows asked for nothing moved; inside, the same rows, each whole.
        prop_assert_eq!((&old[..from], &old[to..]), (&new[..from], &new[to..]));
        let mut was = old[from..to].to_vec();
        let mut is = new[from..to].to_vec();
        was.sort();
        is.sort();
        prop_assert_eq!(was, is);
        // In order, and equal rows in the order they had.
        let key = |r: &Vec<CellId>| -> Vec<Vec<char>> {
            (0..xs as usize)
                .map(|k| folded(&doc.cell(r[(k + column as usize) % xs as usize]).unwrap().text))
                .collect()
        };
        let place = |r: &Vec<CellId>| old.iter().position(|o| o == r).unwrap();
        for pair in new[from..to].windows(2) {
            let (a, b) = (key(&pair[0]), key(&pair[1]));
            if a == b {
                prop_assert!(place(&pair[0]) < place(&pair[1]), "equal rows changed order");
            } else {
                prop_assert_eq!(a < b, !descending, "rows out of order");
            }
        }
        // Every other cell of the document is untouched, and a second sort has nothing to do.
        let changes = editor.apply(command, None, &mut ids).unwrap();
        prop_assert!(changes.is_empty());
        let mut back = editor.into_document();
        back.cell_mut(cell).unwrap().grid_mut().unwrap().clone_from(&grid);
        prop_assert_eq!(back, doc);
    }

    /// Flatten keeps every cell that was inside (with its ID and text), makes a full
    /// table with one row per innermost cell, and can itself be flattened and
    /// hierarchified; wrap keeps the cells of its block, in order, inside one new cell.
    #[test]
    fn flatten_and_wrap_keep_the_cells_they_rearrange(spec in spec(), n in any::<[u32; 5]>()) {
        let doc = document(&spec);
        let Some(cell) = grid_cell(&doc, n[0]) else { return Ok(()); };
        let inside = |doc: &Document| -> Vec<CellId> {
            let all = doc.cells_in_order();
            let mut within = Vec::new();
            for id in all {
                let mut at = doc.cell(id).unwrap().parent();
                while let Some(up) = at {
                    if up == cell {
                        within.push(id);
                    }
                    at = doc.cell(up).unwrap().parent();
                }
            }
            within
        };
        let before = inside(&doc);
        let leaves = before.iter().filter(|c| doc.cell(**c).unwrap().grid().is_none()).count();
        let mut ids = SeededIds::from_u64(1);
        let mut editor = Editor::new(doc.clone()).unwrap();
        editor.apply(Command::Flatten { cell }, None, &mut ids).unwrap();
        let flat = editor.document().clone();
        let table = flat.cell(cell).unwrap().grid().unwrap();
        prop_assert_eq!(table.ys() as usize, leaves);
        for id in &before {
            let now = flat.cell(*id).unwrap();
            prop_assert_eq!(now.parent(), Some(cell));
            prop_assert_eq!(&now.text, &doc.cell(*id).unwrap().text);
        }
        // A leaf stands in the column of its level, its parents to its left in order.
        for row in table.cells().chunks(table.xs() as usize) {
            let texts: Vec<&str> = row.iter().map(|c| flat.cell(*c).unwrap().text.as_str()).collect();
            let leaf = row.iter().rposition(|c| before.contains(c) && doc.cell(*c).unwrap().grid().is_none());
            let leaf = leaf.expect("a row without its leaf");
            let mut at = row[leaf];
            for column in (0..leaf).rev() {
                at = doc.cell(at).unwrap().parent().unwrap();
                prop_assert_eq!(texts[column], doc.cell(at).unwrap().text.as_str());
            }
            prop_assert_eq!(doc.cell(at).unwrap().parent(), Some(cell));
            prop_assert!(texts[leaf + 1..].iter().all(|t| t.is_empty()));
        }
        editor.apply(Command::Hierarchify { cell }, None, &mut ids).unwrap();
        editor.document().validate().unwrap();

        // Wrap, on the document as it was.
        let mut editor = Editor::new(doc.clone()).unwrap();
        let grid = doc.cell(cell).unwrap().grid().unwrap();
        let (x, y) = (n[1] % grid.xs(), n[2] % grid.ys());
        let (xs, ys) = (1 + n[3] % (grid.xs() - x), 1 + n[4] % (grid.ys() - y));
        let mut block = Vec::new();
        for cy in y..y + ys {
            for cx in x..x + xs {
                block.push(grid.at(cx, cy).unwrap());
            }
        }
        match editor.apply(Command::Wrap { cell, x, y, xs, ys }, None, &mut ids) {
            Ok(_) => {
                let wrapped = editor.document();
                let wrapper = wrapped.child(cell, x, y).unwrap();
                prop_assert!(doc.cell(wrapper).is_none());
                let inner = wrapped.cell(wrapper).unwrap().grid().unwrap();
                prop_assert_eq!((inner.xs(), inner.ys(), inner.cells()), (xs, ys, block.as_slice()));
                for id in block {
                    let (was, now) = (doc.cell(id).unwrap(), wrapped.cell(id).unwrap());
                    prop_assert_eq!(now.parent(), Some(wrapper));
                    // The same cell in all but its place.
                    let own = |c: &wombat_core::Cell| {
                        format!("{:?}", (&c.text, &c.runs, &c.links, &c.note, c.grid(), c.cell_color))
                    };
                    prop_assert_eq!(own(now), own(was));
                }
            }
            Err(error) => prop_assert!(matches!(error, wombat_core::Error::Limit(_)), "{error}"),
        }
    }
}
