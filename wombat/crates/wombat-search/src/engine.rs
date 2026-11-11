//! The search index of a set of documents, and searching it (decisions B3, A73-A75).
//!
//! # What matches
//!
//! A word or phrase matches a cell if its folded form ([`crate::fold`]) is found
//! anywhere in the folded form of one of the cell's fields: its text (of a formula cell
//! the formula), what a formula cell shows (its result), its note, the file name of its
//! image, the external targets of its links, or what a cell shows that one of its links
//! leads to. "Anywhere" means that a word need not be complete
//! (`app` finds `apple` while it is being typed) and need not start a word (`apple`
//! finds `pineapple`, and any part of a Chinese sentence is found); how good a match
//! is decides the order instead.
//!
//! # The ranking rule
//!
//! Every word of the query that is not negated gets points for its best match in the
//! cell, fewer being better: 4 times the field (text 0, displayed result 1, note 2,
//! text of a linked cell 3, image name 4, link target 5) plus the kind of match (0 the
//! whole field, 1 a whole word, 2 the start of a word, 3 inside a word). A word that is
//! not in the cell at all, which `OR` allows, gets 32. The points are added up. Results
//! are ordered by
//!
//! 1. fewer points,
//! 2. (only if asked for) the more recent edit,
//! 3. the shorter text of the cell,
//! 4. the cell nearer the top of its document (smaller depth),
//! 5. the name of the document, then the order of cells within the document.
//!
//! Nothing else plays a part, so the order is the same on every machine and every run.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use unicode_segmentation::UnicodeSegmentation;
use wombat_core::eval::Calc;
use wombat_core::links::{DocLinks, Resolved};
use wombat_core::summary::Summary;
use wombat_core::{CellId, Changes, DocId, Document, LinkIndex, LinkTarget};

use crate::fold::{fold, word_ends, word_starts, FoldMap};
use crate::index::{
    pair_key, single_key, DocIndex, DocSearch, Entry, Proposed, FIELD_IMAGE, FIELD_NOTE,
    FIELD_SHOWN, FIELD_TEXT, FIELD_URLS, NONE,
};
use crate::query::{Filter, Has, Order, Query, QueryError};

/// The points of a word that is not in the cell (see the ranking rule).
pub const ABSENT: u32 = 32;
/// How many user-perceived characters a snippet shows at most.
pub const SNIPPET_WIDTH: usize = 80;

/// The part of a cell in which a match was found.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Field {
    /// The text of the cell.
    Text,
    /// What the cell displays where that differs from its text (a formula's result;
    /// nothing produces one before milestone 5).
    Shown,
    /// The note.
    Note,
    /// The text of a cell that a link of this cell leads to.
    LinkedText,
    /// The file name of the image.
    Image,
    /// The external target of a link (a URL or file path).
    Url,
}

/// A stretch of a text to show for a hit, with the places to highlight.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Snippet {
    /// At most [`SNIPPET_WIDTH`] user-perceived characters of the field, with line
    /// breaks and other control characters turned into spaces.
    pub text: String,
    /// The matches within `text`: start and length in code points.
    pub ranges: Vec<(u32, u32)>,
    /// Whether the field goes on before the snippet.
    pub cut_start: bool,
    /// Whether the field goes on after the snippet.
    pub cut_end: bool,
}

/// One result.
#[derive(Clone, PartialEq, Debug)]
pub struct Hit {
    /// The document.
    pub doc: DocId,
    /// The cell.
    pub cell: CellId,
    /// The field with the best match; [`Field::Text`] when the query has no words.
    pub field: Field,
    /// The content of that field: the cell's text, what it shows (the result of a
    /// formula cell), its note, the image name, what the linked cell shows, or the
    /// link target (of several links, the first that has a
    /// word of the query; the points are for the best).
    pub matched: String,
    /// Where the words of the query are in `matched`: start and length in code points,
    /// in order, never overlapping, never cutting a user-perceived character.
    pub ranges: Vec<(u32, u32)>,
    /// A short stretch of `matched` around the first match.
    pub snippet: Snippet,
    /// The text of the cell, as stored: of a formula cell its formula.
    pub text: String,
    /// What a formula cell shows: its result. `None` for any other cell.
    pub shown: Option<String>,
    /// The location trail: what the cells that the cell is inside of show, from the
    /// root cell down (as [`wombat_core::links::place_shown`] gives it).
    pub trail: Vec<String>,
    /// The points of the match (see the ranking rule); fewer is better.
    pub score: u32,
    /// How deep the cell is: 0 for the root cell.
    pub depth: u32,
    /// When the cell was last edited.
    pub last_edit: Option<i64>,
}

/// Which results to return.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Options {
    /// The order.
    pub order: Order,
    /// How many of the first results to leave out (for the next page).
    pub offset: usize,
    /// How many results to return at most.
    pub limit: usize,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            order: Order::Relevance,
            offset: 0,
            limit: 50,
        }
    }
}

impl Options {
    /// All results, in this order.
    pub fn all(order: Order) -> Options {
        Options {
            order,
            offset: 0,
            limit: usize::MAX,
        }
    }
}

