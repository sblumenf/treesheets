//! Shared by the tests that hold the search index to the reference scanner: queries made
//! from whatever the documents contain, the comparison itself, and a follower for the
//! random runs of `wombat-core/tests/support/links.rs`.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use wombat_core::{CellId, CellKind, Changes, DocId, Document, Editor, LinkTarget};
use wombat_search::query::{Filter, Has, Order, Query, Term};
use wombat_search::{naive, Options, SearchIndex};

/// A small random number generator: the tests need choices that depend on the documents
/// as they are at each step, which a fixed list of generated values cannot give.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        (!items.is_empty()).then(|| &items[self.below(items.len())])
    }
}

pub const ORDERS: [Order; 4] = [
    Order::Relevance,
    Order::RelevanceThenRecent,
    Order::Recent,
    Order::Document,
];

/// A piece of some text of some cell: of its text, note, image name or link target,
/// sometimes in another case, sometimes a single character, sometimes absent everywhere.
fn word(rng: &mut Rng, docs: &[&Document]) -> Term {
    let doc = docs[rng.below(docs.len())];
    let cells = doc.cells_in_order();
    let mut source = String::new();
    for _ in 0..6 {
        let cell = doc.cell(cells[rng.below(cells.len())]).unwrap();
        source = match rng.below(8) {
            0 => cell.note.clone(),
            1 => cell
                .image
                .as_ref()
                .map(|i| i.file.clone())
                .unwrap_or_default(),
            2 => cell
                .links
                .iter()
                .find_map(|link| match &link.target {
                    LinkTarget::External(target) => Some(target.clone()),
                    _ => None,
                })
                .unwrap_or_default(),
            _ => cell.text.clone(),
        };
        if !source.is_empty() {
            break;
        }
    }
    let chars: Vec<char> = source.chars().collect();
    let mut text: String = match (chars.len(), rng.below(10)) {
        (0, _) | (_, 0) => ["zz", "a", "é", "日", "x y", "E", ""][rng.below(7)].to_owned(),
        (len, 1) => chars[rng.below(len)].to_string(),
        (_, 2) => source.clone(),
        (len, _) => {
            let from = rng.below(len);
            let to = (from + 1 + rng.below(4)).min(len);
            chars[from..to].iter().collect()
        }
    };
    match rng.below(6) {
        0 => text = text.to_uppercase(),
        1 => text = text.to_lowercase(),
        _ => {}
    }
    Term {
        text,
        exact: rng.below(5) == 0,
    }
}

fn filter(rng: &mut Rng, docs: &[&Document]) -> Filter {
    let doc = docs[rng.below(docs.len())];
    let cells = doc.cells_in_order();
    let id = cells[rng.below(cells.len())];
    let cell = doc.cell(id).unwrap();
    let tags: Vec<&String> = doc.tags().keys().collect();
    let gone = CellId::parse("00000000-0000-0000-0000-00000000dead").unwrap();
    match rng.below(24) {
        0 => Filter::Tag(
            rng.pick(&tags)
                .map_or(cell.text.clone(), |tag| (*tag).clone()),
        ),
        1 => Filter::Has(Has::Tag),
        2 => Filter::Has(Has::Note),
        3 => Filter::Has(Has::Image),
        4 => Filter::Has(Has::Grid),
        5 => Filter::Has(Has::Link),
        6 => Filter::Has(Has::BrokenLink),
        7 => Filter::Has(Has::Text),
        8 => Filter::LinksTo(id),
        9 => Filter::LinkedFrom(id),
        10 => {
            let around = cell.last_edit.unwrap_or(1);
            let from = (rng.below(3) != 0).then(|| around - rng.below(3) as i64);
            let to = (rng.below(3) != 0).then(|| around + rng.below(3) as i64);
            Filter::Edited { from, to }
        }
        11 => Filter::Recent([1, 5, 10, 20, 50, 99][rng.below(6)]),
        12 => Filter::Doc(doc.id()),
        13 => Filter::DocName(["d0", "D1.wombat", "d2.WOMBAT", "nothing"][rng.below(4)].into()),
        14 | 15 => Filter::Within(if rng.below(8) == 0 { gone } else { id }),
        16 => Filter::Kind(CellKind::ALL[rng.below(CellKind::ALL.len())]),
        17 => Filter::CellColor(cell.cell_color),
        18 => Filter::TextColor(cell.text_color),
        19 => Filter::Style(cell.style),
        20 => Filter::TextIs(cell.text.clone()),
        21 => Filter::Image(
            cell.image
                .as_ref()
                .map_or("i1.png".into(), |i| i.file.clone()),
        ),
        22 => Filter::LinksTo(gone),
        _ => Filter::Doc(DocId::parse("00000000-0000-0000-0000-00000000d0c0").unwrap()),
    }
}

