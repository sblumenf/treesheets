//! The results of the formula cells of one document, kept up to date (decisions A60-A65).
//!
//! In TreeSheets `Document::RecalcFormulas` works every formula out again after every
//! change and writes the result into the cell's text. Here a formula cell's text stays its
//! formula, and [`Calc`] holds what each one shows. A result is never saved: it follows
//! from the document and from the variables of the last run.
//!
//! **Order.** Formulas are worked out in document order, each one after the formulas whose
//! results it reads, exactly in the order TreeSheets reaches them, because where formulas
//! refer to each other in a circle that order decides which of them says `#CYCLE!`. Where
//! TreeSheets calls itself for a formula inside a formula, this keeps its own list of the
//! formulas in progress, so a chain of ten thousand formulas costs no stack: an attempt
//! that meets a result it does not have yet stops, the missing ones are worked out, and
//! the attempt is made again (what it had already read is remembered).
//!
//! **After a change** ([`Calc::update`], called by the editor with every change report):
//! if only cells without a grid changed, the formulas that read them, and those that read
//! these, are worked out again and nothing else. A name in a formula means the nearest
//! cell with that text that has a grid, so anything that adds, removes or moves cells, or
//! changes the text of a cell with a grid, can change what a name means: then everything is
//! worked out again. So is everything when a circle or the limit on work is involved, where
//! the order matters.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use crate::edit::Changes;
use crate::id::CellId;
use crate::model::{CellKind, Document};

use super::formula::{self, formula_of, Check, Collected, Contents, Env, Memo, Outcome, Wait};
use super::num::strict_num;
use super::run::{Failure, Value, Variables};

/// Most work that the formulas of one document may take in one recalculation; formulas
/// beyond it show `#LIMIT!` (decisions B52, A81). Work is: one for every cell of a grid
/// that is read (a table that was read before in the same recalculation counts one), one
/// for every 16 bytes of text made, a little for every name in a long formula, and
/// [`FORMULA_WORK`] for every formula. Measured, one unit of work is at most about 0.4
/// microseconds, so the limit is just under a second.
pub const MAX_WORK: u64 = 2_000_000;
/// The work that every formula counts for, whatever it does: finding it, reading it and
/// keeping its result takes about as long as reading six cells; and one more for every 32
/// bytes of its text.
pub const FORMULA_WORK: u64 = 6;

/// What a formula cell shows.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Res {
    text: String,
    failed: bool,
    /// The cells read to work it out.
    cost: u64,
}

/// The places of the cells of a document, for finding what a name stands for.
#[derive(Clone, Debug, Default)]
struct Index {
    /// The place of each cell in document order.
    place: HashMap<CellId, u32>,
    order: Vec<CellId>,
    /// By place: the place of the parent (the root: itself), the depth, and the place just
    /// after the last cell inside.
    parent: Vec<u32>,
    depth: Vec<u32>,
    end: Vec<u32>,
    /// The cells with a grid that a name can stand for, by text, in document order; and
    /// what each cell with a grid had as text and kind.
    by_text: HashMap<String, Vec<u32>>,
    grid_cells: HashMap<CellId, (String, CellKind)>,
    /// The formula cells with a grid: their text is their result.
    fgrid: Vec<u32>,
    /// The names that the variable-assign cells of the document assign to, and those cells
    /// with the cells of their grids.
    assigned: BTreeSet<String>,
    assign_cells: HashSet<CellId>,
}

impl Index {
    /// One walk over the document, in document order, with one look at each cell: this is
    /// what working everything out costs before any formula is read (decision A81).
    fn build(doc: &Document) -> Index {
        let count = doc.cell_count();
        let mut index = Index {
            place: HashMap::with_capacity(count),
            parent: Vec::with_capacity(count),
            depth: Vec::with_capacity(count),
            end: Vec::with_capacity(count),
            ..Index::default()
        };
        let mut order = Vec::with_capacity(count);
        // A cell, the place of its parent (none for the root), and its depth.
        let mut stack: Vec<(CellId, Option<u32>, u32)> = vec![(doc.root(), None, 0)];
        while let Some((id, parent, depth)) = stack.pop() {
            // A valid document never revisits a cell; this keeps a broken one from looping.
            if order.len() >= count {
                break;
            }
            let Some(cell) = doc.cell(id) else { continue };
            let here = u32::try_from(order.len()).unwrap_or(u32::MAX);
            order.push(id);
            index.place.insert(id, here);
            index.parent.push(parent.unwrap_or(here));
            index.depth.push(depth);
            index.end.push(here.saturating_add(1));
            if cell.kind == CellKind::VarAssign {
                index.assign_cells.insert(id);
                if !cell.text.is_empty() {
                    index.assigned.insert(cell.text.clone());
                }
                for child in cell.grid().map_or(&[][..], |g| g.cells()) {
                    index.assign_cells.insert(*child);
                    if let Some(c) = doc.cell(*child).filter(|c| !c.text.is_empty()) {
                        index.assigned.insert(c.text.clone());
                    }
                }
            }
            if let Some(grid) = cell.grid() {
                index.grid_cells.insert(id, (cell.text.clone(), cell.kind));
                match cell.kind {
                    CellKind::VarAssign => {}
                    CellKind::Formula => index.fgrid.push(here),
                    _ => index
                        .by_text
                        .entry(cell.text.clone())
                        .or_default()
                        .push(here),
                }
                let inside = depth.saturating_add(1);
                stack.extend(grid.cells().iter().rev().map(|c| (*c, Some(here), inside)));
            }
        }
        // The end of a cell is the end of the last cell inside it: later cells first.
        for n in (0..order.len()).rev() {
            let own = u32::try_from(n).unwrap_or(u32::MAX).saturating_add(1);
            let end = index.end.get(n).copied().unwrap_or(0).max(own);
            if let Some(e) = index.end.get_mut(n) {
                *e = end;
            }
            let parent = index.parent.get(n).copied().unwrap_or(0) as usize;
            if parent != n {
                if let Some(e) = index.end.get_mut(parent) {
                    *e = (*e).max(end);
                }
            }
        }
        index.order = order;
        index
    }

