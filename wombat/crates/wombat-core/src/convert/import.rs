//! Import: what a delimited or indented text file holds, as a [`Table`] of texts.
//!
//! The readers follow TreeSheets' importers (`System::Import`, `Grid::CSVImport`,
//! `System::FillRows`) and are bounded: a file cannot make more cells, longer texts or
//! deeper nesting than a document may have, whatever it holds.

use super::delimited::{self, ParseLimits};
use super::Delimiter;
use crate::construct::Construct;
use crate::error::Error;
use crate::eval::is_formula_text;
use crate::id::{CellId, IdSource};
use crate::limits::{MAX_CELLS, MAX_DEPTH, MAX_FILE_BYTES, MAX_TEXT_CODE_POINTS};
use crate::model::{code_points, CellKind, Document};
use crate::text;

/// A grid of texts to import: what [`parse_table`] and [`parse_outline`] give, and what
/// [`to_document`] and [`crate::Command::Import`] turn into cells. It is always a
/// rectangle of at least one cell, nested no deeper than a document may be, with no more
/// cells than a document may have.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Table {
    pub(crate) xs: u32,
    pub(crate) ys: u32,
    /// Row by row.
    pub(crate) cells: Vec<Entry>,
    /// The levels of grids: 1 for a table with no table inside.
    pub(crate) height: u32,
    /// The cells of this table and of those inside.
    pub(crate) count: usize,
    /// The least the cells take in a saved document.
    bytes: u64,
}

/// The least a cell takes in a saved document: the comma and line end before it, one
/// space of indentation, and `{"id": "` and `"}` around the 36 characters of the ID. (The
/// first cell of a grid has no comma before it; the grid itself takes far more than that.)
const MIN_CELL_BYTES: u64 = 49;

/// The least a text adds to its cell in a saved document: `, "text": "` and `"` around it.
fn text_bytes(text: &str) -> u64 {
    if text.is_empty() {
        0
    } else {
        (text.len() as u64).saturating_add(12)
    }
}

/// One cell of a [`Table`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Entry {
    pub(crate) text: String,
    pub(crate) formula: bool,
    pub(crate) grid: Option<Table>,
}

impl Entry {
    /// A cell with this text; characters that cell text cannot hold become spaces.
    pub fn new(text: &str) -> Entry {
        Entry {
            text: text::clean(text),
            formula: false,
            grid: None,
        }
    }

    /// The same cell as a formula cell, if its text is a formula (`=` and more).
    pub fn as_formula(mut self) -> Entry {
        self.formula = is_formula_text(&self.text);
        self
    }

    /// The same cell with a grid inside.
    pub fn with_grid(mut self, grid: Table) -> Entry {
        self.grid = Some(grid);
        self
    }

    /// The text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the cell becomes a formula cell.
    pub fn is_formula(&self) -> bool {
        self.formula
    }

    /// The grid inside.
    pub fn grid(&self) -> Option<&Table> {
        self.grid.as_ref()
    }
}

impl Table {
    /// A table of `xs` columns from its cells row by row. Refused unless the cells fill
    /// whole rows, there is at least one, and the limits of a document hold: cells,
    /// nesting, length of a text.
    pub fn new(xs: u32, cells: Vec<Entry>) -> Result<Table, Error> {
        if xs == 0 || cells.is_empty() || !cells.len().is_multiple_of(xs as usize) {
            return Err(Error::Invalid(
                "a table has at least one cell and only whole rows".into(),
            ));
        }
        let ys = u32::try_from(cells.len() / xs as usize)
            .map_err(|_| Error::Limit(format!("more than {MAX_CELLS} cells")))?;
        let (mut height, mut count) = (1u32, cells.len());
        for entry in &cells {
            if code_points(&entry.text) > MAX_TEXT_CODE_POINTS {
                return Err(Error::Limit(format!(
                    "a text longer than {MAX_TEXT_CODE_POINTS} characters"
                )));
            }
            if let Some(inner) = &entry.grid {
                height = height.max(inner.height.saturating_add(1));
                count = count.saturating_add(inner.count);
            }
        }
        if height > MAX_DEPTH {
            return Err(Error::Limit(format!(
                "grids nested deeper than {MAX_DEPTH}"
            )));
        }
        // The cell that gets the table is one more.
        if count >= MAX_CELLS {
            return Err(Error::Limit(format!(
                "{count} cells are more than a document may have, which is {MAX_CELLS}"
            )));
        }
        // What the cells would at least take in a saved document: a cheap sum, so that a
        // table too large to save is refused here and not after a document was made of it.
        let texts = cells.iter().fold(0u64, |sum, entry| {
            let inner = entry.grid.as_ref().map_or(0, |grid| grid.bytes);
            sum.saturating_add(text_bytes(&entry.text))
                .saturating_add(inner)
        });
        let bytes = (cells.len() as u64)
            .saturating_mul(MIN_CELL_BYTES)
            .saturating_add(texts);
        if bytes > MAX_FILE_BYTES {
            let room = MAX_FILE_BYTES.saturating_mul(count as u64) / bytes.max(1);
            return Err(Error::Limit(format!(
                "{count} cells with their text would make a document of at least {bytes} \
                 bytes; a document may be at most {MAX_FILE_BYTES} bytes, which is room for \
                 at most about {room} cells such as these (and never more than {MAX_CELLS}); \
                 import a part of it"
            )));
        }
        Ok(Table {
            xs,
            ys,
            cells,
            height,
            count,
            bytes,
        })
    }

