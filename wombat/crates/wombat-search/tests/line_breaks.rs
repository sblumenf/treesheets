//! Searching and replacing in cell text of several lines. A line break is white space: in
//! folded form it is one space, so a word after a break is a word and a phrase is found
//! across a break as across a space (nobody searching for "buy milk" knows where a line
//! was broken, and notes have always been searched so). Only a search for exactly the
//! characters written tells a break from a space.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use wombat_core::construct::Construct;
use wombat_core::{format, CellId, Command, Document, Editor, SeededIds};
use wombat_search::query::{parse, Order, Query, Term};
use wombat_search::replace::{places, replace, ReplaceOptions, Scope};
use wombat_search::{Field, Options, SearchIndex};

/// A document whose root has one column of cells with these texts.
fn document(texts: &[&str]) -> (Document, Vec<CellId>) {
    let mut ids = SeededIds::from_u64(3);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    doc.add_grid(root, 1, texts.len() as u32, &mut ids).unwrap();
    let cells = doc.cell(root).unwrap().grid().unwrap().cells().to_vec();
    for (id, text) in cells.iter().zip(texts) {
        doc.cell_mut(*id).unwrap().text = (*text).to_string();
    }
    doc.validate().unwrap();
    (doc, cells)
}

fn index_of(editor: &Editor) -> SearchIndex {
    let mut index = SearchIndex::new();
    index.insert_document(editor.document(), editor.calc(), "d.wombat");
    index
}

/// The cells that a query in the text syntax finds, in document order.
fn found(index: &SearchIndex, query: &str) -> Vec<CellId> {
    let query = parse(query, 0).unwrap().query;
    hits(index, &query)
}

fn hits(index: &SearchIndex, query: &Query) -> Vec<CellId> {
    let results = index.search(query, &Options::all(Order::Document)).unwrap();
    results.hits.iter().map(|hit| hit.cell).collect()
}

#[test]
fn a_line_break_is_white_space_between_words_to_a_search() {
    let (doc, cells) = document(&[
        "buy\nmilk",
        "buy milk",
        "buymilk",
        "milk\n\nbuy",
        "\nmilk",
        "one\ntwo\nthree",
        "buy\n\nmilk",
    ]);
    let editor = Editor::new(doc).unwrap();
    let index = index_of(&editor);
    index.check().unwrap();
    // A word at the start of a line is found like any word.
    assert_eq!(
        found(&index, "milk"),
        [cells[0], cells[1], cells[2], cells[3], cells[4], cells[6]]
    );
    assert_eq!(found(&index, "two"), [cells[5]]);
    // Both words, wherever: every cell that has both.
    assert_eq!(
        found(&index, "buy milk"),
        [cells[0], cells[1], cells[2], cells[3], cells[6]]
    );
    // A phrase is found across one break as across one space; two breaks are two spaces.
    assert_eq!(found(&index, "\"buy milk\""), [cells[0], cells[1]]);
    assert_eq!(found(&index, "\"buy  milk\""), [cells[6]]);
    assert_eq!(found(&index, "\"two three\""), [cells[5]]);
    // A line break written in a phrase is the same white space.
    assert_eq!(found(&index, "\"buy\nmilk\""), [cells[0], cells[1]]);
    let phrase = Query::Term(Term::new("buy\r\nmilk"));
    assert_eq!(hits(&index, &phrase), [cells[6]]);
    // Exactly as written: a break is a break and a space is a space.
    let exact = |text: &str| hits(&index, &Query::Term(Term::exact(text)));
    assert_eq!(exact("buy milk"), [cells[1]]);
    assert_eq!(exact("buy\nmilk"), [cells[0]]);
    assert_eq!(
        exact("\n"),
        [cells[0], cells[3], cells[4], cells[5], cells[6]]
    );
}

#[test]
fn a_hit_in_a_text_of_several_lines_says_where_and_shows_one_line() {
    let (doc, cells) = document(&["first line\nsecond line\nthird"]);
    let editor = Editor::new(doc).unwrap();
    let index = index_of(&editor);
    let query = parse("\"line second\"", 0).unwrap().query;
    let results = index.search(&query, &Options::default()).unwrap();
    let hit = &results.hits[0];
    assert_eq!((hit.cell, hit.field), (cells[0], Field::Text));
    // The text as it is, and the range across the break, in code points.
    assert_eq!(hit.matched, "first line\nsecond line\nthird");
    assert_eq!(hit.text, hit.matched);
    assert_eq!(hit.ranges, [(6, 11)]);
    // What is shown for the hit is one line: a break is a space there.
    assert_eq!(hit.snippet.text, "first line second line third");
    assert_eq!(hit.snippet.ranges, [(6, 11)]);
}

#[test]
fn the_places_of_a_term_count_a_line_break_as_one_character() {
    let text = "a\nb a b\n\na\nb";
    assert_eq!(
        places(text, &Term::new("a b")).whole,
        [(0, 3), (4, 3), (9, 3)]
    );
    assert_eq!(places(text, &Term::exact("a b")).whole, [(4, 3)]);
    assert_eq!(places(text, &Term::exact("a\nb")).whole, [(0, 3), (9, 3)]);
    assert_eq!(places(text, &Term::new("\n")).whole.len(), 6);
    assert_eq!(
        places(text, &Term::exact("\n")).whole,
        [(1, 1), (7, 1), (8, 1), (10, 1)]
    );
}

