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
        if *xs > 0 && *ys > 0 && doc.depth(cell).unwrap() < 3 {
            doc.add_grid(cell, *xs, *ys, &mut ids).unwrap();
            queue.extend(doc.cell(cell).unwrap().grid().unwrap().cells());
        }
    }
    doc.tags_mut().insert("tag1".into(), Tag::default());
    let mut editor = Editor::new(doc).unwrap();
    for op in &spec.decorate {
        // Only what decorates: text ranges, properties, cell styles, widths.
        if (4..10).contains(&op.kind) {
            let command = support::command(editor.document(), op);
            let _ = editor.apply(command, Some(1_600_000_000_000), &mut ids);
        }
    }
    editor.into_document()
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
/// are refused, for every kind of command.
#[test]
fn the_random_commands_reach_every_kind_both_ways() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let mut carried_out = vec![0usize; support::KINDS as usize];
    let mut refused = vec![0usize; support::KINDS as usize];
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
                Ok(_) => {}
                Err(_) => refused[op.kind as usize] += 1,
            }
        }
    }
    println!("carried out per kind: {carried_out:?}\nrefused per kind:     {refused:?}");
    for kind in 0..support::KINDS as usize {
        assert!(
            carried_out[kind] >= 20,
            "kind {kind} was carried out {} times",
            carried_out[kind]
        );
        // Setting a whole text and styling whole cells cannot fail on an existing cell.
        assert!(
            refused[kind] >= 5 || [3, 8, 16].contains(&kind),
            "kind {kind} was refused {} times",
            refused[kind]
        );
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