    fn place(&self, id: CellId) -> Option<u32> {
        self.place.get(&id).copied()
    }

    /// The part of an ascending list of places that lies inside the cell at `scope`.
    fn inside<'a>(&self, places: &'a [u32], scope: u32) -> &'a [u32] {
        let end = self.end.get(scope as usize).copied().unwrap_or(scope);
        let from = places.partition_point(|p| *p < scope);
        let to = places.partition_point(|p| *p < end);
        places.get(from..to).unwrap_or_default()
    }

    /// Depth, then place: the order of TreeSheets' search (`ForNearest`), breadth first.
    fn key(&self, place: u32) -> (u32, u32) {
        (self.depth.get(place as usize).copied().unwrap_or(0), place)
    }
}

/// Which formulas read what, to know what a change affects.
#[derive(Clone, Debug, Default)]
struct Readers {
    /// By the cell that has a grid: the formulas that read the cells of that grid.
    children: HashMap<CellId, BTreeSet<CellId>>,
    /// By cell: the formulas that read everything inside it.
    tree: HashMap<CellId, BTreeSet<CellId>>,
    /// By formula: the keys it is listed under in the two maps above.
    of: HashMap<CellId, (Vec<CellId>, Vec<CellId>)>,
}

impl Readers {
    fn register(&mut self, formula: CellId, mut children: Vec<CellId>, mut tree: Vec<CellId>) {
        if children.is_empty() && tree.is_empty() {
            // Most formulas of a large document: nothing to keep, nothing to forget.
            return;
        }
        children.sort_unstable();
        children.dedup();
        tree.sort_unstable();
        tree.dedup();
        for c in &children {
            self.children.entry(*c).or_default().insert(formula);
        }
        for c in &tree {
            self.tree.entry(*c).or_default().insert(formula);
        }
        self.of.insert(formula, (children, tree));
    }

    fn forget(&mut self, formula: CellId) {
        let Some((children, tree)) = self.of.remove(&formula) else {
            return;
        };
        for c in children {
            if let Some(set) = self.children.get_mut(&c) {
                set.remove(&formula);
                if set.is_empty() {
                    self.children.remove(&c);
                }
            }
        }
        for c in tree {
            if let Some(set) = self.tree.get_mut(&c) {
                set.remove(&formula);
                if set.is_empty() {
                    self.tree.remove(&c);
                }
            }
        }
    }
}

/// How the last update was done, for tests and measurements.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Stats {
    /// Whether every formula was worked out again.
    pub full: bool,
    /// How many formulas were worked out.
    pub evaluated: usize,
    /// The work that took (see [`MAX_WORK`]), with that of an attempt to work out only
    /// some formulas that had to be given up for all of them.
    pub work: u64,
}

/// What the last run of the program reported.
#[derive(Clone, PartialEq, Debug)]
pub struct RunReport {
    /// The first thing that went wrong, if anything.
    pub failure: Option<Failure>,
    /// The value of the root cell: what TreeSheets' `ts.evaluate` returns.
    pub result: Option<Value>,
}

/// The results of the formula cells of a document, and the variables of its last run.
#[derive(Clone, Debug)]
pub struct Calc {
    formulas: HashSet<CellId>,
    results: HashMap<CellId, Res>,
    variables: Variables,
    variables_changed: bool,
    index: Option<Index>,
    readers: Readers,
    /// Whether formulas in a circle were met, or the limit on work: results then depend on
    /// the order, and only a full recalculation is right.
    tainted: bool,
    over: bool,
    total_cost: u64,
    budget: u64,
    recalculated: Vec<CellId>,
    group: Option<BTreeSet<CellId>>,
    stats: Stats,
    last_run: Option<RunReport>,
}

/// A cell that a formula reads: of the document, or of a variable's value.
#[derive(Clone, Copy, Debug)]
enum Node<'a> {
    Doc(CellId),
    Var(&'a Value),
}

