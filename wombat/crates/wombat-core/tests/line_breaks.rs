//! Line breaks in cell text: U+000A is a character of cell text, CR LF and a lone CR become
//! one, and every other control character is still a space. Tag names have none. What each
//! command, the file format and the conversions do with a text of several lines.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use wombat_core::construct::Construct;
use wombat_core::convert::{Entry, Table};
use wombat_core::links::copy_between;
use wombat_core::{
    format, CellId, Clip, Color, Command, Document, Editor, Error, LinkTarget, SeededIds, Style,
};

const NOW: Option<i64> = Some(1_700_000_000_000);

/// A document whose root has a grid of `xs` by `ys` cells with the given texts, row by row.
fn table(xs: u32, ys: u32, texts: &[&str]) -> (Editor, SeededIds, Vec<CellId>) {
    let mut ids = SeededIds::from_u64(11);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), xs, ys, &mut ids).unwrap();
    let cells = kids(&doc, doc.root());
    for (id, text) in cells.iter().zip(texts) {
        doc.cell_mut(*id).unwrap().text = (*text).to_string();
    }
    (Editor::new(doc).unwrap(), ids, cells)
}

fn kids(doc: &Document, cell: CellId) -> Vec<CellId> {
    doc.cell(cell)
        .unwrap()
        .grid()
        .map_or(Vec::new(), |g| g.cells().to_vec())
}

fn text(editor: &Editor, cell: CellId) -> &str {
    &editor.document().cell(cell).unwrap().text
}

fn texts(doc: &Document, cell: CellId) -> Vec<String> {
    kids(doc, cell)
        .iter()
        .map(|c| doc.cell(*c).unwrap().text.clone())
        .collect()
}

fn saves_and_reloads(doc: &Document) -> String {
    doc.validate().unwrap();
    let saved = format::to_string(doc).expect("the document must save");
    assert_eq!(format::from_slice(saved.as_bytes()).unwrap(), *doc);
    saved
}

/// The four commands that take a text, each putting `new` into an empty cell.
fn text_commands(cell: CellId, new: &str) -> Vec<Command> {
    let text = new.to_string();
    vec![
        Command::InsertText {
            cell,
            at: 0,
            text: text.clone(),
        },
        Command::ReplaceText {
            cell,
            start: 0,
            len: 0,
            text: text.clone(),
        },
        Command::SetText {
            cell,
            text: text.clone(),
        },
        Command::EnterText { cell, text },
    ]
}

// ---- the rule ---------------------------------------------------------------------------

#[test]
fn every_text_command_keeps_a_line_feed_and_makes_one_of_the_other_line_ends() {
    let cases = [
        ("a\nb", "a\nb"),
        ("a\r\nb", "a\nb"),
        ("a\rb", "a\nb"),
        ("\r\n\r\n", "\n\n"),
        ("\r\r\n\n\r", "\n\n\n\n"),
        ("a\tb\u{0}c\u{b}d\u{c}e\u{1f}f", "a b c d e f"),
        ("\n", "\n"),
    ];
    for (given, kept) in cases {
        let (_, _, cells) = table(1, 1, &[""]);
        for command in text_commands(cells[0], given) {
            let (mut editor, mut ids, cells) = table(1, 1, &[""]);
            let what = format!("{command:?}");
            editor.apply(command, NOW, &mut ids).unwrap();
            assert_eq!(text(&editor, cells[0]), kept, "{what}");
            saves_and_reloads(editor.document());
            editor.undo().unwrap().unwrap();
            assert_eq!(text(&editor, cells[0]), "", "{what}");
            editor.redo().unwrap().unwrap();
            assert_eq!(text(&editor, cells[0]), kept, "{what}");
        }
    }
}

#[test]
fn entering_the_text_a_cell_has_with_other_line_ends_changes_nothing() {
    let (mut editor, mut ids, cells) = table(1, 1, &["a\nb"]);
    for same in ["a\nb", "a\r\nb", "a\rb"] {
        for command in [
            Command::SetText {
                cell: cells[0],
                text: same.into(),
            },
            Command::EnterText {
                cell: cells[0],
                text: same.into(),
            },
        ] {
            let changes = editor.apply(command, NOW, &mut ids).unwrap();
            assert!(changes.is_empty(), "{same:?}");
        }
    }
    assert_eq!(editor.undo_len(), 0);
}

