//! `text::difference`: the smallest replacement that turns one text into another without
//! cutting a user-perceived character of either. Compared with a search through every
//! replacement there is.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use wombat_core::text::{byte_offset, difference, is_boundary};

/// `old` with `len` code points at `start` replaced by `ins`.
fn replaced(old: &str, start: u32, len: u32, ins: &str) -> String {
    let from = byte_offset(old, start).unwrap();
    let to = byte_offset(old, start + len).unwrap();
    format!("{}{ins}{}", &old[..from], &old[to..])
}

fn count(text: &str) -> u32 {
    text.chars().count() as u32
}

/// Every replacement that turns `old` into `new` and cuts no user-perceived character of
/// either, as (start, length, text put in): found by trying every range of `old`.
fn allowed(old: &str, new: &str) -> Vec<(u32, u32, String)> {
    let mut found = Vec::new();
    for start in 0..=count(old) {
        for len in 0..=count(old) - start {
            let (head, tail) = (
                &old[..byte_offset(old, start).unwrap()],
                &old[byte_offset(old, start + len).unwrap()..],
            );
            if head.len() + tail.len() > new.len() || !new.starts_with(head) || !new.ends_with(tail)
            {
                continue;
            }
            let ins = &new[head.len()..new.len() - tail.len()];
            let cuts_none = is_boundary(old, start) == Some(true)
                && is_boundary(old, start + len) == Some(true)
                && is_boundary(new, start) == Some(true)
                && is_boundary(new, start + count(ins)) == Some(true);
            if cuts_none {
                found.push((start, len, ins.to_string()));
            }
        }
    }
    found
}

/// What "smallest" means: of the allowed replacements, the one that starts latest, and of
/// those that start there the one that replaces least.
fn smallest(old: &str, new: &str) -> Option<(u32, u32, String)> {
    if old == new {
        return None;
    }
    allowed(old, new)
        .into_iter()
        .max_by_key(|(start, len, _)| (*start, std::cmp::Reverse(*len)))
}

fn some(start: u32, len: u32, ins: &str) -> Option<(u32, u32, String)> {
    Some((start, len, ins.to_string()))
}

#[test]
fn the_difference_is_the_part_that_changed() {
    let d = difference;
    assert_eq!(d("", ""), None);
    assert_eq!(d("abc", "abc"), None);
    assert_eq!(d("", "abc"), some(0, 0, "abc"));
    assert_eq!(d("abc", ""), some(0, 3, ""));
    assert_eq!(d("abc", "abXc"), some(2, 0, "X"));
    assert_eq!(d("abXc", "abc"), some(2, 1, ""));
    assert_eq!(d("the cat sat", "the dog sat"), some(4, 3, "dog"));
    assert_eq!(d("abc", "xyz"), some(0, 3, "xyz"));
    // Positions are code points, not bytes.
    assert_eq!(d("日本語", "日本x語"), some(2, 0, "x"));
    // Where a character repeats, the start is kept as long as can be: the new one is the
    // last, and of two the last is the one that went.
    assert_eq!(d("aa", "aaa"), some(2, 0, "a"));
    assert_eq!(d("aaa", "aa"), some(2, 1, ""));
    assert_eq!(d("abab", "ab"), some(2, 2, ""));
}

#[test]
fn the_difference_cuts_no_user_perceived_character() {
    let d = difference;
    // An accent typed onto the e: the e is replaced with it, not cut from it.
    assert_eq!(d("ex", "e\u{301}x"), some(0, 1, "e\u{301}"));
    assert_eq!(d("e\u{301}x", "ex"), some(0, 2, "e"));
    // Another accent on the same letter: the whole character.
    assert_eq!(d("ae\u{301}x", "ae\u{300}x"), some(1, 2, "e\u{300}"));
    // Flags: Germany and France, of which the first goes; then France becomes "DE" + R.
    let (de, fr) = ("\u{1F1E9}\u{1F1EA}", "\u{1F1EB}\u{1F1F7}");
    assert_eq!(d(&format!("{de}{fr}"), fr), some(0, 2, ""));
    assert_eq!(
        d(&format!("{de}{fr}"), &format!("{de}\u{1F1F7}")),
        some(2, 2, "\u{1F1F7}")
    );
    // One regional indicator more before a run of them shifts every flag of the run: the
    // common end is not cut at the same places in both, so all of it is replaced.
    let run = "\u{1F1E9}".repeat(5);
    let longer = format!("\u{1F1E9}{run}");
    assert_eq!(d(&format!("a{run}"), &longer), some(0, 6, &longer));
    assert_eq!(d(&format!("a{run}"), &run), some(0, 1, ""));
    // A skin tone added to a thumb; a joiner between two people.
    assert_eq!(
        d("a\u{1F44D}b", "a\u{1F44D}\u{1F3FD}b"),
        some(1, 1, "\u{1F44D}\u{1F3FD}")
    );
    assert_eq!(
        d("\u{1F468}\u{1F469}", "\u{1F468}\u{200D}\u{1F469}"),
        some(0, 2, "\u{1F468}\u{200D}\u{1F469}")
    );
}

