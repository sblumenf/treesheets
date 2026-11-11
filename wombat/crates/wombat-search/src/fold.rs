//! Folding: the form in which text is compared (decision A72).
//!
//! Two texts match when their folded forms do. Folding
//!
//! - ignores letter case, by Unicode's full case mappings (`ß` and `SS`, `Σ`, `σ` and
//!   `ς`, and also `I`, `i`, `İ` and `ı` fold together),
//! - ignores compatibility forms (NFKD): full-width `Ａ` is `a`, half-width `ｶ` is `カ`,
//!   `ﬁ` is `fi`, `①` is `1`,
//! - ignores the accents of Latin and Greek letters where Unicode holds them as separate
//!   marks: `é`, `e` + U+0301 and `e` are the same; `ø` and `ł` are letters of their own.
//!   Other scripts keep their marks (Japanese `が` is not `か`, Russian `й` is not `и`),
//!   in their composed form (NFC),
//! - turns line breaks, tabs, other control characters and every kind of space into one
//!   space each, and drops soft hyphens, zero-width spaces and variation selectors.
//!
//! Text is folded one user-perceived character (extended grapheme cluster) at a time, so
//! every piece of folded text belongs to one whole character of the original; that is
//! how a match is turned back into a range that never cuts a character ([`FoldMap`]).

use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Samples that go through every rule above, for the cache fingerprint: an index cached
/// under other folding rules must not be believed ([`fingerprint`]).
const SAMPLES: &str = "Straße ΣΊΣΥΦΟΣ ς İstanbul ıI ＡＢｃ ｶﾞ ﬁ ① é e\u{301} ø й が か\u{3099} 한글 \
                       中文 a\tb\u{a0}c\u{3000}d so\u{ad}ft z\u{200b}w ❤\u{fe0f} 👨\u{200d}👩 Ǆ ǅ ǆ ẞ ΐ ŉ";

/// A number that changes if folding or the word rules change for any of the samples.
pub fn fingerprint() -> u64 {
    let mut text = fold(SAMPLES);
    for c in SAMPLES.chars() {
        text.push(if is_word_char(c) { 'w' } else { '-' });
        text.push(if is_wordless(c) { 'c' } else { '-' });
    }
    wombat_core::summary::digest(text.as_bytes()) as u64
}

fn ascii(byte: u8) -> u8 {
    match byte {
        b'A'..=b'Z' => byte.to_ascii_lowercase(),
        0..=0x1f | 0x7f => b' ',
        _ => byte,
    }
}

/// Characters that fold to themselves whatever surrounds them: kana, CJK ideographs and
/// Hangul syllables. Only a shortcut; the long way gives the same.
fn plain(c: char) -> bool {
    matches!(u32::from(c), 0x3041..=0x3096 | 0x30a1..=0x30fa | 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xac00..=0xd7a3)
}

fn dropped(c: char) -> bool {
    matches!(
        u32::from(c),
        0xad | 0x200b | 0x2060 | 0xfeff | 0xfe00..=0xfe0f
    )
}

/// Latin and Greek letters, whose accents are ignored.
fn loses_accents(base: char) -> bool {
    let c = u32::from(base);
    c < 0x400 || (0x1e00..=0x1fff).contains(&c)
}

fn case_fold(c: char, out: &mut Vec<char>) {
    // Lower, upper, lower: the upper case brings `ß` to `SS` and `ς` to `Σ`; the first
    // lower case brings `ẞ` to `ß` so that it goes the same way.
    for lower in c.to_lowercase() {
        for upper in lower.to_uppercase() {
            out.extend(upper.to_lowercase());
        }
    }
}

/// The folded forms of the single characters from U+0080 up to here, worked out once:
/// the letters of most alphabets, which would otherwise go the long way each time.
const KNOWN: u32 = 0x3100;

fn known(c: char) -> Option<&'static str> {
    static TABLE: OnceLock<Vec<Box<str>>> = OnceLock::new();
    let at = u32::from(c)
        .checked_sub(0x80)
        .filter(|_| u32::from(c) < KNOWN)?;
    let table = TABLE.get_or_init(|| {
        let of = |code: u32| {
            let mut out = String::new();
            if let Some(c) = char::from_u32(code) {
                fold_long(c.encode_utf8(&mut [0; 4]), &mut out);
            }
            out.into_boxed_str()
        };
        (0x80..KNOWN).map(of).collect()
    });
    table.get(at as usize).map(|folded| &**folded)
}

/// Folds one user-perceived character.
fn fold_cluster(cluster: &str, out: &mut String) {
    if let [byte] = cluster.as_bytes() {
        out.push(char::from(ascii(*byte)));
        return;
    }
    let mut chars = cluster.chars();
    if let (Some(only), None) = (chars.next(), chars.next()) {
        if plain(only) {
            out.push(only);
            return;
        }
        if let Some(folded) = known(only) {
            out.push_str(folded);
            return;
        }
    }
    fold_long(cluster, out);
}

