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
    paste_clip(source, Clip::of(source, cell)?, target, to, ids)
}

/// As [`copy_between`] with the clip already taken from `source`, for a caller that had to
/// change it first: [`crate::storage::carry_images`] gives images other file names where
/// the target's image folder has other files of the same name. The tags that the copied
/// cells have in `source` and that `target` lacks are added to it, in the same unit of
/// its history (decision B70).
pub fn paste_clip(
    source: &Document,
    clip: Clip,
    target: &mut Editor,
    to: CellId,
    ids: &mut dyn IdSource,
) -> Result<Changes, Error> {
    if clip.source() != source.id() {
        return refused("the cells were not taken from that document");
    }
    let mut commands = tags_for(source, &clip, target.document());
    let paste = Command::PasteCells { to, clip };
    if commands.is_empty() {
        return target.apply(paste, None, ids);
    }
    if target.in_group() {
        // Part of the caller's unit, which reports the tags when it ends.
        for command in commands {
            target.apply(command, None, ids)?;
        }
        return target.apply(paste, None, ids);
    }
    commands.push(paste);
    apply_all(target, commands, None, ids)
}

/// The commands that give `target` the tags that cells of `clip` have in `source` and that
/// it does not have. A tag that it has keeps its colours.
fn tags_for(source: &Document, clip: &Clip, target: &Document) -> Vec<Command> {
    if source.id() == target.id() {
        return Vec::new();
    }
    let mut names: Vec<&str> = clip
        .texts()
        .filter(|text| source.tags().contains_key(*text) && !target.tags().contains_key(*text))
        .collect();
    names.sort_unstable();
    names.dedup();
    let tag = |name: &str| {
        let tag = source.tags().get(name)?;
        Some(Command::SetTag {
            name: name.to_string(),
            cell_color: tag.cell_color,
            text_color: tag.text_color,
        })
    };
    names.into_iter().filter_map(tag).collect()
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
    let clip = Clip::of(source.document(), cell)?;
    move_clip(source, clip, target, to, now, ids)
}

/// As [`move_between`] with the clip already taken from the document in `source`, for a
/// caller that had to change the names of image files in it first (see [`paste_clip`]).
/// Refused if the clip is anything else than its top cell as `source` has it now. The
/// tags that the moved cells have in `source` and that `target` lacks are added to it.
pub fn move_clip(
    source: &mut Editor,
    clip: Clip,
    target: &mut Editor,
    to: CellId,
    now: Option<i64>,
    ids: &mut dyn IdSource,
) -> Result<Moved, Error> {
    let Some(cell) = clip.top() else {
        return refused("nothing to move");
    };
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
    if !clip.is_of(source.document(), cell) {
        return refused("the cells to move are not as the document has them");
    }
    let moved: Vec<CellId> = clip.cell_ids().collect();

    // The links of the target are corrected first: afterwards the moved cells' own links
    // would be found among them. In place, so that a link keeps its range, its unknown
    // fields and its neighbours (decision A85).
    let mut commands = tags_for(source.document(), &clip, target.document());
    commands.extend([
        Command::RetargetLinks {
            cells: moved.clone(),
            from: Some(from_doc),
            to: Some(to_doc),
        },
        Command::AdoptCells { to, clip },
    ]);
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
