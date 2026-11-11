//! Find and replace: what is replaced, that one call is one undo step, and that undoing
//! it gives the identical file back.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use proptest::prelude::*;
use wombat_core::construct::Construct;
use wombat_core::CellKind;
use wombat_core::{format, CellId, Command, Document, Editor, Error, LinkTarget, SeededIds, Style};
use wombat_search::query::{parse, Term};
use wombat_search::replace::{places, plan, replace, ReplaceOptions, Scope};
use wombat_search::{Options, SearchIndex};

/// A document whose root has one column of cells with these texts; the first cell has a
/// column of its own with `inner`.
fn document(texts: &[&str], inner: &[&str]) -> (Document, Vec<CellId>, Vec<CellId>) {
    let mut ids = SeededIds::from_u64(3);
    let mut doc = Document::new(&mut ids);
    let mut fill = |doc: &mut Document, cell: CellId, texts: &[&str]| {
        if texts.is_empty() {
            return Vec::new();
        }
        doc.add_grid(cell, 1, texts.len() as u32, &mut ids).unwrap();
        let cells = doc.cell(cell).unwrap().grid().unwrap().cells().to_vec();
        for (id, text) in cells.iter().zip(texts) {
            doc.cell_mut(*id).unwrap().text = text.to_string();
        }
        cells
    };
    let root = doc.root();
    let outer = fill(&mut doc, root, texts);
    let inner = fill(&mut doc, outer[0], inner);
    doc.validate().unwrap();
    (doc, outer, inner)
}

fn texts(doc: &Document) -> Vec<String> {
    let order = doc.cells_in_order();
    order
        .iter()
        .map(|id| doc.cell(*id).unwrap().text.clone())
        .collect()
}

#[test]
fn every_place_is_replaced_in_one_step_that_undo_takes_back_exactly() {
    let (mut doc, outer, _) = document(
        &[
            "Apple and apple",
            "pineAPPLE",
            "pear",
            "Äpple",
            "ＡＰＰＬＥ!",
        ],
        &["apple", "x"],
    );
    doc.cell_mut(outer[2]).unwrap().note = "an apple a day".into();
    doc.cell_mut(doc.root()).unwrap().text = "apple root".into();
    let before = format::to_string(&doc).unwrap();
    let mut editor = Editor::new(doc).unwrap();
    let mut ids = SeededIds::from_u64(1);
    let mut index = SearchIndex::new();
    index.insert_document(editor.document(), editor.calc(), "d.wombat");
    // An earlier step, to see that undo takes back the replacing and no more.
    let style = Command::SetStyle {
        cells: vec![outer[2]],
        set: Style {
            bold: true,
            ..Style::default()
        },
        clear: Style::default(),
    };
    let changes = editor.apply(style, None, &mut ids).unwrap();
    index
        .apply(editor.document(), editor.calc(), &changes)
        .unwrap();
    let styled = format::to_string(editor.document()).unwrap();

    let done = replace(
        &mut editor,
        &Term::new("apple"),
        "Fig",
        Scope::Document,
        ReplaceOptions::default(),
        Some(7),
        &mut ids,
    )
    .unwrap();
    assert_eq!((done.counts.cells, done.counts.places), (6, 7));
    assert_eq!(
        texts(editor.document()),
        // Whatever the case, accents or width of what was there; the replacement as
        // written. The root's text too (TreeSheets leaves the cell it starts from).
        [
            "Fig root",
            "Fig and Fig",
            "Fig",
            "x",
            "pineFig",
            "pear",
            "Fig",
            "Fig!"
        ]
    );
    // Notes are not touched, as in TreeSheets.
    assert_eq!(
        editor.document().cell(outer[2]).unwrap().note,
        "an apple a day"
    );
    assert_eq!(editor.document().cell(outer[0]).unwrap().last_edit, Some(7));
    assert_eq!(editor.document().cell(outer[2]).unwrap().last_edit, None);
    assert_eq!(done.changes.changed.len(), 6);
    // The search index follows the report it was given.
    index
        .apply(editor.document(), editor.calc(), &done.changes)
        .unwrap();
    let mut fresh = SearchIndex::new();
    fresh.insert_document(editor.document(), editor.calc(), "d.wombat");
    assert_eq!(index.encoded(), fresh.encoded());
    let count = |index: &SearchIndex, text: &str| {
        let query = parse(text, 0).unwrap().query;
        index.search(&query, &Options::default()).unwrap().total
    };
    assert_eq!((count(&index, "fig"), count(&index, "apple")), (6, 1));
    // One undo: everything back, to the byte. One redo: replaced again.
    assert_eq!(editor.undo_len(), 2);
    let back = editor.undo().unwrap().unwrap();
    index
        .apply(editor.document(), editor.calc(), &back)
        .unwrap();
    assert_eq!(format::to_string(editor.document()).unwrap(), styled);
    assert_eq!(count(&index, "apple"), 7);
    editor.redo().unwrap().unwrap();
    assert_eq!(texts(editor.document())[1], "Fig and Fig");
    editor.undo().unwrap();
    editor.undo().unwrap();
    assert_eq!(format::to_string(editor.document()).unwrap(), before);
}

