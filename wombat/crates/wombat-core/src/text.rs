//! The rules of cell text: which characters it may hold, where it may be cut, and what
//! happens to runs and links when it is edited.
//!
//! **Characters** (decision A32). Cell text and tag names hold no character below U+0020 (no
//! line breaks, tabs or other control characters), as in TreeSheets, where a cell's lines
//! come from wrapping. Editing commands turn such characters into spaces ([`clean`]), as
//! TreeSheets does, and [`crate::Document::validate`] refuses a document that has one.
//! Notes may hold any text.
//!
//! **Cuts** (decisions A11, A33). Positions are counted in Unicode code points, and a command
//! may only cut the text between two user-perceived characters (extended grapheme clusters,
//! Unicode Standard Annex 29). A command whose position or range would cut one is refused,
//! never moved to somewhere else; [`is_boundary`], [`floor_boundary`] and [`ceil_boundary`]
//! let an interface find the places that are allowed.
//!
//! **Runs and links under editing** follow TreeSheets (`src/textruns.h`):
//!
//! - *Inserting* at a position: a run that the position is inside of, or at the end of,
//!   grows (typing after bold text is bold); at the very start of the text, a run that starts
//!   there grows too. A link grows only when the position is strictly inside it, so typing
//!   at either end of a link does not extend it. Everything further on moves along.
//! - *Deleting* a range: runs and links shrink by what they lose; one that loses everything
//!   disappears. Runs that become neighbours with the same style and colour merge into one,
//!   and a run that equals the cell's base style without a colour is dropped. Links never
//!   merge when text is deleted.
//! - *Replacing* a range: the new text takes the style and the link of the first replaced
//!   character.
//! - *Styling or colouring* a range splits the runs at its ends, changes what is inside
//!   (making runs for the parts that had none), and then merges and drops as above.
//! - *Linking* a range replaces whatever links it overlapped, cutting them at its ends;
//!   neighbouring links to the same target merge.

use unicode_segmentation::GraphemeCursor;

use crate::model::{Color, Extra, Link, LinkTarget, Run, Style};

/// Whether cell text may contain `c` (decision A32): anything from U+0020 up.
pub fn is_allowed(c: char) -> bool {
    c >= ' '
}

/// Whether `text` holds only characters that cell text may contain.
pub fn is_clean(text: &str) -> bool {
    text.chars().all(is_allowed)
}

/// `text` with every character that cell text may not contain turned into a space, as
/// TreeSheets does. The number of code points stays the same.
pub fn clean(text: &str) -> String {
    text.chars()
        .map(|c| if is_allowed(c) { c } else { ' ' })
        .collect()
}

/// The byte offset in `text` of the code point position `pos`; `None` if `pos` is past the
/// end. The end itself (`pos` equal to the number of code points) is a position.
pub fn byte_offset(text: &str, pos: u32) -> Option<usize> {
    let mut indices = text.char_indices().map(|(at, _)| at);
    match indices.nth(pos as usize) {
        Some(at) => Some(at),
        // `nth` consumed everything: `pos` is the end exactly if it equals the count.
        None => (text.chars().count() == pos as usize).then_some(text.len()),
    }
}

fn boundary_at_byte(text: &str, at: usize) -> bool {
    GraphemeCursor::new(at, text.len(), true)
        .is_boundary(text, 0)
        .unwrap_or(false)
}

/// Whether the code point position `pos` of `text` is between two user-perceived characters
/// (or at the start or end), so that text may be cut there. `None` if `pos` is past the end.
pub fn is_boundary(text: &str, pos: u32) -> Option<bool> {
    byte_offset(text, pos).map(|at| boundary_at_byte(text, at))
}

