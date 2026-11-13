//! `Editor::cancel_group`, and the redo history that is kept while a group is open: after
//! a cancel nothing is left of the group, in the document or in what the editor counts.

use std::collections::HashMap;

use super::*;
use crate::model::CellKind;

/// Everything that a cancelled group must leave as it was.
#[derive(PartialEq, Debug)]
struct Kept {
    doc: Document,
    saved: String,
    undo: usize,
    redo: usize,
    bytes: usize,
    bound: u64,
    held: HashMap<CellId, u32>,
}

fn kept(e: &Editor) -> Kept {
    e.audit().unwrap();
    Kept {
        doc: e.document().clone(),
        saved: crate::format::to_string(e.document()).unwrap(),
        undo: e.undo_len(),
        redo: e.redo_len(),
        bytes: e.history_bytes(),
        bound: e.size_bound(),
        held: e.held.clone(),
    }
}

/// The documents that redoing everything leads through, found on a copy of the editor.
fn redone(e: &Editor) -> Vec<Document> {
    let mut copy = e.clone();
    let mut docs = Vec::new();
    while copy.redo().unwrap().is_some() {
        docs.push(copy.document().clone());
    }
    docs
}

/// Redoing everything leads through these documents, and undoing everything back to this
/// one.
fn history_leads(e: &mut Editor, forward: &[Document], back: &Document) {
    for doc in forward {
        e.redo().unwrap().unwrap();
        assert_eq!(e.document(), doc, "redo");
        e.audit().unwrap();
    }
    assert_eq!(e.redo().unwrap(), None);
    while e.undo().unwrap().is_some() {}
    assert_eq!(e.document(), back, "undo of everything");
    e.audit().unwrap();
}

fn grid(cell: CellId, xs: u32, ys: u32) -> Command {
    Command::CreateGrid { cell, xs, ys }
}

fn tag(name: &str) -> Command {
    Command::SetTag {
        name: name.into(),
        cell_color: RED,
        text_color: BLUE,
    }
}

fn enter(cell: CellId, text: &str) -> Command {
    Command::EnterText {
        cell,
        text: text.into(),
    }
}

/// A 2 by 2 document with three units to undo and two to redo, the latter holding cells.
fn with_history() -> (Fixture, Document) {
    let mut f = Fixture::new(2, 2);
    let original = f.doc().clone();
    let (root, c) = (f.root(), f.cells.clone());
    f.run(insert(c[0], 0, "x")).unwrap();
    f.run(grid(c[1], 2, 1)).unwrap();
    f.run(tag("t")).unwrap();
    f.run(Command::InsertRows {
        cell: root,
        at: 1,
        count: 1,
    })
    .unwrap();
    f.run(grid(c[2], 1, 2)).unwrap();
    f.e.undo().unwrap().unwrap();
    f.e.undo().unwrap().unwrap();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (3, 2));
    assert_eq!(f.e.held.len(), 4, "the row of two and the grid of two");
    (f, original)
}

/// Commands of many kinds, each checked: carried out, part of the group, the redo history
/// and the counts untouched.
fn many_kinds(f: &mut Fixture) {
    let (root, c) = (f.root(), f.cells.clone());
    let (undo, redo) = (f.e.undo_len(), f.e.redo_len());
    let commands = vec![
        insert(c[0], 0, "ab"),
        styled(c[0], 0, 1, "b"),
        linked(c[0], 1, 1, "u"),
        grid(c[3], 2, 2),
        Command::InsertRows {
            cell: root,
            at: 0,
            count: 2,
        },
        Command::InsertColumns {
            cell: c[3],
            at: 1,
            count: 1,
        },
        Command::MoveRows {
            cell: root,
            from: 0,
            count: 1,
            to: 3,
        },
        Command::RemoveGrid { cell: c[1] },
        Command::DeleteRows {
            cell: root,
            at: 3,
            count: 1,
        },
        tag("u"),
        Command::RemoveTag { name: "t".into() },
        props(
            &[c[2], c[3]],
            &[Prop::CellColor(RED), Prop::Note("n".into())],
        ),
        Command::SetColumnWidth {
            cell: root,
            column: 0,
            width: 77,
        },
        enter(c[2], "=1+2"),
        Command::SwapCells { a: c[0], b: c[2] },
        Command::ClearCells { cells: vec![c[3]] },
    ];
    for command in commands {
        let what = format!("{command:?}");
        assert!(!f.run(command).unwrap().is_empty(), "{what}");
        f.e.audit().unwrap();
        assert_eq!((f.e.undo_len(), f.e.redo_len()), (undo, redo), "{what}");
    }
}

