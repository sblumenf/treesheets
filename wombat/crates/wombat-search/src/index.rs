//! What the search index keeps about one document, and how it follows edits (decisions
//! A13, A53, A73).
//!
//! A [`DocSearch`] holds, for every cell: its ID, the cell it is in and its place there,
//! what is searched (the texts as they are and in folded form), and the properties that
//! filters ask about. Over these it keeps a table that says, for every folded character
//! and every pair of neighbouring folded characters, which cells may have it: this is
//! what makes a search look at a few cells instead of all.
//!
//! The table only ever proposes cells; whether a cell matches is always decided by
//! looking at its text. So the table may name too many cells but never too few, which
//! allows three economies (decision A111):
//!
//! - A key is not stored: it is hashed into one of a fixed number of lists (twice the
//!   number of cells, a power of two), and keys that share a list share its cells. A
//!   document of very varied text (every pair of characters a new one, as in Chinese)
//!   therefore costs four bytes for each pair and no more.
//! - A list that more than one cell in 32 would be on is kept as one bit per cell.
//!   Nothing is "too common to list": the letter `e` has its bits like any other key.
//! - An edit only adds: the lists are one packed array built in one go, additions go
//!   to a small table beside it, and what an edit makes stale is counted. When half of
//!   the table is stale, or it has doubled, it is built again from the cells (an
//!   amount of work that the edits before it pay for).
//!
//! Following a change report costs in proportion to the cells it names (to the length
//! of their texts), plus the cells directly inside a grid that gained, lost or
//! reordered cells.
//!
//! A formula cell is kept with its formula (its text) and with what it shows (its
//! result). Results are not in the document: they come from the [`Calc`] of the
//! document's editor, or are worked out here for a document that nobody edits, and after
//! an edit the cells whose results changed are read again like the cells that the change
//! report names (decision A90).

use std::collections::{BTreeSet, HashMap};
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::OnceLock;

use wombat_core::eval::Calc;
use wombat_core::links::DocLinks;
use wombat_core::summary::{Reader, Summary, Writer};
use wombat_core::{Cell, CellId, CellKind, Changes, Color, DocId, Document, LinkTarget, Style};

use crate::fold::{self, fold_into};

/// "No cell": the parent of the root.
pub(crate) const NONE: u32 = u32::MAX;

/// What the table says about a key: the cells that may have it. No other cell does.
#[derive(Clone, Copy)]
enum Proposed<'a> {
    /// No cell.
    None,
    /// These cells: the ones listed when the table was built, and the ones added since.
    List(&'a [u32], &'a [u32]),
    /// The cells whose bit is set (bit `slot % 64` of word `slot / 64`).
    Bits(&'a [u64]),
}

impl Proposed<'_> {
    fn has(&self, slot: u32) -> bool {
        match self {
            Proposed::None => false,
            Proposed::List(built, added) => built.contains(&slot) || added.contains(&slot),
            Proposed::Bits(bits) => bit(bits, slot as usize),
        }
    }
}

/// Whether bit `at` is set.
pub(crate) fn bit(bits: &[u64], at: usize) -> bool {
    bits.get(at / 64)
        .is_some_and(|word| word >> (at % 64) & 1 == 1)
}

/// Sets bit `at`, if `bits` reaches that far.
pub(crate) fn set_bit(bits: &mut [u64], at: usize) {
    if let Some(word) = bits.get_mut(at / 64) {
        *word |= 1 << (at % 64);
    }
}

/// The table: see the module documentation.
#[derive(Clone, Debug, Default)]
struct Table {
    /// The number of lists is two to this power.
    log: u32,
    /// The lists as built, packed: list `n` is `slots[starts[n]..starts[n + 1]]`.
    starts: Vec<u32>,
    slots: Vec<u32>,
    /// The lists that are kept as bits instead (and then not in `slots`).
    dense: FastMap<u32, Vec<u64>>,
    /// What edits added since the table was built.
    added: FastMap<u32, Vec<u32>>,
}

impl Table {
    fn list_of(&self, key: u64) -> u32 {
        // The top bits of a multiplication mix all bits of the key.
        let mixed = key.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        mixed.checked_shr(64 - self.log).unwrap_or(0) as u32
    }

    fn built(&self, list: u32) -> &[u32] {
        let from = self.starts.get(list as usize).copied().unwrap_or(0) as usize;
        let to = self.starts.get(list as usize + 1).copied().unwrap_or(0) as usize;
        self.slots.get(from..to).unwrap_or_default()
    }

    fn get(&self, list: u32) -> Proposed<'_> {
        if let Some(bits) = self.dense.get(&list) {
            return Proposed::Bits(bits);
        }
        let added = self.added.get(&list).map_or(&[][..], Vec::as_slice);
        match (self.built(list), added) {
            ([], []) => Proposed::None,
            (built, added) => Proposed::List(built, added),
        }
    }

    /// Puts a slot on a list; the caller knows that it is not on it yet (or does not
    /// mind). Returns whether anything was added.
    fn post(&mut self, list: u32, slot: u32, common_at: usize) -> bool {
        if let Some(bits) = self.dense.get_mut(&list) {
            let word = slot as usize / 64;
            if bits.len() <= word {
                bits.resize(word + 1, 0);
            }
            let had = bit(bits, slot as usize);
            set_bit(bits, slot as usize);
            return !had;
        }
        let mut added = self.added.remove(&list).unwrap_or_default();
        added.push(slot);
        let built = self.built(list);
        if built.len() + added.len() <= common_at {
            self.added.insert(list, added);
            return true;
        }
        // It has become common: as bits from here on.
        let all = added.iter().chain(built);
        let words = all.clone().max().map_or(0, |last| *last as usize / 64 + 1);
        let mut bits = vec![0u64; words];
        for slot in all {
            set_bit(&mut bits, *slot as usize);
        }
        self.dense.insert(list, bits);
        true
    }

    fn bytes(&self) -> usize {
        let map = |len: usize, each: usize| len * (each + 1);
        let dense: usize = self.dense.values().map(|bits| bits.capacity() * 8).sum();
        let added: usize = self.added.values().map(|list| list.capacity() * 4).sum();
        (self.starts.capacity() + self.slots.capacity()) * 4
            + map(
                self.dense.capacity(),
                std::mem::size_of::<(u32, Vec<u64>)>(),
            )
            + map(
                self.added.capacity(),
                std::mem::size_of::<(u32, Vec<u32>)>(),
            )
            + dense
            + added
    }
}

