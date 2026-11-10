//! The construction layer: unrestricted, unrecorded write access to a [`Document`].
//!
//! It is for code that **builds** a document before anyone edits it: the file reader, the
//! `.cts` importer, tests. It is not a way to edit: nothing done here can be undone or is
//! reported as a change, and nothing is checked until [`Document::validate`] runs (saving and
//! [`crate::edit::Editor::new`] run it and refuse an invalid document). The methods exist
//! only for code that says `use wombat_core::construct::Construct`, and a document inside an
//! [`crate::edit::Editor`] cannot be reached by them at all (decision A31).

use std::collections::BTreeMap;

use crate::error::Error;
use crate::id::{CellId, IdSource};
use crate::model::{Cell, Document, Extra, Tag};

/// Write access to a document under construction. See the module documentation.
pub trait Construct {
    /// The cell with this ID, to set its text and properties.
    fn cell_mut(&mut self, id: CellId) -> Option<&mut Cell>;

    /// Gives a cell that has no grid a grid of `xs` by `ys` new empty cells.
    fn add_grid(
        &mut self,
        cell: CellId,
        xs: u32,
        ys: u32,
        ids: &mut dyn IdSource,
    ) -> Result<(), Error>;

    /// The tags of the document, by name.
    fn tags_mut(&mut self) -> &mut BTreeMap<String, Tag>;

    /// The unrecognised document-level fields.
    fn extra_mut(&mut self) -> &mut Extra;
}

impl Construct for Document {
    fn cell_mut(&mut self, id: CellId) -> Option<&mut Cell> {
        Document::cell_mut(self, id)
    }

    fn add_grid(
        &mut self,
        cell: CellId,
        xs: u32,
        ys: u32,
        ids: &mut dyn IdSource,
    ) -> Result<(), Error> {
        Document::add_grid(self, cell, xs, ys, ids)
    }

    fn tags_mut(&mut self) -> &mut BTreeMap<String, Tag> {
        &mut self.tags
    }

    fn extra_mut(&mut self) -> &mut Extra {
        &mut self.extra
    }
}
