//! Unit tests of every command: what it does, what it refuses, undo and redo.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use super::step::{ChangeSet, Step};
use super::*;
use crate::construct::Construct;
use crate::id::IdSource;
use crate::id::{DocId, Id, SeededIds};
use crate::limits::MAX_DEPTH;
use crate::model::{Cell, Extra, Grid, Link, Run};
use crate::view::{Selection, ViewState};

const NOW: Option<i64> = Some(1_700_000_000_000);

/// An editor on a document whose root has an `xs` by `ys` grid of cells "0", "1", ...
fn editor(xs: u32, ys: u32) -> (Editor, SeededIds, Vec<CellId>) {
    let mut ids = SeededIds::from_u64(7);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), xs, ys, &mut ids).unwrap();
    let cells = doc
        .cell(doc.root())
        .unwrap()
        .grid()
        .unwrap()
        .cells()
        .to_vec();
    for (n, id) in cells.iter().enumerate() {
        doc.cell_mut(*id).unwrap().text = n.to_string();
    }
    (Editor::new(doc).unwrap(), ids, cells)
}

struct Fixture {
    e: Editor,
    ids: SeededIds,
    cells: Vec<CellId>,
}

impl Fixture {
    fn new(xs: u32, ys: u32) -> Fixture {
        let (e, ids, cells) = editor(xs, ys);
        Fixture { e, ids, cells }
    }

    /// One cell with this text.
    fn text(text: &str) -> (Fixture, CellId) {
        let mut f = Fixture::new(1, 1);
        let id = f.cells[0];
        f.ok(Command::SetText {
            cell: id,
            text: text.into(),
        });
        f.e.clear_history();
        (f, id)
    }

    fn run(&mut self, command: Command) -> Result<Changes, Error> {
        let before = self.e.document().clone();
        let history = (self.e.undo_len(), self.e.redo_len());
        let result = self.e.apply(command, NOW, &mut self.ids);
        self.e.document().validate().unwrap();
        if result.is_err() {
            assert_eq!(
                *self.e.document(),
                before,
                "a failed command changed something"
            );
            assert_eq!((self.e.undo_len(), self.e.redo_len()), history);
        }
        result
    }

    /// Applies a command that must succeed, and checks that undo and redo reverse it.
    fn ok(&mut self, command: Command) -> Changes {
        let before = self.e.document().clone();
        let units = self.e.undo_len();
        let changes = self.run(command).unwrap();
        let after = self.e.document().clone();
        if self.e.undo_len() > units {
            let undone = self.e.undo().unwrap().unwrap();
            assert_eq!(
                *self.e.document(),
                before,
                "undo did not restore the document"
            );
            self.e.document().validate().unwrap();
            let redone = self.e.redo().unwrap().unwrap();
            assert_eq!(
                *self.e.document(),
                after,
                "redo did not restore the document"
            );
            assert_eq!(redone, changes, "redo reports what the command reported");
            assert_eq!(
                (&undone.created, &undone.removed, &undone.tags),
                (&changes.removed, &changes.created, &changes.tags)
            );
        } else {
            assert_eq!(after, before);
            assert!(changes.is_empty());
        }
        changes
    }

    fn bad(&mut self, command: Command) -> Error {
        self.run(command).unwrap_err()
    }

    fn cell(&self, id: CellId) -> &Cell {
        self.e.document().cell(id).unwrap()
    }

    fn doc(&self) -> &Document {
        self.e.document()
    }

    fn root(&self) -> CellId {
        self.doc().root()
    }

    fn grid(&self, id: CellId) -> &Grid {
        self.cell(id).grid().unwrap()
    }