/// What of a cell is searched. **Every reader of a cell's searchable content goes
/// through [`searchable`]**, the index and the reference scanner alike, so that there is
/// one place to extend.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Searchable {
    /// The text of the cell, as stored. Ranges of a match in the text refer to this.
    pub text: String,
    /// What the cell displays, where that is not its stored text: the result of a
    /// formula cell, also when it happens to read the same as the formula. It is
    /// searched as a field of its own ([`crate::Field::Shown`]).
    pub shown: Option<String>,
    /// The note.
    pub note: String,
    /// The file name of the image.
    pub image: Option<String>,
    /// The external targets of the cell's links (URLs and file paths), in text order.
    pub urls: Vec<String>,
}

/// What of the cell `id` is searched: see [`Searchable`]. `calc` holds the results of
/// the formula cells of the cell's document.
pub fn searchable(calc: &Calc, id: CellId, cell: &Cell) -> Searchable {
    let result = || calc.result(id).map(|(text, _)| text.to_owned());
    let urls = cell.links.iter().filter_map(|link| match &link.target {
        LinkTarget::External(target) => Some(target.clone()),
        _ => None,
    });
    Searchable {
        text: cell.text.clone(),
        shown: (cell.kind == CellKind::Formula).then(result).flatten(),
        note: cell.note.clone(),
        image: cell.image.as_ref().map(|image| image.file.clone()),
        urls: urls.collect(),
    }
}

/// A hasher for keys that are already well spread (random IDs) or small numbers.
#[derive(Default, Clone, Copy)]
pub(crate) struct Mix(u64);

impl Hasher for Mix {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            for (to, from) in word.iter_mut().zip(chunk) {
                *to = *from;
            }
            self.write_u64(u64::from_le_bytes(word));
        }
    }
    fn write_u64(&mut self, n: u64) {
        let mixed = (self.0.rotate_left(23) ^ n).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        self.0 = mixed ^ (mixed >> 29);
    }
    fn write_u128(&mut self, n: u128) {
        self.write_u64((n >> 64) as u64 ^ n as u64);
    }
    fn write_u32(&mut self, n: u32) {
        self.write_u64(u64::from(n));
    }
    fn write_usize(&mut self, n: usize) {
        self.write_u64(n as u64);
    }
}

pub(crate) type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<Mix>>;

/// The less common parts of a cell, kept apart so that most cells do not pay for them.
#[derive(Clone, PartialEq, Debug, Default)]
pub(crate) struct Rare {
    pub note: String,
    pub shown: Option<String>,
    pub image: Option<String>,
    pub urls: Vec<String>,
}

/// One cell as the index keeps it.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub id: CellId,
    /// The slot of the cell this one is in; [`NONE`] for the root.
    pub parent: u32,
    /// Its place among the cells of that cell's grid, row by row.
    pub pos: u32,
    pub text: Box<str>,
    /// The note, a displayed result, an image name, link targets: if it has any.
    pub rare: Option<Box<Rare>>,
    /// The folded forms of text, displayed result, note, image name and link targets,
    /// in this order with a line break between them, without the empty ones at the end;
    /// `None` when that is the text itself (a cell with only lower-case plain text).
    folded: Option<Box<str>>,
    pub last_edit: Option<i64>,
    pub cell_color: Color,
    pub text_color: Color,
    pub kind: CellKind,
    pub style: Style,
    pub has_grid: bool,
    pub has_links: bool,
    pub live: bool,
    /// How many columns the cell's grid has; 0 without a grid. With `pos` of the cells
    /// inside, this says which of them are in one row.
    pub cols: u32,
    /// How many characters (code points) the cell shows ([`Entry::display`]): results
    /// are ordered by it, and so it is asked of every cell that matches.
    pub shown_len: u32,
}

/// Which field of the folded form a place is in, by the number of line breaks before it.
pub(crate) const FIELD_TEXT: usize = 0;
pub(crate) const FIELD_SHOWN: usize = 1;
pub(crate) const FIELD_NOTE: usize = 2;
pub(crate) const FIELD_IMAGE: usize = 3;
pub(crate) const FIELD_URLS: usize = 4;

