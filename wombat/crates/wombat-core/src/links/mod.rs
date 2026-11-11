//! Links and backlinks, within and across documents (decisions B1, B2, A50-A56).
//!
//! A link is a range of a cell's text with a target ([`crate::model::Link`]); a target that
//! is a cell is a cell ID, with a document ID when the cell is in another document. This
//! module answers what a link points at ([`LinkIndex::resolve`]) and what points at a cell
//! ([`LinkIndex::backlinks`]).
//!
//! The [`LinkIndex`] is a disposable copy (decision A13): one [`DocLinks`] per document,
//! which holds the IDs of the document's cells and its links, and a table from each
//! target cell to the cells that link to it. It is built by reading documents
//! ([`LinkIndex::insert_document`]) and kept up to date from the change report of every
//! command, undo, redo and group ([`LinkIndex::apply`]) at a cost that depends on the
//! cells that changed, not on the document. Rebuilding it from the documents always gives
//! an equal index.
//!
//! Nothing here reads files: [`crate::scan::link_index`] builds the index of a workspace
//! folder.

mod transfer;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::Value;

use crate::edit::{Changes, Command};
use crate::id::{CellId, DocId};
use crate::model::{Cell, Document, Link, LinkTarget};
use crate::summary::{id_from_hex, id_to_hex, Summary};

pub use transfer::{copy_between, move_between, Moved};

/// The unknown field in which the `.cts` importer keeps the text that a link without a
/// target pointed at in TreeSheets (decision B14).
pub const CTS_TARGET: &str = "cts_target";

/// One link as the index keeps it.
#[derive(Clone, PartialEq, Debug)]
pub struct LinkEntry {
    /// Position of the first code point of the linked text.
    pub start: u32,
    /// Number of code points.
    pub len: u32,
    /// The linked text.
    pub text: String,
    /// Where the link goes, as it is stored.
    pub target: LinkTarget,
    /// For a link without a target that was imported from TreeSheets: the text of the
    /// cell it pointed at there.
    pub hint: Option<String>,
}

/// The links of one cell, in the order of the text, with their linked texts.
pub fn entries(cell: &Cell) -> Vec<LinkEntry> {
    let text = &cell.text;
    // One pass over the text for all links, which are sorted.
    let mut offsets = text
        .char_indices()
        .map(|(at, _)| at)
        .chain(std::iter::once(text.len()));
    let mut byte = offsets.next().unwrap_or(0);
    let mut at = 0u32;
    let mut seek = |to: u32| {
        while at < to {
            match offsets.next() {
                Some(next) => byte = next,
                None => break,
            }
            at = at.saturating_add(1);
        }
        byte
    };
    cell.links
        .iter()
        .map(|link| {
            let from = seek(link.start);
            let to = seek(link.start.saturating_add(link.len));
            LinkEntry {
                start: link.start,
                len: link.len,
                text: text.get(from..to).unwrap_or_default().to_owned(),
                target: link.target.clone(),
                hint: hint_of(link),
            }
        })
        .collect()
}