#[test]
fn a_line_break_is_one_position_and_a_place_to_cut() {
    let (mut editor, mut ids, cells) = table(1, 1, &[""]);
    let cell = cells[0];
    // CR LF is one user-perceived character of two code points; in the cell it is one.
    let insert = |at, text: &str| Command::InsertText {
        cell,
        at,
        text: text.into(),
    };
    editor
        .apply(insert(0, "e\u{301}\r\nx"), NOW, &mut ids)
        .unwrap();
    assert_eq!(text(&editor, cell), "e\u{301}\nx");
    let now = text(&editor, cell).to_string();
    assert_eq!(wombat_core::text::is_boundary(&now, 2), Some(true));
    assert_eq!(wombat_core::text::is_boundary(&now, 3), Some(true));
    // Typing before, after and instead of the break.
    editor.apply(insert(2, "1"), NOW, &mut ids).unwrap();
    editor.apply(insert(4, "2"), NOW, &mut ids).unwrap();
    assert_eq!(text(&editor, cell), "e\u{301}1\n2x");
    let replace = Command::ReplaceText {
        cell,
        start: 3,
        len: 1,
        text: " ".into(),
    };
    editor.apply(replace, NOW, &mut ids).unwrap();
    assert_eq!(text(&editor, cell), "e\u{301}1 2x");
    // A carriage return typed in front of a line feed is a line break of its own: each
    // text that comes in is made clean by itself.
    editor.apply(insert(3, "\n"), NOW, &mut ids).unwrap();
    editor.apply(insert(3, "\r"), NOW, &mut ids).unwrap();
    assert_eq!(text(&editor, cell), "e\u{301}1\n\n 2x");
    let delete = Command::DeleteText {
        cell,
        start: 3,
        len: 1,
    };
    editor.apply(delete, NOW, &mut ids).unwrap();
    assert_eq!(text(&editor, cell), "e\u{301}1\n 2x");
    // The accent still cannot be cut off its letter.
    let cut = editor.apply(insert(1, "\n"), NOW, &mut ids);
    assert!(matches!(cut, Err(Error::Refused(_))), "{cut:?}");
    saves_and_reloads(editor.document());
}

#[test]
fn runs_and_links_reach_across_a_line_break_and_follow_edits_at_it() {
    let (mut editor, mut ids, cells) = table(1, 1, &["ab\ncd"]);
    let cell = cells[0];
    let bold = Style {
        bold: true,
        ..Style::default()
    };
    for command in [
        Command::SetRangeStyle {
            cell,
            start: 1,
            len: 3,
            set: bold,
            clear: Style::default(),
        },
        Command::SetRangeColor {
            cell,
            start: 2,
            len: 1,
            color: Some(Color::rgb(1, 2, 3)),
        },
        Command::SetLink {
            cell,
            start: 1,
            len: 3,
            target: Some(LinkTarget::External("https://example.org".into())),
        },
        // Another break inside the run and the link: both grow by one.
        Command::InsertText {
            cell,
            at: 2,
            text: "\r\n".into(),
        },
    ] {
        editor.apply(command, NOW, &mut ids).unwrap();
        saves_and_reloads(editor.document());
    }
    let c = editor.document().cell(cell).unwrap();
    assert_eq!(c.text, "ab\n\ncd");
    assert_eq!(
        c.runs.iter().map(|r| (r.start, r.len)).collect::<Vec<_>>(),
        [(1, 2), (3, 1), (4, 1)]
    );
    assert_eq!(
        c.links.iter().map(|l| (l.start, l.len)).collect::<Vec<_>>(),
        [(1, 4)]
    );
    // Deleting both breaks leaves the ranges on what is left.
    let delete = Command::DeleteText {
        cell,
        start: 2,
        len: 2,
    };
    editor.apply(delete, NOW, &mut ids).unwrap();
    let c = editor.document().cell(cell).unwrap();
    assert_eq!(c.text, "abcd");
    assert_eq!(
        c.runs.iter().map(|r| (r.start, r.len)).collect::<Vec<_>>(),
        [(1, 2)]
    );
    assert_eq!(
        c.links.iter().map(|l| (l.start, l.len)).collect::<Vec<_>>(),
        [(1, 2)]
    );
    while editor.undo().unwrap().is_some() {}
    let c = editor.document().cell(cell).unwrap();
    assert_eq!(
        (c.text.as_str(), c.runs.len(), c.links.len()),
        ("ab\ncd", 0, 0)
    );
}

// ---- the model and the file -------------------------------------------------------------