impl Entry {
    /// A cell with this content and the properties of a new cell.
    fn of(id: CellId, what: Searchable) -> Entry {
        let Searchable {
            text,
            shown,
            note,
            image,
            urls,
        } = what;
        let mut folded = String::with_capacity(text.len());
        fold_into(&text, &mut folded);
        let rest = [shown.as_deref(), Some(note.as_str()), image.as_deref()];
        let rest = rest.into_iter().map(Option::unwrap_or_default);
        for part in rest.chain(urls.iter().map(String::as_str)) {
            folded.push('\n');
            fold_into(part, &mut folded);
        }
        folded.truncate(folded.trim_end_matches('\n').len());
        let any = !note.is_empty() || shown.is_some() || image.is_some() || !urls.is_empty();
        let shown_len = shown.as_deref().unwrap_or(&text).chars().count() as u32;
        let rare = any.then(|| {
            Box::new(Rare {
                note,
                shown,
                image,
                urls,
            })
        });
        Entry {
            id,
            parent: NONE,
            pos: 0,
            folded: (folded != text).then(|| folded.into_boxed_str()),
            text: text.into_boxed_str(),
            rare,
            last_edit: None,
            cell_color: Color::WHITE,
            text_color: Color::BLACK,
            kind: CellKind::Data,
            style: Style::default(),
            has_grid: false,
            has_links: false,
            live: true,
            cols: 0,
            shown_len,
        }
    }

    fn new(id: CellId, cell: &Cell, what: Searchable) -> Entry {
        Entry {
            last_edit: cell.last_edit,
            cell_color: cell.cell_color,
            text_color: cell.text_color,
            kind: cell.kind,
            style: cell.style,
            has_grid: cell.grid().is_some(),
            cols: cell.grid().map_or(0, |grid| grid.xs()),
            has_links: !cell.links.is_empty(),
            ..Entry::of(id, what)
        }
    }

    /// The folded forms of all fields: see the field `folded`.
    pub fn folded(&self) -> &str {
        self.folded.as_deref().unwrap_or(&self.text)
    }

    /// The folded form of the text alone.
    pub fn folded_text(&self) -> &str {
        let all = self.folded();
        all.split_once('\n').map_or(all, |(text, _)| text)
    }

    pub fn note(&self) -> &str {
        self.rare.as_ref().map_or("", |rare| rare.note.as_str())
    }

    pub fn image(&self) -> Option<&str> {
        self.rare.as_ref()?.image.as_deref()
    }

    pub fn shown(&self) -> Option<&str> {
        self.rare.as_ref()?.shown.as_deref()
    }

    /// What the cell shows: the result of a formula cell, the text of any other. This
    /// is what a link to the cell and a location trail go by (decision A66).
    pub fn display(&self) -> &str {
        self.shown().unwrap_or(&self.text)
    }

    /// The folded form of [`Entry::display`].
    pub fn folded_display(&self) -> &str {
        if self.shown().is_none() {
            return self.folded_text();
        }
        self.folded().split('\n').nth(FIELD_SHOWN).unwrap_or("")
    }

    pub fn urls(&self) -> &[String] {
        self.rare.as_ref().map_or(&[], |rare| rare.urls.as_slice())
    }

    fn bytes(&self) -> usize {
        let rare = self.rare.as_ref().map_or(0, |rare| {
            let texts = [&rare.shown, &rare.image].into_iter().flatten();
            std::mem::size_of::<Rare>()
                + rare.note.capacity()
                + texts.map(String::capacity).sum::<usize>()
                + rare
                    .urls
                    .iter()
                    .map(|url| url.capacity() + 24)
                    .sum::<usize>()
        });
        self.text.len() + self.folded.as_ref().map_or(0, |f| f.len()) + rare
    }
}

/// Calls `each` with every key of a folded form: one per character, and one per pair of
/// neighbouring characters within a field. (With the single characters, a search for
/// one letter need not look at every cell.) Keys come as often as they occur.
pub(crate) fn each_key(folded: &str, mut each: impl FnMut(u64)) {
    let mut before: Option<char> = None;
    for c in folded.chars() {
        if c == '\n' {
            before = None;
            continue;
        }
        each(single_key(c));
        if let Some(before) = before {
            each(pair_key(before, c));
        }
        before = Some(c);
    }
}

pub(crate) fn single_key(c: char) -> u64 {
    u64::from(c)
}

pub(crate) fn pair_key(first: char, second: char) -> u64 {
    ((u64::from(first) + 1) << 32) | u64::from(second)
}

/// Where the cells are, by slot.
#[derive(Clone, Debug)]
struct Layout {
    places: Vec<u32>,
    depths: Vec<u8>,
}

/// What the search index keeps about one document. See the module documentation.
#[derive(Clone, Debug)]
pub struct DocSearch {
    id: DocId,
    root: u32,
    pub(crate) entries: Vec<Entry>,
    free: Vec<u32>,
    slots: FastMap<CellId, u32>,
    table: Table,
    /// How many entries the table had when it was built, how many were added since,
    /// and how many of all of them are known stale.
    built: usize,
    posted: usize,
    stale: usize,
    compact_at: usize,
    /// A list that more than this many cells are on is kept as bits, which is smaller
    /// from one cell in 32 on. A 32nd of the cells when the table was last built, and
    /// at least 64.
    common_at: usize,
    common_floor: usize,
    tags: BTreeSet<String>,
    /// One bit per slot: whether the cell has links. Made when asked for, dropped by
    /// every edit.
    linking: OnceLock<Vec<u64>>,
    /// For each slot its place in document order and its depth; made when asked for,
    /// dropped when cells are added, removed or moved.
    layout: OnceLock<Layout>,
}

impl DocSearch {
    /// The document's ID.
    pub fn id(&self) -> DocId {
        self.id
    }

    /// The number of cells of the document.
    pub fn cell_count(&self) -> usize {
        self.slots.len()
    }

    /// Whether the document has this cell.
    pub fn has_cell(&self, cell: CellId) -> bool {
        self.slots.contains_key(&cell)
    }

    pub(crate) fn slot(&self, cell: CellId) -> Option<u32> {
        self.slots.get(&cell).copied()
    }