fn hint_of(link: &Link) -> Option<String> {
    if link.target != LinkTarget::Unknown {
        return None;
    }
    link.extra
        .get(CTS_TARGET)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// What the link index keeps about one document: the IDs of its cells, and the links of
/// the cells that have any.
#[derive(Clone, PartialEq, Debug)]
pub struct DocLinks {
    id: DocId,
    cells: HashSet<CellId>,
    links: BTreeMap<CellId, Vec<LinkEntry>>,
}

/// From a target cell to the cells that link to it, whatever document the link names.
type Back = HashMap<CellId, BTreeSet<(DocId, CellId)>>;

fn link_back(back: &mut Back, doc: DocId, from: CellId, entries: &[LinkEntry]) {
    for entry in entries {
        if let LinkTarget::Cell { cell, .. } = entry.target {
            back.entry(cell).or_default().insert((doc, from));
        }
    }
}

fn unlink_back(back: &mut Back, doc: DocId, from: CellId, entries: &[LinkEntry]) {
    for entry in entries {
        if let LinkTarget::Cell { cell, .. } = entry.target {
            if let Some(sources) = back.get_mut(&cell) {
                sources.remove(&(doc, from));
                if sources.is_empty() {
                    back.remove(&cell);
                }
            }
        }
    }
}

impl DocLinks {
    /// The document's ID.
    pub fn id(&self) -> DocId {
        self.id
    }

    /// Whether the document has this cell.
    pub fn has_cell(&self, cell: CellId) -> bool {
        self.cells.contains(&cell)
    }

    /// The number of cells of the document.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// The links of a cell, in the order of its text; empty if it has none.
    pub fn links_of(&self, cell: CellId) -> &[LinkEntry] {
        self.links.get(&cell).map_or(&[], Vec::as_slice)
    }

    /// The cells that have links, by ID, each with its links.
    pub fn links(&self) -> impl Iterator<Item = (CellId, &[LinkEntry])> {
        self.links
            .iter()
            .map(|(cell, list)| (*cell, list.as_slice()))
    }

    /// Follows a change report, keeping `back` in step if there is one.
    fn follow(&mut self, mut back: Option<&mut Back>, doc: &Document, changes: &Changes) {
        let id = self.id;
        for cell in changes.removed.iter().chain(&changes.changed) {
            if let Some(old) = self.links.remove(cell) {
                if let Some(back) = back.as_deref_mut() {
                    unlink_back(back, id, *cell, &old);
                }
            }
        }
        for cell in &changes.removed {
            self.cells.remove(cell);
        }
        for cell in &changes.created {
            self.cells.insert(*cell);
        }
        for cell in changes.created.iter().chain(&changes.changed) {
            let Some(found) = doc.cell(*cell).filter(|c| !c.links.is_empty()) else {
                continue;
            };
            let new = entries(found);
            if let Some(back) = back.as_deref_mut() {
                link_back(back, id, *cell, &new);
            }
            self.links.insert(*cell, new);
        }
    }
}

impl Summary for DocLinks {
    const KIND: &'static str = "links";
    const VERSION: u32 = 1;

    fn of(doc: &Document) -> DocLinks {
        let mut links = BTreeMap::new();
        for (id, cell) in &doc.cells {
            if !cell.links.is_empty() {
                links.insert(*id, entries(cell));
            }
        }
        DocLinks {
            id: doc.id(),
            cells: doc.cells.keys().copied().collect(),
            links,
        }
    }

    fn update(&mut self, doc: &Document, changes: &Changes) {
        self.follow(None, doc, changes);
    }

    /// `{"id": hex, "cells": all cell IDs as hex, sorted, in one string, "links": [[cell,
    /// start, length, text, kind, ...]]}` with kind `"u"` and the target, `"c"` and the
    /// cell and, if named, the document, or `"?"` and, if there is one, the hint.
    fn encode(&self) -> Value {
        let mut id = String::new();
        id_to_hex(self.id.0, &mut id);
        let mut sorted: Vec<CellId> = self.cells.iter().copied().collect();
        sorted.sort_unstable();
        let mut cells = String::with_capacity(sorted.len().saturating_mul(32));
        for cell in sorted {
            id_to_hex(cell.0, &mut cells);
        }
        let mut links = Vec::new();
        for (cell, list) in &self.links {
            let mut from = String::new();
            id_to_hex(cell.0, &mut from);
            for entry in list {
                let mut item = vec![
                    Value::from(from.clone()),
                    Value::from(entry.start),
                    Value::from(entry.len),
                    Value::from(entry.text.clone()),
                ];
                match &entry.target {
                    LinkTarget::External(target) => {
                        item.push("u".into());
                        item.push(target.clone().into());
                    }
                    LinkTarget::Cell { doc, cell } => {
                        item.push("c".into());
                        let mut hex = String::new();
                        id_to_hex(cell.0, &mut hex);
                        item.push(hex.into());
                        if let Some(doc) = doc {
                            let mut hex = String::new();
                            id_to_hex(doc.0, &mut hex);
                            item.push(hex.into());
                        }
                    }
                    LinkTarget::Unknown => {
                        item.push("?".into());
                        if let Some(hint) = &entry.hint {
                            item.push(hint.clone().into());
                        }
                    }
                }
                links.push(Value::Array(item));
            }
        }
        serde_json::json!({"id": id, "cells": cells, "links": links})
    }

    fn decode(value: &Value) -> Option<DocLinks> {
        let object = value.as_object()?;
        if object.len() != 3 {
            return None;
        }
        let id = DocId(id_from_hex(object.get("id")?.as_str()?)?);
        let hex = object.get("cells")?.as_str()?;
        if !hex.is_ascii() || hex.len() % 32 != 0 {
            return None;
        }
        let mut cells = HashSet::with_capacity(hex.len() / 32);
        let mut rest = hex;
        while !rest.is_empty() {
            let (one, more) = rest.split_at_checked(32)?;
            if !cells.insert(CellId(id_from_hex(one)?)) {
                return None;
            }
            rest = more;
        }
        let mut links: BTreeMap<CellId, Vec<LinkEntry>> = BTreeMap::new();
        for item in object.get("links")?.as_array()? {
            let item = item.as_array()?;
            let text_at = |n: usize| item.get(n).and_then(Value::as_str);
            let number_at = |n: usize| {
                let number = item.get(n).and_then(Value::as_u64)?;
                u32::try_from(number).ok()
            };
            let from = CellId(id_from_hex(text_at(0)?)?);
            let (target, hint, used) = match (text_at(4)?, item.len()) {
                ("u", 6) => (LinkTarget::External(text_at(5)?.to_owned()), None, 6),
                ("c", 6 | 7) => {
                    let cell = CellId(id_from_hex(text_at(5)?)?);
                    let doc = match item.get(6) {
                        Some(doc) => Some(DocId(id_from_hex(doc.as_str()?)?)),
                        None => None,
                    };
                    (LinkTarget::Cell { doc, cell }, None, item.len())
                }
                ("?", 5) => (LinkTarget::Unknown, None, 5),
                ("?", 6) => (LinkTarget::Unknown, Some(text_at(5)?.to_owned()), 6),
                _ => return None,
            };
            if used != item.len() || !cells.contains(&from) {
                return None;
            }
            let entry = LinkEntry {
                start: number_at(1)?,
                len: number_at(2)?,
                text: text_at(3)?.to_owned(),
                target,
                hint,
            };
            let list = links.entry(from).or_default();
            // In the order of the text, as `entries` gives them.
            if list.last().is_some_and(|last| last.start >= entry.start) {
                return None;
            }
            list.push(entry);
        }
        Some(DocLinks { id, cells, links })
    }
}

/// What a link points at.
#[derive(Clone, PartialEq, Debug)]
pub enum Resolved {
    /// Something outside Wombat: a URL or a file path ([`external`] tells which).
    External(String),
    /// A cell that exists.
    Cell {
        /// The document the cell is in.
        doc: DocId,
        /// The cell.
        cell: CellId,
        /// Whether the cell was found in another document than the link says: it was
        /// moved there, and the link can be corrected ([`LinkIndex::repairs`]).
        moved: bool,
    },
    /// Nothing.
    Broken(Broken),
}

/// Why a link points at nothing.
#[derive(Clone, PartialEq, Debug)]
pub enum Broken {
    /// The document it names is not known, and no known document has the cell.
    NoDocument(DocId),
    /// The document it is said to be in (given here) has no such cell, and no other known
    /// document has it either.
    NoCell(DocId),
    /// Several files have the document ID that the link leads to (copies of a file made by
    /// hand, decision A28), so it cannot be said which is meant.
    AmbiguousDocument(DocId),
    /// The cell is not where the link says, and several documents (given here, sorted)
    /// have a cell with that ID, so it cannot be said which is meant.
    AmbiguousCell(Vec<DocId>),
    /// The link has no target that this version knows: an imported TreeSheets link whose
    /// text matched no cell ([`unknown_links`]), or something newer.
    Unknown,
}

/// One link of a document with what it points at.
#[derive(Clone, PartialEq, Debug)]
pub struct Outgoing {
    /// The document that has the link.
    pub doc: DocId,
    /// The cell that has the link.
    pub cell: CellId,
    /// The link.
    pub link: LinkEntry,
    /// What it points at.
    pub resolved: Resolved,
}

/// One link that points at a given cell.
#[derive(Clone, PartialEq, Debug)]
pub struct Backlink {
    /// The document that has the link.
    pub doc: DocId,
    /// The cell that has the link.
    pub cell: CellId,
    /// Position of the first code point of the linked text.
    pub start: u32,
    /// Number of code points.
    pub len: u32,
    /// The linked text.
    pub text: String,
    /// The cell the link points at.
    pub target: CellId,
    /// Whether the link names another document than the one the target is in now.
    pub moved: bool,
}

/// The links of a set of documents, for resolving links and listing backlinks. See the
/// module documentation.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct LinkIndex {
    docs: BTreeMap<DocId, DocLinks>,
    back: Back,
    ambiguous: BTreeSet<DocId>,
}

