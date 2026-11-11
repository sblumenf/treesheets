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

use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet};

use aho_corasick::AhoCorasick;
use memchr::memmem::Finder;

use unicode_segmentation::UnicodeSegmentation;
use wombat_core::eval::Calc;
use wombat_core::links::{DocLinks, Resolved};
use wombat_core::summary::Summary;
use wombat_core::{CellId, Changes, DocId, Document, LinkIndex, LinkTarget};

use crate::fold::{fold, word_ends, word_starts, FoldMap};
use crate::index::{
    bit, pair_key, set_bit, single_key, DocIndex, DocSearch, Entry, FIELD_IMAGE, FIELD_NOTE,
    FIELD_SHOWN, FIELD_TEXT, FIELD_URLS, NONE,
};
use crate::query::{Filter, Has, Order, Query, QueryError};

/// The points of a word that is not in the cell (see the ranking rule).
pub const ABSENT: u32 = 32;
/// How many user-perceived characters a snippet shows at most.
pub const SNIPPET_WIDTH: usize = 80;
/// How many highlight ranges a hit carries at most: the first ones.
/// [`Hit::range_count`] says how many there are. (One long cell can have a hundred
/// thousand matches, which no interface can show.)
pub const MAX_RANGES: usize = 1000;
/// Up to this many cells that may have a word, the cells that link to them are looked
/// up one by one; above it, every cell with links is looked at.
const FEW: usize = 2048;

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
    /// in order, never overlapping, never cutting a user-perceived character. At most
    /// the first [`MAX_RANGES`] of them.
    pub ranges: Vec<(u32, u32)>,
    /// How many such ranges there are in all.
    pub range_count: usize,
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
    /// What is odd about the query without being wrong ([`Query::warnings`]): a word
    /// with nothing to search for, which matches no cell.
    pub warnings: Vec<String>,
}

/// [`SearchIndex::apply`] was given a document that the index does not have.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NotIndexed(pub DocId);

impl std::fmt::Display for NotIndexed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the search index does not have the document {}", self.0)
    }
}

impl std::error::Error for NotIndexed {}

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
    /// does not have is an error and nothing is done: it has no name to be found
    /// under, so it must be put in with [`SearchIndex::insert_document`] first. (It
    /// used to be read whole under an empty name, which hid the mistake.)
    pub fn apply(
        &mut self,
        doc: &Document,
        calc: &Calc,
        changes: &Changes,
    ) -> Result<(), NotIndexed> {
        let held = self.docs.get_mut(&doc.id()).ok_or(NotIndexed(doc.id()))?;
        held.search.update(doc, calc, changes);
        self.links.apply(doc, changes);
        Ok(())
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
        let end = options.offset.saturating_add(options.limit);
        let (total, mut found) = run.matches(options.order, None, end);
        if end < found.len() && end > 0 {
            // Only the part asked for needs sorting.
            found.select_nth_unstable(end - 1);
            found.truncate(end);
        }
        found.sort_unstable();
        let page = found.iter().skip(options.offset).take(options.limit);
        let hits = page
            .filter_map(|found| run.hit(found, options.order))
            .collect();
        Ok(Results {
            total,
            hits,
            warnings: query.warnings(),
        })
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
        let (_, mut found) = run.matches(Order::Document, Some(doc), usize::MAX);
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
        Ok(next.and_then(|found| run.hit(found, Order::Document)))
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
    /// The same, ready to be searched for: made once for all cells.
    finder: Finder<'static>,
    exact: bool,
    /// How many times it is in the query outside a negation: so many times its points
    /// count. 0 for a word that is only negated, which neither counts nor is
    /// highlighted.
    weight: u32,
    /// One bit per cell of the index ([`Run::spans`]): whether the cell's own fields
    /// may have the word. `None` when nothing is known: any cell may.
    own: Option<Vec<u64>>,
    /// The same with the cells that may have it by the text of a cell they link to.
    any: Option<Vec<u64>>,
}

