//! Shared by the random tests of the link index (here and in `wombat-cts`, on imported
//! TreeSheets documents): random edits of several documents at once, with links between
//! them, and after every one of them the rule of milestone 4: the index that followed the
//! change reports equals an index built from the documents.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use wombat_core::eval::Calc;
use wombat_core::links::{move_between, Resolved};
use wombat_core::{
    CellId, Changes, Clip, Command, DocId, Document, Editor, Id, LinkIndex, LinkTarget, SeededIds,
};

use crate::support::{self, op, Op};

/// One random step. The numbers pick documents and cells.
#[derive(Clone, Debug)]
pub enum Act {
    /// Any command of the engine, in one document.
    Apply(usize, Op),
    /// A link from a cell to a cell of another document, or to one that does not exist.
    Link(usize, [u32; 4]),
    /// Several commands as one unit.
    Group(usize, Vec<Op>),
    Undo(usize),
    Redo(usize),
    /// Copies a cell of one document over a cell of another (or of the same).
    Paste(usize, usize, [u32; 2]),
    /// Moves a cell of one document to an empty cell of another.
    Move(usize, usize, [u32; 2]),
    /// Corrects the links of a document to cells that have moved.
    Repair(usize),
    /// Types a formula or a number into a cell, or runs the program (milestone 5): links
    /// and formulas have to hold together.
    Formula(usize, [u32; 3]),
    /// Makes links of a document that name one document name another, in place (the
    /// command `RetargetLinks`, with which moves and repairs correct links; here also
    /// where nothing moved, so that the links end up wrong).
    Retarget(usize, [u32; 3]),
}

pub fn act() -> impl Strategy<Value = Act> {
    let doc = || 0..6usize;
    prop_oneof![
        12 => (doc(), op()).prop_map(|(d, op)| Act::Apply(d, op)),
        10 => (doc(), any::<[u32; 4]>()).prop_map(|(d, n)| Act::Link(d, n)),
        2 => (doc(), proptest::collection::vec(op(), 0..4)).prop_map(|(d, ops)| Act::Group(d, ops)),
        5 => doc().prop_map(Act::Undo),
        3 => doc().prop_map(Act::Redo),
        3 => (doc(), doc(), any::<[u32; 2]>()).prop_map(|(a, b, n)| Act::Paste(a, b, n)),
        4 => (doc(), doc(), any::<[u32; 2]>()).prop_map(|(a, b, n)| Act::Move(a, b, n)),
        1 => doc().prop_map(Act::Repair),
        6 => (doc(), any::<[u32; 3]>()).prop_map(|(d, n)| Act::Formula(d, n)),
        2 => (doc(), any::<[u32; 3]>()).prop_map(|(d, n)| Act::Retarget(d, n)),
    ]
}

/// What a run did, so that a test can tell that it did enough.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// Commands carried out.
    pub applied: usize,
    /// Links whose target was still there after a command, and which resolved to it.
    pub survived: usize,
    /// Links whose target a command removed, and which were reported broken.
    pub broken: usize,
    pub pastes: usize,
    pub moves: usize,
    /// Links that led to the same cell after a copy or a move between documents as before.
    pub kept: usize,
    pub repairs: usize,
    /// `RetargetLinks` commands given directly that changed links.
    pub retargets: usize,
    /// Formulas and numbers entered, and runs of the program.
    pub formulas: usize,
    /// The most formula cells that the documents held at one time.
    pub formula_cells: usize,
    /// The most links the index held at one time.
    pub links: usize,
}

impl Stats {
    pub fn add(&mut self, other: Stats) {
        self.applied += other.applied;
        self.survived += other.survived;
        self.broken += other.broken;
        self.pastes += other.pastes;
        self.moves += other.moves;
        self.kept += other.kept;
        self.repairs += other.repairs;
        self.retargets += other.retargets;
        self.formulas += other.formulas;
        self.formula_cells = self.formula_cells.max(other.formula_cells);
        self.links = self.links.max(other.links);
    }
}

fn rebuilt(editors: &[Editor]) -> LinkIndex {
    let mut index = LinkIndex::new();
    for editor in editors {
        index.insert_document(editor.document());
    }
    index
}

