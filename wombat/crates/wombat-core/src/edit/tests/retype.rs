//! `Editor::retype`: the text of an editor goes into its cell as one replacement of the
//! part that differs, so that the styles and links of the rest stay.

use proptest::prelude::*;

use super::*;
use crate::model::CellKind;

/// "the quick brown fox" with "quick" bold and "fox" a link.
fn fox() -> (Fixture, CellId) {
    let (mut f, id) = Fixture::text("the quick brown fox");
    f.ok(styled(id, 4, 5, "b"));
    f.ok(linked(id, 16, 3, "u"));
    f.e.clear_history();
    (f, id)
}

fn retype(f: &mut Fixture, id: CellId, text: &str) -> Changes {
    let before = f.doc().clone();
    let changes = f.e.retype(id, text, NOW, &mut f.ids).unwrap();
    f.doc().validate().unwrap();
    assert_eq!(changes.is_empty(), *f.doc() == before, "{text:?}");
    changes
}

#[test]
fn one_letter_fixed_leaves_the_bold_word_and_the_link() {
    let (mut f, id) = fox();
    let changes = retype(&mut f, id, "The quick brown fox");
    assert_eq!(changes.changed, [id]);
    assert_eq!(f.cell(id).text, "The quick brown fox");
    assert_eq!(f.runs(id), [(4, 5, "b".into(), None)]);
    assert_eq!(f.links(id), [(16, 3, "u".into())]);
    assert_eq!(f.cell(id).last_edit, NOW);
    // Between the two, and longer: the link moves along.
    retype(&mut f, id, "The quick brooown fox");
    assert_eq!(f.runs(id), [(4, 5, "b".into(), None)]);
    assert_eq!(f.links(id), [(18, 3, "u".into())]);
    // Each is one command: one unit of the history, undone as any other.
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (2, 0));
    f.e.undo().unwrap().unwrap();
    f.e.undo().unwrap().unwrap();
    assert_eq!(f.cell(id).text, "the quick brown fox");
    assert_eq!(f.links(id), [(16, 3, "u".into())]);
}

/// The defect that `retype` is there for: the same correction through the commands that
/// replace the whole text takes the bold and the link away (as in TreeSheets, whose own
/// typing inserts and deletes and so keeps them).
#[test]
fn the_same_fix_through_set_text_or_enter_text_loses_both() {
    for enter in [false, true] {
        let (mut f, id) = fox();
        let text = "The quick brown fox".to_string();
        f.ok(if enter {
            Command::EnterText { cell: id, text }
        } else {
            Command::SetText { cell: id, text }
        });
        assert_eq!(f.cell(id).text, "The quick brown fox");
        assert_eq!(f.runs(id), []);
        assert_eq!(f.links(id), []);
    }
    // After `retype` the text is there already, and `EnterText` finds nothing to do.
    let (mut f, id) = fox();
    retype(&mut f, id, "The quick brown fox");
    let text = "The quick brown fox".to_string();
    assert!(f.ok(Command::EnterText { cell: id, text }).is_empty());
    assert_eq!(f.runs(id), [(4, 5, "b".into(), None)]);
    assert_eq!(f.links(id), [(16, 3, "u".into())]);
}

#[test]
fn the_text_it_has_already_is_nothing() {
    let (mut f, id) = fox();
    f.ok(insert(id, 0, "x"));
    f.e.undo().unwrap().unwrap();
    let before = (f.e.undo_len(), f.e.redo_len(), f.e.history_bytes());
    assert_eq!(before, (0, 1, 0));
    assert!(retype(&mut f, id, "the quick brown fox").is_empty());
    assert_eq!(
        (f.e.undo_len(), f.e.redo_len(), f.e.history_bytes()),
        before,
        "no entry in the history, and what could be redone still can"
    );
    // A text that is the cell's once it is cleaned, as every text command cleans it.
    assert!(retype(&mut f, id, "the\u{1}quick\u{1f}brown fox").is_empty());
    assert_eq!(f.e.redo_len(), 1);
    // Something new empties the redo history, as a command does.
    retype(&mut f, id, "the quick brown fox!");
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (1, 0));
}

#[test]
fn control_characters_become_what_the_text_commands_make_of_them() {
    let (mut f, id) = fox();
    retype(&mut f, id, "the quick\u{2}red fox");
    assert_eq!(f.cell(id).text, crate::text::clean("the quick\u{2}red fox"));
    assert!(crate::text::is_clean(&f.cell(id).text));
    assert_eq!(f.runs(id), [(4, 5, "b".into(), None)]);
    assert_eq!(f.links(id), [(14, 3, "u".into())]);
}

