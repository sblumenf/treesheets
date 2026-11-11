//! What the review of milestone 4 found about cells that move between documents: a moved
//! cell could not move back while the history of the first document remembered it
//! (decision A84), and correcting the links to moved cells rewrote them, which lost their
//! unknown fields and joined neighbours (decision A85).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, HashSet};

use wombat_core::construct::Construct;
use wombat_core::links::{move_between, Resolved};
use wombat_core::{
    format, CellId, DocId, Document, Editor, Link, LinkIndex, LinkTarget, SeededIds,
};

/// A document of one column with these texts.
fn doc(seed: u64, texts: &[&str]) -> Document {
    let mut ids = SeededIds::from_u64(seed);
    let mut d = Document::new(&mut ids);
    let root = d.root();
    d.add_grid(root, 1, texts.len() as u32, &mut ids).unwrap();
    let kids = d.cell(root).unwrap().grid().unwrap().cells().to_vec();
    for (k, t) in kids.iter().zip(texts) {
        d.cell_mut(*k).unwrap().text = (*t).to_string();
    }
    d.validate().unwrap();
    d
}

fn kid(d: &Document, y: u32) -> CellId {
    d.child(d.root(), 0, y).unwrap()
}

fn link(start: u32, len: u32, doc: Option<DocId>, cell: CellId) -> Link {
    let mut extra = BTreeMap::new();
    extra.insert("future".to_string(), serde_json::json!({ "keep": start }));
    Link {
        start,
        len,
        target: LinkTarget::Cell { doc, cell },
        extra,
    }
}

/// No cell ID is in a document twice (the model's own check), the documents can be saved,
/// and `cell` is in exactly the documents named.
fn check(editors: &[&Editor], cell: CellId, there: &[usize], what: &str) {
    for (n, editor) in editors.iter().enumerate() {
        let doc = editor.document();
        doc.validate().unwrap();
        let ids = doc.cells_in_order();
        let distinct: HashSet<CellId> = ids.iter().copied().collect();
        assert_eq!(
            ids.len(),
            distinct.len(),
            "{what}: an ID twice in document {n}"
        );
        assert_eq!(
            doc.cell_count(),
            ids.len(),
            "{what}: cells outside the tree"
        );
        format::to_string(doc).unwrap();
        assert_eq!(
            doc.cell(cell).is_some(),
            there.contains(&n),
            "{what}: document {n}"
        );
    }
}

/// Undoes and redoes everything in every order of the editors, checking after each step.
fn undo_and_redo_every_way(editors: &mut [Editor], cell: CellId, what: &str) {
    let orders: &[&[usize]] = match editors.len() {
        2 => &[&[0, 1], &[1, 0]],
        _ => &[&[0, 1, 2], &[2, 1, 0], &[1, 0, 2], &[1, 2, 0]],
    };
    let saved: Vec<String> = editors
        .iter()
        .map(|e| format::to_string(e.document()).unwrap())
        .collect();
    for order in orders {
        let mut copies: Vec<Editor> = editors.to_vec();
        let depth: Vec<usize> = copies.iter().map(Editor::undo_len).collect();
        // One unit at a time, going round the editors in this order, until none is left.
        let mut left = depth.clone();
        while left.iter().any(|n| *n > 0) {
            for n in *order {
                if left[*n] > 0 {
                    copies[*n].undo().unwrap().unwrap();
                    left[*n] -= 1;
                    for copy in &copies {
                        let doc = copy.document();
                        doc.validate().unwrap();
                        let ids = doc.cells_in_order();
                        let distinct: HashSet<CellId> = ids.iter().copied().collect();
                        assert_eq!(ids.len(), distinct.len(), "{what}: an ID twice");
                    }
                }
            }
        }
        // Everything undone: the cell is where it started, and only there.
        let homes = copies
            .iter()
            .filter(|e| e.document().cell(cell).is_some())
            .count();
        assert_eq!(homes, 1, "{what}: the cell after undoing everything");
        assert!(copies[0].document().cell(cell).is_some());
        // And redone, in the same roundabout order: as it was.
        let mut left = depth;
        while left.iter().any(|n| *n > 0) {
            for n in order.iter().rev() {
                if left[*n] > 0 {
                    copies[*n].redo().unwrap().unwrap();
                    left[*n] -= 1;
                    copies[*n].document().validate().unwrap();
                }
            }
        }
        for (copy, saved) in copies.iter().zip(&saved) {
            assert_eq!(
                &format::to_string(copy.document()).unwrap(),
                saved,
                "{what}"
            );
        }
    }
}