/// The heart of it: the index that followed the changes is the index built from scratch.
fn check(editors: &[Editor], index: &LinkIndex, what: &str) {
    assert!(*index == rebuilt(editors), "the index drifted after {what}");
    // And the rule of milestone 5: the results of the formula cells, kept up to date
    // through every command, copy and move, are those of working everything out.
    for editor in editors {
        let calc = editor.calc();
        let full = Calc::with_variables(editor.document(), calc.variables().clone());
        assert!(
            calc.results() == full.results(),
            "the results drifted after {what}"
        );
    }
}

fn pick(doc: &Document, n: u32, keep: impl Fn(&Document, CellId) -> bool) -> Option<CellId> {
    let cells: Vec<CellId> = doc
        .cells_in_order()
        .into_iter()
        .filter(|c| keep(doc, *c))
        .collect();
    (!cells.is_empty()).then(|| cells[n as usize % cells.len()])
}

fn any_cell(_: &Document, _: CellId) -> bool {
    true
}

fn not_root(doc: &Document, cell: CellId) -> bool {
    cell != doc.root()
}

/// Every link that resolves to a cell where it says, as (source document, source cell,
/// target as stored, target document, target cell).
type Held = Vec<(DocId, CellId, LinkTarget, DocId, CellId)>;

fn held(index: &LinkIndex) -> Held {
    let ids: Vec<DocId> = index.documents().map(|d| d.id()).collect();
    let mut out = Vec::new();
    for id in ids {
        for link in index.outgoing(id) {
            if let Resolved::Cell {
                doc,
                cell,
                moved: false,
            } = link.resolved
            {
                out.push((id, link.cell, link.link.target, doc, cell));
            }
        }
    }
    out
}

/// After a command in the document `edited`: every link whose target cell is still in its
/// document resolves to that very cell and is listed among its backlinks; every link
/// whose target is gone does not resolve to it. Links of cells that the command itself
/// changed or removed are left out (the command may have changed the link).
fn check_survival(
    before: &Held,
    editors: &[Editor],
    index: &LinkIndex,
    edited: DocId,
    changes: &Changes,
    stats: &mut Stats,
    what: &str,
) {
    for (from, cell, target, to_doc, to_cell) in before {
        let touched = changes.changed.binary_search(cell).is_ok()
            || changes.removed.binary_search(cell).is_ok();
        if *from == edited && touched {
            continue;
        }
        let there = editors
            .iter()
            .any(|e| e.document().id() == *to_doc && e.document().cell(*to_cell).is_some());
        let direct = Resolved::Cell {
            doc: *to_doc,
            cell: *to_cell,
            moved: false,
        };
        let now = index.resolve(*from, target);
        if there {
            assert!(now == direct, "a link no longer resolves after {what}");
            let back = index.backlinks(*to_doc, *to_cell);
            assert!(
                back.iter().any(|b| b.doc == *from && b.cell == *cell),
                "a backlink is missing after {what}"
            );
            stats.survived += 1;
        } else {
            assert!(now != direct, "a link to a removed cell resolves: {what}");
            assert!(index.backlinks(*to_doc, *to_cell).is_empty());
            stats.broken += 1;
        }
    }
}

/// Something else that follows the same edits (the search index of milestone 6, tested in
/// `wombat-search`): it is told of every change report, and asked to check itself after
/// every step.
pub trait Hook {
    /// A command, undo, redo or group changed the document of `editor`, whose results
    /// are up to date with it.
    fn changed(&mut self, editor: &Editor, changes: &Changes);
    /// A step is over; `editors` hold the documents as they are now.
    fn step(&mut self, editors: &[Editor], what: &str);
}

struct NoHook;

impl Hook for NoHook {
    fn changed(&mut self, _: &Editor, _: &Changes) {}
    fn step(&mut self, _: &[Editor], _: &str) {}
}

fn follow(index: &mut LinkIndex, hook: &mut dyn Hook, editor: &Editor, changes: &Changes) {
    index.apply(editor.document(), changes);
    hook.changed(editor, changes);
}

/// What every link that leads to a cell leads to: by the document and cell that have the
/// link and its range, the document and cell it resolves to, and whether that cell was
/// found by its ID alone, in another document than the link says ("moved").
type Meaning = BTreeMap<(DocId, CellId, u32, u32), (DocId, CellId, bool)>;

fn meaning(index: &LinkIndex) -> Meaning {
    let ids: Vec<DocId> = index.documents().map(|d| d.id()).collect();
    let mut out = Meaning::new();
    for id in ids {
        for link in index.outgoing(id) {
            if let Resolved::Cell { doc, cell, moved } = link.resolved {
                out.insert(
                    (id, link.cell, link.link.start, link.link.len),
                    (doc, cell, moved),
                );
            }
        }
    }
    out
}