    /// The texts of a grid, rows separated by `/`.
    fn texts(&self, id: CellId) -> String {
        let grid = self.grid(id);
        grid.cells()
            .chunks(grid.xs() as usize)
            .map(|row| {
                let texts: Vec<&str> = row.iter().map(|c| self.cell(*c).text.as_str()).collect();
                texts.join(" ")
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    /// The runs of a cell as (start, length, style letters, colour).
    fn runs(&self, id: CellId) -> Vec<(u32, u32, String, Option<Color>)> {
        self.cell(id)
            .runs
            .iter()
            .map(|r| (r.start, r.len, letters(r.style), r.color))
            .collect()
    }

    fn links(&self, id: CellId) -> Vec<(u32, u32, String)> {
        self.cell(id)
            .links
            .iter()
            .map(|l| {
                let target = match &l.target {
                    LinkTarget::External(url) => url.clone(),
                    LinkTarget::Cell { cell, .. } => format!("cell {cell}"),
                    LinkTarget::Unknown => "?".into(),
                };
                (l.start, l.len, target)
            })
            .collect()
    }
}

fn letters(style: Style) -> String {
    let mut out = String::new();
    for (on, letter) in [
        (style.bold, 'b'),
        (style.italic, 'i'),
        (style.fixed, 'f'),
        (style.underline, 'u'),
        (style.strike, 's'),
    ] {
        if on {
            out.push(letter);
        }
    }
    out
}

fn style(letters: &str) -> Style {
    Style {
        bold: letters.contains('b'),
        italic: letters.contains('i'),
        fixed: letters.contains('f'),
        underline: letters.contains('u'),
        strike: letters.contains('s'),
    }
}

fn styled(cell: CellId, start: u32, len: u32, letters: &str) -> Command {
    Command::SetRangeStyle {
        cell,
        start,
        len,
        set: style(letters),
        clear: Style::default(),
    }
}

fn linked(cell: CellId, start: u32, len: u32, url: &str) -> Command {
    Command::SetLink {
        cell,
        start,
        len,
        target: Some(LinkTarget::External(url.into())),
    }
}

fn insert(cell: CellId, at: u32, text: &str) -> Command {
    Command::InsertText {
        cell,
        at,
        text: text.into(),
    }
}

fn delete(cell: CellId, start: u32, len: u32) -> Command {
    Command::DeleteText { cell, start, len }
}

fn props(cells: &[CellId], props: &[Prop]) -> Command {
    Command::SetProps {
        cells: cells.to_vec(),
        props: props.to_vec(),
    }
}

const RED: Color = Color::rgb(255, 0, 0);
const BLUE: Color = Color::rgb(0, 0, 255);

#[test]
fn text_is_inserted_deleted_replaced_and_set() {
    let (mut f, id) = Fixture::text("hello");
    let changes = f.ok(insert(id, 5, " world"));
    assert_eq!(f.cell(id).text, "hello world");
    // A single text edit reports exactly one cell.
    assert_eq!(
        changes,
        Changes {
            changed: vec![id],
            ..Changes::default()
        }
    );
    assert_eq!(f.cell(id).last_edit, NOW);
    f.ok(insert(id, 0, ">"));
    assert_eq!(f.cell(id).text, ">hello world");
    f.ok(delete(id, 1, 6));
    assert_eq!(f.cell(id).text, ">world");
    f.ok(Command::ReplaceText {
        cell: id,
        start: 0,
        len: 1,
        text: "the ".into(),
    });
    assert_eq!(f.cell(id).text, "the world");
    f.ok(Command::SetText {
        cell: id,
        text: "new".into(),
    });
    assert_eq!(f.cell(id).text, "new");
    // Nothing to do: no change and no history entry.
    let units = f.e.undo_len();
    for nothing in [insert(id, 1, ""), delete(id, 2, 0)] {
        assert!(f.ok(nothing).is_empty());
    }
    assert!(f
        .ok(Command::SetText {
            cell: id,
            text: "new".into()
        })
        .is_empty());
    assert_eq!(f.e.undo_len(), units);
    // Out of range, or no such cell.
    for bad in [insert(id, 4, "x"), delete(id, 2, 2), delete(id, 4, 0)] {
        assert!(matches!(f.bad(bad), Error::Refused(_)));
    }
    assert!(matches!(
        f.bad(delete(id, u32::MAX, u32::MAX)),
        Error::Refused(_)
    ));
    let missing = CellId(Id::from_u128(1));
    assert!(matches!(f.bad(insert(missing, 0, "x")), Error::NotFound(_)));
    // Positions are code points: after the emoji is position 1.
    let (mut f, id) = Fixture::text("\u{1F600}b");
    f.ok(insert(id, 1, "a"));
    assert_eq!(f.cell(id).text, "\u{1F600}ab");
}

#[test]
fn no_time_given_means_no_stamp() {
    let (mut f, id) = Fixture::text("a");
    let before = f.cell(id).last_edit;
    f.e.apply(insert(id, 1, "b"), None, &mut f.ids).unwrap();
    assert_eq!(f.cell(id).last_edit, before);
    f.e.apply(insert(id, 1, "c"), Some(5), &mut f.ids).unwrap();
    assert_eq!(f.cell(id).last_edit, Some(5));
    f.e.undo().unwrap();
    assert_eq!(f.cell(id).last_edit, before);
}

#[test]
fn control_characters_become_spaces_and_are_invalid_in_documents() {
    let (mut f, id) = Fixture::text("ab");
    f.ok(insert(id, 1, "x\ny\tz\u{0}"));
    assert_eq!(f.cell(id).text, "ax y z b");
    f.ok(Command::ReplaceText {
        cell: id,
        start: 0,
        len: 1,
        text: "\r\n".into(),
    });
    assert_eq!(f.cell(id).text, "  x y z b");
    f.ok(Command::SetText {
        cell: id,
        text: "one\ntwo".into(),
    });
    assert_eq!(f.cell(id).text, "one two");
    // Notes may hold line breaks.
    f.ok(props(&[id], &[Prop::Note("one\ntwo".into())]));
    assert_eq!(f.cell(id).note, "one\ntwo");
    // The construction layer can write anything; validation and saving refuse it.
    let mut doc = f.e.into_document();
    doc.cell_mut(id).unwrap().text = "a\nb".into();
    assert!(matches!(doc.validate(), Err(Error::Invalid(_))));
    assert!(crate::format::to_string(&doc).is_err());
    assert!(Editor::new(doc.clone()).is_err());
    doc.cell_mut(id).unwrap().text = "ab".into();
    doc.tags_mut().insert("a\tb".into(), Tag::default());
    assert!(matches!(doc.validate(), Err(Error::Invalid(_))));
}

use crate::model::Tag;

#[test]
fn runs_follow_the_text_as_in_treesheets() {
    // "ab[cd]ef": cd bold.
    let (mut f, id) = Fixture::text("abcdef");
    f.ok(styled(id, 2, 2, "b"));
    assert_eq!(f.runs(id), [(2, 2, "b".into(), None)]);
    // Before the run: it moves. At its start: it moves (the text takes the style before).
    f.ok(insert(id, 0, "X"));
    assert_eq!(f.runs(id), [(3, 2, "b".into(), None)]);
    f.ok(insert(id, 3, "X"));
    assert_eq!(f.runs(id), [(4, 2, "b".into(), None)]);
    // Inside and at its end: it grows.
    f.ok(insert(id, 5, "X"));
    assert_eq!(f.runs(id), [(4, 3, "b".into(), None)]);
    f.ok(insert(id, 7, "XX"));
    assert_eq!(f.runs(id), [(4, 5, "b".into(), None)]);
    // After it: nothing.
    f.ok(insert(id, 10, "X"));
    assert_eq!(f.runs(id), [(4, 5, "b".into(), None)]);
    assert_eq!(f.cell(id).text, "XabXcXdXXeXf");
    // Deleting across its start, inside it, and all of it.
    f.ok(delete(id, 3, 2));
    assert_eq!(f.runs(id), [(3, 4, "b".into(), None)]);
    f.ok(delete(id, 4, 1));
    assert_eq!(f.runs(id), [(3, 3, "b".into(), None)]);
    f.ok(delete(id, 2, 5));
    assert_eq!(f.runs(id), []);

    // At the very start of the text a run that starts there grows.
    let (mut f, id) = Fixture::text("abc");
    f.ok(styled(id, 0, 1, "i"));
    f.ok(insert(id, 0, "X"));
    assert_eq!(f.runs(id), [(0, 2, "i".into(), None)]);

    // Deleting what separates two runs of the same style merges them; a different style
    // does not.
    let (mut f, id) = Fixture::text("aaXbbYcc");
    f.ok(styled(id, 0, 2, "b"));
    f.ok(styled(id, 3, 2, "b"));
    f.ok(styled(id, 6, 2, "i"));
    f.ok(delete(id, 2, 1));
    assert_eq!(
        f.runs(id),
        [(0, 4, "b".into(), None), (5, 2, "i".into(), None)]
    );
    f.ok(delete(id, 4, 1));
    assert_eq!(
        f.runs(id),
        [(0, 4, "b".into(), None), (4, 2, "i".into(), None)]
    );

    // Replacing takes the style of the first replaced character.
    let (mut f, id) = Fixture::text("abcdef");
    f.ok(styled(id, 2, 2, "b"));
    f.ok(Command::ReplaceText {
        cell: id,
        start: 1,
        len: 2,
        text: "XYZ".into(),
    });
    assert_eq!(
        (f.cell(id).text.as_str(), f.runs(id)),
        ("aXYZdef", vec![(4, 1, "b".into(), None)])
    );
    f.ok(Command::ReplaceText {
        cell: id,
        start: 4,
        len: 2,
        text: "QQQ".into(),
    });
    assert_eq!(
        (f.cell(id).text.as_str(), f.runs(id)),
        ("aXYZQQQf", vec![(4, 3, "b".into(), None)])
    );
    // Setting the whole text drops runs and links.
    f.ok(linked(id, 0, 2, "https://example.org"));
    f.ok(Command::SetText {
        cell: id,
        text: "plain".into(),
    });
    assert!(f.cell(id).runs.is_empty() && f.cell(id).links.is_empty());
}

#[test]
fn links_follow_the_text_as_in_treesheets() {
    let (mut f, id) = Fixture::text("abcdef");
    f.ok(linked(id, 2, 2, "u"));
    assert_eq!(f.links(id), [(2, 2, "u".into())]);
    // Typing at either end of a link does not extend it; inside does.
    f.ok(insert(id, 2, "X"));
    assert_eq!(f.links(id), [(3, 2, "u".into())]);
    f.ok(insert(id, 5, "X"));
    assert_eq!(f.links(id), [(3, 2, "u".into())]);
    f.ok(insert(id, 4, "X"));
    assert_eq!(f.links(id), [(3, 3, "u".into())]);
    assert_eq!(f.cell(id).text, "abXcXdXef");
    // Deleting shrinks it; deleting all of it removes it.
    f.ok(delete(id, 5, 3));
    assert_eq!(f.links(id), [(3, 2, "u".into())]);
    f.ok(delete(id, 0, 4));
    assert_eq!(f.links(id), [(0, 1, "u".into())]);
    f.ok(delete(id, 0, 1));
    assert_eq!(f.links(id), []);

    // Replacing inside a link keeps the replacement in the link.
    let (mut f, id) = Fixture::text("abcdef");
    f.ok(linked(id, 2, 2, "u"));
    f.ok(Command::ReplaceText {
        cell: id,
        start: 3,
        len: 2,
        text: "XYZ".into(),
    });
    assert_eq!(f.links(id), [(2, 4, "u".into())]);

    // Linking over existing links cuts them; the same target next to it merges.
    let (mut f, id) = Fixture::text("abcdefgh");
    f.ok(linked(id, 0, 3, "one"));
    f.ok(linked(id, 5, 3, "two"));
    f.ok(linked(id, 2, 4, "mid"));
    assert_eq!(
        f.links(id),
        [
            (0, 2, "one".into()),
            (2, 4, "mid".into()),
            (6, 2, "two".into())
        ]
    );
    f.ok(linked(id, 2, 4, "one"));
    assert_eq!(f.links(id), [(0, 6, "one".into()), (6, 2, "two".into())]);
    // Unlinking a part.
    f.ok(Command::SetLink {
        cell: id,
        start: 1,
        len: 6,
        target: None,
    });
    assert_eq!(f.links(id), [(0, 1, "one".into()), (7, 1, "two".into())]);
    // A link to a cell.
    let root = f.root();
    f.ok(Command::SetLink {
        cell: id,
        start: 3,
        len: 1,
        target: Some(LinkTarget::Cell {
            doc: None,
            cell: root,
        }),
    });
    assert_eq!(f.links(id)[1], (3, 1, format!("cell {root}")));
    // Refused: empty or unknown target, range outside the text.
    for target in [LinkTarget::External(String::new()), LinkTarget::Unknown] {
        let bad = Command::SetLink {
            cell: id,
            start: 0,
            len: 1,
            target: Some(target),
        };
        assert!(matches!(f.bad(bad), Error::Refused(_)));
    }
    assert!(matches!(f.bad(linked(id, 7, 2, "u")), Error::Refused(_)));
}

#[test]
fn ranges_are_styled_and_coloured() {
    let (mut f, id) = Fixture::text("abcdefgh");
    f.ok(styled(id, 2, 4, "b"));
    // Overlapping: the runs split at the ends of the range.
    f.ok(styled(id, 4, 4, "i"));
    assert_eq!(
        f.runs(id),
        [
            (2, 2, "b".into(), None),
            (4, 2, "bi".into(), None),
            (6, 2, "i".into(), None)
        ]
    );
    // Clearing a bit merges what becomes equal and drops what becomes plain.
    f.ok(Command::SetRangeStyle {
        cell: id,
        start: 0,
        len: 8,
        set: Style::default(),
        clear: style("i"),
    });
    assert_eq!(f.runs(id), [(2, 4, "b".into(), None)]);
    f.ok(Command::SetRangeColor {
        cell: id,
        start: 4,
        len: 4,
        color: Some(RED),
    });
    assert_eq!(
        f.runs(id),
        [
            (2, 2, "b".into(), None),
            (4, 2, "b".into(), Some(RED)),
            (6, 2, "".into(), Some(RED))
        ]
    );
    f.ok(Command::SetRangeColor {
        cell: id,
        start: 0,
        len: 8,
        color: None,
    });
    assert_eq!(f.runs(id), [(2, 4, "b".into(), None)]);
    // Styling a range with what it already has changes nothing.
    assert!(f.ok(styled(id, 3, 2, "b")).is_empty());
    assert!(f.ok(styled(id, 3, 0, "i")).is_empty());
    assert!(matches!(f.bad(styled(id, 7, 2, "b")), Error::Refused(_)));
    // A run that equals the cell's base style is no run.
    let (mut f, id) = Fixture::text("abcd");
    f.ok(Command::SetStyle {
        cells: vec![id],
        set: style("b"),
        clear: Style::default(),
    });
    assert!(f.ok(styled(id, 1, 2, "b")).is_empty());
    f.ok(Command::SetRangeStyle {
        cell: id,
        start: 1,
        len: 2,
        set: Style::default(),
        clear: style("b"),
    });
    assert_eq!(f.runs(id), [(1, 2, "".into(), None)]);
}

#[test]
fn a_user_perceived_character_is_never_cut() {
    // (text, a position inside a character, the next allowed position)
    let cases = [
        ("a\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}b", 3, 6), // family emoji
        ("ae\u{301}b", 2, 3),                                    // e + combining acute
        ("a\u{1F1E9}\u{1F1EA}b", 2, 3),                          // flag
        ("a\u{1100}\u{1161}\u{11A8}b", 2, 4),                    // Hangul jamo syllable
        ("a\u{1F44D}\u{1F3FD}b", 2, 3),                          // emoji + skin tone
    ];
    for (text, inside, next) in cases {
        let (mut f, id) = Fixture::text(text);
        let commands = [
            insert(id, inside, "x"),
            delete(id, 1, inside - 1),
            delete(id, inside, next - inside),
            Command::ReplaceText {
                cell: id,
                start: inside,
                len: next - inside,
                text: "x".into(),
            },
            styled(id, 0, inside, "b"),
            styled(id, inside, next - inside, "b"),
            linked(id, inside, next - inside, "u"),
            Command::SetRangeColor {
                cell: id,
                start: 0,
                len: inside,
                color: Some(RED),
            },
        ];
        for command in commands {
            let error = f.bad(command.clone());
            assert!(
                error.to_string().contains("cut a character"),
                "{command:?}: {error}"
            );
        }
        // Whole characters are fine.
        f.ok(styled(id, 1, next - 1, "b"));
        f.ok(linked(id, 1, next - 1, "u"));
        f.ok(insert(id, next, "x"));
        f.ok(delete(id, 1, next - 1));
        assert_eq!(f.cell(id).text, "axb");
        // The x typed after the bold character is bold (a run grows at its end); a link
        // does not grow there.
        assert_eq!(f.runs(id), [(1, 1, "b".into(), None)]);
        assert!(f.cell(id).links.is_empty());
    }
}

#[test]
fn properties_are_set_on_one_or_many_cells() {
    let mut f = Fixture::new(2, 2);
    let cells = f.cells.clone();
    let all = [
        Prop::CellColor(RED),
        Prop::TextColor(BLUE),
        Prop::RelSize(-2),
        Prop::TextAlign(TextAlign::Center),
        Prop::VertAlign(VertAlign::Bottom),
        Prop::Kind(CellKind::Code),
        Prop::Note("note\nwith lines".into()),
        Prop::DrawStyle(DrawStyle::Bubbles),
        Prop::Image(Some(ImageRef::new("a.png"))),
        Prop::ImageScale(2.5),
        Prop::LastEdit(Some(42)),
    ];
    // Several properties on several cells are one undo unit.
    let units = f.e.undo_len();
    let changes = f.ok(props(&cells, &all));
    assert_eq!(f.e.undo_len(), units + 1);
    let mut sorted = cells.clone();
    sorted.sort();
    assert_eq!(changes.changed, sorted);
    for id in &cells {
        let c = f.cell(*id);
        assert_eq!(
            (
                c.cell_color,
                c.text_color,
                c.rel_size,
                c.text_align,
                c.vert_align
            ),
            (RED, BLUE, -2, TextAlign::Center, VertAlign::Bottom)
        );
        assert_eq!(
            (c.kind, c.note.as_str(), c.draw_style),
            (CellKind::Code, "note\nwith lines", DrawStyle::Bubbles)
        );
        assert_eq!(
            c.image.as_ref().map(|i| (i.file.as_str(), i.scale)),
            Some(("a.png", 2.5))
        );
        assert_eq!(c.last_edit, Some(42));
    }
    // The same again changes nothing, except the stamp that colours give.
    let again = f.ok(props(&cells[..1], &all[2..8]));
    assert!(again.is_empty());
    f.ok(props(&cells[..1], &[Prop::Image(None)]));
    assert!(f.cell(cells[0]).image.is_none());
    // Colours stamp the edit time, as in TreeSheets.
    f.ok(props(&cells[..1], &[Prop::CellColor(BLUE)]));
    assert_eq!(f.cell(cells[0]).last_edit, NOW);

    // Grid properties.
    let root = f.root();
    let grid_props = [
        Prop::BorderColor(RED),
        Prop::OuterSpacing(7),
        Prop::Vertical(false),
    ];
    f.ok(props(&[root], &grid_props));
    let grid = f.grid(root);
    assert_eq!(
        (grid.border_color, grid.outer_spacing, grid.vertical),
        (RED, 7, false)
    );
    // If one cell of several cannot take a property, nothing changes anywhere.
    for prop in grid_props {
        let error = f.bad(props(&[root, cells[1]], &[Prop::RelSize(9), prop]));
        assert!(matches!(error, Error::Refused(_)));
    }
    for bad in [
        Prop::Image(Some(ImageRef::new("../x.png"))),
        Prop::Image(Some(ImageRef {
            scale: 0.0,
            ..ImageRef::new("x.png")
        })),
        Prop::ImageScale(f64::NAN),
    ] {
        assert!(matches!(
            f.bad(props(&[cells[1]], &[bad])),
            Error::Refused(_)
        ));
    }
    assert!(matches!(
        f.bad(props(&[cells[0]], &[Prop::ImageScale(1.0)])),
        Error::Refused(_)
    ));
    let missing = CellId(Id::from_u128(9));
    assert!(matches!(
        f.bad(props(&[cells[0], missing], &[Prop::RelSize(1)])),
        Error::NotFound(_)
    ));
}

#[test]
fn text_colour_and_cell_style_reach_the_runs() {
    let (mut f, id) = Fixture::text("abcdef");
    f.ok(styled(id, 0, 2, "b"));
    f.ok(Command::SetRangeColor {
        cell: id,
        start: 1,
        len: 3,
        color: Some(RED),
    });
    assert_eq!(f.runs(id).len(), 3);
    // Bold on the whole cell: base style and every run; the run that was only bold is
    // now the same as the base and disappears.
    f.ok(Command::SetStyle {
        cells: vec![id],
        set: style("b"),
        clear: Style::default(),
    });
    assert_eq!(letters(f.cell(id).style), "b");
    assert_eq!(f.runs(id), [(1, 3, "b".into(), Some(RED))]);
    f.ok(Command::SetStyle {
        cells: vec![id],
        set: style("i"),
        clear: style("b"),
    });
    assert_eq!(letters(f.cell(id).style), "i");
    assert_eq!(f.runs(id), [(1, 3, "i".into(), Some(RED))]);
    // A new text colour takes the colours off the runs, as in TreeSheets.
    f.ok(props(&[id], &[Prop::TextColor(BLUE)]));
    assert_eq!((f.cell(id).text_color, f.runs(id)), (BLUE, vec![]));
}

#[test]
fn column_widths_are_set() {
    let mut f = Fixture::new(3, 1);
    let root = f.root();
    let width = |column, width| Command::SetColumnWidth {
        cell: root,
        column,
        width,
    };
    let changes = f.ok(width(1, 120));
    assert_eq!(f.grid(root).col_widths, [80, 120, 80]);
    assert_eq!(changes.changed, [root]);
    assert!(changes.grids.is_empty());
    // TreeSheets' minimum.
    f.ok(width(2, 1));
    assert_eq!(f.grid(root).col_widths, [80, 120, MIN_COL_WIDTH]);
    assert!(f.ok(width(1, 120)).is_empty());
    assert!(matches!(f.bad(width(3, 10)), Error::Refused(_)));
    let leaf = f.cells[0];
    let bad = Command::SetColumnWidth {
        cell: leaf,
        column: 0,
        width: 10,
    };
    assert!(matches!(f.bad(bad), Error::Refused(_)));
}

#[test]
fn grids_are_created_and_removed() {
    let mut f = Fixture::new(2, 1);
    let (root, a) = (f.root(), f.cells[0]);
    f.ok(props(
        &[a],
        &[
            Prop::CellColor(RED),
            Prop::TextColor(BLUE),
            Prop::RelSize(3),
            Prop::TextAlign(TextAlign::Right),
        ],
    ));
    f.ok(Command::SetStyle {
        cells: vec![a],
        set: style("bi"),
        clear: Style::default(),
    });
    f.ok(props(
        &[root],
        &[
            Prop::BorderColor(RED),
            Prop::OuterSpacing(9),
            Prop::Vertical(false),
        ],
    ));
    f.ok(Command::SetColumnWidth {
        cell: root,
        column: 0,
        width: 33,
    });
    let changes = f.ok(Command::CreateGrid {
        cell: a,
        xs: 3,
        ys: 2,
    });
    let grid = f.grid(a).clone();
    assert_eq!((grid.xs(), grid.ys(), grid.cells().len()), (3, 2, 6));
    // As in TreeSheets: the new grid takes after the grid around it, its columns get the
    // width of the column it is in, and its cells take after the cell.
    assert_eq!(
        (grid.border_color, grid.outer_spacing, grid.vertical),
        (RED, 9, false)
    );
    assert_eq!(grid.col_widths, [33, 33, 33]);
    for id in grid.cells() {
        let c = f.cell(*id);
        assert_eq!(
            (
                c.cell_color,
                c.text_color,
                c.rel_size,
                c.text_align,
                letters(c.style).as_str()
            ),
            (RED, BLUE, 3, TextAlign::Right, "bi")
        );
        assert_eq!(
            (c.text.as_str(), c.parent(), c.last_edit),
            ("", Some(a), NOW)
        );
    }
    let mut created = grid.cells().to_vec();
    created.sort();
    assert_eq!(
        changes,
        Changes {
            created,
            grids: vec![a],
            ..Changes::default()
        }
    );
    // Refusals.
    assert!(matches!(
        f.bad(Command::CreateGrid {
            cell: a,
            xs: 1,
            ys: 1
        }),
        Error::Refused(_)
    ));
    let b = f.cells[1];
    for (xs, ys) in [(0, 1), (1, 0)] {
        assert!(matches!(
            f.bad(Command::CreateGrid { cell: b, xs, ys }),
            Error::Refused(_)
        ));
    }
    for (xs, ys) in [(u32::MAX, u32::MAX), (5000, 5000)] {
        assert!(matches!(
            f.bad(Command::CreateGrid { cell: b, xs, ys }),
            Error::Limit(_)
        ));
    }
    // Removing takes everything inside along.
    let inner = grid.cells()[0];
    f.ok(Command::CreateGrid {
        cell: inner,
        xs: 1,
        ys: 2,
    });
    assert_eq!(f.doc().cell_count(), 1 + 2 + 6 + 2);
    let changes = f.ok(Command::RemoveGrid { cell: a });
    assert_eq!(f.doc().cell_count(), 3);
    assert_eq!(
        (changes.removed.len(), changes.grids.as_slice()),
        (8, &[a][..])
    );
    assert!(f.cell(a).grid().is_none());
    assert!(matches!(
        f.bad(Command::RemoveGrid { cell: a }),
        Error::Refused(_)
    ));
}

#[test]
fn nesting_is_limited_for_commands() {
    let mut f = Fixture::new(1, 1);
    let mut cell = f.cells[0];
    for _ in 1..MAX_DEPTH {
        f.ok(Command::CreateGrid { cell, xs: 1, ys: 1 });
        cell = f.doc().child(cell, 0, 0).unwrap();
    }
    assert_eq!(f.doc().depth(cell), Some(MAX_DEPTH));
    assert!(matches!(
        f.bad(Command::CreateGrid { cell, xs: 1, ys: 1 }),
        Error::Limit(_)
    ));
    // Copying or moving a cell with a grid to the deepest level is refused too.
    let top = f.cells[0];
    let second = f.doc().child(top, 0, 0).unwrap();
    assert!(matches!(
        f.bad(Command::CopyCell {
            from: second,
            to: cell
        }),
        Error::Limit(_)
    ));
    f.ok(Command::InsertRows {
        cell: f.root(),
        at: 1,
        count: 1,
    });
    let empty = f.doc().child(f.root(), 0, 1).unwrap();
    f.ok(Command::CreateGrid {
        cell: empty,
        xs: 1,
        ys: 1,
    });
    assert!(matches!(
        f.bad(Command::SwapCells { a: empty, b: cell }),
        Error::Limit(_)
    ));
}

#[test]
fn rows_and_columns_are_inserted() {
    // 0 1 / 2 3
    let mut f = Fixture::new(2, 2);
    let root = f.root();
    let (c1, c2) = (f.cells[1], f.cells[2]);
    f.ok(props(&[c1], &[Prop::CellColor(RED), Prop::RelSize(2)]));
    f.ok(props(
        &[c2],
        &[Prop::CellColor(BLUE), Prop::TextAlign(TextAlign::Center)],
    ));
    f.ok(Command::SetColumnWidth {
        cell: root,
        column: 1,
        width: 50,
    });
    let changes = f.ok(Command::InsertRows {
        cell: root,
        at: 1,
        count: 1,
    });
    assert_eq!(f.texts(root), "0 1/ /2 3");
    assert_eq!(
        (changes.created.len(), changes.grids.as_slice()),
        (2, &[root][..])
    );
    assert!(changes.changed.is_empty());
    // The new cells take after the row above.
    let new = f.doc().child(root, 1, 1).unwrap();
    assert_eq!(
        (
            f.cell(new).cell_color,
            f.cell(new).rel_size,
            f.cell(new).last_edit
        ),
        (RED, 2, NOW)
    );
    assert_eq!(
        f.cell(f.doc().child(root, 0, 1).unwrap()).cell_color,
        Color::WHITE
    );
    // At the top they take after the first row; at the end they are appended.
    f.ok(Command::InsertRows {
        cell: root,
        at: 0,
        count: 2,
    });
    assert_eq!(f.texts(root), " / /0 1/ /2 3");
    assert_eq!(f.cell(f.doc().child(root, 1, 0).unwrap()).cell_color, RED);
    f.ok(Command::InsertRows {
        cell: root,
        at: 5,
        count: 1,
    });
    assert_eq!(f.texts(root), " / /0 1/ /2 3/ ");
    let last = f.cell(f.doc().child(root, 0, 5).unwrap());
    assert_eq!(
        (last.cell_color, last.text_align),
        (BLUE, TextAlign::Center)
    );
    // Columns: after the cell to the left, or the first column when inserted before it.
    f.ok(Command::DeleteRows {
        cell: root,
        at: 0,
        count: 2,
    });
    f.ok(Command::DeleteRows {
        cell: root,
        at: 1,
        count: 1,
    });
    f.ok(Command::DeleteRows {
        cell: root,
        at: 2,
        count: 1,
    });
    assert_eq!(f.texts(root), "0 1/2 3");
    f.ok(Command::InsertColumns {
        cell: root,
        at: 2,
        count: 2,
    });
    assert_eq!(f.texts(root), "0 1  /2 3  ");
    assert_eq!(f.cell(f.doc().child(root, 3, 0).unwrap()).cell_color, RED);
    // New columns get the width of the column the grid's cell is in; the root has none.
    assert_eq!(f.grid(root).col_widths, [80, 50, 80, 80]);
    f.ok(Command::InsertColumns {
        cell: root,
        at: 0,
        count: 1,
    });
    assert_eq!(f.texts(root), " 0 1  / 2 3  ");
    assert_eq!(f.cell(f.doc().child(root, 0, 1).unwrap()).cell_color, BLUE);
    assert_eq!(f.grid(root).col_widths, [80, 80, 50, 80, 80]);
    f.ok(Command::InsertColumns {
        cell: root,
        at: 2,
        count: 1,
    });
    assert_eq!(f.texts(root), " 0  1  / 2  3  ");
    // Refusals and nothing-to-do.
    assert!(f
        .ok(Command::InsertRows {
            cell: root,
            at: 0,
            count: 0
        })
        .is_empty());
    assert!(matches!(
        f.bad(Command::InsertRows {
            cell: root,
            at: 3,
            count: 1
        }),
        Error::Refused(_)
    ));
    assert!(matches!(
        f.bad(Command::InsertColumns {
            cell: root,
            at: 7,
            count: 1
        }),
        Error::Refused(_)
    ));
    assert!(matches!(
        f.bad(Command::InsertRows {
            cell: c1,
            at: 0,
            count: 1
        }),
        Error::Refused(_)
    ));
    assert!(matches!(
        f.bad(Command::InsertRows {
            cell: root,
            at: 0,
            count: u32::MAX
        }),
        Error::Limit(_)
    ));
    assert!(matches!(
        f.bad(Command::InsertColumns {
            cell: root,
            at: 0,
            count: 3_000_000
        }),
        Error::Limit(_)
    ));
}

#[test]
fn rows_and_columns_are_deleted() {
    // 0 1 2 / 3 4 5 / 6 7 8
    let mut f = Fixture::new(3, 3);
    let root = f.root();
    let c4 = f.cells[4];
    f.ok(Command::CreateGrid {
        cell: c4,
        xs: 2,
        ys: 2,
    });
    f.ok(Command::SetColumnWidth {
        cell: root,
        column: 1,
        width: 50,
    });
    let changes = f.ok(Command::DeleteRows {
        cell: root,
        at: 1,
        count: 1,
    });
    assert_eq!(f.texts(root), "0 1 2/6 7 8");
    // The row's three cells and the four cells of the grid inside one of them.
    assert_eq!(
        (changes.removed.len(), changes.grids.as_slice()),
        (7, &[root][..])
    );
    assert_eq!(f.doc().cell_count(), 7);
    f.ok(Command::DeleteColumns {
        cell: root,
        at: 0,
        count: 2,
    });
    assert_eq!(f.texts(root), "2/8");
    assert_eq!(f.grid(root).col_widths, [80]);
    f.e.undo().unwrap();
    assert_eq!(f.grid(root).col_widths, [80, 50, 80]);
    f.ok(Command::DeleteColumns {
        cell: root,
        at: 1,
        count: 1,
    });
    assert_eq!(
        (f.texts(root).as_str(), f.grid(root).col_widths.as_slice()),
        ("0 2/6 8", &[80, 80][..])
    );
    assert!(f
        .ok(Command::DeleteRows {
            cell: root,
            at: 0,
            count: 0
        })
        .is_empty());
    for bad in [
        Command::DeleteRows {
            cell: root,
            at: 1,
            count: 2,
        },
        Command::DeleteColumns {
            cell: root,
            at: 2,
            count: 1,
        },
        Command::DeleteRows {
            cell: root,
            at: u32::MAX,
            count: 2,
        },
        // The root keeps its grid.
        Command::DeleteRows {
            cell: root,
            at: 0,
            count: 2,
        },
        Command::DeleteColumns {
            cell: root,
            at: 0,
            count: 2,
        },
    ] {
        assert!(matches!(f.bad(bad), Error::Refused(_)));
    }
    // Deleting every row of another grid removes the grid.
    let c0 = f.cells[0];
    f.ok(Command::CreateGrid {
        cell: c0,
        xs: 2,
        ys: 2,
    });
    let changes = f.ok(Command::DeleteRows {
        cell: c0,
        at: 0,
        count: 2,
    });
    assert!(f.cell(c0).grid().is_none());
    assert_eq!(
        (changes.removed.len(), changes.grids.as_slice()),
        (4, &[c0][..])
    );
}

#[test]
fn cells_are_cleared() {
    let mut f = Fixture::new(2, 1);
    let (a, b) = (f.cells[0], f.cells[1]);
    f.ok(styled(a, 0, 1, "b"));
    f.ok(linked(a, 0, 1, "u"));
    f.ok(props(
        &[a],
        &[
            Prop::Kind(CellKind::Formula),
            Prop::Image(Some(ImageRef::new("a.png"))),
            Prop::Note("n".into()),
            Prop::CellColor(RED),
        ],
    ));
    f.ok(Command::CreateGrid {
        cell: a,
        xs: 1,
        ys: 2,
    });
    f.ok(props(&[b], &[Prop::Kind(CellKind::Code)]));
    let stamp = f.cell(a).last_edit;
    let changes = f.ok(Command::ClearCells { cells: vec![a, b] });
    let c = f.cell(a);
    assert!(c.text.is_empty() && c.runs.is_empty() && c.links.is_empty());
    assert!(c.image.is_none() && c.grid().is_none());
    // As TreeSheets' Clear: a formula becomes data, other kinds, colours and the note stay.
    assert_eq!(
        (c.kind, c.cell_color, c.note.as_str(), c.last_edit),
        (CellKind::Data, RED, "n", stamp)
    );
    assert_eq!(f.cell(b).kind, CellKind::Code);
    assert_eq!(
        (
            changes.changed.len(),
            changes.removed.len(),
            changes.grids.as_slice()
        ),
        (2, 2, &[a][..])
    );
    // Already empty: nothing to do.
    assert!(f.ok(Command::ClearCells { cells: vec![a] }).is_empty());
    // A cell and a cell inside it: the inner one goes with the grid.
    f.ok(Command::CreateGrid {
        cell: a,
        xs: 1,
        ys: 1,
    });
    let inner = f.doc().child(a, 0, 0).unwrap();
    f.ok(Command::ClearCells {
        cells: vec![a, inner],
    });
    assert!(f.doc().cell(inner).is_none());
    let missing = CellId(Id::from_u128(3));
    assert!(matches!(
        f.bad(Command::ClearCells {
            cells: vec![b, missing]
        }),
        Error::NotFound(_)
    ));
}

#[test]
fn deleting_cells_removes_rows_and_columns_left_empty_as_treesheets_does() {
    let del = |cell, x, y, xs, ys| Command::DeleteCells { cell, x, y, xs, ys };
    // 0 1 2 / 3 4 5 / 6 7 8
    let mut f = Fixture::new(3, 3);
    let root = f.root();
    // One cell: the rest of its row and column have content, so it is only emptied.
    f.ok(del(root, 1, 1, 1, 1));
    assert_eq!(f.texts(root), "0 1 2/3  5/6 7 8");
    // A whole row: the row goes.
    f.ok(del(root, 0, 0, 3, 1));
    assert_eq!(f.texts(root), "3  5/6 7 8");
    // A whole column.
    f.ok(del(root, 2, 0, 1, 2));
    assert_eq!(f.texts(root), "3 /6 7");
    // The cell that completes an empty column: the column goes (not the row).
    f.ok(del(root, 1, 1, 1, 1));
    assert_eq!(f.texts(root), "3/6");
    // Everything in the root: emptied, but the root keeps its grid.
    f.ok(del(root, 0, 0, 1, 2));
    assert_eq!(f.texts(root), "/");
    assert_eq!(f.doc().cell_count(), 3);

    // In another cell, deleting everything removes the grid.
    let mut f = Fixture::new(1, 1);
    let a = f.cells[0];
    f.ok(Command::CreateGrid {
        cell: a,
        xs: 2,
        ys: 2,
    });
    let inner = f.grid(a).cells().to_vec();
    for (n, id) in inner.iter().enumerate() {
        f.ok(Command::SetText {
            cell: *id,
            text: format!("i{n}"),
        });
    }
    f.ok(del(a, 0, 0, 2, 2));
    assert!(f.cell(a).grid().is_none());
    f.e.undo().unwrap();
    assert_eq!(f.texts(a), "i0 i1/i2 i3");
    // A cell in an empty row and an empty column, wider than high or square: the row.
    f.ok(Command::ClearCells {
        cells: vec![inner[1], inner[2], inner[3]],
    });
    f.ok(del(a, 1, 1, 1, 1));
    assert_eq!(f.texts(a), "i0 ");
    // Both the row and the column are now empty, and the selection spans the whole
    // row but not the whole column: the column goes, the grid stays.
    f.ok(Command::ClearCells {
        cells: vec![inner[0]],
    });
    f.ok(Command::InsertRows {
        cell: a,
        at: 1,
        count: 1,
    });
    f.ok(del(a, 0, 0, 2, 1));
    assert_eq!((f.grid(a).xs(), f.grid(a).ys()), (2, 1));
    // Refusals.
    for bad in [
        del(a, 0, 0, 0, 1),
        del(a, 0, 0, 3, 1),
        del(a, 2, 0, 1, 1),
        del(a, 0, 1, 1, 1),
        del(a, u32::MAX, 0, 2, 1),
    ] {
        assert!(matches!(f.bad(bad), Error::Refused(_)));
    }
}

#[test]
fn rows_and_columns_are_moved() {
    // 0 1 2 / 3 4 5 / 6 7 8 / 9 10 11
    let mut f = Fixture::new(3, 4);
    let root = f.root();
    let rows = |from, count, to| Command::MoveRows {
        cell: root,
        from,
        count,
        to,
    };
    let cols = |from, count, to| Command::MoveColumns {
        cell: root,
        from,
        count,
        to,
    };
    let changes = f.ok(rows(0, 1, 3));
    assert_eq!(f.texts(root), "3 4 5/6 7 8/9 10 11/0 1 2");
    assert_eq!(
        changes,
        Changes {
            grids: vec![root],
            ..Changes::default()
        }
    );
    f.ok(rows(2, 2, 0));
    assert_eq!(f.texts(root), "9 10 11/0 1 2/3 4 5/6 7 8");
    f.ok(rows(0, 2, 1));
    assert_eq!(f.texts(root), "3 4 5/9 10 11/0 1 2/6 7 8");
    f.ok(Command::SetColumnWidth {
        cell: root,
        column: 0,
        width: 10,
    });
    f.ok(cols(0, 1, 2));
    assert_eq!(f.texts(root), "4 5 3/10 11 9/1 2 0/7 8 6");
    assert_eq!(f.grid(root).col_widths, [80, 80, 10]);
    f.ok(cols(1, 2, 0));
    assert_eq!(f.texts(root), "5 3 4/11 9 10/2 0 1/8 6 7");
    assert_eq!(f.grid(root).col_widths, [80, 10, 80]);
    assert!(f.ok(rows(1, 2, 1)).is_empty());
    assert!(f.ok(rows(1, 0, 3)).is_empty());
    for bad in [
        rows(3, 2, 0),
        rows(0, 2, 3),
        cols(0, 4, 0),
        rows(u32::MAX, 2, 0),
    ] {
        assert!(matches!(f.bad(bad), Error::Refused(_)));
    }
}

#[test]
fn cells_are_swapped_and_moved() {
    let mut f = Fixture::new(2, 2);
    let root = f.root();
    let (a, b, c, d) = (f.cells[0], f.cells[1], f.cells[2], f.cells[3]);
    f.ok(Command::CreateGrid {
        cell: a,
        xs: 1,
        ys: 2,
    });
    let (a0, a1) = (
        f.doc().child(a, 0, 0).unwrap(),
        f.doc().child(a, 0, 1).unwrap(),
    );
    f.ok(Command::SetText {
        cell: a0,
        text: "inner".into(),
    });
    f.ok(props(&[d], &[Prop::CellColor(RED)]));
    // In the same grid.
    let changes = f.ok(Command::SwapCells { a: b, b: c });
    assert_eq!(f.texts(root), "0 2/1 3");
    assert_eq!(
        changes,
        Changes {
            grids: vec![root],
            ..Changes::default()
        }
    );
    // Across grids, taking a grid along.
    let changes = f.ok(Command::SwapCells { a: d, b: a0 });
    assert_eq!(
        (f.texts(root).as_str(), f.texts(a).as_str()),
        ("0 2/1 inner", "3/")
    );
    assert_eq!(
        (f.cell(d).parent(), f.cell(a0).parent()),
        (Some(a), Some(root))
    );
    let mut grids = vec![root, a];
    grids.sort();
    assert_eq!(
        changes,
        Changes {
            grids,
            ..Changes::default()
        }
    );
    assert!(f.ok(Command::SwapCells { a: b, b }).is_empty());
    for (x, y) in [(a, d), (d, a), (root, b), (b, root)] {
        assert!(matches!(
            f.bad(Command::SwapCells { a: x, b: y }),
            Error::Refused(_)
        ));
    }

    // Moving: the cell goes to where an empty cell is, with its ID, properties and grid.
    assert!(f.cell(a1).text.is_empty());
    let error = f.bad(Command::MoveCell { from: a, to: a1 });
    assert!(error.to_string().contains("inside"), "{error}");
    assert!(matches!(
        f.bad(Command::MoveCell { from: b, to: c }),
        Error::Refused(_)
    ));
    assert!(matches!(
        f.bad(Command::MoveCell { from: a1, to: a1 }),
        Error::Refused(_)
    ));
    assert!(matches!(
        f.bad(Command::MoveCell { from: root, to: a1 }),
        Error::Refused(_)
    ));
    let changes = f.ok(Command::MoveCell { from: a0, to: a1 });
    assert_eq!(
        (f.texts(root).as_str(), f.texts(a).as_str()),
        ("0 2/1 ", "3/inner")
    );
    assert_eq!(changes.grids.len(), 2);
    assert_eq!(f.cell(d).cell_color, RED);
    // An image or a grid makes a cell not empty.
    f.ok(Command::ClearCells { cells: vec![b] });
    f.ok(props(&[b], &[Prop::Image(Some(ImageRef::new("i.png")))]));
    assert!(matches!(
        f.bad(Command::MoveCell { from: c, to: b }),
        Error::Refused(_)
    ));
    // Moving a cell with a grid into another grid.
    f.ok(props(&[b], &[Prop::Image(None)]));
    f.ok(Command::MoveCell { from: a, to: b });
    assert_eq!(f.texts(root), " 2/0 ");
    assert_eq!(f.texts(a), "3/inner");
}

#[test]
fn a_subtree_is_copied_with_fresh_ids_and_its_own_links() {
    let mut f = Fixture::new(2, 2);
    let root = f.root();
    let (a, b, c) = (f.cells[0], f.cells[1], f.cells[2]);
    f.ok(Command::CreateGrid {
        cell: a,
        xs: 2,
        ys: 1,
    });
    let (a0, a1) = (
        f.doc().child(a, 0, 0).unwrap(),
        f.doc().child(a, 1, 0).unwrap(),
    );
    f.ok(Command::SetText {
        cell: a0,
        text: "abcd".into(),
    });
    f.ok(Command::CreateGrid {
        cell: a1,
        xs: 1,
        ys: 1,
    });
    let deep = f.doc().child(a1, 0, 0).unwrap();
    let own = f.doc().id();
    let other = DocId(Id::from_u128(77));
    let link = |start, doc, cell| Command::SetLink {
        cell: a0,
        start,
        len: 1,
        target: Some(LinkTarget::Cell { doc, cell }),
    };
    f.ok(link(0, None, deep)); // into the copied part
    f.ok(link(1, Some(own), a)); // to the copied cell itself, with the document named
    f.ok(link(2, None, c)); // outside the copied part
    f.ok(link(3, Some(other), deep)); // another document
    f.ok(styled(a, 0, 1, "b"));
    f.ok(props(
        &[a],
        &[
            Prop::CellColor(RED),
            Prop::Note("n".into()),
            Prop::RelSize(1),
            Prop::Image(Some(ImageRef::new("p.png"))),
        ],
    ));
    f.ok(Command::SetColumnWidth {
        cell: a,
        column: 1,
        width: 30,
    });
    f.ok(Command::CreateGrid {
        cell: b,
        xs: 1,
        ys: 3,
    });
    let old_inside = f.grid(b).cells().to_vec();

    let before = f.doc().cell_count();
    let changes = f.ok(Command::CopyCell { from: a, to: b });
    assert_eq!(f.doc().cell_count(), before);
    assert_eq!(
        (changes.changed.as_slice(), changes.grids.as_slice()),
        (&[b][..], &[b][..])
    );
    assert_eq!((changes.created.len(), changes.removed.len()), (3, 3));
    assert!(old_inside.iter().all(|id| f.doc().cell(*id).is_none()));
    // Same content, other IDs, same place.
    let (ca, cb) = (f.cell(a), f.cell(b));
    assert_eq!(
        (
            &cb.text,
            &cb.runs,
            cb.cell_color,
            &cb.note,
            cb.rel_size,
            &cb.image
        ),
        (
            &ca.text,
            &ca.runs,
            ca.cell_color,
            &ca.note,
            ca.rel_size,
            &ca.image
        )
    );
    assert_eq!(f.doc().child(root, 1, 0), Some(b));
    assert_eq!(f.grid(b).col_widths, [80, 30]);
    let (b0, b1) = (
        f.doc().child(b, 0, 0).unwrap(),
        f.doc().child(b, 1, 0).unwrap(),
    );
    let copy_deep = f.doc().child(b1, 0, 0).unwrap();
    assert!(![a0, a1, deep].contains(&b0) && ![a0, a1, deep].contains(&b1) && copy_deep != deep);
    assert_eq!(f.cell(b0).text, "abcd");
    let targets: Vec<LinkTarget> = f.cell(b0).links.iter().map(|l| l.target.clone()).collect();
    assert_eq!(
        targets,
        [
            LinkTarget::Cell {
                doc: None,
                cell: copy_deep
            },
            LinkTarget::Cell {
                doc: Some(own),
                cell: b
            },
            LinkTarget::Cell { doc: None, cell: c },
            LinkTarget::Cell {
                doc: Some(other),
                cell: deep
            },
        ]
    );
    // The original is untouched.
    assert_eq!(
        f.cell(a0).links[0].target,
        LinkTarget::Cell {
            doc: None,
            cell: deep
        }
    );

    // Undo and redo bring back the very same IDs.
    f.e.undo().unwrap();
    assert_eq!(f.grid(b).cells(), old_inside);
    f.e.redo().unwrap();
    assert_eq!(f.grid(b).cells(), [b0, b1]);

    // Into a cell inside the copied one, and over a cell that contains it.
    f.ok(Command::CopyCell { from: a, to: deep });
    assert_eq!(f.texts(deep), "abcd ");
    let inner_copy = f.doc().child(deep, 1, 0).unwrap();
    assert!(
        f.texts(inner_copy).is_empty(),
        "a copy of the cell as it was before"
    );
    f.ok(Command::CopyCell { from: a0, to: a });
    assert_eq!(f.cell(a).text, "abcd");
    assert!(f.cell(a).grid().is_none() && f.doc().cell(a0).is_none());
    // A leaf over a cell: plain overwrite.
    f.ok(Command::CopyCell { from: c, to: a });
    assert_eq!((f.cell(a).text.as_str(), f.cell(a).links.len()), ("2", 0));
    assert!(matches!(
        f.bad(Command::CopyCell { from: a, to: a }),
        Error::Refused(_)
    ));
    let missing = CellId(Id::from_u128(3));
    assert!(matches!(
        f.bad(Command::CopyCell {
            from: missing,
            to: a
        }),
        Error::NotFound(_)
    ));
    assert!(matches!(
        f.bad(Command::CopyCell {
            from: a,
            to: missing
        }),
        Error::NotFound(_)
    ));

    // An ID source that repeats itself is refused, and nothing changes.
    struct Stuck;
    impl IdSource for Stuck {
        fn next_id(&mut self) -> Id {
            Id::from_u128(5)
        }
    }
    let before = f.doc().clone();
    for command in [
        Command::CopyCell { from: b, to: c },
        Command::CreateGrid {
            cell: c,
            xs: 2,
            ys: 1,
        },
        Command::InsertRows {
            cell: root,
            at: 0,
            count: 1,
        },
    ] {
        assert!(matches!(
            f.e.apply(command, NOW, &mut Stuck),
            Err(Error::Refused(_))
        ));
        assert_eq!(*f.doc(), before);
    }
}

#[test]
fn tags_are_added_recoloured_and_removed() {
    let mut f = Fixture::new(1, 1);
    let tag = |name: &str, cell_color, text_color| Command::SetTag {
        name: name.into(),
        cell_color,
        text_color,
    };
    let changes = f.ok(tag("todo", RED, BLUE));
    assert_eq!(
        changes,
        Changes {
            tags: vec!["todo".into()],
            ..Changes::default()
        }
    );
    assert_eq!(f.doc().tags()["todo"].cell_color, RED);
    assert!(f.ok(tag("todo", RED, BLUE)).is_empty());
    f.ok(tag("todo", BLUE, BLUE));
    assert_eq!(f.doc().tags()["todo"].cell_color, BLUE);
    f.ok(tag("done", RED, RED));
    assert_eq!(f.doc().tags().len(), 2);
    f.ok(Command::RemoveTag {
        name: "todo".into(),
    });
    assert_eq!(f.doc().tags().keys().collect::<Vec<_>>(), ["done"]);
    assert!(matches!(
        f.bad(Command::RemoveTag {
            name: "todo".into()
        }),
        Error::NotFound(_)
    ));
    for name in ["", "a\nb"] {
        assert!(matches!(f.bad(tag(name, RED, RED)), Error::Refused(_)));
    }
    // Unknown fields of a tag survive a change of colour.
    let mut doc = f.e.into_document();
    doc.tags_mut()
        .get_mut("done")
        .unwrap()
        .extra
        .insert("zz".into(), 1.into());
    let mut e = Editor::new(doc).unwrap();
    e.apply(tag("done", BLUE, RED), NOW, &mut f.ids).unwrap();
    assert_eq!(e.document().tags()["done"].extra["zz"], 1);
}

#[test]
fn history_groups_bounds_and_redo() {
    let (mut f, id) = Fixture::text("");
    let original = f.doc().clone();
    // A group undoes as one unit.
    f.e.begin_group();
    for n in 0..3 {
        f.run(insert(id, n, "x")).unwrap();
    }
    assert!(
        f.e.undo().is_err() && f.e.redo().is_err(),
        "no undo inside a group"
    );
    // A failing command inside a group undoes only itself.
    f.bad(insert(id, 99, "x"));
    f.e.begin_group(); // nested groups count as the outer one
    f.run(styled(id, 0, 1, "b")).unwrap();
    f.e.end_group();
    assert_eq!(f.e.undo_len(), 0);
    f.e.end_group();
    f.e.end_group(); // one too many: ignored
    assert_eq!(f.e.undo_len(), 1);
    let grouped = f.doc().clone();
    let changes = f.e.undo().unwrap().unwrap();
    assert_eq!(*f.doc(), original);
    assert_eq!(changes.changed, [id]);
    f.e.redo().unwrap();
    assert_eq!(*f.doc(), grouped);
    // An empty group leaves nothing behind.
    f.e.begin_group();
    f.e.end_group();
    assert_eq!(f.e.undo_len(), 1);

    // A new command after an undo empties the redo history.
    f.run(insert(id, 3, "y")).unwrap();
    f.run(insert(id, 4, "z")).unwrap();
    f.e.undo().unwrap();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (2, 1));
    f.bad(insert(id, 99, "x"));
    assert!(f.run(insert(id, 0, "")).unwrap().is_empty());
    assert_eq!(
        f.e.redo_len(),
        1,
        "failed and empty commands leave redo alone"
    );
    f.run(insert(id, 4, "w")).unwrap();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (3, 0));
    assert_eq!(f.e.redo().unwrap(), None);
    assert_eq!(f.cell(id).text, "xxxyw");

    // A bound on the history drops the oldest units.
    f.e.set_history_limit(Some(2));
    assert_eq!(f.e.undo_len(), 2);
    f.run(insert(id, 0, "1")).unwrap();
    f.run(insert(id, 0, "2")).unwrap();
    assert_eq!(f.e.undo_len(), 2);
    while f.e.undo().unwrap().is_some() {}
    assert_eq!(f.cell(id).text, "xxxyw");
    assert_eq!(f.e.redo_len(), 2);
    while f.e.redo().unwrap().is_some() {}
    assert_eq!((f.cell(id).text.as_str(), f.e.undo_len()), ("21xxxyw", 2));
    f.e.clear_history();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (0, 0));
    assert_eq!(f.e.undo().unwrap(), None);
}