#[test]
fn a_cancelled_group_of_many_kinds_of_command_leaves_no_trace() {
    let (mut f, original) = with_history();
    let before = kept(&f.e);
    let forward = redone(&f.e);
    f.e.begin_group();
    many_kinds(&mut f);
    assert!(f.e.in_group());
    let last = f.doc().clone();
    let changes = f.e.cancel_group().unwrap();
    assert!(!f.e.in_group());
    assert_eq!(kept(&f.e), before);
    // The report is that of the way back: from the document just before the cancel.
    let ids = |doc: &Document| doc.cells_in_order();
    let only = |a: &Document, b: &Document| {
        let mut ids: Vec<CellId> = ids(a)
            .into_iter()
            .filter(|id| b.cell(*id).is_none())
            .collect();
        ids.sort();
        ids
    };
    assert_eq!(changes.created, only(f.doc(), &last));
    assert_eq!(changes.removed, only(&last, f.doc()));
    assert!(!changes.created.is_empty() && !changes.removed.is_empty());
    assert_eq!(changes.tags, ["t", "u"]);
    for id in ids(f.doc()) {
        let (now, then) = (f.cell(id), last.cell(id));
        let same = then == Some(now);
        assert!(
            same || changes.changed.contains(&id)
                || changes.grids.contains(&id)
                || changes.created.contains(&id),
            "{id} differs and is not reported"
        );
    }
    // What could be redone before can be redone, to the same documents.
    history_leads(&mut f.e, &forward, &original);
}

#[test]
fn the_same_group_ended_is_one_unit_and_nothing_can_be_redone() {
    let (mut f, original) = with_history();
    let before = kept(&f.e);
    f.e.begin_group();
    many_kinds(&mut f);
    let last = f.doc().clone();
    assert!(!f.e.end_group().is_empty());
    f.e.audit().unwrap();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (before.undo + 1, 0));
    // What the redo history held is held no longer; what the group removed is.
    assert!(before.held.keys().all(|id| !f.e.held.contains_key(id)));
    assert!(!f.e.held.is_empty());
    f.e.undo().unwrap().unwrap();
    assert_eq!(*f.doc(), before.doc);
    f.e.audit().unwrap();
    history_leads(&mut f.e, &[last], &original);
}

#[test]
fn cancel_closes_every_group_that_is_open() {
    let (mut f, id) = Fixture::text("abc");
    let before = kept(&f.e);
    f.e.begin_group();
    f.run(insert(id, 0, "1")).unwrap();
    f.e.begin_group();
    f.run(insert(id, 0, "2")).unwrap();
    f.e.begin_group();
    f.run(insert(id, 0, "3")).unwrap();
    assert!(f.e.end_group().is_empty(), "an inner group");
    assert_eq!(f.e.cancel_group().unwrap().changed, [id]);
    assert_eq!(kept(&f.e), before);
    assert!(!f.e.in_group());
    // No group is left open: ending one does nothing, a command is a unit of its own,
    // and undo is not refused.
    assert!(f.e.end_group().is_empty());
    f.run(insert(id, 0, "4")).unwrap();
    assert_eq!(f.e.undo_len(), 1);
    f.e.undo().unwrap().unwrap();
    assert_eq!(*f.doc(), before.doc);
    assert!(f.e.cancel_group().unwrap().is_empty());
    // The next group reports what it changed, and nothing of the cancelled one.
    f.e.begin_group();
    f.run(tag("z")).unwrap();
    let only_the_tag = Changes {
        tags: vec!["z".into()],
        ..Changes::default()
    };
    assert_eq!(f.e.end_group(), only_the_tag);
}

#[test]
fn cancel_without_a_group_or_of_a_group_that_changed_nothing_does_nothing() {
    let (mut f, _) = with_history();
    let c = f.cells.clone();
    let before = kept(&f.e);
    assert!(f.e.cancel_group().unwrap().is_empty());
    assert_eq!(kept(&f.e), before);
    // An empty group.
    f.e.begin_group();
    assert!(f.e.cancel_group().unwrap().is_empty());
    assert_eq!(kept(&f.e), before);
    assert!(!f.e.in_group());
    // A group of commands that ask for nothing new, and of commands that are refused.
    f.e.begin_group();
    assert!(f.run(insert(c[0], 0, "")).unwrap().is_empty());
    f.bad(insert(c[0], 99, "x"));
    assert!(f.e.cancel_group().unwrap().is_empty());
    assert_eq!(kept(&f.e), before);
}