/// The results of a search.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Results {
    /// How many cells match in all.
    pub total: usize,
    /// The results asked for, in order.
    pub hits: Vec<Hit>,
}

/// Cuts a snippet of at most `width` user-perceived characters out of `text`, around
/// the first of `ranges` (start and length in code points).
pub fn snippet(text: &str, ranges: &[(u32, u32)], width: usize) -> Snippet {
    // Where each user-perceived character starts, in bytes and in code points.
    let mut starts: Vec<(usize, u32)> = Vec::new();
    let mut at = 0u32;
    for (byte, cluster) in text.grapheme_indices(true) {
        starts.push((byte, at));
        at = at.saturating_add(cluster.chars().count() as u32);
    }
    let count = starts.len();
    let anchor = ranges.first().map_or(0, |range| range.0);
    let first = starts
        .partition_point(|start| start.1 <= anchor)
        .saturating_sub(1);
    let mut from = first.saturating_sub(width / 4);
    let to = from.saturating_add(width).min(count);
    if to == count {
        from = count.saturating_sub(width);
    }
    let edge = |n: usize| starts.get(n).copied().unwrap_or((text.len(), at));
    let ((byte_from, cp_from), (byte_to, cp_to)) = (edge(from), edge(to));
    let shown = text.get(byte_from..byte_to).unwrap_or_default();
    let clipped = ranges.iter().filter_map(|(start, len)| {
        let start_in = (*start).max(cp_from);
        let end_in = start.saturating_add(*len).min(cp_to);
        (start_in < end_in).then(|| (start_in - cp_from, end_in - start_in))
    });
    Snippet {
        text: shown
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect(),
        ranges: clipped.collect(),
        cut_start: from > 0,
        cut_end: to < count,
    }
}

/// Sorts ranges and joins those that overlap or touch.
pub(crate) fn merge(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    ranges.retain(|range| range.1 > 0);
    ranges.sort_unstable();
    let mut out: Vec<(u32, u32)> = Vec::with_capacity(ranges.len());
    for (start, len) in ranges {
        match out.last_mut() {
            Some(last) if last.0.saturating_add(last.1) >= start => {
                let end = start.saturating_add(len).max(last.0.saturating_add(last.1));
                last.1 = end - last.0;
            }
            _ => out.push((start, len)),
        }
    }
    out
}

/// The points of a field and a kind of match.
pub(crate) fn points(field: Field, kind: u32) -> u32 {
    let field = match field {
        Field::Text => 0,
        Field::Shown => 1,
        Field::Note => 2,
        Field::LinkedText => 3,
        Field::Image => 4,
        Field::Url => 5,
    };
    field * 4 + kind
}

/// The key by which results are sorted: smaller comes first.
pub(crate) type Key = [u64; 6];

/// The sort key of a result. `name_rank` orders the documents by name.
pub(crate) fn sort_key(
    order: Order,
    score: u32,
    last_edit: Option<i64>,
    text_len: usize,
    depth: u32,
    name_rank: usize,
    place: u32,
) -> Key {
    // Later edits first; cells without an edit time last.
    let recency = last_edit.map_or(u64::MAX, |time| !((time as u64) ^ (1 << 63)));
    let (score, len, depth) = (u64::from(score), text_len as u64, u64::from(depth));
    let (name, place) = (name_rank as u64, u64::from(place));
    match order {
        Order::Relevance => [score, len, depth, name, place, 0],
        Order::RelevanceThenRecent => [score, recency, len, depth, name, place],
        Order::Recent => [recency, score, len, depth, name, place],
        Order::Document => [name, place, 0, 0, 0, 0],
    }
}

struct Held {
    search: DocSearch,
    name: String,
}

/// The search index of a set of documents: for each, what [`DocSearch`] keeps, and the
/// link index of milestone 4 for all of them. It is a disposable copy of what the
/// documents say (decision A13). Build it from documents ([`SearchIndex::insert_document`])
/// or from a workspace folder ([`crate::workspace::index`]), and keep it up to date with
/// [`SearchIndex::apply`] after every command, undo, redo and group.
#[derive(Default)]
pub struct SearchIndex {
    docs: BTreeMap<DocId, Held>,
    links: LinkIndex,
}

impl Clone for SearchIndex {
    fn clone(&self) -> SearchIndex {
        let copy = |held: &Held| Held {
            search: held.search.clone(),
            name: held.name.clone(),
        };
        SearchIndex {
            docs: self
                .docs
                .iter()
                .map(|(id, held)| (*id, copy(held)))
                .collect(),
            links: self.links.clone(),
        }
    }
}

impl SearchIndex {
    /// An index of no documents.
    pub fn new() -> SearchIndex {
        SearchIndex::default()
    }

    /// Reads a document into the index, in place of what the index had under its ID.
    /// `calc` holds the results of its formula cells: [`wombat_core::Editor::calc`] of
    /// the editor that has it open, or `Calc::new(doc)` for a document as loaded.
    /// `name` is what the document is called in results and by `doc:` (its file name).
    pub fn insert_document(&mut self, doc: &Document, calc: &Calc, name: &str) {
        self.insert(DocIndex::with(doc, calc), name);
    }