#[test]
fn replacing_across_and_with_line_breaks_is_one_step_and_the_index_follows() {
    let (doc, cells) = document(&["buy\nmilk today", "buy milk", "no match\nhere"]);
    let before = format::to_string(&doc).unwrap();
    let mut editor = Editor::new(doc).unwrap();
    let mut ids = SeededIds::from_u64(1);
    let mut index = index_of(&editor);
    let text = |editor: &Editor, n: usize| editor.document().cell(cells[n]).unwrap().text.clone();
    let follow = |index: &mut SearchIndex, editor: &Editor, changes: &wombat_core::Changes| {
        index
            .apply(editor.document(), editor.calc(), changes)
            .unwrap();
        index.check().unwrap();
        let fresh = index_of(editor);
        assert_eq!(index.encoded(), fresh.encoded());
    };

    // The phrase is replaced with the break it reaches across; the new text brings a
    // break of its own, written as a file from elsewhere writes it.
    let done = replace(
        &mut editor,
        &Term::new("buy milk"),
        "get\r\noat milk",
        Scope::Document,
        ReplaceOptions::default(),
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!((done.counts.cells, done.counts.places), (2, 2));
    assert_eq!(text(&editor, 0), "get\noat milk today");
    assert_eq!(text(&editor, 1), "get\noat milk");
    assert_eq!(text(&editor, 2), "no match\nhere");
    follow(&mut index, &editor, &done.changes);
    assert_eq!(found(&index, "\"get oat\""), [cells[0], cells[1]]);
    assert_eq!(found(&index, "buy"), []);

    // Exactly a break, replaced by a space: the lines are joined, and only there.
    let done = replace(
        &mut editor,
        &Term::exact("\n"),
        " ",
        Scope::Cells(&cells[..2]),
        ReplaceOptions::default(),
        None,
        &mut ids,
    )
    .unwrap();
    assert_eq!(done.counts.places, 2);
    assert_eq!(text(&editor, 0), "get oat milk today");
    assert_eq!(text(&editor, 2), "no match\nhere");
    follow(&mut index, &editor, &done.changes);

    // Two steps of undo give the file back to the byte, and the index goes along.
    assert_eq!(editor.undo_len(), 2);
    for _ in 0..2 {
        let back = editor.undo().unwrap().unwrap();
        follow(&mut index, &editor, &back);
    }
    assert_eq!(format::to_string(editor.document()).unwrap(), before);
    assert_eq!(found(&index, "\"buy milk\""), [cells[0], cells[1]]);
}

#[test]
fn the_index_follows_an_edit_that_adds_or_removes_a_line_break() {
    // A cell of plain lower-case text is kept without a folded form of its own; with a
    // break it needs one, and loses it again when the break goes.
    let (doc, cells) = document(&["plain text", "other"]);
    let mut editor = Editor::new(doc).unwrap();
    let mut ids = SeededIds::from_u64(1);
    let mut index = index_of(&editor);
    let mut apply = |editor: &mut Editor, index: &mut SearchIndex, command| {
        let changes = editor.apply(command, None, &mut ids).unwrap();
        index
            .apply(editor.document(), editor.calc(), &changes)
            .unwrap();
        index.check().unwrap();
        assert_eq!(index.encoded(), index_of(editor).encoded());
    };
    let insert = Command::InsertText {
        cell: cells[0],
        at: 5,
        text: "\r\n".into(),
    };
    apply(&mut editor, &mut index, insert);
    assert_eq!(
        editor.document().cell(cells[0]).unwrap().text,
        "plain\n text"
    );
    assert_eq!(found(&index, "\"plain  text\""), [cells[0]]);
    assert_eq!(found(&index, "\"plain text\""), []);
    // The break does not make the text run into the next field: nothing of the note is
    // found as text, and the other way round.
    let delete = Command::DeleteText {
        cell: cells[0],
        start: 5,
        len: 1,
    };
    apply(&mut editor, &mut index, delete);
    assert_eq!(found(&index, "\"plain text\""), [cells[0]]);
    let set = Command::SetText {
        cell: cells[1],
        text: "\n".into(),
    };
    apply(&mut editor, &mut index, set);
    assert_eq!(found(&index, "other"), []);
}

#[test]
fn a_break_at_the_end_of_a_text_does_not_join_it_to_the_note() {
    let (mut doc, cells) = document(&["ends with a break\n", "plain"]);
    doc.cell_mut(cells[0]).unwrap().note = "the note".into();
    doc.cell_mut(cells[1]).unwrap().note = "second\nnote".into();
    let editor = Editor::new(doc).unwrap();
    let index = index_of(&editor);
    index.check().unwrap();
    // A phrase is found within one field, never from the text into the note.
    assert_eq!(found(&index, "\"break the\""), []);
    assert_eq!(found(&index, "\"break  the\""), []);
    assert_eq!(found(&index, "\"plain second\""), []);
    assert_eq!(found(&index, "\"second note\""), [cells[1]]);
    let query = parse("note", 0).unwrap().query;
    let results = index
        .search(&query, &Options::all(Order::Document))
        .unwrap();
    let fields: Vec<Field> = results.hits.iter().map(|hit| hit.field).collect();
    assert_eq!(fields, [Field::Note, Field::Note]);
    let query = parse("break", 0).unwrap().query;
    let results = index
        .search(&query, &Options::all(Order::Document))
        .unwrap();
    assert_eq!(results.hits[0].field, Field::Text);
    assert_eq!(results.hits[0].ranges, [(12, 5)]);
}
