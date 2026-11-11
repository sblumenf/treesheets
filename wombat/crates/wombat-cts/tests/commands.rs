//! Random sequences of commands on real documents: every TreeSheets example, the referee
//! fixtures and the quirk files, imported. The checks are those of
//! `wombat-core/tests/commands.rs` (shared code in its `support` folder): valid after every
//! command, complete change reports, refused commands change nothing, undo everything gives
//! the original bytes, redo everything gives the final bytes, undo and redo in any order.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

#[path = "../../wombat-core/tests/support/mod.rs"]
mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use support::{act, op, run_sequence, run_walk};
use wombat_core::{Document, SeededIds, ViewState};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "cts") {
            out.push(path);
        }
    }
}

/// Every `.cts` at hand, imported once: (name, document, view state).
fn documents() -> &'static [(String, Document, ViewState)] {
    static DOCUMENTS: OnceLock<Vec<(String, Document, ViewState)>> = OnceLock::new();
    DOCUMENTS.get_or_init(|| {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        for dir in [
            "../../../TS/examples",
            "../../referee/fixtures",
            "../../referee/ops",
            "tests/quirks",
        ] {
            collect(&here.join(dir), &mut files);
        }
        files.sort();
        let mut ids = SeededIds::from_u64(2024);
        files
            .iter()
            .map(|path| {
                let imported = wombat_cts::import_file(path, &mut ids).unwrap();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                (name, imported.document, imported.view)
            })
            .collect()
    })
}

/// Every imported document gets its own fixed sequence of 40 random commands.
#[test]
fn every_imported_document_survives_commands_undo_and_redo() {
    let mut runner = TestRunner::deterministic();
    let (mut done, mut refused, mut cells) = (0, 0, 0);
    for (name, doc, view) in documents() {
        let ops = proptest::collection::vec(op(), 40)
            .new_tree(&mut runner)
            .unwrap()
            .current();
        let result = std::panic::catch_unwind(|| run_sequence(doc.clone(), &ops, false));
        let (d, r) = result.unwrap_or_else(|_| panic!("failed on {name}"));
        done += d;
        refused += r;
        cells += doc.cell_count();
        // The view state of a document follows its changes without ever naming a cell
        // that is gone.
        let mut editor = wombat_core::Editor::new(doc.clone()).unwrap();
        let mut view = view.clone();
        let mut ids = SeededIds::from_u64(3);
        for op in &ops {
            let command = support::command(editor.document(), op);
            if let Ok(changes) = editor.apply(command, None, &mut ids) {
                view.apply_changes(editor.document(), &changes);
                for id in &view.folded {
                    let cell = editor.document().cell(*id);
                    assert!(
                        cell.is_some_and(|c| c.grid().is_some()),
                        "{name}: stale fold"
                    );
                }
                let stale = |id| editor.document().cell(id).is_none();
                assert!(!view.zoom.is_some_and(stale), "{name}: stale zoom");
                assert!(
                    !view.selection.as_ref().is_some_and(|s| stale(s.cell)),
                    "{name}: stale selection"
                );
            }
        }
    }
    println!(
        "{} imported documents, {cells} cells: {done} commands carried out, {refused} refused",
        documents().len()
    );
    assert!(documents().len() >= 49 && done > 500 && refused > 100);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    #[test]
    fn imported_documents_any_sequence(
        which in any::<proptest::sample::Index>(),
        ops in proptest::collection::vec(op(), 0..40),
    ) {
        let (_, doc, _) = which.get(documents());
        run_sequence(doc.clone(), &ops, false);
    }

    #[test]
    fn imported_documents_undo_and_redo_in_any_order(
        which in any::<proptest::sample::Index>(),
        acts in proptest::collection::vec(act(), 0..40),
    ) {
        let (_, doc, _) = which.get(documents());
        run_walk(doc.clone(), &acts, false);
    }
}