impl LinkIndex {
    /// An index of no documents.
    pub fn new() -> LinkIndex {
        LinkIndex::default()
    }

    /// Reads a document into the index, in place of what the index had under its ID.
    pub fn insert_document(&mut self, doc: &Document) {
        self.insert(DocLinks::of(doc));
    }

    /// Puts the summary of a document into the index, in place of what the index had under
    /// its ID.
    pub fn insert(&mut self, summary: DocLinks) {
        let id = summary.id;
        self.remove(id);
        for (cell, list) in &summary.links {
            link_back(&mut self.back, id, *cell, list);
        }
        self.docs.insert(id, summary);
    }

    /// Takes a document out of the index (it was closed, or deleted); links to its cells
    /// are broken from then on. Returns whether it was there.
    pub fn remove(&mut self, doc: DocId) -> bool {
        let Some(old) = self.docs.remove(&doc) else {
            return false;
        };
        for (cell, list) in &old.links {
            unlink_back(&mut self.back, doc, *cell, list);
        }
        true
    }

    /// Brings the index up to date after a command, undo, redo or group changed `doc`:
    /// `changes` is what the editor returned, `doc` the document afterwards. Only the
    /// cells that the report names are looked at. A document the index does not have yet
    /// is read whole.
    pub fn apply(&mut self, doc: &Document, changes: &Changes) {
        match self.docs.get_mut(&doc.id()) {
            Some(summary) => summary.follow(Some(&mut self.back), doc, changes),
            None => self.insert_document(doc),
        }
    }

