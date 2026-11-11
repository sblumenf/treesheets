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

use crate::edit::Changes;
use crate::id::CellId;
use crate::model::{CellKind, Document};

use super::formula::{self, formula_of, Check, Contents, Env, Memo, Outcome, Wait};
use super::num::strict_num;
use super::run::{Failure, Value, Variables};

/// Most cells that the formulas of one document may read in one recalculation; formulas
/// beyond it show `#LIMIT!` (decision B52).
pub const MAX_WORK: u64 = 50_000_000;

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
    fn build(doc: &Document) -> Index {
        let order = doc.cells_in_order();
        let mut index = Index {
            place: HashMap::with_capacity(order.len()),
            parent: vec![0; order.len()],
            depth: vec![0; order.len()],
            end: vec![0; order.len()],
            ..Index::default()
        };
        for (n, id) in order.iter().enumerate() {
            index.place.insert(*id, u32::try_from(n).unwrap_or(u32::MAX));
        }
        for (n, id) in order.iter().enumerate() {
            let Some(cell) = doc.cell(*id) else { continue };
            let here = u32::try_from(n).unwrap_or(u32::MAX);
            let parent = cell
                .parent()
                .and_then(|p| index.place.get(&p).copied())
                .unwrap_or(here);
            let depth = if parent == here {
                0
            } else {
                index
                    .depth
                    .get(parent as usize)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(1)
            };
            if let (Some(p), Some(d)) = (index.parent.get_mut(n), index.depth.get_mut(n)) {
                *p = parent;
                *d = depth;
            }
            if cell.kind == CellKind::VarAssign {
                index.assign_cells.insert(*id);
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
            if cell.grid().is_some() {
                index
                    .grid_cells
                    .insert(*id, (cell.text.clone(), cell.kind));
                match cell.kind {
                    CellKind::VarAssign => {}
                    CellKind::Formula => index.fgrid.push(here),
                    _ => index.by_text.entry(cell.text.clone()).or_default().push(here),
                }
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
    /// The cost of the formula so far, and the part of it done in this attempt.
    work: u64,
    fresh: u64,
    budget: u64,
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
        self.work = self.work.saturating_add(n);
        self.fresh = self.fresh.saturating_add(n);
        if self.fresh > self.budget {
            self.over = true;
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
        if self.over {
            return Err("#LIMIT!".to_string());
        }
        self.spend(1);
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
                self.collect_doc(id, tree, sub, into)
            }
            Node::Var(v) => self.collect_value(v, tree, sub, into, 0),
        }
    }

    fn collect_grid(&mut self, node: Node<'a>, tree: bool, into: &mut Contents<Node<'a>>) -> Check {
        match node {
            Node::Doc(id) => {
                if tree {
                    self.reads_tree.push(id);
                } else {
                    self.reads_children.push(id);
                }
                let cells = self.doc.cell(id).and_then(|c| c.grid());
                for child in cells.map_or(&[][..], |g| g.cells()) {
                    self.collect_doc(*child, tree, false, into)?;
                }
                Ok(())
            }
            Node::Var(v) => {
                for child in v.grid.as_deref().map_or(&[][..], |g| g.cells.as_slice()) {
                    self.collect_value(child, tree, false, into, 1)?;
                }
                Ok(())
            }
        }
    }

    fn read_grid(&mut self, node: Node<'a>) {
        if let Node::Doc(id) = node {
            self.reads_children.push(id);
        }
    }

    fn text(&self, node: Node<'a>) -> String {
        match node {
            Node::Doc(id) => self.shown(id).to_string(),
            Node::Var(v) => v.text.clone(),
        }
    }

    fn waiting(&self) -> bool {
        !self.need.is_empty()
    }

    fn work(&self) -> u64 {
        self.work
    }

    fn add_work(&mut self, work: u64) {
        self.work = self.work.saturating_add(work);
    }
}

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
    work: u64,
    budget: u64,
    over: bool,
    tainted: bool,
    evaluated: usize,
}

impl<'a> Pass<'a> {
    /// Brings the result of formula cell `start` up to date, after those it reads.
    fn ensure(&mut self, start: CellId) {
        if self.results.contains_key(&start) {
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
            fresh: 0,
            budget: self.budget.saturating_sub(self.work),
            over: false,
            tainted: false,
            reads_children: Vec::new(),
            reads_tree: Vec::new(),
        };
        let outcome = formula::evaluate(&formula_of(text), &mut ctx, &mut frame.memo);
        self.work = self.work.saturating_add(ctx.fresh);
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

    /// The formula cells whose results changed in the last command, undo, redo or group,
    /// in document order after a full recalculation and by ID otherwise.
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
        let structural = !changes.created.is_empty()
            || !changes.removed.is_empty()
            || !changes.grids.is_empty();
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
        let mut pass = Pass {
            doc,
            index: &index,
            variables: &self.variables,
            results: &mut self.results,
            readers: &mut self.readers,
            busy: HashSet::new(),
            work: 0,
            budget: self.budget,
            over: false,
            tainted: false,
            evaluated: 0,
        };
        for (_, id) in &order {
            pass.ensure(*id);
        }
        self.tainted = pass.tainted;
        self.over = pass.over;
        self.stats = Stats {
            full: true,
            evaluated: pass.evaluated,
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
        let mut pass = Pass {
            doc,
            index,
            variables: &self.variables,
            results: &mut self.results,
            readers: &mut readers,
            busy: HashSet::new(),
            work: 0,
            budget: self.budget,
            over: false,
            tainted: false,
            evaluated: 0,
        };
        for (_, id) in &order {
            pass.ensure(*id);
        }
        let (tainted, over, evaluated) = (pass.tainted, pass.over, pass.evaluated);
        let cost = |results: &HashMap<CellId, Res>, ids: &mut dyn Iterator<Item = CellId>| {
            ids.filter_map(|id| results.get(&id))
                .fold(0u64, |a, r| a.saturating_add(r.cost))
        };
        let spent = cost(&self.results, &mut affected.iter().copied());
        let saved = old.iter().fold(0u64, |a, (_, r)| a.saturating_add(r.cost));
        let total = self.total_cost.saturating_sub(saved).saturating_add(spent);
        if tainted || over || total.saturating_mul(2) > self.budget {
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
        self.recalculated = affected
            .iter()
            .copied()
            .filter(|id| {
                let now = self.results.get(id).map(|r| (&r.text, r.failed));
                before.get(id).map(|r| (&r.text, r.failed)) != now
            })
            .collect();
        self.stats = Stats {
            full: false,
            evaluated,
        };
        true
    }
}
