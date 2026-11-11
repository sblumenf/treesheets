//! Find and replace (TreeSheets' "replace in selection" and "replace all").
//!
//! As in TreeSheets, every place where the searched text is found in the text of a cell
//! is replaced, from left to right, each match starting after the one before it ends;
//! notes are not touched. The replacement is put in as it is written, whatever the case
//! of what it replaces. Whether case counts in finding is the word's own setting
//! ([`Term::exact`]; TreeSheets has one switch for all searches); a word that is found
//! in folded form also finds accented and full-width forms ([`crate::fold`]), which
//! TreeSheets' lower-casing does not.
//!
//! Two things are never replaced without being asked for (decisions A105, A106). A match
//! that covers only part of a character: `s` is found in `Straße` twice, but only the
//! first is a character of its own, and replacing the `ß` would destroy a letter that
//! was not searched for. And formula cells, unless [`ReplaceOptions::formulas`] is set.
//! Both are counted and reported, by the dry run ([`plan`]) as by the replace itself.
//!
//! The change is made of ordinary `ReplaceText` commands, so the styles and links of the
//! replaced text follow the rules of [`wombat_core::text`], and one call is one unit of
//! undo for the document.

use wombat_core::{CellId, CellKind, Changes, Command, Document, Editor, Error, IdSource};

use crate::fold::{fold, FoldMap};
use crate::query::Term;

/// Where to replace.
#[derive(Clone, Copy, Debug)]
pub enum Scope<'a> {
    /// In every cell of the document.
    Document,
    /// In this cell and every cell inside it.
    Branch(CellId),
    /// In these cells only (TreeSheets' "replace in current selection").
    Cells(&'a [CellId]),
}

/// What a replace may touch.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ReplaceOptions {
    /// Whether formula cells are changed too. Off unless asked for (decision A106): a
    /// formula is not prose, and a word replaced in every `=SUM(` breaks every sum.
    /// With it, the formula text is what is searched and replaced, and the cell
    /// calculates again.
    pub formulas: bool,
}

/// A place where the searched text was found but not replaced, because the match
/// covers only part of a character there (see [`places`]).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Skipped {
    /// The cell.
    pub cell: CellId,
    /// Where the characters are that the match touches: start and length in code points.
    pub start: u32,
    /// See `start`.
    pub len: u32,
    /// Those characters.
    pub text: String,
}

/// What a replace found, and what it did or would do.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Counts {
    /// How many cells are changed.
    pub cells: usize,
    /// How many places are replaced.
    pub places: usize,
    /// The places that are left alone because the match covers only part of a
    /// character there, in the cells that the replace looks at.
    pub skipped: Vec<Skipped>,
    /// How many formula cells have places that could be replaced. They are among
    /// `cells` only with [`ReplaceOptions::formulas`]; without it they are left alone.
    pub formula_cells: usize,
    /// How many places those formula cells have.
    pub formula_places: usize,
    /// Whether formula cells were included ([`ReplaceOptions::formulas`]).
    pub formulas_included: bool,
}

/// What a replace would do: the dry run ([`plan`]).
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Plan {
    /// What was found.
    pub counts: Counts,
    /// The commands that make the change.
    pub commands: Vec<Command>,
}

/// What a replace did.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Replaced {
    /// What was found and replaced.
    pub counts: Counts,
    /// What the editor reports as changed.
    pub changes: Changes,
}

/// The places where a term is found in a text ([`places`]): start and length in code
/// points, in order.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Places {
    /// Where the match is a whole number of characters: these are replaced. None
    /// overlaps another.
    pub whole: Vec<(u32, u32)>,
    /// Where the match covers only part of a character (the `s` in `ß`, the `f` in
    /// `ﬁ`, one person of a family emoji): the characters it touches. These are never
    /// replaced, since that would destroy what was not searched for. A place that
    /// overlaps a replaced one is not listed.
    pub partial: Vec<(u32, u32)>,
}

