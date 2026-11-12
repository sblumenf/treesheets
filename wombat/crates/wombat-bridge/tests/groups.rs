//! Groups of commands and entering an editor's text through an open document: a group
//! is one step of the undo history or, cancelled, none at all; what each call reports;
//! what a group means for the saved flag, for saving, closing and dropping.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod common;

use std::path::{Path, PathBuf};

use common::{example, ids, set_text, text_in_file};
use wombat_bridge::api::document::OpenDocument;
use wombat_bridge::api::wire::{self as w, Command, ErrorKind, Report};

fn open_example(name: &str) -> (OpenDocument, PathBuf) {
    common::open_example("groups", name)
}

/// The second cell of the root grid, "what to buy", which the helpers of `common` write
/// to and read from the file.
fn note(doc: &OpenDocument) -> String {
    doc.grid(doc.root().unwrap()).unwrap().cells[1].id.clone()
}

fn text_of(doc: &OpenDocument, cell: &str) -> String {
    doc.cell(cell.into()).unwrap().text
}

/// The first cell of the grid of "Costs", which holds 3, and the formula cell that adds
/// the costs up.
fn cost_and_total(doc: &OpenDocument) -> (String, String) {
    let root = doc.grid(doc.root().unwrap()).unwrap();
    let costs = doc.grid(root.cells[2].id.clone()).unwrap();
    (costs.cells[0].id.clone(), root.cells[5].id.clone())
}

fn set(cell: &str, text: &str) -> Command {
    Command::SetText {
        cell: cell.into(),
        text: text.into(),
    }
}

/// What another program does to a file: a new file under the name, with other content.
fn change_by_someone_else(path: &Path) {
    let other = path.with_extension("other");
    let mut doc = OpenDocument::copy_with(&example(), &other, ids()).unwrap();
    doc.apply(set_text(&doc, "theirs")).unwrap();
    doc.save().unwrap();
    drop(doc);
    std::fs::rename(&other, path).unwrap();
}

/// Which file is under the name: every save puts a new one there.
#[cfg(unix)]
fn file_number(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).unwrap().ino()
}

#[test]
fn a_group_is_one_step_of_the_undo_history() {
    let (mut doc, _) = open_example("one-step");
    let note = note(&doc);
    let (cost, total) = cost_and_total(&doc);
    assert!(!doc.in_group().unwrap());
    doc.begin_group().unwrap();
    assert!(doc.in_group().unwrap());
    // Each command is carried out at once and reports for itself.
    let first = doc.apply(set(&note, "one")).unwrap();
    assert_eq!(first.changed, std::slice::from_ref(&note));
    assert_eq!(text_of(&doc, &note), "one");
    let second = doc.apply(set(&cost, "7")).unwrap();
    assert_eq!(
        (&second.changed, &second.recalculated),
        (&vec![cost.clone()], &vec![total.clone()])
    );
    // The end reports the group as a whole.
    let whole = doc.end_group().unwrap();
    let mut both = vec![note.clone(), cost.clone()];
    both.sort();
    let expected = Report {
        changed: both,
        recalculated: vec![total.clone()],
        ..Report::default()
    };
    assert_eq!(whole, expected);
    assert!(!doc.in_group().unwrap());
    assert!(!doc.is_saved().unwrap());

    let undone = doc.undo().unwrap().unwrap();
    assert_eq!(undone, expected);
    assert_eq!(
        (text_of(&doc, &note).as_str(), text_of(&doc, &cost).as_str()),
        ("what to buy", "3")
    );
    assert_eq!(doc.undo().unwrap(), None);
    assert_eq!(doc.redo().unwrap().unwrap(), expected);
    assert_eq!(
        (text_of(&doc, &note).as_str(), text_of(&doc, &cost).as_str()),
        ("one", "7")
    );
}