#[test]
fn a_formula_stays_a_formula_and_a_text_stays_a_text() {
    let (mut f, id) = Fixture::text("");
    let text = "=1+2".to_string();
    f.ok(Command::EnterText { cell: id, text });
    assert_eq!(
        (f.cell(id).kind, f.e.shown(id)),
        (CellKind::Formula, Some("3"))
    );
    // The formula is edited where it differs, and worked out again.
    retype(&mut f, id, "=1+40");
    assert_eq!(f.cell(id).text, "=1+40");
    assert_eq!(
        (f.cell(id).kind, f.e.shown(id)),
        (CellKind::Formula, Some("41"))
    );
    assert_eq!(f.e.recalculated(), [id]);
    // Like the text commands, it never changes the kind (A65): that is `EnterText`'s.
    retype(&mut f, id, "plain");
    assert_eq!(f.cell(id).kind, CellKind::Formula);
    let (mut f, id) = fox();
    retype(&mut f, id, "=1+2");
    assert_eq!(
        (f.cell(id).kind, f.e.shown(id)),
        (CellKind::Data, Some("=1+2"))
    );
    let text = "=1+2".to_string();
    f.ok(Command::EnterText { cell: id, text });
    assert_eq!(
        (f.cell(id).kind, f.e.shown(id)),
        (CellKind::Formula, Some("3"))
    );
}

#[test]
fn an_edit_inside_a_styled_run_or_a_link_stays_inside() {
    let (mut f, id) = fox();
    // A letter more in the bold word, then one replaced, then two gone.
    retype(&mut f, id, "the quiick brown fox");
    assert_eq!(f.runs(id), [(4, 6, "b".into(), None)]);
    retype(&mut f, id, "the quIick brown fox");
    assert_eq!(f.runs(id), [(4, 6, "b".into(), None)]);
    retype(&mut f, id, "the quck brown fox");
    assert_eq!(f.runs(id), [(4, 4, "b".into(), None)]);
    assert_eq!(f.links(id), [(15, 3, "u".into())]);
    // The same in the link.
    retype(&mut f, id, "the quck brown foox");
    assert_eq!(f.links(id), [(15, 4, "u".into())]);
    // Typing after the bold word is bold, as when it is typed in TreeSheets; typing at
    // the end of the link does not make the link longer.
    retype(&mut f, id, "the quck! brown foox");
    assert_eq!(f.runs(id), [(4, 5, "b".into(), None)]);
    retype(&mut f, id, "the quck! brown foox!");
    assert_eq!(f.links(id), [(16, 4, "u".into())]);
    // The whole bold word gone: so is the run.
    retype(&mut f, id, "the  brown foox!");
    assert_eq!(f.runs(id), []);
    assert_eq!(f.links(id), [(11, 4, "u".into())]);
}

/// What `retype` cannot know: it sees two texts, not the keys. Deleting the bold word
/// and typing somewhere else in one editing is one difference that spans both, and the
/// new text takes the style of the first replaced character (`ReplaceText`'s rule): here
/// what was plain becomes bold. A command per key would get this right.
#[test]
fn one_difference_spans_two_changes_far_apart() {
    let (mut f, id) = Fixture::text("big bold word here");
    f.ok(styled(id, 4, 4, "b"));
    retype(&mut f, id, "big word here!");
    assert_eq!(f.cell(id).text, "big word here!");
    assert_eq!(f.runs(id), [(4, 10, "b".into(), None)]);
    // With the changes the other way round the far one is the first: nothing is bold.
    let (mut f, id) = Fixture::text("big bold word here");
    f.ok(styled(id, 4, 4, "b"));
    retype(&mut f, id, "Big word here");
    assert_eq!(f.runs(id), []);
}

#[test]
fn a_user_perceived_character_is_replaced_whole() {
    // An accent typed onto a letter would be cut from it by the bare difference, which
    // `ReplaceText` allows (it looks only at what it takes out), but here the bold e is
    // replaced with it.
    let (mut f, id) = Fixture::text("ae x");
    f.ok(styled(id, 1, 1, "b"));
    retype(&mut f, id, "ae\u{301} x");
    assert_eq!(f.cell(id).text, "ae\u{301} x");
    assert_eq!(f.runs(id), [(1, 2, "b".into(), None)]);
    // Taking the accent away again cuts the character in the cell: refused as a
    // `DeleteText`, done here.
    f.bad(delete(id, 2, 1));
    retype(&mut f, id, "ae x");
    assert_eq!(f.runs(id), [(1, 1, "b".into(), None)]);
    // Half a flag.
    let (mut f, id) = Fixture::text("\u{1F1E9}\u{1F1EA}!");
    retype(&mut f, id, "\u{1F1E9}!");
    assert_eq!(f.cell(id).text, "\u{1F1E9}!");
}