    /// The same table with the guard of an export taken off: every text that
    /// [`crate::convert::delimited::guard`] would have made loses the single quote it
    /// starts with. With `formulas`, a text that is a formula after that becomes a formula
    /// cell. For a file that Wombat itself exported with the guard on, and only for such a
    /// file: elsewhere a text like `'=1+1` may be what its author wrote.
    pub fn unguarded(mut self, formulas: bool) -> Table {
        for entry in &mut self.cells {
            if let Some(text) = delimited::unguard(&entry.text) {
                entry.text = text.to_string();
                entry.formula = formulas && is_formula_text(&entry.text);
            }
            entry.grid = entry.grid.take().map(|grid| grid.unguarded(formulas));
        }
        self
    }

    /// The number of columns.
    pub fn xs(&self) -> u32 {
        self.xs
    }

    /// The number of rows.
    pub fn ys(&self) -> u32 {
        self.ys
    }

    /// The cells, row by row.
    pub fn cells(&self) -> &[Entry] {
        &self.cells
    }

    /// The levels of grids: 1 for a table with no table inside.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The number of cells, those of the tables inside included.
    pub fn cell_count(&self) -> usize {
        self.count
    }
}

/// Windows-1252 for the bytes 0x80 to 0x9F, where it differs from Latin-1.
const CP1252: [char; 32] = [
    '\u{20ac}', '\u{81}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{8d}', '\u{17d}', '\u{8f}',
    '\u{90}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}', '\u{9d}', '\u{17e}', '\u{178}',
];

/// What the bytes of a file to import are read as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Encoding {
    /// UTF-8, with or without a byte-order mark.
    Utf8,
    /// UTF-16, the low byte of each unit first (Excel's "Unicode text").
    Utf16Le,
    /// UTF-16, the high byte first.
    Utf16Be,
    /// Windows-1252 (Excel's plain "CSV" on Windows in western Europe and the Americas).
    Windows1252,
}

impl Encoding {
    /// The name for a person.
    pub fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf16Le => "UTF-16 (little-endian)",
            Encoding::Utf16Be => "UTF-16 (big-endian)",
            Encoding::Windows1252 => "Windows-1252",
        }
    }
}

const UTF8_MARK: &[u8] = &[0xef, 0xbb, 0xbf];
const UTF16_LE_MARK: &[u8] = &[0xff, 0xfe];
const UTF16_BE_MARK: &[u8] = &[0xfe, 0xff];

fn not_text(encoding: Encoding, at: usize) -> Error {
    Error::Invalid(format!(
        "the file is not {}: byte {at} (counted from 0) does not belong; if it is in another \
         encoding, say which (utf-8, utf-16le, utf-16be or windows-1252)",
        encoding.name()
    ))
}

fn utf8(bytes: &[u8], skipped: usize) -> Result<String, Error> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.to_string()),
        Err(error) => Err(not_text(
            Encoding::Utf8,
            skipped.saturating_add(error.valid_up_to()),
        )),
    }
}

