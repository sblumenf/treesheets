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
//! - is at most [`MAX_STEM_UNITS`] UTF-16 units before the extension, the most a sheet
//!   name may have in Excel (which counts that way: a letter outside the Basic
//!   Multilingual Plane is two), so that a file opens as a sheet of the same name;
//! - differs from every other name of the same export in its suffix, which is lower-case
//!   ASCII, so also on file systems that ignore case or the way accents are stored.

use std::collections::BTreeMap;

use crate::id::CellId;

/// The most UTF-16 units of a name before its extension.
pub const MAX_STEM_UNITS: usize = 31;
/// How many digits of a cell's ID a suffix has, and how many when that is not enough.
const ID_DIGITS: usize = 8;
const LONG_ID_DIGITS: usize = 16;

/// The words of a name: the labels joined by `-`, every character that is not a letter or
/// digit turned into `-`, runs of `-` made one, and of a long result the end, which says
/// most about the table: as much as `units` UTF-16 units hold. `table` if nothing is left.
pub fn words(labels: &[&str], units: usize) -> String {
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
    let mut room = units;
    let mut start = out.len();
    while let Some(c) = start.checked_sub(1).and_then(|at| out.get(at)) {
        match room.checked_sub(c.len_utf16()) {
            Some(left) => room = left,
            None => break,
        }
        start = start.saturating_sub(1);
    }
    let kept: String = out
        .into_iter()
        .skip(start)
        .skip_while(|c| *c == '-')
        .collect();
    if kept.is_empty() {
        "table".chars().take(units).collect()
    } else {
        kept
    }
}

/// The names, without extension, of the tables of one export: for each, its labels (the
/// texts on the way to it) and the ID of the cell it sits in. The suffix is the first
/// eight digits of that ID, so a table keeps its name from one export to the next for as
/// long as its cell exists and its labels stay.
///
/// Two tables of one export can have IDs that start with the same eight digits: among
/// 10,000 tables with random IDs that happens in about one export in a hundred. Those
/// tables, and only those, get sixteen digits. Should even these be equal (which random
/// IDs do not do; a crafted file can), they get their number in the export instead
/// (`n1`, `n2`, ...). Every other table keeps its name.
pub fn stems(tables: &[(Vec<&str>, CellId)]) -> Vec<String> {
    let hex = |id: &CellId, digits: usize| -> String {
        format!("{:032x}", id.0.as_u128())
            .chars()
            .take(digits)
            .collect()
    };
    let mut times: BTreeMap<String, usize> = BTreeMap::new();
    for digits in [ID_DIGITS, LONG_ID_DIGITS] {
        for (_, id) in tables {
            let count = times.entry(hex(id, digits)).or_default();
            *count = count.saturating_add(1);
        }
    }
    let alone = |suffix: &String| times.get(suffix) == Some(&1);
    tables
        .iter()
        .enumerate()
        .map(|(n, (labels, id))| {
            let (short, long) = (hex(id, ID_DIGITS), hex(id, LONG_ID_DIGITS));
            let suffix = if alone(&short) {
                short
            } else if alone(&long) {
                long
            } else {
                format!("n{}", n.saturating_add(1))
            };
            let room = MAX_STEM_UNITS.saturating_sub(suffix.len().saturating_add(1));
            format!("{}-{suffix}", words(labels, room))
        })
        .collect()
}