#[test]
fn the_difference_of_texts_with_line_breaks() {
    let d = difference;
    assert_eq!(d("ab", "a\nb"), some(1, 0, "\n"));
    assert_eq!(d("a\nb", "ab"), some(1, 1, ""));
    assert_eq!(d("a\nb", "a\n\nb"), some(2, 0, "\n"));
    assert_eq!(d("one\ntwo\nthree", "one\n2\nthree"), some(4, 3, "2"));
    assert_eq!(d("a\nb", "a b"), some(1, 1, " "));
    // An accent after a line break stands by itself, so the break is not touched.
    assert_eq!(d("a\nb", "a\n\u{301}b"), some(2, 0, "\u{301}"));
    // A carriage return with a line feed is one character (cell text has none, but any
    // two texts must be told apart rightly).
    assert_eq!(d("a\r\nb", "a\rb"), some(1, 2, "\r"));
    assert_eq!(d("a\nb", "a\r\nb"), some(1, 1, "\r\n"));
}

/// Flags all through, one indicator fewer: every cut of the common end differs, which is
/// the worst there is for finding it. Must take time in proportion to the text, not to
/// its square (at this size the square is hours).
#[test]
fn a_long_run_of_flags_is_told_apart_in_time() {
    let old = format!("a{}", "\u{1F1E9}".repeat(200_001));
    let new = &old[..old.len() - 4];
    // The text before the change is common, and so it is kept: the last flag goes, and
    // half a flag is what the one before it becomes.
    assert_eq!(difference(&old, new), some(200_001, 1, ""));
    let shifted = format!("\u{1F1E9}{}", &old[1..]);
    assert_eq!(difference(&old, &shifted), some(0, 200_002, &shifted));
    // One huge character (a letter with 200,000 accents) that loses its last accent.
    let accents = format!("a{}", "\u{301}".repeat(200_000));
    let fewer = &accents[..accents.len() - 2];
    assert_eq!(difference(&accents, fewer), some(0, 200_001, fewer));
}

/// Code points that join into user-perceived characters in many ways, and line breaks.
const PIECES: &[&str] = &[
    "a",
    "a",
    "b",
    " ",
    "\n",
    "\r",
    "e",
    "\u{301}",
    "\u{1F1E9}",
    "\u{1F1E9}",
    "\u{1F1EA}",
    "\u{200D}",
    "\u{1F468}",
    "\u{1F469}",
    "\u{1F3FD}",
    "\u{2764}",
    "\u{FE0F}",
    "\u{1100}",
    "\u{1161}",
    "\u{11A8}",
    "\u{D55C}",
];

fn pieces(len: std::ops::Range<usize>) -> impl Strategy<Value = Vec<&'static str>> {
    proptest::collection::vec(proptest::sample::select(PIECES), len)
}

/// Two texts: unrelated ones, or the second made from the first by one change somewhere,
/// as an editor makes it.
fn pair() -> impl Strategy<Value = (String, String)> {
    let unrelated = (pieces(0..8), pieces(0..8)).prop_map(|(a, b)| (a.concat(), b.concat()));
    let edited =
        (pieces(0..12), pieces(0..4), any::<(usize, usize)>()).prop_map(|(old, ins, (at, len))| {
            let at = at % (old.len() + 1);
            let len = len % (old.len() - at + 1);
            let mut new = old.clone();
            new.splice(at..at + len, ins);
            (old.concat(), new.concat())
        });
    prop_oneof![1 => unrelated, 3 => edited]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4000))]

    #[test]
    fn the_difference_is_right_allowed_and_smallest((old, new) in pair()) {
        let found = difference(&old, &new);
        prop_assert_eq!(found.is_none(), old == new, "none exactly for equal texts");
        if let Some((start, len, ins)) = &found {
            prop_assert_eq!(replaced(&old, *start, *len, ins), new.clone());
            prop_assert!(*len > 0 || !ins.is_empty(), "something is replaced");
            prop_assert_eq!(is_boundary(&old, *start), Some(true));
            prop_assert_eq!(is_boundary(&old, start + len), Some(true));
            prop_assert_eq!(is_boundary(&new, *start), Some(true));
            prop_assert_eq!(is_boundary(&new, start + count(ins)), Some(true));
        }
        prop_assert_eq!(found, smallest(&old, &new));
    }

    /// The other way round it leads back, from the same place.
    #[test]
    fn the_difference_and_its_reverse((old, new) in pair()) {
        if let Some((start, len, ins)) = difference(&old, &new) {
            let from = byte_offset(&old, start).unwrap();
            let to = byte_offset(&old, start + len).unwrap();
            prop_assert_eq!(difference(&new, &old), some(start, count(&ins), &old[from..to]));
        }
    }
}
