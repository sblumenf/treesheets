//! Copying and moving cells from one document to another (decisions A50 and A51).
//!
//! An [`Editor`] edits one document, so a move between documents is two changes: one in
//! each document's history. [`move_between`] makes both, or neither.

use crate::edit::{Changes, Clip, Command, Editor};
use crate::error::{refused, Error};
use crate::id::{CellId, IdSource};
use crate::model::Document;

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
/// root, if `to` is not empty, if an ID of the moved cells is in use in `target`, or while
/// a group is open in either editor. Cells can move back to a document they left, also
/// while its history still holds them (decision A84). Reads both documents whole once,
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
    let moved: Vec<CellId> = clip.cell_ids().collect();

    // The links of the target are corrected first: afterwards the moved cells' own links
    // would be found among them. In place, so that a link keeps its range, its unknown
    // fields and its neighbours (decision A85).
    let commands = vec![
        Command::RetargetLinks {
            cells: moved.clone(),
            from: Some(from_doc),
            to: Some(to_doc),
        },
        Command::AdoptCells { to, clip },
    ];
    let arrived = apply_all(target, commands, now, ids)?;

    // In the source the release comes first: the links of the cells that leave are gone
    // by then, and only those of the cells that stay are corrected.
    let commands = vec![
        Command::ReleaseCell { cell },
        Command::RetargetLinks {
            cells: moved,
            from: None,
            to: Some(to_doc),
        },
    ];
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