    pub(crate) fn entry(&self, slot: u32) -> Option<&Entry> {
        self.entries.get(slot as usize).filter(|entry| entry.live)
    }

    pub(crate) fn has_tag(&self, name: &str) -> bool {
        self.tags.contains(name)
    }

    /// Sets, in `out` (one bit per slot), the cells that may have every one of `keys`:
    /// all cells that do, and perhaps others. `keys` must not be empty.
    pub(crate) fn candidates(&self, keys: &[u64], out: &mut [u64]) {
        let mut lists: Vec<u32> = keys.iter().map(|key| self.table.list_of(*key)).collect();
        lists.sort_unstable();
        lists.dedup();
        let mut shortest: Option<(&[u32], &[u32])> = None;
        let mut bits: Vec<&[u64]> = Vec::new();
        for list in lists {
            match self.table.get(list) {
                Proposed::None => return,
                Proposed::Bits(found) => bits.push(found),
                Proposed::List(built, added) => {
                    let len = |pair: (&[u32], &[u32])| pair.0.len() + pair.1.len();
                    if shortest.is_none_or(|shortest| len((built, added)) < len(shortest)) {
                        shortest = Some((built, added));
                    }
                }
            }
        }
        match shortest {
            // The shortest list, without the cells that some list of bits rules out.
            Some((built, added)) => {
                for slot in built.iter().chain(added) {
                    if bits.iter().all(|bits| bit(bits, *slot as usize)) {
                        set_bit(out, *slot as usize);
                    }
                }
            }
            // Bits only: the cells that are in all of them.
            None => {
                for (n, word) in out.iter_mut().enumerate() {
                    let all = bits.iter().map(|bits| bits.get(n).copied().unwrap_or(0));
                    *word = all.fold(u64::MAX, |all, next| all & next);
                }
            }
        }
    }

    /// One bit per slot: whether the cell there has links.
    pub(crate) fn linking(&self) -> &[u64] {
        self.linking.get_or_init(|| {
            let mut bits = vec![0u64; self.entries.len().div_ceil(64)];
            for (slot, entry) in self.entries.iter().enumerate() {
                if entry.live && entry.has_links {
                    set_bit(&mut bits, slot);
                }
            }
            bits
        })
    }