/// What one attempt at one formula sees.
struct Ctx<'a, 'r> {
    doc: &'a Document,
    index: &'a Index,
    variables: &'a Variables,
    results: &'r HashMap<CellId, Res>,
    busy: &'r HashSet<CellId>,
    me: CellId,
    /// Formulas whose results are missing; while there are any the attempt only looks for
    /// more of them.
    need: Vec<CellId>,
    /// The cost of the formula so far, and what is left for it. The cost is the same
    /// however often the formula had to wait for others: what is done while only looking
    /// for missing formulas is not counted, and is done again, and counted, afterwards.
    work: u64,
    budget: u64,
    /// The contents of the grids read in this recalculation, to read each once.
    cache: &'r mut Cache<'a>,
    over: bool,
    tainted: bool,
    reads_children: Vec<CellId>,
    reads_tree: Vec<CellId>,
}

enum Known {
    Yes,
    Failed(String),
    No,
}

impl<'a> Ctx<'a, '_> {
    fn spend(&mut self, n: u64) {
        if !self.need.is_empty() || self.over {
            return;
        }
        self.work = self.work.saturating_add(n);
        if self.work > self.budget {
            // Nothing is read or made from here on, so this is all it cost.
            self.work = self.budget.saturating_add(1);
            self.over = true;
        }
    }

    /// Counts the reading of `cells` cells; an error if that is more than is allowed, so
    /// that they are then not read at all.
    fn charge(&mut self, cells: u64) -> Check {
        self.spend(cells);
        if self.over {
            return Err("#LIMIT!".to_string());
        }
        Ok(())
    }

    /// The number of cells inside a cell of the document, itself included.
    fn extent(&self, id: CellId) -> u64 {
        let index = self.index;
        index.place(id).map_or(1, |place| {
            let end = index.end.get(place as usize).copied().unwrap_or(place);
            u64::from(end.saturating_sub(place)).max(1)
        })
    }

    /// Whether the formula's own cell is among the cells of the grid of `id` (with `tree`:
    /// anywhere inside). Its contents are then its own and not for other formulas.
    fn within(&self, id: CellId, tree: bool) -> bool {
        let index = self.index;
        let (Some(me), Some(place)) = (index.place(self.me), index.place(id)) else {
            return true;
        };
        if tree {
            let end = index.end.get(place as usize).copied().unwrap_or(place);
            place < me && me < end
        } else {
            index.parent.get(me as usize).copied() == Some(place)
        }
    }

    /// `FormulaEvaluator::Update` for a formula cell other than the one being worked out.
    fn update(&mut self, id: CellId) -> Known {
        if let Some(res) = self.results.get(&id) {
            if res.failed {
                Known::Failed(res.text.clone())
            } else {
                Known::Yes
            }
        } else if self.busy.contains(&id) {
            self.tainted = true;
            Known::Failed("#CYCLE!".to_string())
        } else {
            self.need.push(id);
            Known::No
        }
    }

    fn shown(&self, id: CellId) -> &str {
        match self.doc.cell(id) {
            Some(cell) if cell.kind == CellKind::Formula => {
                self.results.get(&id).map_or("", |r| r.text.as_str())
            }
            Some(cell) => cell.text.as_str(),
            None => "",
        }
    }

    fn add(text: &str, node: Node<'a>, into: &mut Contents<Node<'a>>) {
        if let Some(d) = strict_num(text) {
            into.numbers.push(d);
            into.number = Some(node);
        } else if !text.is_empty() {
            into.text = Some(node);
            into.texts = into.texts.saturating_add(1);
        }
    }

    fn collect_doc(
        &mut self,
        id: CellId,
        tree: bool,
        sub: bool,
        into: &mut Contents<Node<'a>>,
    ) -> Check {
        let Some(cell) = self.doc.cell(id) else {
            return Ok(());
        };
        let formula = cell.kind == CellKind::Formula;
        if id != self.me && !(sub && formula) {
            let known = if formula { self.update(id) } else { Known::Yes };
            match known {
                Known::Failed(e) => return Err(e),
                Known::Yes => Ctx::add(self.shown(id), Node::Doc(id), into),
                Known::No => {}
            }
        }
        if tree {
            for child in cell.grid().map_or(&[][..], |g| g.cells()) {
                self.collect_doc(*child, true, true, into)?;
            }
        }
        Ok(())
    }

    /// The same for a cell of a variable's value. A formula cell in there counts by the
    /// result it had when the program ran (decision B54).
    fn collect_value(
        &mut self,
        v: &'a Value,
        tree: bool,
        sub: bool,
        into: &mut Contents<Node<'a>>,
        level: u32,
    ) -> Check {
        if self.over {
            return Err("#LIMIT!".to_string());
        }
        self.spend(1);
        if !(sub && v.kind == CellKind::Formula) {
            Ctx::add(&v.text, Node::Var(v), into);
        }
        if tree && level < super::MAX_VALUE_DEPTH {
            for child in v.grid.as_deref().map_or(&[][..], |g| g.cells.as_slice()) {
                self.collect_value(child, true, true, into, level.saturating_add(1))?;
            }
        }
        Ok(())
    }

    /// The cell with a grid nearest to the formula's cell whose text is `name`
    /// (`ForNearest` in `FormulaEvaluator::Named`).
    fn nearest(&mut self, name: &str) -> Option<CellId> {
        let index = self.index;
        let me = index.place(self.me)?;
        let mut scope = index.parent.get(me as usize).copied()?;
        if scope == me {
            return None; // the root is in no grid
        }
        loop {
            let named = index.by_text.get(name).map_or(&[][..], |v| v.as_slice());
            let named = index.inside(named, scope);
            self.spend(named.len() as u64);
            let best = named.iter().map(|p| index.key(*p)).min();
            // Formula cells with a grid that the search reaches before that one: their
            // text is their result.
            let mut formulas: Vec<(u32, u32)> = index
                .inside(&index.fgrid, scope)
                .iter()
                .map(|p| index.key(*p))
                .filter(|key| best.is_none_or(|b| *key < b))
                .collect();
            formulas.sort_unstable();
            self.spend(formulas.len() as u64);
            for (_, place) in formulas {
                let Some(id) = index.order.get(place as usize).copied() else {
                    continue;
                };
                if id == self.me || self.busy.contains(&id) {
                    // Its result is not there yet, so it has no name (decision B51).
                    self.tainted = true;
                    continue;
                }
                match self.results.get(&id) {
                    Some(res) if res.text == name => return Some(id),
                    Some(_) => {}
                    None => {
                        self.need.push(id);
                        return None;
                    }
                }
            }
            if let Some((_, place)) = best {
                return index.order.get(place as usize).copied();
            }
            let up = index.parent.get(scope as usize).copied()?;
            if up == scope {
                return None;
            }
            scope = up;
        }
    }
}