/// Folds one user-perceived character by the rules, with no shortcut.
fn fold_long(cluster: &str, out: &mut String) {
    let mut folded = Vec::new();
    for c in cluster.nfkd() {
        case_fold(c, &mut folded);
    }
    let mut base = None;
    let kept = folded.into_iter().nfkd().filter(|c| {
        let base = *base.get_or_insert(*c);
        let accent = (0x300..=0x36f).contains(&u32::from(*c)) && loses_accents(base);
        !accent && !dropped(*c)
    });
    for c in kept.nfc() {
        out.push(if c.is_control() || c.is_whitespace() {
            ' '
        } else {
            c
        });
    }
}

/// Appends the folded form of `text` to `out`.
pub fn fold_into(text: &str, out: &mut String) {
    let bytes = text.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        // A stretch of plain characters each followed by another one (or by the end):
        // each is a user-perceived character by itself, whatever comes before it.
        let plain = |n: usize| bytes.get(n).is_none_or(u8::is_ascii);
        let mut end = at;
        while end < bytes.len() && plain(end) && plain(end + 1) {
            end += 1;
        }
        out.extend(
            bytes
                .get(at..end)
                .unwrap_or_default()
                .iter()
                .map(|byte| char::from(ascii(*byte))),
        );
        // Then up to the next such stretch, one user-perceived character at a time.
        let mut next = end;
        while next < bytes.len() && !(plain(next) && plain(next + 1)) {
            next += 1;
        }
        // (Not inside a character: a stretch ends before a plain character.)
        for cluster in text.get(end..next).unwrap_or_default().graphemes(true) {
            fold_cluster(cluster, out);
        }
        if next == at {
            // Cannot happen: one of the two loops passes any character. It keeps a
            // mistake here from looping for ever.
            break;
        }
        at = next;
    }
}

/// The folded form of `text`.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    fold_into(text, &mut out);
    out
}

/// A text folded, with what is needed to turn a place in the folded text back into a
/// range of the original, in code points, that holds whole characters only.
#[derive(Clone, Debug)]
pub struct FoldMap {
    /// The folded text.
    pub folded: String,
    /// For each user-perceived character of the original: where its folded form ends
    /// (in bytes of `folded`), and where it starts and how long it is (in code points).
    clusters: Vec<(u32, u32, u32)>,
}

impl FoldMap {
    /// Folds `text`.
    pub fn new(text: &str) -> FoldMap {
        let mut folded = String::with_capacity(text.len());
        let mut clusters = Vec::new();
        let mut at = 0u32;
        for cluster in text.graphemes(true) {
            fold_cluster(cluster, &mut folded);
            let len = cluster.chars().count() as u32;
            clusters.push((folded.len() as u32, at, len));
            at = at.saturating_add(len);
        }
        FoldMap { folded, clusters }
    }

    /// `text` as it is, not folded, for turning a place in it into a range of whole
    /// characters in the same way.
    pub fn literal(text: &str) -> FoldMap {
        let mut clusters = Vec::new();
        let mut at = 0u32;
        for (byte, cluster) in text.grapheme_indices(true) {
            let len = cluster.chars().count() as u32;
            clusters.push(((byte + cluster.len()) as u32, at, len));
            at = at.saturating_add(len);
        }
        FoldMap {
            folded: text.to_owned(),
            clusters,
        }
    }

    /// The range of the original, as start and length in code points, made of the
    /// characters whose folded forms the bytes `from..to` of the folded text touch.
    pub fn range(&self, from: usize, to: usize) -> (u32, u32) {
        // The first character whose folded form ends after `from`.
        let first = self.clusters.partition_point(|c| c.0 as usize <= from);
        // The first character whose folded form ends at or after `to`.
        let last = self.clusters.partition_point(|c| (c.0 as usize) < to);
        let start = self.clusters.get(first).map_or(0, |c| c.1);
        let end = match self.clusters.get(last.max(first)) {
            Some(c) => c.1.saturating_add(c.2),
            None => self.clusters.last().map_or(0, |c| c.1.saturating_add(c.2)),
        };
        (start, end.saturating_sub(start))
    }