#[test]
fn undoing_a_creation_and_redoing_it_gives_the_same_ids() {
    let mut f = Fixture::new(2, 2);
    let root = f.root();
    f.run(Command::InsertRows {
        cell: root,
        at: 1,
        count: 2,
    })
    .unwrap();
    f.run(Command::CreateGrid {
        cell: f.cells[0],
        xs: 2,
        ys: 2,
    })
    .unwrap();
    f.run(Command::InsertColumns {
        cell: f.cells[0],
        at: 1,
        count: 1,
    })
    .unwrap();
    let after = f.doc().clone();
    let saved = crate::format::to_string(&after).unwrap();
    while f.e.undo().unwrap().is_some() {}
    assert_eq!(f.doc().cell_count(), 5);
    while f.e.redo().unwrap().is_some() {}
    assert_eq!(*f.doc(), after);
    assert_eq!(crate::format::to_string(f.doc()).unwrap(), saved);
}

#[test]
fn folding_is_view_state_and_follows_the_changes() {
    let mut f = Fixture::new(2, 1);
    let (root, a, b) = (f.root(), f.cells[0], f.cells[1]);
    f.ok(Command::CreateGrid {
        cell: a,
        xs: 1,
        ys: 1,
    });
    let inner = f.doc().child(a, 0, 0).unwrap();
    f.ok(Command::CreateGrid {
        cell: inner,
        xs: 1,
        ys: 1,
    });
    let mut view = ViewState::new(f.doc().id());
    let units = f.e.undo_len();
    assert!(view.set_folded(f.doc(), a, true).unwrap());
    assert!(!view.set_folded(f.doc(), a, true).unwrap());
    view.set_folded(f.doc(), inner, true).unwrap();
    view.set_folded(f.doc(), root, true).unwrap();
    assert!(view.is_folded(a) && view.is_folded(inner));
    assert!(matches!(
        view.set_folded(f.doc(), b, true),
        Err(Error::Refused(_))
    ));
    assert!(view
        .set_folded(f.doc(), CellId(Id::from_u128(1)), true)
        .is_err());
    assert_eq!(f.e.undo_len(), units, "folding is not in the history");
    view.selection = Some(Selection {
        cell: inner,
        xs: 1,
        ys: 1,
        cursor: None,
        extra: Extra::new(),
    });
    view.zoom = Some(inner);

    // The grid of `a` goes: its fold and everything about the cells inside go too.
    let changes = f.run(Command::RemoveGrid { cell: a }).unwrap();
    view.apply_changes(f.doc(), &changes);
    assert_eq!(view.folded.iter().collect::<Vec<_>>(), [&root]);
    assert_eq!((view.selection.clone(), view.zoom), (None, None));
    // Undo brings the cells back, not their folds.
    let changes = f.e.undo().unwrap().unwrap();
    view.apply_changes(f.doc(), &changes);
    assert!(f.cell(a).grid().is_some() && !view.is_folded(a) && !view.is_folded(inner));
    assert!(view.set_folded(f.doc(), root, false).unwrap());
}