impl<'a> Env for Ctx<'a, '_> {
    type Node = Node<'a>;

    fn named(&mut self, name: &str) -> Result<Node<'a>, String> {
        if self.over {
            return Err("#LIMIT!".to_string());
        }
        if let Some(id) = self.nearest(name) {
            return Ok(Node::Doc(id));
        }
        if let Some(v) = self.variables.get(name) {
            return Ok(Node::Var(v));
        }
        Err(if self.index.assigned.contains(name) {
            "#N/A"
        } else {
            "#NAME?"
        }
        .to_string())
    }

    fn size(&self, node: Node<'a>) -> Option<(u32, u32)> {
        match node {
            Node::Doc(id) => self.doc.cell(id)?.grid().map(|g| (g.xs(), g.ys())),
            Node::Var(v) => v.grid.as_deref().map(super::run::ValueGrid::size),
        }
    }

    fn child(&self, node: Node<'a>, x: u32, y: u32) -> Option<Node<'a>> {
        match node {
            Node::Doc(id) => self.doc.child(id, x, y).map(Node::Doc),
            Node::Var(v) => v.grid.as_deref()?.at(x, y).map(Node::Var),
        }
    }

    fn collect(
        &mut self,
        node: Node<'a>,
        tree: bool,
        sub: bool,
        into: &mut Contents<Node<'a>>,
    ) -> Check {
        match node {
            Node::Doc(id) => {
                if tree {
                    self.reads_tree.push(id);
                }
                self.charge(if tree { self.extent(id) } else { 1 })?;
                self.collect_doc(id, tree, sub, into)
            }
            Node::Var(v) => self.collect_value(v, tree, sub, into, 0),
        }
    }

    fn collect_grid(&mut self, node: Node<'a>, tree: bool) -> Collected<Node<'a>> {
        let mut into = Contents::default();
        match node {
            Node::Doc(id) => {
                if tree {
                    self.reads_tree.push(id);
                } else {
                    self.reads_children.push(id);
                }
                // What another formula read here before in this recalculation is what
                // this one would read, unless its own cell is in there.
                let shared = !self.within(id, tree);
                if shared {
                    if let Some(known) = self.cache.get(&(id, tree)) {
                        let known = known.clone();
                        self.charge(1)?;
                        return known;
                    }
                }
                let cells = self.doc.cell(id).and_then(|c| c.grid());
                let cells = cells.map_or(&[][..], |g| g.cells());
                self.charge(if tree {
                    self.extent(id).saturating_sub(1)
                } else {
                    cells.len() as u64
                })?;
                let clean = !self.tainted;
                let mut result = Ok(());
                for child in cells {
                    result = self.collect_doc(*child, tree, false, &mut into);
                    if result.is_err() {
                        break;
                    }
                }
                let result = result.map(|()| Rc::new(into));
                // Not if it met a formula that is not worked out yet, or one in progress
                // (what that one shows depends on who asks).
                if shared && clean && !self.tainted && self.need.is_empty() {
                    self.cache.insert((id, tree), result.clone());
                }
                result
            }
            Node::Var(v) => {
                for child in v.grid.as_deref().map_or(&[][..], |g| g.cells.as_slice()) {
                    self.collect_value(child, tree, false, &mut into, 1)?;
                }
                Ok(Rc::new(into))
            }
        }
    }

    fn read_grid(&mut self, node: Node<'a>) {
        if let Node::Doc(id) = node {
            self.reads_children.push(id);
        }
    }

    fn text(&self, node: Node<'a>) -> &str {
        match node {
            Node::Doc(id) => self.shown(id),
            Node::Var(v) => &v.text,
        }
    }

    fn waiting(&self) -> bool {
        !self.need.is_empty()
    }

    fn work(&self) -> u64 {
        self.work
    }

    fn add_work(&mut self, work: u64) -> bool {
        self.spend(work);
        !self.over
    }
}

/// What reading the grid of a cell gave in this recalculation, without or with everything
/// inside.
type Cache<'a> = HashMap<(CellId, bool), Collected<Node<'a>>>;

/// A formula in progress.
struct Frame<'a> {
    cell: CellId,
    /// The formulas it waits for, and how many of them are done.
    need: Vec<CellId>,
    next: usize,
    memo: Vec<Memo<Node<'a>>>,
    reads_children: Vec<CellId>,
    reads_tree: Vec<CellId>,
}