#[test]
fn a_refused_command_inside_takes_back_only_itself_and_cancel_the_rest() {
    let (mut f, original) = with_history();
    let c = f.cells.clone();
    let before = kept(&f.e);
    let forward = redone(&f.e);
    f.e.begin_group();
    // Refused before anything else was done in the group, and after.
    f.bad(insert(c[0], 99, "x"));
    f.run(grid(c[0], 2, 2)).unwrap();
    f.run(Command::RemoveGrid { cell: c[1] }).unwrap();
    let inside = kept(&f.e);
    f.bad(grid(c[0], 1, 1));
    f.bad(Command::DeleteRows {
        cell: c[0],
        at: 1,
        count: 5,
    });
    assert_eq!(
        kept(&f.e),
        inside,
        "a refused command leaves the group as it was"
    );
    assert!(f.e.in_group());
    f.run(insert(c[2], 0, "y")).unwrap();
    f.e.cancel_group().unwrap();
    assert_eq!(kept(&f.e), before);
    history_leads(&mut f.e, &forward, &original);
}

#[test]
fn what_could_be_redone_stays_while_a_group_is_open_and_goes_when_it_ends() {
    let (mut f, id) = Fixture::text("abc");
    f.run(insert(id, 3, "d")).unwrap();
    f.run(insert(id, 4, "e")).unwrap();
    f.e.undo().unwrap().unwrap();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (1, 1));
    // A command in a group leaves it alone, in an inner group too; so does the end of an
    // inner group.
    f.e.begin_group();
    f.run(insert(id, 0, "x")).unwrap();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (1, 1));
    f.e.begin_group();
    f.run(insert(id, 0, "y")).unwrap();
    assert!(f.e.end_group().is_empty());
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (1, 1));
    // Undo and redo are refused all the same, and change nothing.
    let inside = kept(&f.e);
    assert!(matches!(f.e.undo(), Err(Error::Refused(_))));
    assert!(matches!(f.e.redo(), Err(Error::Refused(_))));
    assert_eq!(kept(&f.e), inside);
    // The end of the group is where the new unit takes the place of what was undone.
    assert_eq!(f.e.end_group().changed, [id]);
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (2, 0));
    assert_eq!(f.e.redo().unwrap(), None);
    f.e.audit().unwrap();

    // A group in which nothing changed is no unit and takes nothing away.
    f.e.undo().unwrap().unwrap();
    let before = kept(&f.e);
    assert_eq!((before.undo, before.redo), (1, 1));
    f.e.begin_group();
    assert!(f.run(insert(id, 0, "")).unwrap().is_empty());
    f.bad(insert(id, 99, "x"));
    assert!(f.e.end_group().is_empty());
    assert_eq!(kept(&f.e), before);
    // A group that changed something and changed it back is a unit, as two commands
    // would be two.
    f.e.begin_group();
    f.run(insert(id, 0, "x")).unwrap();
    f.run(delete(id, 0, 1)).unwrap();
    assert_eq!(f.e.redo_len(), 1);
    f.e.end_group();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (2, 0));
    assert_eq!(*f.doc(), before.doc);
    f.e.audit().unwrap();
}

/// Gives the same IDs again when set back to its seed.
struct Again(SeededIds);

impl IdSource for Again {
    fn next_id(&mut self) -> Id {
        self.0.next_id()
    }
}