impl Needle {
    /// Calls `each` with where each match starts in `hay`, from left to right, each
    /// match starting after the one before it ends, until `each` says to stop.
    fn each_match(&self, hay: &[u8], mut each: impl FnMut(usize) -> bool) {
        if hay.len() < self.text.len() {
            return;
        }
        if let [byte] = self.text.as_bytes() {
            for from in memchr::memchr_iter(*byte, hay) {
                if !each(from) {
                    return;
                }
            }
            return;
        }
        let mut at = 0usize;
        while let Some(found) = hay.get(at..).and_then(|rest| self.finder.find(rest)) {
            if !each(at + found) {
                return;
            }
            at += found + self.text.len();
        }
    }
}

/// A filter, ready to be tested.
enum Test {
    /// With the cells that may be the tag, by the table, as for a word.
    Tag(String, Option<Vec<u64>>),
    RowTag(String),
    /// The folded form of what the formula must contain.
    Formula(String),
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
    /// With the cells that may show the text, by the table.
    TextIs(String, Option<Vec<u64>>),
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
    /// Where each document's cells are in the bit sets of this search: the first of
    /// its 64-bit words (bit `slot` from there is the cell in that slot).
    spans: BTreeMap<DocId, usize>,
    /// How many words a bit set of all cells has.
    words: usize,
    /// Where the query has many words: all of them that are compared in folded form,
    /// to be found in one reading of a cell ([`InDoc::at_once`]), and which needle each
    /// is. One reading costs more than looking for one word and less than looking for
    /// four, whatever their number.
    many: Option<(AhoCorasick, Vec<usize>)>,
}

/// From this many words on, a cell is read once for all of them.
const MANY: usize = 4;

/// The keys of the table that a cell must have to have the folded text `folded`: its
/// pairs of neighbouring characters, or its one character.
fn keys_of(folded: &str) -> Vec<u64> {
    let mut keys = Vec::new();
    let mut before = None;
    for c in folded.chars() {
        if let Some(before) = before {
            keys.push(pair_key(before, c));
        }
        before = Some(c);
    }
    if keys.is_empty() {
        keys.extend(before.map(single_key));
    }
    keys
}