    /// Puts the summary of a document into the index, in place of what the index had
    /// under its ID.
    pub fn insert(&mut self, summary: DocIndex, name: &str) {
        let DocIndex { search, links } = summary;
        self.links.insert(links);
        let name = name.to_owned();
        self.docs.insert(search.id(), Held { search, name });
    }

    /// Takes a document out of the index. Returns whether it was there.
    pub fn remove(&mut self, doc: DocId) -> bool {
        self.links.remove(doc);
        self.docs.remove(&doc).is_some()
    }

    /// Brings the index up to date after a command, undo, redo or group changed `doc`:
    /// `changes` is what the editor returned, `doc` the document afterwards and `calc`
    /// the editor's results at that moment ([`wombat_core::Editor::calc`]), which also
    /// say which formula cells now show something else (decision A90). Call it once
    /// for every change report, before the next command. A document that the index
    /// does not have yet is read whole, under an empty name.
    pub fn apply(&mut self, doc: &Document, calc: &Calc, changes: &Changes) {
        match self.docs.get_mut(&doc.id()) {
            Some(held) => {
                held.search.update(doc, calc, changes);
                self.links.apply(doc, changes);
            }
            None => self.insert_document(doc, calc, ""),
        }
    }

    /// The link index of the same documents.
    pub fn links(&self) -> &LinkIndex {
        &self.links
    }

    /// The link index, to mark document IDs that several files have
    /// ([`LinkIndex::mark_ambiguous`]).
    pub fn links_mut(&mut self) -> &mut LinkIndex {
        &mut self.links
    }

    /// The documents in the index, by ID, with their names.
    pub fn documents(&self) -> impl Iterator<Item = (&DocSearch, &str)> {
        self.docs
            .values()
            .map(|held| (&held.search, held.name.as_str()))
    }

    /// What the index has about one document.
    pub fn document(&self, doc: DocId) -> Option<&DocSearch> {
        self.docs.get(&doc).map(|held| &held.search)
    }

    /// The name a document was given.
    pub fn name(&self, doc: DocId) -> Option<&str> {
        self.docs.get(&doc).map(|held| held.name.as_str())
    }

    /// The number of cells in all documents.
    pub fn cell_count(&self) -> usize {
        self.docs
            .values()
            .map(|held| held.search.cell_count())
            .sum()
    }

    /// The memory the search index takes, in bytes, as far as it can be counted
    /// ([`DocSearch::memory_bytes`]); the link index is not included.
    pub fn memory_bytes(&self) -> usize {
        let memory = |held: &Held| held.search.memory_bytes() + held.name.capacity();
        self.docs.values().map(memory).sum()
    }

    /// The bytes of text that are searched.
    pub fn text_bytes(&self) -> usize {
        self.docs
            .values()
            .map(|held| held.search.text_bytes())
            .sum()
    }

    /// Checks what searches rely on ([`DocSearch::check`]). For tests.
    #[doc(hidden)]
    pub fn check(&self) -> Result<(), String> {
        self.docs.values().try_for_each(|held| held.search.check())
    }

    /// The summaries of the documents, as bytes, by ID: equal for two indexes of equal
    /// documents, however each came about. For tests.
    #[doc(hidden)]
    pub fn encoded(&self) -> Vec<(DocId, Vec<u8>)> {
        let encode = |(id, held): (&DocId, &Held)| {
            let links = self.links.document(*id).map(DocLinks::encode);
            let mut bytes = held.search.encode();
            bytes.extend(links.unwrap_or_default());
            (*id, bytes)
        };
        self.docs.iter().map(encode).collect()
    }

    /// The cells that match `query`: how many there are, and the ones that `options`
    /// asks for, in order. Fails only for a query over the limits of [`crate::query`].
    pub fn search(&self, query: &Query, options: &Options) -> Result<Results, QueryError> {
        query.check()?;
        let run = Run::new(self, query);
        let mut found = run.matches(options.order, None);
        let total = found.len();
        let end = options.offset.saturating_add(options.limit);
        if end < found.len() && end > 0 {
            // Only the part asked for needs sorting.
            found.select_nth_unstable(end - 1);
            found.truncate(end);
        }
        found.sort_unstable();
        let page = found.iter().skip(options.offset).take(options.limit);
        let hits = page.filter_map(|found| run.hit(found)).collect();
        Ok(Results { total, hits })
    }

    /// The next cell after `from` that matches `query`, in the order of the document
    /// `doc` (the previous one with `reverse`), going round from the end to the start as
    /// TreeSheets' "next match" does: so with one match, that match again; `None` if
    /// nothing in the document matches. With `from` not a cell of the document, the
    /// first match (the last with `reverse`).
    pub fn next_match(
        &self,
        query: &Query,
        doc: DocId,
        from: Option<CellId>,
        reverse: bool,
    ) -> Result<Option<Hit>, QueryError> {
        query.check()?;
        let Some(held) = self.docs.get(&doc) else {
            return Ok(None);
        };
        let run = Run::new(self, query);
        let mut found = run.matches(Order::Document, Some(doc));
        found.sort_unstable();
        let order = held.search.order();
        let here = from
            .and_then(|cell| held.search.slot(cell))
            .and_then(|slot| order.get(slot as usize).copied())
            .map(u64::from);
        let place = |found: &Found| found.key.get(1).copied().unwrap_or(0);
        let next = match (here, reverse) {
            (None, false) => found.first(),
            (None, true) => found.last(),
            (Some(here), false) => {
                let after = found.iter().find(|found| place(found) > here);
                after.or(found.first())
            }
            (Some(here), true) => {
                let before = found.iter().rev().find(|found| place(found) < here);
                before.or(found.last())
            }
        };
        Ok(next.and_then(|found| run.hit(found)))
    }
}