    /// Says that several files have this document ID, so that links into the document are
    /// reported as ambiguous instead of resolved to whichever file was read.
    pub fn mark_ambiguous(&mut self, doc: DocId) {
        self.ambiguous.insert(doc);
    }

    /// The documents in the index, by ID.
    pub fn documents(&self) -> impl Iterator<Item = &DocLinks> {
        self.docs.values()
    }

    /// What the index has about one document.
    pub fn document(&self, doc: DocId) -> Option<&DocLinks> {
        self.docs.get(&doc)
    }

    /// The documents that have a cell with this ID, sorted. More than one means that IDs
    /// were duplicated (decision A28).
    pub fn owners(&self, cell: CellId) -> Vec<DocId> {
        self.docs
            .values()
            .filter(|doc| doc.has_cell(cell))
            .map(|doc| doc.id)
            .collect()
    }

    /// What a link in the document `from` with this target points at.
    ///
    /// A cell is looked for where the link says: in `from`, or in the document it names.
    /// If it is not there, it is looked for in all documents of the index, because cell
    /// IDs are unique across documents (decision A5) and a cell can move to another
    /// document with its ID: found in exactly one, the link resolves there and is marked
    /// `moved`; found in several, or in a document whose ID several files have, nothing is
    /// guessed and the link is reported as ambiguous.
    pub fn resolve(&self, from: DocId, target: &LinkTarget) -> Resolved {
        let (doc, cell) = match target {
            LinkTarget::External(target) => return Resolved::External(target.clone()),
            LinkTarget::Unknown => return Resolved::Broken(Broken::Unknown),
            LinkTarget::Cell { doc, cell } => (doc.unwrap_or(from), *cell),
        };
        // A document always finds its own cells, also if another file has its ID.
        if doc != from && self.ambiguous.contains(&doc) {
            return Resolved::Broken(Broken::AmbiguousDocument(doc));
        }
        let known = self.docs.get(&doc);
        if known.is_some_and(|d| d.has_cell(cell)) {
            return Resolved::Cell {
                doc,
                cell,
                moved: false,
            };
        }
        let owners = self.owners(cell);
        match owners.as_slice() {
            [] if known.is_some() => Resolved::Broken(Broken::NoCell(doc)),
            [] => Resolved::Broken(Broken::NoDocument(doc)),
            [one] if self.ambiguous.contains(one) && *one != from => {
                Resolved::Broken(Broken::AmbiguousDocument(*one))
            }
            [one] => Resolved::Cell {
                doc: *one,
                cell,
                moved: true,
            },
            _ => Resolved::Broken(Broken::AmbiguousCell(owners)),
        }
    }