/// What milestone 3b builds on: any rearrangement of existing cells is `SetGrid` steps.
#[test]
fn the_general_grid_step_rearranges_and_reverses() {
    let (e, _, cells) = editor(2, 2);
    let mut doc = e.into_document();
    let original = doc.clone();
    let root = doc.root();
    // Transpose-like: the same cells in another order and shape, 1 by 4.
    let mut grid = Grid::new(1, 4, vec![cells[3], cells[1], cells[2], cells[0]]);
    grid.col_widths = vec![40];
    let mut seen = ChangeSet::default();
    let undo_a = Step::SetGrid {
        cell: root,
        grid: Some(grid),
    }
    .apply(&mut doc, &mut seen)
    .unwrap();
    doc.validate().unwrap();
    // Wrap-like: three cells move into a new grid inside the fourth.
    let outer = Grid::new(1, 1, vec![cells[0]]);
    let inner = Grid::new(3, 1, vec![cells[1], cells[2], cells[3]]);
    let undo_b = Step::SetGrid {
        cell: root,
        grid: Some(outer),
    }
    .apply(&mut doc, &mut seen)
    .unwrap();
    let undo_c = Step::SetGrid {
        cell: cells[0],
        grid: Some(inner),
    }
    .apply(&mut doc, &mut seen)
    .unwrap();
    doc.validate().unwrap();
    assert_eq!(doc.cell(cells[2]).unwrap().parent(), Some(cells[0]));
    let changes = seen.finish(&doc);
    let mut grids = vec![root, cells[0]];
    grids.sort();
    assert_eq!(
        changes,
        Changes {
            changed: vec![root],
            grids,
            ..Changes::default()
        }
    );
    let mut seen = ChangeSet::default();
    for step in [undo_c, undo_b, undo_a] {
        step.apply(&mut doc, &mut seen).unwrap();
    }
    assert_eq!(doc, original);
    // A step that cannot apply changes nothing.
    let missing = CellId(Id::from_u128(1));
    let bad = [
        Step::SetGrid {
            cell: root,
            grid: Some(Grid::new(1, 1, vec![missing])),
        },
        Step::SetGrid {
            cell: root,
            grid: Some(Grid::new(2, 1, vec![cells[0]])),
        },
        Step::RemoveCells(vec![cells[0], missing]),
        Step::RemoveCells(vec![root]),
        Step::AddCells(vec![doc.cell(cells[0]).unwrap().clone()]),
        Step::RemoveLines {
            owner: root,
            axis: super::step::Axis::Rows,
            at: 0,
            count: 2,
        },
        Step::InsertLines {
            owner: root,
            axis: super::step::Axis::Rows,
            at: 0,
            ids: vec![cells[0]],
            widths: vec![],
        },
        Step::MoveLines {
            owner: root,
            axis: super::step::Axis::Columns,
            from: 1,
            count: 2,
            to: 0,
        },
        Step::Splice {
            cell: cells[0],
            at: 1,
            del: 1,
            ins: String::new(),
        },
        Step::Swap {
            a: root,
            b: cells[0],
        },
    ];
    for step in bad {
        assert!(step.clone().apply(&mut doc, &mut seen).is_err(), "{step:?}");
        assert_eq!(doc, original);
    }
}