/// One matching cell before it is made into a [`Hit`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Found {
    key: Key,
    doc: DocId,
    slot: u32,
    score: u32,
}

/// A word of the query, ready to be looked for.
struct Needle {
    /// The folded form, or the word itself if it must be found exactly.
    text: String,
    exact: bool,
    /// Whether it counts for ranking and highlighting: it is not under a negation.
    positive: bool,
}

/// A filter, ready to be tested.
enum Test {
    Tag(String),
    Has(Has),
    /// One of these cells: what the link filters come to.
    OneOf(HashSet<(DocId, CellId)>),
    Edited(Option<i64>, Option<i64>),
    Recent(u8),
    Docs(BTreeSet<DocId>),
    Within(CellId),
    Kind(wombat_core::CellKind),
    CellColor(wombat_core::Color),
    TextColor(wombat_core::Color),
    Style(wombat_core::Style),
    TextIs(String),
    Image(String),
}

enum Node {
    All,
    Term(usize),
    Test(Test),
    And(Vec<Node>),
    Or(Vec<Node>),
    Not(Box<Node>),
}

/// One place where a word was found in a cell.
struct Occurrence {
    field: Field,
    /// Which of the cell's link targets or links, for those two fields.
    sub: u32,
    /// The bytes of the match within the field's folded form (or within the field
    /// itself, for a word that must be found exactly).
    from: usize,
    to: usize,
    kind: u32,
}

/// Whether a document name is what `doc:` asks for.
pub(crate) fn name_is(name: &str, wanted: &str) -> bool {
    let plain = |name: &str| {
        let lower = name.to_lowercase();
        lower
            .strip_suffix(".wombat")
            .map(str::to_owned)
            .unwrap_or(lower)
    };
    plain(name) == plain(wanted)
}

/// One search: the query made ready, over one index.
struct Run<'a> {
    index: &'a SearchIndex,
    node: Node,
    needles: Vec<Needle>,
    /// The documents by name, for the order of results.
    ranks: BTreeMap<DocId, usize>,
}