/// The trace that the design asked for. A command in a group now runs while the redo
/// history still holds cells. Their IDs stay taken for as long as redo can bring them
/// back: through the whole group, and after a cancel.
#[test]
fn an_id_that_redo_holds_is_not_given_out_in_a_group() {
    let mut f = Fixture::new(2, 1);
    let c = f.cells.clone();
    let mut again = Again(SeededIds::from_u64(42));
    f.e.apply(grid(c[0], 1, 1), NOW, &mut again).unwrap();
    let made = f.doc().child(c[0], 0, 0).unwrap();
    let with = f.doc().clone();
    f.e.undo().unwrap().unwrap();
    assert!(f.e.holds(made));
    let before = kept(&f.e);
    assert_eq!(before.held, HashMap::from([(made, 1)]));

    f.e.begin_group();
    f.run(insert(c[1], 0, "x")).unwrap();
    // (Before the redo history was kept, the command above had emptied it, and the ID
    // was given to the cell below.)
    again.0 = SeededIds::from_u64(42);
    let error = f.e.apply(grid(c[1], 1, 1), NOW, &mut again).unwrap_err();
    assert!(matches!(error, Error::Refused(_)), "{error:?}");
    assert!(f.e.holds(made));
    f.e.audit().unwrap();
    f.e.cancel_group().unwrap();
    assert_eq!(kept(&f.e), before);
    again.0 = SeededIds::from_u64(42);
    assert!(f.e.apply(grid(c[1], 1, 1), NOW, &mut again).is_err());
    f.e.redo().unwrap().unwrap();
    assert_eq!(*f.doc(), with);
    f.e.audit().unwrap();

    // The same group ended: redo is gone, nothing holds the cell, the ID is free.
    f.e.undo().unwrap().unwrap();
    f.e.begin_group();
    f.run(insert(c[1], 0, "x")).unwrap();
    f.e.end_group();
    assert!(!f.e.holds(made) && f.e.held.is_empty());
    again.0 = SeededIds::from_u64(42);
    f.e.apply(grid(c[1], 1, 1), NOW, &mut again).unwrap();
    assert_eq!(f.doc().child(c[1], 0, 0), Some(made));
    f.e.audit().unwrap();
}

/// A second document with IDs of its own: a cell with a grid of two cells to move out.
fn elsewhere() -> (Document, CellId) {
    let mut ids = SeededIds::from_u64(1234);
    let mut doc = Document::new(&mut ids);
    doc.add_grid(doc.root(), 1, 1, &mut ids).unwrap();
    let cell = doc.child(doc.root(), 0, 0).unwrap();
    doc.cell_mut(cell).unwrap().text = "moved".into();
    doc.add_grid(cell, 2, 1, &mut ids).unwrap();
    (doc, cell)
}

/// The other half of the trace: `AdoptCells` takes a cell whose ID the history holds
/// (decision A84, argued for the undo history: the unit that holds it is older and is
/// reached only after the adoption is undone). The redo history holds IDs too, and with a
/// group open a command can meet them. It is sound because redo is never reached with the
/// adoption in the document: a cancel takes the adoption out first, and the end of the
/// group forgets the redo history.
#[test]
fn a_cell_that_redo_holds_is_adopted_and_released_inside_a_group() {
    let (other, moved) = elsewhere();
    let clip = Clip::of(&other, moved).unwrap();
    let inside: Vec<CellId> = clip.cell_ids().collect();
    assert_eq!(inside.len(), 3);
    let mut f = Fixture::new(3, 1);
    let c = f.cells.clone();
    f.run(Command::ClearCells { cells: c.clone() }).unwrap();
    f.e.clear_history();
    let adopt = |to: CellId| Command::AdoptCells {
        to,
        clip: clip.clone(),
    };
    f.run(adopt(c[0])).unwrap();
    let arrived = f.doc().clone();
    f.e.undo().unwrap().unwrap();
    // The redo history holds the three cells, and they are not in the document.
    let before = kept(&f.e);
    assert_eq!((before.undo, before.redo), (0, 1));
    assert!(inside
        .iter()
        .all(|id| f.e.holds(*id) && before.held[id] == 1));

    f.e.begin_group();
    f.run(insert(c[2], 0, "x")).unwrap();
    // The same cells come in at another place: not refused, and the redo history still
    // holds them, once.
    f.run(adopt(c[1])).unwrap();
    f.e.audit().unwrap();
    assert!(inside.iter().all(|id| f.e.held[id] == 1 && !f.e.holds(*id)));
    assert_eq!(f.e.redo_len(), 1);
    // Twice in the document they cannot be.
    f.bad(adopt(c[0]));
    // They leave again inside the group: held twice now, by redo and by the group.
    f.run(Command::ReleaseCell { cell: moved }).unwrap();
    f.e.audit().unwrap();
    assert!(inside.iter().all(|id| f.e.held[id] == 2 && f.e.holds(*id)));
    // And come in a third time, where they were released from.
    let left = f.doc().child(f.root(), 1, 0).unwrap();
    f.run(adopt(left)).unwrap();
    f.e.audit().unwrap();
    f.e.cancel_group().unwrap();
    assert_eq!(kept(&f.e), before);
    f.e.redo().unwrap().unwrap();
    assert_eq!(*f.doc(), arrived);
    f.e.audit().unwrap();
    assert!(f.e.held.keys().all(|id| !inside.contains(id)));

    // The same with the group ended: the redo history is forgotten with its hold on the
    // cells, the group's unit takes them out and brings them back.
    f.e.undo().unwrap().unwrap();
    assert_eq!(kept(&f.e), before);
    f.e.begin_group();
    f.run(adopt(c[1])).unwrap();
    f.run(Command::ReleaseCell { cell: moved }).unwrap();
    let left = f.doc().child(f.root(), 1, 0).unwrap();
    f.run(adopt(left)).unwrap();
    f.e.end_group();
    f.e.audit().unwrap();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (1, 0));
    // Only the group's release holds them now.
    assert!(inside.iter().all(|id| f.e.held[id] == 1 && !f.e.holds(*id)));
    let grouped = f.doc().clone();
    f.e.undo().unwrap().unwrap();
    assert_eq!(*f.doc(), before.doc);
    f.e.audit().unwrap();
    assert!(inside.iter().all(|id| f.e.holds(*id)));
    f.e.redo().unwrap().unwrap();
    assert_eq!(*f.doc(), grouped);
    f.e.audit().unwrap();
}