    /// How many slots there are, with or without a cell.
    pub(crate) fn slot_count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Sets after how many stale entries the table is built again (when they are also
    /// half of it), and the fewest cells that make a list common, and builds the table
    /// again. For tests, which want to see both happen in small documents.
    #[doc(hidden)]
    pub fn set_thresholds(&mut self, stale: usize, common: usize) {
        self.compact_at = stale;
        self.common_floor = common;
        self.compact();
    }

    /// How many times more entries the table has than a table built now would have,
    /// roughly: (entries built and added, entries known to be stale).
    #[doc(hidden)]
    pub fn table_load(&self) -> (usize, usize) {
        (self.built + self.posted, self.stale)
    }

    /// The memory the index of this document takes, in bytes, as far as it can be
    /// counted: the texts and their folded forms, the per-cell records, and the tables.
    pub fn memory_bytes(&self) -> usize {
        let entries = self.entries.capacity() * std::mem::size_of::<Entry>();
        let texts: usize = self.entries.iter().map(Entry::bytes).sum();
        // A hash table keeps about one byte of bookkeeping per place and is at most 7/8 full.
        let slots = self.slots.capacity() * (std::mem::size_of::<(CellId, u32)>() + 1);
        let order = self
            .layout
            .get()
            .map_or(0, |layout| layout.places.capacity() * 5);
        let linking = self.linking.get().map_or(0, |bits| bits.capacity() * 8);
        let table = self.table.bytes();
        entries + texts + slots + table + order + linking + self.free.capacity() * 4
    }

    /// The bytes of text that are searched (texts, notes, image names, link targets).
    pub fn text_bytes(&self) -> usize {
        let rare = |entry: &Entry| {
            let image = entry.image().map_or(0, str::len);
            let shown = entry.shown().map_or(0, str::len);
            image + shown + entry.urls().iter().map(String::len).sum::<usize>()
        };
        let live = self.entries.iter().filter(|entry| entry.live);
        live.map(|entry| entry.text.len() + entry.note().len() + rare(entry))
            .sum()
    }

    fn empty(id: DocId) -> DocSearch {
        DocSearch {
            id,
            root: NONE,
            entries: Vec::new(),
            free: Vec::new(),
            slots: FastMap::default(),
            table: Table::default(),
            built: 0,
            posted: 0,
            stale: 0,
            compact_at: 4096,
            common_at: 64,
            common_floor: 64,
            tags: BTreeSet::new(),
            linking: OnceLock::new(),
            layout: OnceLock::new(),
        }
    }

    /// The lists of the table that a folded form is on, sorted, each once.
    fn lists(table: &Table, folded: &str, out: &mut Vec<u32>) {
        out.clear();
        let count = 1usize << table.log;
        if folded.len() < count / 4 {
            each_key(folded, |key| out.push(table.list_of(key)));
            out.sort_unstable();
            out.dedup();
            return;
        }
        // A long text is on a large part of the lists: mark them, then read them off,
        // which costs its length and not the sorting of it.
        let mut seen = vec![0u64; count.div_ceil(64)];
        each_key(folded, |key| {
            set_bit(&mut seen, table.list_of(key) as usize)
        });
        for (n, word) in seen.iter().enumerate() {
            let mut word = *word;
            while word != 0 {
                out.push((n * 64) as u32 + word.trailing_zeros());
                word &= word - 1;
            }
        }
    }

    /// Puts a cell in a new slot, without telling the table.
    fn push(&mut self, entry: Entry) -> u32 {
        let id = entry.id;
        let slot = match self.free.pop() {
            Some(slot) => {
                if let Some(place) = self.entries.get_mut(slot as usize) {
                    *place = entry;
                }
                slot
            }
            None => {
                self.entries.push(entry);
                (self.entries.len() - 1) as u32
            }
        };
        self.slots.insert(id, slot);
        slot
    }

    fn post(&mut self, slot: u32, lists: &[u32]) {
        for list in lists {
            let added = self.table.post(*list, slot, self.common_at);
            self.posted += usize::from(added);
        }
    }

    /// Puts a cell in a new slot.
    fn add(&mut self, entry: Entry, scratch: &mut Vec<u32>) -> u32 {
        let slot = self.push(entry);
        let folded = self.entries.get(slot as usize).map_or("", Entry::folded);
        DocSearch::lists(&self.table, folded, scratch);
        self.post(slot, scratch);
        slot
    }

    /// Puts new content in the slot of a cell that changed, keeping its place.
    fn replace(&mut self, slot: u32, mut entry: Entry, scratch: &mut Vec<u32>) {
        let Some(old) = self.entries.get_mut(slot as usize) else {
            return;
        };
        let mut old_lists = Vec::new();
        DocSearch::lists(&self.table, old.folded(), &mut old_lists);
        DocSearch::lists(&self.table, entry.folded(), scratch);
        entry.parent = old.parent;
        entry.pos = old.pos;
        *old = entry;
        // Only what is new is added; what is no longer there stays, as stale. (Both
        // are sorted: one walk through the two.)
        let mut fresh: Vec<u32> = Vec::new();
        let mut gone = 0usize;
        let (mut a, mut b) = (old_lists.iter().peekable(), scratch.iter().peekable());
        loop {
            match (a.peek(), b.peek()) {
                (Some(old), Some(new)) if old == new => {
                    a.next();
                    b.next();
                }
                (Some(old), Some(new)) if old < new => {
                    gone += 1;
                    a.next();
                }
                (_, Some(new)) => {
                    fresh.push(**new);
                    b.next();
                }
                (Some(_), None) => {
                    gone += 1;
                    a.next();
                }
                (None, None) => break,
            }
        }
        self.stale = self.stale.saturating_add(gone);
        self.post(slot, &fresh);
    }

    fn kill(&mut self, slot: u32, scratch: &mut Vec<u32>) {
        let Some(entry) = self.entries.get_mut(slot as usize) else {
            return;
        };
        DocSearch::lists(&self.table, entry.folded(), scratch);
        self.stale = self.stale.saturating_add(scratch.len());
        entry.live = false;
        entry.text = Box::default();
        entry.folded = None;
        entry.rare = None;
        self.free.push(slot);
    }

    fn common_for(&self, cells: usize) -> usize {
        (cells / 32).max(self.common_floor)
    }

    /// Builds the table again from the cells, without what is stale: count how long
    /// each list gets, then fill them.
    fn compact(&mut self) {
        let cells = self.slots.len();
        self.common_at = self.common_for(cells);
        let log = (cells.saturating_mul(2).max(64))
            .next_power_of_two()
            .trailing_zeros()
            .min(24);
        let mut table = Table {
            log,
            ..Table::default()
        };
        let count = 1usize << log;
        let mut lens = vec![0u32; count + 1];
        let mut scratch = Vec::new();
        // The lists of every cell, one after the other, and how many each cell has:
        // kept between counting and filling, so that they are worked out once.
        let mut all: Vec<u32> = Vec::new();
        let mut counts: Vec<u32> = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            scratch.clear();
            if entry.live {
                DocSearch::lists(&table, entry.folded(), &mut scratch);
            }
            for list in &scratch {
                if let Some(len) = lens.get_mut(*list as usize) {
                    *len += 1;
                }
            }
            counts.push(scratch.len() as u32);
            all.extend_from_slice(&scratch);
        }
        // Where each list starts; the common ones get bits and no room here.
        let words = self.entries.len().div_ceil(64);
        let mut total = 0u32;
        for (list, len) in lens.iter_mut().enumerate() {
            let own = *len;
            *len = total;
            if own as usize > self.common_at {
                table.dense.insert(list as u32, vec![0u64; words]);
            } else {
                total += own;
            }
        }
        let mut next = lens.clone();
        let mut slots = vec![0u32; total as usize];
        let mut rest = all.as_slice();
        for (slot, count) in counts.iter().enumerate() {
            let (lists, after) = rest.split_at_checked(*count as usize).unwrap_or_default();
            rest = after;
            for list in lists {
                if let Some(bits) = table.dense.get_mut(list) {
                    set_bit(bits, slot);
                } else if let Some(at) = next.get_mut(*list as usize) {
                    if let Some(place) = slots.get_mut(*at as usize) {
                        *place = slot as u32;
                    }
                    *at += 1;
                }
            }
        }
        let built = all.len();
        table.starts = lens;
        table.slots = slots;
        self.table = table;
        self.built = built;
        self.posted = 0;
        self.stale = 0;
    }

    /// Makes the cells of the grid of `cell` know where they are.
    fn place_children(&mut self, doc: &Document, cell: CellId) {
        let (Some(slot), Some(found)) = (self.slot(cell), doc.cell(cell)) else {
            return;
        };
        if let Some(entry) = self.entries.get_mut(slot as usize) {
            entry.has_grid = found.grid().is_some();
            entry.cols = found.grid().map_or(0, |grid| grid.xs());
        }
        let children = found.grid().map_or(&[][..], |grid| grid.cells());
        for (pos, child) in children.iter().enumerate() {
            let child = self
                .slot(*child)
                .and_then(|s| self.entries.get_mut(s as usize));
            if let Some(child) = child {
                child.parent = slot;
                child.pos = pos as u32;
            }
        }
    }

