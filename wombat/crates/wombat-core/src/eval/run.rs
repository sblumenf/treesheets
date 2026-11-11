//! Running the program of a document: TreeSheets' Program > Run (`Evaluator` in
//! `src/evaluator.h`, `Grid::Eval` and `Grid::EvalGridCell` in `src/grid.h`, `Text::Eval`
//! in `src/text.h`).
//!
//! Every grid is a small program. Its cells are worked through row by row from left to
//! right, and then column by column from top to bottom, each time with one value in hand
//! that starts empty at the beginning of a row or column:
//!
//! - a data cell (also a formula cell, by its result) becomes the value in hand; if it has
//!   a grid, that grid is run first and its result is the value;
//! - an operation cell replaces the value by the operation's result, taking the cells
//!   after it as further arguments;
//! - a variable-assign cell names the value, a variable-read cell fetches a named value;
//! - a view cell is overwritten with a copy of the value. This is the only thing a run
//!   changes in the document.
//!
//! The run happens here on a private copy of the document ([`Value`]), in the order and as
//! often as TreeSheets does it (a grid inside a grid is run once per pass over the outer
//! one, and once more when the outer one turns out to be all data), because variables and
//! views carry over from one pass to the next. Afterwards [`Outcome::views`] says which
//! cells of the document were overwritten and with what; the command engine turns that into
//! steps. Nothing here touches the document.

use std::collections::BTreeMap;

use crate::id::CellId;
use crate::model::{Cell, CellKind, Color, Document, Grid};

use super::formula::aggregate;
use super::num::{num_text, stream_num, strict_num, x86};

/// Deepest nesting of a value that a run makes (decision B53).
pub const MAX_VALUE_DEPTH: u32 = 64;
/// Deepest nesting at which a run still works a grid through.
const MAX_RUN_DEPTH: u32 = 64;
/// Most cells that one run may visit and copy together (decision B53).
pub const MAX_RUN_WORK: u64 = 20_000_000;
/// Most cells that a run may hold in its copy of the document, with what its views were
/// filled with, and in its variables together: twice what a document may have (decision
/// A82).
pub const MAX_RUN_CELLS: u64 = 2 * crate::limits::MAX_CELLS as u64;
/// Most bytes of text in those cells: twice what a file may have.
pub const MAX_RUN_TEXT: u64 = 2 * crate::limits::MAX_FILE_BYTES;
/// The bytes of text copied or made that count as one step of a run.
const TEXT_BYTES_PER_WORK: u64 = 16;

/// Where the look of a cell in a value comes from.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Props {
    /// A cell made new by an operation, with the text size given.
    Default { rel_size: i32 },
    /// A cell made new with the text size of this cell (an empty view takes the size of
    /// the cell it is in).
    SizeOf(CellId),
    /// Everything of this cell of the document.
    Full(CellId),
    /// The same without styled ranges and links: the text was replaced.
    Plain(CellId),
    /// Colours, style, alignments and draw style of this cell (`Cell::CloneStyleFrom`).
    Style(CellId),
}

/// A cell outside the document: what an operation works on, what a variable holds, what a
/// run results in.
#[derive(Clone, PartialEq, Debug)]
pub struct Value {
    pub(crate) text: String,
    pub(crate) kind: CellKind,
    /// For a copy of a formula cell: its formula. The text is then its result.
    pub(crate) formula: Option<String>,
    pub(crate) props: Props,
    pub(crate) grid: Option<Box<ValueGrid>>,
    /// The cell of the document this stands for, in the copy of the document only.
    origin: Option<CellId>,
    /// Whether a view put new content here.
    replaced: bool,
}

/// The grid of a [`Value`].
#[derive(Clone, PartialEq, Debug)]
pub struct ValueGrid {
    pub(crate) xs: u32,
    pub(crate) ys: u32,
    pub(crate) cells: Vec<Value>,
    pub(crate) border_color: Color,
    pub(crate) outer_spacing: u32,
    pub(crate) vertical: bool,
    pub(crate) col_widths: Vec<u32>,
}

impl ValueGrid {
    fn new(xs: u32, ys: u32, cells: Vec<Value>) -> ValueGrid {
        ValueGrid {
            xs,
            ys,
            cells,
            border_color: Color::BORDER,
            outer_spacing: Grid::DEFAULT_OUTER_SPACING,
            vertical: true,
            col_widths: vec![Grid::DEFAULT_COL_WIDTH; xs as usize],
        }
    }

