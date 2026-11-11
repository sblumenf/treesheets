//! Turning TreeSheets' links by text into Wombat's links by cell ID (decision B1).
//!
//! TreeSheets finds the target of an internal link each time the link is followed
//! (`FormulaEvaluator::LinkTarget` and `ForNearest` in `src/evaluator.h`): starting with the
//! cell that holds the grid the linking cell is in, and then each cell around that one in
//! turn, it searches that cell and everything inside it breadth first (the cell itself, then
//! the cells of its grid row by row, then the cells of their grids, and so on) for the first
//! cell, other than the linking cell and the root, whose text is the target. The text of a
//! formula cell is there what it shows: its result, which the core works out
//! (`wombat_core::eval::Calc`).
//!
//! Searching like that for every link would take time proportional to links times cells,
//! which a hostile file could make very long. [`Resolver`] gives the same answers from an
//! index: among the cells with the wanted text inside a given cell, the first one breadth
//! first is the one with the smallest (depth, position in document order).

use std::collections::HashMap;

use wombat_core::eval::Calc;
use wombat_core::{CellId, Document};

/// Answers "which cell does a link in this cell to this text go to?" for one document.
pub(crate) struct Resolver<'a> {
    /// The place of each cell in document order.
    place: HashMap<CellId, u32>,
    /// By place: the place of the parent (the root: itself), the depth, and the place just
    /// after the last cell inside this one.
    parent: Vec<u32>,
    depth: Vec<u32>,
    end: Vec<u32>,
    order: Vec<CellId>,
    /// By text: the cells that have it, as candidates for being a target.
    by_text: HashMap<&'a str, Candidates>,
}

/// The cells with one text, in document order, with a tree over them for "least key in a
/// range" (a segment tree: `tree[n + i]` is candidate `i`, `tree[i]` the lesser of its two
/// children).
struct Candidates {
    places: Vec<u32>,
    tree: Vec<Key>,
}

/// Depth, then place in document order: the order of a breadth-first search.
type Key = (u32, u32);
const NONE: Key = (u32::MAX, u32::MAX);

impl Candidates {
    fn new(keys: &[Key]) -> Candidates {
        let n = keys.len();
        let mut tree = vec![NONE; n.saturating_mul(2)];
        for (slot, key) in tree.iter_mut().skip(n).zip(keys) {
            *slot = *key;
        }
        for i in (1..n).rev() {
            let left = tree.get(i.saturating_mul(2)).copied().unwrap_or(NONE);
            let right = tree
                .get(i.saturating_mul(2).saturating_add(1))
                .copied()
                .unwrap_or(NONE);
            if let Some(slot) = tree.get_mut(i) {
                *slot = left.min(right);
            }
        }
        Candidates {
            places: keys.iter().map(|key| key.1).collect(),
            tree,
        }
    }

    /// The least key among candidates `from..to`.
    fn least(&self, from: usize, to: usize) -> Key {
        let n = self.places.len();
        let (mut l, mut r) = (from.saturating_add(n), to.min(n).saturating_add(n));
        let mut best = NONE;
        while l < r {
            if l & 1 == 1 {
                best = best.min(self.tree.get(l).copied().unwrap_or(NONE));
                l = l.saturating_add(1);
            }
            if r & 1 == 1 {
                r = r.saturating_sub(1);
                best = best.min(self.tree.get(r).copied().unwrap_or(NONE));
            }
            l /= 2;
            r /= 2;
        }
        best
    }
}

impl<'a> Resolver<'a> {
    /// Indexes `document` for links to the given texts. `calc` has what its formula cells
    /// show; it is needed only if there are texts to find.
    pub(crate) fn new(
        document: &'a Document,
        wanted: &[&'a str],
        calc: Option<&Calc>,
    ) -> Resolver<'a> {
        let order = document.cells_in_order();
        let count = u32::try_from(order.len()).unwrap_or(u32::MAX);
        let place: HashMap<CellId, u32> = order.iter().copied().zip(0..count).collect();
        let mut parent = Vec::with_capacity(order.len());
        let mut depth: Vec<u32> = Vec::with_capacity(order.len());
        for (id, at) in order.iter().zip(0..count) {
            let up = document
                .cell(*id)
                .and_then(|cell| cell.parent())
                .and_then(|p| place.get(&p).copied());
            // A parent comes before its cells in document order, so its depth is known.
            let d = up
                .and_then(|p| depth.get(p as usize))
                .map_or(0, |d| d.saturating_add(1));
            parent.push(up.unwrap_or(at));
            depth.push(d);
        }
        // The end of a cell is the furthest end of anything inside it: go backwards, so that
        // each cell is complete before it is counted into its parent.
        let mut end: Vec<u32> = (0..count).map(|at| at.saturating_add(1)).collect();
        for at in (0..order.len()).rev() {
            let (Some(&own), Some(&up)) = (end.get(at), parent.get(at)) else {
                continue;
            };
            if let Some(slot) = end.get_mut(up as usize) {
                *slot = (*slot).max(own);
            }
        }

        let mut keys: HashMap<&str, Vec<Key>> = wanted.iter().map(|t| (*t, Vec::new())).collect();
        for (id, at) in order.iter().zip(0..count) {
            let Some(cell) = document.cell(*id) else {
                continue;
            };
            // Never the root. A formula cell goes by its result, as in TreeSheets.
            if at == 0 {
                continue;
            }
            let shown = calc
                .and_then(|calc| calc.shown(document, *id))
                .unwrap_or(cell.text.as_str());
            if let (Some(list), Some(d)) = (keys.get_mut(shown), depth.get(at as usize)) {
                list.push((*d, at));
            }
        }
        let by_text = keys
            .into_iter()
            .map(|(text, keys)| (text, Candidates::new(&keys)))
            .collect();
        Resolver {
            place,
            parent,
            depth,
            end,
            order,
            by_text,
        }
    }

    /// The cell that a link in `from` to the text `target` goes to, by TreeSheets' rule.
    pub(crate) fn resolve(&self, from: CellId, target: &str) -> Option<CellId> {
        let own = *self.place.get(&from)?;
        let candidates = self.by_text.get(target)?;
        let places = &candidates.places;
        // Where the linking cell itself is among the candidates, to leave it out.
        let own_index = places.binary_search(&own).ok();
        let mut scope = own;
        while scope != 0 {
            scope = *self.parent.get(scope as usize)?;
            let from_index = places.partition_point(|p| *p < scope);
            let to_index =
                places.partition_point(|p| *p < *self.end.get(scope as usize).unwrap_or(&0));
            let best = match own_index {
                Some(i) if (from_index..to_index).contains(&i) => candidates
                    .least(from_index, i)
                    .min(candidates.least(i.saturating_add(1), to_index)),
                _ => candidates.least(from_index, to_index),
            };
            if best != NONE {
                return self.order.get(best.1 as usize).copied();
            }
        }
        None
    }

    /// The deepest cell's depth.
    pub(crate) fn max_depth(&self) -> u32 {
        self.depth.iter().copied().max().unwrap_or(0)
    }
}
