//! One formula: reading it and working it out (`FormulaEvaluator::Parser` in TreeSheets'
//! `src/evaluator.h`).
//!
//! A formula is `=` and then an expression: numbers, `+ - * /` and parentheses, `&` to join
//! texts, `'literal text'`, names of grids (bare or in double quotes), the aggregate
//! functions over a name or a `SUBSET(...)`, and the text functions. What the names stand
//! for is not known here: the parser asks an [`Env`], so that the same code serves cells of
//! the document and the values that a run of the program left behind.
//!
//! The first thing that goes wrong decides the error, and reading carries on after it, as
//! in TreeSheets, so every name in the formula is looked at whatever came before.

use std::cell::RefCell;
use std::rc::Rc;

use super::num::{
    is_letter, is_name_char, is_space, lower, num_text, strict_num, trim, trim_end, upper,
};

/// Deepest nesting of parentheses and function calls in one formula. TreeSheets has no
/// limit and would run out of stack instead (decision B52).
pub const MAX_NESTING: u32 = 64;
/// Longest formula that is worked out, in code points (decision B52).
pub const MAX_FORMULA_LEN: usize = 4096;
/// Longest text that a formula may make or pass on, in bytes: a sixteenth of the code
/// points that the text of a cell may have (decision A80). Beyond it: `#LIMIT!`.
pub const MAX_TEXT_BYTES: usize = crate::limits::MAX_TEXT_CODE_POINTS / 16;
/// The bytes of text made that count as much work as one cell read (decision A80).
const TEXT_BYTES_PER_WORK: u64 = 16;

/// What a name or a `SUBSET()` gave: `FormulaEvaluator::Contents`.
#[derive(Clone, Debug)]
pub(crate) struct Contents<N> {
    /// The numbers, in the order the cells were visited.
    pub(crate) numbers: Vec<f64>,
    /// The last cell that was a number.
    pub(crate) number: Option<N>,
    /// The last cell with other text, and how many there were.
    pub(crate) text: Option<N>,
    pub(crate) texts: u32,
    /// The aggregates of the numbers that were asked for before, so that a table that many
    /// formulas read is added up once.
    done: RefCell<Vec<(&'static str, f64)>>,
}

impl<N> Contents<N> {
    /// The aggregate `f`, called `name`, of the numbers.
    fn aggregate(&self, name: &'static str, f: Aggregate) -> f64 {
        if let Some((_, d)) = self.done.borrow().iter().find(|(n, _)| *n == name) {
            return *d;
        }
        let d = f(&self.numbers);
        self.done.borrow_mut().push((name, d));
        d
    }
}

impl<N> Default for Contents<N> {
    fn default() -> Contents<N> {
        Contents {
            numbers: Vec::new(),
            number: None,
            text: None,
            texts: 0,
            done: RefCell::new(Vec::new()),
        }
    }
}

/// An error value such as `#NAME?`. `Ok(())` is "no error".
pub(crate) type Check = Result<(), String>;

/// The evaluation cannot go on until other formulas have their results; the [`Env`] knows
/// which (see `calc.rs`).
#[derive(Debug)]
pub(crate) struct Wait;

/// What a formula can see of the document.
pub(crate) trait Env {
    /// A cell, of the document or of a value.
    type Node: Copy;

