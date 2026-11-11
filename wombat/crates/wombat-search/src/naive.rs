//! The reference: the same answers as [`crate::engine`], found by reading every cell of
//! every document with the plainest code that could be right. It keeps nothing between
//! searches and uses no table, no shortcut and nothing of the index; its cost is that of
//! all the text for every search. The tests hold the index to it.
//!
//! What it shares with the index is the definition of the things themselves: the folded
//! form of text ([`crate::fold::fold`]), what of a cell is searched
//! ([`crate::index::searchable`]), which characters make words, how a snippet is cut,
//! the link index of milestone 4 for what a link leads to, and the results of formula
//! cells, which it is given ([`Calc`]) and reads cell by cell.

use unicode_segmentation::UnicodeSegmentation;
use wombat_core::eval::Calc;
use wombat_core::links::{self, Resolved};
use wombat_core::{Cell, CellId, CellKind, DocId, Document, LinkIndex, LinkTarget};

use crate::engine::{snippet, Field, Hit, Options, Results, ABSENT, SNIPPET_WIDTH};
use crate::fold::{fold, is_word_char, is_wordless};
use crate::index::searchable;
use crate::query::{Filter, Has, Order, Query, QueryError, Term};

/// One place where a term is in a field: the range in code points, and the kind.
type Place = (u32, u32, u32);

/// A field of a cell: which, which of several, and its content.
type FieldText = (Field, u32, Prepared);

/// A text, one user-perceived character at a time: the characters to compare, as they
/// are (`.0`) and folded (`.1`), each with the user-perceived character it came from;
/// and where each user-perceived character is in the text, in code points.
struct Prepared {
    text: String,
    exact: (Vec<char>, Vec<usize>),
    folded: (Vec<char>, Vec<usize>),
    spans: Vec<(u32, u32)>,
}

impl Prepared {
    fn new(text: String) -> Prepared {
        let mut exact = (Vec::new(), Vec::new());
        let mut folded = (Vec::new(), Vec::new());
        let mut spans: Vec<(u32, u32)> = Vec::new();
        let mut at = 0u32;
        for cluster in text.graphemes(true) {
            for c in cluster.chars() {
                exact.0.push(c);
                exact.1.push(spans.len());
            }
            for c in fold(cluster).chars() {
                folded.0.push(c);
                folded.1.push(spans.len());
            }
            let len = cluster.chars().count() as u32;
            spans.push((at, len));
            at += len;
        }
        Prepared {
            text,
            exact,
            folded,
            spans,
        }
    }
}

/// A document to search: its name, the document, and the results of its formula cells
/// (its editor's, or `Calc::new` of a document as loaded).
pub type Named<'a> = (&'a str, &'a Document, &'a Calc);

struct World<'a> {
    docs: &'a [Named<'a>],
    links: LinkIndex,
}

impl World<'_> {
    fn doc(&self, id: DocId) -> Option<(&Document, &Calc)> {
        let found = self.docs.iter().find(|(_, doc, _)| doc.id() == id);
        found.map(|(_, doc, calc)| (*doc, *calc))
    }

    /// Every field of a cell that is searched.
    fn fields(&self, doc: &Document, calc: &Calc, id: CellId, cell: &Cell) -> Vec<FieldText> {
        let what = searchable(calc, id, cell);
        let mut out = vec![(Field::Text, 0, Prepared::new(what.text))];
        out.extend(
            what.shown
                .map(|shown| (Field::Shown, 0, Prepared::new(shown))),
        );
        out.push((Field::Note, 0, Prepared::new(what.note)));
        out.extend(
            what.image
                .map(|image| (Field::Image, 0, Prepared::new(image))),
        );
        for (n, url) in what.urls.into_iter().enumerate() {
            out.push((Field::Url, n as u32, Prepared::new(url)));
        }
        for (n, link) in cell.links.iter().enumerate() {
            if !matches!(link.target, LinkTarget::Cell { .. }) {
                continue;
            }
            let Resolved::Cell {
                doc: to,
                cell: target,
                ..
            } = self.links.resolve(doc.id(), &link.target)
            else {
                continue;
            };
            // What the cell at the other end shows (decision A66).
            let shown = self.doc(to).and_then(|(doc, calc)| calc.shown(doc, target));
            if let Some(shown) = shown {
                out.push((Field::LinkedText, n as u32, Prepared::new(shown.to_owned())));
            }
        }
        out
    }
}

fn starts_word(text: &[char], at: usize) -> bool {
    let (Some(before), after) = (at.checked_sub(1).and_then(|n| text.get(n)), text.get(at)) else {
        return true;
    };
    !is_word_char(*before) || is_wordless(*before) || after.is_some_and(|c| is_wordless(*c))
}