    /// For each slot its place in document order ([`NONE`] for a slot without a cell).
    pub(crate) fn order(&self) -> &[u32] {
        &self.layout().places
    }

    fn layout(&self) -> &Layout {
        self.layout.get_or_init(|| {
            // The cells of each cell, in the order of their places.
            let mut inside: Vec<(u32, u32, u32)> = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.live && entry.parent != NONE)
                .map(|(slot, entry)| (entry.parent, entry.pos, slot as u32))
                .collect();
            inside.sort_unstable();
            let mut first = vec![0u32; self.entries.len() + 1];
            for (parent, _, _) in &inside {
                if let Some(count) = first.get_mut(*parent as usize + 1) {
                    *count += 1;
                }
            }
            let mut total = 0u32;
            for count in &mut first {
                total += *count;
                *count = total;
            }
            let mut places = vec![NONE; self.entries.len()];
            let mut depths = vec![0u8; self.entries.len()];
            let mut next = 0u32;
            let mut stack = vec![(self.root, 0u8)];
            while let Some((slot, depth)) = stack.pop() {
                let Some(place) = places
                    .get_mut(slot as usize)
                    .filter(|place| **place == NONE)
                else {
                    continue;
                };
                *place = next;
                next += 1;
                if let Some(of) = depths.get_mut(slot as usize) {
                    *of = depth;
                }
                let from = first.get(slot as usize).copied().unwrap_or(0) as usize;
                let to = first.get(slot as usize + 1).copied().unwrap_or(0) as usize;
                let children = inside.get(from..to).unwrap_or_default();
                let below = depth.saturating_add(1);
                stack.extend(children.iter().rev().map(|(_, _, child)| (*child, below)));
            }
            // A valid document has reached every cell by now; whatever a broken one
            // leaves over still gets a place, so that results stay in a fixed order.
            for (slot, place) in places.iter_mut().enumerate() {
                let live = self.entries.get(slot).is_some_and(|entry| entry.live);
                if live && *place == NONE {
                    *place = next;
                    next += 1;
                }
            }
            Layout { places, depths }
        })
    }

    /// The slots of all cells in document order.
    pub(crate) fn sequence(&self) -> Vec<u32> {
        let order = self.order();
        let mut sequence = vec![NONE; self.slots.len()];
        for (slot, place) in order.iter().enumerate() {
            if let Some(to) = sequence.get_mut(*place as usize) {
                *to = slot as u32;
            }
        }
        sequence
    }

    /// The depth of a cell: 0 for the root.
    pub(crate) fn depth(&self, slot: u32) -> u32 {
        let depths = &self.layout().depths;
        depths.get(slot as usize).copied().map_or(0, u32::from)
    }

    /// Whether the cell in `slot` is the cell in `branch` or inside it.
    pub(crate) fn is_within(&self, slot: u32, branch: u32) -> bool {
        let mut at = slot;
        for _ in 0..1024 {
            if at == branch {
                return true;
            }
            match self.entries.get(at as usize) {
                Some(entry) if entry.parent != NONE => at = entry.parent,
                _ => return false,
            }
        }
        false
    }

    /// What the cells around a cell show, from the root down, as
    /// [`wombat_core::links::place_shown`] gives it.
    pub(crate) fn trail(&self, slot: u32) -> Vec<String> {
        let mut trail = Vec::new();
        let mut at = self.entries.get(slot as usize).map_or(NONE, |e| e.parent);
        while let Some(entry) = self.entries.get(at as usize).filter(|_| trail.len() < 1024) {
            trail.push(entry.display().to_owned());
            at = entry.parent;
        }
        trail.reverse();
        trail
    }

    /// Checks that the table proposes every cell for every one of its keys, and that
    /// the cells form one tree. For tests: this is what searches rely on.
    #[doc(hidden)]
    pub fn check(&self) -> Result<(), String> {
        let mut scratch = Vec::new();
        let mut live = 0usize;
        for (slot, entry) in self.entries.iter().enumerate() {
            if !entry.live {
                continue;
            }
            live += 1;
            if self.slot(entry.id) != Some(slot as u32) {
                return Err(format!("cell {} is not found by its ID", entry.id));
            }
            DocSearch::lists(&self.table, entry.folded(), &mut scratch);
            for list in &scratch {
                if !self.table.get(*list).has(slot as u32) {
                    return Err(format!(
                        "cell {} is missing from the list {list} of the table",
                        entry.id
                    ));
                }
            }
            if (entry.parent == NONE) != (slot as u32 == self.root) {
                return Err(format!("cell {} has the wrong parent", entry.id));
            }
        }
        if live != self.slots.len() {
            return Err("the number of cells is wrong".into());
        }
        if self.order().iter().filter(|place| **place != NONE).count() != live {
            return Err("the order does not reach every cell".into());
        }
        Ok(())
    }

    /// Reads a whole document, with the results of its formula cells: those of its
    /// editor ([`wombat_core::Editor::calc`]) for a document that is being edited.
    pub fn with(doc: &Document, calc: &Calc) -> DocSearch {
        let mut index = DocSearch::empty(doc.id());
        let order = doc.cells_in_order();
        index.entries.reserve_exact(order.len());
        index.slots.reserve(order.len());
        for id in &order {
            let read = doc
                .cell(*id)
                .map(|cell| Entry::new(*id, cell, searchable(calc, *id, cell)));
            if let Some(entry) = read {
                index.push(entry);
            }
        }
        for id in &order {
            index.place_children(doc, *id);
        }
        index.root = index.slot(doc.root()).unwrap_or(NONE);
        index.tags = doc.tags().keys().cloned().collect();
        index.compact();
        index
    }

    fn read(doc: &Document, calc: &Calc, id: CellId) -> Option<Entry> {
        let cell = doc.cell(id)?;
        Some(Entry::new(id, cell, searchable(calc, id, cell)))
    }
}

