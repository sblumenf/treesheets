//! What the tests of an open document share.
#![allow(dead_code, clippy::unwrap_used, clippy::indexing_slicing)]

use std::path::{Path, PathBuf};

use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::Command;
use wombat_core::SeededIds;

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