#[test]
fn the_model_takes_a_line_feed_in_a_text_and_no_other_control_character() {
    let build = |text: &str| {
        let mut ids = SeededIds::from_u64(1);
        let mut doc = Document::new(&mut ids);
        let root = doc.root();
        doc.cell_mut(root).unwrap().text = text.to_string();
        doc
    };
    saves_and_reloads(&build("two\nlines\n"));
    for bad in ["a\rb", "a\r\nb", "a\tb", "\u{0}", "\u{1f}"] {
        let doc = build(bad);
        let error = doc.validate().unwrap_err();
        assert!(matches!(error, Error::Invalid(_)), "{bad:?}: {error}");
        assert!(format::to_string(&doc).is_err(), "{bad:?}");
        assert!(Editor::new(doc).is_err(), "{bad:?}");
    }
}

#[test]
fn a_tag_name_has_no_line_break() {
    let mut ids = SeededIds::from_u64(1);
    let mut doc = Document::new(&mut ids);
    doc.tags_mut().insert("a\nb".into(), Default::default());
    assert!(matches!(doc.validate(), Err(Error::Invalid(_))));

    let (mut editor, mut ids, _) = table(1, 1, &[""]);
    for name in ["a\nb", "\n", "a\rb", "a\tb"] {
        let command = Command::SetTag {
            name: name.into(),
            cell_color: Color::WHITE,
            text_color: Color::BLACK,
        };
        let result = editor.apply(command, NOW, &mut ids);
        assert!(matches!(result, Err(Error::Refused(_))), "{name:?}");
    }
    assert!(editor.document().tags().is_empty());
}

/// The saved text of a document with one cell whose text is `text` as JSON writes it.
fn file_with_text(json_text: &str) -> String {
    let mut ids = SeededIds::from_u64(5);
    let mut doc = Document::new(&mut ids);
    let root = doc.root();
    doc.cell_mut(root).unwrap().text = "MARK".into();
    let saved = format::to_string(&doc).unwrap();
    assert_eq!(saved.matches("\"MARK\"").count(), 1);
    saved.replace("\"MARK\"", &format!("\"{json_text}\""))
}

#[test]
fn a_file_with_a_line_break_in_a_text_loads_and_saves_as_it_was() {
    let file = file_with_text("one\\ntwo\\n\\nfour");
    let doc = format::from_slice(file.as_bytes()).unwrap();
    assert_eq!(doc.cell(doc.root()).unwrap().text, "one\ntwo\n\nfour");
    assert_eq!(format::to_string(&doc).unwrap(), file);
    // The other way JSON can write the same character.
    let doc = format::from_slice(file_with_text("one\\u000atwo").as_bytes()).unwrap();
    assert_eq!(doc.cell(doc.root()).unwrap().text, "one\ntwo");
}

#[test]
fn a_file_with_another_line_end_in_a_text_is_refused_not_repaired() {
    // A file is not a way in for text to be cleaned: it either follows the format or is
    // not a Wombat file. (An editor that rewrote LF as CR LF made the file text itself
    // differ, which JSON does not mind; inside a string a break is always written `\n`.)
    for bad in ["a\\rb", "a\\r\\nb", "a\\tb", "a\\u0000b", "a\\u000db"] {
        let result = format::from_slice(file_with_text(bad).as_bytes());
        assert!(
            matches!(result, Err(Error::Invalid(_))),
            "{bad}: {result:?}"
        );
    }
}

// ---- cells that come from elsewhere -----------------------------------------------------

#[test]
fn an_import_keeps_line_feeds_and_makes_them_of_the_other_line_ends() {
    let entries = ["a\nb", "c\r\nd", "e\rf", "g\th"].map(Entry::new).to_vec();
    assert_eq!(entries[1].text(), "c\nd");
    let (mut editor, mut ids, cells) = table(1, 1, &[""]);
    let command = Command::Import {
        cell: cells[0],
        table: Table::new(2, entries).unwrap(),
        replace: true,
    };
    editor.apply(command, NOW, &mut ids).unwrap();
    assert_eq!(
        texts(editor.document(), cells[0]),
        ["a\nb", "c\nd", "e\nf", "g h"]
    );
    saves_and_reloads(editor.document());
    editor.undo().unwrap().unwrap();
    editor.redo().unwrap().unwrap();
    assert_eq!(texts(editor.document(), cells[0])[2], "e\nf");
}