#[test]
fn a_moved_cell_can_move_back_and_on_in_the_same_session() {
    let mut ids = SeededIds::from_u64(9);
    let mut a = Editor::new(doc(1, &["one", "two", "", "see one"])).unwrap();
    let mut b = Editor::new(doc(2, &["b1", "", "", ""])).unwrap();
    let mut c = Editor::new(doc(3, &["", "c2"])).unwrap();
    let x = kid(a.document(), 0);
    // Something inside it, and a link to it that must follow it everywhere.
    let seer = kid(a.document(), 3);
    let target = LinkTarget::Cell { doc: None, cell: x };
    let set = wombat_core::Command::SetLink {
        cell: seer,
        start: 4,
        len: 3,
        target: Some(target),
    };
    a.apply(set, None, &mut ids).unwrap();
    let grid = wombat_core::Command::CreateGrid {
        cell: x,
        xs: 1,
        ys: 2,
    };
    a.apply(grid, None, &mut ids).unwrap();
    let inside = a.document().child(x, 0, 1).unwrap();
    let resolves_to = |editors: &[&Editor], n: usize, moved: bool| {
        let mut index = LinkIndex::new();
        for e in editors {
            index.insert_document(e.document());
        }
        let stored = &a_link(editors[0], seer);
        let want = Resolved::Cell {
            doc: editors[n].document().id(),
            cell: x,
            moved,
        };
        assert_eq!(index.resolve(editors[0].document().id(), stored), want);
    };
    fn a_link(editor: &Editor, cell: CellId) -> LinkTarget {
        editor.document().cell(cell).unwrap().links[0]
            .target
            .clone()
    }

    // A -> B -> A.
    let to = kid(b.document(), 1);
    move_between(&mut a, x, &mut b, to, None, &mut ids).unwrap();
    check(&[&a, &b, &c], x, &[1], "A to B");
    resolves_to(&[&a, &b, &c], 1, false);
    let back = kid(a.document(), 2);
    move_between(&mut b, x, &mut a, back, None, &mut ids).unwrap();
    check(&[&a, &b, &c], x, &[0], "B back to A");
    check(&[&a, &b, &c], inside, &[0], "B back to A, the cell inside");
    resolves_to(&[&a, &b, &c], 0, false);
    undo_and_redo_every_way(&mut [a.clone(), b.clone()], x, "A to B to A");

    // And again A -> B -> C -> B.
    let to = kid(b.document(), 2);
    move_between(&mut a, x, &mut b, to, None, &mut ids).unwrap();
    let to = kid(c.document(), 0);
    move_between(&mut b, x, &mut c, to, None, &mut ids).unwrap();
    check(&[&a, &b, &c], x, &[2], "B to C");
    // A is not part of this move: its link still names B, and finds the cell by its ID.
    resolves_to(&[&a, &b, &c], 2, true);
    let to = kid(b.document(), 3);
    move_between(&mut c, x, &mut b, to, None, &mut ids).unwrap();
    check(&[&a, &b, &c], x, &[1], "C back to B");
    check(&[&a, &b, &c], inside, &[1], "C back to B, the cell inside");
    resolves_to(&[&a, &b, &c], 1, false);
    undo_and_redo_every_way(&mut [a.clone(), b.clone(), c.clone()], x, "A B A B C B");

    // Each side on its own, all the way back and forth: never an ID twice in a document.
    for n in 0..3 {
        let mut editors = [a.clone(), b.clone(), c.clone()];
        let mut steps = 0;
        while editors[n].undo().unwrap().is_some() {
            steps += 1;
            editors[n].document().validate().unwrap();
        }
        for _ in 0..steps {
            editors[n].redo().unwrap().unwrap();
            editors[n].document().validate().unwrap();
        }
        let saved = [&a, &b, &c][n];
        assert!(editors[n].document() == saved.document());
    }

    // An ID that is in the document is still refused, whatever the history holds: after
    // undoing the release in B, the cell is in B again and cannot arrive there twice.
    let mut b2 = b.clone();
    let mut c2 = c.clone();
    let to = kid(c2.document(), 0);
    move_between(&mut b2, x, &mut c2, to, None, &mut ids).unwrap();
    b2.undo().unwrap().unwrap();
    assert!(b2.document().cell(x).is_some() && c2.document().cell(x).is_some());
    let place = kid(b2.document(), 1);
    let error = move_between(&mut c2, x, &mut b2, place, None, &mut ids).unwrap_err();
    assert!(error.to_string().contains("already in use"), "{error}");
}