/// The nearest position at or before `pos` where `text` may be cut. A `pos` past the end
/// gives the end.
pub fn floor_boundary(text: &str, pos: u32) -> u32 {
    let mut last = 0u32;
    for (n, (at, _)) in text.char_indices().enumerate() {
        let n = u32::try_from(n).unwrap_or(u32::MAX);
        if n > pos {
            return last;
        }
        if boundary_at_byte(text, at) {
            last = n;
        }
    }
    let len = u32::try_from(text.chars().count()).unwrap_or(u32::MAX);
    if pos >= len {
        len
    } else {
        last
    }
}

/// The nearest position at or after `pos` where `text` may be cut. A `pos` past the end
/// gives the end.
pub fn ceil_boundary(text: &str, pos: u32) -> u32 {
    for (n, (at, _)) in text.char_indices().enumerate() {
        let n = u32::try_from(n).unwrap_or(u32::MAX);
        if n >= pos && boundary_at_byte(text, at) {
            return n;
        }
    }
    u32::try_from(text.chars().count()).unwrap_or(u32::MAX)
}

fn end(start: u32, len: u32) -> u32 {
    start.saturating_add(len)
}

fn same_style(a: &Run, b: &Run) -> bool {
    a.style == b.style && a.color == b.color && a.extra == b.extra
}

/// Merges neighbouring runs of equal style and drops runs that are empty or do not differ
/// from the base style.
pub(crate) fn normalize_runs(runs: &mut Vec<Run>, base: Style) {
    let mut out: Vec<Run> = Vec::with_capacity(runs.len());
    for run in runs.drain(..) {
        if run.len == 0 || (run.style == base && run.color.is_none() && run.extra.is_empty()) {
            continue;
        }
        match out.last_mut() {
            Some(last) if end(last.start, last.len) == run.start && same_style(last, &run) => {
                last.len = last.len.saturating_add(run.len);
            }
            _ => out.push(run),
        }
    }
    *runs = out;
}

/// `n` code points were inserted at `pos`.
pub(crate) fn runs_insert(runs: &mut [Run], pos: u32, n: u32) {
    if n == 0 {
        return;
    }
    for run in runs {
        if (run.start < pos && pos <= end(run.start, run.len)) || (pos == 0 && run.start == 0) {
            run.len = run.len.saturating_add(n);
        } else if run.start >= pos {
            run.start = run.start.saturating_add(n);
        }
    }
}

/// Where a position ends up when `n` code points from `pos` on are removed.
fn after_removal(x: u32, pos: u32, n: u32) -> u32 {
    if x <= pos {
        x
    } else if x >= end(pos, n) {
        x.saturating_sub(n)
    } else {
        pos
    }
}

/// The `n` code points from `pos` on were removed.
pub(crate) fn runs_remove(runs: &mut Vec<Run>, pos: u32, n: u32, base: Style) {
    if n == 0 {
        return;
    }
    for run in runs.iter_mut() {
        let start = after_removal(run.start, pos, n);
        let stop = after_removal(end(run.start, run.len), pos, n);
        run.start = start;
        run.len = stop.saturating_sub(start);
    }
    normalize_runs(runs, base);
}

/// The `n` code points from `pos` on were replaced by `m` code points.
pub(crate) fn runs_replace(runs: &mut Vec<Run>, pos: u32, n: u32, m: u32, base: Style) {
    if n == 0 {
        runs_insert(runs, pos, m);
    } else if m == 0 {
        runs_remove(runs, pos, n, base);
    } else {
        // Insert after the first replaced character, so that the new text takes its style.
        runs_insert(runs, pos.saturating_add(1), m);
        runs_remove(runs, pos, 1, base);
        runs_remove(runs, pos.saturating_add(m), n.saturating_sub(1), base);
    }
    normalize_runs(runs, base);
}