impl Summary for DocSearch {
    const KIND: &'static str = "words";
    /// 2: what formula cells show is kept (the bytes have the same layout as in 1).
    /// 3: the number of columns of each cell's grid is kept.
    const VERSION: u32 = 3;

    fn fingerprint() -> u64 {
        fold::fingerprint()
    }

    /// Works the formulas out, as after loading (without the variables of a run).
    fn of(doc: &Document) -> DocSearch {
        DocSearch::with(doc, &Calc::new(doc))
    }

    fn update(&mut self, doc: &Document, calc: &Calc, changes: &Changes) {
        let mut scratch = Vec::new();
        for id in &changes.removed {
            if let Some(slot) = self.slots.remove(id) {
                self.kill(slot, &mut scratch);
            }
        }
        let mut moved: Vec<CellId> = changes.grids.clone();
        // The cells whose results changed are not in the report (they did not change),
        // but what is kept of them did.
        let named = changes.created.iter().chain(&changes.changed);
        for id in named.chain(calc.recalculated()) {
            match (DocSearch::read(doc, calc, *id), self.slot(*id)) {
                (Some(entry), Some(slot)) => self.replace(slot, entry, &mut scratch),
                (Some(entry), None) => {
                    // A new cell says where it is itself; the cell it is in gives the rest.
                    moved.push(*id);
                    moved.extend(doc.cell(*id).and_then(Cell::parent));
                    self.add(entry, &mut scratch);
                }
                (None, Some(slot)) => {
                    self.slots.remove(id);
                    self.kill(slot, &mut scratch);
                }
                (None, None) => {}
            }
        }
        moved.sort_unstable();
        moved.dedup();
        for id in &moved {
            self.place_children(doc, *id);
        }
        self.root = self.slot(doc.root()).unwrap_or(NONE);
        if let Some(root) = self.entries.get_mut(self.root as usize) {
            root.parent = NONE;
            root.pos = 0;
        }
        for name in &changes.tags {
            if doc.tags().contains_key(name) {
                self.tags.insert(name.clone());
            } else {
                self.tags.remove(name);
            }
        }
        if !(changes.removed.is_empty() && changes.created.is_empty() && moved.is_empty()) {
            self.layout = OnceLock::new();
        }
        self.linking = OnceLock::new();
        // Built again when half of it is stale, or when it has doubled by additions
        // (which are kept less compactly, and by then the lists are too few).
        let all = self.built.saturating_add(self.posted);
        let stale = self.stale >= self.compact_at && self.stale.saturating_mul(2) >= all;
        if stale || self.posted >= self.compact_at.max(self.built) {
            self.compact();
        }
    }

    /// The document ID; the tags; then the cells in document order, each with its ID,
    /// the place in that order of the cell it is in (plus one; 0 for the root), and what
    /// the index keeps of it. The folded forms and the table are not written: they are
    /// made again when this is read, by the rules of the version that reads.
    fn encode(&self) -> Vec<u8> {
        let mut out = Writer::new();
        out.id(self.id.0);
        out.num(self.tags.len() as u64);
        for tag in &self.tags {
            out.text(tag);
        }
        let order = self.order();
        let sequence = self.sequence();
        out.num(sequence.len() as u64);
        for slot in &sequence {
            let Some(entry) = self.entries.get(*slot as usize) else {
                continue;
            };
            out.id(entry.id.0);
            let extra =
                entry.shown().is_some() || entry.image().is_some() || !entry.urls().is_empty();
            let parent = order.get(entry.parent as usize).copied();
            out.num(parent.map_or(0, |place| u64::from(place) + 1));
            out.num(u64::from(entry.cols));
            out.text(&entry.text);
            out.text(entry.note());
            let style = entry.style;
            let bits = [
                style.bold,
                style.italic,
                style.fixed,
                style.underline,
                style.strike,
            ];
            let bits = bits
                .iter()
                .rev()
                .fold(0u8, |all, bit| (all << 1) | u8::from(*bit));
            out.byte(bits | u8::from(entry.has_links) << 5 | u8::from(extra) << 6);
            let kind = CellKind::ALL.iter().position(|kind| *kind == entry.kind);
            out.byte(kind.unwrap_or(0) as u8);
            for color in [entry.cell_color, entry.text_color] {
                out.byte(color.r);
                out.byte(color.g);
                out.byte(color.b);
            }
            match entry.last_edit {
                None => out.byte(0),
                Some(time) => {
                    out.byte(1);
                    out.signed(time);
                }
            }
            if let Some(rare) = entry.rare.as_ref().filter(|_| extra) {
                for text in [&rare.shown, &rare.image] {
                    match text {
                        None => out.byte(0),
                        Some(text) => {
                            out.byte(1);
                            out.text(text);
                        }
                    }
                }
                out.num(rare.urls.len() as u64);
                for url in &rare.urls {
                    out.text(url);
                }
            }
        }
        out.finish()
    }