#[test]
fn cells_made_and_removed_in_a_group_while_the_histories_hold_cells() {
    let (mut f, original) = with_history();
    let (root, c) = (f.root(), f.cells.clone());
    let before = kept(&f.e);
    let forward = redone(&f.e);
    for cancel in [true, false] {
        f.e.begin_group();
        // New cells, with IDs that nothing holds; some of them go again.
        f.run(grid(c[0], 3, 3)).unwrap();
        f.run(Command::DeleteColumns {
            cell: c[0],
            at: 0,
            count: 2,
        })
        .unwrap();
        // Cells that were there before the group go: the group holds them.
        f.run(Command::RemoveGrid { cell: c[1] }).unwrap();
        f.run(Command::DeleteRows {
            cell: root,
            at: 1,
            count: 1,
        })
        .unwrap();
        f.e.audit().unwrap();
        assert!(before.held.iter().all(|(id, n)| f.e.held[id] == *n));
        assert!(f.e.held.len() > before.held.len());
        if cancel {
            f.e.cancel_group().unwrap();
            assert_eq!(kept(&f.e), before);
        } else {
            f.e.end_group();
            assert_eq!((f.e.undo_len(), f.e.redo_len()), (before.undo + 1, 0));
            f.e.audit().unwrap();
            f.e.undo().unwrap().unwrap();
            assert_eq!(*f.doc(), before.doc);
        }
    }
    // After the cancel everything could be redone; after the end, only the group.
    assert_eq!(f.e.redo_len(), 1);
    let mut copy = f.e.clone();
    history_leads(&mut copy, &redone(&f.e), &original);
    assert_eq!(forward.len(), 2);
}

/// The 1 by 4 document "A" (with 1 and 2 inside), `=SUM(A)`, "B" (with 3 inside),
/// `=SUM(B)`: the cells inside A and B, and the two formula cells.
fn sums() -> (Fixture, [CellId; 2], [CellId; 2]) {
    let mut f = Fixture::new(1, 4);
    let c = f.cells.clone();
    for (cell, text) in [
        (c[0], "A"),
        (c[1], "=SUM(A)"),
        (c[2], "B"),
        (c[3], "=SUM(B)"),
    ] {
        f.run(enter(cell, text)).unwrap();
    }
    f.run(grid(c[0], 1, 2)).unwrap();
    f.run(grid(c[2], 1, 1)).unwrap();
    let a = f.doc().child(c[0], 0, 0).unwrap();
    let a2 = f.doc().child(c[0], 0, 1).unwrap();
    let b = f.doc().child(c[2], 0, 0).unwrap();
    for (cell, text) in [(a, "1"), (a2, "2"), (b, "3")] {
        f.run(enter(cell, text)).unwrap();
    }
    f.e.clear_history();
    assert_eq!((f.e.shown(c[1]), f.e.shown(c[3])), (Some("3"), Some("3")));
    assert_eq!(f.cell(c[1]).kind, CellKind::Formula);
    (f, [a, b], [c[1], c[3]])
}