impl<'a> Run<'a> {
    fn new(index: &'a SearchIndex, query: &Query) -> Run<'a> {
        let mut names: Vec<(&str, DocId)> = index
            .docs
            .iter()
            .map(|(id, held)| (held.name.as_str(), *id))
            .collect();
        names.sort_unstable();
        let ranks = names
            .iter()
            .enumerate()
            .map(|(n, (_, id))| (*id, n))
            .collect();
        let mut run = Run {
            index,
            node: Node::All,
            needles: Vec::new(),
            ranks,
        };
        run.node = run.compile(query, true);
        run
    }

    fn compile(&mut self, query: &Query, positive: bool) -> Node {
        match query {
            Query::All => Node::All,
            Query::Term(term) => {
                let text = if term.exact {
                    term.text.clone()
                } else {
                    fold(&term.text)
                };
                self.needles.push(Needle {
                    text,
                    exact: term.exact,
                    positive,
                });
                Node::Term(self.needles.len() - 1)
            }
            Query::Filter(filter) => Node::Test(self.test(filter)),
            Query::And(list) => Node::And(list.iter().map(|q| self.compile(q, positive)).collect()),
            Query::Or(list) => Node::Or(list.iter().map(|q| self.compile(q, positive)).collect()),
            Query::Not(inner) => Node::Not(Box::new(self.compile(inner, false))),
        }
    }

    fn test(&self, filter: &Filter) -> Test {
        let links = &self.index.links;
        match filter {
            Filter::Tag(name) => Test::Tag(name.clone()),
            Filter::Has(Has::BrokenLink) => {
                let broken = links.broken();
                Test::OneOf(broken.iter().map(|link| (link.doc, link.cell)).collect())
            }
            Filter::Has(has) => Test::Has(*has),
            Filter::LinksTo(cell) => {
                let mut from = HashSet::new();
                for doc in links.owners(*cell) {
                    let back = links.backlinks(doc, *cell);
                    from.extend(back.iter().map(|link| (link.doc, link.cell)));
                }
                Test::OneOf(from)
            }
            Filter::LinkedFrom(cell) => {
                let mut to = HashSet::new();
                for doc in links.owners(*cell) {
                    let list = links.document(doc).map_or(&[][..], |d| d.links_of(*cell));
                    for link in list {
                        if let Resolved::Cell { doc, cell, .. } = links.resolve(doc, &link.target) {
                            to.insert((doc, cell));
                        }
                    }
                }
                Test::OneOf(to)
            }
            Filter::Edited { from, to } => Test::Edited(*from, *to),
            Filter::Recent(percent) => Test::Recent(*percent),
            Filter::Doc(doc) => Test::Docs(BTreeSet::from([*doc])),
            Filter::DocName(name) => {
                let named = self
                    .index
                    .docs
                    .iter()
                    .filter(|(_, held)| name_is(&held.name, name));
                Test::Docs(named.map(|(id, _)| *id).collect())
            }
            Filter::Within(cell) => Test::Within(*cell),
            Filter::Kind(kind) => Test::Kind(*kind),
            Filter::CellColor(color) => Test::CellColor(*color),
            Filter::TextColor(color) => Test::TextColor(*color),
            Filter::Style(style) => Test::Style(*style),
            Filter::TextIs(text) => Test::TextIs(text.clone()),
            Filter::Image(name) => Test::Image(name.clone()),
        }
    }

    /// The documents that the query can match in at all, where it says so at its top.
    fn scope(&self, node: &Node) -> Option<BTreeSet<DocId>> {
        match node {
            Node::Test(Test::Docs(docs)) => Some(docs.clone()),
            Node::Test(Test::Within(cell)) => {
                Some(self.index.links.owners(*cell).into_iter().collect())
            }
            Node::And(list) => {
                let mut scopes = list.iter().filter_map(|node| self.scope(node));
                let first = scopes.next()?;
                Some(scopes.fold(first, |all, next| &all & &next))
            }
            _ => None,
        }
    }

    /// The cells that can match `node`, per document, as slots: more than match,
    /// perhaps (and perhaps twice), but never fewer. `None` for "any cell".
    fn proposed(&self, node: &Node) -> Option<BTreeMap<DocId, Vec<u32>>> {
        match node {
            Node::Term(n) => self.proposed_for(self.needles.get(*n)?),
            Node::And(list) => {
                let size =
                    |map: &BTreeMap<DocId, Vec<u32>>| map.values().map(Vec::len).sum::<usize>();
                list.iter()
                    .filter_map(|node| self.proposed(node))
                    .min_by_key(size)
            }
            Node::Or(list) => {
                let mut all: BTreeMap<DocId, Vec<u32>> = BTreeMap::new();
                for node in list {
                    for (doc, slots) in self.proposed(node)? {
                        all.entry(doc).or_default().extend(slots);
                    }
                }
                Some(all)
            }
            Node::All | Node::Test(_) | Node::Not(_) => None,
        }
    }

    fn proposed_for(&self, needle: &Needle) -> Option<BTreeMap<DocId, Vec<u32>>> {
        // A word that must be found exactly is looked for in the texts as they are,
        // which the table, made of folded text, says nothing certain about.
        if needle.exact {
            return None;
        }
        let mut keys = Vec::new();
        let mut before = None;
        for c in needle.text.chars() {
            if let Some(before) = before {
                keys.push(pair_key(before, c));
            }
            before = Some(c);
        }
        if keys.is_empty() {
            // One character: only those of scripts without spaces are in the table.
            keys.push(u64::from(before.filter(|c| single_key(*c))?));
        }
        let mut all: BTreeMap<DocId, Vec<u32>> = BTreeMap::new();
        for (id, held) in &self.index.docs {
            // The shortest list among the word's keys. A key that no cell of the
            // document has means that no cell of it has the word; keys that are all
            // too common to have lists mean that any cell may.
            let mut shortest: Option<&[u32]> = None;
            let mut none = false;
            for key in &keys {
                match held.search.proposed(*key) {
                    Proposed::None => none = true,
                    Proposed::Any => {}
                    Proposed::Some(list) => {
                        if shortest.is_none_or(|shortest| list.len() < shortest.len()) {
                            shortest = Some(list);
                        }
                    }
                }
            }
            match (none, shortest) {
                (true, _) => {}
                (false, Some(list)) => {
                    all.insert(*id, list.to_vec());
                }
                (false, None) => {
                    all.insert(*id, (0..held.search.slot_count()).collect());
                }
            }
        }
        // A cell also matches by the text of a cell that it links to.
        if self.index.links.has_cell_links() {
            let mut linking: Vec<(DocId, u32)> = Vec::new();
            for (id, slots) in &all {
                let Some(held) = self.index.docs.get(id) else {
                    continue;
                };
                for slot in slots {
                    let Some(entry) = held.search.entries.get(*slot as usize) else {
                        continue;
                    };
                    for (doc, cell) in self.index.links.sources(entry.id) {
                        let slot = self
                            .index
                            .docs
                            .get(&doc)
                            .and_then(|held| held.search.slot(cell));
                        linking.extend(slot.map(|slot| (doc, slot)));
                    }
                }
            }
            for (doc, slot) in linking {
                all.entry(doc).or_default().push(slot);
            }
        }
        Some(all)
    }

    /// Every cell that matches, with its sort key; in one document only, if given.
    fn matches(&self, order: Order, only: Option<DocId>) -> Vec<Found> {
        let scope = self.scope(&self.node);
        let proposed = self.proposed(&self.node);
        let mut found = Vec::new();
        for (id, held) in &self.index.docs {
            let out = only.is_some_and(|only| only != *id)
                || scope.as_ref().is_some_and(|scope| !scope.contains(id));
            if out {
                continue;
            }
            let mut doc = InDoc {
                run: self,
                id: *id,
                search: &held.search,
                recent: Vec::new(),
                memo: vec![UNKNOWN; self.needles.len()],
            };
            let rank = self.ranks.get(id).copied().unwrap_or(0);
            let mut one = |doc: &mut InDoc, slot: u32| {
                let Some(entry) = held.search.entry(slot) else {
                    return;
                };
                doc.memo.fill(UNKNOWN);
                if !doc.eval(&self.node, slot, entry) {
                    return;
                }
                let score = doc.score(entry);
                let len = entry.text.chars().count();
                // (Where the cells are is worked out on a document's first match: a
                // document without one is spared that.)
                let depth = held.search.depth(slot);
                let place = held.search.order();
                let at = place.get(slot as usize).copied().unwrap_or(NONE);
                found.push(Found {
                    key: sort_key(order, score, entry.last_edit, len, depth, rank, at),
                    doc: *id,
                    slot,
                    score,
                });
            };
            match &proposed {
                None => {
                    for slot in 0..held.search.entries.len() {
                        one(&mut doc, slot as u32);
                    }
                }
                Some(proposed) => {
                    let Some(slots) = proposed.get(id) else {
                        continue;
                    };
                    let mut seen = vec![false; held.search.entries.len()];
                    for slot in slots {
                        match seen.get_mut(*slot as usize) {
                            Some(seen) if !*seen => *seen = true,
                            _ => continue,
                        }
                        one(&mut doc, *slot);
                    }
                }
            }
        }
        found
    }

    /// The points of the needle's best match in the cell, or [`NO_MATCH`]: what
    /// [`Run::occurrences`] would give, without looking further than needed. (Most
    /// searches end here for most cells, so this is where the time goes.)
    fn best(&self, doc: DocId, entry: &Entry, needle: &Needle) -> u32 {
        if needle.text.is_empty() {
            return 0;
        }
        let mut best = NO_MATCH;
        if needle.exact {
            self.occurrences(doc, entry, needle, &mut |found| {
                best = best.min(points(found.field, found.kind));
            });
            return best;
        }
        // The fields are in the order of their points (text, displayed result, note,
        // image name, link targets), but for the linked texts: so the first field with
        // a match decides, and in it the first match that is a whole word (a match of
        // the whole field is the first one).
        let all = entry.folded();
        let mut at = 0usize;
        let mut first: Option<(usize, usize, usize)> = None;
        while let Some(found) = all.get(at..).and_then(|rest| rest.find(&needle.text)) {
            let from = at + found;
            let to = from + needle.text.len();
            let (field_at, start, end) = *first.get_or_insert_with(|| {
                let before = all.get(..from).unwrap_or_default();
                let start = before.rfind('\n').map_or(0, |last| last + 1);
                let rest = all.get(to..).unwrap_or_default();
                let end = to + rest.find('\n').unwrap_or(rest.len());
                let field_at = before.bytes().filter(|byte| *byte == b'\n').count();
                (field_at, start, end)
            });
            if from >= end {
                break;
            }
            let field = match field_at {
                FIELD_TEXT => Field::Text,
                FIELD_SHOWN => Field::Shown,
                FIELD_NOTE => Field::Note,
                FIELD_IMAGE => Field::Image,
                _ => Field::Url,
            };
            let kind = kind_of(all, from, to, start, end);
            best = best.min(points(field, kind));
            if kind <= 1 || field == Field::Url {
                break;
            }
            at = to;
        }
        // A link target after the first one that has a match may have a better one.
        if first.is_some_and(|(field_at, _, _)| field_at >= FIELD_URLS) {
            best = NO_MATCH;
            self.occurrences(doc, entry, needle, &mut |found| {
                best = best.min(points(found.field, found.kind));
            });
            return best;
        }
        if entry.has_links && best > points(Field::LinkedText, 0) {
            let links = &self.index.links;
            let list = links
                .document(doc)
                .map_or(&[][..], |d| d.links_of(entry.id));
            for link in list {
                if !matches!(link.target, LinkTarget::Cell { .. }) {
                    continue;
                }
                let Some(target) = self.target(doc, &link.target) else {
                    continue;
                };
                find_in(
                    target.folded_display(),
                    &needle.text,
                    Field::LinkedText,
                    0,
                    &mut |found| {
                        best = best.min(points(found.field, found.kind));
                    },
                );
            }
        }
        best
    }

    /// Calls `each` for every place where the needle is found in the cell.
    fn occurrences(
        &self,
        doc: DocId,
        entry: &Entry,
        needle: &Needle,
        each: &mut dyn FnMut(Occurrence),
    ) {
        if needle.text.is_empty() {
            return;
        }
        if needle.exact {
            let urls = entry.urls().iter().map(String::as_str);
            let fields = [
                (Field::Text, Some(&*entry.text)),
                (Field::Shown, entry.shown()),
                (Field::Note, Some(entry.note())),
                (Field::Image, entry.image()),
            ];
            let fields = fields.into_iter().map(|(field, text)| (field, 0, text));
            let urls = urls
                .enumerate()
                .map(|(n, url)| (Field::Url, n as u32, Some(url)));
            for (field, sub, text) in fields.chain(urls) {
                find_in(text.unwrap_or_default(), &needle.text, field, sub, each);
            }
        } else {
            // All fields at once: they are one string, with line breaks between them,
            // and a folded needle has no line break.
            let all = entry.folded();
            let mut field_at = 0usize;
            let mut field_start = 0usize;
            let mut at = 0usize;
            while let Some(found) = all.get(at..).and_then(|rest| rest.find(&needle.text)) {
                let from = at + found;
                let to = from + needle.text.len();
                // The fields passed since the last match.
                let passed = all.get(field_start..from).unwrap_or_default();
                if let Some(last) = passed.rfind('\n') {
                    field_at += passed.bytes().filter(|byte| *byte == b'\n').count();
                    field_start += last + 1;
                }
                let rest = all.get(to..).unwrap_or_default();
                let field_end = to + rest.find('\n').unwrap_or(rest.len());
                let (field, sub) = match field_at {
                    FIELD_TEXT => (Field::Text, 0),
                    FIELD_SHOWN => (Field::Shown, 0),
                    FIELD_NOTE => (Field::Note, 0),
                    FIELD_IMAGE => (Field::Image, 0),
                    n => (Field::Url, n.saturating_sub(FIELD_URLS) as u32),
                };
                each(Occurrence {
                    field,
                    sub,
                    from: from - field_start,
                    to: to - field_start,
                    kind: kind_of(all, from, to, field_start, field_end),
                });
                at = to;
            }
        }
        if !entry.has_links {
            return;
        }
        let links = &self.index.links;
        let list = links
            .document(doc)
            .map_or(&[][..], |d| d.links_of(entry.id));
        for (n, link) in list.iter().enumerate() {
            if !matches!(link.target, LinkTarget::Cell { .. }) {
                continue;
            }
            let Some(target) = self.target(doc, &link.target) else {
                continue;
            };
            let text = if needle.exact {
                target.display()
            } else {
                target.folded_display()
            };
            find_in(text, &needle.text, Field::LinkedText, n as u32, each);
        }
    }

    /// The cell that a link in `doc` leads to, if it leads to one in the index.
    fn target(&self, doc: DocId, target: &LinkTarget) -> Option<&'a Entry> {
        let Resolved::Cell { doc, cell, .. } = self.index.links.resolve(doc, target) else {
            return None;
        };
        let search = &self.index.docs.get(&doc)?.search;
        search.entry(search.slot(cell)?)
    }