    /// Columns and rows.
    pub fn size(&self) -> (u32, u32) {
        (self.xs, self.ys)
    }

    /// The cells, row by row.
    pub fn cells(&self) -> &[Value] {
        &self.cells
    }

    /// The cell in column `x` of row `y`.
    pub fn at(&self, x: u32, y: u32) -> Option<&Value> {
        if x >= self.xs || y >= self.ys {
            return None;
        }
        self.cells.get(
            (y as usize)
                .checked_mul(self.xs as usize)?
                .checked_add(x as usize)?,
        )
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (y as usize)
            .saturating_mul(self.xs as usize)
            .saturating_add(x as usize)
    }
}

impl Value {
    fn fresh(rel_size: i32) -> Value {
        Value {
            text: String::new(),
            kind: CellKind::Data,
            formula: None,
            props: Props::Default { rel_size },
            grid: None,
            origin: None,
            replaced: false,
        }
    }

    /// A cell made new that holds a number (`make_unique<Cell>()` and `SetNum`).
    fn number(d: f64) -> Value {
        let mut v = Value::fresh(0);
        v.text = num_text(d);
        v
    }

    /// A data cell with the look of `like` around a new grid.
    fn holder(like: &Value, grid: ValueGrid) -> Value {
        let mut v = Value::fresh(0);
        v.props = match like.props {
            Props::Full(id) | Props::Plain(id) | Props::Style(id) => Props::Style(id),
            Props::Default { .. } | Props::SizeOf(_) => Props::Default { rel_size: 0 },
        };
        v.grid = Some(Box::new(grid));
        v
    }

    /// The text: of a formula cell its result.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The kind of cell.
    pub fn kind(&self) -> CellKind {
        self.kind
    }

    /// The grid, if any.
    pub fn grid(&self) -> Option<&ValueGrid> {
        self.grid.as_deref()
    }

    /// Whether the look is that of a cell of the document, not of a cell made new.
    pub(crate) fn is_copy(&self) -> bool {
        matches!(self.props, Props::Full(_) | Props::Plain(_))
    }

    /// Replaces the text (`Text::SetText`: styled ranges and links go).
    fn set_text(&mut self, text: String) {
        self.text = text;
        if let Props::Full(id) = self.props {
            self.props = Props::Plain(id);
        }
    }

    fn set_num(&mut self, d: f64) {
        self.set_text(num_text(d));
    }

    fn num(&self) -> f64 {
        stream_num(&self.text)
    }

    /// The value as TreeSheets' text export writes it (`ts.get_subtree_text(0)`): the text
    /// alone for a cell without a grid, else one line per cell of the grid, two more spaces
    /// per level, with a space after the text of a cell that shows an image.
    pub fn to_text(&self, doc: &Document) -> String {
        let Some(grid) = &self.grid else {
            return self.text.clone();
        };
        let mut out = String::new();
        let mut todo: Vec<(&Value, usize)> = grid.cells.iter().rev().map(|c| (c, 2)).collect();
        while let Some((cell, indent)) = todo.pop() {
            out.extend(std::iter::repeat_n(' ', indent));
            out.push_str(&cell.text);
            let image = match cell.props {
                Props::Full(id) | Props::Plain(id) => {
                    doc.cell(id).is_some_and(|c| c.image.is_some())
                }
                _ => false,
            };
            if image {
                out.push(' ');
            }
            out.push('\n');
            if let Some(grid) = &cell.grid {
                todo.extend(
                    grid.cells
                        .iter()
                        .rev()
                        .map(|c| (c, indent.saturating_add(2))),
                );
            }
        }
        out
    }