/// In how many of the documents a cell with this ID is. More than one after a move that
/// was undone on one side only: then where a link leads is another matter (decision A50).
fn homes(editors: &[Editor], cell: CellId) -> usize {
    editors
        .iter()
        .filter(|e| e.document().cell(cell).is_some())
        .count()
}

/// After the cells `moved` went from the document `from` to `to`: every link that led to
/// a cell before still leads to that cell, wherever link and cell are now; and a link of
/// the two documents that named the right document before names the right one now (found
/// in review of milestone 4: nothing checked that the links are corrected on a move).
#[allow(clippy::too_many_arguments)]
fn check_moved(
    before: &Meaning,
    homes_before: &dyn Fn(CellId) -> usize,
    editors: &[Editor],
    index: &LinkIndex,
    (from, to): (DocId, DocId),
    moved: &BTreeSet<CellId>,
    stats: &mut Stats,
    what: &str,
) {
    let after = meaning(index);
    for ((source, cell, start, len), (doc, target, by_id)) in before {
        if homes_before(*target) != 1 || homes(editors, *target) != 1 {
            continue;
        }
        let went = |doc: DocId, cell: &CellId| {
            if doc == from && moved.contains(cell) {
                to
            } else {
                doc
            }
        };
        let now = after.get(&(went(*source, cell), *cell, *start, *len));
        let Some((now_doc, now_cell, now_by_id)) = now else {
            panic!("a link no longer leads to a cell after {what}");
        };
        assert!(
            (*now_doc, *now_cell) == (went(*doc, target), *target),
            "a link leads to another cell after {what}"
        );
        if !by_id && (*source == from || *source == to) {
            assert!(!now_by_id, "a link was not corrected by {what}");
        }
        stats.kept += 1;
    }
}

/// After the cells of `clip` (from the document `from`) were pasted over `place` in the
/// document `to`: a link in a copy leads where the link of the original led: to the copy
/// of a cell that was copied with it, and else to the very same cell.
#[allow(clippy::too_many_arguments)]
fn check_pasted(
    before: &Meaning,
    homes_before: &dyn Fn(CellId) -> usize,
    editors: &[Editor],
    index: &LinkIndex,
    (from, to): (DocId, DocId),
    originals: &[CellId],
    copies: &[CellId],
    stats: &mut Stats,
    what: &str,
) {
    assert_eq!(originals.len(), copies.len(), "{what}");
    let copy_of: BTreeMap<CellId, CellId> = originals
        .iter()
        .copied()
        .zip(copies.iter().copied())
        .collect();
    let after = meaning(index);
    for ((source, cell, start, len), (doc, target, by_id)) in before {
        let Some(copy) = copy_of.get(cell).filter(|_| *source == from) else {
            continue;
        };
        // A link that found its cell by ID alone keeps saying what it said.
        if *by_id || homes_before(*target) != 1 {
            continue;
        }
        let want = match copy_of.get(target).filter(|_| *doc == from) {
            Some(copied) => (to, *copied),
            // The cell it led to may be among those that the paste replaced.
            None if homes(editors, *target) != 1 => continue,
            None => (*doc, *target),
        };
        let now = after.get(&(to, *copy, *start, *len));
        assert!(
            now == Some(&(want.0, want.1, false)),
            "the copy of a link leads elsewhere after {what}: {now:?}, not {want:?}"
        );
        stats.kept += 1;
    }
}

/// Runs the steps on the documents, with every check after each. At the end everything is
/// undone, and the index must have followed that too.
pub fn run(docs: Vec<Document>, acts: &[Act]) -> Stats {
    run_with(docs, acts, &mut NoHook)
}

