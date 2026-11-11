//! Where editing meets calculating (milestone 5, decisions A60-A66): the editor keeps the
//! results of the formula cells up to date, commands can read what a cell shows, and two
//! commands: `Run` (TreeSheets' Program > Run) and `EnterText` (what TreeSheets does when
//! typing into a cell ends).

use std::collections::HashMap;

use crate::error::Error;
use crate::eval::{self, is_formula_text, Calc, RunReport, Value, Variables};
use crate::id::CellId;
use crate::limits::MAX_DEPTH;
use crate::model::{Cell, CellKind, Document, Grid};

use super::command::{self, cell, check_room, depth, descendants, set, Fresh};
use super::step::{Field, Step};
use super::{Changes, Editor, Tx};

/// What a command can see and leave behind besides the document.
#[derive(Default)]
pub(super) struct Ext<'a> {
    /// The results of the formula cells before the command.
    pub(super) calc: Option<&'a Calc>,
    /// What a run assigned and reported, for the editor to keep if the command stands.
    pub(super) ran: Option<(Variables, RunReport)>,
}

impl Tx<'_> {
    /// What a cell shows: the result of a formula cell, the text of any other
    /// (TreeSheets' `text.t`, which sorting and the like go by).
    pub(crate) fn shown(&self, id: CellId) -> &str {
        match self.ext.calc {
            Some(calc) => calc.shown(self.doc, id).unwrap_or(""),
            None => self.doc.cell(id).map_or("", |c| c.text.as_str()),
        }
    }
}

impl Editor {
    /// The results of the formula cells, up to date with the document, and the variables
    /// and report of the last run.
    pub fn calc(&self) -> &Calc {
        &self.calc
    }

    /// What a cell shows: the result of a formula cell, the text of any other.
    pub fn shown(&self, id: CellId) -> Option<&str> {
        self.calc.shown(&self.doc, id)
    }

    /// The formula cells whose results the last command, undo, redo or group changed.
    pub fn recalculated(&self) -> &[CellId] {
        self.calc.recalculated()
    }

    /// Brings the results up to date after a change, taking over what a run left.
    pub(super) fn recalculate(&mut self, ran: Option<(Variables, RunReport)>, changes: &Changes) {
        if let Some((variables, report)) = ran {
            self.calc.ran(variables, report);
        }
        self.calc.update(&self.doc, changes);
    }
}

/// `EnterText`: sets the text, and a data or formula cell becomes a formula cell if the
/// text is a formula and a data cell if not (`Document::EndFormulaEdit`).
pub(super) fn enter_text(
    tx: &mut Tx,
    id: CellId,
    text: &str,
    now: Option<i64>,
) -> Result<(), Error> {
    command::set_text(tx, id, text, now)?;
    let c = cell(tx.doc(), id)?;
    let kind = match c.kind {
        CellKind::Data | CellKind::Formula if is_formula_text(&c.text) => CellKind::Formula,
        CellKind::Data | CellKind::Formula => CellKind::Data,
        other => other,
    };
    if kind != c.kind {
        set(tx, id, Field::Kind(kind))?;
    }
    Ok(())
}

/// The cells of the document that a value becomes: its own cell with the given ID, and
/// the cells inside with new IDs.
struct Made {
    top: Cell,
    grid: Option<Grid>,
    inside: Vec<Cell>,
}

fn make(
    doc: &Document,
    value: &Value,
    id: CellId,
    parent: Option<CellId>,
    now: Option<i64>,
    fresh: &mut Fresh,
) -> Result<Made, Error> {
    let stamp = |v: &Value, mut c: Cell| {
        // A cell that an operation made new is new now.
        if !v.is_copy() {
            c.last_edit = now;
        }
        c
    };
    let top = stamp(value, value.to_cell(id, parent, doc));
    let mut inside = Vec::new();
    // Each grid still to do: the value that has it, and the ID its cell got.
    let mut todo = vec![(value, id)];
    let mut top_grid = None;
    // Where in `inside` the cell with each ID is.
    let mut place: HashMap<CellId, usize> = HashMap::new();
    while let Some((v, owner)) = todo.pop() {
        let Some(grid) = v.grid() else { continue };
        let mut ids = Vec::with_capacity(grid.cells().len());
        for child in grid.cells() {
            ids.push(fresh.next(doc)?);
            todo.push((child, *ids.last().unwrap_or(&owner)));
        }
        for (child, child_id) in grid.cells().iter().zip(&ids) {
            place.insert(*child_id, inside.len());
            inside.push(stamp(child, child.to_cell(*child_id, Some(owner), doc)));
        }
        let made = v.to_grid(ids);
        if owner == id {
            top_grid = made;
        } else if let Some(c) = place.get(&owner).and_then(|n| inside.get_mut(*n)) {
            c.grid = made;
        }
    }
    Ok(Made {
        top,
        grid: top_grid,
        inside,
    })
}