fn utf16(bytes: &[u8], skipped: usize, encoding: Encoding) -> Result<String, Error> {
    let unit: fn([u8; 2]) -> u16 = match encoding {
        Encoding::Utf16Be => u16::from_be_bytes,
        _ => u16::from_le_bytes,
    };
    let pairs = bytes.chunks_exact(2);
    let odd = !pairs.remainder().is_empty();
    let units = pairs.map(|pair| {
        unit([
            pair.first().copied().unwrap_or(0),
            pair.get(1).copied().unwrap_or(0),
        ])
    });
    let mut text = String::with_capacity(bytes.len() / 2);
    // Where the unit being read starts.
    let mut at = skipped;
    for c in char::decode_utf16(units) {
        match c {
            Ok(c) => {
                text.push(c);
                at = at.saturating_add(c.len_utf16().saturating_mul(2));
            }
            Err(_) => return Err(not_text(encoding, at)),
        }
    }
    if odd {
        return Err(not_text(encoding, at));
    }
    Ok(text)
}

/// The text of a file to import, and what it was read as. Nothing is guessed (decision
/// A115):
///
/// - Without an `encoding`, the bytes are UTF-8, with or without a byte-order mark, or,
///   if they start with the byte-order mark of UTF-16, that. Anything else is an error
///   that names the first byte that does not belong.
/// - With one, they are read as that: for a file in Windows-1252, or in UTF-16 without a
///   mark, somebody has to say so. A wrong byte is an error in the same way; in
///   Windows-1252 every byte is a character, so that never fails.
///
/// The byte-order mark of the encoding is not part of the text.
pub fn decode(bytes: &[u8], encoding: Option<Encoding>) -> Result<(String, Encoding), Error> {
    let after = |mark: &[u8]| bytes.strip_prefix(mark).map(|rest| (rest, mark.len()));
    let encoding = encoding.unwrap_or(if bytes.starts_with(UTF16_LE_MARK) {
        Encoding::Utf16Le
    } else if bytes.starts_with(UTF16_BE_MARK) {
        Encoding::Utf16Be
    } else {
        Encoding::Utf8
    });
    let text = match encoding {
        Encoding::Utf8 => {
            let (rest, skipped) = after(UTF8_MARK).unwrap_or((bytes, 0));
            utf8(rest, skipped)?
        }
        Encoding::Utf16Le => {
            let (rest, skipped) = after(UTF16_LE_MARK).unwrap_or((bytes, 0));
            utf16(rest, skipped, encoding)?
        }
        Encoding::Utf16Be => {
            let (rest, skipped) = after(UTF16_BE_MARK).unwrap_or((bytes, 0));
            utf16(rest, skipped, encoding)?
        }
        Encoding::Windows1252 => bytes
            .iter()
            .map(|b| match b.checked_sub(0x80).map(usize::from) {
                Some(n) if n < 32 => CP1252.get(n).copied().unwrap_or(' '),
                _ => char::from(*b),
            })
            .collect(),
    };
    Ok((text, encoding))
}

fn entry(text: &str, formulas: bool) -> Entry {
    let entry = Entry::new(text);
    if formulas {
        entry.as_formula()
    } else {
        entry
    }
}

fn nothing() -> Error {
    Error::Invalid("nothing to import: the file has no lines".into())
}

/// Reads delimited text (CSV, TSV, semicolon separated) as one table: a row per line, a
/// column per field, short rows padded with empty cells to a rectangle. How quotes, line
/// breaks and the rest are read is in [`delimited::parse`]. Characters that cell text
/// cannot hold (tabs and other control characters) become spaces.
///
/// With `formulas`, a field that starts with `=` and has more becomes a formula cell, as
/// if it had been typed. Without, every cell is plain text, which is what TreeSheets'
/// import does, and what is right for a file from somewhere else: its formulas were
/// written for another program and would otherwise start calculating here unasked.
///
/// A file without any line is refused; so is one that would make more cells than a
/// document may have, or a text longer than a cell may hold.
pub fn parse_table(text: &str, delimiter: Delimiter, formulas: bool) -> Result<Table, Error> {
    let limits = ParseLimits {
        cells: MAX_CELLS.saturating_sub(1),
        field: MAX_TEXT_CODE_POINTS,
    };
    let (rows, widest) = delimited::parse(text, delimiter, limits)?;
    if rows.is_empty() || widest == 0 {
        return Err(nothing());
    }
    let mut cells = Vec::with_capacity(rows.len().saturating_mul(widest));
    for row in rows {
        let missing = widest.saturating_sub(row.len());
        cells.extend(row.iter().map(|field| entry(field, formulas)));
        cells.extend(std::iter::repeat_with(|| Entry::new("")).take(missing));
    }
    let xs = u32::try_from(widest).map_err(|_| Error::Limit("too many columns".into()))?;
    Table::new(xs, cells)
}

