//! Copying and moving cells from one document to another (decisions A50 and A51).
//!
//! An [`Editor`] edits one document, so a move between documents is two changes: one in
//! each document's history. [`move_between`] makes both, or neither.

use std::collections::HashSet;

use crate::edit::{Changes, Clip, Command, Editor};
use crate::error::{refused, Error};
use crate::id::{CellId, DocId, IdSource};
use crate::model::{Document, LinkTarget};

/// Makes the cell `to` of the document in `target` a copy of the cell `cell` of `source`
/// with everything inside it ([`Command::PasteCells`]). The copies inside get new IDs;
/// links in the copy that pointed at cells outside it still point at those cells of
/// `source`. One unit of `target`'s history; `source` is only read.
pub fn copy_between(
    source: &Document,
    cell: CellId,
    target: &mut Editor,
    to: CellId,
    ids: &mut dyn IdSource,
) -> Result<Changes, Error> {
    let clip = Clip::of(source, cell)?;
    target.apply(Command::PasteCells { to, clip }, None, ids)
}

/// What [`move_between`] changed in each of the two documents.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Moved {
    /// What changed in the document the cells left.
    pub source: Changes,
    /// What changed in the document the cells arrived in.
    pub target: Changes,
}

/// The commands that make the links of `doc` to any of `cells`, which name the document
/// `from` (`None`: no document, the cells were its own), name the document `to` instead.
/// Reads the whole document.
fn retarget(
    doc: &Document,
    cells: &HashSet<CellId>,
    from: Option<DocId>,
    to: DocId,
) -> Vec<Command> {
    let mut out = Vec::new();
    for id in doc.cells_in_order() {
        let Some(found) = doc.cell(id) else { continue };
        for link in &found.links {
            match link.target {
                LinkTarget::Cell { doc: named, cell } if named == from && cells.contains(&cell) => {
                    out.push(Command::SetLink {
                        cell: id,
                        start: link.start,
                        len: link.len,
                        target: Some(LinkTarget::Cell {
                            doc: Some(to),
                            cell,
                        }),
                    });
                }
                _ => {}
            }
        }
    }
    out
}

/// Applies commands as one group; on an error the group is taken back.
fn apply_all(
    editor: &mut Editor,
    commands: Vec<Command>,
    now: Option<i64>,
    ids: &mut dyn IdSource,
) -> Result<Changes, Error> {
    editor.begin_group();
    let mut failed = None;
    let mut did = false;
    for command in commands {
        match editor.apply(command, now, ids) {
            Ok(changes) => did |= !changes.is_empty(),
            Err(error) => {
                failed = Some(error);
                break;
            }
        }
    }
    let changes = editor.end_group();
    match failed {
        None => Ok(changes),
        Some(error) => {
            if did {
                take_back(editor)?;
            }
            Err(error)
        }
    }
}

/// Undoes the newest unit of the history and forgets it. (An editor whose history is set
/// to keep no units cannot take anything back; then the unit stays done.)
fn take_back(editor: &mut Editor) -> Result<(), Error> {
    if editor.undo()?.is_some() {
        editor.clear_redo();
    }
    Ok(())
}

/// Moves the cell `cell`, with everything inside it, from the document in `source` to
/// where the empty cell `to` of the document in `target` is. The moved cells keep their
/// IDs, so links to them survive (decision B1):
///
/// - in `target` the cells arrive with their IDs ([`Command::AdoptCells`]); `to` is removed;
/// - in `source` they are gone ([`Command::ReleaseCell`]) and a new empty cell stands where
///   `cell` was;
/// - links between the moved cells are unchanged; their links to cells that stay in
///   `source` now name that document, and their links to cells of `target` no longer name one;
/// - links to the moved cells from the other cells of `source` now name `target`'s
///   document, and those from the cells of `target` no longer name a document;
/// - links from other documents are not touched. They still name `source`'s document and
///   are found by the cell's ID ([`super::Resolved::Cell`] with `moved`); when such a
///   document is edited, [`super::LinkIndex::repairs`] corrects them.
///
/// The move is one unit in each of the two histories, and **not one unit of undo**: there
/// is no history above the documents. Undoing it in both documents gives both back exactly
/// as they were. Undoing it in one only leaves the cells in neither document (undone in
/// `target`) or in both (undone in `source`), until the other is undone or the first
/// redone; nothing is lost in either case, and a workspace reports the second as shared
/// IDs. If the second half fails here, the first is taken back and the error returned,
/// with both documents as they were (what `target` could redo before is forgotten).
///
/// Refused if the documents are the same (use [`Command::MoveCell`]), if `cell` is the
/// root, if `to` is not empty, if an ID of the moved cells is in use in `target` or its
/// history, or while a group is open in either editor. Reads both documents whole once,
/// to find the links to correct.
pub fn move_between(
    source: &mut Editor,
    cell: CellId,
    target: &mut Editor,
    to: CellId,
    now: Option<i64>,
    ids: &mut dyn IdSource,
) -> Result<Moved, Error> {
    let (from_doc, to_doc) = (source.document().id(), target.document().id());
    if from_doc == to_doc {
        return refused("both are the same document; move cells within it with MoveCell");
    }
    if source.in_group() || target.in_group() {
        return refused("cells cannot move between documents while a group of commands is open");
    }
    if cell == source.document().root() {
        return refused("the root cell cannot leave its document");
    }
    let clip = Clip::of(source.document(), cell)?;
    let moved: HashSet<CellId> = clip.cell_ids().collect();

    // The links of the target are corrected first: afterwards the moved cells' own links
    // would be found among them.
    let mut commands = retarget(target.document(), &moved, Some(from_doc), to_doc);
    commands.push(Command::AdoptCells { to, clip });
    let arrived = apply_all(target, commands, now, ids)?;

    let mut commands = vec![Command::ReleaseCell { cell }];
    commands.extend(retarget_after_release(source.document(), &moved, to_doc));
    match apply_all(source, commands, now, ids) {
        Ok(left) => Ok(Moved {
            source: left,
            target: arrived,
        }),
        Err(error) => {
            take_back(target)?;
            Err(error)
        }
    }
}

/// As [`retarget`] for the document the cells leave, without the links of the cells that
/// leave (the release comes first in the group, and they are gone by then).
fn retarget_after_release(doc: &Document, moved: &HashSet<CellId>, to: DocId) -> Vec<Command> {
    let mut commands = retarget(doc, moved, None, to);
    commands.retain(|command| match command {
        Command::SetLink { cell, .. } => !moved.contains(cell),
        _ => true,
    });
    commands
}
