//! What the search index keeps about one document, and how it follows edits (decisions
//! A13, A53, A73).
//!
//! A [`DocSearch`] holds, for every cell: its ID, the cell it is in and its place there,
//! what is searched (the texts as they are and in folded form), and the properties that
//! filters ask about. Over these it keeps a table from every pair of neighbouring folded
//! characters (and every single character of a script written without spaces) to the
//! cells that have it, which is what makes a search look at a few cells instead of all.
//!
//! The table only ever proposes cells; whether a cell matches is always decided by
//! looking at its text. So the table may name too many cells but never too few, and an
//! edit only adds to it: what an edit makes stale is counted, and when half of the table
//! is stale it is built again from the cells (an amount of work that the edits before it
//! pay for). Following a change report costs in proportion to the cells it names, plus
//! the cells directly inside a grid that gained, lost or reordered cells.

use std::collections::{BTreeSet, HashMap};
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::OnceLock;

use wombat_core::links::DocLinks;
use wombat_core::summary::{Reader, Summary, Writer};
use wombat_core::{Cell, CellId, CellKind, Changes, Color, DocId, Document, LinkTarget, Style};

use crate::fold::{self, fold_into};

/// "No cell": the parent of the root.
pub(crate) const NONE: u32 = u32::MAX;
/// In place of a list of the table: so many cells have the key that the list would
/// say little, and is not kept (see [`DocSearch::common_at`]).
const COMMON: u32 = u32::MAX;

/// What the table says about a key.
pub(crate) enum Proposed<'a> {
    /// No cell has it.
    None,
    /// Too many cells have it to list them: any cell may.
    Any,
    /// These cells may have it; no other does.
    Some(&'a [u32]),
}

/// What of a cell is searched. **Every reader of a cell's searchable content goes
/// through [`searchable`]**, the index and the reference scanner alike, so that there is
/// one place to extend.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Searchable {
    /// The text of the cell, as stored. Ranges of a match in the text refer to this.
    pub text: String,
    /// What the cell displays, where that is not its stored text: the result of a
    /// formula. Nothing computes one yet (milestone 5); when something does, it is
    /// returned here and is searched as a field of its own ([`crate::Field::Shown`]).
    pub shown: Option<String>,
    /// The note.
    pub note: String,
    /// The file name of the image.
    pub image: Option<String>,
    /// The external targets of the cell's links (URLs and file paths), in text order.
    pub urls: Vec<String>,
}