    /// The model cell for this value, with the given place: its own content, not its grid.
    pub(crate) fn to_cell(&self, id: CellId, parent: Option<CellId>, doc: &Document) -> Cell {
        let mut cell = Cell::new(id, parent);
        match self.props {
            Props::Default { rel_size } => cell.rel_size = rel_size,
            Props::SizeOf(source) => {
                cell.rel_size = doc.cell(source).map_or(0, |like| like.rel_size);
            }
            Props::Full(source) | Props::Plain(source) => {
                if let Some(like) = doc.cell(source) {
                    cell = like.clone();
                    cell.id = id;
                    cell.parent = parent;
                    cell.grid = None;
                }
            }
            Props::Style(source) => {
                if let Some(like) = doc.cell(source) {
                    cell.cell_color = like.cell_color;
                    cell.text_color = like.text_color;
                    cell.draw_style = like.draw_style;
                    cell.text_align = like.text_align;
                    cell.vert_align = like.vert_align;
                    cell.style = like.style;
                }
            }
        }
        cell.kind = self.kind;
        // A formula cell is stored by its formula, anything else by the text it shows.
        let text = match (&self.formula, self.kind) {
            (Some(formula), CellKind::Formula) => formula.clone(),
            _ => self.text.clone(),
        };
        if cell.text != text || matches!(self.props, Props::Plain(_)) {
            // Ranges of another text mean nothing here.
            cell.runs.clear();
            cell.links.clear();
        }
        cell.text = text;
        cell
    }

    /// The model grid for the grid of this value, with the given cells.
    pub(crate) fn to_grid(&self, cells: Vec<CellId>) -> Option<Grid> {
        let own = self.grid.as_deref()?;
        let mut grid = Grid::new(own.xs, own.ys, cells);
        grid.border_color = own.border_color;
        grid.outer_spacing = own.outer_spacing;
        grid.vertical = own.vertical;
        grid.col_widths.clone_from(&own.col_widths);
        grid.col_widths
            .resize(own.xs as usize, Grid::DEFAULT_COL_WIDTH);
        Some(grid)
    }

    /// The number of cells, this one included, and the bytes of their texts.
    fn weight(&self) -> (u64, u64) {
        let (mut cells, mut bytes) = (0u64, 0u64);
        let mut todo = vec![self];
        while let Some(v) = todo.pop() {
            cells = cells.saturating_add(1);
            let formula = v.formula.as_ref().map_or(0, String::len);
            bytes = bytes.saturating_add(v.text.len().saturating_add(formula) as u64);
            if let Some(grid) = &v.grid {
                todo.extend(grid.cells.iter());
            }
        }
        (cells, bytes)
    }

    /// The number of cells inside, and how deep they nest.
    pub(crate) fn extent(&self) -> (usize, u32) {
        let (mut count, mut height) = (0usize, 0u32);
        let mut todo = vec![(self, 0u32)];
        while let Some((v, level)) = todo.pop() {
            height = height.max(level);
            if let Some(grid) = &v.grid {
                count = count.saturating_add(grid.cells.len());
                todo.extend(grid.cells.iter().map(|c| (c, level.saturating_add(1))));
            }
        }
        (count, height)
    }
}

/// What went wrong first in a run. The run goes on after it, as in TreeSheets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FailureKind {
    /// An operation cell whose text is no operation.
    UnknownOperation,
    /// An operation with nothing before it to work on.
    NoInput,
    /// An operation without enough cells after it.
    NoArgument,
    /// A variable that was read before anything was assigned to it.
    NoVariable,
}

/// The first failure of a run: what, the text of the cell, and the cell if it is one of
/// the document.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Failure {
    /// What went wrong.
    pub kind: FailureKind,
    /// The text of the operation or variable cell.
    pub name: String,
    /// The cell, unless it is a copy made during the run.
    pub cell: Option<CellId>,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = &self.name;
        match self.kind {
            FailureKind::UnknownOperation => write!(f, "unknown operation \"{name}\""),
            FailureKind::NoInput => write!(f, "nothing for \"{name}\" to work on"),
            FailureKind::NoArgument => write!(f, "\"{name}\" is missing an argument"),
            FailureKind::NoVariable => write!(f, "the variable \"{name}\" has no value"),
        }
    }
}

/// The variables that a run assigned, by name.
pub type Variables = BTreeMap<String, Value>;

/// What a run gave.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Outcome {
    /// The value of the root cell, if any.
    pub(crate) result: Option<Value>,
    /// The cells of the document that views overwrote, in document order, none inside
    /// another, with their new content.
    pub(crate) views: Vec<(CellId, Value)>,
    pub(crate) variables: Variables,
    pub(crate) failure: Option<Failure>,
}

/// The run stopped at a limit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OverLimit {
    /// Values nested too deeply, or too many steps.
    Work,
    /// Too much held in views and variables.
    Held,
}

