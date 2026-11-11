//! Names for exported files, made from cell text without ever trusting it.
//!
//! A table's file is named `<words>-<suffix>.csv`. The words come from cell text and are
//! only there for the reader; the suffix alone makes the name unique. A name made here
//!
//! - holds only letters and digits (of any script) and `-`: no path separator, no dot, no
//!   space, no control character, nothing that Windows, macOS, Linux or a spreadsheet's
//!   sheet names forbid;
//! - starts with a letter or digit and ends with the suffix, so it is never `.` or `..`,
//!   never hidden, never an option to a command, and never one of Windows' reserved names
//!   (`CON`, `NUL`, `COM1` and the like), which have no `-`;
//! - is at most [`MAX_STEM_CHARS`] characters before the extension, the most a sheet name
//!   may have in Excel, so that a file opens as a sheet of the same name;
//! - differs from every other name of the same export in its suffix, which is ASCII, so
//!   also on file systems that ignore case or the way accents are stored.

use crate::id::CellId;

/// The most characters of a name before its extension.
pub const MAX_STEM_CHARS: usize = 31;
/// How many digits of a cell's ID a suffix has.
const ID_DIGITS: usize = 8;
/// The most characters of the words: what the suffix and the `-` before it leave.
const MAX_WORD_CHARS: usize = MAX_STEM_CHARS - 1 - ID_DIGITS;

/// The words of a name: the labels joined by `-`, every character that is not a letter or
/// digit turned into `-`, runs of `-` made one, and of a long result the end, which says
/// most about the table. `table` if nothing is left.
pub fn words(labels: &[&str]) -> String {
    let mut out: Vec<char> = Vec::new();
    for c in labels.iter().flat_map(|l| l.chars().chain(['-'])) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if out.last().is_some_and(|last| *last != '-') {
            out.push('-');
        }
    }
    while out.last() == Some(&'-') {
        out.pop();
    }
    let skip = out.len().saturating_sub(MAX_WORD_CHARS);
    let kept: String = out
        .into_iter()
        .skip(skip)
        .skip_while(|c| *c == '-')
        .collect();
    if kept.is_empty() {
        "table".to_string()
    } else {
        kept
    }
}

/// The names, without extension, of the tables of one export: for each, its labels (the
/// texts on the way to it) and the ID of the cell it sits in. The suffix is the first
/// eight digits of that ID, so a table keeps its name from one export to the next for as
/// long as its cell exists and its labels stay. Random IDs make two equal suffixes in one
/// document practically impossible; should a file have them, every table gets its number
/// in the export as its suffix instead (`n1`, `n2`, ...).
pub fn stems(tables: &[(Vec<&str>, CellId)]) -> Vec<String> {
    let hex = |id: &CellId| -> String {
        format!("{:032x}", id.0.as_u128())
            .chars()
            .take(ID_DIGITS)
            .collect()
    };
    let mut suffixes: Vec<String> = tables.iter().map(|(_, id)| hex(id)).collect();
    let distinct: std::collections::BTreeSet<&String> = suffixes.iter().collect();
    if distinct.len() != suffixes.len() {
        suffixes = (1..=tables.len()).map(|n| format!("n{n}")).collect();
    }
    tables
        .iter()
        .zip(suffixes)
        .map(|((labels, _), suffix)| format!("{}-{suffix}", words(labels)))
        .collect()
}