/// What of `cell` is searched: see [`Searchable`]. `doc` is the cell's document, for
/// what milestone 5 will need to find a displayed result.
pub fn searchable(_doc: &Document, cell: &Cell) -> Searchable {
    let urls = cell.links.iter().filter_map(|link| match &link.target {
        LinkTarget::External(target) => Some(target.clone()),
        _ => None,
    });
    Searchable {
        text: cell.text.clone(),
        shown: None,
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

/// The keys of the table for a folded form: one per pair of neighbouring characters
/// within a field, and one per character of a script written without spaces (so that a
/// search for one such character need not look at every cell). Sorted, each once.
pub(crate) fn keys(folded: &str, out: &mut Vec<u64>) {
    out.clear();
    let mut before: Option<char> = None;
    for c in folded.chars() {
        if c == '\n' {
            before = None;
            continue;
        }
        if single_key(c) {
            out.push(u64::from(c));
        }
        if let Some(before) = before {
            out.push(pair_key(before, c));
        }
        before = Some(c);
    }
    out.sort_unstable();
    out.dedup();
}

pub(crate) fn single_key(c: char) -> bool {
    u32::from(c) >= 0x2e80
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
    table: FastMap<u64, Vec<u32>>,
    /// How many entries of the table were added, and how many of them are known stale.
    posted: usize,
    stale: usize,
    compact_at: usize,
    /// A key that this many cells have is common: its list is dropped, and a search
    /// for something made of common keys only looks at every cell. That costs such a
    /// search little (it would have looked at a large part of the cells anyway) and
    /// saves most of the table, which is mostly the lists of a few very common keys.
    /// An eighth of the cells when the table was last built, and at least 64.
    common_at: usize,
    common_floor: usize,
    tags: BTreeSet<String>,
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

    /// The cells that the table proposes for a key; `None` if no cell has the key.
    pub(crate) fn proposed(&self, key: u64) -> Proposed<'_> {
        match self.table.get(&key).map(Vec::as_slice) {
            None => Proposed::None,
            Some([COMMON, ..]) => Proposed::Any,
            Some(list) => Proposed::Some(list),
        }
    }

    /// How many slots there are, with or without a cell.
    pub(crate) fn slot_count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Sets after how many stale entries the table is built again (when they are also
    /// half of it), and the fewest cells that make a key common, and builds the table
    /// again. For tests, which want to see both happen in small documents.
    #[doc(hidden)]
    pub fn set_thresholds(&mut self, stale: usize, common: usize) {
        self.compact_at = stale;
        self.common_floor = common;
        self.compact();
    }

    /// How many times more entries the table has than a table built now would have,
    /// roughly: (entries added, entries known to be stale).
    #[doc(hidden)]
    pub fn table_load(&self) -> (usize, usize) {
        (self.posted, self.stale)
    }

    /// The memory the index of this document takes, in bytes, as far as it can be
    /// counted: the texts and their folded forms, the per-cell records, and the tables.
    pub fn memory_bytes(&self) -> usize {
        let entries = self.entries.capacity() * std::mem::size_of::<Entry>();
        let texts: usize = self.entries.iter().map(Entry::bytes).sum();
        // A hash table keeps about one byte of bookkeeping per place and is at most 7/8 full.
        let slots = self.slots.capacity() * (std::mem::size_of::<(CellId, u32)>() + 1);
        let table = self.table.capacity() * (std::mem::size_of::<(u64, Vec<u32>)>() + 1);
        let lists: usize = self.table.values().map(|list| list.capacity() * 4).sum();
        let order = self
            .layout
            .get()
            .map_or(0, |layout| layout.places.capacity() * 5);
        entries + texts + slots + table + lists + order + self.free.capacity() * 4
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
            table: FastMap::default(),
            posted: 0,
            stale: 0,
            compact_at: 4096,
            common_at: 64,
            common_floor: 64,
            tags: BTreeSet::new(),
            layout: OnceLock::new(),
        }
    }

    /// Adds a slot to the list of a key, unless the key is common or becomes so now.
    /// The same slot is not added twice in a row. Returns the change in the number of
    /// entries of the table.
    fn post_one(
        table: &mut FastMap<u64, Vec<u32>>,
        key: u64,
        slot: u32,
        common_at: usize,
    ) -> isize {
        let list = table.entry(key).or_default();
        if list.first() == Some(&COMMON) || list.last() == Some(&slot) {
            0
        } else if list.len() >= common_at {
            let dropped = list.len() as isize;
            *list = vec![COMMON];
            -dropped
        } else {
            list.push(slot);
            1
        }
    }

    fn post(&mut self, slot: u32, keys: &[u64]) {
        let mut posted = 0isize;
        for key in keys {
            posted += DocSearch::post_one(&mut self.table, *key, slot, self.common_at);
        }
        self.posted = self.posted.saturating_add_signed(posted);
    }

    /// Adds a slot to the table under every key of a folded form, and returns the
    /// change in the number of entries. While one cell is added its entries are the
    /// last of their lists, so no list gets it twice.
    fn post_all(
        table: &mut FastMap<u64, Vec<u32>>,
        slot: u32,
        folded: &str,
        common_at: usize,
    ) -> isize {
        let mut posted = 0isize;
        let mut post = |key: u64| posted += DocSearch::post_one(table, key, slot, common_at);
        let mut before: Option<char> = None;
        for c in folded.chars() {
            if c == '\n' {
                before = None;
                continue;
            }
            if single_key(c) {
                post(u64::from(c));
            }
            if let Some(before) = before {
                post(pair_key(before, c));
            }
            before = Some(c);
        }
        posted
    }

    /// Puts a cell in a new slot.
    fn add(&mut self, entry: Entry) -> u32 {
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
        let folded = self
            .entries
            .get(slot as usize)
            .map_or("", |entry| entry.folded());
        let posted = DocSearch::post_all(&mut self.table, slot, folded, self.common_at);
        self.posted = self.posted.saturating_add_signed(posted);
        slot
    }

    /// Puts new content in the slot of a cell that changed, keeping its place.
    fn replace(&mut self, slot: u32, mut entry: Entry, scratch: &mut Vec<u64>) {
        let Some(old) = self.entries.get_mut(slot as usize) else {
            return;
        };
        let mut old_keys = Vec::new();
        keys(old.folded(), &mut old_keys);
        keys(entry.folded(), scratch);
        entry.parent = old.parent;
        entry.pos = old.pos;
        *old = entry;
        // Only what is new is added; what is no longer there stays, as stale.
        let fresh: Vec<u64> = scratch
            .iter()
            .filter(|key| old_keys.binary_search(key).is_err())
            .copied()
            .collect();
        let gone = old_keys
            .iter()
            .filter(|key| scratch.binary_search(key).is_err());
        self.stale = self.stale.saturating_add(gone.count());
        self.post(slot, &fresh);
    }

    fn kill(&mut self, slot: u32, scratch: &mut Vec<u64>) {
        let Some(entry) = self.entries.get_mut(slot as usize) else {
            return;
        };
        keys(entry.folded(), scratch);
        self.stale = self.stale.saturating_add(scratch.len());
        entry.live = false;
        entry.text = Box::default();
        entry.folded = None;
        entry.rare = None;
        self.free.push(slot);
    }

    fn common_for(&self, cells: usize) -> usize {
        (cells / 8).max(self.common_floor)
    }

    /// Builds the table again from the cells, without what is stale.
    fn compact(&mut self) {
        let mut table: FastMap<u64, Vec<u32>> = FastMap::default();
        let mut posted = 0isize;
        self.common_at = self.common_for(self.slots.len());
        for (slot, entry) in self.entries.iter().enumerate() {
            if entry.live {
                posted +=
                    DocSearch::post_all(&mut table, slot as u32, entry.folded(), self.common_at);
            }
        }
        let posted = posted.max(0) as usize;
        for list in table.values_mut() {
            list.shrink_to_fit();
        }
        self.table = table;
        self.posted = posted;
        self.stale = 0;
    }

    /// Makes the cells of the grid of `cell` know where they are.
    fn place_children(&mut self, doc: &Document, cell: CellId) {
        let (Some(slot), Some(found)) = (self.slot(cell), doc.cell(cell)) else {
            return;
        };
        if let Some(entry) = self.entries.get_mut(slot as usize) {
            entry.has_grid = found.grid().is_some();
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

    /// The texts of the cells around a cell, from the root down, as
    /// [`wombat_core::links::place`] gives them.
    pub(crate) fn trail(&self, slot: u32) -> Vec<String> {
        let mut trail = Vec::new();
        let mut at = self.entries.get(slot as usize).map_or(NONE, |e| e.parent);
        while let Some(entry) = self.entries.get(at as usize).filter(|_| trail.len() < 1024) {
            trail.push(entry.text.to_string());
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
            keys(entry.folded(), &mut scratch);
            for key in &scratch {
                let list = self.table.get(key).map_or(&[][..], Vec::as_slice);
                if list.first() != Some(&COMMON) && !list.contains(&(slot as u32)) {
                    return Err(format!(
                        "cell {} is missing under the key {key:x}",
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

    /// [`Summary::of`] with another reading of what a cell's searchable content is
    /// than [`searchable`]. For tests of the displayed-result field, which nothing
    /// fills before milestone 5; an index made this way cannot follow edits.
    #[doc(hidden)]
    pub fn of_with(doc: &Document, what: &dyn Fn(&Document, &Cell) -> Searchable) -> DocSearch {
        let mut index = DocSearch::empty(doc.id());
        let order = doc.cells_in_order();
        index.common_at = index.common_for(order.len());
        index.entries.reserve_exact(order.len());
        index.slots.reserve(order.len());
        for id in &order {
            let read = doc
                .cell(*id)
                .map(|cell| Entry::new(*id, cell, what(doc, cell)));
            if let Some(entry) = read {
                index.add(entry);
            }
        }
        for id in &order {
            index.place_children(doc, *id);
        }
        index.root = index.slot(doc.root()).unwrap_or(NONE);
        index.tags = doc.tags().keys().cloned().collect();
        for list in index.table.values_mut() {
            list.shrink_to_fit();
        }
        index
    }

    fn read(doc: &Document, id: CellId) -> Option<Entry> {
        let cell = doc.cell(id)?;
        Some(Entry::new(id, cell, searchable(doc, cell)))
    }
}

impl Summary for DocSearch {
    const KIND: &'static str = "words";
    const VERSION: u32 = 1;

    fn fingerprint() -> u64 {
        fold::fingerprint()
    }

    fn of(doc: &Document) -> DocSearch {
        DocSearch::of_with(doc, &searchable)
    }

    fn update(&mut self, doc: &Document, changes: &Changes) {
        let mut scratch = Vec::new();
        for id in &changes.removed {
            if let Some(slot) = self.slots.remove(id) {
                self.kill(slot, &mut scratch);
            }
        }
        let mut moved: Vec<CellId> = changes.grids.clone();
        for id in changes.created.iter().chain(&changes.changed) {
            match (DocSearch::read(doc, *id), self.slot(*id)) {
                (Some(entry), Some(slot)) => self.replace(slot, entry, &mut scratch),
                (Some(entry), None) => {
                    // A new cell says where it is itself; the cell it is in gives the rest.
                    moved.push(*id);
                    moved.extend(doc.cell(*id).and_then(Cell::parent));
                    self.add(entry);
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
        if self.stale >= self.compact_at && self.stale.saturating_mul(2) >= self.posted {
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
        let count = read.count(27)?;
        if count == 0 {
            return None;
        }
        index.entries.reserve_exact(count);
        index.slots.reserve(count);
        index.common_at = index.common_for(count);
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
            if let Some(above) = parent.checked_sub(1) {
                entry.parent = above;
                let taken = children.get_mut(above as usize)?;
                entry.pos = *taken;
                *taken += 1;
            }
            index.add(entry);
        }
        for (entry, count) in index.entries.iter_mut().zip(&children) {
            entry.has_grid = *count > 0;
        }
        index.root = 0;
        for list in index.table.values_mut() {
            list.shrink_to_fit();
        }
        read.is_empty().then_some(index)
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

impl Summary for DocIndex {
    const KIND: &'static str = "search";
    const VERSION: u32 = 1;

    fn fingerprint() -> u64 {
        DocSearch::fingerprint()
            ^ u64::from(DocSearch::VERSION) << 32
            ^ u64::from(DocLinks::VERSION) << 48
    }

    fn of(doc: &Document) -> DocIndex {
        DocIndex {
            search: DocSearch::of(doc),
            links: DocLinks::of(doc),
        }
    }

    fn update(&mut self, doc: &Document, changes: &Changes) {
        self.search.update(doc, changes);
        self.links.update(doc, changes);
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