#[derive(Clone, Copy)]
enum Op {
    /// Two numbers to one.
    Binary(fn(f64, f64) -> f64),
    /// One number to one.
    Unary(fn(f64) -> f64),
    Graph,
    /// A row or column of numbers to one; `sum` reads numbers as the other operations do.
    List(&'static str),
    Transpose,
    If,
}

impl Op {
    fn args(self) -> usize {
        match self {
            Op::Binary(_) => 2,
            Op::If => 3,
            _ => 1,
        }
    }
}

fn truth(b: bool) -> f64 {
    if b {
        1.0
    } else {
        0.0
    }
}

/// The operation with this name (`Evaluator::Init`). Names are exact, in lower case.
fn operation(name: &str) -> Option<Op> {
    Some(match name {
        "+" => Op::Binary(|a, b| x86(a, b, a + b)),
        "-" => Op::Binary(|a, b| x86(a, b, a - b)),
        "*" => Op::Binary(|a, b| x86(a, b, a * b)),
        "/" => Op::Binary(|a, b| if b != 0.0 { x86(a, b, a / b) } else { 0.0 }),
        "<" => Op::Binary(|a, b| truth(a < b)),
        ">" => Op::Binary(|a, b| truth(a > b)),
        "<=" => Op::Binary(|a, b| truth(a <= b)),
        ">=" => Op::Binary(|a, b| truth(a >= b)),
        #[allow(clippy::float_cmp)]
        "=" | "==" => Op::Binary(|a, b| truth(a == b)),
        #[allow(clippy::float_cmp)]
        "!=" | "<>" => Op::Binary(|a, b| truth(a != b)),
        "inc" => Op::Unary(|a| x86(a, a, a + 1.0)),
        "dec" => Op::Unary(|a| x86(a, a, a - 1.0)),
        "neg" => Op::Unary(|a| -a),
        "graph" => Op::Graph,
        "sum" => Op::List("sum"),
        "min" => Op::List("MIN"),
        "max" => Op::List("MAX"),
        "avg" => Op::List("AVG"),
        "median" => Op::List("MEDIAN"),
        "transpose" => Op::Transpose,
        "if" => Op::If,
        _ => return None,
    })
}

/// Whether a text is the name of an operation (`Evaluator::InferCellType`: only such a
/// cell can be marked as an operation in TreeSheets).
pub fn is_operation(name: &str) -> bool {
    operation(name).is_some()
}

struct Runner {
    vars: Variables,
    vert: bool,
    failure: Option<Failure>,
    work: u64,
    depth: u32,
    over: bool,
    /// The cells, and the bytes of their texts, of the copy of the document and of the
    /// variables; and whether they went over what a run may hold.
    held: (u64, u64),
    full: bool,
}

impl Runner {
    fn spend(&mut self) -> bool {
        self.spend_n(1)
    }

    fn spend_n(&mut self, n: u64) -> bool {
        self.work = self.work.saturating_add(n);
        if self.work > MAX_RUN_WORK {
            self.over = true;
        }
        !self.over
    }

    /// Counts a text of `bytes` bytes that is about to be copied or made.
    fn spend_text(&mut self, bytes: usize) -> bool {
        self.spend_n(bytes as u64 / TEXT_BYTES_PER_WORK)
    }

    /// Whether `new` may be kept (in a view or a variable) in the place of `old`: not if
    /// the run would then hold more than it may. Asked before `new` is copied.
    fn keep(&mut self, new: &Value, old: Option<(u64, u64)>) -> bool {
        let (cells, bytes) = new.weight();
        let (old_cells, old_bytes) = old.unwrap_or((0, 0));
        let held = (
            self.held.0.saturating_sub(old_cells).saturating_add(cells),
            self.held.1.saturating_sub(old_bytes).saturating_add(bytes),
        );
        // Giving back is always allowed; so is what the document itself holds.
        let more = cells > old_cells || bytes > old_bytes;
        if more && (held.0 > MAX_RUN_CELLS || held.1 > MAX_RUN_TEXT) {
            self.over = true;
            self.full = true;
            return false;
        }
        self.held = held;
        true
    }

    /// A copy that belongs to no place in the document (`Cell::Clone`).
    fn copy(&mut self, v: &Value) -> Value {
        self.copy_at(v, 0)
    }

