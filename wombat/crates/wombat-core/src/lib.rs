//! The Wombat core: nested-grid documents, with no interface.
//!
//! See `docs/PLAN.md`, `docs/DECISIONS.md` and `docs/FILE_FORMAT.md` in the `wombat` folder.
//!
//! - [`model`]: the in-memory document (cells, grids, rich text, links, tags).
//! - [`edit`]: commands, undo and redo: the only way to change a document.
//! - [`text`]: the rules of cell text under editing.
//! - [`eval`]: formula cells and the program of a document (operations, variables, views).
//! - [`construct`]: unrecorded access for building a document, before it is edited.
//! - [`convert`]: import and export as CSV, TSV, indented text and tables to join; no files.
//! - [`export_files`]: writing an export to a file or a folder (the file layer, with `storage`).
//! - [`format`]: the `.wombat` JSON text of a document.
//! - [`view`]: view state (folds, selection, zoom), which is kept out of the document.
//! - [`storage`]: the only module that touches files (atomic save, images, view-state files).
//! - [`workspace`]: a folder of documents.
//! - [`links`]: what links point at and what points at a cell, across documents.
//! - [`summary`], [`scan`]: indexes over a workspace, one summary per document; `scan` reads
//!   the folder and keeps a cache on disk (it belongs to the file layer, with `storage`).

pub mod construct;
pub mod convert;
pub mod edit;
pub mod error;
pub mod eval;
pub mod export_files;
pub mod format;
pub mod id;
mod json;
pub mod limits;
pub mod links;
pub mod model;
pub mod scan;
pub mod storage;
pub mod summary;
pub mod text;
pub mod view;
pub mod workspace;

pub use edit::{CellView, Changes, Clip, Command, Editor, GridView, Prop};
pub use error::Error;
pub use id::{CellId, DocId, Id, IdSource, SeededIds};
pub use links::LinkIndex;
pub use model::{
    Cell, CellKind, Color, Document, DrawStyle, FreshIds, Grid, ImageRef, Link, LinkTarget, Run,
    Style, Tag, TextAlign, VertAlign,
};
pub use view::{Selection, ViewState};
pub use workspace::{IdCollision, Workspace};

/// The version of the core library.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