    /// All links of a document with what they point at, sorted by the ID of the cell and
    /// then by position. (For document order, walk the document and ask
    /// [`DocLinks::links_of`] for each cell.)
    pub fn outgoing(&self, doc: DocId) -> Vec<Outgoing> {
        let Some(summary) = self.docs.get(&doc) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (cell, list) in &summary.links {
            for link in list {
                out.push(Outgoing {
                    doc,
                    cell: *cell,
                    resolved: self.resolve(doc, &link.target),
                    link: link.clone(),
                });
            }
        }
        out
    }

    /// The links of a document that point at nothing.
    pub fn broken_in(&self, doc: DocId) -> Vec<Outgoing> {
        let mut out = self.outgoing(doc);
        out.retain(|link| matches!(link.resolved, Resolved::Broken(_)));
        out
    }

    /// The links of all documents that point at nothing, sorted by document ID, cell ID
    /// and position.
    pub fn broken(&self) -> Vec<Outgoing> {
        self.docs
            .keys()
            .flat_map(|doc| self.broken_in(*doc))
            .collect()
    }

    /// The links, in all documents of the index, that point at the cell `cell` of the
    /// document `doc`, sorted by the ID of the document that has the link, the ID of the
    /// cell that has it, and the position in its text.
    pub fn backlinks(&self, doc: DocId, cell: CellId) -> Vec<Backlink> {
        let mut out = Vec::new();
        self.backlinks_into(doc, cell, &mut out);
        out
    }

    fn backlinks_into(&self, doc: DocId, cell: CellId, out: &mut Vec<Backlink>) {
        let Some(sources) = self.back.get(&cell) else {
            return;
        };
        for (from_doc, from_cell) in sources {
            let Some(summary) = self.docs.get(from_doc) else {
                continue;
            };
            for link in summary.links_of(*from_cell) {
                if !matches!(link.target, LinkTarget::Cell { cell: to, .. } if to == cell) {
                    continue;
                }
                if let Resolved::Cell {
                    doc: found, moved, ..
                } = self.resolve(*from_doc, &link.target)
                {
                    if found == doc {
                        out.push(Backlink {
                            doc: *from_doc,
                            cell: *from_cell,
                            start: link.start,
                            len: link.len,
                            text: link.text.clone(),
                            target: cell,
                            moved,
                        });
                    }
                }
            }
        }
    }

    /// The links that would point at nothing if `cells` and everything inside them were
    /// deleted from `doc`, for a warning before deleting: the links to those cells from
    /// cells that would stay, in this and other documents. In the order of the cells in
    /// the document, then as [`LinkIndex::backlinks`].
    pub fn would_break(&self, doc: &Document, cells: &[CellId]) -> Vec<Backlink> {
        let mut going = Vec::new();
        let mut seen = HashSet::new();
        let mut stack: Vec<CellId> = cells.iter().rev().copied().collect();
        while let Some(id) = stack.pop() {
            let Some(cell) = doc.cell(id) else { continue };
            if !seen.insert(id) {
                continue;
            }
            going.push(id);
            if let Some(grid) = cell.grid() {
                stack.extend(grid.cells().iter().rev());
            }
        }
        let own = doc.id();
        let mut out = Vec::new();
        for id in going {
            self.backlinks_into(own, id, &mut out);
        }
        out.retain(|link| link.doc != own || !seen.contains(&link.cell));
        out
    }