    fn copy_at(&mut self, v: &Value, level: u32) -> Value {
        let grid = match &v.grid {
            Some(_) if level >= MAX_VALUE_DEPTH => {
                self.over = true;
                None
            }
            Some(grid) => {
                let mut cells = Vec::with_capacity(grid.cells.len());
                for c in &grid.cells {
                    if !self.spend() || !self.spend_text(c.text.len()) {
                        break;
                    }
                    cells.push(self.copy_at(c, level.saturating_add(1)));
                }
                // Cut short at a limit: the run is given up anyway.
                cells.resize_with(grid.cells.len(), || Value::fresh(0));
                Some(Box::new(ValueGrid {
                    cells,
                    col_widths: grid.col_widths.clone(),
                    ..**grid
                }))
            }
            None => None,
        };
        Value {
            text: v.text.clone(),
            kind: v.kind,
            formula: v.formula.clone(),
            props: v.props,
            grid,
            origin: None,
            replaced: false,
        }
    }

    /// Notes the first failure (`Evaluator::Fail`).
    fn fail(&mut self, kind: FailureKind, c: &Value) {
        if self.failure.is_none() {
            self.failure = Some(Failure {
                kind,
                name: c.text.clone(),
                cell: c.origin,
            });
        }
    }

    /// Gives the variable `name` a copy of `value`, unless the run may not hold that much.
    fn set_symbol(&mut self, name: &str, value: &Value) {
        let old = self.vars.get(name).map(Value::weight);
        if name.is_empty() || !self.keep(value, old) {
            return;
        }
        let copy = self.copy(value);
        self.vars.insert(name.to_string(), copy);
    }

    /// `Evaluator::Assign`: the value under the text of `sym`, and if both have grids of
    /// one size, each cell of the value under the text of the cell in the same place.
    fn assign(&mut self, sym: &Value, val: &Value) {
        self.set_symbol(&sym.text, val);
        if let (Some(names), Some(values)) = (&sym.grid, &val.grid) {
            if names.size() == values.size() {
                for x in 0..names.xs {
                    for y in 0..names.ys {
                        if let (Some(n), Some(v)) = (names.at(x, y), values.at(x, y)) {
                            self.set_symbol(&n.text, v);
                        }
                    }
                }
            }
        }
    }

    /// `Cell::Eval`: the grid if there is one, else the text.
    fn eval(&mut self, c: &mut Value) -> Option<Value> {
        if !self.spend() {
            return None;
        }
        if c.grid.is_some() {
            if self.depth >= MAX_RUN_DEPTH {
                self.over = true;
                return None;
            }
            self.depth = self.depth.saturating_add(1);
            let result = self.eval_grid(c);
            self.depth = self.depth.saturating_sub(1);
            return result;
        }
        match c.kind {
            CellKind::VarRead => Some(match self.vars.remove(&c.text) {
                Some(v) => {
                    let copy = self.copy(&v);
                    self.vars.insert(c.text.clone(), v);
                    copy
                }
                None => {
                    self.fail(FailureKind::NoVariable, c);
                    let mut v = self.copy(c);
                    v.kind = CellKind::Data;
                    v.set_text("**Variable Load Error**".to_string());
                    v
                }
            }),
            CellKind::Data => Some(self.copy(c)),
            CellKind::Formula => {
                let mut v = self.copy(c);
                v.kind = CellKind::Data;
                v.formula = None;
                Some(v)
            }
            _ => None,
        }
    }

    /// `Grid::Eval`.
    fn eval_grid(&mut self, owner: &mut Value) -> Option<Value> {
        let (xs, ys) = owner.grid.as_ref()?.size();
        // What an empty view in this grid becomes: a new cell of the owner's text size.
        let empty = match owner.props {
            Props::Full(id) | Props::Plain(id) | Props::SizeOf(id) => Props::SizeOf(id),
            Props::Default { rel_size } => Props::Default { rel_size },
            Props::Style(_) => Props::Default { rel_size: 0 },
        };
        let mut acc: Option<Value> = None;
        let mut alldata = true;
        if xs > 1 || ys == 1 {
            for y in 0..ys {
                let mut x = 0;
                while x < xs {
                    if x == 0 {
                        acc = None;
                    }
                    let mut at = (x, y);
                    acc = self.eval_grid_cell(owner, empty, &mut at, acc, &mut alldata, false);
                    x = at.0.saturating_add(1);
                }
            }
        }
        if ys > 1 {
            for x in 0..xs {
                let mut y = 0;
                while y < ys {
                    if y == 0 {
                        acc = None;
                    }
                    let mut at = (x, y);
                    acc = self.eval_grid_cell(owner, empty, &mut at, acc, &mut alldata, true);
                    y = at.1.saturating_add(1);
                }
            }
        }
        if alldata {
            let mut result = self.copy(owner);
            if let Some(grid) = result.grid.as_mut() {
                for rc in &mut grid.cells {
                    // A cell whose evaluation failed keeps its original contents.
                    if let Some(evaluated) = self.eval(rc) {
                        *rc = evaluated;
                    }
                }
            }
            return Some(result);
        }
        acc
    }