/// [`run`], with something else following along.
pub fn run_with(docs: Vec<Document>, acts: &[Act], hook: &mut dyn Hook) -> Stats {
    let mut ids = SeededIds::from_u64(4242);
    let mut editors: Vec<Editor> = docs.into_iter().map(|d| Editor::new(d).unwrap()).collect();
    let count = editors.len();
    let mut index = rebuilt(&editors);
    let originals: Vec<Document> = editors.iter().map(|e| e.document().clone()).collect();
    let mut stats = Stats::default();
    for act in acts {
        let what = format!("{act:?}");
        match act {
            Act::Apply(d, op) => {
                let d = d % count;
                let before = held(&index);
                let old = index.clone();
                let doc = editors[d].document();
                let id = doc.id();
                let command = support::command(doc, op);
                let Ok(changes) = editors[d].apply(command, Some(1), &mut ids) else {
                    continue;
                };
                if changes.is_empty() {
                    continue;
                }
                follow(&mut index, hook, &editors[d], &changes);
                check(&editors, &index, &what);
                check_survival(&before, &editors, &index, id, &changes, &mut stats, &what);
                stats.applied += 1;
                // Undo mends what the command broke; redo breaks it again, the same way.
                let new = index.clone();
                let back = editors[d].undo().unwrap().unwrap();
                follow(&mut index, hook, &editors[d], &back);
                assert!(index == old, "undo of {what}");
                let again = editors[d].redo().unwrap().unwrap();
                follow(&mut index, hook, &editors[d], &again);
                assert!(index == new, "redo of {what}");
            }
            Act::Link(d, n) => {
                let d = d % count;
                let doc = editors[d].document();
                let has_text = |doc: &Document, c: CellId| !doc.cell(c).unwrap().text.is_empty();
                let Some(cell) = pick(doc, n[0], has_text) else {
                    continue;
                };
                let len = doc.cell(cell).unwrap().text.chars().count() as u32;
                let other = editors[(d + 1 + n[1] as usize % count.max(2)) % count].document();
                let there = pick(other, n[2], any_cell).unwrap();
                let nowhere = CellId(Id::from_u128(u128::from(n[2]) + 1));
                let target = match n[3] % 8 {
                    0 => LinkTarget::Cell {
                        doc: Some(other.id()),
                        cell: nowhere,
                    },
                    1 => LinkTarget::Cell {
                        doc: Some(DocId(Id::from_u128(5))),
                        cell: there,
                    },
                    // A cell of another document without naming it: "moved".
                    2 => LinkTarget::Cell {
                        doc: None,
                        cell: there,
                    },
                    3 => LinkTarget::Cell {
                        doc: None,
                        cell: pick(doc, n[2], any_cell).unwrap(),
                    },
                    _ => LinkTarget::Cell {
                        doc: Some(other.id()),
                        cell: there,
                    },
                };
                let command = Command::SetLink {
                    cell,
                    start: 0,
                    len,
                    target: Some(target),
                };
                if let Ok(changes) = editors[d].apply(command, None, &mut ids) {
                    follow(&mut index, hook, &editors[d], &changes);
                }
            }
            Act::Group(d, ops) => {
                let d = d % count;
                editors[d].begin_group();
                for op in ops {
                    let command = support::command(editors[d].document(), op);
                    let _ = editors[d].apply(command, None, &mut ids);
                }
                let changes = editors[d].end_group();
                follow(&mut index, hook, &editors[d], &changes);
            }
            Act::Undo(d) => {
                let d = d % count;
                if let Some(changes) = editors[d].undo().unwrap() {
                    follow(&mut index, hook, &editors[d], &changes);
                }
            }
            Act::Redo(d) => {
                let d = d % count;
                if let Some(changes) = editors[d].redo().unwrap() {
                    follow(&mut index, hook, &editors[d], &changes);
                }
            }
            Act::Paste(from, to, n) => {
                let (from, to) = (from % count, to % count);
                let source = editors[from].document();
                let clip = Clip::of(source, pick(source, n[0], any_cell).unwrap()).unwrap();
                let Some(place) = pick(editors[to].document(), n[1], not_root) else {
                    continue;
                };
                let originals: Vec<CellId> = clip.cell_ids().collect();
                let from_id = source.id();
                let paste = Command::PasteCells { to: place, clip };
                let before = editors[to].document().clone();
                let (links, held_before, old) = (meaning(&index), held(&index), index.clone());
                let all_before: Vec<Editor> = editors.clone();
                if let Ok(changes) = editors[to].apply(paste, None, &mut ids) {
                    // As for every command: an exact report, and a size bound that holds.
                    support::check_report(&before, editors[to].document(), &changes, &what);
                    support::check_bound(&editors[to], &what);
                    follow(&mut index, hook, &editors[to], &changes);
                    stats.pastes += usize::from(!changes.is_empty());
                    if changes.is_empty() {
                        continue;
                    }
                    // The links: those of the copies, and everybody else's.
                    let doc = editors[to].document();
                    let mut copies = vec![place];
                    let inside = |c: &CellId| {
                        let mut at = doc.cell(*c).and_then(|c| c.parent());
                        while let Some(up) = at {
                            if up == place {
                                return true;
                            }
                            at = doc.cell(up).and_then(|c| c.parent());
                        }
                        false
                    };
                    copies.extend(doc.cells_in_order().into_iter().filter(inside));
                    let to_id = doc.id();
                    check_pasted(
                        &links,
                        &|cell| homes(&all_before, cell),
                        &editors,
                        &index,
                        (from_id, to_id),
                        &originals,
                        &copies,
                        &mut stats,
                        &what,
                    );
                    let mut ignored = Stats::default();
                    let seen = (&held_before, &editors[..], &index);
                    check_survival(seen.0, seen.1, seen.2, to_id, &changes, &mut ignored, &what);
                    // Undo and redo give the links before and after, exactly.
                    let new = index.clone();
                    let back = editors[to].undo().unwrap().unwrap();
                    follow(&mut index, hook, &editors[to], &back);
                    assert!(index == old, "undo of {what}");
                    assert!(*editors[to].document() == before, "undo of {what}");
                    let again = editors[to].redo().unwrap().unwrap();
                    follow(&mut index, hook, &editors[to], &again);
                    assert!(index == new, "redo of {what}");
                }
            }
            Act::Move(from, to, n) => {
                let (from, to) = (from % count, to % count);
                if from == to {
                    continue;
                }
                let Some(cell) = pick(editors[from].document(), n[0], not_root) else {
                    continue;
                };
                // Mostly onto an empty cell, as the command needs; sometimes not.
                let empty = |doc: &Document, c: CellId| {
                    let cell = doc.cell(c).unwrap();
                    let bare =
                        cell.text.is_empty() && cell.grid().is_none() && cell.image.is_none();
                    not_root(doc, c) && (bare || n[1] % 9 == 0)
                };
                let Some(place) = pick(editors[to].document(), n[1], empty) else {
                    continue;
                };
                let before = (editors[from].clone(), editors[to].clone());
                let (links, old) = (meaning(&index), index.clone());
                let all_before: Vec<Editor> = editors.clone();
                let moving: BTreeSet<CellId> = Clip::of(editors[from].document(), cell)
                    .unwrap()
                    .cell_ids()
                    .collect();
                let (from_id, to_id) = (editors[from].document().id(), editors[to].document().id());
                let mut done = false;
                let (source, target) = if from < to {
                    let (a, b) = editors.split_at_mut(to);
                    (&mut a[from], &mut b[0])
                } else {
                    let (a, b) = editors.split_at_mut(from);
                    (&mut b[0], &mut a[to])
                };
                match move_between(source, cell, target, place, None, &mut ids) {
                    Ok(moved) => {
                        follow(&mut index, hook, source, &moved.source);
                        follow(&mut index, hook, target, &moved.target);
                        let (old_source, old_target) = (before.0.document(), before.1.document());
                        support::check_report_loose(old_source, source.document(), &moved.source);
                        support::check_report_loose(old_target, target.document(), &moved.target);
                        support::check_bound(source, &what);
                        support::check_bound(target, &what);
                        // The cell is where it was sent, and only there.
                        assert!(source.document().cell(cell).is_none());
                        assert!(target.document().cell(cell).is_some());
                        stats.moves += 1;
                        done = true;
                    }
                    Err(_) => {
                        assert!(*source.document() == *before.0.document(), "{what}");
                        assert!(*target.document() == *before.1.document(), "{what}");
                        assert_eq!(source.undo_len(), before.0.undo_len());
                        assert_eq!(target.undo_len(), before.1.undo_len());
                    }
                }
                if done {
                    check_moved(
                        &links,
                        &|cell| homes(&all_before, cell),
                        &editors,
                        &index,
                        (from_id, to_id),
                        &moving,
                        &mut stats,
                        &what,
                    );
                    // Undone in both documents, in either order, everything is as it was,
                    // the links too; and redone, as after the move.
                    let new = index.clone();
                    let order = if n[0] % 2 == 0 {
                        [from, to]
                    } else {
                        [to, from]
                    };
                    for d in order {
                        let back = editors[d].undo().unwrap().unwrap();
                        follow(&mut index, hook, &editors[d], &back);
                        editors[d].document().validate().unwrap();
                    }
                    assert!(index == old, "undo of {what}");
                    assert!(*editors[from].document() == *before.0.document(), "{what}");
                    assert!(*editors[to].document() == *before.1.document(), "{what}");
                    for d in order {
                        let again = editors[d].redo().unwrap().unwrap();
                        follow(&mut index, hook, &editors[d], &again);
                    }
                    assert!(index == new, "redo of {what}");
                }
            }
            Act::Formula(d, n) => {
                let d = d % count;
                let doc = editors[d].document();
                let Some(cell) = pick(doc, n[0], not_root) else {
                    continue;
                };
                // A name: the text of some cell that has a grid.
                let has_grid = |doc: &Document, c: CellId| doc.cell(c).unwrap().grid().is_some();
                let name = pick(doc, n[1], has_grid)
                    .map(|c| doc.cell(c).unwrap().text.replace('"', ""))
                    .unwrap_or_default();
                let text = match n[2] % 8 {
                    0 => format!("=SUM(\"{name}\")"),
                    1 => format!("=TREECOUNT(\"{name}\") + 1"),
                    2 => format!("=COUNT(SUBSET(\"{name}\", 0, -)) & 'x'"),
                    3 => "=1/3".to_string(),
                    4 => format!("{}", n[1] % 50),
                    5 => name,
                    _ => String::new(),
                };
                let command = if n[2] % 8 == 7 {
                    Command::Run
                } else {
                    Command::EnterText { cell, text }
                };
                let before = editors[d].document().clone();
                if let Ok(changes) = editors[d].apply(command, None, &mut ids) {
                    support::check_report(&before, editors[d].document(), &changes, &what);
                    support::check_bound(&editors[d], &what);
                    follow(&mut index, hook, &editors[d], &changes);
                    stats.formulas += 1;
                }
            }
            Act::Retarget(d, n) => {
                let d = d % count;
                let doc = editors[d].document();
                // The cells that links of this document lead to, by the document named.
                let mut named: BTreeMap<Option<DocId>, Vec<CellId>> = BTreeMap::new();
                for id in doc.cells_in_order() {
                    for link in &doc.cell(id).unwrap().links {
                        if let LinkTarget::Cell { doc, cell } = &link.target {
                            named.entry(*doc).or_default().push(*cell);
                        }
                    }
                }
                let Some((from, cells)) = named.into_iter().nth(n[0] as usize % 3) else {
                    continue;
                };
                // All of them, or every other one.
                let cells: Vec<CellId> = cells.into_iter().step_by(1 + n[1] as usize % 2).collect();
                let to = match n[2] as usize % (count + 2) {
                    0 => None,
                    1 => Some(DocId(Id::from_u128(u128::from(n[2])))),
                    other => Some(editors[other - 2].document().id()),
                };
                let before = doc.clone();
                let command = Command::RetargetLinks { cells, from, to };
                let changes = editors[d].apply(command, None, &mut ids).unwrap();
                support::check_report(&before, editors[d].document(), &changes, &what);
                support::check_bound(&editors[d], &what);
                follow(&mut index, hook, &editors[d], &changes);
                stats.retargets += usize::from(!changes.is_empty());
            }
            Act::Repair(d) => {
                let d = d % count;
                let commands = index.repairs(editors[d].document());
                let mut all = true;
                for command in commands {
                    match editors[d].apply(command, None, &mut ids) {
                        Ok(changes) => {
                            follow(&mut index, hook, &editors[d], &changes);
                            stats.repairs += 1;
                        }
                        Err(_) => all = false,
                    }
                }
                let id = editors[d].document().id();
                let moved = |link: &wombat_core::links::Outgoing| {
                    matches!(link.resolved, Resolved::Cell { moved: true, .. })
                };
                assert!(
                    !all || !index.outgoing(id).iter().any(moved),
                    "a moved link is left after {what}"
                );
            }
        }
        check(&editors, &index, &what);
        hook.step(&editors, &what);
        for editor in &editors {
            editor.document().validate().unwrap();
        }
        let formulas: usize = editors.iter().map(|e| e.calc().results().len()).sum();
        stats.formula_cells = stats.formula_cells.max(formulas);
        let links: usize = index.documents().map(|d| d.links().count()).sum();
        stats.links = stats.links.max(links);
    }
    for editor in &mut editors {
        while let Some(changes) = editor.undo().unwrap() {
            follow(&mut index, hook, editor, &changes);
        }
    }
    check(&editors, &index, "undoing everything");
    hook.step(&editors, "undoing everything");
    // Copies and moves between documents are undone like everything else.
    for (editor, original) in editors.iter().zip(&originals) {
        assert!(editor.document() == original, "undoing everything");
    }
    stats
}