/// What `recalculated` holds at each point of a group: pinned, because an interface
/// repaints from it.
#[test]
fn what_recalculated_says_inside_a_group_and_after_its_end_or_cancel() {
    let (mut f, [a, b], [sum_a, sum_b]) = sums();
    let sorted = |ids: &[CellId]| {
        let mut ids = ids.to_vec();
        ids.sort();
        ids
    };
    let both = sorted(&[sum_a, sum_b]);
    // Opening a group leaves it as the command before left it.
    let last = f.e.recalculated().to_vec();
    assert_eq!(last, [sum_b]);
    f.e.begin_group();
    assert_eq!(f.e.recalculated(), last);
    // After each command inside: what that command recalculated, not the group so far.
    f.run(enter(a, "10")).unwrap();
    assert_eq!(f.e.recalculated(), [sum_a]);
    f.run(enter(b, "30")).unwrap();
    assert_eq!(f.e.recalculated(), [sum_b]);
    // A refused command leaves it as the command before left it.
    f.bad(insert(b, 99, "x"));
    assert_eq!(f.e.recalculated(), [sum_b]);
    // The end of an inner group leaves it alone as well.
    f.e.begin_group();
    assert!(f.e.end_group().is_empty());
    assert_eq!(f.e.recalculated(), [sum_b]);
    // A command that changes nothing, or no result: nothing.
    assert!(f.run(insert(b, 0, "")).unwrap().is_empty());
    assert_eq!(f.e.recalculated(), []);
    f.run(tag("t")).unwrap();
    assert_eq!(f.e.recalculated(), []);
    // The end of the group: everything that recalculated in it, by ID.
    f.e.end_group();
    assert_eq!(f.e.recalculated(), both);
    assert_eq!(
        (f.e.shown(sum_a), f.e.shown(sum_b)),
        (Some("12"), Some("30"))
    );

    // A cancel: what taking the group back recalculated, as an undo of it would say.
    f.e.begin_group();
    f.run(enter(a, "100")).unwrap();
    f.run(enter(b, "300")).unwrap();
    f.run(tag("u")).unwrap();
    assert_eq!(f.e.recalculated(), []);
    f.e.cancel_group().unwrap();
    assert_eq!(sorted(f.e.recalculated()), both);
    assert_eq!(
        (f.e.shown(sum_a), f.e.shown(sum_b)),
        (Some("12"), Some("30"))
    );
    // Without a group it does nothing, and says nothing new.
    f.e.cancel_group().unwrap();
    assert_eq!(sorted(f.e.recalculated()), both);
    // A result that the group changed and changed back is as it was before the cancel:
    // not named (where the end of a group names it).
    f.e.begin_group();
    f.run(enter(a, "5")).unwrap();
    f.run(enter(a, "10")).unwrap();
    f.run(enter(b, "7")).unwrap();
    f.e.cancel_group().unwrap();
    assert_eq!(f.e.recalculated(), [sum_b]);
    f.e.begin_group();
    f.run(enter(a, "5")).unwrap();
    f.run(enter(a, "10")).unwrap();
    f.e.end_group();
    assert_eq!(f.e.recalculated(), [sum_a]);
    // A cancelled empty group takes nothing back and recalculates nothing.
    f.e.begin_group();
    f.e.cancel_group().unwrap();
    assert_eq!(f.e.recalculated(), []);
    // The results are those of working everything out.
    let anew = crate::eval::Calc::new(f.doc());
    assert_eq!(f.e.calc().results(), anew.results());
}

#[test]
fn a_formula_cell_made_in_a_cancelled_group_is_no_formula_cell_after() {
    let (mut f, [a, _], [sum_a, _]) = sums();
    let before = kept(&f.e);
    let results = |e: &Editor| {
        let all = e.calc().results();
        all.into_iter()
            .map(|(id, text)| (id, text.to_string()))
            .collect::<Vec<_>>()
    };
    let had = results(&f.e);
    f.e.begin_group();
    f.run(enter(a, "=SUM(B)")).unwrap();
    assert_eq!(f.e.shown(sum_a), Some("5"));
    f.run(Command::ClearCells { cells: vec![sum_a] }).unwrap();
    assert_eq!(f.e.calc().results().len(), 2);
    f.e.cancel_group().unwrap();
    assert_eq!(kept(&f.e), before);
    assert_eq!(results(&f.e), had);
    assert_eq!((f.e.shown(a), f.e.shown(sum_a)), (Some("1"), Some("3")));
}