/// A random query about what the documents contain.
pub fn query(rng: &mut Rng, docs: &[&Document], depth: usize) -> Query {
    let part = |rng: &mut Rng| query(rng, docs, depth + 1);
    match rng.below(if depth >= 2 { 9 } else { 16 }) {
        0..=4 => Query::Term(word(rng, docs)),
        5..=7 => Query::Filter(filter(rng, docs)),
        8 => Query::All,
        9..=11 => Query::And((0..2 + rng.below(2)).map(|_| part(rng)).collect()),
        12 | 13 => Query::Or((0..2 + rng.below(2)).map(|_| part(rng)).collect()),
        14 => Query::Not(Box::new(part(rng))),
        _ => Query::And(vec![
            Query::Term(word(rng, docs)),
            Query::Filter(filter(rng, docs)),
        ]),
    }
}

/// How much was compared, so that a test can tell that it compared enough.
#[derive(Clone, Copy, Debug, Default)]
pub struct Compared {
    pub queries: usize,
    /// Queries that some cell matched, and that some cell did not.
    pub selective: usize,
    pub hits: usize,
    pub highlighted: usize,
    pub by_linked_text: usize,
}

impl Compared {
    pub fn add(&mut self, other: Compared) {
        self.queries += other.queries;
        self.selective += other.selective;
        self.hits += other.hits;
        self.highlighted += other.highlighted;
        self.by_linked_text += other.by_linked_text;
    }
}

/// The heart of the milestone: for one query, the index gives exactly what reading every
/// cell gives: the same cells in the same order with the same fields, ranges, snippets,
/// trails and points; also page by page, and for "next match".
pub fn compare(
    indexes: &[&SearchIndex],
    reference: &naive::Reference,
    docs: &[(&str, &Document)],
    query: &Query,
    rng: &mut Rng,
    what: &str,
    stats: &mut Compared,
) {
    let order = ORDERS[rng.below(4)];
    let cells: usize = docs.iter().map(|(_, doc)| doc.cell_count()).sum();
    // All results where there are few cells; of many, mostly a stretch somewhere in
    // the list (making and checking a hundred thousand hits takes long).
    let mut options = Options::all(order);
    if cells > 400 && rng.below(6) != 0 {
        options.offset = [0, 0, rng.below(cells), rng.below(50)][rng.below(4)];
        options.limit = 40;
    }
    let expected = reference.search(query, &options).unwrap();
    stats.queries += 1;
    stats.selective += usize::from(expected.total > 0 && expected.total < cells);
    stats.hits += expected.total;
    for hit in &expected.hits {
        stats.highlighted += usize::from(!hit.ranges.is_empty());
        stats.by_linked_text += usize::from(hit.field == wombat_search::Field::LinkedText);
        check_hit(hit, docs);
    }
    // A page of the list, and the next and previous match from some cell.
    let skip = rng.below(expected.hits.len() + 2);
    let offset = options.offset + skip;
    // (Not past the end of the stretch that was asked for above.)
    let limit = rng.below(4).min(options.limit.saturating_sub(skip));
    let page = Options {
        order,
        offset,
        limit,
    };
    let (_, doc) = docs[rng.below(docs.len())];
    let cells = doc.cells_in_order();
    let from = (rng.below(5) != 0).then(|| cells[rng.below(cells.len())]);
    for index in indexes {
        let found = index.search(query, &options).unwrap();
        if found != expected {
            assert_eq!(found.total, expected.total, "{query:?} after {what}");
            for (a, b) in found.hits.iter().zip(&expected.hits) {
                assert_eq!(a, b, "{query:?} ({order:?}) after {what}");
            }
            panic!("{query:?} after {what}: other results");
        }
        let part = index.search(query, &page).unwrap();
        assert_eq!(part.total, expected.total);
        let whole: Vec<_> = expected.hits.iter().skip(skip).take(limit).collect();
        assert_eq!(
            part.hits.iter().collect::<Vec<_>>(),
            whole,
            "{query:?} page"
        );
        if order != Order::Document || options.limit != usize::MAX {
            continue;
        }
        // The results are in the order of the documents: the next match after a cell
        // is the first one further on in its document, or else the first of all.
        let place = |cell: CellId| cells.iter().position(|id| *id == cell);
        let here = from.and_then(place);
        let mut in_doc: Vec<_> = expected
            .hits
            .iter()
            .filter(|hit| hit.doc == doc.id())
            .collect();
        for reverse in [false, true] {
            let next = index.next_match(query, doc.id(), from, reverse).unwrap();
            let further = |hit: &&&wombat_search::Hit| match (here, place(hit.cell)) {
                (Some(here), Some(at)) if reverse => at < here,
                (Some(here), Some(at)) => at > here,
                _ => false,
            };
            let expected = in_doc.iter().find(further).or(in_doc.first()).copied();
            assert_eq!(
                next.as_ref(),
                expected,
                "{query:?} next from {from:?} after {what}"
            );
            in_doc.reverse();
        }
    }
}