    /// The content of a field of a cell.
    fn field_text(&self, doc: DocId, entry: &'a Entry, field: Field, sub: u32) -> Option<&'a str> {
        match field {
            Field::Text => Some(&entry.text),
            Field::Shown => entry.shown(),
            Field::Note => Some(entry.note()),
            Field::Image => entry.image(),
            Field::Url => entry.urls().get(sub as usize).map(String::as_str),
            Field::LinkedText => {
                let links = self.index.links.document(doc)?.links_of(entry.id);
                let target = self.target(doc, &links.get(sub as usize)?.target)?;
                Some(target.display())
            }
        }
    }

    /// Makes the result that is shown for a matching cell.
    fn hit(&self, found: &Found) -> Option<Hit> {
        let search = &self.index.docs.get(&found.doc)?.search;
        let entry = search.entry(found.slot)?;
        // Every place where a word of the query is, and the best field among them.
        let mut places: Vec<(Occurrence, bool)> = Vec::new();
        for needle in self.needles.iter().filter(|needle| needle.positive) {
            self.occurrences(found.doc, entry, needle, &mut |occurrence| {
                places.push((occurrence, needle.exact));
            });
        }
        let best = places
            .iter()
            .map(|(place, _)| (place.field, place.sub))
            .min();
        let (field, sub) = best.unwrap_or((Field::Text, 0));
        let matched = self
            .field_text(found.doc, entry, field, sub)
            .unwrap_or_default();
        places.retain(|(place, _)| (place.field, place.sub) == (field, sub));
        let map = |exact: bool| {
            let wanted = places.iter().any(|(_, is)| *is == exact);
            wanted.then(|| {
                if exact {
                    FoldMap::literal(matched)
                } else {
                    FoldMap::new(matched)
                }
            })
        };
        let (folded, literal) = (map(false), map(true));
        let ranges = places.iter().filter_map(|(place, exact)| {
            let map = if *exact { &literal } else { &folded };
            Some(map.as_ref()?.range(place.from, place.to))
        });
        let ranges = merge(ranges.collect());
        Some(Hit {
            doc: found.doc,
            cell: entry.id,
            field,
            snippet: snippet(matched, &ranges, SNIPPET_WIDTH),
            matched: matched.to_owned(),
            ranges,
            text: entry.text.to_string(),
            shown: entry.shown().map(str::to_owned),
            trail: search.trail(found.slot),
            score: found.score,
            depth: search.depth(found.slot),
            last_edit: entry.last_edit,
        })
    }
}

/// The kind of a match at the bytes `from..to` of `text`, in a field that is the bytes
/// `start..end`: 0 the whole field, 1 a whole word, 2 the start of a word, 3 inside one.
fn kind_of(text: &str, from: usize, to: usize, start: usize, end: usize) -> u32 {
    if from == start && to == end {
        0
    } else if !word_starts(text, from) {
        3
    } else if word_ends(text, to) {
        1
    } else {
        2
    }
}

/// Finds `needle` in one field, from left to right, each match starting after the one
/// before it ends.
fn find_in(text: &str, needle: &str, field: Field, sub: u32, each: &mut dyn FnMut(Occurrence)) {
    let mut at = 0usize;
    while let Some(found) = text.get(at..).and_then(|rest| rest.find(needle)) {
        let from = at + found;
        let to = from + needle.len();
        each(Occurrence {
            field,
            sub,
            from,
            to,
            kind: kind_of(text, from, to, 0, text.len()),
        });
        at = to;
    }
}

const UNKNOWN: u32 = u32::MAX;
const NO_MATCH: u32 = u32::MAX - 1;

/// A search at work in one document.
struct InDoc<'a> {
    run: &'a Run<'a>,
    id: DocId,
    search: &'a DocSearch,
    /// For each percentage asked about: the edit time from which a cell is recent.
    recent: Vec<(u8, i64)>,
    /// For the cell at hand: the points of each needle, once known.
    memo: Vec<u32>,
}