#[test]
fn run_and_link_fields_keep_unknown_fields_apart() {
    // Two neighbouring runs that differ only in an unknown field do not merge.
    let (f, id) = Fixture::text("abcd");
    let mut doc = f.e.into_document();
    let mut extra = Extra::new();
    extra.insert("zz".into(), 1.into());
    let run = |start, extra: &Extra| Run {
        start,
        len: 2,
        style: style("b"),
        color: None,
        extra: extra.clone(),
    };
    doc.cell_mut(id).unwrap().runs = vec![run(0, &Extra::new()), run(2, &extra)];
    let link = |start, extra: &Extra| Link {
        start,
        len: 2,
        target: LinkTarget::External("u".into()),
        extra: extra.clone(),
    };
    doc.cell_mut(id).unwrap().links = vec![link(0, &Extra::new()), link(2, &extra)];
    let mut e = Editor::new(doc).unwrap();
    let mut ids = SeededIds::from_u64(1);
    e.apply(styled(id, 0, 4, "i"), NOW, &mut ids).unwrap();
    assert_eq!(e.document().cell(id).unwrap().runs.len(), 2);
    e.apply(delete(id, 1, 2), NOW, &mut ids).unwrap();
    let c = e.document().cell(id).unwrap();
    assert_eq!((c.runs.len(), c.links.len()), (2, 2));
    assert_eq!(c.runs[1].extra, extra);
}