    /// The links from other documents that would point at nothing if the document `doc`
    /// were deleted, sorted by target cell ID and then as [`LinkIndex::backlinks`].
    pub fn would_break_document(&self, doc: DocId) -> Vec<Backlink> {
        let Some(summary) = self.docs.get(&doc) else {
            return Vec::new();
        };
        let mut targets: Vec<CellId> = summary
            .cells
            .iter()
            .filter(|cell| self.back.contains_key(cell))
            .copied()
            .collect();
        targets.sort_unstable();
        let mut out = Vec::new();
        for cell in targets {
            self.backlinks_into(doc, cell, &mut out);
        }
        out.retain(|link| link.doc != doc);
        out
    }

    /// The commands that correct the links of `doc` whose cell has moved to another
    /// document than they say ([`Resolved::Cell`] with `moved`): each becomes a link that
    /// names the document the cell is in now. One command for all the links to one cell
    /// that name the same document, in the order of [`LinkIndex::outgoing`]. A corrected
    /// link keeps its range and its unknown fields (decision A85).
    pub fn repairs(&self, doc: &Document) -> Vec<Command> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for link in self.outgoing(doc.id()) {
            if let (
                Resolved::Cell {
                    doc: found,
                    cell,
                    moved: true,
                },
                LinkTarget::Cell { doc: named, .. },
            ) = (link.resolved, &link.link.target)
            {
                if seen.insert((*named, cell)) {
                    out.push(Command::RetargetLinks {
                        cells: vec![cell],
                        from: *named,
                        to: Some(found),
                    });
                }
            }
        }
        out
    }
}

/// Where a cell is, for showing a link's target: its text and the texts of the cells it is
/// inside of.
#[derive(Clone, PartialEq, Debug)]
pub struct Place {
    /// The text of the cell.
    pub text: String,
    /// The texts of the cells around it, from the root cell down to the cell whose grid
    /// it is in; empty for the root cell.
    pub trail: Vec<String>,
}

/// The text and location trail of a cell of `doc`, as stored: of a formula cell its
/// formula. `None` if there is no such cell. To show a cell to someone, use
/// [`place_shown`].
pub fn place(doc: &Document, cell: CellId) -> Option<Place> {
    place_by(doc, cell, &|id| doc.cell(id).map(|c| c.text.as_str()))
}

/// The text and location trail of a cell of `doc` as the cells show them: for a formula
/// cell its result (`calc` holds the results of `doc`), as links go by what cells show
/// (decision A66).
pub fn place_shown(doc: &Document, calc: &crate::eval::Calc, cell: CellId) -> Option<Place> {
    place_by(doc, cell, &|id| calc.shown(doc, id))
}

fn place_by<'a>(
    doc: &'a Document,
    cell: CellId,
    text: &dyn Fn(CellId) -> Option<&'a str>,
) -> Option<Place> {
    let found = doc.cell(cell)?;
    let mut trail = Vec::new();
    let mut up = found.parent();
    while let Some(id) = up {
        // A valid document has no loops; this keeps a broken one from hanging.
        if trail.len() > doc.cell_count() {
            return None;
        }
        let parent = doc.cell(id)?;
        trail.push(text(id)?.to_string());
        up = parent.parent();
    }
    trail.reverse();
    Some(Place {
        text: text(cell)?.to_string(),
        trail,
    })
}