#[test]
fn what_is_refused_leaves_everything_as_it_was() {
    let (mut f, id) = fox();
    f.ok(insert(id, 0, "x"));
    f.e.undo().unwrap().unwrap();
    let before = f.doc().clone();
    let nowhere = CellId(Id::from_u128(0xdead));
    let error = f.e.retype(nowhere, "x", NOW, &mut f.ids).unwrap_err();
    assert!(matches!(error, Error::NotFound(_)), "{error:?}");
    // Also when the text is the one a cell would have: there is no such cell.
    let error = f.e.retype(nowhere, "", NOW, &mut f.ids).unwrap_err();
    assert!(matches!(error, Error::NotFound(_)), "{error:?}");
    f.e.set_size_limit(f.e.size_bound() + 10);
    let long = format!("the quick brown fox{}", "x".repeat(100));
    let error = f.e.retype(id, &long, NOW, &mut f.ids).unwrap_err();
    assert!(matches!(error, Error::Limit(_)), "{error:?}");
    assert_eq!(*f.doc(), before);
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (0, 1));
}

#[test]
fn in_a_group_it_is_part_of_the_group() {
    let (mut f, id) = fox();
    let before = f.doc().clone();
    f.e.begin_group();
    retype(&mut f, id, "the quick brown fo");
    retype(&mut f, id, "the quick brown f");
    f.run(styled(id, 0, 3, "i")).unwrap();
    retype(&mut f, id, "the quick brown fix");
    assert_eq!(f.e.undo_len(), 0);
    assert_eq!(f.e.end_group().changed, [id]);
    assert_eq!(f.e.undo_len(), 1);
    // (Typing at the end of a link does not make it longer.)
    assert_eq!(f.links(id), [(16, 1, "u".into())]);
    f.e.undo().unwrap().unwrap();
    assert_eq!(*f.doc(), before);
}

/// The texts of the property: user-perceived characters of several code points, some
/// repeated, so that differences fall inside styled parts and beside them.
const PIECES: &[&str] = &[
    "a",
    "a",
    "b",
    " ",
    "e\u{301}",
    "\u{1F1E9}\u{1F1EA}",
    "\u{1F468}\u{200D}\u{1F469}",
    "\u{1}",
];

fn text() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::sample::select(PIECES), 0..10)
        .prop_map(|pieces| pieces.concat())
}

proptest! {
    /// Whatever the two texts and whatever is styled and linked: the cell gets the text
    /// (cleaned), the document is valid, it is at most one unit, undo gives back the
    /// cell as it was, and what is before and after the part that differs keeps its
    /// styles and links exactly.
    #[test]
    fn any_text_becomes_any_other(
        old in text(),
        new in text(),
        marks in proptest::collection::vec(any::<(u32, u32, bool)>(), 0..4),
    ) {
        let (mut f, id) = Fixture::text(&old);
        let old = f.cell(id).text.clone();
        let len = old.chars().count() as u32;
        for (start, count, link) in marks {
            let start = crate::text::floor_boundary(&old, start % (len + 1));
            let end = crate::text::ceil_boundary(&old, start + count % 4);
            let command = if link {
                linked(id, start, end - start, "u")
            } else {
                styled(id, start, end - start, "b")
            };
            f.run(command).unwrap();
        }
        f.e.clear_history();
        let before = f.cell(id).clone();
        let changes = retype(&mut f, id, &new);
        let wanted = crate::text::clean(&new);
        prop_assert_eq!(&f.cell(id).text, &wanted);
        prop_assert_eq!(f.e.undo_len(), usize::from(!changes.is_empty()));
        // What each code point outside the difference looks like and links to.
        let look = |c: &Cell, at: u32| {
            let inside = |start: u32, len: u32| start <= at && at < start + len;
            (
                c.runs.iter().find(|r| inside(r.start, r.len)).map(|r| r.style),
                c.links.iter().find(|l| inside(l.start, l.len)).map(|l| l.target.clone()),
            )
        };
        if let Some((start, replaced, put)) = crate::text::difference(&old, &wanted) {
            let after = f.cell(id);
            let shift = put.chars().count() as u32;
            for at in 0..start {
                prop_assert_eq!(look(&before, at), look(after, at), "before, at {}", at);
            }
            for at in start + replaced..len {
                let now = at - replaced + shift;
                prop_assert_eq!(look(&before, at), look(after, now), "after, at {}", at);
            }
            f.e.undo().unwrap().unwrap();
        }
        prop_assert_eq!(f.cell(id), &before);
    }
}