fn ends_word(text: &[char], at: usize) -> bool {
    let (before, Some(after)) = (at.checked_sub(1).and_then(|n| text.get(n)), text.get(at)) else {
        return true;
    };
    !is_word_char(*after) || is_wordless(*after) || before.is_some_and(|c| is_wordless(*c))
}

/// Where `term` is in `text`.
fn places(term: &Term, text: &Prepared) -> Vec<Place> {
    let (hay, from) = if term.exact {
        (&text.exact.0, &text.exact.1)
    } else {
        (&text.folded.0, &text.folded.1)
    };
    let spans = &text.spans;
    let needle: Vec<char> = if term.exact {
        term.text.chars().collect()
    } else {
        fold(&term.text).chars().collect()
    };
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    let mut i = 0usize;
    while i + needle.len() <= hay.len() {
        let end = i + needle.len();
        if hay.get(i..end) != Some(needle.as_slice()) {
            i += 1;
            continue;
        }
        let kind = if i == 0 && end == hay.len() {
            0
        } else if !starts_word(hay, i) {
            3
        } else if ends_word(hay, end) {
            1
        } else {
            2
        };
        let first = from.get(i).and_then(|n| spans.get(*n));
        let last = from.get(end - 1).and_then(|n| spans.get(*n));
        if let (Some(first), Some(last)) = (first, last) {
            out.push((first.0, last.0 + last.1 - first.0, kind));
        }
        i = end;
    }
    out
}

fn rank(field: Field) -> u32 {
    match field {
        Field::Text => 0,
        Field::Shown => 1,
        Field::Note => 2,
        Field::LinkedText => 3,
        Field::Image => 4,
        Field::Url => 5,
    }
}

/// One cell being looked at.
struct Look<'a> {
    world: &'a World<'a>,
    name: &'a str,
    doc: &'a Document,
    id: CellId,
    cell: &'a Cell,
    fields: &'a [FieldText],
}

impl Look<'_> {
    fn found(&self, term: &Term) -> bool {
        let empty = if term.exact {
            term.text.is_empty()
        } else {
            fold(&term.text).is_empty()
        };
        empty
            || self
                .fields
                .iter()
                .any(|(_, _, text)| !places(term, text).is_empty())
    }

    fn passes(&self, filter: &Filter) -> bool {
        let (world, doc, cell) = (self.world, self.doc, self.cell);
        let resolved = || {
            cell.links
                .iter()
                .map(|link| world.links.resolve(doc.id(), &link.target))
        };
        match filter {
            Filter::Tag(name) => cell.text == *name && doc.tags().contains_key(name),
            Filter::Has(Has::Text) => !cell.text.is_empty(),
            Filter::Has(Has::Note) => !cell.note.is_empty(),
            Filter::Has(Has::Image) => cell.image.is_some(),
            Filter::Has(Has::Grid) => cell.grid().is_some(),
            Filter::Has(Has::Link) => !cell.links.is_empty(),
            Filter::Has(Has::BrokenLink) => resolved().any(|to| matches!(to, Resolved::Broken(_))),
            Filter::Has(Has::Tag) => doc.tags().contains_key(&cell.text),
            Filter::LinksTo(target) => {
                resolved().any(|to| matches!(to, Resolved::Cell { cell, .. } if cell == *target))
            }
            Filter::LinkedFrom(source) => world.docs.iter().any(|(_, from, _)| {
                let links = from.cell(*source).map_or(&[][..], |cell| cell.links.as_slice());
                links.iter().any(|link| {
                    let to = world.links.resolve(from.id(), &link.target);
                    matches!(to, Resolved::Cell { doc: d, cell: c, .. } if d == doc.id() && c == self.id)
                })
            }),
            Filter::Edited { from, to } => cell.last_edit.is_some_and(|time| {
                from.is_none_or(|from| time >= from) && to.is_none_or(|to| time <= to)
            }),
            Filter::Recent(percent) => {
                let mut times: Vec<i64> = doc
                    .cells_in_order()
                    .iter()
                    .filter_map(|id| doc.cell(*id))
                    .map(|cell| cell.last_edit.unwrap_or(i64::MIN))
                    .collect();
                times.sort();
                times.reverse();
                let last = (times.len() * usize::from(*percent) / 100).min(times.len().saturating_sub(1));
                times.get(last).is_some_and(|from| cell.last_edit.unwrap_or(i64::MIN) >= *from)
            }
            Filter::Doc(id) => doc.id() == *id,
            Filter::DocName(name) => {
                let once = |name: &str| {
                    let lower = name.to_lowercase();
                    lower.strip_suffix(".wombat").map(str::to_owned).unwrap_or(lower)
                };
                once(self.name) == once(name)
            }
            Filter::Within(branch) => {
                let mut at = Some(self.id);
                let mut steps = 0;
                while let Some(id) = at.filter(|_| steps <= doc.cell_count()) {
                    if id == *branch {
                        return true;
                    }
                    at = doc.cell(id).and_then(Cell::parent);
                    steps += 1;
                }
                false
            }
            Filter::Kind(kind) => cell.kind == *kind,
            Filter::CellColor(color) => cell.cell_color == *color,
            Filter::TextColor(color) => cell.text_color == *color,
            Filter::Style(style) => cell.style == *style,
            Filter::TextIs(text) => cell.text == *text,
            Filter::Image(name) => cell.image.as_ref().is_some_and(|image| image.file == *name),
        }
    }

    fn matches(&self, query: &Query) -> bool {
        match query {
            Query::All => true,
            Query::Term(term) => self.found(term),
            Query::Filter(filter) => self.passes(filter),
            Query::And(list) => list.iter().all(|query| self.matches(query)),
            Query::Or(list) => list.iter().any(|query| self.matches(query)),
            Query::Not(inner) => !self.matches(inner),
        }
    }
}