/// One recalculation, of all formulas or of some.
struct Pass<'a> {
    doc: &'a Document,
    index: &'a Index,
    variables: &'a Variables,
    results: &'a mut HashMap<CellId, Res>,
    readers: &'a mut Readers,
    busy: HashSet<CellId>,
    /// The cost of the formulas worked out so far.
    done: u64,
    budget: u64,
    over: bool,
    tainted: bool,
    evaluated: usize,
    cache: Cache<'a>,
}

impl<'a> Pass<'a> {
    fn new(
        doc: &'a Document,
        index: &'a Index,
        variables: &'a Variables,
        results: &'a mut HashMap<CellId, Res>,
        readers: &'a mut Readers,
        budget: u64,
    ) -> Pass<'a> {
        Pass {
            doc,
            index,
            variables,
            results,
            readers,
            busy: HashSet::new(),
            done: 0,
            budget,
            over: false,
            tainted: false,
            evaluated: 0,
            cache: HashMap::new(),
        }
    }

    /// Brings the result of formula cell `start` up to date, after those it reads.
    fn ensure(&mut self, start: CellId) {
        if self.results.contains_key(&start) {
            return;
        }
        if self.over {
            // Nothing is worked out any more: no more than writing the result down.
            let outcome = limit();
            self.results.insert(
                start,
                Res {
                    text: outcome.text,
                    failed: outcome.failed,
                    cost: 0,
                },
            );
            self.evaluated = self.evaluated.saturating_add(1);
            return;
        }
        let frame = |cell| Frame {
            cell,
            need: Vec::new(),
            next: 0,
            memo: Vec::new(),
            reads_children: Vec::new(),
            reads_tree: Vec::new(),
        };
        let mut stack: Vec<Frame<'a>> = vec![frame(start)];
        self.busy.insert(start);
        while let Some(top) = stack.last_mut() {
            if let Some(wanted) = top.need.get(top.next).copied() {
                match self.results.get(&wanted) {
                    // TreeSheets stops reading a grid at the first formula in it that
                    // failed, so the ones after it are not for now.
                    Some(res) if res.failed => top.next = top.need.len(),
                    Some(_) => top.next = top.next.saturating_add(1),
                    None if self.busy.contains(&wanted) => top.next = top.need.len(),
                    None => {
                        self.busy.insert(wanted);
                        stack.push(frame(wanted));
                    }
                }
                continue;
            }
            let cell = top.cell;
            let outcome = if self.over {
                Ok((limit(), 0))
            } else {
                self.attempt(top)
            };
            match outcome {
                Ok((outcome, cost)) => {
                    self.results.insert(
                        cell,
                        Res {
                            text: outcome.text,
                            failed: outcome.failed,
                            cost,
                        },
                    );
                    self.busy.remove(&cell);
                    self.done = self.done.saturating_add(cost);
                    self.evaluated = self.evaluated.saturating_add(1);
                    if let Some(done) = stack.pop() {
                        self.readers
                            .register(cell, done.reads_children, done.reads_tree);
                    }
                }
                Err(need) => {
                    top.need = need;
                    top.next = 0;
                }
            }
        }
    }

    /// One attempt at the formula of `frame`: its result and cost, or the formulas whose
    /// results it needs first.
    fn attempt(&mut self, frame: &mut Frame<'a>) -> Result<(Outcome, u64), Vec<CellId>> {
        let text = self.doc.cell(frame.cell).map_or("", |c| c.text.as_str());
        let mut ctx = Ctx {
            doc: self.doc,
            index: self.index,
            variables: self.variables,
            results: self.results,
            busy: &self.busy,
            me: frame.cell,
            need: Vec::new(),
            work: 0,
            budget: self.budget.saturating_sub(self.done),
            cache: &mut self.cache,
            over: false,
            tainted: false,
            reads_children: Vec::new(),
            reads_tree: Vec::new(),
        };
        ctx.spend(FORMULA_WORK.saturating_add(text.len() as u64 / 32));
        let outcome = formula::evaluate(&formula_of(text), &mut ctx, &mut frame.memo);
        self.tainted |= ctx.tainted;
        frame.reads_children.append(&mut ctx.reads_children);
        frame.reads_tree.append(&mut ctx.reads_tree);
        if ctx.over {
            self.over = true;
            return Ok((limit(), ctx.work));
        }
        match outcome {
            Ok(outcome) => Ok((outcome, ctx.work)),
            Err(Wait) => Err(ctx.need),
        }
    }
}