/// Only the outermost group counts, and only its end reports. What the core remembers
/// of the last command's recalculation must not be handed out again by an end that ends
/// nothing.
#[test]
fn only_the_end_of_the_outermost_group_reports() {
    let (mut doc, _) = open_example("outermost");
    let (cost, total) = cost_and_total(&doc);
    let alone = doc.apply(set(&cost, "4")).unwrap();
    assert_eq!(alone.recalculated, std::slice::from_ref(&total));
    // No group is open: nothing ends, nothing is cancelled.
    assert_eq!(doc.end_group().unwrap(), Report::default());
    assert_eq!(doc.cancel_group().unwrap(), Report::default());
    assert_eq!(text_of(&doc, &cost), "4");

    doc.begin_group().unwrap();
    doc.begin_group().unwrap();
    doc.apply(set(&cost, "5")).unwrap();
    assert_eq!(doc.end_group().unwrap(), Report::default());
    assert!(doc.in_group().unwrap());
    let whole = doc.end_group().unwrap();
    assert_eq!(
        (whole.changed, whole.recalculated),
        (vec![cost.clone()], vec![total])
    );
    assert!(!doc.in_group().unwrap());
    // One step for all of it.
    doc.undo().unwrap().unwrap();
    assert_eq!(text_of(&doc, &cost), "4");
}

#[test]
fn undo_and_redo_inside_a_group_are_refused() {
    let (mut doc, _) = open_example("refused");
    let note = note(&doc);
    doc.apply(set(&note, "before")).unwrap();
    doc.apply(set(&note, "undone")).unwrap();
    doc.undo().unwrap().unwrap();
    doc.begin_group().unwrap();
    doc.apply(set(&note, "inside")).unwrap();
    assert_eq!(doc.undo().unwrap_err().kind, ErrorKind::Refused);
    assert_eq!(doc.redo().unwrap_err().kind, ErrorKind::Refused);
    // Refused means nothing happened: the group is open and holds what it held.
    assert!(doc.in_group().unwrap());
    assert_eq!(text_of(&doc, &note), "inside");
}

/// The cross of an editor: everything since the group was opened is taken back, and
/// what could be undone and redone before can be still.
#[test]
fn a_cancelled_group_leaves_the_document_and_both_histories_as_they_were() {
    let (mut doc, _) = open_example("cancel");
    let note = note(&doc);
    let (cost, total) = cost_and_total(&doc);
    doc.apply(set(&note, "before")).unwrap();
    doc.apply(set(&note, "undone")).unwrap();
    doc.undo().unwrap().unwrap();

    doc.begin_group().unwrap();
    doc.retype(note.clone(), "before and more".into()).unwrap();
    doc.begin_group().unwrap();
    doc.apply(set(&cost, "9")).unwrap();
    // With the group inside it, ended or not.
    let report = doc.cancel_group().unwrap();
    let mut both = vec![note.clone(), cost.clone()];
    both.sort();
    let expected = Report {
        changed: both,
        // What taking the group back recalculated.
        recalculated: vec![total],
        ..Report::default()
    };
    assert_eq!(report, expected);
    assert!(!doc.in_group().unwrap());
    assert_eq!(
        (text_of(&doc, &note).as_str(), text_of(&doc, &cost).as_str()),
        ("before", "3")
    );

    // What was undone before the group can still be redone, and then undone twice.
    assert!(doc.redo().unwrap().is_some());
    assert_eq!(text_of(&doc, &note), "undone");
    assert_eq!(doc.redo().unwrap(), None);
    doc.undo().unwrap().unwrap();
    doc.undo().unwrap().unwrap();
    assert_eq!(text_of(&doc, &note), "what to buy");
    assert_eq!(doc.undo().unwrap(), None);
}

/// A group that is ended empties the redo history, as a command does; one in which
/// nothing changed does not.
#[test]
fn an_ended_group_empties_the_redo_history_if_it_changed_anything() {
    let (mut doc, _) = open_example("redo");
    let note = note(&doc);
    doc.apply(set(&note, "undone")).unwrap();
    doc.undo().unwrap().unwrap();
    doc.begin_group().unwrap();
    doc.retype(note.clone(), "what to buy".into()).unwrap();
    assert_eq!(doc.end_group().unwrap(), Report::default());
    assert!(doc.redo().unwrap().is_some());
    doc.undo().unwrap().unwrap();
    doc.begin_group().unwrap();
    doc.retype(note.clone(), "something else".into()).unwrap();
    assert_eq!(doc.end_group().unwrap().changed, [note]);
    assert_eq!(doc.redo().unwrap(), None);
}