#[test]
fn case_counts_only_when_asked_and_scopes_limit_where() {
    let (doc, outer, inner) = document(&["Apple apple", "apple", "APPLE"], &["apple", "Apple"]);
    let run = |term: Term, scope: Scope| {
        let mut editor = Editor::new(doc.clone()).unwrap();
        let done = replace(
            &mut editor,
            &term,
            "kiwi",
            scope,
            ReplaceOptions::default(),
            None,
            &mut SeededIds::from_u64(1),
        )
        .unwrap();
        (texts(editor.document()), done.counts.places)
    };
    assert_eq!(
        run(Term::exact("apple"), Scope::Document),
        (
            ["", "Apple kiwi", "kiwi", "Apple", "kiwi", "APPLE"]
                .map(String::from)
                .to_vec(),
            3
        )
    );
    // Within a branch: the cell and what is inside it.
    assert_eq!(
        run(Term::new("apple"), Scope::Branch(outer[0])).0,
        ["", "kiwi kiwi", "kiwi", "kiwi", "apple", "APPLE"]
    );
    // Only these cells (TreeSheets' "replace in current selection").
    assert_eq!(
        run(Term::new("apple"), Scope::Cells(&[inner[1], outer[2]])),
        (
            ["", "Apple apple", "apple", "kiwi", "apple", "kiwi"]
                .map(String::from)
                .to_vec(),
            2
        )
    );
    // Nothing to find: nothing done, and nothing in the history.
    let mut editor = Editor::new(doc.clone()).unwrap();
    let mut ids = SeededIds::from_u64(1);
    for term in [
        Term::new("zebra"),
        Term::new(""),
        Term::exact(""),
        Term::new("\u{301}"),
    ] {
        let done = replace(
            &mut editor,
            &term,
            "x",
            Scope::Document,
            ReplaceOptions::default(),
            Some(1),
            &mut ids,
        )
        .unwrap();
        assert_eq!(
            (done.counts.cells, done.counts.places, editor.undo_len()),
            (0, 0, 0)
        );
        assert!(done.changes.is_empty());
    }
    assert!(*editor.document() == doc);
    let gone = CellId::parse("00000000-0000-0000-0000-00000000dead").unwrap();
    let done = replace(
        &mut editor,
        &Term::new("a"),
        "x",
        Scope::Branch(gone),
        ReplaceOptions::default(),
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(done.counts.places, 0);
    // Replacing something by itself is nothing either.
    let done = replace(
        &mut editor,
        &Term::exact("apple"),
        "apple",
        Scope::Document,
        ReplaceOptions::default(),
        Some(1),
        &mut ids,
    )
    .unwrap();
    assert_eq!((done.counts.places, editor.undo_len()), (3, 0));
}

#[test]
fn places_are_whole_characters_from_left_to_right_and_never_overlap() {
    let fold = |text: &str, word: &str| places(text, &Term::new(word)).whole;
    let part = |text: &str, word: &str| places(text, &Term::new(word)).partial;
    assert_eq!(fold("aaaa", "aa"), [(0, 2), (2, 2)]);
    assert_eq!(fold("aaa", "aa"), [(0, 2)]);
    assert_eq!(fold("Straße", "ss"), [(4, 1)]);
    // (This test used to say that `s` is found in the `ß` as a place to replace: that
    // was the fault. Half a letter is not a place.)
    assert_eq!(fold("Straße", "s"), [(0, 1)]);
    assert_eq!(part("Straße", "s"), [(4, 1)], "ß is s twice: one place");
    assert_eq!(fold("e\u{301}e", "e"), [(0, 2), (2, 1)]);
    assert_eq!(fold("ｶﾞｶﾞ", "ガ"), [(0, 2), (2, 2)]);
    assert_eq!(fold("so\u{ad}ft", "soft"), [(0, 5)]);
    assert_eq!(fold("x\u{ad}soft\u{ad}y", "soft"), [(2, 4)]);
    assert_eq!(fold("x", ""), []);
    // A whole match that starts inside a partial one is still found.
    assert_eq!(fold("sß", "ss"), [(1, 1)]);
    assert_eq!(
        part("sß", "ss"),
        [],
        "what overlaps a replaced place is not listed"
    );
    assert_eq!((fold("ßs", "ss"), part("ßs", "ss")), (vec![(0, 1)], vec![]));
    let exact = |text: &str, word: &str| places(text, &Term::exact(word));
    assert_eq!(exact("Aa", "a").whole, [(1, 1)]);
    // Part of a character is not the character.
    let accent = exact("e\u{301}e", "e");
    assert_eq!((accent.whole, accent.partial), (vec![(2, 1)], vec![(0, 2)]));
    let pair = exact("👨\u{200d}👩", "👩");
    assert_eq!((pair.whole, pair.partial), (vec![], vec![(0, 3)]));
    // The replacement is not searched again: this ends.
    let (doc, outer, _) = document(&["aXa"], &[]);
    let mut editor = Editor::new(doc).unwrap();
    replace(
        &mut editor,
        &Term::new("a"),
        "aa",
        Scope::Document,
        ReplaceOptions::default(),
        None,
        &mut SeededIds::from_u64(1),
    )
    .unwrap();
    assert_eq!(editor.document().cell(outer[0]).unwrap().text, "aaXaa");
}

#[test]
fn styles_and_links_of_replaced_text_follow_the_rules_of_text_editing() {
    let (doc, outer, _) = document(&["one apple two"], &[]);
    let mut editor = Editor::new(doc).unwrap();
    let mut ids = SeededIds::from_u64(1);
    let bold = Style {
        bold: true,
        ..Style::default()
    };
    let setup = [
        Command::SetRangeStyle {
            cell: outer[0],
            start: 4,
            len: 5,
            set: bold,
            clear: Style::default(),
        },
        Command::SetLink {
            cell: outer[0],
            start: 2,
            len: 5,
            target: Some(LinkTarget::External("https://example.com".into())),
        },
    ];
    for command in setup {
        editor.apply(command, None, &mut ids).unwrap();
    }
    replace(
        &mut editor,
        &Term::new("apple"),
        "fig",
        Scope::Document,
        ReplaceOptions::default(),
        None,
        &mut ids,
    )
    .unwrap();
    let cell = editor.document().cell(outer[0]).unwrap();
    assert_eq!(cell.text, "one fig two");
    // The new text takes the style and the link of the first replaced character; the
    // link shrinks with the text it covered.
    assert_eq!(
        (cell.runs[0].start, cell.runs[0].len, cell.runs[0].style),
        (4, 3, bold)
    );
    assert_eq!((cell.links[0].start, cell.links[0].len), (2, 5));
    editor.document().validate().unwrap();
}

#[test]
fn a_replacement_that_cannot_be_made_everywhere_is_made_nowhere() {
    // An accent alone as the replacement joins the letter before it into one character;
    // the next place to replace then ends inside that character, which no text command
    // allows. Everything is put back.
    let (doc, _, _) = document(&["aa", "a b"], &[]);
    let mut editor = Editor::new(doc.clone()).unwrap();
    let mut ids = SeededIds::from_u64(1);
    let error = replace(
        &mut editor,
        &Term::new("a"),
        "\u{301}",
        Scope::Document,
        ReplaceOptions::default(),
        Some(1),
        &mut ids,
    );
    assert!(matches!(error, Err(Error::Refused(_))), "{error:?}");
    assert!(*editor.document() == doc);
    assert_eq!((editor.undo_len(), editor.redo_len()), (0, 0));
    // Inside a group of the caller's it is refused outright.
    editor.begin_group();
    let error = replace(
        &mut editor,
        &Term::new("a"),
        "x",
        Scope::Document,
        ReplaceOptions::default(),
        None,
        &mut ids,
    );
    assert!(matches!(error, Err(Error::Refused(_))));
    editor.end_group();
    assert!(*editor.document() == doc);
}

const PIECES: &[&str] = &[
    "a", "A", "b", " ", "é", "e\u{301}", "E", "ß", "ss", "S", "日", "本", "ｶﾞ", "ガ", "👍🏽", "-",
];

fn pieces(max: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec(0..PIECES.len(), 0..max)
        .prop_map(|n| n.iter().map(|n| PIECES[*n]).collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// Any replacement: the result is the text with every place replaced, and undoing
    /// it gives the document back exactly.
    #[test]
    fn replacing_and_undoing_gives_back_the_same_bytes(
        cells in proptest::collection::vec(pieces(12), 1..6),
        word in pieces(3),
        with in pieces(3),
        exact in any::<bool>(),
    ) {
        let texts: Vec<&str> = cells.iter().map(String::as_str).collect();
        let (doc, outer, _) = document(&texts, &[]);
        let term = Term { text: word, exact };
        let before = format::to_string(&doc).unwrap();
        let mut editor = Editor::new(doc.clone()).unwrap();
        let list = plan(&doc, &term, &with, Scope::Document, ReplaceOptions::default()).commands;
        match replace(&mut editor, &term, &with, Scope::Document, ReplaceOptions::default(), Some(5), &mut SeededIds::from_u64(1)) {
            Ok(done) => {
                prop_assert_eq!(done.counts.places, list.len());
                for (id, text) in outer.iter().zip(&cells) {
                    // The same, worked out on the characters: the parts between the
                    // places, with the replacement between them.
                    let chars: Vec<char> = text.chars().collect();
                    let mut expected = String::new();
                    let mut at = 0usize;
                    for (start, len) in places(text, &term).whole {
                        expected.extend(&chars[at..start as usize]);
                        expected.push_str(&with);
                        at = (start + len) as usize;
                        prop_assert_eq!(wombat_core::text::is_boundary(text, start), Some(true));
                        prop_assert_eq!(wombat_core::text::is_boundary(text, start + len), Some(true));
                    }
                    expected.extend(&chars[at..]);
                    prop_assert_eq!(&editor.document().cell(*id).unwrap().text, &expected);
                }
                editor.document().validate().unwrap();
                prop_assert!(editor.undo_len() <= 1);
                editor.undo().unwrap();
            }
            Err(error) => prop_assert!(matches!(error, Error::Refused(_)), "{:?}", error),
        }
        prop_assert_eq!(format::to_string(editor.document()).unwrap(), before);
    }
}

/// One piece of the hostile alphabet: one user-perceived character, and its folded
/// form **as written here by hand from the Unicode data** (case folding, NFKD, accents
/// of Latin letters dropped), not as the code under test folds it.
const HOSTILE: &[(&str, &str)] = &[
    ("s", "s"),
    ("S", "s"),
    ("ß", "ss"),
    ("ẞ", "ss"),
    ("f", "f"),
    ("i", "i"),
    ("ﬁ", "fi"),
    ("ﬃ", "ffi"),
    ("ｆ", "f"),
    ("Ａ", "a"),
    ("a", "a"),
    ("İ", "i"),
    ("ı", "i"),
    ("I", "i"),
    ("σ", "σ"),
    ("ς", "σ"),
    ("Σ", "σ"),
    ("é", "e"),
    ("e\u{301}", "e"),
    ("e", "e"),
    ("\u{1112}\u{1161}\u{11ab}", "한"),
    ("한", "한"),
    ("하", "하"),
    ("👩", "👩"),
    ("👨\u{200d}👩\u{200d}👧", "👨\u{200d}👩\u{200d}👧"),
    ("👨", "👨"),
    ("🇩🇪", "🇩🇪"),
    ("🇪🇸", "🇪🇸"),
    ("½", "1⁄2"),
    ("1", "1"),
    ("2", "2"),
    ("⁄", "⁄"),
    ("㎏", "kg"),
    ("k", "k"),
    ("K", "k"),
    ("g", "g"),
    ("㎠", "cm2"),
    ("²", "2"),
    (" ", " "),
    ("-", "-"),
];

/// Queries that find wholes and parts of the pieces above: as typed, and folded by hand.
const WORDS: &[(&str, &str)] = &[
    ("s", "s"),
    ("ss", "ss"),
    ("S", "s"),
    ("f", "f"),
    ("i", "i"),
    ("fi", "fi"),
    ("ffi", "ffi"),
    ("ff", "ff"),
    ("a", "a"),
    ("A", "a"),
    ("σ", "σ"),
    ("ς", "σ"),
    ("e", "e"),
    ("é", "e"),
    ("한", "한"),
    ("하", "하"),
    ("👩", "👩"),
    ("👨", "👨"),
    ("🇩", "🇩"),
    ("🇩🇪", "🇩🇪"),
    ("1", "1"),
    ("2", "2"),
    ("1⁄2", "1⁄2"),
    ("⁄", "⁄"),
    ("k", "k"),
    ("g", "g"),
    ("kg", "kg"),
    ("cm", "cm"),
    ("m2", "m2"),
    (" ", " "),
    ("s ", "s "),
    ("ss-", "ss-"),
    ("is", "is"),
    ("si", "si"),
    ("ı", "i"),
    ("İ", "i"),
    ("ß", "ss"),
    ("ﬁ", "fi"),
    ("½", "1⁄2"),
    ("㎏", "kg"),
    ("e\u{301}", "e"),
    ("\u{301}", ""),
    ("\u{200d}", "\u{200d}"),
];

/// Replace-all as the plainest code would do it, on the pieces and their hand-written
/// folded forms: at each piece from left to right, take the shortest run of whole
/// pieces from there whose folded forms (or, for an exact word, the pieces themselves)
/// make up the word exactly; replace it and go on after it. Returns the new text and
/// which pieces were replaced.
fn reference(pieces: &[usize], word: &str, exact: bool, with: &str) -> (String, Vec<bool>) {
    let form = |n: usize| if exact { HOSTILE[n].0 } else { HOSTILE[n].1 };
    if word.is_empty() {
        let text = pieces.iter().map(|n| HOSTILE[*n].0).collect();
        return (text, vec![false; pieces.len()]);
    }
    let mut out = String::new();
    let mut gone = vec![false; pieces.len()];
    let mut at = 0;
    while at < pieces.len() {
        // The end of the shortest run of whole pieces from here that is the word.
        let mut run = String::new();
        let mut matched = None;
        for (end, piece) in pieces.iter().enumerate().skip(at) {
            run.push_str(form(*piece));
            if run == word {
                matched = Some(end);
            }
            if run.len() >= word.len() {
                break;
            }
        }
        match matched {
            Some(end) => {
                out.push_str(with);
                gone[at..=end].fill(true);
                at = end + 1;
            }
            None => {
                out.push_str(HOSTILE[pieces[at]].0);
                at += 1;
            }
        }
    }
    (out, gone)
}

#[test]
fn the_hostile_alphabet_is_what_its_table_says() {
    use unicode_segmentation::UnicodeSegmentation;
    for (piece, _) in HOSTILE {
        assert_eq!(
            piece.graphemes(true).count(),
            1,
            "{piece:?} is one character"
        );
    }
    // The reviewer's examples, as the tool would be asked.
    let one = |text: &str, word: &str, with: &str| {
        let (doc, outer, _) = document(&[text], &[]);
        let mut editor = Editor::new(doc).unwrap();
        let done = replace(
            &mut editor,
            &Term::new(word),
            with,
            Scope::Document,
            ReplaceOptions::default(),
            None,
            &mut SeededIds::from_u64(1),
        )
        .unwrap();
        let text = editor.document().cell(outer[0]).unwrap().text.clone();
        (text, done.counts.places, done.counts.skipped.len())
    };
    assert_eq!(one("Straße", "s", "X"), ("Xtraße".into(), 1, 1));
    assert_eq!(one("ﬁnance", "f", "Q"), ("ﬁnance".into(), 0, 1));
    assert_eq!(one("㎏", "k", "K"), ("㎏".into(), 0, 1));
    assert_eq!(one("½", "1", "7"), ("½".into(), 0, 1));
    assert_eq!(
        one("👨\u{200d}👩\u{200d}👧", "👩", "W"),
        ("👨\u{200d}👩\u{200d}👧".into(), 0, 1)
    );
    assert_eq!(one("🇩🇪", "🇩", "D"), ("🇩🇪".into(), 0, 1));
    // The whole of such a character is a place like any other.
    assert_eq!(one("Straße", "ss", "X"), ("StraXe".into(), 1, 0));
    assert_eq!(one("ﬁnance", "fi", "Q"), ("Qnance".into(), 1, 0));
    assert_eq!(one("1½ ㎏", "1⁄2", "0.5"), ("10.5 ㎏".into(), 1, 0));
    // What was skipped is said, with the characters that were touched.
    let (doc, outer, _) = document(&["Straße, Masse"], &[]);
    let found = plan(
        &doc,
        &Term::new("s"),
        "X",
        Scope::Document,
        ReplaceOptions::default(),
    );
    assert_eq!((found.counts.places, found.counts.cells), (3, 1));
    let skipped = &found.counts.skipped;
    assert_eq!(skipped.len(), 1);
    assert_eq!(
        (
            skipped[0].cell,
            skipped[0].start,
            skipped[0].len,
            skipped[0].text.as_str()
        ),
        (outer[0], 4, 1, "ß")
    );
}

#[test]
fn formula_cells_are_left_alone_unless_asked_for() {
    let (mut doc, outer, _) = document(&["=SUM(A1)", "the sum is ours", "=1+1"], &[]);
    doc.cell_mut(outer[0]).unwrap().kind = CellKind::Formula;
    doc.cell_mut(outer[2]).unwrap().kind = CellKind::Formula;
    let term = Term::new("s");
    let dry = plan(&doc, &term, "X", Scope::Document, ReplaceOptions::default());
    assert_eq!(
        (
            dry.counts.cells,
            dry.counts.places,
            dry.counts.formula_cells,
            dry.counts.formula_places
        ),
        (1, 3, 1, 1)
    );
    assert!(!dry.counts.formulas_included);
    let mut ids = SeededIds::from_u64(1);
    let mut editor = Editor::new(doc.clone()).unwrap();
    let done = replace(
        &mut editor,
        &term,
        "X",
        Scope::Document,
        ReplaceOptions::default(),
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(
        done.counts, dry.counts,
        "the dry run says what the replace does"
    );
    assert_eq!(
        texts(editor.document())[1..],
        ["=SUM(A1)", "the Xum iX ourX", "=1+1"]
    );
    assert_eq!(done.changes.changed, [outer[1]]);
    // Asked for: the formula text is replaced like any text.
    let all = ReplaceOptions { formulas: true };
    let mut editor = Editor::new(doc.clone()).unwrap();
    let done = replace(
        &mut editor,
        &term,
        "X",
        Scope::Document,
        all,
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(
        (
            done.counts.cells,
            done.counts.places,
            done.counts.formula_cells,
            done.counts.formula_places
        ),
        (2, 4, 1, 1)
    );
    assert!(done.counts.formulas_included);
    assert_eq!(
        done.counts,
        plan(&doc, &term, "X", Scope::Document, all).counts
    );
    assert_eq!(
        texts(editor.document())[1..],
        ["=XUM(A1)", "the Xum iX ourX", "=1+1"]
    );
    assert_eq!(
        editor.document().cell(outer[0]).unwrap().kind,
        CellKind::Formula
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1500))]

    /// Over an alphabet made to hurt: whatever is replaced, every character that was
    /// not exactly matched is still there, in order, and the result is the one the
    /// plain reference above gives.
    #[test]
    fn replacing_never_destroys_what_was_not_matched(
        pieces in proptest::collection::vec(0..HOSTILE.len(), 0..14),
        word in proptest::sample::select(WORDS),
        typed in proptest::collection::vec(0..HOSTILE.len(), 1..4),
        from_text in any::<bool>(),
        with in proptest::sample::select(&["", "X", "ss", "é", "👩", "-"][..]),
        exact in any::<bool>(),
    ) {
        // The word: one of the list, or made of pieces as someone would type them
        // (their folded forms, or the pieces themselves).
        // `word` is what is typed; `wanted` what it stands for: itself if exact,
        // else its folded form, by the tables.
        let (word, folded): (String, String) = if from_text {
            (typed.iter().map(|n| HOSTILE[*n].0).collect(), typed.iter().map(|n| HOSTILE[*n].1).collect())
        } else {
            (word.0.to_owned(), word.1.to_owned())
        };
        let wanted = if exact { word.clone() } else { folded };
        let text: String = pieces.iter().map(|n| HOSTILE[*n].0).collect();
        let (doc, outer, _) = document(&[&text], &[]);
        let term = Term { text: word.clone(), exact };
        let found = places(&text, &term);
        let (expected, gone) = reference(&pieces, &wanted, exact, with);
        // The places are exactly the runs of pieces that the reference replaced.
        let mut starts = Vec::new();
        let mut at = 0u32;
        for n in &pieces {
            starts.push(at);
            at += HOSTILE[*n].0.chars().count() as u32;
        }
        let covered = |place: &(u32, u32), n: usize| {
            starts[n] >= place.0 && starts[n] < place.0 + place.1
        };
        for (n, replaced) in gone.iter().enumerate() {
            prop_assert_eq!(
                found.whole.iter().any(|place| covered(place, n)), *replaced,
                "piece {} of {:?}, word {:?}", n, &text, &word
            );
            // A partial place never claims a replaced piece.
            prop_assert!(!(*replaced && found.partial.iter().any(|place| covered(place, n))));
        }
        let mut editor = Editor::new(doc).unwrap();
        match replace(&mut editor, &term, with, Scope::Document, ReplaceOptions::default(), None, &mut SeededIds::from_u64(1)) {
            Ok(done) => {
                let result = editor.document().cell(outer[0]).unwrap().text.clone();
                prop_assert_eq!(&result, &expected, "{:?} / {:?}", &text, &word);
                prop_assert_eq!(done.counts.places, found.whole.len());
                // Every piece that was not matched is still there, in order.
                let mut rest = result.as_str();
                for (n, _) in gone.iter().enumerate().filter(|(_, gone)| !**gone) {
                    let piece = HOSTILE[pieces[n]].0;
                    let Some(found) = rest.find(piece) else {
                        return Err(TestCaseError::fail(format!("{piece:?} is gone from {result:?}")));
                    };
                    rest = &rest[found + piece.len()..];
                }
            }
            // Refused whole (the replacement would join a neighbour): nothing changed.
            Err(error) => {
                prop_assert!(matches!(error, Error::Refused(_)), "{:?}", error);
                prop_assert_eq!(&editor.document().cell(outer[0]).unwrap().text, &text);
            }
        }
    }
}