    /// Whether the bytes `from..to` of the folded text are the whole folded forms of
    /// the characters that [`FoldMap::range`] gives for them: the match neither begins
    /// nor ends inside the folded form of one character.
    pub fn is_whole(&self, from: usize, to: usize) -> bool {
        let first = self.clusters.partition_point(|c| c.0 as usize <= from);
        let starts_at = first
            .checked_sub(1)
            .and_then(|before| self.clusters.get(before))
            .map_or(0, |c| c.0 as usize);
        let last = self.clusters.partition_point(|c| (c.0 as usize) < to);
        let ends_at = self.clusters.get(last).map(|c| c.0 as usize);
        from < to && starts_at == from && ends_at == Some(to)
    }
}

/// Whether `c` is part of a word: a letter or a digit of any script.
pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// Whether `c` belongs to a script that is written without spaces between words
/// (Chinese, Japanese, Thai, Lao, Khmer, Burmese): each such character counts as a word
/// of its own, so that a match anywhere in such text is as good as a match at the start
/// of a word.
pub fn is_wordless(c: char) -> bool {
    matches!(
        u32::from(c),
        0x0e00..=0x0eff | 0x1000..=0x109f | 0x1780..=0x17ff | 0x2e80..=0x9fff | 0xf900..=0xfaff
            | 0xff66..=0xff9f | 0x20000..=0x3ffff
    )
}

/// Whether a word starts at the byte `at` of `text` (which must be a character boundary).
pub fn word_starts(text: &str, at: usize) -> bool {
    let before = text.get(..at).and_then(|t| t.chars().next_back());
    let after = text.get(at..).and_then(|t| t.chars().next());
    match (before, after) {
        (None, _) => true,
        (Some(before), after) => {
            !is_word_char(before) || is_wordless(before) || after.is_some_and(is_wordless)
        }
    }
}