/// Entering the text of an editor keeps the styles and links of the part that did not
/// change, which setting the text would drop.
#[test]
fn retyping_replaces_only_the_part_that_differs() {
    let (mut doc, _) = open_example("retype");
    let note = note(&doc);
    let bold = w::Style {
        bold: true,
        ..w::Style::default()
    };
    doc.apply(Command::SetRangeStyle {
        cell: note.clone(),
        start: 0,
        len: 4,
        set: bold,
        clear: w::Style::default(),
    })
    .unwrap();
    doc.set_fixed_time(Some(1_700_000_000_000));
    let report = doc
        .retype(note.clone(), "what to buy tomorrow".into())
        .unwrap();
    let expected = Report {
        changed: vec![note.clone()],
        ..Report::default()
    };
    assert_eq!(report, expected);
    let look = doc.cell(note.clone()).unwrap();
    assert_eq!(look.text, "what to buy tomorrow");
    let run = w::RunInfo {
        start: 0,
        len: 4,
        style: bold,
        color: None,
    };
    assert_eq!(look.runs, [run]);
    // One step of the undo history, like a command.
    doc.undo().unwrap().unwrap();
    assert_eq!(text_of(&doc, &note), "what to buy");
    assert_eq!(doc.cell(note.clone()).unwrap().runs, [run]);
}

#[test]
fn retyping_the_text_that_is_there_changes_nothing() {
    let (mut doc, path) = open_example("retype-same");
    let note = note(&doc);
    assert_eq!(
        doc.retype(note.clone(), "what to buy".into()).unwrap(),
        Report::default()
    );
    assert!(doc.is_saved().unwrap());
    assert_eq!(doc.undo().unwrap(), None);
    // What is refused changes nothing either.
    let nobody = "00000000-0000-0000-0000-000000000001".to_string();
    assert_eq!(
        doc.retype(nobody, "x".into()).unwrap_err().kind,
        ErrorKind::NotFound
    );
    assert_eq!(
        doc.retype("no ID".into(), "x".into()).unwrap_err().kind,
        ErrorKind::BadRequest
    );
    assert!(doc.is_saved().unwrap());
    assert_eq!(doc.undo().unwrap(), None);
    doc.retype(note, "ours".into()).unwrap();
    assert!(!doc.is_saved().unwrap());
    doc.close().unwrap();
    assert_eq!(text_in_file(&path), "ours");
}

/// A cancelled group is no change to the document: where the file held the document
/// when the group was opened, and was not written since, it holds it again, and the
/// document is as saved as it was.
#[cfg(unix)]
#[test]
fn a_cancelled_group_leaves_a_saved_document_saved() {
    let (mut doc, path) = open_example("cancel-saved");
    let note = note(&doc);
    let written = file_number(&path);
    doc.begin_group().unwrap();
    doc.retype(note.clone(), "typed".into()).unwrap();
    assert!(!doc.is_saved().unwrap());
    doc.cancel_group().unwrap();
    assert!(doc.is_saved().unwrap());
    // So closing writes nothing, and does not look at the file.
    change_by_someone_else(&path);
    doc.close().unwrap();
    assert_eq!(text_in_file(&path), "theirs");
    assert_ne!(file_number(&path), written);

    // An empty group that is cancelled or ended is nothing at all.
    let (mut doc, _) = open_example("cancel-empty");
    doc.begin_group().unwrap();
    doc.cancel_group().unwrap();
    doc.begin_group().unwrap();
    doc.end_group().unwrap();
    assert!(doc.is_saved().unwrap());
}

#[test]
fn a_cancelled_group_leaves_an_unsaved_document_unsaved() {
    let (mut doc, path) = open_example("cancel-unsaved");
    let note = note(&doc);
    doc.apply(set(&note, "ours")).unwrap();
    doc.begin_group().unwrap();
    doc.retype(note.clone(), "ours, typed".into()).unwrap();
    doc.cancel_group().unwrap();
    assert!(!doc.is_saved().unwrap());
    // An inner group is no new start: one that opens after the outer group was saved
    // half-way does not make the file the document that a cancel leads back to.
    doc.save().unwrap();
    doc.begin_group().unwrap();
    doc.retype(note.clone(), "outer".into()).unwrap();
    doc.save().unwrap();
    doc.begin_group().unwrap();
    doc.retype(note.clone(), "inner".into()).unwrap();
    doc.cancel_group().unwrap();
    assert_eq!(text_of(&doc, &note), "ours");
    assert!(!doc.is_saved().unwrap());
    assert_eq!(text_in_file(&path), "outer");
}