/// Applies `change` to the style of every code point in `from..to` (which must be inside the
/// text), splitting runs at the ends and making runs for the parts that had none.
pub(crate) fn runs_modify(
    runs: &mut Vec<Run>,
    from: u32,
    to: u32,
    base: Style,
    change: impl Fn(&mut Run),
) {
    if from >= to {
        return;
    }
    let mut out: Vec<Run> = Vec::with_capacity(runs.len().saturating_add(2));
    let mut pos = from;
    let gap = |out: &mut Vec<Run>, a: u32, b: u32| {
        if a < b {
            let mut run = Run {
                start: a,
                len: b.saturating_sub(a),
                style: base,
                color: None,
                extra: Extra::new(),
            };
            change(&mut run);
            out.push(run);
        }
    };
    for mut run in runs.drain(..) {
        let stop = end(run.start, run.len);
        if stop <= from {
            out.push(run);
        } else if run.start >= to {
            gap(&mut out, pos, to);
            pos = to;
            out.push(run);
        } else {
            // The parts of the run outside the range keep their style.
            if run.start < from {
                let mut head = run.clone();
                head.len = from.saturating_sub(run.start);
                run.len = run.len.saturating_sub(head.len);
                run.start = from;
                out.push(head);
            }
            let mut tail = None;
            if stop > to {
                let mut rest = run.clone();
                rest.start = to;
                rest.len = stop.saturating_sub(to);
                run.len = to.saturating_sub(run.start);
                tail = Some(rest);
            }
            gap(&mut out, pos, run.start);
            pos = end(run.start, run.len);
            change(&mut run);
            out.push(run);
            out.extend(tail);
        }
    }
    gap(&mut out, pos, to);
    *runs = out;
    normalize_runs(runs, base);
}

/// The runs with every colour removed (the text takes the cell's text colour again).
pub(crate) fn runs_without_colors(runs: &[Run], base: Style) -> Vec<Run> {
    let mut out: Vec<Run> = runs
        .iter()
        .cloned()
        .map(|mut run| {
            run.color = None;
            run
        })
        .collect();
    normalize_runs(&mut out, base);
    out
}

/// Sets the colour of a run; a helper for [`runs_modify`].
pub(crate) fn set_run_color(color: Option<Color>) -> impl Fn(&mut Run) {
    move |run| run.color = color
}

fn same_target(a: &Link, b: &Link) -> bool {
    a.target == b.target && a.extra == b.extra
}

/// `n` code points were inserted at `pos`. They join a link they are strictly inside of,
/// or, with `at_end`, one that ends at `pos`.
pub(crate) fn links_insert(links: &mut [Link], pos: u32, n: u32, at_end: bool) {
    if n == 0 {
        return;
    }
    for link in links {
        let stop = end(link.start, link.len);
        if link.start < pos && (pos < stop || (at_end && pos == stop)) {
            link.len = link.len.saturating_add(n);
        } else if link.start >= pos {
            link.start = link.start.saturating_add(n);
        }
    }
}

/// The `n` code points from `pos` on were removed.
pub(crate) fn links_remove(links: &mut Vec<Link>, pos: u32, n: u32) {
    if n == 0 {
        return;
    }
    for link in links.iter_mut() {
        let start = after_removal(link.start, pos, n);
        let stop = after_removal(end(link.start, link.len), pos, n);
        link.start = start;
        link.len = stop.saturating_sub(start);
    }
    links.retain(|link| link.len > 0);
}

/// The `n` code points from `pos` on were replaced by `m` code points.
pub(crate) fn links_replace(links: &mut Vec<Link>, pos: u32, n: u32, m: u32) {
    if n == 0 {
        links_insert(links, pos, m, false);
    } else if m == 0 {
        links_remove(links, pos, n);
    } else {
        links_insert(links, pos.saturating_add(1), m, true);
        links_remove(links, pos, 1);
        links_remove(links, pos.saturating_add(m), n.saturating_sub(1));
    }
}