    /// `Grid::EvalGridCell`: one cell of a grid, with the value in hand.
    fn eval_grid_cell(
        &mut self,
        owner: &mut Value,
        empty: Props,
        at: &mut (u32, u32),
        acc: Option<Value>,
        alldata: &mut bool,
        vert: bool,
    ) -> Option<Value> {
        if self.over {
            return None;
        }
        let grid = owner.grid.as_mut()?;
        let (xs, ys) = grid.size();
        let i = grid.index(at.0, at.1);
        let ct = grid.cells.get(i)?.kind;
        *alldata = *alldata && matches!(ct, CellKind::Data | CellKind::VarRead | CellKind::Formula);
        self.vert = vert;
        match ct {
            CellKind::VarAssign => {
                if vert {
                    return acc; // vertical assignments are not done
                }
                // With nothing in hand, the grid of the cell itself may give something.
                // (TreeSheets reads through an empty value in hand here, which is not
                // defined; this is the nearest defined thing.)
                let acc = match acc {
                    Some(v) if v.grid.is_some() || !v.text.is_empty() => v,
                    _ => self.eval(grid.cells.get_mut(i)?)?,
                };
                // The names are those of the cell as it is, grid included: taken out of
                // the grid for the moment, so that nothing has to be copied.
                let slot = grid.cells.get_mut(i)?;
                let sym = std::mem::replace(slot, Value::fresh(0));
                self.assign(&sym, &acc);
                *slot = sym;
                Some(acc)
            }
            CellKind::ViewH | CellKind::ViewV => {
                if (ct == CellKind::ViewH) == vert {
                    // A view of the other direction passes itself on.
                    let copy = self.copy(grid.cells.get(i)?);
                    return Some(copy);
                }
                let nothing = Value {
                    props: empty,
                    ..Value::fresh(0)
                };
                if !self.keep(
                    acc.as_ref().unwrap_or(&nothing),
                    grid.cells.get(i).map(Value::weight),
                ) {
                    return None;
                }
                let mut new = match &acc {
                    Some(v) => self.copy(v),
                    None => nothing,
                };
                let slot = grid.cells.get_mut(i)?;
                new.kind = ct;
                new.origin = slot.origin;
                new.replaced = true;
                *slot = new;
                acc
            }
            CellKind::Code => {
                let c = grid.cells.get(i)?;
                let Some(op) = operation(&c.text) else {
                    self.fail(FailureKind::UnknownOperation, c);
                    return None;
                };
                let Some(acc) = acc else {
                    self.fail(FailureKind::NoInput, c);
                    return None;
                };
                let nargs = op.args();
                if nargs == 1 {
                    return Some(self.exec1(op, acc, 0));
                }
                // The arguments follow the operation, to the right of it or below it.
                let (pos, len) = if vert { (at.1, ys) } else { (at.0, xs) };
                let last = (pos as usize).saturating_add(nargs).saturating_sub(1);
                if last >= len as usize {
                    self.fail(FailureKind::NoArgument, c);
                    return None;
                }
                let last = u32::try_from(last).ok()?;
                if vert {
                    at.1 = last;
                } else {
                    at.0 = last;
                }
                let arg = |back: u32| {
                    if vert {
                        grid.index(at.0, last.saturating_sub(back))
                    } else {
                        grid.index(last.saturating_sub(back), at.1)
                    }
                };
                let (first, second) = (arg(1), arg(0));
                if nargs == 2 {
                    let right = grid.cells.get_mut(second)?;
                    Some(self.exec2(op, acc, right, 0))
                } else {
                    // `if` is so far the only operation with three: the condition in hand
                    // picks which of the two cells after it is evaluated.
                    if acc.text.is_empty() {
                        return Some(acc);
                    }
                    let pick = if acc.num() != 0.0 { first } else { second };
                    self.eval(grid.cells.get_mut(pick)?)
                }
            }
            _ => self.eval(grid.cells.get_mut(i)?),
        }
    }

