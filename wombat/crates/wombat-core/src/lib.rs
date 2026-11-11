//! The Wombat core: nested-grid documents, with no interface.
//!
//! See `docs/PLAN.md`, `docs/DECISIONS.md` and `docs/FILE_FORMAT.md` in the `wombat` folder.
//!
//! - [`model`]: the in-memory document (cells, grids, rich text, links, tags).
//! - [`edit`]: commands, undo and redo: the only way to change a document.
//! - [`text`]: the rules of cell text under editing.
//! - [`eval`]: formula cells and the program of a document (operations, variables, views).
//! - [`construct`]: unrecorded access for building a document, before it is edited.
//! - [`format`]: the `.wombat` JSON text of a document.
//! - [`view`]: view state (folds, selection, zoom), which is kept out of the document.
//! - [`storage`]: the only module that touches files (atomic save, images, view-state files).
//! - [`workspace`]: a folder of documents.

pub mod construct;
pub mod edit;
pub mod error;
pub mod eval;
pub mod format;
pub mod id;
mod json;
pub mod limits;
pub mod model;
pub mod storage;
pub mod text;
pub mod view;
pub mod workspace;

pub use edit::{Changes, Command, Editor, Prop};
pub use error::Error;
pub use id::{CellId, DocId, Id, IdSource, SeededIds};
pub use model::{
    Cell, CellKind, Color, Document, DrawStyle, FreshIds, Grid, ImageRef, Link, LinkTarget, Run,
    Style, Tag, TextAlign, VertAlign,
};
pub use view::{Selection, ViewState};
pub use workspace::{IdCollision, Workspace};

/// The version of the core library.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
