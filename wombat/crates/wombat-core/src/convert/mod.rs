//! Import and export: documents as delimited text (CSV, TSV), indented text and a folder
//! of tables that a spreadsheet can join (milestone 7, decisions A95-A104).
//!
//! Everything here is conversion between values: a document and text. Nothing reads or
//! writes a file; that is [`crate::export_files`] and the caller.
//!
//! - [`delimited`]: writing and reading delimited text (RFC 4180), and the rule that keeps
//!   an exported cell from being run as a formula by a spreadsheet.
//! - [`grid_csv`]: one grid as one table. [`flat_csv`]: a grid with everything nested in
//!   it as one table, each row after the rows it is inside of. [`outline`]: TreeSheets'
//!   indented text.
//! - [`lookup`]: one table per grid, with ID columns that join them.
//! - [`parse_table`], [`parse_outline`]: what a file to import holds, as a [`Table`] that
//!   [`to_document`] or [`crate::Command::Import`] turns into cells.

pub mod delimited;
mod export;
mod import;
pub mod lookup;
pub mod names;

pub use export::{flat_csv, grid_csv, outline};
pub use import::{decode, parse_outline, parse_table, to_document, Entry, Table};

use crate::error::Error;
use crate::eval::Calc;
use crate::id::CellId;
use crate::model::Document;

/// The most bytes one export may come to (for a folder: all its files together). A
/// flattened export repeats the outer rows on every inner row, so it can be far larger
/// than the document; past this size it is refused instead of filling memory.
pub const MAX_EXPORT_BYTES: usize = 512 * 1024 * 1024;

/// What separates the fields of a row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Delimiter {
    /// `,`: CSV.
    Comma,
    /// `;`: what spreadsheets write as CSV where the comma is the decimal sign.
    Semicolon,
    /// A tab: TSV.
    Tab,
}

impl Delimiter {
    /// The character.
    pub fn as_char(self) -> char {
        match self {
            Delimiter::Comma => ',',
            Delimiter::Semicolon => ';',
            Delimiter::Tab => '\t',
        }
    }
}

/// Which fields are written in double quotes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Quoting {
    /// Those that need it: a field with the delimiter, a double quote or a line break in
    /// it, and the empty field of a row that has no other (an empty line is no row).
    Minimal,
    /// All, as TreeSheets writes CSV.
    All,
}

/// How delimited text is written. The default is what a spreadsheet reads without
/// questions and without danger: see each field.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExportOptions {
    /// The delimiter; a comma by default.
    pub delimiter: Delimiter,
    /// Which fields are quoted; by default those that need it.
    pub quoting: Quoting,
    /// Whether the text starts with a UTF-8 byte-order mark. On by default: without it
    /// Excel reads a CSV file in the system's old encoding and shows every non-ASCII
    /// letter wrong.
    pub bom: bool,
    /// Whether lines end in CR LF (RFC 4180, the default) instead of LF.
    pub crlf: bool,
    /// Whether text that a spreadsheet would run as a formula is made harmless
    /// ([`delimited::guard`]). On by default.
    pub guard: bool,
    /// Whether a formula cell is exported as its formula instead of what it shows.
    pub formulas: bool,
    /// Whether every column is followed by a column with the notes of its cells.
    pub notes: bool,
}

impl Default for ExportOptions {
    fn default() -> ExportOptions {
        ExportOptions {
            delimiter: Delimiter::Comma,
            quoting: Quoting::Minimal,
            bom: true,
            crlf: true,
            guard: true,
            formulas: false,
            notes: false,
        }
    }
}

impl ExportOptions {
    /// The bytes of TreeSheets' own export on Linux and macOS: no byte-order mark, LF,
    /// nothing guarded, and for CSV every field quoted.
    pub fn treesheets(delimiter: Delimiter) -> ExportOptions {
        ExportOptions {
            delimiter,
            quoting: match delimiter {
                Delimiter::Tab => Quoting::Minimal,
                _ => Quoting::All,
            },
            bom: false,
            crlf: false,
            guard: false,
            formulas: false,
            notes: false,
        }
    }
}

/// A document with what its formula cells show: what an export reads from.
#[derive(Clone, Copy)]
pub struct Source<'a> {
    /// The document.
    pub doc: &'a Document,
    /// The results of its formula cells ([`crate::Editor::calc`], or [`Calc::new`]).
    pub calc: &'a Calc,
}

impl<'a> Source<'a> {
    /// What a cell exports as: what it shows, or with `formulas` its own text.
    pub(crate) fn text(&self, id: CellId, formulas: bool) -> &'a str {
        if formulas {
            self.doc.cell(id).map_or("", |c| c.text.as_str())
        } else {
            self.calc.shown(self.doc, id).unwrap_or("")
        }
    }
}

pub(crate) fn too_large() -> Error {
    Error::Limit(format!(
        "the export would be larger than {MAX_EXPORT_BYTES} bytes"
    ))
}
