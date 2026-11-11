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
//! The change is made of ordinary `ReplaceText` commands, so the styles and links of the
//! replaced text follow the rules of [`wombat_core::text`], and one call is one unit of
//! undo for the document.

use wombat_core::{CellId, Changes, Command, Document, Editor, Error, IdSource};

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

/// What a replace did.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Replaced {
    /// How many cells were changed.
    pub cells: usize,
    /// How many places were replaced.
    pub places: usize,
    /// What the editor reports as changed.
    pub changes: Changes,
}

/// The places where `term` is found in `text`, in order: start and length in code
/// points, whole user-perceived characters, none overlapping another.
pub fn places(text: &str, term: &Term) -> Vec<(u32, u32)> {
    let (map, needle) = if term.exact {
        (FoldMap::literal(text), term.text.clone())
    } else {
        (FoldMap::new(text), fold(&term.text))
    };
    let mut out: Vec<(u32, u32)> = Vec::new();
    if needle.is_empty() {
        return out;
    }
    let mut at = 0usize;
    while let Some(found) = map.folded.get(at..).and_then(|rest| rest.find(&needle)) {
        let from = at + found;
        at = from + needle.len();
        let (start, len) = map.range(from, at);
        // Two matches inside one character (`s` twice in the `ss` that `ß` folds to)
        // are one place.
        let free = out.last().is_none_or(|last| last.0 + last.1 <= start);
        if free && len > 0 {
            out.push((start, len));
        }
    }
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

/// The commands that replace `term` by `with` in the texts of the cells of `scope`,
/// with the number of cells they change. Within a cell they go from the last place to
/// the first, so that each command's position is right when its turn comes.
pub fn commands(doc: &Document, term: &Term, with: &str, scope: Scope) -> (Vec<Command>, usize) {
    let mut out = Vec::new();
    let mut changed = 0usize;
    for id in cells(doc, scope) {
        let Some(cell) = doc.cell(id) else { continue };
        let found = places(&cell.text, term);
        changed += usize::from(!found.is_empty());
        for (start, len) in found.into_iter().rev() {
            out.push(Command::ReplaceText {
                cell: id,
                start,
                len,
                text: with.to_owned(),
            });
        }
    }
    (out, changed)
}

/// Replaces `term` by `with` in the texts of the cells of `scope`, as one unit of undo.
/// `now` stamps the changed cells' edit times, as for any text command.
///
/// If a command is refused (the replacement would join up with the text beside it into
/// one user-perceived character, which text commands do not allow), everything is put
/// back and the refusal returned. Not allowed while a group of commands is open.
pub fn replace(
    editor: &mut Editor,
    term: &Term,
    with: &str,
    scope: Scope,
    now: Option<i64>,
    ids: &mut dyn IdSource,
) -> Result<Replaced, Error> {
    if editor.in_group() {
        return Err(Error::Refused(
            "replace while a group of commands is open".into(),
        ));
    }
    let (list, cells) = commands(editor.document(), term, with, scope);
    let places = list.len();
    let units = editor.undo_len();
    editor.begin_group();
    let mut failed = None;
    for command in list {
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
    Ok(Replaced {
        cells,
        places,
        changes,
    })
}