#[test]
fn cells_with_line_breaks_are_pasted_and_copied_between_documents() {
    let (source, _, from) = table(2, 1, &["one\ntwo", "\n"]);
    let root = source.document().root();
    let clip = Clip::of(source.document(), root).unwrap();
    // The clip as it leaves the program and comes back.
    let saved = format::to_string(&clip.as_document().unwrap()).unwrap();
    let back = format::from_slice(saved.as_bytes()).unwrap();
    let through = Clip::from_document(&back, clip.parent()).unwrap();
    assert_eq!(through, clip);

    let mut ids = SeededIds::from_u64(77);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), 3, 1, &mut ids).unwrap();
    let to = kids(&doc, doc.root());
    let mut target = Editor::new(doc).unwrap();
    let paste = Command::PasteCells {
        to: to[0],
        clip: through,
    };
    target.apply(paste, NOW, &mut ids).unwrap();
    assert_eq!(texts(target.document(), to[0]), ["one\ntwo", "\n"]);
    copy_between(source.document(), from[0], &mut target, to[1], &mut ids).unwrap();
    assert_eq!(text(&target, to[1]), "one\ntwo");
    let copy = Command::CopyCell {
        from: to[1],
        to: to[2],
    };
    target.apply(copy, NOW, &mut ids).unwrap();
    assert_eq!(text(&target, to[2]), "one\ntwo");
    saves_and_reloads(target.document());
    while target.undo().unwrap().is_some() {}
    assert_eq!(text(&target, to[1]), "");
    while target.redo().unwrap().is_some() {}
    assert_eq!(text(&target, to[2]), "one\ntwo");
}

// ---- the commands that read a text ------------------------------------------------------

#[test]
fn a_cell_of_several_lines_cannot_become_a_tag() {
    let (mut editor, mut ids, cells) = table(3, 1, &["plain", "two\nlines", "two\nlines"]);
    let before = editor.document().clone();
    // All or nothing: the cell that could be a tag is not made one either.
    let tag = Command::TagCells {
        cells: cells.clone(),
    };
    let result = editor.apply(tag, NOW, &mut ids);
    assert!(matches!(result, Err(Error::Refused(_))), "{result:?}");
    assert_eq!(*editor.document(), before);
    saves_and_reloads(editor.document());
    // Taking the tag away from such a cell finds no tag and does nothing.
    let untag = Command::UntagCells {
        cells: vec![cells[1]],
    };
    assert!(editor.apply(untag, NOW, &mut ids).unwrap().is_empty());
    // The single-line cell alone is tagged as before.
    let tag = Command::TagCells {
        cells: vec![cells[0]],
    };
    editor.apply(tag, NOW, &mut ids).unwrap();
    assert!(editor.document().tags().contains_key("plain"));
}

#[test]
fn collapse_joins_with_a_space_and_keeps_the_breaks_inside_each_text() {
    let (mut editor, mut ids, _) = table(3, 1, &["a\nb", "c\nd", "\n"]);
    let root = editor.document().root();
    let collapse = Command::Collapse {
        cell: root,
        x: 0,
        y: 0,
        xs: 3,
        ys: 1,
    };
    editor.apply(collapse, NOW, &mut ids).unwrap();
    assert_eq!(texts(editor.document(), root), ["a\nb c\nd \n"]);
    saves_and_reloads(editor.document());
    editor.undo().unwrap().unwrap();
    assert_eq!(texts(editor.document(), root), ["a\nb", "c\nd", "\n"]);
}

#[test]
fn a_cell_is_split_at_and_beside_a_line_break() {
    for (at, parts) in [
        (1, ["a", "\nb"]),
        (2, ["a\n", "b"]),
        (0, ["", "a\nb"]),
        (3, ["a\nb", ""]),
    ] {
        let (mut editor, mut ids, cells) = table(1, 1, &["a\nb"]);
        let split = Command::SplitCell { cell: cells[0], at };
        editor.apply(split, NOW, &mut ids).unwrap();
        assert_eq!(texts(editor.document(), cells[0]), parts, "at {at}");
        assert_eq!(text(&editor, cells[0]), "");
        saves_and_reloads(editor.document());
        editor.undo().unwrap().unwrap();
        assert_eq!(text(&editor, cells[0]), "a\nb");
    }
}