fn limit() -> Outcome {
    Outcome {
        text: "#LIMIT!".to_string(),
        failed: true,
    }
}

impl Calc {
    /// The results for a document as it is, with no variables: as after loading.
    pub fn new(doc: &Document) -> Calc {
        Calc::with_variables(doc, Variables::new())
    }

    /// The results for a document with the variables of a run.
    pub fn with_variables(doc: &Document, variables: Variables) -> Calc {
        let mut calc = Calc {
            formulas: HashSet::new(),
            results: HashMap::new(),
            variables,
            variables_changed: false,
            index: None,
            readers: Readers::default(),
            tainted: false,
            over: false,
            total_cost: 0,
            budget: MAX_WORK,
            recalculated: Vec::new(),
            group: None,
            stats: Stats::default(),
            last_run: None,
        };
        calc.formulas = doc
            .cells_in_order()
            .into_iter()
            .filter(|id| doc.cell(*id).is_some_and(|c| c.kind == CellKind::Formula))
            .collect();
        calc.full(doc);
        calc.recalculated.clear();
        calc
    }

    /// What the cell shows: the result of a formula cell, the text of any other.
    pub fn shown<'a>(&'a self, doc: &'a Document, id: CellId) -> Option<&'a str> {
        let cell = doc.cell(id)?;
        Some(match self.results.get(&id) {
            Some(res) if cell.kind == CellKind::Formula => res.text.as_str(),
            _ => cell.text.as_str(),
        })
    }

    /// The result of a formula cell, and whether it is an error value such as `#NAME?`.
    pub fn result(&self, id: CellId) -> Option<(&str, bool)> {
        self.results.get(&id).map(|r| (r.text.as_str(), r.failed))
    }

    /// Every formula cell with its result, ordered by ID.
    pub fn results(&self) -> BTreeMap<CellId, &str> {
        self.results
            .iter()
            .map(|(id, r)| (*id, r.text.as_str()))
            .collect()
    }

    /// The formula cells whose results changed in the last command, undo or redo, in
    /// document order; after a group, those of all its commands, by ID; after a group
    /// that was cancelled, those of taking it back
    /// ([`crate::Editor::recalculated`] says it in full).
    pub fn recalculated(&self) -> &[CellId] {
        &self.recalculated
    }

    /// The variables that the last run assigned.
    pub fn variables(&self) -> &Variables {
        &self.variables
    }

    /// What the last run reported.
    pub fn last_run(&self) -> Option<&RunReport> {
        self.last_run.as_ref()
    }

    /// How the last update was done.
    pub fn stats(&self) -> Stats {
        self.stats
    }

    /// Sets the limit on the cells read in one recalculation (for tests and for
    /// interfaces with little time).
    pub fn set_budget(&mut self, doc: &Document, cells: u64) {
        self.budget = cells;
        self.stats = Stats::default();
        self.full(doc);
        self.recalculated.clear();
    }

    /// Takes over what a run assigned and reported; the next update works everything out
    /// again.
    pub(crate) fn ran(&mut self, variables: Variables, report: RunReport) {
        self.variables_changed = self.variables != variables;
        self.variables = variables;
        self.last_run = Some(report);
    }

    /// Starts collecting what recalculates over several updates.
    pub(crate) fn begin_group(&mut self) {
        if self.group.is_none() {
            self.group = Some(BTreeSet::new());
        }
    }

    /// Ends that: `recalculated` is then everything since `begin_group`.
    pub(crate) fn end_group(&mut self, doc: &Document) {
        if let Some(group) = self.group.take() {
            self.recalculated = group
                .into_iter()
                .filter(|id| doc.cell(*id).is_some())
                .collect();
        }
    }

    /// Stops collecting and drops what was collected: `recalculated` is then what the
    /// next update finds, for a group that is taken back and not ended.
    pub(crate) fn cancel_group(&mut self) {
        self.group = None;
    }

    /// Brings the results up to date after a change of the document.
    pub(crate) fn update(&mut self, doc: &Document, changes: &Changes) {
        self.recalculated.clear();
        self.stats = Stats::default();
        for id in &changes.removed {
            self.formulas.remove(id);
        }
        for id in changes.created.iter().chain(&changes.changed) {
            if doc.cell(*id).is_some_and(|c| c.kind == CellKind::Formula) {
                self.formulas.insert(*id);
            } else {
                self.formulas.remove(id);
            }
        }
        let variables_changed = std::mem::take(&mut self.variables_changed);
        if changes.is_empty() && !variables_changed {
            return;
        }
        if self.formulas.is_empty() && self.results.is_empty() {
            self.index = None;
            return;
        }
        let structural =
            !changes.created.is_empty() || !changes.removed.is_empty() || !changes.grids.is_empty();
        if variables_changed
            || structural
            || self.tainted
            || self.over
            || !self.partial(doc, &changes.changed)
        {
            self.full(doc);
        }
        if let Some(group) = &mut self.group {
            group.extend(self.recalculated.iter().copied());
        }
    }