/// A grid of the root with the given cells in one row.
fn row_of(cells: &[CellId]) -> Option<Grid> {
    Some(Grid::new(cells.len() as u32, 1, cells.to_vec()))
}

/// Found in review of milestone 3a: a step that makes a cell the child of another cell must
/// give it back to its old parent when undone, whatever the steps around it do.
#[test]
fn rolling_back_an_adoption_gives_the_cell_back() {
    let (e, mut ids, cells) = editor(2, 1);
    let mut doc = e.into_document();
    let original = doc.clone();
    let wrapper = Cell::new(CellId(ids.next_id()), Some(doc.root()));
    let id = wrapper.id();
    let mut seen = ChangeSet::default();
    let undo_add = Step::AddCells(vec![wrapper])
        .apply(&mut doc, &mut seen)
        .unwrap();
    let undo_grid = Step::SetGrid {
        cell: id,
        grid: row_of(&cells[..1]),
    }
    .apply(&mut doc, &mut seen)
    .unwrap();
    // The command fails here: its steps are undone, newest first.
    for step in [undo_grid, undo_add] {
        step.apply(&mut doc, &mut seen).unwrap();
    }
    assert_eq!(doc, original);
    doc.validate().unwrap();
}

#[test]
fn a_cell_adopted_from_a_removed_cell_returns_to_it_on_undo() {
    let (mut e, mut ids, cells) = editor(2, 1);
    e.apply(
        Command::CreateGrid {
            cell: cells[1],
            xs: 1,
            ys: 1,
        },
        None,
        &mut ids,
    )
    .unwrap();
    let mut doc = e.into_document();
    let original = doc.clone();
    let root = doc.root();
    let inner = doc.child(cells[1], 0, 0).unwrap();
    let mut seen = ChangeSet::default();
    // Flatten-like: the inner cell moves up, the cell it was in goes, still listing it.
    let steps = [
        Step::SetGrid {
            cell: root,
            grid: row_of(&[cells[0], inner]),
        },
        Step::RemoveCells(vec![cells[1]]),
    ];
    let mut undo = Vec::new();
    for step in steps {
        undo.push(step.apply(&mut doc, &mut seen).unwrap());
    }
    doc.validate().unwrap();
    let changed = doc.clone();
    let mut redo = Vec::new();
    for step in undo.into_iter().rev() {
        redo.push(step.apply(&mut doc, &mut seen).unwrap());
    }
    assert_eq!(doc, original);
    doc.validate().unwrap();
    for step in redo.into_iter().rev() {
        step.apply(&mut doc, &mut seen).unwrap();
    }
    assert_eq!(doc, changed);
}