/// Links `from..to` (which must be inside the text) to `target` instead of whatever any of
/// it linked to; `None` unlinks it.
pub(crate) fn links_set(links: &mut Vec<Link>, from: u32, to: u32, target: Option<LinkTarget>) {
    if from >= to {
        return;
    }
    let mut out: Vec<Link> = Vec::with_capacity(links.len().saturating_add(2));
    fn add(out: &mut Vec<Link>, link: Link) {
        if link.len == 0 {
            return;
        }
        match out.last_mut() {
            Some(last) if end(last.start, last.len) == link.start && same_target(last, &link) => {
                last.len = last.len.saturating_add(link.len);
            }
            _ => out.push(link),
        }
    }
    let mut new = target.map(|target| Link {
        start: from,
        len: to.saturating_sub(from),
        target,
        extra: Extra::new(),
    });
    for link in links.drain(..) {
        let stop = end(link.start, link.len);
        if link.start < from {
            let mut head = link.clone();
            head.len = stop.min(from).saturating_sub(link.start);
            add(&mut out, head);
        }
        if stop > from {
            if let Some(new) = new.take() {
                add(&mut out, new);
            }
        }
        if stop > to {
            let mut tail = link;
            tail.start = tail.start.max(to);
            tail.len = stop.saturating_sub(tail.start);
            add(&mut out, tail);
        }
    }
    if let Some(new) = new.take() {
        add(&mut out, new);
    }
    *links = out;
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    #[test]
    fn control_characters_become_spaces() {
        assert_eq!(clean("a\nb\tc\u{0}d\u{1f}e f"), "a b c d e f");
        assert!(is_clean("a b\u{7f}\u{85}é😀"));
        assert!(!is_clean("a\rb"));
        assert_eq!(clean("\r\n").chars().count(), 2);
    }

    #[test]
    fn positions_are_code_points() {
        let text = "a😀é";
        assert_eq!(byte_offset(text, 0), Some(0));
        assert_eq!(byte_offset(text, 1), Some(1));
        assert_eq!(byte_offset(text, 2), Some(5));
        assert_eq!(byte_offset(text, 3), Some(text.len()));
        assert_eq!(byte_offset(text, 4), None);
        assert_eq!(byte_offset("", 0), Some(0));
        assert_eq!(byte_offset("", 1), None);
    }

    /// Each text with the positions where it may be cut.
    fn cases() -> Vec<(&'static str, Vec<u32>)> {
        vec![
            ("abc", vec![0, 1, 2, 3]),
            // e + combining acute, then x.
            ("e\u{301}x", vec![0, 2, 3]),
            // Family: man ZWJ woman ZWJ girl (5 code points), then a.
            (
                "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}a",
                vec![0, 5, 6],
            ),
            // Thumbs up + skin tone.
            ("\u{1F44D}\u{1F3FD}", vec![0, 2]),
            // Two flags: DE, FR. A cut between D and E would make two half flags.
            ("\u{1F1E9}\u{1F1EA}\u{1F1EB}\u{1F1F7}", vec![0, 2, 4]),
            // Hangul: conjoining jamo L V T form one syllable; a precomposed one after.
            ("\u{1100}\u{1161}\u{11A8}\u{D55C}", vec![0, 3, 4]),
            // Heart + variation selector 16.
            ("\u{2764}\u{FE0F}!", vec![0, 2, 3]),
            ("", vec![0]),
        ]
    }

    #[test]
    fn boundaries_are_between_user_perceived_characters() {
        for (text, allowed) in cases() {
            let len = text.chars().count() as u32;
            for pos in 0..=len {
                assert_eq!(
                    is_boundary(text, pos),
                    Some(allowed.contains(&pos)),
                    "{text:?} at {pos}"
                );
                let floor = *allowed.iter().rfind(|b| **b <= pos).unwrap();
                let ceil = *allowed.iter().find(|b| **b >= pos).unwrap();
                assert_eq!(floor_boundary(text, pos), floor, "{text:?} floor {pos}");
                assert_eq!(ceil_boundary(text, pos), ceil, "{text:?} ceil {pos}");
            }
            assert_eq!(is_boundary(text, len + 1), None);
            assert_eq!(floor_boundary(text, len + 7), len);
            assert_eq!(ceil_boundary(text, len + 7), len);
        }
    }
}
