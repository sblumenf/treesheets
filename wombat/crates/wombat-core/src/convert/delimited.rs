//! Delimited text: comma, semicolon or tab separated values.
//!
//! **Writing** gives valid RFC 4180 text: a field that holds the delimiter, a double quote
//! or a line break is put in double quotes with its quotes doubled. Two things go beyond
//! the RFC: a row whose only field is empty is written `""`, because an empty line is no
//! row to most readers (TreeSheets and this module among them), and [`guard`]. (And a first
//! field that starts with the character U+FEFF is quoted, so that it is not taken for a
//! byte-order mark.)
//!
//! **Reading** accepts every valid RFC 4180 text and reads what is not valid the way
//! TreeSheets' importer does where that is defined (`Grid::CSVImport`): see [`parse`].

use super::{too_large, Delimiter, ExportOptions, Quoting, MAX_EXPORT_BYTES};
use crate::error::Error;

/// Whether `text` is a plain number with a sign: `-12`, `+3.5`, `-1,5`, `-.5`, `-2e10`.
/// A spreadsheet reads it as a number or as text, never as a formula with an effect.
fn is_signed_number(text: &str) -> bool {
    let rest = match text.strip_prefix(['-', '+']) {
        Some(rest) => rest,
        None => return false,
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let (mantissa, exponent) = match rest.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e.strip_prefix(['-', '+']).unwrap_or(e))),
        None => (rest, None),
    };
    let mantissa_ok = match mantissa.split_once(['.', ',']) {
        Some((whole, fraction)) => (whole.is_empty() || digits(whole)) && digits(fraction),
        None => digits(mantissa),
    };
    mantissa_ok && exponent.is_none_or(digits)
}

/// Whether a spreadsheet that opens a CSV file could run `text` as a formula: it starts
/// with `=`, `+`, `-`, `@`, a tab or a carriage return (the list of OWASP's "CSV
/// injection"), and is not just a number with a sign.
pub fn is_risky(text: &str) -> bool {
    text.starts_with(['=', '+', '-', '@', '\t', '\r']) && !is_signed_number(text)
}

/// `text` made safe to open in a spreadsheet: if [`is_risky`], a single quote is put in
/// front, which Excel, LibreOffice and Google Sheets all take as "this is text". The
/// quote stays visible in the cell; that is the price of a file that cannot run anything.
pub fn guard(text: &str) -> std::borrow::Cow<'_, str> {
    if is_risky(text) {
        std::borrow::Cow::Owned(format!("'{text}"))
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

/// Builds delimited text row by row, within [`MAX_EXPORT_BYTES`].
pub struct Writer<'a> {
    options: &'a ExportOptions,
    out: String,
    /// The fields of the row being written so far, and whether the only one is empty.
    fields: usize,
    lone_empty: bool,
    limit: usize,
}