#[test]
fn a_history_forgotten_while_a_group_is_open_stays_forgotten() {
    let build = || {
        let (mut f, _) = with_history();
        let c = f.cells.clone();
        let at_begin = f.doc().clone();
        f.e.begin_group();
        f.run(insert(c[0], 0, "y")).unwrap();
        f.run(Command::RemoveGrid { cell: c[1] }).unwrap();
        (f, at_begin)
    };
    // The whole history: the group's commands are kept, and can still be taken back.
    let (mut f, at_begin) = build();
    let removed = f.e.held.len() - 4;
    f.e.clear_history();
    f.e.audit().unwrap();
    assert_eq!(
        (f.e.undo_len(), f.e.redo_len(), f.e.history_bytes()),
        (0, 0, 0)
    );
    assert!(f.e.in_group());
    assert_eq!(f.e.held.len(), removed);
    f.e.cancel_group().unwrap();
    f.e.audit().unwrap();
    assert_eq!(*f.doc(), at_begin);
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (0, 0));
    assert!(f.e.held.is_empty());
    // Or ended: the group is then the only unit.
    let (mut f, at_begin) = build();
    f.e.clear_history();
    f.e.end_group();
    f.e.audit().unwrap();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (1, 0));
    f.e.undo().unwrap().unwrap();
    assert_eq!(*f.doc(), at_begin);
    assert_eq!(f.e.undo().unwrap(), None);
    // Only what could be redone.
    let (mut f, at_begin) = build();
    f.e.clear_redo();
    f.e.audit().unwrap();
    f.e.cancel_group().unwrap();
    f.e.audit().unwrap();
    assert_eq!(*f.doc(), at_begin);
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (3, 0));
    // A history that keeps no units, set inside the group: the ended group is forgotten
    // at once, with what it held.
    let (mut f, _) = build();
    f.e.set_history_limit(Some(0));
    assert!(f.e.in_group());
    f.e.end_group();
    f.e.audit().unwrap();
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (0, 0));
    assert!(f.e.held.is_empty());
}

#[test]
fn the_bound_on_the_saved_size_is_as_before_and_commands_fit_again() {
    let (mut f, id) = Fixture::text("abc");
    let room = 100;
    f.e.set_size_limit(f.e.size_bound() + room);
    let before = kept(&f.e);
    f.e.begin_group();
    f.run(insert(id, 0, &"x".repeat(90))).unwrap();
    assert_eq!(f.e.size_bound(), before.bound + 90);
    let error = f.bad(insert(id, 0, &"y".repeat(20)));
    assert!(matches!(error, Error::Limit(_)), "{error:?}");
    f.e.cancel_group().unwrap();
    assert_eq!(kept(&f.e), before);
    // The room is there again.
    f.run(insert(id, 0, &"y".repeat(90))).unwrap();
    f.bad(insert(id, 0, &"y".repeat(20)));
    f.e.audit().unwrap();
    assert_eq!(f.e.size_bound(), crate::edit::size_bound(f.doc()));
}

/// A document whose bound is over the limit although its file fits is written out to see
/// whether a command leaves it fitting (decision A43), and what was found is kept until the
/// document changes by another way than a command. A cancel is such a way: here the group
/// ends on a document that is under the limit, for which nothing is looked at; taken back,
/// the document is over the bound again, and the editor must look.
#[test]
fn after_a_cancel_the_editor_looks_again_whether_the_document_fits() {
    let mut f = Fixture::new(1, 3);
    let c = f.cells.clone();
    f.run(Command::SetText {
        cell: c[0],
        text: "x".repeat(3000),
    })
    .unwrap();
    f.run(grid(c[1], 1, 1)).unwrap();
    f.run(grid(c[2], 1, 2)).unwrap();
    f.e.clear_history();
    let empty = f.doc().child(c[1], 0, 0).unwrap();
    let written = |e: &Editor| crate::format::to_string(e.document()).unwrap().len() as u64;
    // Not a byte of room: the file fits exactly, the bound is far over.
    let limit = written(&f.e);
    f.e.set_size_limit(limit);
    assert!(f.e.size_bound() > limit);
    // A cell with two cells inside moves one level down: the bound stays, the file grows.
    let deeper = Command::MoveCell {
        from: c[2],
        to: empty,
    };
    let refused = |e: &mut Editor, ids: &mut SeededIds| {
        let before = e.document().clone();
        let error = e.apply(deeper.clone(), NOW, ids).unwrap_err();
        assert!(matches!(error, Error::Limit(_)), "{error:?}");
        assert_eq!(*e.document(), before);
    };
    refused(&mut f.e.clone(), &mut f.ids);
    f.e.begin_group();
    f.run(delete(c[0], 0, 3000)).unwrap();
    assert!(f.e.size_bound() <= limit);
    f.run(insert(c[0], 0, "y")).unwrap();
    f.e.cancel_group().unwrap();
    assert!(f.e.size_bound() > limit);
    refused(&mut f.e, &mut f.ids);
    assert!(written(&f.e) <= limit);
}

