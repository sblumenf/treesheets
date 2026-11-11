//! Delimited text: comma, semicolon or tab separated values.
//!
//! **Writing** gives valid RFC 4180 text: a field that holds the delimiter, a double quote
//! or a line break is put in double quotes with its quotes doubled. Four things go beyond
//! the RFC. A field that holds a comma, a semicolon or a tab is quoted whichever of them
//! is the delimiter, because a spreadsheet's import dialog proposes all three as
//! separators at once (decision A113). A row whose only field is empty is written `""`,
//! because an empty line is no row to most readers (TreeSheets and this module among
//! them). A first field that starts with the character U+FEFF is quoted, so that it is not
//! taken for a byte-order mark. And [`guard`].
//!
//! **Reading** accepts every valid RFC 4180 text and reads what is not valid the way
//! TreeSheets' importer does where that is defined (`Grid::CSVImport`): see [`parse`].
//! [`detect`] finds the delimiter of a text that does not say it.

use super::{too_large, Delimiter, ExportOptions, Quoting, MAX_EXPORT_BYTES};
use crate::error::Error;

/// Whether `text` is a plain number with a sign: `-12`, `+3.5`, `-1,5`, `-.5`, `-2e10`.
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

/// Whether `text` has in it what a formula needs to do more than calculate: a
/// parenthesis (a function is called with one), a `|` (the mark of a DDE link, such as
/// `cmd|' /C calc'!A0`), or a `!` right before a letter, digit, `$`, `'`, `_` or `#` (a
/// reference to another sheet or workbook, such as `'\\host\share\[b.xlsx]S'!A1`).
fn could_act(text: &str) -> bool {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        let next = chars.peek().copied();
        let refers = c == '!' && next.is_some_and(|n| n.is_alphanumeric() || "$'_#".contains(n));
        if c == '(' || c == '|' || refers {
            return true;
        }
    }
    false
}

/// Whether a spreadsheet that opens a CSV file could take `text` for a formula that does
/// something ("CSV injection"; decision A114 has the reasons and a table of examples).
/// The rule goes by the first character, a full-width form counting as the plain one:
///
/// - **a tab or a carriage return:** always.
/// - **`=`:** always, except the text `=` alone.
/// - **`+` or `-`:** unless the text is one of these shapes, which nothing can run:
///   only signs, dashes and spaces (`-`, `--`, `+`, `- -`); a sign followed by digits,
///   spaces and `( ) . , / -` with at least one digit (a signed number, a telephone number,
///   a date: `-5`, `+44 20 7946 0958`, `+1 (555) 010-0100`, `-2026-03-04`); a number with
///   an exponent (`-2e10`); a sign, a space and text without a parenthesis, a `|` or a
///   reference to another sheet (a list item: `- item one`).
/// - **`@`:** only with a parenthesis, a `|` or a reference to another sheet in the text
///   (`@SUM(1)`); `@handle` is left alone.
///
/// Where the rule is not sure, it says yes.
pub fn is_risky(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let rest = chars.as_str();
    match first {
        '\t' | '\r' => true,
        '=' | '\u{ff1d}' => !rest.is_empty(),
        '@' | '\u{ff20}' => could_act(rest),
        '+' | '-' => {
            let dash =
                |c: char| matches!(c, '+' | '-' | ' ' | '\u{2010}'..='\u{2015}' | '\u{2212}');
            let figure = |c: char| c.is_ascii_digit() || " ().,/-".contains(c);
            let only_dashes = rest.chars().all(dash);
            let figures = rest.chars().all(figure) && rest.chars().any(|c| c.is_ascii_digit());
            let item = rest.starts_with(' ') && !could_act(rest);
            !(only_dashes || figures || item || is_signed_number(text))
        }
        // Full-width signs: taken for the plain ones where they are typed, so no shape of
        // theirs is trusted.
        '\u{ff0b}' | '\u{ff0d}' => true,
        _ => false,
    }
}

/// `text` without the single quotes it starts with.
fn unquoted(text: &str) -> &str {
    text.trim_start_matches('\'')
}

/// `text` made safe to open in a spreadsheet: if [`is_risky`], a single quote is put in
/// front, so that the field no longer starts with what makes a formula. **The quote is
/// visible:** LibreOffice and Excel show it as a character of the cell when they open a
/// CSV file (only typed into a cell is a leading quote a mark that disappears). That is
/// the price of a file that cannot run anything.
///
/// A text that already starts with single quotes and would be risky without them
/// (`'=1+1`, typed so by its author) gets one more, so that [`unguard`] can tell the
/// quote that was added from one that was there: `unguard(guard(text))` is `text`, always.
pub fn guard(text: &str) -> std::borrow::Cow<'_, str> {
    if is_risky(unquoted(text)) {
        std::borrow::Cow::Owned(format!("'{text}"))
    } else {
        std::borrow::Cow::Borrowed(text)
    }
}