    /// The cell that `name` stands for (`FormulaEvaluator::Named`), or why there is none.
    fn named(&mut self, name: &str) -> Result<Self::Node, String>;
    /// Columns and rows of the grid of a cell, if it has one.
    fn size(&self, node: Self::Node) -> Option<(u32, u32)>;
    /// A cell of the grid of `node`.
    fn child(&self, node: Self::Node, x: u32, y: u32) -> Option<Self::Node>;
    /// Adds the contents of one cell (`Collect` for a cell): its own, unless it is the
    /// formula's own cell or `sub` and a formula, and with `tree` those of everything in
    /// its grid.
    fn collect(
        &mut self,
        node: Self::Node,
        tree: bool,
        sub: bool,
        into: &mut Contents<Self::Node>,
    ) -> Check;
    /// The contents of the cells of the grid of `node` (`Collect` for a grid). Shared, not
    /// copied: many formulas read the same tables.
    fn collect_grid(&mut self, node: Self::Node, tree: bool) -> Collected<Self::Node>;
    /// Notes that the formula read the cells of the grid of `node`.
    fn read_grid(&mut self, node: Self::Node);
    /// The text a cell shows.
    fn text(&self, node: Self::Node) -> &str;
    /// Whether the evaluation has to stop and wait for other formulas.
    fn waiting(&self) -> bool;
    /// The work done so far, for the memory of finished calls.
    fn work(&self) -> u64;
    /// Adds work: what a remembered call stands for, or a text about to be made. False if
    /// the work allowed is used up.
    fn add_work(&mut self, work: u64) -> bool;
}

/// What a call into the [`Env`] gave: the contents, shared, or an error value.
pub(crate) type Collected<N> = Result<Rc<Contents<N>>, String>;

/// A call into the [`Env`] that was finished in an earlier attempt at the same formula, so
/// that an attempt after waiting does not repeat it.
#[derive(Clone, Debug)]
pub(crate) struct Memo<N> {
    end: usize,
    result: Collected<N>,
    work: u64,
}

/// A number, also with the text it came from, if any, or a text: `Value` in TreeSheets.
#[derive(Clone, Debug)]
struct Val {
    d: f64,
    t: String,
    text: bool,
}

impl Val {
    fn num(d: f64) -> Val {
        Val {
            d,
            t: String::new(),
            text: false,
        }
    }

    fn text(t: String) -> Val {
        Val {
            d: 0.0,
            t,
            text: true,
        }
    }
}

type Aggregate = fn(&[f64]) -> f64;

fn sum(v: &[f64]) -> f64 {
    v.iter().fold(0.0, |a, b| a + b)
}

fn min(v: &[f64]) -> f64 {
    let mut best = v.first().copied().unwrap_or(0.0);
    for x in v {
        if *x < best {
            best = *x;
        }
    }
    best
}

fn max(v: &[f64]) -> f64 {
    let mut best = v.first().copied().unwrap_or(0.0);
    for x in v {
        if best < *x {
            best = *x;
        }
    }
    best
}

#[allow(clippy::cast_precision_loss)] // a count of cells
fn avg(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        sum(v) / v.len() as f64
    }
}

/// The middle number when sorted, or the average of the two middle ones. Zero and minus
/// zero count as equal and keep their order, which is what TreeSheets' `nth_element` does
/// for up to three numbers (for more, which of the two zeroes it picks is not defined).
pub(crate) fn median(v: &[f64]) -> f64 {
    let mut sorted = v.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    let at = |i: usize| sorted.get(i).copied().unwrap_or(0.0);
    if sorted.is_empty() {
        0.0
    } else if sorted.len() % 2 == 1 {
        at(mid)
    } else {
        (at(mid) + max(sorted.get(..mid).unwrap_or_default())) / 2.0
    }
}

#[allow(clippy::cast_precision_loss)]
fn count(v: &[f64]) -> f64 {
    v.len() as f64
}

/// The aggregate functions (`FormulaEvaluator::Functions`).
pub(crate) const FUNCTIONS: &[(&str, Aggregate)] = &[
    ("SUM", sum),
    ("MIN", min),
    ("MAX", max),
    ("AVG", avg),
    ("AVERAGE", avg),
    ("MEDIAN", median),
    ("COUNT", count),
];

/// The aggregates that the operations share with the formulas (`aggregate::` in TreeSheets).
pub(crate) fn aggregate(name: &str) -> Option<Aggregate> {
    FUNCTIONS.iter().find(|(n, _)| *n == name).map(|(_, f)| *f)
}

/// The functions on text with their number of arguments (`TextFunctions`).
const TEXT_FUNCTIONS: &[(&str, usize)] = &[
    ("LEN", 1),
    ("LEFT", 2),
    ("RIGHT", 2),
    ("MID", 3),
    ("UPPER", 1),
    ("LOWER", 1),
    ("TRIM", 1),
    ("SUBSTITUTE", 3),
];

fn is_subset(function: &str) -> bool {
    function == "SUBSET" || function == "TREESUBSET"
}

/// The formula of a formula cell: its text, with `=` put in front if it does not start with
/// one (as TreeSheets does when a cell is marked as a formula, `Cell::SyncFormula`).
pub fn formula_of(text: &str) -> std::borrow::Cow<'_, str> {
    if text.starts_with('=') {
        text.into()
    } else {
        format!("={text}").into()
    }
}