/// Every kind of step followed by its inverse gives the identical document, and the
/// inverse of the inverse the changed one again: also where the step takes cells that are
/// children of other cells.
#[test]
fn every_step_and_its_inverse_cancel_exactly() {
    use super::step::{Axis, Field};
    let (mut e, mut ids, cells) = editor(2, 2);
    for cell in [cells[0], cells[3]] {
        e.apply(Command::CreateGrid { cell, xs: 2, ys: 2 }, None, &mut ids)
            .unwrap();
    }
    let doc = e.into_document();
    let root = doc.root();
    let a = doc.cell(cells[0]).unwrap().grid().unwrap().cells().to_vec();
    let b = doc.cell(cells[3]).unwrap().grid().unwrap().cells().to_vec();
    let lone = Cell::new(CellId(ids.next_id()), Some(root));
    let steps = [
        Step::Splice {
            cell: cells[1],
            at: 0,
            del: 1,
            ins: "xy".into(),
        },
        Step::Set {
            cell: cells[1],
            field: Field::Note("n".into()),
        },
        Step::Tag {
            name: "t".into(),
            tag: Some(Tag::default()),
        },
        Step::AddCells(vec![lone]),
        // Cells of the grid of one cell become the grid of another, or rows and columns
        // of it, while their old parent still lists them.
        Step::SetGrid {
            cell: cells[1],
            grid: row_of(&a[..2]),
        },
        Step::SetGrid {
            cell: cells[3],
            grid: row_of(&a),
        },
        Step::SetGrid {
            cell: cells[3],
            grid: None,
        },
        Step::InsertLines {
            owner: cells[3],
            axis: Axis::Rows,
            at: 1,
            ids: a[..2].to_vec(),
            widths: vec![],
        },
        Step::InsertLines {
            owner: cells[3],
            axis: Axis::Columns,
            at: 2,
            ids: a[2..].to_vec(),
            widths: vec![33],
        },
        Step::RemoveLines {
            owner: cells[3],
            axis: Axis::Columns,
            at: 0,
            count: 1,
        },
        Step::MoveLines {
            owner: root,
            axis: Axis::Rows,
            from: 0,
            count: 1,
            to: 1,
        },
        Step::Swap { a: a[0], b: b[3] },
    ];
    for step in steps {
        let mut work = doc.clone();
        let mut seen = ChangeSet::default();
        let what = format!("{step:?}");
        let inverse = step.apply(&mut work, &mut seen).unwrap();
        let changed = work.clone();
        assert_ne!(changed, doc, "{what}");
        let again = inverse.apply(&mut work, &mut seen).unwrap();
        assert_eq!(work, doc, "undo of {what}");
        again.apply(&mut work, &mut seen).unwrap();
        assert_eq!(work, changed, "redo of {what}");
    }
    // Removing cells is the inverse of adding them.
    let mut work = doc.clone();
    let mut seen = ChangeSet::default();
    let back = Step::RemoveCells(b.clone()).apply(&mut work, &mut seen);
    back.unwrap().apply(&mut work, &mut seen).unwrap();
    assert_eq!(work, doc);
}

mod restructure;