    /// `Evaluator::Execute` for an operation on the value in hand alone.
    fn exec1(&mut self, op: Op, mut left: Value, level: u32) -> Value {
        if level > MAX_VALUE_DEPTH || !self.spend() {
            self.over = true;
            return left;
        }
        let inside = level.saturating_add(1);
        match op {
            Op::Unary(f) => {
                if !left.text.is_empty() {
                    let n = left.num();
                    left.set_num(f(n));
                } else if let Some(grid) = left.grid.as_mut() {
                    for c in &mut grid.cells {
                        let cell = std::mem::replace(c, Value::fresh(0));
                        *c = self.exec1(op, cell, inside);
                    }
                }
            }
            Op::Graph => {
                if !left.text.is_empty() {
                    let n = left.num();
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let bars = if n > 0.0 { n.min(1000.0) as usize } else { 0 };
                    // A thousand bars for a cell that held "1e3": counted four times.
                    if self.spend_text(bars.saturating_mul(4)) {
                        left.set_text("|".repeat(bars));
                    }
                } else if let Some(grid) = left.grid.as_mut() {
                    for c in &mut grid.cells {
                        let cell = std::mem::replace(c, Value::fresh(0));
                        *c = self.exec1(op, cell, inside);
                    }
                }
            }
            Op::List(name) => {
                let Some(grid) = left.grid.take() else {
                    return left;
                };
                if grid.xs == 1 || grid.ys == 1 {
                    return list(name, &grid.cells);
                }
                // One result per row, or per column when working downwards.
                let (xs, ys) = grid.size();
                let lines = if self.vert { xs } else { ys };
                let mut results = Vec::with_capacity(lines as usize);
                for n in 0..lines {
                    let line: Vec<&Value> = if self.vert {
                        (0..ys).filter_map(|y| grid.at(n, y)).collect()
                    } else {
                        (0..xs).filter_map(|x| grid.at(x, n)).collect()
                    };
                    results.push(list_of(name, &line));
                }
                let new = if self.vert {
                    ValueGrid::new(lines, 1, results)
                } else {
                    ValueGrid::new(1, lines, results)
                };
                return Value::holder(&left, new);
            }
            Op::Transpose => {
                if let Some(grid) = left.grid.as_mut() {
                    transpose(grid);
                }
            }
            Op::Binary(_) | Op::If => {}
        }
        left
    }