/// Whether a text typed into a cell makes it a formula cell: it starts with `=` and has
/// more than that (`Cell::IsFormulaText`).
pub fn is_formula_text(text: &str) -> bool {
    text.starts_with('=') && text.chars().nth(1).is_some()
}

struct Parser<'a, E: Env> {
    env: &'a mut E,
    s: Vec<char>,
    i: usize,
    error: Option<String>,
    depth: u32,
    memo: &'a mut Vec<Memo<E::Node>>,
    calls: usize,
    /// The work that the texts made so far in this attempt stand for.
    text_work: u64,
}

/// A range of columns, rows or levels, both ends included.
type Span = (u32, u32);

impl<E: Env> Parser<'_, E> {
    fn fail(&mut self, e: &str) -> f64 {
        if self.error.is_none() {
            self.error = Some(e.to_string());
        }
        0.0
    }

    fn fail_val(&mut self, e: &str) -> Val {
        Val::num(self.fail(e))
    }

    fn at(&self, i: usize) -> Option<char> {
        self.s.get(i).copied()
    }

    fn space(&mut self) {
        while self.at(self.i).is_some_and(is_space) {
            self.i = self.i.saturating_add(1);
        }
    }

    fn eat(&mut self, ch: char) -> bool {
        self.space();
        if self.at(self.i) != Some(ch) {
            return false;
        }
        self.i = self.i.saturating_add(1);
        true
    }

    fn mid(&self, start: usize, end: usize) -> String {
        self.s.get(start..end).unwrap_or_default().iter().collect()
    }

    fn num(&mut self, v: &Val) -> f64 {
        if !v.text {
            return v.d;
        }
        match strict_num(&v.t) {
            Some(d) => d,
            None => self.fail("#VALUE!"),
        }
    }

    fn str(&mut self, v: Val) -> String {
        if !v.text && !v.d.is_finite() {
            self.fail("#NUM!");
        }
        if v.text || !v.t.is_empty() {
            v.t
        } else {
            num_text(v.d)
        }
    }

    /// Whether a text of `bytes` bytes may be made: not if it is longer than a text may
    /// be, or if the work of this recalculation is used up. Asked before the text is made,
    /// so that nothing larger is ever allocated (decision A80).
    fn room(&mut self, bytes: usize) -> bool {
        let work = (bytes as u64 / TEXT_BYTES_PER_WORK).saturating_add(1);
        if bytes > MAX_TEXT_BYTES || !self.env.add_work(work) {
            self.fail("#LIMIT!");
            return false;
        }
        self.text_work = self.text_work.saturating_add(work);
        true
    }

    /// A text that was made from texts already counted and is no longer than they were.
    fn made(&mut self, t: String) -> Val {
        if self.room(t.len()) {
            Val::text(t)
        } else {
            Val::num(0.0)
        }
    }

    /// Stops reading: the formula is nested too deeply.
    fn give_up(&mut self) -> Val {
        self.i = self.s.len();
        self.fail_val("#ERROR!")
    }

    fn concat(&mut self) -> Result<Val, Wait> {
        let mut v = self.expr()?;
        while self.eat('&') {
            let mut left = self.str(v);
            let right = self.expr()?;
            let right = self.str(right);
            v = if self.room(left.len().saturating_add(right.len())) {
                left.push_str(&right);
                Val::text(left)
            } else {
                Val::num(0.0)
            };
        }
        Ok(v)
    }

    fn expr(&mut self) -> Result<Val, Wait> {
        let mut v = self.term()?;
        loop {
            if self.eat('+') {
                let left = self.num(&v);
                let right = self.term()?;
                v = Val::num(left + self.num(&right));
            } else if self.eat('-') {
                let left = self.num(&v);
                let right = self.term()?;
                v = Val::num(left - self.num(&right));
            } else {
                return Ok(v);
            }
        }
    }

    fn term(&mut self) -> Result<Val, Wait> {
        let mut v = self.unary()?;
        loop {
            if self.eat('*') {
                let left = self.num(&v);
                let right = self.unary()?;
                v = Val::num(left * self.num(&right));
            } else if self.eat('/') {
                let left = self.num(&v);
                let right = self.unary()?;
                let d = self.num(&right);
                v = if d != 0.0 {
                    Val::num(left / d)
                } else {
                    self.fail_val("#DIV/0!")
                };
            } else {
                return Ok(v);
            }
        }
    }

    /// Any number of signs and then a primary. A loop, where TreeSheets calls itself, so
    /// that a long row of signs costs no stack.
    fn unary(&mut self) -> Result<Val, Wait> {
        let (mut signed, mut negative) = (false, false);
        loop {
            if self.eat('-') {
                negative = !negative;
            } else if !self.eat('+') {
                break;
            }
            signed = true;
        }
        let v = self.primary()?;
        if !signed {
            return Ok(v);
        }
        let d = self.num(&v);
        Ok(Val::num(if negative { -d } else { d }))
    }

    fn primary(&mut self) -> Result<Val, Wait> {
        if self.depth >= MAX_NESTING {
            return Ok(self.give_up());
        }
        self.depth = self.depth.saturating_add(1);
        let v = self.primary_inner();
        self.depth = self.depth.saturating_sub(1);
        v
    }

    fn primary_inner(&mut self) -> Result<Val, Wait> {
        if self.eat('(') {
            let v = self.concat()?;
            return Ok(if self.eat(')') {
                v
            } else {
                self.fail_val("#ERROR!")
            });
        }
        if self.eat('\'') {
            return Ok(self.literal());
        }
        if self.at(self.i) == Some('"') {
            let Some(name) = self.name(')') else {
                return Ok(self.fail_val("#ERROR!"));
            };
            return self.one_named(&name);
        }
        let start = self.i;
        let digit = |c: char| c.is_ascii_digit() || c == '.';
        if self.at(self.i).is_some_and(digit) {
            while self.at(self.i).is_some_and(digit) {
                self.i = self.i.saturating_add(1);
            }
            if matches!(self.at(self.i), Some('e' | 'E')) {
                self.i = self.i.saturating_add(1);
                if matches!(self.at(self.i), Some('+' | '-')) {
                    self.i = self.i.saturating_add(1);
                }
                while self.at(self.i).is_some_and(|c| c.is_ascii_digit()) {
                    self.i = self.i.saturating_add(1);
                }
            }
            // Not infinite here: a literal has no "inf", and too large is no number.
            return Ok(match strict_num_literal(&self.mid(start, self.i)) {
                Some(d) => Val::num(d),
                None => self.fail_val("#ERROR!"),
            });
        }
        // Words separated by spaces, e.g. =Unit price * 2, but SUM (Costs) is a function.
        while self.at(self.i).is_some_and(is_name_char) {
            self.i = self.i.saturating_add(1);
            let mut next = self.i;
            while self.at(next) == Some(' ') {
                next = next.saturating_add(1);
            }
            if self.at(next).is_some_and(is_name_char) {
                self.i = next;
            }
        }
        let name = self.mid(start, self.i);
        if name.is_empty() {
            return Ok(self.fail_val("#ERROR!"));
        }
        if !self.eat('(') {
            return self.one_named(&name);
        }
        let function: String = name.chars().map(upper).collect();
        if is_subset(&function) {
            let contents = self.subset(false, &function)?;
            return Ok(self.one(contents));
        }
        if let Some((_, args)) = TEXT_FUNCTIONS.iter().find(|(n, _)| *n == function) {
            return self.text_function(&function, *args);
        }
        let (tree, function) = match function.strip_prefix("TREE") {
            Some(rest) => (true, rest),
            None => (false, function.as_str()),
        };
        let Some((name, f)) = FUNCTIONS.iter().find(|(n, _)| *n == function).copied() else {
            return Ok(self.fail_val("#NAME?"));
        };
        Ok(match self.argument(tree)? {
            Ok(contents) => Val::num(contents.aggregate(name, f)),
            Err(e) => self.fail_val(&e),
        })
    }

    /// The rest of a literal text, after its opening quote.
    fn literal(&mut self) -> Val {
        let mut t = String::new();
        while let Some(c) = self.at(self.i) {
            self.i = self.i.saturating_add(1);
            if c != '\'' {
                t.push(c);
            } else if self.at(self.i) == Some('\'') {
                t.push('\'');
                self.i = self.i.saturating_add(1);
            } else {
                return self.made(t);
            }
        }
        self.fail_val("#ERROR!")
    }

    /// The rest of a call of one of the text functions.
    fn text_function(&mut self, function: &str, nargs: usize) -> Result<Val, Wait> {
        let mut a = Vec::new();
        loop {
            a.push(self.concat()?);
            if !self.eat(',') {
                break;
            }
        }
        if !self.eat(')') || a.len() != nargs {
            return Ok(self.fail_val("#ERROR!"));
        }
        let mut a = a.into_iter();
        let mut arg = || a.next().unwrap_or_else(|| Val::num(0.0));
        let (first, second, third) = (arg(), arg(), arg());
        let t = self.str(first);
        let part = |from: usize, len: usize| -> String { t.chars().skip(from).take(len).collect() };
        Ok(match function {
            #[allow(clippy::cast_precision_loss)]
            "LEN" => Val::num(t.chars().count() as f64),
            "UPPER" => self.made(t.chars().map(upper).collect()),
            "LOWER" => self.made(t.chars().map(lower).collect()),
            "TRIM" => self.made(trim(&t).to_string()),
            "LEFT" => {
                let n = self.count(&second);
                self.made(part(0, n))
            }
            "RIGHT" => {
                let n = self.count(&second);
                self.made(part(t.chars().count().saturating_sub(n), n))
            }
            "MID" => {
                let start = self.count(&second);
                if start < 1 {
                    return Ok(self.fail_val("#VALUE!"));
                }
                let n = self.count(&third);
                self.made(part(start.saturating_sub(1), n))
            }
            _ => {
                let old = self.str(second);
                if old.is_empty() {
                    Val::text(t)
                } else {
                    let new = self.str(third);
                    // How long the result will be, before it is made.
                    let found = t.matches(old.as_str()).count();
                    let len = t
                        .len()
                        .saturating_sub(found.saturating_mul(old.len()))
                        .saturating_add(found.saturating_mul(new.len()));
                    if self.room(len) {
                        Val::text(t.replace(&old, &new))
                    } else {
                        Val::num(0.0)
                    }
                }
            }
        })
    }

    /// A count of characters: a number that is not negative.
    fn count(&mut self, v: &Val) -> usize {
        let d = self.num(v);
        if d >= 0.0 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            return d.min(1e9) as usize;
        }
        self.fail("#VALUE!");
        0
    }

    /// Runs one call into the environment, or takes its result from an earlier attempt.
    fn call(
        &mut self,
        run: impl FnOnce(&mut Self) -> Collected<E::Node>,
    ) -> Result<Collected<E::Node>, Wait> {
        let index = self.calls;
        self.calls = self.calls.saturating_add(1);
        // If this call has to wait, the formula is read again from its start, and the texts
        // made so far are made again: that is counted here, once, whether it happens or
        // not, so that what a formula costs does not depend on who had to wait for whom.
        let again = (self.i as u64 / TEXT_BYTES_PER_WORK).saturating_add(self.text_work);
        self.env.add_work(again);
        if let Some(memo) = self.memo.get(index) {
            self.i = memo.end;
            self.env.add_work(memo.work);
            return Ok(memo.result.clone());
        }
        let before = self.env.work();
        let result = run(self);
        if self.env.waiting() {
            return Err(Wait);
        }
        self.memo.push(Memo {
            end: self.i,
            result: result.clone(),
            work: self.env.work().saturating_sub(before),
        });
        Ok(result)
    }

    /// The contents of the cell named `name`: of its grid, or its own if it has none
    /// (`FormulaEvaluator::Collect` by name).
    fn collect_name(&mut self, name: &str, tree: bool) -> Result<Collected<E::Node>, Wait> {
        self.call(|p| {
            let node = p.env.named(name)?;
            if p.env.size(node).is_some() {
                p.env.collect_grid(node, tree)
            } else {
                let mut contents = Contents::default();
                p.env.collect(node, tree, false, &mut contents)?;
                Ok(Rc::new(contents))
            }
        })
    }

    fn one_named(&mut self, name: &str) -> Result<Val, Wait> {
        let contents = self.collect_name(name, false)?;
        Ok(self.one(contents))
    }

    /// The contents of the parentheses of a function: the name of a grid, or a `SUBSET()`.
    fn argument(&mut self, tree: bool) -> Result<Collected<E::Node>, Wait> {
        self.space();
        let start = self.i;
        while self.at(self.i).is_some_and(is_letter) {
            self.i = self.i.saturating_add(1);
        }
        let function: String = self.mid(start, self.i).chars().map(upper).collect();
        if is_subset(&function) && self.eat('(') {
            let contents = self.subset(tree, &function)?;
            return Ok(contents.and_then(|contents| {
                if self.eat(')') {
                    Ok(contents)
                } else {
                    Err("#ERROR!".to_string())
                }
            }));
        }
        self.i = start;
        let name = self.name(')');
        match name {
            Some(name) if self.eat(')') => self.collect_name(&name, tree),
            _ => Ok(Err("#ERROR!".to_string())),
        }
    }

    /// A name, until `end` or in quotes.
    fn name(&mut self, end: char) -> Option<String> {
        let quoted = self.eat('"');
        let stop = if quoted { '"' } else { end };
        let rest = self.s.get(self.i..)?;
        let e = self.i.checked_add(rest.iter().position(|c| *c == stop)?)?;
        let name = self.mid(self.i, e);
        self.i = if quoted { e.saturating_add(1) } else { e };
        Some(if quoted {
            name
        } else {
            trim_end(&name).to_string()
        })
    }

    /// The one number of a name or `SUBSET()` on its own, or else its one text.
    fn one(&mut self, contents: Collected<E::Node>) -> Val {
        let contents = match contents {
            Ok(contents) => contents,
            Err(e) => return self.fail_val(&e),
        };
        let (node, d) = match (contents.numbers.as_slice(), contents.number, contents.text) {
            ([d], Some(node), _) => (node, Some(*d)),
            ([], _, Some(node)) if contents.texts == 1 => (node, None),
            _ => return self.fail_val("#VALUE!"),
        };
        if !self.room(self.env.text(node).len()) {
            return Val::num(0.0);
        }
        let t = self.env.text(node).to_string();
        match d {
            Some(d) => Val { d, t, text: false },
            None => Val::text(t),
        }
    }

    /// The rest of `SUBSET(name, columns, rows)` or `TREESUBSET(name, columns, rows,
    /// levels)`; with `tree` also what is inside the cells, which adds nothing to
    /// `TREESUBSET`.
    fn subset(&mut self, tree: bool, function: &str) -> Result<Collected<E::Node>, Wait> {
        let function = function.to_string();
        self.call(move |p| {
            let mut cells = Vec::new();
            p.cells(&mut cells, &function, 0)?;
            let mut contents = Contents::default();
            for (cell, sub) in cells {
                p.env
                    .collect(cell, tree && function != "TREESUBSET", sub, &mut contents)?;
            }
            Ok(Rc::new(contents))
        })
    }

    /// The cells of the rest of a `SUBSET()` or `TREESUBSET()`, and whether each is in a
    /// grid inside the named one. Instead of a name it can take another of the two, whose
    /// cells' grids it then uses.
    fn cells(&mut self, cells: &mut Vec<(E::Node, bool)>, function: &str, nesting: u32) -> Check {
        let error = || Err("#ERROR!".to_string());
        if self.depth.saturating_add(nesting) >= MAX_NESTING {
            self.i = self.s.len();
            return error();
        }
        let mut grids = Vec::new();
        self.space();
        let start = self.i;
        while self.at(self.i).is_some_and(is_letter) {
            self.i = self.i.saturating_add(1);
        }
        let inner: String = self.mid(start, self.i).chars().map(upper).collect();
        let nested = is_subset(&inner) && self.eat('(');
        let mut name = String::new();
        if nested {
            let mut picked = Vec::new();
            self.cells(&mut picked, &inner, nesting.saturating_add(1))?;
            grids.extend(
                picked
                    .into_iter()
                    .map(|(cell, _)| cell)
                    .filter(|cell| self.env.size(*cell).is_some()),
            );
        } else {
            self.i = start;
            match self.name(',') {
                Some(n) => name = n,
                None => return error(),
            }
        }
        let levels = function == "TREESUBSET";
        let (Some(xs), Some(ys)) = (self.comma_range(), self.comma_range()) else {
            return error();
        };
        let ls = if levels {
            match self.comma_range() {
                Some(ls) => ls,
                None => return error(),
            }
        } else {
            (0, 0)
        };
        if !self.eat(')') {
            return error();
        }
        if !nested {
            let named = self.env.named(&name)?;
            if self.env.size(named).is_some() {
                grids.push(named);
            }
        }
        if grids.is_empty() {
            return Err("#VALUE!".to_string());
        }
        // Clipped to each grid, so that it can grow and shrink.
        for grid in grids {
            self.add_cells(cells, grid, 0, xs, ys, ls);
        }
        Ok(())
    }

    /// The cells of `grid` in the columns and rows asked for, each followed by those of its
    /// own grid while the levels go on. Calls itself no deeper than grids nest.
    fn add_cells(
        &mut self,
        cells: &mut Vec<(E::Node, bool)>,
        grid: E::Node,
        level: u32,
        xs: Span,
        ys: Span,
        ls: Span,
    ) {
        if level > super::MAX_VALUE_DEPTH {
            return;
        }
        self.env.read_grid(grid);
        let Some((gx, gy)) = self.env.size(grid) else {
            return;
        };
        for y in ys.0..=ys.1.min(gy.saturating_sub(1)) {
            for x in xs.0..=xs.1.min(gx.saturating_sub(1)) {
                let Some(cell) = self.env.child(grid, x, y) else {
                    continue;
                };
                if level >= ls.0 {
                    cells.push((cell, level > 0));
                }
                if level < ls.1 && self.env.size(cell).is_some() {
                    self.add_cells(cells, cell, level.saturating_add(1), xs, ys, ls);
                }
            }
        }
    }

    /// A comma and then a range.
    fn comma_range(&mut self) -> Option<Span> {
        if !self.eat(',') {
            return None;
        }
        self.range()
    }

    /// A column or row number, or a range of them: `0-3`, `3-` (to the end), `-4` (from
    /// the start) or `-` for all.
    fn range(&mut self) -> Option<Span> {
        let from = self.index();
        let lo = from.unwrap_or(0);
        if !self.eat('-') {
            return from.map(|lo| (lo, lo));
        }
        Some((lo, self.index().unwrap_or(i32::MAX as u32)))
    }

    fn index(&mut self) -> Option<u32> {
        self.space();
        let start = self.i;
        while self.at(self.i).is_some_and(|c| c.is_ascii_digit()) {
            self.i = self.i.saturating_add(1);
        }
        if self.i == start {
            return None;
        }
        // Too large for 64 bits is no number, and anything over 2^31 - 1 counts as that.
        let n = self.mid(start, self.i).parse::<u64>().ok()?;
        Some(u32::try_from(n.min(i32::MAX as u64)).unwrap_or(i32::MAX as u32))
    }
}