    /// Works every formula out again.
    fn full(&mut self, doc: &Document) {
        let old = std::mem::take(&mut self.results);
        self.readers = Readers::default();
        self.total_cost = 0;
        self.tainted = false;
        self.over = false;
        self.recalculated.clear();
        self.formulas
            .retain(|id| doc.cell(*id).is_some_and(|c| c.kind == CellKind::Formula));
        if self.formulas.is_empty() {
            self.index = None;
            self.stats = Stats {
                full: true,
                evaluated: 0,
                work: self.stats.work,
            };
            return;
        }
        let index = Index::build(doc);
        let mut order: Vec<(u32, CellId)> = self
            .formulas
            .iter()
            .filter_map(|id| Some((index.place(*id)?, *id)))
            .collect();
        order.sort_unstable();
        let mut pass = Pass::new(
            doc,
            &index,
            &self.variables,
            &mut self.results,
            &mut self.readers,
            self.budget,
        );
        for (_, id) in &order {
            pass.ensure(*id);
        }
        self.tainted = pass.tainted;
        self.over = pass.over;
        self.stats = Stats {
            full: true,
            evaluated: pass.evaluated,
            work: self.stats.work.saturating_add(pass.done),
        };
        self.total_cost = self
            .results
            .values()
            .fold(0u64, |a, r| a.saturating_add(r.cost));
        self.recalculated = order
            .iter()
            .map(|(_, id)| *id)
            .filter(|id| {
                let now = self.results.get(id).map(|r| (&r.text, r.failed));
                old.get(id).map(|r| (&r.text, r.failed)) != now
            })
            .collect();
        self.index = Some(index);
    }

