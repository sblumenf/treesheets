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
use wombat_core::{format, CellId, Command, Document, Editor, Error, LinkTarget, SeededIds, Style};
use wombat_search::query::{parse, Term};
use wombat_search::replace::{commands, places, replace, Scope};
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
    index.insert_document(editor.document(), "d.wombat");
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
    index.apply(editor.document(), &changes);
    let styled = format::to_string(editor.document()).unwrap();

    let done = replace(
        &mut editor,
        &Term::new("apple"),
        "Fig",
        Scope::Document,
        Some(7),
        &mut ids,
    )
    .unwrap();
    assert_eq!((done.cells, done.places), (6, 7));
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
    index.apply(editor.document(), &done.changes);
    let mut fresh = SearchIndex::new();
    fresh.insert_document(editor.document(), "d.wombat");
    assert_eq!(index.encoded(), fresh.encoded());
    let count = |index: &SearchIndex, text: &str| {
        let query = parse(text, 0).unwrap().query;
        index.search(&query, &Options::default()).unwrap().total
    };
    assert_eq!((count(&index, "fig"), count(&index, "apple")), (6, 1));
    // One undo: everything back, to the byte. One redo: replaced again.
    assert_eq!(editor.undo_len(), 2);
    let back = editor.undo().unwrap().unwrap();
    index.apply(editor.document(), &back);
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
            None,
            &mut SeededIds::from_u64(1),
        )
        .unwrap();
        (texts(editor.document()), done.places)
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
        let done = replace(&mut editor, &term, "x", Scope::Document, Some(1), &mut ids).unwrap();
        assert_eq!((done.cells, done.places, editor.undo_len()), (0, 0, 0));
        assert!(done.changes.is_empty());
    }
    assert!(*editor.document() == doc);
    let gone = CellId::parse("00000000-0000-0000-0000-00000000dead").unwrap();
    let done = replace(
        &mut editor,
        &Term::new("a"),
        "x",
        Scope::Branch(gone),
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(done.places, 0);
    // Replacing something by itself is nothing either.
    let done = replace(
        &mut editor,
        &Term::exact("apple"),
        "apple",
        Scope::Document,
        Some(1),
        &mut ids,
    )
    .unwrap();
    assert_eq!((done.places, editor.undo_len()), (3, 0));
}

#[test]
fn places_are_whole_characters_from_left_to_right_and_never_overlap() {
    let fold = |text: &str, word: &str| places(text, &Term::new(word));
    assert_eq!(fold("aaaa", "aa"), [(0, 2), (2, 2)]);
    assert_eq!(fold("aaa", "aa"), [(0, 2)]);
    assert_eq!(fold("Straße", "ss"), [(4, 1)]);
    assert_eq!(
        fold("Straße", "s"),
        [(0, 1), (4, 1)],
        "ß is s twice, but one place"
    );
    assert_eq!(fold("e\u{301}e", "e"), [(0, 2), (2, 1)]);
    assert_eq!(fold("ｶﾞｶﾞ", "ガ"), [(0, 2), (2, 2)]);
    assert_eq!(fold("so\u{ad}ft", "soft"), [(0, 5)]);
    assert_eq!(fold("x", ""), []);
    let exact = |text: &str, word: &str| places(text, &Term::exact(word));
    assert_eq!(exact("Aa", "a"), [(1, 1)]);
    assert_eq!(
        exact("e\u{301}e", "e"),
        [(0, 2), (2, 1)],
        "part of a character is the character"
    );
    assert_eq!(exact("👨\u{200d}👩", "👩"), [(0, 3)]);
    // The replacement is not searched again: this ends.
    let (doc, outer, _) = document(&["aXa"], &[]);
    let mut editor = Editor::new(doc).unwrap();
    replace(
        &mut editor,
        &Term::new("a"),
        "aa",
        Scope::Document,
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
        let (list, _) = commands(&doc, &term, &with, Scope::Document);
        match replace(&mut editor, &term, &with, Scope::Document, Some(5), &mut SeededIds::from_u64(1)) {
            Ok(done) => {
                prop_assert_eq!(done.places, list.len());
                for (id, text) in outer.iter().zip(&cells) {
                    // The same, worked out on the characters: the parts between the
                    // places, with the replacement between them.
                    let chars: Vec<char> = text.chars().collect();
                    let mut expected = String::new();
                    let mut at = 0usize;
                    for (start, len) in places(text, &term) {
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