/// A line of an outline while it is read: its text and the lines inside it.
struct Node {
    text: String,
    inside: Vec<Node>,
}

/// The number of spaces and tabs a line starts with; each counts as one.
fn indentation(line: &str) -> usize {
    line.bytes()
        .take_while(|b| *b == b' ' || *b == b'\t')
        .count()
}

/// `System::FillRows`: reads lines from `at` on into `rows` for as long as they belong
/// there, and returns the index of the first line that does not.
fn fill(
    rows: &mut Vec<Node>,
    lines: &[&str],
    column: usize,
    mut at: usize,
    levels: u32,
    top: bool,
) -> usize {
    while let Some(line) = lines.get(at) {
        let indent = indentation(line);
        if indent < column && !top {
            return at;
        }
        match rows.last_mut() {
            // Deeper than this level: the grid of the line before, as long as nesting
            // may go deeper; beyond that the line is a row of this level.
            Some(last) if indent > column && levels > 1 => {
                at = fill(
                    &mut last.inside,
                    lines,
                    indent,
                    at,
                    levels.saturating_sub(1),
                    false,
                );
            }
            _ => {
                rows.push(Node {
                    text: line.get(indent..).unwrap_or("").to_string(),
                    inside: Vec::new(),
                });
                at = at.saturating_add(1);
            }
        }
    }
    lines.len()
}

fn table_of(rows: Vec<Node>, formulas: bool) -> Result<Table, Error> {
    let mut cells = Vec::with_capacity(rows.len());
    for node in rows {
        let mut cell = entry(&node.text, formulas);
        if !node.inside.is_empty() {
            cell = cell.with_grid(table_of(node.inside, formulas)?);
        }
        cells.push(cell);
    }
    Table::new(1, cells)
}

/// Reads indented text as TreeSheets' "Tab or space delimited text" import does: every
/// line is a cell of a grid of one column, and a line indented further than the one
/// before it starts a grid inside that one. So:
///
/// - Each space and each tab at the start of a line counts as one step of indentation;
///   any deeper indentation is one level deeper, however many steps it is.
/// - The first line sets the indentation of the top level; a later line indented less
///   than that is a row of the top level too.
/// - Empty lines are skipped; a line of only spaces is an empty cell.
/// - Lines end at CR LF, LF or CR. Spaces at the end of a line are kept; characters that
///   cell text cannot hold become spaces.
/// - Nesting stops at `levels` grids (give [`crate::limits::MAX_DEPTH`] less the depth of
///   the cell the table goes into); lines indented deeper become rows of that level.
///
/// `formulas` is as for [`parse_table`]. A text without any line is refused, and one with
/// more lines than a document may have cells.
pub fn parse_outline(text: &str, levels: u32, formulas: bool) -> Result<Table, Error> {
    let lines: Vec<&str> = text
        .split(['\r', '\n'])
        .filter(|line| !line.is_empty())
        .collect();
    let Some(first) = lines.first() else {
        return Err(nothing());
    };
    if lines.len() >= MAX_CELLS {
        return Err(Error::Limit(format!("more than {MAX_CELLS} cells")));
    }
    let mut rows = Vec::new();
    let levels = levels.clamp(1, MAX_DEPTH);
    fill(&mut rows, &lines, indentation(first), 0, levels, true);
    table_of(rows, formulas)
}

/// A new document whose root cell has the table as its grid.
pub fn to_document(table: &Table, ids: &mut dyn IdSource) -> Result<Document, Error> {
    let mut doc = Document::new(ids);
    let mut todo: Vec<(CellId, &Table)> = vec![(doc.root(), table)];
    while let Some((owner, table)) = todo.pop() {
        doc.add_grid(owner, table.xs, table.ys, ids)?;
        let children: Vec<CellId> = doc
            .cell(owner)
            .and_then(|c| c.grid())
            .map(|g| g.cells().to_vec())
            .unwrap_or_default();
        for (id, entry) in children.into_iter().zip(&table.cells) {
            if let Some(cell) = Construct::cell_mut(&mut doc, id) {
                cell.text.clone_from(&entry.text);
                if entry.formula {
                    cell.kind = CellKind::Formula;
                }
            }
            if let Some(inner) = &entry.grid {
                todo.push((id, inner));
            }
        }
    }
    doc.validate()?;
    Ok(doc)
}