/// The terms of a query that are not under a negation.
fn positive<'a>(query: &'a Query, out: &mut Vec<&'a Term>) {
    match query {
        Query::Term(term) => out.push(term),
        Query::And(list) | Query::Or(list) => list.iter().for_each(|query| positive(query, out)),
        Query::All | Query::Filter(_) | Query::Not(_) => {}
    }
}

fn merged(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    ranges.sort();
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (start, len) in ranges {
        let end = start + len;
        match out.last_mut() {
            Some(last) if last.0 + last.1 >= start => last.1 = end.max(last.0 + last.1) - last.0,
            _ => out.push((start, len)),
        }
    }
    out
}

/// What results are sorted by; the document's name and ID stand for its rank.
type SortKey<'a> = (Vec<u64>, &'a str, DocId, usize);

/// The documents read once, ready to be searched any number of times: every field of
/// every cell, folded. A search still looks at every one of them.
pub struct Reference<'a> {
    world: World<'a>,
    /// For each document, for each cell in document order, its fields.
    fields: Vec<Vec<Vec<FieldText>>>,
}

impl<'a> Reference<'a> {
    /// Reads the documents, each given with its name.
    pub fn new(docs: &'a [Named<'a>]) -> Reference<'a> {
        let mut links = LinkIndex::new();
        for (_, doc, _) in docs {
            links.insert_document(doc);
        }
        let world = World { docs, links };
        let of_doc = |doc: &Document, calc: &Calc| {
            let cells = doc.cells_in_order();
            let cells = cells.iter().filter_map(|id| Some((*id, doc.cell(*id)?)));
            cells
                .map(|(id, cell)| world.fields(doc, calc, id, cell))
                .collect()
        };
        let fields = docs
            .iter()
            .map(|(_, doc, calc)| of_doc(doc, calc))
            .collect();
        Reference { world, fields }
    }

    /// What [`crate::SearchIndex::search`] returns for an index of the documents.
    pub fn search(&self, query: &Query, options: &Options) -> Result<Results, QueryError> {
        query.check()?;
        let found = self.all(query, options.order);
        let total = found.len();
        let page = found.into_iter().skip(options.offset).take(options.limit);
        Ok(Results {
            total,
            hits: page.map(|(_, hit)| finish(self.world.docs, hit)).collect(),
        })
    }

    /// What [`crate::SearchIndex::next_match`] returns for an index of the documents.
    pub fn next_match(
        &self,
        query: &Query,
        doc: DocId,
        from: Option<CellId>,
        reverse: bool,
    ) -> Result<Option<Hit>, QueryError> {
        query.check()?;
        let docs = self.world.docs;
        let Some((_, document, _)) = docs.iter().find(|(_, d, _)| d.id() == doc) else {
            return Ok(None);
        };
        let found = self.all(query, Order::Document).into_iter();
        let hits: Vec<Hit> = found
            .map(|(_, hit)| hit)
            .filter(|hit| hit.doc == doc)
            .collect();
        // Walk the document from the cell after `from`, round to `from` itself.
        let mut order = document.cells_in_order();
        if reverse {
            order.reverse();
        }
        let start = from.and_then(|from| order.iter().position(|id| *id == from));
        if let Some(start) = start {
            order.rotate_left(start + 1);
        }
        let next = order
            .iter()
            .find_map(|id| hits.iter().find(|hit| hit.cell == *id));
        Ok(next.cloned().map(|hit| finish(docs, hit)))
    }

    /// Every matching cell of all documents, sorted, as (key, hit); the snippets and
    /// trails are left for [`finish`], which only the hits that are returned need.
    fn all(&self, query: &Query, order: Order) -> Vec<(SortKey<'a>, Hit)> {
        let world = &self.world;
        let docs = world.docs;
        let mut terms = Vec::new();
        positive(query, &mut terms);
        let mut out = Vec::new();
        for ((name, doc, calc), fields) in docs.iter().zip(&self.fields) {
            let cells = doc.cells_in_order().into_iter();
            let cells = cells.filter_map(|id| Some((id, doc.cell(id)?)));
            for ((place, (id, cell)), fields) in cells.enumerate().zip(fields) {
                let look = Look {
                    world,
                    name,
                    doc,
                    id,
                    cell,
                    fields,
                };
                if !look.matches(query) {
                    continue;
                }
                // The points, and every place of every word.
                let mut score = 0u32;
                let mut found: Vec<(Field, u32, u32, u32)> = Vec::new();
                for term in &terms {
                    let mut best = if look.found(term) { 0 } else { ABSENT };
                    let mut any = false;
                    for (field, sub, text) in look.fields {
                        for (start, len, kind) in places(term, text) {
                            let points = rank(*field) * 4 + kind;
                            best = if any { best.min(points) } else { points };
                            any = true;
                            found.push((*field, *sub, start, len));
                        }
                    }
                    score += best;
                }
                let best = found
                    .iter()
                    .map(|(field, sub, _, _)| (rank(*field), *sub))
                    .min();
                let (field, sub) = match best {
                    None => (Field::Text, 0),
                    Some(best) => found
                        .iter()
                        .map(|(field, sub, _, _)| (*field, *sub))
                        .find(|(field, sub)| (rank(*field), *sub) == best)
                        .unwrap_or((Field::Text, 0)),
                };
                let matched = look
                    .fields
                    .iter()
                    .find(|(f, s, _)| (*f, *s) == (field, sub));
                let matched = matched.map_or(String::new(), |(_, _, text)| text.text.clone());
                let in_field = found.iter().filter(|(f, s, _, _)| (*f, *s) == (field, sub));
                let ranges = merged(in_field.map(|(_, _, start, len)| (*start, *len)).collect());
                let depth = doc.depth(id).unwrap_or(0);
                let recency = match cell.last_edit {
                    None => u64::MAX,
                    // Later first: the distance from the latest possible time.
                    Some(time) => (i128::from(i64::MAX) - i128::from(time)) as u64,
                };
                let len = cell.text.chars().count() as u64;
                let rest = [len, u64::from(depth)];
                let first: Vec<u64> = match order {
                    Order::Relevance => [&[u64::from(score)][..], &rest].concat(),
                    Order::RelevanceThenRecent => {
                        [&[u64::from(score), recency][..], &rest].concat()
                    }
                    Order::Recent => [&[recency, u64::from(score)][..], &rest].concat(),
                    Order::Document => Vec::new(),
                };
                let hit = Hit {
                    doc: doc.id(),
                    cell: id,
                    field,
                    snippet: Default::default(),
                    matched,
                    ranges,
                    text: cell.text.clone(),
                    shown: (cell.kind == CellKind::Formula)
                        .then(|| calc.shown(doc, id).map(str::to_owned))
                        .flatten(),
                    trail: Vec::new(),
                    score,
                    depth,
                    last_edit: cell.last_edit,
                };
                out.push(((first, *name, doc.id(), place), hit));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

/// Gives a hit its snippet and its location trail.
fn finish(docs: &[Named], mut hit: Hit) -> Hit {
    hit.snippet = snippet(&hit.matched, &hit.ranges, SNIPPET_WIDTH);
    let doc = docs.iter().find(|(_, doc, _)| doc.id() == hit.doc);
    let place = doc.and_then(|(_, doc, calc)| links::place_shown(doc, calc, hit.cell));
    hit.trail = place.map(|place| place.trail).unwrap_or_default();
    hit
}

/// [`Reference::search`] of these documents, each given with its name.
pub fn search(docs: &[Named], query: &Query, options: &Options) -> Result<Results, QueryError> {
    Reference::new(docs).search(query, options)
}

/// [`Reference::next_match`] of these documents.
pub fn next_match(
    docs: &[Named],
    query: &Query,
    doc: DocId,
    from: Option<CellId>,
    reverse: bool,
) -> Result<Option<Hit>, QueryError> {
    Reference::new(docs).next_match(query, doc, from, reverse)
}
