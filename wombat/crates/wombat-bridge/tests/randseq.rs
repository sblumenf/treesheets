//! Random sequences of valid commands, groups (ended and cancelled, also nested), entered
//! texts, undo and redo through an open document of the bridge, beside an editor of the
//! core that is given the same: every report, every error and the bytes of every saved
//! file must be the same. The bridge adds nothing to what the core does and loses nothing
//! of it, whatever the commands are; and it never calls a document saved that its file
//! does not hold.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

// The generator of commands that the core's own tests use, by its path: a copy would not
// learn of new commands.
#[path = "../../wombat-core/tests/support/mod.rs"]
mod support;

use std::path::Path;

use proptest::prelude::*;
use support::{command, op, Op};
use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{BridgeError, Report};
use wombat_bridge::convert::{command_out, report_out};
use wombat_core::{format, storage, Changes, Editor, Error, SeededIds};

/// What goes with a command: nothing, or one of the calls around it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum With {
    Nothing,
    /// A group is opened before the command.
    Begin,
    /// A group is ended after it.
    End,
    /// The open group is cancelled after it.
    Cancel,
    /// A text is entered into a cell after it.
    Retype,
}

fn with() -> impl Strategy<Value = With> {
    prop_oneof![
        6 => Just(With::Nothing),
        3 => Just(With::Begin),
        2 => Just(With::End),
        2 => Just(With::Cancel),
        2 => Just(With::Retype),
    ]
}

fn report(shadow: &Editor, changes: &Changes) -> Report {
    report_out(changes, shadow.recalculated())
}

/// An undo or a redo of both: the same report, the same nothing, or the same refusal.
fn same_step(
    got: Result<Option<Report>, BridgeError>,
    want: Result<Option<Changes>, Error>,
    shadow: &Editor,
    what: &str,
) {
    match (got, want) {
        (Ok(got), Ok(want)) => assert_eq!(got, want.map(|c| report(shadow, &c)), "{what}"),
        (Err(got), Err(want)) => assert_eq!(got.message, want.to_string(), "{what}"),
        (got, want) => panic!("{what}: the bridge says {got:?}, the core {want:?}"),
    }
}

fn run(steps: &[(Op, With)], undo_every: usize, seed: u64) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("randseq");
    std::fs::create_dir_all(&dir).unwrap();
    // One file for all cases, which run one after another: a case that left its
    // document open would be refused here.
    let path = dir.join("document.wombat");
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/assets/example.wombat");
    std::fs::copy(example, &path).unwrap();
    let mut bridge = OpenDocument::open_with(&path, SeededIds::from_u64(seed)).unwrap();
    let mut ids = SeededIds::from_u64(seed);
    let mut shadow = Editor::new(storage::load_document(&path).unwrap()).unwrap();
    // Whether the file holds the document as it was when the open group was opened.
    let mut held = false;
    for (i, (op, with)) in steps.iter().enumerate() {
        if *with == With::Begin {
            if !shadow.in_group() {
                held = bridge.is_saved().unwrap();
            }
            bridge.begin_group().unwrap();
            shadow.begin_group();
        }
        let core = command(shadow.document(), op);
        let now = (op.n[4] % 3 != 0).then_some(1_700_000_000_000 + i64::from(op.n[4] % 1000));
        let wire = command_out(&core).expect("a command of the generator has a serialised form");
        bridge.set_fixed_time(now);
        let got = bridge.apply(wire);
        match (got, shadow.apply(core.clone(), now, &mut ids)) {
            (Ok(got), Ok(changes)) => assert_eq!(got, report(&shadow, &changes), "{core:?}"),
            (Err(got), Err(error)) => assert_eq!(got.message, error.to_string(), "{core:?}"),
            (got, want) => panic!("the bridge says {got:?}, the core {want:?}, to {core:?}"),
        }
        match with {
            With::Nothing | With::Begin => {}
            With::End => {
                let open = shadow.in_group();
                let changes = shadow.end_group();
                // Only the end of the outermost group has something to say.
                let want = match open && !shadow.in_group() {
                    true => report(&shadow, &changes),
                    false => Report::default(),
                };
                assert_eq!(bridge.end_group().unwrap(), want, "the end of a group");
            }
            With::Cancel => {
                let open = shadow.in_group();
                let saved = bridge.is_saved().unwrap();
                let changes = shadow.cancel_group().unwrap();
                let want = match open {
                    true => report(&shadow, &changes),
                    false => Report::default(),
                };
                assert_eq!(bridge.cancel_group().unwrap(), want, "a cancelled group");
                // The document is as it was when the group was opened. If the file
                // held it then and was not written since, it holds it now.
                let want = if open && held {
                    true
                } else {
                    saved && changes.is_empty()
                };
                assert_eq!(bridge.is_saved().unwrap(), want, "saved after a cancel");
            }
            With::Retype => {
                let cells = shadow.document().cells_in_order();
                let cell = cells[op.n[2] as usize % cells.len()];
                let old = &shadow.document().cell(cell).unwrap().text;
                let keep = op.n[3] as usize % (old.chars().count() + 1);
                let text: String = old.chars().take(keep).chain("né w".chars()).collect();
                let got = bridge.retype(cell.to_string(), text.clone());
                match (got, shadow.retype(cell, &text, now, &mut ids)) {
                    (Ok(got), Ok(changes)) => assert_eq!(got, report(&shadow, &changes)),
                    (Err(got), Err(error)) => assert_eq!(got.message, error.to_string()),
                    (got, want) => panic!("retype: the bridge says {got:?}, the core {want:?}"),
                }
            }
        }
        assert_eq!(bridge.in_group().unwrap(), shadow.in_group());
        if undo_every > 0 && i % undo_every == undo_every - 1 {
            // Inside a group both are refused, by both.
            same_step(bridge.undo(), shadow.undo(), &shadow, "undo");
            if op.n[0] % 2 == 0 {
                same_step(bridge.redo(), shadow.redo(), &shadow, "redo");
            }
        }
        // What an editor holds can always be saved (the size bound sees to that).
        let text = format::to_string(shadow.document()).unwrap();
        let file = || std::fs::read_to_string(&path).unwrap();
        // "Saved" is never said of a document that the file does not hold.
        if bridge.is_saved().unwrap() {
            assert_eq!(file(), text, "said to be saved after {core:?}");
        }
        // Not after every step, so that some saves have several steps to write. Inside
        // a group too: what is there is written.
        if op.n[1] % 3 != 0 {
            held &= bridge.is_saved().unwrap();
            bridge.save().unwrap();
            assert!(bridge.is_saved().unwrap());
            assert_eq!(file(), text, "the file after {core:?}");
        }
    }
    // What the bridge's own editor counts as it goes is what a fresh count gives, after
    // all the groups that were ended, cancelled and left open.
    bridge.audit().unwrap();
    support::check_bound(&shadow, "the end");
    // Closing saves what the last steps left, with a group open or not.
    bridge.close().unwrap();
    let text = format::to_string(shadow.document()).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, ..ProptestConfig::default() })]
    #[test]
    fn the_bridge_agrees_with_the_core(
        steps in proptest::collection::vec((op(), with()), 1..40),
        undo_every in 0usize..5,
        seed in any::<u64>(),
    ) {
        run(&steps, undo_every, seed);
    }
}