impl InDoc<'_> {
    /// The points of a needle in the cell, or [`NO_MATCH`].
    fn needle(&mut self, n: usize, entry: &Entry) -> u32 {
        if let Some(known) = self.memo.get(n).copied().filter(|known| *known != UNKNOWN) {
            return known;
        }
        let Some(needle) = self.run.needles.get(n) else {
            return NO_MATCH;
        };
        let best = self.run.best(self.id, entry, needle);
        if let Some(memo) = self.memo.get_mut(n) {
            *memo = best;
        }
        best
    }

    fn eval(&mut self, node: &Node, slot: u32, entry: &Entry) -> bool {
        match node {
            Node::All => true,
            Node::Term(n) => self.needle(*n, entry) != NO_MATCH,
            Node::Test(test) => self.test(test, slot, entry),
            Node::And(list) => list.iter().all(|node| self.eval(node, slot, entry)),
            Node::Or(list) => list.iter().any(|node| self.eval(node, slot, entry)),
            Node::Not(inner) => !self.eval(inner, slot, entry),
        }
    }

    /// The points of the cell: those of every needle that counts, added up.
    fn score(&mut self, entry: &Entry) -> u32 {
        let mut score = 0u32;
        for n in 0..self.run.needles.len() {
            if self
                .run
                .needles
                .get(n)
                .is_some_and(|needle| needle.positive)
            {
                let points = self.needle(n, entry);
                score = score.saturating_add(if points == NO_MATCH { ABSENT } else { points });
            }
        }
        score
    }

    fn test(&mut self, test: &Test, slot: u32, entry: &Entry) -> bool {
        match test {
            Test::Tag(name) => *entry.text == **name && self.search.has_tag(name),
            Test::Has(has) => match has {
                Has::Text => !entry.text.is_empty(),
                Has::Note => !entry.note().is_empty(),
                Has::Image => entry.image().is_some(),
                Has::Grid => entry.has_grid,
                Has::Link => entry.has_links,
                Has::Tag => self.search.has_tag(&entry.text),
                // Made into a set of cells before the search starts.
                Has::BrokenLink => false,
            },
            Test::OneOf(cells) => cells.contains(&(self.id, entry.id)),
            Test::Edited(from, to) => entry.last_edit.is_some_and(|time| {
                from.is_none_or(|from| time >= from) && to.is_none_or(|to| time <= to)
            }),
            Test::Recent(percent) => {
                let from = self.recent_from(*percent);
                entry.last_edit.unwrap_or(i64::MIN) >= from
            }
            Test::Docs(docs) => docs.contains(&self.id),
            Test::Within(cell) => self
                .search
                .slot(*cell)
                .is_some_and(|branch| self.search.is_within(slot, branch)),
            Test::Kind(kind) => entry.kind == *kind,
            Test::CellColor(color) => entry.cell_color == *color,
            Test::TextColor(color) => entry.text_color == *color,
            Test::Style(style) => entry.style == *style,
            Test::TextIs(text) => *entry.text == **text,
            Test::Image(name) => entry.image() == Some(name.as_str()),
        }
    }

    /// The edit time from which a cell is among the most recent `percent` of its
    /// document, as TreeSheets counts them: the cells sorted by edit time, latest
    /// first, up to and including the one at (number of cells x percent / 100); cells
    /// edited at the same time as that one are in as well.
    fn recent_from(&mut self, percent: u8) -> i64 {
        if let Some((_, from)) = self.recent.iter().find(|(p, _)| *p == percent) {
            return *from;
        }
        let live = self.search.entries.iter().filter(|entry| entry.live);
        let mut times: Vec<i64> = live.map(|e| e.last_edit.unwrap_or(i64::MIN)).collect();
        times.sort_unstable_by(|a, b| b.cmp(a));
        let last = times.len().saturating_mul(usize::from(percent)) / 100;
        let from = times
            .get(last)
            .or(times.last())
            .copied()
            .unwrap_or(i64::MIN);
        self.recent.push((percent, from));
        from
    }
}