/// The same for a word that must be found exactly, as far as anything is certain: a
/// plain letter or digit is in the folded text as its lower-case form whatever is
/// around it, and two of them side by side stay side by side.
fn keys_of_exact(text: &str) -> Vec<u64> {
    let plain = |c: &char| c.is_ascii_alphanumeric();
    let mut keys = Vec::new();
    let mut before: Option<char> = None;
    let mut single = None;
    for c in text.chars() {
        let lower = c.to_ascii_lowercase();
        if plain(&c) {
            single = Some(single_key(lower));
            keys.extend(before.map(|before| pair_key(before, lower)));
        }
        before = plain(&c).then_some(lower);
    }
    if keys.is_empty() {
        keys.extend(single);
    }
    keys
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
        let mut spans = BTreeMap::new();
        let mut words = 0usize;
        for (id, held) in &index.docs {
            spans.insert(*id, words);
            words += (held.search.slot_count() as usize).div_ceil(64);
        }
        let mut run = Run {
            index,
            node: Node::All,
            needles: Vec::new(),
            ranks,
            spans,
            words,
            many: None,
        };
        run.node = run.compile(query, true);
        let folded = |needle: &&Needle| !needle.exact && !needle.text.is_empty();
        let which: Vec<usize> = (0..run.needles.len())
            .filter(|n| run.needles.get(*n).is_some_and(|needle| folded(&needle)))
            .collect();
        if which.len() >= MANY {
            let texts = which.iter().filter_map(|n| run.needles.get(*n));
            let texts: Vec<&[u8]> = texts.map(|needle| needle.text.as_bytes()).collect();
            // Few and short words: the form that is fastest to read with, and here
            // small to make.
            let short = texts.iter().map(|text| text.len()).sum::<usize>() <= 512;
            let kind = short.then_some(aho_corasick::AhoCorasickKind::DFA);
            let built = AhoCorasick::builder().kind(kind).build(texts);
            run.many = built.ok().map(|built| (built, which));
        }
        run
    }

    /// One bit per cell: the cells that may have all of `keys` in their own fields.
    fn having(&self, keys: &[u64]) -> Option<Vec<u64>> {
        if keys.is_empty() {
            return None;
        }
        let mut bits = vec![0u64; self.words];
        for (id, held) in &self.index.docs {
            let from = self.spans.get(id).copied().unwrap_or(0);
            let to = from + (held.search.slot_count() as usize).div_ceil(64);
            if let Some(out) = bits.get_mut(from..to) {
                held.search.candidates(keys, out);
            }
        }
        Some(bits)
    }

    /// `own` with the cells that link to one of its cells: a cell also matches a word
    /// by what a cell shows that it links to.
    fn with_linking(&self, own: &[u64]) -> Vec<u64> {
        let mut any = own.to_vec();
        if !self.index.links.has_cell_links() {
            return any;
        }
        let count: usize = own.iter().map(|word| word.count_ones() as usize).sum();
        for (id, held) in &self.index.docs {
            let from = self.spans.get(id).copied().unwrap_or(0);
            if count > FEW {
                // Too many to look up: any cell with links may link to one of them.
                let linking = held.search.linking();
                for (word, links) in any.iter_mut().skip(from).zip(linking) {
                    *word |= links;
                }
                continue;
            }
            let words = (held.search.slot_count() as usize).div_ceil(64);
            for (n, word) in own.iter().enumerate().skip(from).take(words) {
                let mut word = *word;
                while word != 0 {
                    let slot = (n - from) * 64 + word.trailing_zeros() as usize;
                    word &= word - 1;
                    let Some(entry) = held.search.entries.get(slot) else {
                        continue;
                    };
                    for (doc, cell) in self.index.links.sources(entry.id) {
                        let slot = self
                            .index
                            .docs
                            .get(&doc)
                            .and_then(|held| held.search.slot(cell));
                        if let (Some(slot), Some(base)) = (slot, self.spans.get(&doc)) {
                            set_bit(&mut any, base * 64 + slot as usize);
                        }
                    }
                }
            }
        }
        any
    }

    fn compile(&mut self, query: &Query, positive: bool) -> Node {
        // What costs least to test comes first: `all` and `any` stop at the first
        // part that decides.
        let cost = |node: &Node| match node {
            Node::All | Node::Test(_) => 0,
            Node::Term(_) => 1,
            Node::And(_) | Node::Or(_) | Node::Not(_) => 2,
        };
        match query {
            Query::All => Node::All,
            Query::Term(term) => {
                let text = if term.exact {
                    term.text.clone()
                } else {
                    fold(&term.text)
                };
                let keys = if term.exact {
                    keys_of_exact(&text)
                } else {
                    keys_of(&text)
                };
                // The same word again is the same needle, counted once more.
                let same = |needle: &Needle| needle.text == text && needle.exact == term.exact;
                if let Some(n) = self.needles.iter().position(same) {
                    if let Some(needle) = self.needles.get_mut(n) {
                        needle.weight += u32::from(positive);
                    }
                    return Node::Term(n);
                }
                let own = self.having(&keys);
                let any = own.as_ref().map(|own| self.with_linking(own));
                self.needles.push(Needle {
                    finder: Finder::new(text.as_bytes()).into_owned(),
                    text,
                    exact: term.exact,
                    weight: u32::from(positive),
                    own,
                    any,
                });
                Node::Term(self.needles.len() - 1)
            }
            Query::Filter(filter) => Node::Test(self.test(filter)),
            Query::And(list) => {
                let mut list: Vec<Node> = list.iter().map(|q| self.compile(q, positive)).collect();
                list.sort_by_key(cost);
                Node::And(list)
            }
            Query::Or(list) => {
                let mut list: Vec<Node> = list.iter().map(|q| self.compile(q, positive)).collect();
                list.sort_by_key(cost);
                Node::Or(list)
            }
            Query::Not(inner) => Node::Not(Box::new(self.compile(inner, false))),
        }
    }

    /// One bit per cell for a set of cells.
    fn bits_of(&self, cells: &HashSet<(DocId, CellId)>) -> Vec<u64> {
        let mut bits = vec![0u64; self.words];
        for (doc, cell) in cells {
            let slot = self
                .index
                .docs
                .get(doc)
                .and_then(|held| held.search.slot(*cell));
            if let (Some(slot), Some(base)) = (slot, self.spans.get(doc)) {
                set_bit(&mut bits, base * 64 + slot as usize);
            }
        }
        bits
    }

    fn test(&self, filter: &Filter) -> Test {
        let links = &self.index.links;
        match filter {
            // A cell that shows exactly this has its folded form among its fields.
            Filter::Tag(name) => Test::Tag(name.clone(), self.having(&keys_of(&fold(name)))),
            Filter::RowTag(name) => Test::RowTag(name.clone()),
            Filter::Formula(text) => Test::Formula(fold(text)),
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
            Filter::TextIs(text) => Test::TextIs(text.clone(), self.having(&keys_of(&fold(text)))),
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

    /// The cells that can match `node`, one bit per cell: more than match, perhaps, but
    /// never fewer. `None` for "any cell".
    fn proposed(&self, node: &Node) -> Option<Vec<u64>> {
        match node {
            Node::Term(n) => self.needles.get(*n)?.any.clone(),
            Node::Test(Test::Tag(_, cells) | Test::TextIs(_, cells)) => cells.clone(),
            Node::Test(Test::OneOf(cells)) => Some(self.bits_of(cells)),
            Node::And(list) => {
                let mut sets = list.iter().filter_map(|node| self.proposed(node));
                let mut all = sets.next()?;
                for set in sets {
                    for (word, other) in all.iter_mut().zip(&set) {
                        *word &= other;
                    }
                }
                Some(all)
            }
            Node::Or(list) => {
                let mut all = vec![0u64; self.words];
                for node in list {
                    for (word, other) in all.iter_mut().zip(&self.proposed(node)?) {
                        *word |= other;
                    }
                }
                Some(all)
            }
            Node::All | Node::Test(_) | Node::Not(_) => None,
        }
    }

    /// The cells that match: how many there are, and the first `keep` of them in the
    /// order asked for, not yet sorted (all of them, if there are no more than that);
    /// in one document only, if given. For the order of the documents the points are
    /// not worked out (they do not decide the order): [`Run::hit`] does that for the
    /// results that are returned.
    fn matches(&self, order: Order, only: Option<DocId>, keep: usize) -> (usize, Vec<Found>) {
        let scope = self.scope(&self.node);
        let proposed = self.proposed(&self.node);
        // With a number to keep that is small, the best so far are a heap whose top is
        // the worst of them: most matches are then one comparison and no more.
        let bounded = keep <= 4096;
        let mut total = 0usize;
        let mut found = Vec::new();
        let mut best: BinaryHeap<Found> = BinaryHeap::new();
        for (id, held) in &self.index.docs {
            let out = only.is_some_and(|only| only != *id)
                || scope.as_ref().is_some_and(|scope| !scope.contains(id));
            if out {
                continue;
            }
            let mut doc = self.in_doc(*id, &held.search);
            let rank = self.ranks.get(id).copied().unwrap_or(0);
            let mut one = |doc: &mut InDoc, slot: u32| {
                let Some(entry) = held.search.entry(slot) else {
                    return;
                };
                doc.memo.fill(UNKNOWN);
                doc.scans = 0;
                if !doc.eval(&self.node, slot, entry) {
                    return;
                }
                total += 1;
                let score = match order {
                    Order::Document => 0,
                    _ => doc.score(slot, entry),
                };
                let len = entry.shown_len as usize;
                // (Where the cells are is worked out on a document's first match: a
                // document without one is spared that.)
                let depth = held.search.depth(slot);
                let place = held.search.order();
                let at = place.get(slot as usize).copied().unwrap_or(NONE);
                let one = Found {
                    key: sort_key(order, score, entry.last_edit, len, depth, rank, at),
                    doc: *id,
                    slot,
                    score,
                };
                if !bounded {
                    found.push(one);
                } else if best.len() < keep {
                    best.push(one);
                } else if best.peek().is_some_and(|worst| one < *worst) {
                    if let Some(mut worst) = best.peek_mut() {
                        *worst = one;
                    }
                }
            };
            match &proposed {
                None => {
                    for slot in 0..held.search.entries.len() {
                        one(&mut doc, slot as u32);
                    }
                }
                Some(proposed) => {
                    let words = (held.search.slot_count() as usize).div_ceil(64);
                    let of_doc = proposed.iter().skip(doc.base).take(words);
                    for (n, word) in of_doc.enumerate() {
                        let mut word = *word;
                        while word != 0 {
                            one(&mut doc, (n * 64) as u32 + word.trailing_zeros());
                            word &= word - 1;
                        }
                    }
                }
            }
        }
        if bounded {
            found = best.into_vec();
        }
        (total, found)
    }

    fn in_doc(&'a self, id: DocId, search: &'a DocSearch) -> InDoc<'a> {
        InDoc {
            run: self,
            id,
            base: self.spans.get(&id).copied().unwrap_or(0),
            search,
            recent: Vec::new(),
            tagged: Vec::new(),
            memo: vec![UNKNOWN; self.needles.len()],
            scans: 0,
            state: Vec::new(),
        }
    }

    /// The points of the needle's best match in the cell, or [`NO_MATCH`]: what
    /// [`Run::occurrences`] would give, without looking further than needed. (Most
    /// searches end here for most cells, so this is where the time goes.) `own` is
    /// whether the cell's own fields may have the word at all, by the table.
    fn best(&self, doc: DocId, own: bool, entry: &Entry, needle: &Needle) -> u32 {
        // A word with nothing to search for matches no cell (decision A108).
        if needle.text.is_empty() {
            return NO_MATCH;
        }
        let mut best = NO_MATCH;
        if needle.exact {
            let mut lower = |found: Occurrence| best = best.min(points(found.field, found.kind));
            self.occurrences(doc, own, entry, needle, &mut lower);
            return best;
        }
        // The fields are in the order of their points (text, displayed result, note,
        // image name, link targets), but for the linked texts: so the first field with
        // a match decides, and in it the first match that is a whole word (a match of
        // the whole field is the first one).
        let all = entry.folded();
        let size = needle.text.len();
        let mut first: Option<(usize, usize, usize)> = None;
        needle.each_match(if own { all.as_bytes() } else { &[] }, |from| {
            let to = from + size;
            // Where the field is: worked out once, at the first match.
            let (field_at, start, end) = *first.get_or_insert_with(|| {
                // (A cell with nothing but text is one field.)
                if entry.rare.is_none() {
                    return (FIELD_TEXT, 0, all.len());
                }
                let before = all.get(..from).unwrap_or_default();
                let start = before.rfind('\n').map_or(0, |last| last + 1);
                let rest = all.get(to..).unwrap_or_default();
                let end = to + rest.find('\n').unwrap_or(rest.len());
                let field_at = before.bytes().filter(|byte| *byte == b'\n').count();
                (field_at, start, end)
            });
            if from >= end {
                return false;
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
            kind > 1 && field != Field::Url
        });
        // A link target after the first one that has a match may have a better one.
        if first.is_some_and(|(field_at, _, _)| field_at >= FIELD_URLS) {
            best = NO_MATCH;
            let mut lower = |found: Occurrence| best = best.min(points(found.field, found.kind));
            self.occurrences(doc, true, entry, needle, &mut lower);
            return best;
        }
        if entry.has_links && best > points(Field::LinkedText, 0) {
            let mut lower = |found: Occurrence| best = best.min(points(found.field, found.kind));
            self.linked(doc, entry, needle, &mut lower);
        }
        best
    }

    /// Calls `each` for every place where the needle is found in what the cells show
    /// that the cell's links lead to.
    fn linked(&self, doc: DocId, entry: &Entry, needle: &Needle, each: &mut dyn FnMut(Occurrence)) {
        let links = &self.index.links;
        let list = links
            .document(doc)
            .map_or(&[][..], |d| d.links_of(entry.id));
        for (n, link) in list.iter().enumerate() {
            if !matches!(link.target, LinkTarget::Cell { .. }) {
                continue;
            }
            let Some((target, at)) = self.target(doc, &link.target) else {
                continue;
            };
            // The table says of the cell at the other end, too, whether it may have
            // the word.
            if needle.own.as_ref().is_some_and(|own| !bit(own, at)) {
                continue;
            }
            let text = if needle.exact {
                target.display()
            } else {
                target.folded_display()
            };
            find_in(text, needle, Field::LinkedText, n as u32, each);
        }
    }

    /// Calls `each` for every place where the needle is found in the cell. `own` is
    /// whether the cell's own fields need looking at.
    fn occurrences(
        &self,
        doc: DocId,
        own: bool,
        entry: &Entry,
        needle: &Needle,
        each: &mut dyn FnMut(Occurrence),
    ) {
        if needle.text.is_empty() {
            return;
        }
        if !own {
        } else if needle.exact {
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
                find_in(text.unwrap_or_default(), needle, field, sub, each);
            }
        } else {
            // All fields at once: they are one string, with line breaks between them,
            // and a folded needle has no line break. Where the field at hand ends is
            // looked for once per field, not once per match (which made a cell with
            // many matches cost the square of its length).
            let all = entry.folded();
            let bytes = all.as_bytes();
            let end_from = |start: usize| {
                let rest = bytes.get(start..).unwrap_or_default();
                start + memchr::memchr(b'\n', rest).unwrap_or(rest.len())
            };
            let mut field_at = 0usize;
            let mut field_start = 0usize;
            let mut field_end = end_from(0);
            needle.each_match(bytes, |from| {
                let to = from + needle.text.len();
                while from > field_end {
                    field_at += 1;
                    field_start = field_end + 1;
                    field_end = end_from(field_start);
                }
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
                true
            });
        }
        if entry.has_links {
            self.linked(doc, entry, needle, each);
        }
    }

    /// The cell that a link in `doc` leads to, if it leads to one in the index, and
    /// which bit it is in the bit sets of this search.
    fn target(&self, doc: DocId, target: &LinkTarget) -> Option<(&'a Entry, usize)> {
        let Resolved::Cell { doc, cell, .. } = self.index.links.resolve(doc, target) else {
            return None;
        };
        let search = &self.index.docs.get(&doc)?.search;
        let slot = search.slot(cell)?;
        let base = self.spans.get(&doc).copied()?;
        Some((search.entry(slot)?, base * 64 + slot as usize))
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
                let (target, _) = self.target(doc, &links.get(sub as usize)?.target)?;
                Some(target.display())
            }
        }
    }

    /// Makes the result that is shown for a matching cell.
    fn hit(&self, found: &Found, order: Order) -> Option<Hit> {
        let search = &self.index.docs.get(&found.doc)?.search;
        let entry = search.entry(found.slot)?;
        let positive = || self.needles.iter().filter(|needle| needle.weight > 0);
        // The best field that has a word of the query; then every place in it. (Two
        // passes, so that the places of the other fields are never kept.)
        let mut best: Option<(Field, u32)> = None;
        for needle in positive() {
            self.occurrences(found.doc, true, entry, needle, &mut |place| {
                let at = (place.field, place.sub);
                best = Some(best.map_or(at, |best| best.min(at)));
            });
        }
        let (field, sub) = best.unwrap_or((Field::Text, 0));
        let matched = self
            .field_text(found.doc, entry, field, sub)
            .unwrap_or_default();
        let mut places: Vec<(usize, usize, bool)> = Vec::new();
        for needle in positive().filter(|_| best.is_some()) {
            self.occurrences(found.doc, true, entry, needle, &mut |place| {
                if (place.field, place.sub) == (field, sub) {
                    places.push((place.from, place.to, needle.exact));
                }
            });
        }
        let map = |exact: bool| {
            let wanted = places.iter().any(|(_, _, is)| *is == exact);
            wanted.then(|| {
                if exact {
                    FoldMap::literal(matched)
                } else {
                    FoldMap::new(matched)
                }
            })
        };
        let (folded, literal) = (map(false), map(true));
        let ranges = places.iter().filter_map(|(from, to, exact)| {
            let map = if *exact { &literal } else { &folded };
            Some(map.as_ref()?.range(*from, *to))
        });
        let mut ranges = merge(ranges.collect());
        let range_count = ranges.len();
        ranges.truncate(MAX_RANGES);
        let score = match order {
            // Not worked out when the cells were found: see `matches`.
            Order::Document => {
                let mut doc = self.in_doc(found.doc, search);
                doc.score(found.slot, entry)
            }
            _ => found.score,
        };
        Some(Hit {
            doc: found.doc,
            cell: entry.id,
            field,
            snippet: snippet(matched, &ranges, SNIPPET_WIDTH),
            matched: matched.to_owned(),
            ranges,
            range_count,
            text: entry.text.to_string(),
            shown: entry.shown().map(str::to_owned),
            trail: search.trail(found.slot),
            score,
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

/// Finds the needle in one field, from left to right, each match starting after the one
/// before it ends.
fn find_in(text: &str, needle: &Needle, field: Field, sub: u32, each: &mut dyn FnMut(Occurrence)) {
    needle.each_match(text.as_bytes(), |from| {
        let to = from + needle.text.len();
        each(Occurrence {
            field,
            sub,
            from,
            to,
            kind: kind_of(text, from, to, 0, text.len()),
        });
        true
    });
}

const UNKNOWN: u32 = u32::MAX;
const NO_MATCH: u32 = u32::MAX - 1;

/// A search at work in one document.
struct InDoc<'a> {
    run: &'a Run<'a>,
    id: DocId,
    /// Where the document's cells are in the bit sets of the search ([`Run::spans`]).
    base: usize,
    search: &'a DocSearch,
    /// For each percentage asked about: the edit time from which a cell is recent.
    recent: Vec<(u8, i64)>,
    /// For each tag asked about: the rows that have it, as (slot of the cell whose
    /// grid it is, row), sorted.
    tagged: Vec<(String, Vec<(u32, u32)>)>,
    /// For the cell at hand: the points of each needle, once known.
    memo: Vec<u32>,
    /// For the cell at hand: how many words were looked for one by one.
    scans: usize,
    /// For [`InDoc::at_once`]: per word, its best points and where its last match ends.
    state: Vec<(u32, usize)>,
}

impl InDoc<'_> {
    /// Finds every word of the query in the cell's own fields in one reading, and puts
    /// what [`Run::best`] would give for each where [`InDoc::needle`] looks first.
    fn at_once(&mut self, entry: &Entry) {
        let Some((many, which)) = &self.run.many else {
            return;
        };
        self.state.clear();
        self.state.resize(which.len(), (NO_MATCH, 0));
        let all = entry.folded();
        let bytes = all.as_bytes();
        let end_from = |start: usize| {
            let rest = bytes.get(start..).unwrap_or_default();
            start + memchr::memchr(b'\n', rest).unwrap_or(rest.len())
        };
        let (mut field_at, mut field_start, mut field_end) = (0usize, 0usize, end_from(0));
        // Every place of every word, in the order of where they end. Of one word the
        // places are taken from left to right, each starting after the one before it
        // ends, as when it is looked for alone.
        for found in many.find_overlapping_iter(bytes) {
            let (from, to) = (found.start(), found.end());
            let Some(state) = self.state.get_mut(found.pattern().as_usize()) else {
                continue;
            };
            if from < state.1 {
                continue;
            }
            state.1 = to;
            while from > field_end {
                field_at += 1;
                field_start = field_end + 1;
                field_end = end_from(field_start);
            }
            let field = match field_at {
                FIELD_TEXT => Field::Text,
                FIELD_SHOWN => Field::Shown,
                FIELD_NOTE => Field::Note,
                FIELD_IMAGE => Field::Image,
                _ => Field::Url,
            };
            // (An earlier field, a whole word or the whole field: nothing better comes.)
            if state.0 > points(field, 1) {
                let kind = kind_of(all, from, to, field_start, field_end);
                state.0 = state.0.min(points(field, kind));
            }
        }
        for (state, n) in self.state.iter().zip(which) {
            let mut best = state.0;
            let needle = self.run.needles.get(*n);
            if let Some(needle) = needle.filter(|_| entry.has_links) {
                if best > points(Field::LinkedText, 0) {
                    let mut lower = |found: Occurrence| {
                        best = best.min(points(found.field, found.kind));
                    };
                    self.run.linked(self.id, entry, needle, &mut lower);
                }
            }
            if let Some(memo) = self.memo.get_mut(*n) {
                *memo = best;
            }
        }
    }

    /// The points of a needle in the cell, or [`NO_MATCH`].
    fn needle(&mut self, n: usize, slot: u32, entry: &Entry) -> u32 {
        if let Some(known) = self.memo.get(n).copied().filter(|known| *known != UNKNOWN) {
            return known;
        }
        let Some(needle) = self.run.needles.get(n) else {
            return NO_MATCH;
        };
        // What the table rules out is not looked at.
        let at = self.base * 64 + slot as usize;
        let own = needle.own.as_ref().is_none_or(|own| bit(own, at));
        // The third word that needs the cell's text read: read it once for all.
        if own && !needle.exact && self.run.many.is_some() {
            self.scans += 1;
            if self.scans > 2 {
                self.at_once(entry);
                // (A word that is not among those read at once is still unknown.)
                let known = self.memo.get(n).copied().filter(|known| *known != UNKNOWN);
                if let Some(known) = known {
                    return known;
                }
            }
        }
        let best = self.run.best(self.id, own, entry, needle);
        if let Some(memo) = self.memo.get_mut(n) {
            *memo = best;
        }
        best
    }

    fn eval(&mut self, node: &Node, slot: u32, entry: &Entry) -> bool {
        match node {
            Node::All => true,
            Node::Term(n) => self.needle(*n, slot, entry) != NO_MATCH,
            Node::Test(test) => self.test(test, slot, entry),
            Node::And(list) => list.iter().all(|node| self.eval(node, slot, entry)),
            Node::Or(list) => list.iter().any(|node| self.eval(node, slot, entry)),
            Node::Not(inner) => !self.eval(inner, slot, entry),
        }
    }

    /// The points of the cell: those of every needle that counts, added up.
    fn score(&mut self, slot: u32, entry: &Entry) -> u32 {
        let mut score = 0u32;
        for n in 0..self.run.needles.len() {
            let weight = self.run.needles.get(n).map_or(0, |needle| needle.weight);
            if weight > 0 {
                let points = self.needle(n, slot, entry);
                let points = if points == NO_MATCH { ABSENT } else { points };
                score = score.saturating_add(points.saturating_mul(weight));
            }
        }
        score
    }

    fn test(&mut self, test: &Test, slot: u32, entry: &Entry) -> bool {
        match test {
            // What a cell is to the eye goes by what it shows: of a formula cell the
            // result (decision A110).
            Test::Tag(name, _) => entry.display() == name && self.search.has_tag(name),
            Test::RowTag(name) => self.in_tagged_row(name, entry),
            Test::Formula(text) => {
                entry.kind == wombat_core::CellKind::Formula
                    && !text.is_empty()
                    && entry.folded_text().contains(text.as_str())
            }
            Test::Has(has) => match has {
                Has::Text => !entry.display().is_empty(),
                Has::Note => !entry.note().is_empty(),
                Has::Image => entry.image().is_some(),
                Has::Grid => entry.has_grid,
                Has::Link => entry.has_links,
                Has::Tag => self.search.has_tag(entry.display()),
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
            Test::TextIs(text, _) => entry.display() == text,
            Test::Image(name) => entry.image() == Some(name.as_str()),
        }
    }

    /// Whether the cell is in a row that has the tag `name`: see [`Filter::RowTag`].
    fn in_tagged_row(&mut self, name: &str, entry: &Entry) -> bool {
        let search = self.search;
        let row = |entry: &Entry| {
            let columns = search.entries.get(entry.parent as usize);
            (
                entry.parent,
                entry.pos / columns.map_or(1, |e| e.cols.max(1)),
            )
        };
        let known = self.tagged.iter().position(|(tag, _)| tag == name);
        let at = known.unwrap_or_else(|| {
            let mut rows: Vec<(u32, u32)> = Vec::new();
            if search.has_tag(name) {
                let live = search.entries.iter().filter(|entry| entry.live);
                rows.extend(live.filter(|e| e.display() == name).map(row));
                rows.sort_unstable();
                rows.dedup();
            }
            self.tagged.push((name.to_owned(), rows));
            self.tagged.len() - 1
        });
        let rows = self.tagged.get(at).map_or(&[][..], |(_, rows)| rows);
        rows.binary_search(&row(entry)).is_ok()
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