    fn decode(bytes: &[u8]) -> Option<DocSearch> {
        let mut read = Reader::new(bytes);
        let mut index = DocSearch::empty(DocId(read.id()?));
        for _ in 0..read.count(1)? {
            let tag = read.text()?;
            // Sorted, each once: one form per summary.
            if index.tags.last().is_some_and(|last| last.as_str() >= tag) {
                return None;
            }
            index.tags.insert(tag.to_owned());
        }
        let count = read.count(28)?;
        if count == 0 {
            return None;
        }
        index.entries.reserve_exact(count);
        index.slots.reserve(count);
        // How many cells each cell has had so far: the place of the next one.
        let mut children = vec![0u32; count];
        let mut parents: Vec<u32> = Vec::with_capacity(count);
        for place in 0..count {
            let id = CellId(read.id()?);
            let parent = read.num32()?;
            // The root comes first and only it has no parent; every other cell is
            // inside one that came before it. That makes it a tree.
            if (parent == 0) != (place == 0) || parent as usize > place {
                return None;
            }
            // And in document order: a cell comes right after the cell it is in, or
            // after the last cell of something else inside that cell. (No document is
            // deeper than 32, which also bounds the way up.)
            let mut above = place.checked_sub(1);
            let mut depth = 0u32;
            while let Some(at) = above.filter(|at| at + 1 != parent as usize) {
                above = parents
                    .get(at)
                    .and_then(|parent: &u32| (*parent as usize).checked_sub(1));
                depth += 1;
                if above.is_none() || depth > 64 {
                    return None;
                }
            }
            parents.push(parent);
            let cols = read.num32()?;
            let text = read.text()?.to_owned();
            let note = read.text()?.to_owned();
            let bits = read.byte()?;
            let kind = *CellKind::ALL.get(usize::from(read.byte()?))?;
            let mut colors = [Color::BLACK; 2];
            for color in &mut colors {
                *color = Color::rgb(read.byte()?, read.byte()?, read.byte()?);
            }
            let last_edit = match read.byte()? {
                0 => None,
                1 => Some(read.signed()?),
                _ => return None,
            };
            let mut what = Searchable {
                text,
                note,
                ..Searchable::default()
            };
            if bits & 0x40 != 0 {
                for part in [&mut what.shown, &mut what.image] {
                    *part = match read.byte()? {
                        0 => None,
                        1 => Some(read.text()?.to_owned()),
                        _ => return None,
                    };
                }
                for _ in 0..read.count(1)? {
                    what.urls.push(read.text()?.to_owned());
                }
                if what.shown.is_none() && what.image.is_none() && what.urls.is_empty() {
                    return None;
                }
            }
            if bits & 0x80 != 0 || index.slots.contains_key(&id) {
                return None;
            }
            let mut entry = Entry::of(id, what);
            entry.kind = kind;
            entry.cell_color = colors[0];
            entry.text_color = colors[1];
            entry.last_edit = last_edit;
            entry.style = Style {
                bold: bits & 1 != 0,
                italic: bits & 2 != 0,
                fixed: bits & 4 != 0,
                underline: bits & 8 != 0,
                strike: bits & 16 != 0,
            };
            entry.has_links = bits & 0x20 != 0;
            entry.cols = cols;
            if let Some(above) = parent.checked_sub(1) {
                entry.parent = above;
                let taken = children.get_mut(above as usize)?;
                entry.pos = *taken;
                *taken += 1;
            }
            index.push(entry);
        }
        for (entry, count) in index.entries.iter_mut().zip(&children) {
            entry.has_grid = *count > 0;
            // A grid is rows of as many cells as it has columns.
            let whole = match entry.cols {
                0 => *count == 0,
                cols => *count > 0 && *count % cols == 0,
            };
            if !whole {
                return None;
            }
        }
        index.root = 0;
        if !read.is_empty() {
            return None;
        }
        index.compact();
        Some(index)
    }
}

/// What is kept about one document for searching: the summary for words and filters, and
/// the summary of its links (milestone 4), made together so that a workspace's documents
/// are read once for both.
#[derive(Clone, Debug)]
pub struct DocIndex {
    /// The cells, their texts and properties.
    pub search: DocSearch,
    /// The links.
    pub links: DocLinks,
}

impl DocIndex {
    /// Reads a whole document, with the results of its formula cells
    /// ([`DocSearch::with`]).
    pub fn with(doc: &Document, calc: &Calc) -> DocIndex {
        DocIndex {
            search: DocSearch::with(doc, calc),
            links: DocLinks::of(doc),
        }
    }
}

impl Summary for DocIndex {
    const KIND: &'static str = "search";
    const VERSION: u32 = 1;

    fn fingerprint() -> u64 {
        DocSearch::fingerprint()
            ^ u64::from(DocSearch::VERSION) << 32
            ^ u64::from(DocLinks::VERSION) << 48
    }

    fn of(doc: &Document) -> DocIndex {
        DocIndex::with(doc, &Calc::new(doc))
    }

    fn update(&mut self, doc: &Document, calc: &Calc, changes: &Changes) {
        self.search.update(doc, calc, changes);
        self.links.update(doc, calc, changes);
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Writer::new();
        out.bytes(&self.links.encode());
        out.bytes(&self.search.encode());
        out.finish()
    }

    fn decode(bytes: &[u8]) -> Option<DocIndex> {
        let mut read = Reader::new(bytes);
        let links = DocLinks::decode(read.bytes()?)?;
        let search = DocSearch::decode(read.bytes()?)?;
        // The two are of one document and its cells.
        let same = links.id() == search.id() && links.cell_count() == search.cell_count();
        (read.is_empty() && same).then_some(DocIndex { search, links })
    }
}