impl<'a> Writer<'a> {
    /// Starts a text, with the byte-order mark if the options ask for one.
    pub fn new(options: &'a ExportOptions) -> Writer<'a> {
        Writer::with_limit(options, MAX_EXPORT_BYTES)
    }

    /// As [`Writer::new`], refusing to grow past `limit` bytes.
    pub fn with_limit(options: &'a ExportOptions, limit: usize) -> Writer<'a> {
        let mut out = String::new();
        if options.bom {
            out.push('\u{feff}');
        }
        Writer {
            options,
            out,
            fields: 0,
            lone_empty: false,
            limit,
        }
    }

    /// Adds a field of cell content to the current row: guarded if the options say so.
    pub fn field(&mut self, text: &str) -> Result<(), Error> {
        if self.options.guard {
            self.raw(&guard(text))
        } else {
            self.raw(text)
        }
    }

    /// Adds a field to the current row exactly as it is: for IDs, numbers and file names
    /// that the program made itself.
    pub fn raw(&mut self, text: &str) -> Result<(), Error> {
        let delimiter = self.options.delimiter.as_char();
        if self.fields > 0 {
            self.out.push(delimiter);
        }
        // At the very start of a text without a byte-order mark, U+FEFF would be read as
        // one and lost: inside quotes it is text.
        let quote = self.options.quoting == Quoting::All
            || text.contains([delimiter, '"', '\n', '\r'])
            || (self.out.is_empty() && text.starts_with('\u{feff}'));
        if quote {
            self.out.push('"');
            for c in text.chars() {
                if c == '"' {
                    self.out.push('"');
                }
                self.out.push(c);
            }
            self.out.push('"');
        } else {
            self.out.push_str(text);
        }
        self.lone_empty = self.fields == 0 && text.is_empty() && !quote;
        self.fields = self.fields.saturating_add(1);
        if self.out.len() > self.limit {
            return Err(too_large());
        }
        Ok(())
    }

    /// Ends the current row.
    pub fn end_row(&mut self) {
        if self.fields == 1 && self.lone_empty {
            self.out.push_str("\"\"");
        }
        self.out
            .push_str(if self.options.crlf { "\r\n" } else { "\n" });
        self.fields = 0;
        self.lone_empty = false;
    }

    /// The number of bytes written so far.
    pub fn len(&self) -> usize {
        self.out.len()
    }

    /// Whether nothing was written yet.
    pub fn is_empty(&self) -> bool {
        self.out.is_empty()
    }

    /// The text.
    pub fn finish(self) -> String {
        self.out
    }
}

/// The bounds of [`parse`]: a file within them cannot make a table too large to hold.
#[derive(Clone, Copy, Debug)]
pub struct ParseLimits {
    /// The most cells of the table after padding short rows: rows times columns.
    pub cells: usize,
    /// The most code points in one field.
    pub field: usize,
}

/// Reads delimited text into rows of fields. The rows are as long as the file has them
/// (not yet padded); the second value is the number of fields of the longest.
///
/// What it does with each thing, and where that differs from TreeSheets' importer:
///
/// - Rows end at CR LF, LF or CR. Empty lines are no rows, as in TreeSheets.
/// - A field that starts with a double quote runs to the closing quote; `""` inside is
///   one quote; delimiters inside are text. Line breaks inside become one space (a run of
///   them, one space), as in TreeSheets: cell text has no line breaks.
/// - Text between a closing quote and the next delimiter is kept as part of the field.
///   (TreeSheets drops the one character after the closing quote, whatever it is.)
/// - A quote that is not closed runs to the end of the file, as in TreeSheets.
/// - A quote inside a field that did not start with one is an ordinary character.
/// - A delimiter at the end of a row is followed by an empty field, as RFC 4180 says.
///   (TreeSheets drops that field; here a table whose last column is empty comes back
///   with it.)
/// - Spaces around fields are kept.
pub fn parse(
    text: &str,
    delimiter: Delimiter,
    limits: ParseLimits,
) -> Result<(Vec<Vec<String>>, usize), Error> {
    let delimiter = delimiter.as_char();
    let too_many = || Error::Limit(format!("more than {} cells", limits.cells));
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut length = 0usize;
    let mut widest = 0usize;
    // Whether the row has anything: a field ended, or characters of one were read.
    let mut started = false;
    let mut chars = text.chars().peekable();
    let push = |field: &mut String, length: &mut usize, c: char| -> Result<(), Error> {
        *length = length.saturating_add(1);
        if *length > limits.field {
            return Err(Error::Limit(format!(
                "a field longer than {} characters",
                limits.field
            )));
        }
        field.push(c);
        Ok(())
    };
    loop {
        let c = chars.next();
        match c {
            Some('"') if field.is_empty() && length == 0 => {
                // A quoted field counts even when empty: `""` alone on a line is a row.
                started = true;
                loop {
                    match chars.next() {
                        None => break,
                        Some('"') => {
                            if chars.peek() == Some(&'"') {
                                chars.next();
                                push(&mut field, &mut length, '"')?;
                            } else {
                                break;
                            }
                        }
                        Some('\r' | '\n') => {
                            while matches!(chars.peek(), Some('\r' | '\n')) {
                                chars.next();
                            }
                            // The line breaks that end the file inside a quote that was
                            // never closed are the end of the last line, not text.
                            if chars.peek().is_some() {
                                push(&mut field, &mut length, ' ')?;
                            }
                        }
                        Some(c) => push(&mut field, &mut length, c)?,
                    }
                }
                // What follows the closing quote, up to the delimiter, is text too.
                while let Some(&c) = chars.peek() {
                    if c == delimiter || c == '\r' || c == '\n' {
                        break;
                    }
                    chars.next();
                    push(&mut field, &mut length, c)?;
                }
            }
            Some(c) if c == delimiter => {
                started = true;
                row.push(std::mem::take(&mut field));
                length = 0;
                if row.len() >= limits.cells {
                    return Err(too_many());
                }
            }
            Some('\r' | '\n') | None => {
                if started || length > 0 {
                    row.push(std::mem::take(&mut field));
                    widest = widest.max(row.len());
                    rows.push(std::mem::take(&mut row));
                    // Padding makes every row as long as the longest.
                    if rows.len().saturating_mul(widest) > limits.cells {
                        return Err(too_many());
                    }
                }
                started = false;
                length = 0;
                if c.is_none() {
                    break;
                }
            }
            Some(c) => push(&mut field, &mut length, c)?,
        }
    }
    Ok((rows, widest))
}