/// The places where `term` is found in `text`.
///
/// Going from left to right, a match is taken where the searched text, compared in
/// folded form (or exactly, for an exact term), is the whole of one or more
/// user-perceived characters of `text`; the next match starts after it. A match that
/// begins or ends inside a character (`s` finds half of `ß`, which folds to `ss`) is not
/// a place to replace: it is listed as partial, and the search goes on one character
/// further.
pub fn places(text: &str, term: &Term) -> Places {
    let (map, needle) = if term.exact {
        (FoldMap::literal(text), term.text.clone())
    } else {
        (FoldMap::new(text), fold(&term.text))
    };
    let mut out = Places::default();
    if needle.is_empty() {
        return out;
    }
    let mut partial: Vec<(u32, u32)> = Vec::new();
    let mut at = 0usize;
    while let Some(found) = map.folded.get(at..).and_then(|rest| rest.find(&needle)) {
        let from = at + found;
        let to = from + needle.len();
        let range = map.range(from, to);
        if map.is_whole(from, to) && range.1 > 0 {
            out.whole.push(range);
            at = to;
        } else {
            if partial.last() != Some(&range) && range.1 > 0 {
                partial.push(range);
            }
            // One character on: a whole match may start inside this partial one.
            let step = map.folded.get(from..).and_then(|rest| rest.chars().next());
            at = from + step.map_or(1, char::len_utf8);
        }
    }
    let overlaps = |a: &(u32, u32), b: &(u32, u32)| a.0 < b.0 + b.1 && b.0 < a.0 + a.1;
    partial.retain(|place| !out.whole.iter().any(|whole| overlaps(place, whole)));
    out.partial = partial;
    out
}

/// The cells of a scope, in document order.
fn cells(doc: &Document, scope: Scope) -> Vec<CellId> {
    match scope {
        Scope::Document => doc.cells_in_order(),
        Scope::Cells(cells) => cells.to_vec(),
        Scope::Branch(top) => {
            let mut out = Vec::new();
            let mut stack = vec![top];
            while let Some(id) = stack.pop() {
                let Some(cell) = doc.cell(id) else { continue };
                if out.len() >= doc.cell_count() {
                    break;
                }
                out.push(id);
                stack.extend(
                    cell.grid()
                        .map_or(&[][..], |grid| grid.cells())
                        .iter()
                        .rev(),
                );
            }
            out
        }
    }
}

/// What replacing `term` by `with` in the texts of the cells of `scope` would do, and
/// the commands that do it: the dry run. Within a cell the commands go from the last
/// place to the first, so that each command's position is right when its turn comes.
pub fn plan(
    doc: &Document,
    term: &Term,
    with: &str,
    scope: Scope,
    options: ReplaceOptions,
) -> Plan {
    let mut plan = Plan::default();
    plan.counts.formulas_included = options.formulas;
    for id in cells(doc, scope) {
        let Some(cell) = doc.cell(id) else { continue };
        let found = places(&cell.text, term);
        if cell.kind == CellKind::Formula {
            let has = usize::from(!found.whole.is_empty());
            plan.counts.formula_cells = plan.counts.formula_cells.saturating_add(has);
            let more = plan.counts.formula_places.saturating_add(found.whole.len());
            plan.counts.formula_places = more;
            if !options.formulas {
                continue;
            }
        }
        let skipped = found.partial.iter().map(|(start, len)| Skipped {
            cell: id,
            start: *start,
            len: *len,
            text: (cell.text.chars().skip(*start as usize))
                .take(*len as usize)
                .collect(),
        });
        plan.counts.skipped.extend(skipped);
        plan.counts.cells += usize::from(!found.whole.is_empty());
        plan.counts.places += found.whole.len();
        for (start, len) in found.whole.into_iter().rev() {
            plan.commands.push(Command::ReplaceText {
                cell: id,
                start,
                len,
                text: with.to_owned(),
            });
        }
    }
    plan
}

/// Replaces `term` by `with` in the texts of the cells of `scope`, as one unit of undo.
/// `now` stamps the changed cells' edit times, as for any text command. Formula cells
/// are left alone unless `options` says otherwise, and so is every place where the
/// match covers only part of a character ([`places`]); both are counted in the result.
///
/// If a command is refused (the replacement would join up with the text beside it into
/// one user-perceived character, which text commands do not allow), everything is put
/// back and the refusal returned. Not allowed while a group of commands is open.
pub fn replace(
    editor: &mut Editor,
    term: &Term,
    with: &str,
    scope: Scope,
    options: ReplaceOptions,
    now: Option<i64>,
    ids: &mut dyn IdSource,
) -> Result<Replaced, Error> {
    if editor.in_group() {
        return Err(Error::Refused(
            "replace while a group of commands is open".into(),
        ));
    }
    let Plan { counts, commands } = plan(editor.document(), term, with, scope, options);
    let units = editor.undo_len();
    editor.begin_group();
    let mut failed = None;
    for command in commands {
        if let Err(error) = editor.apply(command, now, ids) {
            failed = Some(error);
            break;
        }
    }
    let changes = editor.end_group();
    if let Some(error) = failed {
        if editor.undo_len() > units {
            editor.undo()?;
            editor.clear_redo();
        }
        return Err(error);
    }
    Ok(Replaced { counts, changes })
}