#[test]
fn sorting_compares_a_line_break_as_the_character_it_is() {
    // U+000A sorts before a space and before every letter, whichever way case is folded.
    let (mut editor, mut ids, _) = table(1, 5, &["a b", "a\nb", "ab", "a", "\n"]);
    let root = editor.document().root();
    let sort = |descending| Command::SortRows {
        cell: root,
        column: 0,
        row: 0,
        count: 5,
        descending,
    };
    editor.apply(sort(false), NOW, &mut ids).unwrap();
    assert_eq!(
        texts(editor.document(), root),
        ["\n", "a", "a\nb", "a b", "ab"]
    );
    editor.apply(sort(true), NOW, &mut ids).unwrap();
    assert_eq!(
        texts(editor.document(), root),
        ["ab", "a b", "a\nb", "a", "\n"]
    );
    saves_and_reloads(editor.document());
}

#[test]
fn hierarchify_and_flatten_compare_and_carry_whole_texts_with_their_breaks() {
    // Two rows under the same two-line text become one parent; "x\ny" and "x y" differ.
    let (mut editor, mut ids, _) = table(2, 3, &["x\ny", "one", "x\ny", "two\n2", "x y", "three"]);
    let root = editor.document().root();
    editor
        .apply(Command::Hierarchify { cell: root }, NOW, &mut ids)
        .unwrap();
    let doc = editor.document();
    let tops = kids(doc, root);
    assert_eq!(texts(doc, root), ["x\ny", "x y"]);
    assert_eq!(texts(doc, tops[0]), ["one", "two\n2"]);
    assert_eq!(texts(doc, tops[1]), ["three"]);
    saves_and_reloads(doc);
    editor
        .apply(Command::Flatten { cell: root }, NOW, &mut ids)
        .unwrap();
    assert_eq!(
        texts(editor.document(), root),
        ["x\ny", "one", "x\ny", "two\n2", "x y", "three"]
    );
    saves_and_reloads(editor.document());
    while editor.undo().unwrap().is_some() {}
    assert_eq!(texts(editor.document(), root).len(), 6);
}

#[test]
fn a_check_box_is_added_ticked_and_removed_in_front_of_the_first_line() {
    let given = ["\u{2610} buy\nmilk", "plain\ntext", "\n", "\u{2611}\nx"];
    let (mut editor, mut ids, cells) = table(4, 1, &given);
    let root = editor.document().root();
    let toggle = Command::ToggleCheck {
        cells: cells.clone(),
    };
    let mut step = |editor: &mut Editor| {
        editor.apply(toggle.clone(), NOW, &mut ids).unwrap();
        saves_and_reloads(editor.document());
        texts(editor.document(), root)
    };
    // Not every cell has a box: those without get one, with a space, before the text.
    assert_eq!(
        step(&mut editor),
        [
            "\u{2610} buy\nmilk",
            "\u{2610} plain\ntext",
            "\u{2610} \n",
            "\u{2611}\nx"
        ]
    );
    assert_eq!(
        step(&mut editor),
        [
            "\u{2611} buy\nmilk",
            "\u{2611} plain\ntext",
            "\u{2611} \n",
            "\u{2611}\nx"
        ]
    );
    // The box goes with the space after it; a break after the box is not that space.
    assert_eq!(step(&mut editor), ["buy\nmilk", "plain\ntext", "\n", "\nx"]);
    while editor.undo().unwrap().is_some() {}
    assert_eq!(texts(editor.document(), root), given);
}

// ---- limits -----------------------------------------------------------------------------

#[test]
fn a_line_break_counts_as_one_character_towards_the_limit_and_fits_the_size_bound() {
    use wombat_core::limits::MAX_TEXT_CODE_POINTS;
    let (mut editor, mut ids, cells) = table(1, 1, &[""]);
    let set = |n: usize| Command::SetText {
        cell: cells[0],
        text: "\n".repeat(n),
    };
    let over = editor.apply(set(MAX_TEXT_CODE_POINTS + 1), NOW, &mut ids);
    assert!(matches!(over, Err(Error::Limit(_))));
    // CR LF pairs count after they have become one character each.
    let pairs = Command::SetText {
        cell: cells[0],
        text: "\r\n".repeat(100_000),
    };
    editor.apply(pairs, NOW, &mut ids).unwrap();
    assert_eq!(text(&editor, cells[0]).len(), 100_000);
    let saved = saves_and_reloads(editor.document());
    // Each break is two bytes in the file, and the running bound knows.
    assert!(saved.len() as u64 > 200_000);
    assert!(saved.len() as u64 <= editor.size_bound());
    assert_eq!(
        editor.size_bound(),
        wombat_core::edit::size_bound(editor.document())
    );
}