/// The link of `cell` to follow, as TreeSheets' "open link" picks it: with a cursor
/// position (text is being edited), the link at that position or else the one that ends
/// there; without one, the first link of the cell.
pub fn link_at(doc: &Document, cell: CellId, cursor: Option<u32>) -> Option<&Link> {
    let links = &doc.cell(cell)?.links;
    let Some(cursor) = cursor else {
        return links.first();
    };
    let covers =
        |link: &&Link, at: u32| link.start <= at && at.saturating_sub(link.start) < link.len;
    links.iter().find(|link| covers(link, cursor)).or_else(|| {
        let before = cursor.checked_sub(1)?;
        links.iter().find(|link| covers(link, before))
    })
}

/// What an external target is, so that an interface knows how to open it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum External {
    /// A URL with a scheme (`https:`, `mailto:`...), for the browser or the system.
    Url,
    /// A file path, absolute or relative to the folder of the document.
    Path,
}

/// Tells a URL from a file path by TreeSheets' rule (`Text::HasURLScheme`): a URL starts
/// with a letter followed by at least one more letter, digit, `+`, `-` or `.`, and then a
/// colon. (One letter and a colon is a Windows drive.)
pub fn external(target: &str) -> External {
    let scheme = target.split_once(':').map_or("", |(scheme, _)| scheme);
    let mut chars = scheme.chars();
    let url = chars.next().is_some_and(char::is_alphabetic)
        && scheme.chars().count() >= 2
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if url {
        External::Url
    } else {
        External::Path
    }
}

/// An imported link that has no target (decision B14), with the cells it could go to.
#[derive(Clone, PartialEq, Debug)]
pub struct UnknownLink {
    /// The cell that has the link.
    pub cell: CellId,
    /// Position of the first code point of the linked text.
    pub start: u32,
    /// Number of code points.
    pub len: u32,
    /// The text of the cell that the link pointed at in TreeSheets.
    pub hint: String,
    /// The cells of the document that show exactly that text now (a formula cell its
    /// result, decision A66), in document order, without the root and the linking cell.
    pub candidates: Vec<CellId>,
}

/// The links of `doc` that were imported from TreeSheets without a target, in document
/// order, each with the cells that now show the text it named. Reads the whole document.
pub fn unknown_links(doc: &Document) -> Vec<UnknownLink> {
    let order = doc.cells_in_order();
    let mut found = Vec::new();
    for id in &order {
        let Some(cell) = doc.cell(*id) else { continue };
        for link in &cell.links {
            if let Some(hint) = hint_of(link) {
                found.push(UnknownLink {
                    cell: *id,
                    start: link.start,
                    len: link.len,
                    hint,
                    candidates: Vec::new(),
                });
            }
        }
    }
    if found.is_empty() {
        return found;
    }
    let mut by_text: HashMap<&str, Vec<CellId>> = found
        .iter()
        .map(|link| (link.hint.as_str(), Vec::new()))
        .collect();
    // TreeSheets' links name a cell by what it shows, which for a formula cell is its
    // result.
    let calc = crate::eval::Calc::new(doc);
    for id in order.iter().skip(1) {
        let Some(shown) = calc.shown(doc, *id) else {
            continue;
        };
        if let Some(list) = by_text.get_mut(shown) {
            list.push(*id);
        }
    }
    let candidates: Vec<Vec<CellId>> = found
        .iter()
        .map(|link| {
            let all = by_text.get(link.hint.as_str());
            let others = all.into_iter().flatten().filter(|c| **c != link.cell);
            others.copied().collect()
        })
        .collect();
    for (link, list) in found.iter_mut().zip(candidates) {
        link.candidates = list;
    }
    found
}

/// The commands that give a target to the imported links without one for which exactly one
/// cell of the document now has the text they named. A link with no such cell, or with
/// several, is left alone: nothing is guessed. Giving a link a target drops the text it
/// kept (the unknown field [`CTS_TARGET`]).
pub fn resolve_unknown(doc: &Document) -> Vec<Command> {
    let mut out = Vec::new();
    for link in unknown_links(doc) {
        if let [only] = link.candidates.as_slice() {
            out.push(Command::SetLink {
                cell: link.cell,
                start: link.start,
                len: link.len,
                target: Some(LinkTarget::Cell {
                    doc: None,
                    cell: *only,
                }),
            });
        }
    }
    out
}