#[test]
fn typing_joins_the_unit_before_a_cancelled_group() {
    let (mut f, id) = Fixture::text("");
    f.e.apply_typing(insert(id, 0, "a"), NOW, &mut f.ids)
        .unwrap();
    f.e.begin_group();
    f.e.apply_typing(insert(id, 1, "b"), NOW, &mut f.ids)
        .unwrap();
    f.e.cancel_group().unwrap();
    f.e.apply_typing(insert(id, 1, "c"), NOW, &mut f.ids)
        .unwrap();
    assert_eq!((f.cell(id).text.as_str(), f.e.undo_len()), ("ac", 1));
    f.e.audit().unwrap();
}

/// What cannot happen while the history matches the document, made to happen: a step of
/// the group that cannot be applied. The document is left as it was before the call, and
/// nothing that the editor counts is wrong afterwards.
#[test]
fn a_cancel_that_fails_leaves_the_document_and_closes_the_group() {
    let (mut f, _) = with_history();
    let c = f.cells.clone();
    let units = f.e.undo_len();
    f.e.begin_group();
    f.run(insert(c[0], 0, "y")).unwrap();
    f.run(Command::RemoveGrid { cell: c[1] }).unwrap();
    let inside = f.doc().clone();
    // The oldest of the group, so the last to be taken back: after all the others.
    let nowhere = CellId(Id::from_u128(0xdead));
    f.e.group.insert(0, vec![Step::RemoveCells(vec![nowhere])]);
    assert!(f.e.cancel_group().is_err());
    assert_eq!(*f.doc(), inside);
    f.doc().validate().unwrap();
    assert!(!f.e.in_group());
    assert_eq!((f.e.undo_len(), f.e.redo_len()), (units, 0));
    f.e.audit().unwrap();
    assert_eq!(f.e.held.len(), 0);
    // The editor goes on working.
    f.run(insert(c[0], 0, "z")).unwrap();
    f.e.undo().unwrap().unwrap();
    assert_eq!(*f.doc(), inside);
}

/// `audit` itself: it notices each of the counts when it is wrong.
#[test]
fn the_audit_notices_a_wrong_count() {
    let (f, _) = with_history();
    f.e.audit().unwrap();
    let wrong = |change: &dyn Fn(&mut Editor)| {
        let mut e = f.e.clone();
        change(&mut e);
        e.audit().unwrap_err()
    };
    let some = *f.e.held.keys().next().unwrap();
    assert!(wrong(&|e| *e.held.get_mut(&some).unwrap() += 1).contains("hold"));
    assert!(wrong(&|e| _ = e.held.remove(&some)).contains("hold"));
    assert!(wrong(&|e| _ = e.held.insert(e.doc.root(), 1)).contains("hold"));
    assert!(wrong(&|e| e.undo_cost += 1).contains("undo history"));
    assert!(wrong(&|e| e.undo[0].cost += 1).contains("unit"));
    assert!(wrong(&|e| e.redo[0].cost -= 1).contains("unit"));
    assert!(wrong(&|e| e.size += 1).contains("size"));
    assert!(wrong(&|e| e.group.push(Vec::new())).contains("group"));
    // What an open group holds is counted too.
    let mut e = f.e.clone();
    e.begin_group();
    let remove = Command::RemoveGrid { cell: f.cells[1] };
    e.apply(remove, NOW, &mut SeededIds::from_u64(5)).unwrap();
    e.audit().unwrap();
    e.group.clear();
    assert!(e.audit().unwrap_err().contains("hold"));
}