/// Whether a word ends at the byte `at` of `text` (which must be a character boundary).
pub fn word_ends(text: &str, at: usize) -> bool {
    let before = text.get(..at).and_then(|t| t.chars().next_back());
    let after = text.get(at..).and_then(|t| t.chars().next());
    match (before, after) {
        (_, None) => true,
        (before, Some(after)) => {
            !is_word_char(after) || is_wordless(after) || before.is_some_and(is_wordless)
        }
    }
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
    fn folding_ignores_case_width_and_latin_accents() {
        let same = [
            // Case, by the full mappings.
            ("Apple", "apple"),
            ("STRASSE", "straße"),
            ("Straße", "strasse"),
            ("ẞ", "ss"),
            ("ΣΊΣΥΦΟΣ", "σισυφος"),
            ("Σίσυφος", "σισυφοσ"),
            // Turkish: all four i's are one letter here.
            ("İstanbul", "istanbul"),
            ("ISTANBUL", "ıstanbul"),
            ("ı", "i"),
            ("Ǆ", "ǆ"),
            ("ǅ", "ǆ"),
            ("ŉ", "ʼn"),
            // Accents of Latin and Greek letters, composed or not.
            ("é", "e"),
            ("e\u{301}", "e"),
            ("Ö", "o"),
            ("naïve café", "naive cafe"),
            ("ΐ", "ι"),
            ("ǰ", "j"),
            ("ṩ", "s"),
            // Width and other compatibility forms.
            ("ＡＢＣ１２３", "abc123"),
            ("ｶﾞ", "ガ"),
            ("ｱｲｳ", "アイウ"),
            ("ﬁ", "fi"),
            ("①", "1"),
            ("㎏", "kg"),
            ("x²", "x2"),
            // Spaces and control characters are one space each; some things vanish.
            ("a\tb\nc\r\nd", "a b c  d"),
            ("a\u{a0}b\u{3000}c\u{2003}d", "a b c d"),
            ("so\u{ad}ft", "soft"),
            ("zero\u{200b}width", "zerowidth"),
            ("❤\u{fe0f}", "❤"),
            // Composed and decomposed forms of other scripts are one.
            ("か\u{3099}", "が"),
            ("\u{1112}\u{1161}\u{11ab}", "한"),
            ("й", "и\u{306}"),
        ];
        for (text, other) in same {
            assert_eq!(fold(text), fold(other), "{text:?} and {other:?}");
        }
        let different = [
            ("ø", "o"),
            ("ł", "l"),
            ("が", "か"),
            ("й", "и"),
            ("ё", "е"),
            ("は", "ば"),
            ("한", "하"),
            ("中", "申"),
            ("a b", "ab"),
            ("👍", "👎"),
            ("👍🏽", "👍"),
        ];
        for (text, other) in different {
            assert_ne!(fold(text), fold(other), "{text:?} and {other:?}");
        }
        // What comes out, exactly.
        assert_eq!(fold("Größe ＸＹ ｶﾞ"), "grosse xy ガ");
        assert_eq!(fold("日本語のテキスト"), "日本語のテキスト");
        assert_eq!(fold("한국어 텍스트"), "한국어 텍스트");
        assert_eq!(
            fold("👨\u{200d}👩\u{200d}👧 ok"),
            "👨\u{200d}👩\u{200d}👧 ok"
        );
        assert_eq!(fold(""), "");
        assert_eq!(fold("\u{301}"), "", "a lone accent is nothing");
    }

    #[test]
    fn folding_a_folded_character_changes_nothing_and_shortcuts_agree() {
        let mut worst = 0;
        for c in (0..=0x10ffffu32).filter_map(char::from_u32) {
            let text = c.to_string();
            let once = fold(&text);
            assert_eq!(fold(&once), once, "U+{:04X}", u32::from(c));
            // The shortcuts for plain characters and plain text give what the long way gives.
            let mut long = String::new();
            fold_cluster(&format!("{c}\u{301}"), &mut long);
            if plain(c) {
                assert_eq!(once, text);
                assert_eq!(long, format!("{c}\u{301}"));
            }
            if c.is_ascii() {
                assert_eq!(once.len(), 1);
                assert_eq!(
                    fold(&format!("{c}é")),
                    format!("{once}e"),
                    "U+{:04X}",
                    u32::from(c)
                );
            }
            assert!(!once.contains('\n') && !once.contains(char::is_control));
            worst = worst.max(once.len());
        }
        // One character never becomes a long text (ﷺ is the longest).
        assert!(worst <= 72, "{worst}");
    }

    #[test]
    fn a_place_in_folded_text_goes_back_to_whole_characters() {
        let text = "Aé👨\u{200d}👩ß e\u{301}ｶﾞx";
        let map = FoldMap::new(text);
        assert_eq!(map.folded, fold(text));
        assert_eq!(map.folded, "ae👨\u{200d}👩ss eガx");
        let at = |needle: &str| {
            let from = map.folded.find(needle).unwrap();
            map.range(from, from + needle.len())
        };
        assert_eq!(at("a"), (0, 1));
        assert_eq!(at("e"), (1, 1));
        assert_eq!(at("👨"), (2, 3), "part of an emoji is the whole emoji");
        assert_eq!(at("ss"), (5, 1));
        assert_eq!(
            at("s e"),
            (5, 4),
            "half of ß is ß, and é is two code points"
        );
        assert_eq!(at("ガ"), (9, 2));
        assert_eq!(at("x"), (11, 1));
        assert_eq!(map.range(0, map.folded.len()), (0, 12));
        let literal = FoldMap::literal(text);
        let from = text.find('\u{200d}').unwrap();
        assert_eq!(literal.range(from, from + 3), (2, 3));
        assert_eq!(literal.range(0, 1), (0, 1));
    }

    /// Folding a text in stretches gives what folding it one user-perceived character
    /// at a time gives, whatever meets at the seams.
    #[test]
    fn the_shortcuts_for_plain_text_change_nothing() {
        let pieces = [
            "a", "B", " ", "\r", "\n", "\t", "é", "e", "\u{301}", "\u{308}", "ß", "İ", "\u{600}",
            "\u{200d}", "👨", "\u{fe0f}", "ｶ", "\u{ff9e}", "か", "\u{3099}", "日", "한",
            "\u{1100}", "\u{1161}", "\u{11a8}", "й", "Σ", "ς", "\u{ad}", "🇩", "🇪", "\u{e01}",
            "\u{e33}", "x",
        ];
        let mut seed = 0x1234_5678_9abc_def1u64;
        let mut next = |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        for _ in 0..20_000 {
            let text: String = (0..next(9)).map(|_| pieces[next(pieces.len())]).collect();
            let map = FoldMap::new(&text);
            assert_eq!(fold(&text), map.folded, "{text:?}");
            let mut long = String::new();
            for cluster in text.graphemes(true) {
                fold_long(cluster, &mut long);
            }
            assert_eq!(map.folded, long, "{text:?}");
        }
    }

    #[test]
    fn words_and_scripts_without_spaces() {
        let text = "one two日本語 x";
        let starts: Vec<usize> = (0..=text.len())
            .filter(|at| text.is_char_boundary(*at) && word_starts(text, *at))
            .collect();
        // Before o and t, before each of 日 本 語, after 語, and before x.
        assert_eq!(starts, [0, 4, 7, 10, 13, 16, 17]);
        assert!(word_ends(text, 3) && !word_ends(text, 2) && word_ends(text, 7));
        assert!(word_ends(text, 10) && word_ends(text, text.len()));
        assert!(is_wordless('日') && is_wordless('の') && is_wordless('ก') && !is_wordless('한'));
        assert!(is_word_char('é') && is_word_char('한') && is_word_char('٣') && !is_word_char('-'));
        // The fingerprint is a number that stays the same from run to run.
        assert_eq!(fingerprint(), fingerprint());
    }
}