    /// Works out again only the formulas that the changed cells can affect. False if that
    /// is not enough, with the results as they were.
    fn partial(&mut self, doc: &Document, changed: &[CellId]) -> bool {
        let Some(index) = self.index.as_ref() else {
            return false;
        };
        let mut queue: Vec<CellId> = Vec::new();
        let mut affected: BTreeSet<CellId> = BTreeSet::new();
        for id in changed {
            let Some(cell) = doc.cell(*id) else {
                return false;
            };
            if index.place(*id).is_none() {
                return false;
            }
            let assigns = |id: CellId| doc.cell(id).is_some_and(|c| c.kind == CellKind::VarAssign);
            if assigns(*id) || index.assign_cells.contains(id) || cell.parent().is_some_and(assigns)
            {
                return false;
            }
            if cell.grid().is_some() {
                // A cell with a grid can be what a name stands for: only its text and kind
                // matter for that.
                match index.grid_cells.get(id) {
                    Some((text, kind)) if *text == cell.text && *kind == cell.kind => continue,
                    _ => return false,
                }
            }
            if cell.kind == CellKind::Formula {
                affected.insert(*id);
            }
            queue.push(*id);
        }
        // Whoever reads a cell whose text may have changed, and whoever reads those.
        while let Some(id) = queue.pop() {
            let mut readers: Vec<CellId> = Vec::new();
            let parent = doc.cell(id).and_then(|c| c.parent());
            if let Some(set) = parent.and_then(|p| self.readers.children.get(&p)) {
                readers.extend(set.iter().copied());
            }
            let mut up = Some(id);
            while let Some(cell) = up {
                if let Some(set) = self.readers.tree.get(&cell) {
                    readers.extend(set.iter().copied());
                }
                up = doc.cell(cell).and_then(|c| c.parent());
            }
            for reader in readers {
                if affected.insert(reader) {
                    queue.push(reader);
                }
            }
        }
        if affected
            .iter()
            .any(|id| index.grid_cells.contains_key(id) || !self.formulas.contains(id))
        {
            return false;
        }
        // With so much to do in all that the part could not be trusted afterwards (see the
        // end), everything is worked out straight away, once, not after a wasted attempt.
        if !affected.is_empty() && self.total_cost.saturating_mul(2) > self.budget {
            return false;
        }
        // Cells that stopped being formulas.
        let mut old: Vec<(CellId, Res)> = Vec::new();
        for id in changed {
            if !self.formulas.contains(id) {
                if let Some(res) = self.results.remove(id) {
                    old.push((*id, res));
                }
            }
        }
        for id in &affected {
            if let Some(res) = self.results.remove(id) {
                old.push((*id, res));
            }
        }
        let mut order: Vec<(u32, CellId)> = affected
            .iter()
            .filter_map(|id| Some((index.place(*id)?, *id)))
            .collect();
        order.sort_unstable();
        // The readers of the old attempts stay until the new ones are known to stand.
        let mut readers = Readers::default();
        let mut pass = Pass::new(
            doc,
            index,
            &self.variables,
            &mut self.results,
            &mut readers,
            self.budget,
        );
        for (_, id) in &order {
            pass.ensure(*id);
        }
        let (tainted, over, evaluated) = (pass.tainted, pass.over, pass.evaluated);
        self.stats.work = pass.done;
        let cost = |results: &HashMap<CellId, Res>, ids: &mut dyn Iterator<Item = CellId>| {
            ids.filter_map(|id| results.get(&id))
                .fold(0u64, |a, r| a.saturating_add(r.cost))
        };
        let spent = cost(&self.results, &mut affected.iter().copied());
        let saved = old.iter().fold(0u64, |a, (_, r)| a.saturating_add(r.cost));
        let total = self.total_cost.saturating_sub(saved).saturating_add(spent);
        if tainted || over || (evaluated > 0 && total.saturating_mul(2) > self.budget) {
            // Put back what was there, so that the full recalculation reports the
            // difference to it.
            for id in &affected {
                self.results.remove(id);
            }
            self.results.extend(old);
            return false;
        }
        self.total_cost = total;
        for (id, _) in &old {
            self.readers.forget(*id);
        }
        for (formula, (children, tree)) in readers.of {
            self.readers.register(formula, children, tree);
        }
        let before: HashMap<CellId, Res> = old.into_iter().collect();
        self.recalculated = order
            .iter()
            .map(|(_, id)| *id)
            .filter(|id| {
                let now = self.results.get(id).map(|r| (&r.text, r.failed));
                before.get(id).map(|r| (&r.text, r.failed)) != now
            })
            .collect();
        self.stats = Stats {
            full: false,
            evaluated,
            work: self.stats.work,
        };
        true
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use crate::edit::{Command, Editor};
    use crate::id::SeededIds;

    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n.max(1) as u64) as usize
        }
    }

    const TEXTS: &[&str] = &[
        "1",
        "2",
        "",
        "abc",
        "=SUM(N0)",
        "=SUM(N1)+SUM(N2)",
        "=N3&N3&N3&N3",
        "=SUM(SUBSET(N1,0,-))+MAX(N0)",
        "=TREESUM(N2)",
        "=1/0",
        "=LEN(N4&'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa')",
        "=SUM(N5)+SUM(N4)+SUM(N3)+SUM(N2)+SUM(N1)+SUM(N0)",
        "=Missing",
    ];

    /// With any limit on the work, however small, keeping the results up to date change by
    /// change gives what working everything out gives: which formulas show `#LIMIT!` must
    /// not depend on what was worked out before, or on who had to wait for whom.
    #[test]
    fn a_limit_on_work_gives_the_same_whether_results_are_kept_or_worked_out_anew() {
        let mut limited = 0;
        for seed in 1..=300u64 {
            let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let mut ids = SeededIds::from_u64(seed);
            let mut doc = Document::new(&mut ids);
            let root = doc.root();
            doc.add_grid(root, 1, 6, &mut ids).unwrap();
            let mut cells = Vec::new();
            for n in 0..6u32 {
                let table = doc.child(root, 0, n).unwrap();
                doc.cell_mut(table).unwrap().text = format!("N{n}");
                let rows = 1 + r.below(4) as u32;
                doc.add_grid(table, 1, rows, &mut ids).unwrap();
                for y in 0..rows {
                    let cell = doc.child(table, 0, y).unwrap();
                    let text = TEXTS[r.below(TEXTS.len())];
                    let c = doc.cell_mut(cell).unwrap();
                    c.text = text.to_string();
                    if text.starts_with('=') {
                        c.kind = CellKind::Formula;
                    }
                    cells.push(cell);
                }
            }
            let budget = 10 + r.below(400) as u64;
            let mut editor = Editor::new(doc).unwrap();
            let mut calc = Calc::new(editor.document());
            calc.set_budget(editor.document(), budget);
            for _ in 0..30 {
                let cell = cells[r.below(cells.len())];
                let text = TEXTS[r.below(TEXTS.len())].to_string();
                let command = if r.below(4) == 0 {
                    Command::SetText { cell, text }
                } else {
                    Command::EnterText { cell, text }
                };
                let changes = editor.apply(command, None, &mut ids).unwrap();
                calc.update(editor.document(), &changes);
                let mut fresh = Calc::new(editor.document());
                fresh.set_budget(editor.document(), budget);
                // The same shown; what it cost can differ (who read a table first).
                assert_eq!(
                    calc.results(),
                    fresh.results(),
                    "seed {seed}, budget {budget}"
                );
                for (id, res) in &calc.results {
                    assert_eq!(Some(res.failed), fresh.results.get(id).map(|r| r.failed));
                }
                assert!(calc.stats.work <= budget.saturating_mul(2).saturating_add(2));
                limited += usize::from(fresh.over);
            }
        }
        // The limit was met often, and often not.
        assert!((1000..8000).contains(&limited), "{limited}");
    }
}