/// Saving with a group open writes what is there: that is how half-typed text gets into
/// the file when the app is put away. The group stays open, and when it is cancelled
/// after that the file holds what the document no longer does.
#[test]
fn saving_inside_a_group_writes_what_is_there_and_leaves_the_group_open() {
    let (mut doc, path) = open_example("save-inside");
    let note = note(&doc);
    doc.begin_group().unwrap();
    doc.retype(note.clone(), "half typed".into()).unwrap();
    doc.save().unwrap();
    assert!(doc.is_saved().unwrap());
    assert_eq!(text_in_file(&path), "half typed");
    assert!(doc.in_group().unwrap());

    doc.cancel_group().unwrap();
    assert_eq!(text_of(&doc, &note), "what to buy");
    assert!(!doc.is_saved().unwrap());
    doc.save().unwrap();
    assert_eq!(text_in_file(&path), "what to buy");

    // The same for writing over what is there.
    doc.begin_group().unwrap();
    doc.retype(note.clone(), "again".into()).unwrap();
    doc.overwrite().unwrap();
    assert_eq!(text_in_file(&path), "again");
    assert!(doc.in_group().unwrap());
    doc.cancel_group().unwrap();
    assert!(!doc.is_saved().unwrap());
}

/// A write inside a group ends what the group may say of the file, also a write of the
/// document as it was when the group opened (the file was gone): the flag is kept on the
/// safe side, not worked out.
#[test]
fn after_any_write_inside_a_group_a_cancel_leaves_the_document_unsaved() {
    let (mut doc, path) = open_example("write-inside");
    let note = note(&doc);
    doc.begin_group().unwrap();
    std::fs::remove_file(&path).unwrap();
    doc.save().unwrap();
    doc.retype(note, "typed".into()).unwrap();
    doc.cancel_group().unwrap();
    assert!(!doc.is_saved().unwrap());
    doc.close().unwrap();
    assert_eq!(text_in_file(&path), "what to buy");
}

/// Closing with a group open saves what is there and gives the document up, group and
/// all: the text that was held is in the file, as after a drop.
#[test]
fn closing_inside_a_group_saves_what_is_there() {
    let (mut doc, path) = open_example("close-inside");
    doc.begin_group().unwrap();
    doc.retype(note(&doc), "half typed".into()).unwrap();
    doc.close().unwrap();
    assert_eq!(doc.in_group().unwrap_err().kind, ErrorKind::Closed);
    assert_eq!(doc.begin_group().unwrap_err().kind, ErrorKind::Closed);
    assert_eq!(doc.end_group().unwrap_err().kind, ErrorKind::Closed);
    assert_eq!(doc.cancel_group().unwrap_err().kind, ErrorKind::Closed);
    assert_eq!(
        doc.retype("x".into(), "x".into()).unwrap_err().kind,
        ErrorKind::Closed
    );
    assert_eq!(text_in_file(&path), "half typed");
    // The file is free, and a document opened from it has no group.
    let doc = OpenDocument::open_with(&path, ids()).unwrap();
    assert!(!doc.in_group().unwrap());

    let (mut doc, path) = open_example("drop-inside");
    doc.begin_group().unwrap();
    doc.retype(note(&doc), "half typed".into()).unwrap();
    drop(doc);
    assert_eq!(text_in_file(&path), "half typed");
}

/// A close that fails leaves the document open exactly as it was, and that includes its
/// group: the editing can still be cancelled or ended.
#[test]
fn a_close_that_fails_leaves_the_group_open() {
    let (mut doc, path) = open_example("close-fails");
    let note = note(&doc);
    doc.begin_group().unwrap();
    doc.retype(note.clone(), "half typed".into()).unwrap();
    change_by_someone_else(&path);
    assert_eq!(doc.close().unwrap_err().kind, ErrorKind::ChangedOnDisk);
    assert!(doc.in_group().unwrap());
    assert_eq!(text_of(&doc, &note), "half typed");
    doc.cancel_group().unwrap();
    assert_eq!(text_of(&doc, &note), "what to buy");
    assert_eq!(text_in_file(&path), "theirs");
}
