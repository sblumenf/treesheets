//! The Wombat core: nested-grid documents, with no interface.
//!
//! See `docs/PLAN.md`, `docs/DECISIONS.md` and `docs/FILE_FORMAT.md` in the `wombat` folder.
//!
//! - [`model`]: the in-memory document (cells, grids, rich text, links, tags).
//! - [`format`]: the `.wombat` JSON text of a document.
//! - [`view`]: view state (folds, selection, zoom), which is kept out of the document.
//! - [`storage`]: the only module that touches files (atomic save, images, view-state files).
//! - [`workspace`]: a folder of documents.

pub mod error;
pub mod format;
pub mod id;
mod json;
pub mod limits;
pub mod model;
pub mod storage;
pub mod view;
pub mod workspace;

pub use error::Error;
pub use id::{CellId, DocId, Id, IdSource, SeededIds};
pub use model::{
    Cell, CellKind, Color, Document, DrawStyle, Grid, ImageRef, Link, LinkTarget, Run, Style, Tag,
    TextAlign, VertAlign,
};
pub use view::{Selection, ViewState};
pub use workspace::Workspace;

/// The version of the core library.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
