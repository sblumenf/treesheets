//! What the tests of an open document share.
#![allow(dead_code, clippy::unwrap_used, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::Command;
use wombat_core::{self as core, SeededIds};

/// The document that comes with the app.
pub fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/assets/example.wombat")
}

/// An empty folder for one test of the test file `suite`.
pub fn folder(suite: &str, name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(suite)
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn ids() -> SeededIds {
    SeededIds::from_u64(11)
}

/// A copy of the example to work on, as the app makes one.
pub fn open_example(suite: &str, name: &str) -> (OpenDocument, PathBuf) {
    let path = folder(suite, name).join("example.wombat");
    let doc = OpenDocument::copy_with(&example(), &path, ids()).unwrap();
    (doc, path)
}

/// A command that sets the text of the second cell of the root grid.
pub fn set_text(doc: &OpenDocument, text: &str) -> Command {
    let root = doc.grid(doc.root().unwrap()).unwrap();
    Command::SetText {
        cell: root.cells[1].id.clone(),
        text: text.into(),
    }
}

/// The text of the second cell of the root grid in the file at `path`.
pub fn text_in_file(path: &Path) -> String {
    let doc = wombat_core::storage::load_document(path).unwrap();
    let editor = wombat_core::Editor::new(doc).unwrap();
    let view = editor.grid_view(editor.document().root()).unwrap();
    view.cells[1].text.clone()
}

/// The number of commands the core has. The match in [`index`] does not compile when one
/// is added; then this and [`index`] need the new one, and so do the generator of
/// `roundtrip.rs` and the table of `fields.rs`, whose tests fail until they have it.
pub const COMMANDS: usize = 45;

/// Which command a command is, from 0.
pub fn index(command: &core::Command) -> usize {
    match command {
        core::Command::InsertText { .. } => 0,
        core::Command::DeleteText { .. } => 1,
        core::Command::ReplaceText { .. } => 2,
        core::Command::SetText { .. } => 3,
        core::Command::SetRangeStyle { .. } => 4,
        core::Command::SetRangeColor { .. } => 5,
        core::Command::SetLink { .. } => 6,
        core::Command::SetProps { .. } => 7,
        core::Command::SetStyle { .. } => 8,
        core::Command::SetColumnWidth { .. } => 9,
        core::Command::CreateGrid { .. } => 10,
        core::Command::RemoveGrid { .. } => 11,
        core::Command::InsertRows { .. } => 12,
        core::Command::InsertColumns { .. } => 13,
        core::Command::DeleteRows { .. } => 14,
        core::Command::DeleteColumns { .. } => 15,
        core::Command::ClearCells { .. } => 16,
        core::Command::DeleteCells { .. } => 17,
        core::Command::MoveRows { .. } => 18,
        core::Command::MoveColumns { .. } => 19,
        core::Command::SwapCells { .. } => 20,
        core::Command::MoveCell { .. } => 21,
        core::Command::CopyCell { .. } => 22,
        core::Command::SetTag { .. } => 23,
        core::Command::RemoveTag { .. } => 24,
        core::Command::SortRows { .. } => 25,
        core::Command::Transpose { .. } => 26,
        core::Command::HierarchySwap { .. } => 27,
        core::Command::Hierarchify { .. } => 28,
        core::Command::Flatten { .. } => 29,
        core::Command::Wrap { .. } => 30,
        core::Command::Collapse { .. } => 31,
        core::Command::SplitCell { .. } => 32,
        core::Command::TagCells { .. } => 33,
        core::Command::UntagCells { .. } => 34,
        core::Command::AdjustRelSize { .. } => 35,
        core::Command::AdjustColumnWidths { .. } => 36,
        core::Command::Run => 37,
        core::Command::EnterText { .. } => 38,
        core::Command::ToggleCheck { .. } => 39,
        core::Command::PasteCells { .. } => 40,
        core::Command::AdoptCells { .. } => 41,
        core::Command::ReleaseCell { .. } => 42,
        core::Command::Import { .. } => 43,
        core::Command::RetargetLinks { .. } => 44,
    }
}
