//! Random sequences of valid commands, undo and redo through an open document of the
//! bridge, beside an editor of the core that is given the same: every report, every error
//! and the bytes of every saved file must be the same. The bridge adds nothing to what the
//! core does and loses nothing of it, whatever the commands are.
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
use wombat_bridge::convert::{command_out, report_out};
use wombat_core::{format, storage, Editor, SeededIds};

fn run(ops: &[Op], undo_every: usize, seed: u64) {
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
    let report = |shadow: &Editor, changes: &wombat_core::Changes| {
        report_out(changes, shadow.recalculated())
    };
    for (i, op) in ops.iter().enumerate() {
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
        if undo_every > 0 && i % undo_every == undo_every - 1 {
            let (got, want) = (bridge.undo().unwrap(), shadow.undo().unwrap());
            assert_eq!(got, want.map(|changes| report(&shadow, &changes)), "undo");
            if op.n[0] % 2 == 0 {
                let (got, want) = (bridge.redo().unwrap(), shadow.redo().unwrap());
                assert_eq!(got, want.map(|changes| report(&shadow, &changes)), "redo");
            }
        }
        // What an editor holds can always be saved (the size bound sees to that).
        let text = format::to_string(shadow.document()).unwrap();
        let file = || std::fs::read_to_string(&path).unwrap();
        // "Saved" is never said of a document that the file does not hold.
        if bridge.is_saved().unwrap() {
            assert_eq!(file(), text, "said to be saved after {core:?}");
        }
        // Not after every step, so that some saves have several steps to write.
        if op.n[1] % 3 != 0 {
            bridge.save().unwrap();
            assert!(bridge.is_saved().unwrap());
            assert_eq!(file(), text, "the file after {core:?}");
        }
    }
    // Closing saves what the last steps left.
    bridge.close().unwrap();
    let text = format::to_string(shadow.document()).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 300, ..ProptestConfig::default() })]
    #[test]
    fn the_bridge_agrees_with_the_core(
        ops in proptest::collection::vec(op(), 1..40),
        undo_every in 0usize..5,
        seed in any::<u64>(),
    ) {
        run(&ops, undo_every, seed);
    }
}