/// What must hold for any hit: its ranges are whole user-perceived characters of the
/// matched text, in order and apart; the snippet is a part of it with the same marks.
pub fn check_hit(hit: &wombat_search::Hit, docs: &[(&str, &Document)]) {
    let doc = docs.iter().find(|(_, d)| d.id() == hit.doc).unwrap().1;
    let cell = doc.cell(hit.cell).unwrap();
    assert_eq!(hit.text, cell.text);
    let place = wombat_core::links::place(doc, hit.cell).unwrap();
    assert_eq!(hit.trail, place.trail);
    let len = hit.matched.chars().count() as u32;
    let mut last_end = 0;
    for (n, (start, count)) in hit.ranges.iter().enumerate() {
        assert!(*count > 0 && start + count <= len, "{hit:?}");
        assert!(
            n == 0 || *start > last_end,
            "ranges touch or overlap: {hit:?}"
        );
        last_end = start + count;
        for edge in [*start, start + count] {
            assert_eq!(
                wombat_core::text::is_boundary(&hit.matched, edge),
                Some(true),
                "a range cuts a character: {hit:?}"
            );
        }
    }
    let snippet = &hit.snippet;
    let shown = snippet.text.chars().count() as u32;
    for (start, count) in &snippet.ranges {
        assert!(*count > 0 && start + count <= shown, "{hit:?}");
        for edge in [*start, start + count] {
            let whole = wombat_core::text::is_boundary(&snippet.text, edge);
            assert_eq!(whole, Some(true), "{hit:?}");
        }
    }
    if !snippet.cut_start && !snippet.cut_end {
        assert_eq!(snippet.ranges, hit.ranges);
    }
}

/// Follows the random runs of the link tests with a search index, and after every step
/// holds it to an index built from scratch and to the reference scanner.
pub struct Follower {
    pub index: SearchIndex,
    pub rng: Rng,
    /// How many queries to compare after each step.
    pub queries: usize,
    pub stats: Compared,
    names: Vec<(DocId, String)>,
}

impl Follower {
    pub fn new(docs: &[Document], seed: u64, queries: usize) -> Follower {
        let mut index = SearchIndex::new();
        let mut names = Vec::new();
        for (n, doc) in docs.iter().enumerate() {
            // Names not in the order of the IDs, and two that differ only in case.
            let name = ["d1.wombat", "D0.wombat", "d2.wombat", "d0.wombat"][n % 4].to_owned();
            index.insert_document(doc, &name);
            names.push((doc.id(), name));
        }
        Follower {
            index,
            rng: Rng(seed | 1),
            queries,
            stats: Compared::default(),
            names,
        }
    }

    /// Makes the index build its table again early, so that small runs get there.
    pub fn compact_early(mut self) -> Follower {
        let mut index = SearchIndex::new();
        for (doc, name) in self.index.documents() {
            let mut doc = doc.clone();
            doc.set_thresholds(8, 2);
            let links = self.index.links().document(doc.id()).unwrap().clone();
            index.insert(wombat_search::DocIndex { search: doc, links }, name);
        }
        self.index = index;
        self
    }

    pub fn changed(&mut self, doc: &Document, changes: &Changes) {
        self.index.apply(doc, changes);
    }

    pub fn step(&mut self, editors: &[Editor], what: &str) {
        let name = |id: DocId| {
            self.names
                .iter()
                .find(|(d, _)| *d == id)
                .unwrap()
                .1
                .as_str()
        };
        let docs: Vec<(&str, &Document)> = editors
            .iter()
            .map(|editor| (name(editor.document().id()), editor.document()))
            .collect();
        self.index
            .check()
            .unwrap_or_else(|error| panic!("{error} after {what}"));
        let mut rebuilt = SearchIndex::new();
        for (name, doc) in &docs {
            rebuilt.insert_document(doc, name);
        }
        assert!(
            self.index.encoded() == rebuilt.encoded(),
            "the index drifted after {what}"
        );
        let plain: Vec<&Document> = docs.iter().map(|(_, doc)| *doc).collect();
        // The index that followed the edits, and one built from scratch (whose table
        // has nothing stale): both say what reading every cell says.
        let reference = naive::Reference::new(&docs);
        for _ in 0..self.queries {
            let query = query(&mut self.rng, &plain, 0);
            let both = [&self.index, &rebuilt];
            let (rng, stats) = (&mut self.rng, &mut self.stats);
            compare(&both, &reference, &docs, &query, rng, what, stats);
        }
    }
}