#[test]
fn links_to_a_moved_cell_are_corrected_in_place() {
    let mut ids = SeededIds::from_u64(9);
    let mut a = doc(1, &["target", "abcdef", "stays", ""]);
    let mut b = doc(2, &["b1 xyz", "", ""]);
    let (x, src, stay) = (kid(&a, 0), kid(&a, 1), kid(&a, 2));
    let a_id = a.id();
    // Two links side by side to the cell that will move, each with a field of its own; a
    // link in the moving cell to one that stays; and two in the other document.
    a.cell_mut(src).unwrap().links = vec![link(0, 3, None, x), link(3, 3, None, x)];
    a.cell_mut(x).unwrap().links = vec![link(0, 2, None, stay)];
    let from_b = kid(&b, 0);
    b.cell_mut(from_b).unwrap().links = vec![link(0, 2, Some(a_id), x), link(2, 2, Some(a_id), x)];
    a.validate().unwrap();
    b.validate().unwrap();
    let (mut a, mut b) = (Editor::new(a).unwrap(), Editor::new(b).unwrap());
    let b_id = b.document().id();
    let to = kid(b.document(), 1);
    move_between(&mut a, x, &mut b, to, None, &mut ids).unwrap();
    // In the document it left: still two links, with their fields, now naming B.
    assert_eq!(
        a.document().cell(src).unwrap().links,
        [link(0, 3, Some(b_id), x), link(3, 3, Some(b_id), x)]
    );
    // In the document it arrived in: still two, naming no document.
    assert_eq!(
        b.document().cell(from_b).unwrap().links,
        [link(0, 2, None, x), link(2, 2, None, x)]
    );
    // Its own link names the document it left, and keeps its field.
    assert_eq!(
        b.document().cell(x).unwrap().links,
        [link(0, 2, Some(a_id), stay)]
    );
    // Undone on both sides, everything is as it was, fields included.
    a.undo().unwrap().unwrap();
    b.undo().unwrap().unwrap();
    assert_eq!(
        a.document().cell(src).unwrap().links,
        [link(0, 3, None, x), link(3, 3, None, x)]
    );
    assert_eq!(
        b.document().cell(from_b).unwrap().links,
        [link(0, 2, Some(a_id), x), link(2, 2, Some(a_id), x)]
    );

    // The same for the links of a third document, which `repairs` corrects.
    a.redo().unwrap().unwrap();
    b.redo().unwrap().unwrap();
    let mut c = doc(3, &["to it and to it"]);
    let from_c = kid(&c, 0);
    c.cell_mut(from_c).unwrap().links = vec![link(0, 2, Some(a_id), x), link(2, 3, Some(a_id), x)];
    let mut c = Editor::new(c).unwrap();
    let mut index = LinkIndex::new();
    for e in [&a, &b, &c] {
        index.insert_document(e.document());
    }
    let repairs = index.repairs(c.document());
    assert!(!repairs.is_empty());
    for command in repairs {
        c.apply(command, None, &mut ids).unwrap();
    }
    assert_eq!(
        c.document().cell(from_c).unwrap().links,
        [link(0, 2, Some(b_id), x), link(2, 3, Some(b_id), x)]
    );
}

#[test]
fn a_move_undone_on_one_side_leaves_the_cells_where_the_editors_can_say() {
    let mut ids = SeededIds::from_u64(9);
    let mut a = Editor::new(doc(1, &["one", "two"])).unwrap();
    let mut b = Editor::new(doc(2, &["b1", ""])).unwrap();
    let x = kid(a.document(), 0);
    let to = kid(b.document(), 1);
    move_between(&mut a, x, &mut b, to, None, &mut ids).unwrap();
    assert!(a.holds(x) && !b.holds(x));
    // Undone where it arrived: in neither document, but both histories have it.
    b.undo().unwrap().unwrap();
    assert!(a.document().cell(x).is_none() && b.document().cell(x).is_none());
    assert!(a.holds(x) && b.holds(x));
    // A new command there ends its redo: now only the history of the document it left
    // has the cell, and an interface can know that before that history goes.
    let text = "new".to_string();
    let cell = kid(b.document(), 0);
    b.apply(wombat_core::Command::SetText { cell, text }, None, &mut ids)
        .unwrap();
    assert!(a.holds(x) && !b.holds(x));
    a.undo().unwrap().unwrap();
    assert!(a.document().cell(x).is_some() && !a.holds(x));
}