/// A number written in a formula: digits, points and an exponent, read like the text of a
/// cell.
fn strict_num_literal(text: &str) -> Option<f64> {
    // The reader of cell texts would also take a hexadecimal number, but "x" is not among
    // the characters of a literal, so none gets here.
    strict_num(text)
}

/// The result of a formula: the text it shows, and whether that is an error.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Outcome {
    pub(crate) text: String,
    pub(crate) failed: bool,
}

/// Works out the formula `formula` (with its `=`). `Err(Wait)` if the environment has to
/// bring other formulas up to date first; `memo` then keeps what was already done.
pub(crate) fn evaluate<E: Env>(
    formula: &str,
    env: &mut E,
    memo: &mut Vec<Memo<E::Node>>,
) -> Result<Outcome, Wait> {
    let s: Vec<char> = formula
        .chars()
        .take(MAX_FORMULA_LEN.saturating_add(1))
        .collect();
    if s.len() > MAX_FORMULA_LEN {
        return Ok(Outcome {
            text: "#ERROR!".to_string(),
            failed: true,
        });
    }
    let mut p = Parser {
        env,
        s,
        i: 1,
        error: None,
        depth: 0,
        memo,
        calls: 0,
        text_work: 0,
    };
    let v = p.concat()?;
    p.space();
    if p.i < p.s.len() {
        p.fail("#ERROR!");
    }
    if !v.text && !v.d.is_finite() {
        p.fail("#NUM!");
    }
    Ok(match p.error {
        Some(text) => Outcome { text, failed: true },
        None => Outcome {
            text: if v.text { v.t } else { num_text(v.d) },
            failed: false,
        },
    })
}