/// The opposite of [`guard`], for reading back a file that Wombat exported with the
/// guard on: the single quote that `guard` put in front is taken away again. `None` if
/// `guard` would not have made this text. Only for such files: in a file from anywhere
/// else a text like `'=1+1` may be exactly what its author wrote.
pub fn unguard(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('\'')?;
    is_risky(unquoted(rest)).then_some(rest)
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
        let quote = match self.options.quoting {
            Quoting::All => true,
            Quoting::Minimal => text.contains([',', ';', '\t', '"', '\n', '\r']),
            Quoting::Narrow => text.contains([delimiter, '"', '\n', '\r']),
        } || (self.out.is_empty() && text.starts_with('\u{feff}'));
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

/// How many lines [`detect`] looks at, and how much text at most.
const DETECT_LINES: usize = 100;
const DETECT_CHARS: usize = 64 * 1024;

/// For each line at the start of `text`, how many times `delimiter` separates two fields
/// in it, read as [`parse`] reads: not inside a quoted field. The second value says, for
/// a comma, whether every one of them stands between two digits (`1,5`).
fn separators(text: &str, delimiter: char) -> (Vec<usize>, bool) {
    let mut counts = Vec::new();
    let (mut count, mut seen, mut at_start) = (0usize, false, true);
    let mut decimal = true;
    let mut before = None;
    let mut chars = text.chars().take(DETECT_CHARS).peekable();
    let mut whole = true;
    while let Some(c) = chars.next() {
        match c {
            '"' if at_start => {
                loop {
                    match chars.next() {
                        // The text looked at ends inside a quote: this line does not count.
                        None => whole = false,
                        Some('"') if chars.peek() == Some(&'"') => {
                            chars.next();
                            continue;
                        }
                        Some('"') => {}
                        Some(_) => continue,
                    }
                    break;
                }
                (seen, at_start) = (true, false);
            }
            c if c == delimiter => {
                let digit = |c: Option<char>| c.is_some_and(|c| c.is_ascii_digit());
                decimal &= digit(before) && digit(chars.peek().copied());
                count = count.saturating_add(1);
                (seen, at_start) = (true, true);
            }
            '\r' | '\n' => {
                if seen {
                    counts.push(count);
                }
                (count, seen, at_start) = (0, false, true);
                if counts.len() >= DETECT_LINES {
                    return (counts, decimal);
                }
            }
            _ => (seen, at_start) = (true, false),
        }
        before = Some(c);
    }
    // The last line counts if the text ended there, not if the look was cut short.
    if seen && whole && text.chars().nth(DETECT_CHARS).is_none() {
        counts.push(count);
    }
    (counts, decimal)
}

/// The delimiter that the lines of `text` agree on, for a file that does not say what
/// separates its fields (a `.csv` file is separated by semicolons where the comma is the
/// decimal sign, and some programs write tabs under that name). `None` if the text does
/// not show it; the caller then takes what the file's name says.
///
/// Looked at are the first 100 lines (within the first 64 KiB), read as [`parse`] reads
/// them, so that a delimiter inside a quoted field does not count. A candidate is a
/// delimiter that occurs in every line, the same number of times: it makes a table.
///
/// - Fewer than two lines show nothing: `None`.
/// - One candidate: that one.
/// - A tab among several: the tab, which no prose and no number has in it.
/// - Comma and semicolon both: the semicolon if every comma stands between two digits
///   (`1,5;2,5`: decimal commas), else `None`.
/// - No candidate (a file of one column with commas in its sentences, say): `None`.
pub fn detect(text: &str) -> Option<Delimiter> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut decimal_commas = false;
    let mut candidates = Vec::new();
    for delimiter in [Delimiter::Comma, Delimiter::Semicolon, Delimiter::Tab] {
        let (counts, decimal) = separators(text, delimiter.as_char());
        let first = counts.first().copied()?;
        if counts.len() < 2 {
            return None;
        }
        if first > 0 && counts.iter().all(|count| *count == first) {
            candidates.push(delimiter);
            decimal_commas |= delimiter == Delimiter::Comma && decimal;
        }
    }
    match candidates.as_slice() {
        [one] => Some(*one),
        [.., Delimiter::Tab] => Some(Delimiter::Tab),
        [Delimiter::Comma, Delimiter::Semicolon] if decimal_commas => Some(Delimiter::Semicolon),
        _ => None,
    }
}