/// Whether the cell `id` and everything in it already is what `value` would make of it
/// (IDs and edit times aside), so that a run that changes nothing is nothing (A44).
fn already(doc: &Document, id: CellId, value: &Value) -> bool {
    let mut todo = vec![(id, value)];
    while let Some((id, value)) = todo.pop() {
        let Some(have) = doc.cell(id) else {
            return false;
        };
        let mut want = value.to_cell(id, have.parent(), doc);
        want.last_edit = have.last_edit;
        let mut own = have.clone();
        own.grid = None;
        if own != want {
            return false;
        }
        match (have.grid(), value.grid()) {
            (None, None) => {}
            (Some(grid), Some(wanted)) => {
                if value.to_grid(grid.cells().to_vec()).as_ref() != Some(grid) {
                    return false;
                }
                todo.extend(grid.cells().iter().copied().zip(wanted.cells()));
            }
            _ => return false,
        }
    }
    true
}

/// `Run`: works the program of the document out and overwrites the view cells with what it
/// gave them. A view cell keeps its ID; the cells of a grid it gets are new.
pub(super) fn run_program(tx: &mut Tx, now: Option<i64>, fresh: &mut Fresh) -> Result<(), Error> {
    let calc = tx.ext.calc;
    let shown = |id: CellId| {
        calc.and_then(|c| c.result(id))
            .map(|(text, _)| text.to_string())
    };
    let doc = tx.doc();
    let outcome = eval::run(doc, &shown).map_err(|limit| {
        Error::Limit(match limit {
            eval::OverLimit::Work => format!(
                "the program nests values deeper than {} or takes more than {} steps",
                eval::MAX_VALUE_DEPTH,
                eval::MAX_RUN_WORK
            ),
            eval::OverLimit::Held => format!(
                "the views and variables of the program would hold more than {} cells or \
                 {} bytes of text with the document",
                eval::MAX_RUN_CELLS,
                eval::MAX_RUN_TEXT
            ),
        })
    })?;
    // Everything is made from the document as it is, before any of it changes: a view can
    // show a copy of a cell that another view overwrites.
    let mut made = Vec::new();
    let mut more = 0usize;
    for (id, value) in &outcome.views {
        if already(doc, *id, value) {
            continue;
        }
        let (count, height) = value.extent();
        if depth(doc, *id)?.saturating_add(height) > MAX_DEPTH {
            return Err(Error::Limit(format!(
                "the run would nest grids deeper than {MAX_DEPTH}"
            )));
        }
        more = more.saturating_add(count);
        check_room(tx, more)?;
        let parent = cell(doc, *id)?.parent();
        made.push((*id, make(doc, value, *id, parent, now, fresh)?));
    }
    for (id, made) in made {
        let had_grid = cell(tx.doc(), id)?.grid().is_some();
        let replaced = descendants(tx.doc(), id);
        command::sync_cell(tx, id, &made.top)?;
        if made.grid.is_some() {
            tx.step(Step::AddCells(made.inside))?;
        }
        if had_grid || made.grid.is_some() {
            tx.step(Step::SetGrid {
                cell: id,
                grid: made.grid,
            })?;
            tx.step(Step::RemoveCells(replaced))?;
        }
    }
    tx.ext.ran = Some((
        outcome.variables,
        RunReport {
            failure: outcome.failure,
            result: outcome.result,
        },
    ));
    Ok(())
}