    /// `Evaluator::Execute` for an operation on the value in hand and the cell after it.
    fn exec2(&mut self, op: Op, mut left: Value, right: &mut Value, level: u32) -> Value {
        if level > MAX_VALUE_DEPTH {
            self.over = true;
            return left;
        }
        let Some(mut right) = self.eval(right) else {
            return left;
        };
        let Op::Binary(f) = op else {
            return left;
        };
        let inside = level.saturating_add(1);
        let same_size = match (&left.grid, &right.grid) {
            (Some(a), Some(b)) => a.size() == b.size(),
            _ => false,
        };
        if !left.text.is_empty() && !right.text.is_empty() {
            left.set_num(f(left.num(), right.num()));
        } else if same_size {
            let (Some(mut g1), Some(g2)) = (left.grid.take(), right.grid.as_mut()) else {
                return left;
            };
            let (xs, ys) = g1.size();
            let mut cells: Vec<Value> = (0..g1.cells.len()).map(|_| Value::fresh(0)).collect();
            for x in 0..xs {
                for y in 0..ys {
                    let i = g1.index(x, y);
                    let (Some(c1), Some(c2)) = (g1.cells.get_mut(i), g2.cells.get_mut(i)) else {
                        continue;
                    };
                    let c1 = std::mem::replace(c1, Value::fresh(0));
                    let res = self.exec2(op, c1, c2, inside);
                    if let Some(slot) = cells.get_mut(i) {
                        *slot = res;
                    }
                }
            }
            return Value::holder(&left, ValueGrid::new(xs, ys, cells));
        } else if !right.text.is_empty() {
            if let Some(g1) = left.grid.as_mut() {
                for c in &mut g1.cells {
                    let cell = std::mem::replace(c, Value::fresh(0));
                    *c = self.exec2(op, cell, &mut right, inside);
                }
            }
        }
        left
    }
}

/// An aggregate over a row or column of cells, as a new cell.
fn list(name: &str, cells: &[Value]) -> Value {
    let all: Vec<&Value> = cells.iter().collect();
    list_of(name, &all)
}

fn list_of(name: &str, cells: &[&Value]) -> Value {
    if name == "sum" {
        // `Grid::Sum`: every cell with text counts, by the number it starts with.
        let mut total = 0.0;
        for c in cells.iter().filter(|c| !c.text.is_empty()) {
            let n = c.num();
            total = x86(total, n, total + n);
        }
        return Value::number(total);
    }
    // `Grid::Aggregate`: only cells whose whole text is a number.
    let numbers: Vec<f64> = cells.iter().filter_map(|c| strict_num(&c.text)).collect();
    Value::number(aggregate(name).map_or(0.0, |f| f(&numbers)))
}

/// `Grid::Transpose`: rows become columns; new columns get the width of the last old one.
fn transpose(grid: &mut ValueGrid) {
    let (xs, ys) = grid.size();
    let mut cells: Vec<Value> = (0..grid.cells.len()).map(|_| Value::fresh(0)).collect();
    for y in 0..ys {
        for x in 0..xs {
            let from = grid.index(x, y);
            let to = (x as usize)
                .saturating_mul(ys as usize)
                .saturating_add(y as usize);
            if let (Some(c), Some(slot)) = (grid.cells.get_mut(from), cells.get_mut(to)) {
                *slot = std::mem::replace(c, Value::fresh(0));
            }
        }
    }
    grid.cells = cells;
    grid.xs = ys;
    grid.ys = xs;
    let width = grid
        .col_widths
        .last()
        .copied()
        .unwrap_or(Grid::DEFAULT_COL_WIDTH);
    grid.col_widths.resize(ys as usize, width);
}

/// The copy of the document that a run works on: every cell with the text it shows.
fn snapshot(doc: &Document, shown: &dyn Fn(CellId) -> Option<String>) -> Option<Value> {
    fn build(
        doc: &Document,
        shown: &dyn Fn(CellId) -> Option<String>,
        id: CellId,
        level: u32,
    ) -> Option<Value> {
        let cell = doc.cell(id)?;
        let grid = match cell.grid() {
            Some(_) if level > crate::limits::MAX_DEPTH => None,
            Some(grid) => {
                let mut cells = Vec::with_capacity(grid.cells().len());
                for child in grid.cells() {
                    cells.push(build(doc, shown, *child, level.saturating_add(1))?);
                }
                Some(Box::new(ValueGrid {
                    xs: grid.xs(),
                    ys: grid.ys(),
                    cells,
                    border_color: grid.border_color,
                    outer_spacing: grid.outer_spacing,
                    vertical: grid.vertical,
                    col_widths: grid.col_widths.clone(),
                }))
            }
            None => None,
        };
        let formula = (cell.kind == CellKind::Formula)
            .then(|| super::formula::formula_of(&cell.text).into_owned());
        Some(Value {
            text: match &formula {
                Some(_) => shown(id).unwrap_or_default(),
                None => cell.text.clone(),
            },
            kind: cell.kind,
            formula,
            props: Props::Full(id),
            grid,
            origin: Some(id),
            replaced: false,
        })
    }
    build(doc, shown, doc.root(), 0)
}

/// Runs the program of the document. `shown` gives the result of a formula cell.
pub(crate) fn run(
    doc: &Document,
    shown: &dyn Fn(CellId) -> Option<String>,
) -> Result<Outcome, OverLimit> {
    let mut root = snapshot(doc, shown).ok_or(OverLimit::Work)?;
    let mut runner = Runner {
        vars: Variables::new(),
        vert: false,
        failure: None,
        work: 0,
        depth: 0,
        over: false,
        held: root.weight(),
        full: false,
    };
    let result = runner.eval(&mut root);
    if runner.over {
        return Err(if runner.full {
            OverLimit::Held
        } else {
            OverLimit::Work
        });
    }
    // The cells that views overwrote: the outermost ones, in document order.
    let mut views = Vec::new();
    let mut todo = vec![root];
    while let Some(v) = todo.pop() {
        match (v.origin, v.replaced) {
            (Some(id), true) => views.push((id, v)),
            _ => {
                if let Some(grid) = v.grid {
                    todo.extend(grid.cells.into_iter().rev());
                }
            }
        }
    }
    Ok(Outcome {
        result,
        views,
        variables: runner.vars,
        failure: runner.failure,
    })
}
