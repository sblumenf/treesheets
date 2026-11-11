# Design decisions

Decisions are numbered and never renumbered. To change one, add a new entry that says which one
it replaces and why. Section B lists every place Wombat deliberately behaves differently from
TreeSheets.

## A. Foundations

1. **Three layers.** The core knows what a document is and nothing about screens. Storage reads and
   writes files. An interface (later) shows documents and turns input into commands. Each layer
   talks only to the one beneath it. The proof of concept builds the core and storage only.
2. **Rust** for the core, so that it can later run inside a web interface (as WebAssembly) and
   inside native desktop and phone apps.
3. **Location.** The folder `wombat/` in the TreeSheets fork, to move to its own repository later.
   Nothing in `wombat/` depends on the TreeSheets C++ code at build time.
4. **TreeSheets is the reference and the referee.** Where a behaviour is not specified here, do
   what TreeSheets does (`../../src/`). Tests compare the core against the real TreeSheets app
   through `referee/`. Departures are deliberate and go in section B.
5. **Permanent cell IDs.** Every cell has an ID that never changes for the life of the cell, is
   saved in the file, and is unique across all documents (not only within one). Documents have IDs
   of the same kind.
6. **Workspace.** A workspace is a folder of documents. Links and search work across a workspace.
7. **File format.** One readable text file (JSON) per document with the extension `.wombat`,
   specified in `FILE_FORMAT.md`. It carries a format version number. Content the core does not
   recognise is kept and written back unchanged, so an older core cannot destroy data written by a
   newer one. Saving the same document twice gives byte-identical files. Images are separate files
   beside the document, not embedded in it.
8. **View state is not content.** Cursor, selection, zoom level and which cells are folded are not
   stored in the document file, because they can differ between devices. They are stored
   separately.
9. **Every change is a command.** The only way to change a document is through a command addressed
   by cell ID. Each command is recorded as a small step that can be undone and redone. Undo does
   not rely on keeping copies of whole parts of the document.
10. **The core reports what changed.** Every command returns the list of cells and grids it
    affected, so an interface can redraw only those and the search index can update only those.
11. **Text.** Text is Unicode. Positions and ranges inside a cell's text are counted in Unicode
    code points. Editing commands never split a user-perceived character (for example an emoji made
    of several code points). Right-to-left text is stored in logical order.
12. **Damaged input is harmless.** Loading a damaged, truncated or hostile file returns an error.
    It never crashes, hangs or uses unbounded memory. Saving writes a temporary file and renames
    it, so a failed save cannot leave a half-written document.
13. **The search index is a disposable copy.** It can always be rebuilt from the documents and is
    never the only place any information lives.
14. **Formulas** are TreeSheets' system (operations, variables, views, formula cells), ported and
    checked against the real app.
15. **Licence.** Not yet decided for Wombat as a product. Until it is, the repository's zlib
    licence applies. TreeSheets (c) Wouter van Oortmerssen is credited as the inspiration and as
    the source of the `.cts` format knowledge and the formula system.
16. **IDs are random 128-bit values written as UUIDs.** The core never reads randomness itself:
    whoever creates cells passes in a source of IDs, seeded once from the operating system (tests
    use a fixed seed). Cell IDs and document IDs are different types in the code, so they cannot
    be mixed up. Copying a document file by hand makes two documents with the same IDs; nothing
    detects that yet.
17. **In memory, a document is one map from cell ID to cell.** Each cell records its parent, and
    a grid holds the IDs of its cells rather than the cells. Finding a cell or its parent by ID
    therefore takes the same time whatever the document's size or depth, which commands and undo
    (milestone 3) need, and nothing recurses through owned nested structures. Output order never
    comes from the map: saving walks the tree from the root.
18. **Format versions are major.minor.** A reader refuses a file with a newer major version
    outright. It accepts a newer minor version, because a minor version may only add fields and
    unknown fields are kept (A7). New values for existing fields, such as a new cell kind, need a
    new major version; a reader treats an unknown value of a known field as an error.
19. **Unknown fields are kept as JSON values, not as bytes.** They are written back with their
    keys sorted and numbers respelled if needed, and numbers beyond 64-bit integers and doubles
    are not exact. Room for typed cells (dates and so on) is by new fields, under this rule.
20. **Limits** (A12): files up to 256 MiB, 5,000,000 cells, grids nested 32 deep, 16,777,216
    code points per text. The depth limit follows from the JSON library refusing input nested
    more than 128 deep, which is what keeps hostile files from overflowing the stack; one level
    of grid costs three levels of JSON. TreeSheets has no depth limit, so a `.cts` file nested
    deeper than 32 could not be imported.
21. **Where files go.** Images of `notes.wombat` are in the folder `notes.wombat.images/` beside
    it, referred to by plain file name only. View state is in `.wombat-view/<document ID>.json`
    in the document's folder: keyed by ID so that renaming keeps it, and in one hidden folder so
    that sync can leave it out. A workspace is the `*.wombat` files directly in one folder.
22. **A loaded file is either valid or refused.** The reader does not repair: runs or links out
    of range, duplicate IDs or a grid of the wrong size make the whole file an error. (TreeSheets
    silently drops bad runs.) Saving checks the same rules first, so the core never writes a file
    it would refuse to read.
23. **The `.cts` importer does what TreeSheets' loader does, and refuses where that loader
    would go wrong.** It is a separate crate (`wombat-cts`) so the core carries no `.cts` or
    zlib code. Like the loader it turns control characters in text and tag names into spaces,
    moves or drops runs and links that overlap or leave the text, ignores an image number that
    is not in the file, reads a string that is not UTF-8 as empty (and counts it), limits image
    scales to 1/16..100 and column widths to at least 5, and accepts a tag list that ends with
    the file. Unlike the loader it refuses, as damaged: a file whose compressed part is cut
    short or fails its checksum, more runs or links than characters (the loader then reads the
    rest out of step), and a cell type, draw style or contents byte it does not know.
24. **Importer limits** (A12): the compressed part may decompress to at most 256 MiB; at most
    65,536 images; a grid may have at most 4,194,304 cells (TreeSheets' own limit) and must
    fit in the data that is left, checked before any cell is made; nesting at most 32
    (A20: the deepest example is 15 deep, so the limit was not raised). Zlib is read with
    `miniz_oxide`, pure Rust with one small dependency.
25. **What the importer leaves behind.** It reports, but the document does not keep: the
    "export to HTML/PDF on every save" setting, which Wombat has no feature for. It drops:
    style bits other than the five known ones, the image names of files before version 9
    (TreeSheets ignores them too), and images that no cell shows (returned, not saved).
26. **A file is at most 32 MiB** (replaces the 256 MiB of A20; found in review of milestone 1).
    The reader parses a file into a generic JSON tree and then builds the document, and at the
    peak this takes about 13 times the file's size for an ordinary document (189 MiB for the
    14.2 MB, 101,001-cell test document) and about 17 times in the worst case, a long array of
    small numbers in an unknown field (547 MiB at the limit, measured). At 256 MiB that was
    several gigabytes, more than a phone gives one app; at 32 MiB the worst case stays near
    half a gigabyte, and the limit still holds over 200,000 cells of ordinary text (TreeSheets
    files of that size are rare; its files grow large through embedded images, which Wombat
    keeps outside the document). A reader that builds the document directly, without the tree,
    would cut this several times and is the condition for raising the limit. The `.cts` importer
    keeps its own limit of 256 MiB (A24), because a `.cts` file holds its images. Listing a
    workspace and refusing a newer or foreign file read only the header (`format`, `version`,
    `minor`, `id`, which writers put first) and build nothing, so they cost the same for a file
    of any size; a file damaged after its header is therefore listed and fails when opened.
27. **Saving through links, and permissions.** The temporary file of a save is never opened
    through a link: whatever is at its name is removed and it is created new, so a link planted
    there cannot make a save overwrite another file. If the document's own path is a symbolic
    link, the save goes to the real file (temporary file beside it) and the link stays, because
    that is what someone who links a document into a folder means, and replacing the link would
    silently leave two diverging files; a link that leads nowhere is refused. On Unix the saved
    file keeps the permissions of the one it replaces (a private document stays private); the
    owner cannot be kept without privileges and becomes whoever saves. Image folders and view
    state stay beside the name that was opened.
28. **A copy of a document has IDs of its own** (settles the open end of A16). Copying a
    `.wombat` file by hand makes two files with one document ID, the same cell IDs and one
    shared view-state file, which makes links, backlinks and the search index ambiguous. So a
    copy is made by the core (`Document::with_fresh_ids`, `storage::copy_document`,
    `Workspace::duplicate`, `wombat copy`): new document ID, new cell IDs in document order,
    links to the document's own cells follow the copy (with or without its document ID in
    them), links into other documents and links to cells the document does not have are left
    alone, images are copied, and the view state is copied under the new IDs. IDs written
    inside unknown fields cannot be recognised and are not changed. Hand copies are detected,
    not repaired: `Workspace::list` names the files with the same document ID (from headers
    alone), and `Workspace::id_collisions` loads every document to find shared cell IDs too.
    `Workspace::open_id` still opens the first by name.
29. **Saving produces only what loading accepts, also at the limits** (completes A22; A20's
    "128 deep" becomes a rule). Saving fails with a limit error if the file would be over
    32 MiB. The value of an unknown field may nest arrays and objects at most 24 deep, checked
    when loading and when saving; with grids 32 deep (3 JSON levels each) the deepest possible
    file nests 124 levels, inside the 127 the parser reads, wherever a cell with such a field
    is moved to. The model's consistency check follows the tree with a set of visited cells
    instead of counting them, so a cell listed twice cannot hide one left out. View state has
    the same rules. An object that repeats a key is refused at every level, also inside
    unknown fields, instead of the last one winning, since two readers could disagree on which
    counts.
30. **Image file names must work on every system.** Besides A21: a name may not end with a dot
    and may not be a Windows device name (`CON`, `PRN`, `AUX`, `NUL`, `COM0`-`COM9`,
    `LPT0`-`LPT9`, in any case, alone or before the first dot). A document written on one
    system would otherwise name an image that cannot exist on another.
31. **Two ways to write to a document, kept apart** (carries out A9). *Editing* is
    `edit::Editor`: it owns the document, lends it out read-only, and changes it only through
    commands, which are recorded and reported. *Construction* is the trait
    `construct::Construct` (`cell_mut`, `add_grid`, `tags_mut`, `extra_mut`): unrestricted and
    unrecorded, for code that builds a document before anyone edits it: the file reader, the
    `.cts` importer, tests, `wombat new --from`. Its methods exist only for code that imports
    the trait, and a document inside an editor cannot be reached by them, so no document with
    a history can be changed behind the history's back. `Editor::new` validates what it is
    given. Tags and unknown document fields are read through `tags()` and `extra()`.
32. **Cell text holds no control characters** (replaces B12). A cell's text and a tag's name
    have no character below U+0020: no line breaks or tabs. This is TreeSheets' rule (the lines
    of a cell come from wrapping), the importer already produced only such text, and it keeps
    "one cell, one line of text" true for search and export. Editing commands turn such
    characters into spaces, one for one, as TreeSheets does on load and on `set_text`;
    `validate`, and so loading and saving, refuses a document that has one. Notes may hold any
    text. U+007F and the C1 controls are left alone, as in TreeSheets. This tightens format
    version 1.0 in place instead of making a version 2 (which A18 would ask for), because no
    Wombat files exist outside this repository yet; after a first release it could not be
    done this way.
33. **A command that would cut a user-perceived character is refused, not adjusted**
    (carries out A11). Every position and range end of a text command must be on a boundary
    between extended grapheme clusters (Unicode Annex 29, crate `unicode-segmentation`), or
    the command fails with nothing changed. Refusing was chosen over moving the position
    because a command must do what it says or nothing; `text::is_boundary`, `floor_boundary`
    and `ceil_boundary` let an interface find the allowed places first. Limits of the
    guarantee: runs and links that came from a file may already cut a character (TreeSheets
    does not care, and files are not rejected for it); and inserting or deleting can make two
    neighbouring characters join into one (a lone combining mark typed after a letter, two
    half flags left next to each other), after which an old run or link end may lie inside
    the new character. No command splits what was one character.
34. **The command engine** (carries out A9). A command is a plain value (`edit::Command`)
    naming cells by ID. It is carried out as steps of ten kinds: splice text, set one field of
    a cell or its grid, set a tag, add cells, remove cells, insert lines (rows or columns),
    remove lines, move lines, replace a cell's whole grid, and make two cells trade places.
    Applying a step returns the step that undoes it exactly; the history keeps those steps
    and never a copy of the document. A step holds only what it needs: a step that removes
    cells necessarily holds the removed cells (that is the only place they still exist), a
    text step holds the text it took out, and so on; nothing proportional to the rest of the
    document. One command is one unit of undo; `begin_group`/`end_group` make several
    commands one unit; a new command empties the redo history; `set_history_limit` bounds
    the number of units (older ones are forgotten, their effects stay). A command that fails
    part-way has its applied steps undone before the error is returned, so a refusal
    (`Error::Refused`, `NotFound`, `Limit`) never leaves a trace. A command that asks for
    nothing new succeeds with an empty report and no history entry. Milestone 3b adds
    commands, not machinery: "replace a cell's whole grid" with add and remove expresses any
    rearrangement of cells.
35. **The change report** (carries out A10). Every command, undo and redo returns `Changes`:
    `changed` (cells whose own content or properties differ, including their grid's border,
    spacing and column widths), `created`, `removed`, `grids` (cells whose grid appeared,
    disappeared, changed size or lists other cells) and `tags`. A cell that only moved is not
    listed itself: the cells whose grids it left and entered are in `grids`. Inserting,
    deleting or reordering columns puts the grid's cell in `changed` as well, because its
    width list differs. For a single command the report is exact, tested both ways (nothing
    unreported differs; nothing reported is the same). For a group, it is complete but may
    name a cell that was changed and changed back.
36. **New cells take their IDs from the `IdSource` passed to `Editor::apply`**, and an ID the
    document already has, or one the source repeats, is refused. Undo keeps the removed cells
    inside the undoing step, so redo puts back the very same cells with the same IDs, without
    asking for new ones.
37. **Last-edit times come from the caller.** `Editor::apply` takes `now` (milliseconds since
    1970 UTC) or `None` for "do not stamp"; the core never reads a clock. Following TreeSheets
    (`Text::WasEdited`), these stamp the cell: inserting, deleting, replacing and setting
    text, setting the cell colour or text colour, and being created (by a new grid or an
    inserted row or column). Styling, linking, clearing, moving and the other properties do
    not. A copied cell keeps the time of the cell it was copied from. `Prop::LastEdit` sets a
    time directly.
38. **Folding is outside the document and its history** (carries out A8).
    `ViewState::set_folded` folds and unfolds; it is not a command and cannot be undone.
    After every command, undo and redo the interface calls `ViewState::apply_changes` with
    the report, which drops folds, selection and zoom that name removed cells, and the fold
    of a cell that lost its grid, at a cost that depends on the size of the change only. An
    undo that brings a cell or grid back does not bring its fold back: it returns unfolded.
    Keeping a memory of dropped view state was judged not worth the extra state.
39. **Moving is trading places; copying is the only thing that makes new IDs.** `MoveCell`
    takes a cell, with its ID and everything in it, to where an empty cell is, and the empty
    cell takes its old place; `SwapCells` is the same without the "empty" condition. So a
    link to a moved cell still points at it (B1). Neither is allowed between a cell and
    something inside it. `CopyCell` makes the target an exact copy of the source: the target
    keeps its own ID and place, the cells of the copied grid get new IDs, and links from
    inside the copy to cells inside the source follow to the copies (the rule of A28). The
    copy is taken before the target changes, so copying a cell into its own inside, or over
    a cell it is inside of, is well defined. Copying between documents is not built.
40. **The restructuring commands are TreeSheets' menu actions, one command each**
    (milestone 3b): `SortRows`, `Transpose`, `HierarchySwap`, `Hierarchify`, `Flatten`,
    `Wrap`, `Collapse`, `SplitCell`, `TagCells`, `UntagCells`, `AdjustRelSize`,
    `AdjustColumnWidths`, `ToggleCheck`. Each takes as parameters what TreeSheets takes from
    the selection (the cell that has the grid and a block, a list of cells, one cell, a
    text position) and does to the document what the action does, found by reading the C++
    and compared with the running app (`referee/ops/ui/`). An action that is the same as
    one or a few commands that already exist got no command of its own; the table in
    `milestones/M3b.md` places every one of TreeSheets' 265 actions. The three commands
    that rearrange a whole subtree (hierarchify, flatten, hierarchy swap) work on a private
    copy of that subtree, in the way TreeSheets works on its tree, and then turn the
    difference between the copy and the document into ordinary steps; so their cost is that
    of the subtree, never of the document, and undo, redo and the change report need
    nothing new.
41. **Who keeps the ID when cells are restructured.** A cell that survives keeps its ID
    wherever it goes, so that links to it survive (B1). Where TreeSheets copies cells and
    destroys the originals, the copy that stands for the original is the original:
    - *Sort, transpose:* all cells keep their IDs.
    - *Hierarchify:* all cells keep their IDs; the first cells of rows that repeat an
      earlier first cell are removed (the earlier one stays). No cell is created.
    - *Flatten:* every cell that was inside keeps its ID: a cell without a grid on its
      row, a cell that had a grid on the first of the rows it now stands on. Its repeats
      on the following rows, and the empty cells that fill short rows, are new.
    - *Hierarchy swap:* the swapped cells and the cells they held keep their IDs. When
      cells with the same text merge, the one that was there first keeps its ID and the
      other is removed. The copy of a parent that goes inside the swapped cell is a new
      cell while the parent stays where it was (it still holds other cells); if the swap
      leaves the parent empty, so that it disappears, the copy takes its ID: the parent
      has moved.
    - *Wrap:* the wrapped cells keep their IDs inside the new cell. The new cell is new,
      and so are the empty cells left where wrapped cells were.
    - *Collapse:* the first cell keeps its ID and gets the texts; the others are emptied
      in place (or deleted with their rows or columns).
    - *Split:* the cell keeps its ID and place and becomes the holder; the two parts are
      new cells. The cells of a grid it had keep their IDs.
42. **An eleventh kind of step: giving cells back to their parents** (corrects A34; found
    in review of milestone 3a). The steps "replace a cell's grid" and "insert lines" make
    the cells they list children of the grid's cell. Undoing them did not give those cells
    back to the parents they had, which nothing in 3a noticed because its commands only
    listed cells that were new or already children. A command that failed half-way, or the
    undo of one that moved cells between grids, could leave a cell naming a parent that
    does not list it. Now the undoing step carries the previous parents of the cells that
    were taken (`Step::Reparented`, which wraps the plain undoing step) and puts them back.
    Tested: every kind of step followed by its inverse gives the identical document, and
    the inverse of the inverse the changed one, starting from cells that are children of
    other cells.
43. **What commands build can be saved** (replaces the 5,000,000 cells of A20, completes
    A29; found in review of milestone 3). Commands used to check only the number of cells
    and the length of one text, while saving refuses a file over 32 MiB: a grid of 1,000 by
    1,000 cells was made and could then not be saved. Now:
    - *The limit on cells is 700,000*: what a file of 32 MiB can hold (a cell takes at
      least 48 bytes) and no less, so it refuses no file. The importer of `.cts` files is
      bound by it too; a document with more could never be saved.
    - *The editor keeps an upper bound on the size of the saved file* (`edit/size.rs`,
      `Editor::size_bound`): a sum of one number per cell and tag, each counted as written
      or longer. Every step changes the sum by the parts it touches, so no command looks at
      the whole document for it. To be independent of where a cell is, the bound counts
      every cell as indented as deeply as allowed: an empty cell counts 81 bytes where it
      takes 48 or so.
    - *A command that would take the bound over 32 MiB is refused* (`Error::Limit`, "too
      large to save"), with nothing changed, whichever command it is; this is checked in
      one place, when the command is complete. Before that, a command that asks for more
      new cells than a file can hold is refused before it makes them, so the most memory a
      refused command takes is that of about 414,000 cells (150 MB), not gigabytes.
    - *Undo and redo are never refused*: they lead to documents that were accepted.
    - *A document whose bound is over the limit although its file fits* (loaded from a file
      within about 33 bytes per cell of the limit) can be edited: a command that raises
      the bound is refused; any other is carried out and the document is then written out
      once to see that it still fits, which costs time in proportion to the document for
      every command until the bound is under the limit again. A document that did not fit
      when the editor took it (built in memory) can be made smaller freely.
    - So: any document reached by commands from one that could be saved can be saved and
      loaded again. Tested by a rule on random commands with the limit set just above the
      document (`Editor::set_size_limit`, which exists for that and for interfaces with
      little memory), and by one test at the real limit.
    The cost of the looseness: commands stop at about 414,000 empty cells where a file
    could hold 699,000. Counting indentation exactly would remove it and was not done,
    because steps that move cells between grids would then have to walk what they move.
44. **A command that leaves the document as it was is nothing** (makes A34 and A35 true
    where they were not; found in review). No last-edit stamp, no unit in the history, an
    empty report. `SetProps` now works out what the properties of a cell are after all of
    its `props` and makes steps only for what differs, so a colour set to what it is, or
    set and set back in one command, does nothing; the last-edit time is stamped only if a
    colour (or the colours of runs) really differs at the end, and a `Prop::LastEdit` after
    the last colour in the same command wins over the stamp. `TagCells` sets each tag
    once, with the colours of the last cell that has its text. The other commands already
    compared before they changed anything. Tested as a rule for every command: if the
    document is equal before and after, the report is empty and the history as it was.
45. **A group reports what it changed as a whole** (adds to A35). `Editor::end_group`
    returns the report for the outermost group: what differs between the document before
    its first command and after its last. A cell made and removed again inside the group
    is not named at all. A cell that was changed and changed back may still be named (the
    report is complete, not tight; making it tight would need a copy of every touched
    cell). Closing an inner group returns an empty report.
46. **The cost of the history** (changes A34, which had no bound; found in review).
    - *Bounds by default:* the undo history keeps at most 1,000 units (TreeSheets' number)
      and about 64 MiB (`Editor::set_history_limit`, `set_history_budget`,
      `history_bytes`); the oldest units go first. Memory is the bound that matters, since
      one unit can hold a deleted subtree of any size; the number of units keeps the list
      itself small. The memory is an estimate (the size of each step and of the cells,
      texts, runs and lists it holds; not what unknown fields hold). The newest unit is
      kept even if it alone is over the budget, so that the last command can always be
      undone. The redo history is not bounded separately: it only holds what was undone.
    - *Typing can join the unit before it:* `Editor::apply_typing` is `apply`, except that
      an `InsertText`, `DeleteText` or `ReplaceText` joins the newest unit if that unit is
      nothing but such edits of the same cell. The caller decides when typing continues
      (same cell, no pause, no cursor movement), since the core has no clock. Units made
      by `apply` can be joined too; groups cannot.
    - *A text edit holds only the runs and links it touches.* It used to put the cell's
      whole lists of runs and links into the history whenever any of them moved. Two more
      kinds of step (`PatchRuns`, `PatchLinks`; thirteen kinds now, with A42's) replace the
      ranges that the edit changes and move the ones after it by a number. Measured, three
      keystrokes in a cell with 2,000 runs: 288,784 bytes of history before, 928 after
      (96 KB and 0.3 KB per keystroke). Carrying out the edit still reads all the runs of
      the cell. Styling a range and the other commands still hold whole lists.
    - Not changed: undo is still refused while a group is open.
47. **A link to a cell of its own document does not name the document** (settles what A28
    left open, for milestone 4). `LinkTarget::Cell` has `doc: None` for such a link, never
    the document's own ID: `SetLink` drops an own ID, the reader drops one it finds in a
    file (and so writes the file back without it), the importer never made one, and
    `validate` refuses a document that has one, so no editor ever holds one. This form
    was chosen over always naming the document because a copy with fresh IDs then needs
    no rewriting of the document part, files are shorter, and the links to a cell can be
    found by cell ID alone. A link with another document's ID is left as it is.
48. **An ID in the history is taken** (adds to A36; found in review). A cell that was
    removed but that undo or redo can bring back still owns its ID: the editor keeps the
    set of IDs of the cells that its history holds (updated as units come and go, at the
    cost of the cells in them), and an ID from the `IdSource` that is in the set is
    refused like one that is in the document. When the history forgets a unit, the IDs of
    the cells in it are free again.

Number 49 is not used.

50. **Resolving a link** (milestone 4; numbers A50-A59 and B40-B49 are this milestone's,
    A49 is not used). A link to a cell is looked for where it says: in its own document,
    or in the document it names, found by document ID wherever the file is. If the cell
    is not there, it is looked for by its ID in all documents at hand, because cell IDs
    are unique across documents (A5) and a cell can move to another document with its ID
    (A51): found in exactly one, the link resolves there and is marked *moved*, so that
    it can be corrected (`LinkIndex::repairs`); found in none, the link is *broken*
    (no such document, or no such cell). **Nothing is guessed:** if several files have
    the document ID that the link leads to (hand copies, A28), a link into it from
    another document is *ambiguous*, as is a cell that is not where the link says and is
    in several documents. A document's links to its own cells always resolve within it,
    also when another file has its ID. Of files with one document ID only the first by
    name is indexed. Backlinks are the links that resolve to the cell, sorted by the ID
    of the document that has the link, the ID of the cell, and the position: a fixed
    order, not a meaningful one (an interface sorts as it likes).
51. **Cells that go from one document to another** (`edit/paste.rs`, `links/transfer.rs`).
    A `Clip` is a cell with everything in it, taken out of a document as a value.
    *Copy* is `PasteCells`: as `CopyCell` (A39), the target keeps its ID and the cells
    inside get new ones. *Move* keeps every ID, so that links to the moved cells survive
    (B1): `AdoptCells` puts them, as they are, in the place of an empty cell of the
    other document, and `ReleaseCell` takes them out of theirs, leaving a new empty
    cell. Links in what arrives are rewritten to point at the same cells as before: to
    cells that came along unchanged (or to their copies), to the other cells of the
    document they came from by naming it, to cells of the document they arrive in
    without a document (A47). On a move, `links::move_between` also rewrites the links
    to the moved cells in the two documents (in the source they now name the target; in
    the target they no longer name a document). Links in other documents are not
    touched: they resolve as *moved* (A50) until they are corrected. Rewriting a link
    this way drops its unknown fields.
52. **A move between documents is two units of undo, one in each history.** An editor
    edits one document (A34) and there is no history above the documents, so the move
    cannot be one unit; `move_between` makes both changes or neither (if the second half
    fails, the first is undone and forgotten). Undoing both gives both documents back
    exactly. Undoing one only leaves the cells in neither document (links to them are
    broken until the other is undone) or in both (each document's links find its own;
    other documents' links stay with the document they name; the workspace reports the
    shared IDs): nothing is lost either way. The IDs of moved cells stay taken in the
    source's history (A48), and the target refuses to adopt an ID that it has or that
    its history holds. An interface should offer the two undos together.
53. **An index is one summary per document** (`summary.rs`; carries out A13 for links,
    and is the shape milestone 6 is to use). A `Summary` is made by reading a document
    once, follows edits from the change report (A35) at a cost that depends on the
    cells changed and not on the document, and can be written to a cache and read back.
    The link index (`links::LinkIndex`) is the summaries (`DocLinks`: the IDs of the
    document's cells, and the links of the cells that have any, with the linked text)
    and a table from target cell ID to the cells that link to it (which A47 makes
    possible: the links to a cell are found by its ID alone). It holds nothing that is
    not in the documents, and an index built from the documents equals one that
    followed any sequence of commands, undos, redos and groups: the main test of the
    milestone. It does not hold the order of cells or their place, which would not be
    O(change) to maintain; the text and location trail of a target are read from its
    document when they are shown.
54. **The index of a workspace reads each document once, and its cache is believed only
    when it proves itself** (`scan.rs`). Documents that are not open are indexed by
    loading them one at a time (memory: one document and the summaries). The optional
    cache is `.wombat-index/<kind>.idx` in the workspace folder: a first line with the
    kind, its version, and the length and a checksum of the rest; then the summaries,
    each under the length and a checksum of the bytes of the document file it was made
    from. Every document file is read in full on every scan and its checksum compared,
    so staleness does not depend on modification times: a changed file is parsed again,
    an unchanged one is not, a renamed one is found, a deleted one drops out. A cache
    whose first line is not exactly what this version would write for the rest is
    ignored whole, and an entry that does not decode is parsed again. The checksum is
    128-bit FNV-1a: proof against damage, not against forgery. Failing to write the
    cache is not an error. The command-line tool keeps no cache unless asked
    (`--cache`), so that reading a folder does not write to it.
55. **Deleting what is linked to.** Nothing is rewritten: the link stays in the text of
    the cell that has it and resolves as broken; undoing the deletion brings the cell
    back with its ID (A36) and the link resolves again. `LinkIndex::would_break` and
    `would_break_document` say beforehand which links a deletion would break, for a
    warning. A deleted (or absent) document makes links into it broken ("no such
    document"), also without rewriting.
56. **Imported links without a target are resolved later only when there is no doubt**
    (settles B14's "milestone 4 may"). `links::unknown_links` lists them with the cells
    that now have the text they named; `links::resolve_unknown` gives a target to those
    with exactly one such cell (never the root or the linking cell; a formula cell
    counts by what it shows, A66) and leaves the others. TreeSheets' "nearest cell" rule is applied once, at
    import (B1), and not again: on a document that has since been edited, "nearest"
    would be a guess. Any `SetLink` on the range converts the link by hand.

Numbers 57 to 59 are not used: milestones 4 and 5 were built at the same time, each with
numbers of its own (A50 on and B40 on for milestone 4, A60 on and B50 on for milestone 5).

60. **Calculating gives the same on every machine** (milestone 5; carries out A14 under
    the rule of deterministic output). TreeSheets takes the reading and writing of
    numbers, letter classes and upper and lower case from the C and C++ libraries of the
    machine it runs on. Wombat writes them out (`eval/num.rs`), as the real app behaves on
    Linux in a UTF-8 locale, which is what the referee records:
    - *A cell is a number* if its whole text, spaces around it aside, is one: `12`,
      `-1.5`, `.5`, `5.`, `1e3`, one `+` in front, hexadecimal after `0x` (`0x10` is 16).
      Not `1,5`, not `12abc`, not too large for a double, not so small that it would be
      zero. A number that starts with a zero is read without that zero, so `0e5` and `0.`
      are no numbers and `0-5` is minus five: oddities of the library TreeSheets is built
      with, kept because the referee shows them.
    - *The operations of a program* read the number a text starts with (`12abc` is 12,
      `abc` is 0, an empty text is not a number), as a C++ stream does.
    - *A result is written* with at most ten decimals, no exponent and no zeroes at the
      end; "not a number" as `nan` or `-nan` with the sign an x86 processor gives.
    - *Letters, digits and spaces* are Unicode's (a name may be `Größe` or `日本`),
      upper and lower case map one character to one (`ß` stays `ß`), text lengths and
      positions count code points.
    Never from the locale: the decimal point is always `.`. B56 lists where a TreeSheets
    on another system differs.
61. **A formula cell's text is its formula; what it shows is worked out, never saved**
    (settles what milestones 2 and 3 left open). TreeSheets keeps the result in the
    cell's text and the formula beside it; its file has the formula only. In Wombat a
    cell of kind `formula` has the formula as its text (with `=` assumed in front if it
    is not there, as TreeSheets adds it when a cell is marked as a formula), and
    `eval::Calc` holds what each formula cell shows. The editor owns a `Calc` and brings
    it up to date after every command, undo and redo; `Editor::shown(cell)` is "the
    displayed text of a cell" (the result for a formula cell, the text for any other), and
    `Editor::recalculated()` lists the formula cells whose result the last change
    altered, for an interface or a search index. The file format does not change. After
    loading, results are worked out from the document alone.
62. **Formulas are worked out in TreeSheets' order, without its recursion.** TreeSheets
    goes through the formula cells in document order and works each one out after the
    formulas whose results it reads, by calling itself. The order matters where formulas
    refer to each other in a circle (which of them says `#CYCLE!`, and which error a
    formula with two bad inputs shows), so Wombat keeps it exactly; but it keeps its own
    list of the formulas in progress instead of calling itself, so that a chain of any
    length costs no stack. An attempt that meets a result it does not have yet stops; the
    missing formulas are worked out (as many of them as TreeSheets would reach before the
    first that fails); the attempt is made again, remembering what it had already read.
63. **What is worked out again after a change** (carries out A10 for results).
    `Calc::update` gets the change report of every command, undo and redo.
    - If only cells without a grid changed, and none of them has to do with assigning
      variables: the formulas that read a changed cell, and those that read the results
      of these, and so on, are worked out again; nothing else. `Calc` remembers per
      formula which grids it read the cells of and which cells it read everything
      inside of (not each cell: a sum over a thousand cells is one entry).
    - Everything is worked out again if cells were created, removed or moved, if a cell
      with a grid changed its text or kind, if a variable-assign cell or a cell in its
      grid changed, after a run that assigned other variables, and whenever a circle of
      formulas or the limit on work is involved. The reason: a name in a formula means
      "the nearest cell with this text that has a grid", so any of these can change what
      a name means, for formulas anywhere; and with a circle the order decides.
    - A cell with a grid whose text and kind are as they were (a new colour, say) changes
      nothing.
    Correctness was put first: the rule "keeping up to date equals working everything
    out" is tested on random commands. Measured (release build, 10,001 formulas in 22,005
    cells): everything 57 ms; after one changed number 4 ms, nearly all of it one
    formula that sums the whole document; after an inserted row 45 ms. A document
    without formula cells pays nothing.
64. **Running the program is a command** (`Command::Run`, TreeSheets' Program > Run).
    The run is worked out on a private copy of the document, in TreeSheets' order
    (`eval/run.rs`); then every view cell that got other content is overwritten with it
    by ordinary steps, so the run is one unit of undo with an exact change report. A
    view cell keeps its ID and place and takes text, kind and looks of the value, as in
    TreeSheets; the cells of a grid it gets are new, those it had are removed. A run
    that leaves every view as it was is nothing (A44). The variables that the run
    assigned and its first failure are kept in the editor's `Calc` (`variables`,
    `last_run`): formulas can read variables by name. Like TreeSheets, the variables are
    not part of the document: they are not saved, a loaded document has none (a formula
    that reads one shows `#N/A` until the program runs), and undoing a run leaves them.
65. **How a cell becomes a formula cell.** `Command::EnterText` is "typing into the cell
    has ended" (TreeSheets' `EndFormulaEdit`, also its formula bar and its paste of one
    line): it sets the text, and a data or formula cell becomes a formula cell if the
    text starts with `=` and has more than that, a data cell if not; cells of other kinds
    keep their kind. `SetProps` with a kind is TreeSheets' Program > Mark as. The text
    commands of milestone 3 (`SetText`, `InsertText`, `DeleteText`, `ReplaceText`) leave
    the kind alone: on a formula cell they edit the formula. An interface that lets
    someone type sends `EnterText` at the end, or the text commands while typing and
    `EnterText` to finish.
66. **Commands go by what a cell shows.** In TreeSheets the text of a formula cell is its
    result, so everything that compares texts compares results. Done: `SortRows` sorts
    by what cells show; a run reads formula cells by their results; the importer of
    `.cts` files finds the target of a link by what cells show, so a formula cell can be
    a target (replaces B15), `links::unknown_links` counts them the same way, and
    `links::place_shown` shows a link's target and its trail by what cells show. Not done, and still by the formula: hierarchify (which
    compares texts), flatten, collapse and hierarchy swap (which copy texts into new
    data cells).
67. **Limits on calculating** (A12; B52 and B53 say how TreeSheets differs). A formula
    longer than 4,096 code points, or with parentheses and function calls nested deeper
    than 64, shows `#ERROR!`. The formulas of one document may read 50,000,000 cells in
    one recalculation; from the formula that goes over on, formulas show `#LIMIT!`, and
    every later change works everything out again. A run may visit and copy 20,000,000
    cells and nest values 64 deep, and may not leave grids nested deeper than 32 or more
    cells than a document holds; otherwise the command is refused with nothing changed.
    Nothing in the evaluator calls itself deeper than these bounds.
68. **References stay by name** (A14). A formula names a grid by the text of its cell
    and finds it again at every recalculation, so renaming a table breaks the formulas
    that name it (`#NAME?`), as in TreeSheets. A cheap way to make references survive
    renaming is described in `milestones/M5.md` as a proposal; it is not built.

70. **The search engine is a small index of Wombat's own, not Tantivy** (milestone 6;
    numbers A70-A79 and B60-B69 are this milestone's, A69 is not used; settles the
    open question of the plan). Tantivy 0.24 was tried with its default features off:
    - *It does not build for the web as it is.* `cargo check --target
      wasm32-unknown-unknown` fails in `getrandom` and then in `uuid`: both need their
      `js` feature, which ties the build to a browser's JavaScript, and its writer wants
      threads and its index a directory.
    - *It does not build for phones without a C toolchain.* `aarch64-linux-android` and
      `aarch64-apple-ios` fail in `zstd-sys` (C code, pulled in by its sstable crate even
      with default features off). With the NDK and Xcode it presumably builds; the core
      would then no longer be Rust alone.
    - *It is large.* 119 crates (127 for the web) against 2 here (`unicode-normalization`
      and its `tinyvec`); 19 s to check on four cores; a program that indexes one
      sentence is 4.2 MB stripped, against 2.3 MB for the whole `wombat` tool with search.
    - *It does not fit the task.* It is made for documents of words in a language with
      spaces, committed in segments. What is needed here is a match on any part of a
      word while it is typed, in Chinese, Japanese and Korean too (which needs n-grams
      whatever the library), and an index that follows one keystroke in a cell in
      microseconds without a commit. That is most of the work either way.
    So the fallback of the plan is the engine: `crates/wombat-search`, which builds for
    `wasm32-unknown-unknown`, `aarch64-linux-android` and `aarch64-apple-ios`, with file
    access only through `wombat_core::scan`.
71. **The cache of an index is one entry per document, believed only when it proves
    itself, and never reached through a link** (replaces the layout and the rule of
    A54; found in review of milestone 4).
    - *Layout:* `.wombat-index/<kind>/<document file name>.idx`, so a changed document
      costs the rewriting of its own entry, and nothing is loaded that is not needed.
      An entry is a first line (kind, version of the layout, a number for the rules the
      index was made by, the document file's length, time and checksum, the length and
      checksum of the rest) and the summary as compact bytes (`summary::Writer`), not
      JSON. An entry is good exactly if its first line is the one this version would
      write for its body and its document file; anything else counts as missing.
    - *Memory:* an entry over twice the largest document file (64 MiB) is not read, and
      reading one takes memory in proportion to its size: a count read from it is
      refused if the bytes left could not hold that many items. (The JSON cache of
      milestone 4 took over ten times its size as a tree, and was one file for all.)
    - *Two ways to tell that a document is unchanged* (`scan::CacheMode`). `Verify`
      reads every document file and compares a checksum of its bytes: nothing escapes
      it. `Fast` goes by the file's length and modification time and does not read it,
      which is what makes opening a large workspace quick. **What `Fast` gives up:** a
      file whose content changed while its length and its exact time stayed the same
      (a tool that restores times, or a deliberate act) is not noticed until something
      else changes; `Verify` notices and repairs. To leave no smaller hole, a file is
      believed by length and time only if it was last modified at least two seconds
      before its entry was written (the coarsest time stamps of common file systems);
      otherwise it is read and compared, and the entry written again, so that a change
      within the same tick of the clock cannot hide. A file that changes while it is
      being read gets no time at all. The command-line tool's `--cache` is `Fast`,
      `--cache=verify` is `Verify`. A renamed file is parsed again (entries go by
      name); entries of files that are gone are removed.
    - *Links:* the cache folders and files must be real folders and files
      (`symlink_metadata`), or the cache is neither read nor written. An entry is
      written to a new temporary file and renamed into place, which replaces a link
      instead of following it. Before this, a `.wombat-index/links.idx` that was a
      symbolic link made `--cache` overwrite the file it pointed at.
    - *The rules:* `Summary::fingerprint` is in every first line: for search, a number
      computed by folding fixed samples, so that an index made under other rules of
      folding cannot be believed even if nobody remembered to raise a version. (The
      search summaries do not hold folded text at all, only the texts as they are; the
      folded forms and the table are made when an entry is read.)
    - The checksum is now a fast one of 128 bits over eight bytes at a time
      (`summary::digest`), still no proof against forgery (A54). Entries are not synced
      to disk: one cut short by a crash fails its checksum and is made again.
72. **Text is compared in a folded form** (`wombat-search/src/fold.rs`). Case is ignored
    by Unicode's full case mappings applied lower, upper, lower (`ß` = `SS` = `ss`, `Σ`
    = `σ` = `ς`). The four Turkish i's (`I`, `i`, `İ`, `ı`) are one letter: right for
    everyone but a Turkish reader who wants `ı` and `i` apart, and `case:` is there for
    that. Compatibility forms are ignored (NFKD: full-width `Ａ` is `a`, half-width
    `ｶﾞ` is `ガ`, `ﬁ` is `fi`). Accents are ignored only for Latin and Greek letters and
    only where Unicode holds them as separate marks (`é` = `e`; `ø` and `ł` stay
    letters of their own); other scripts keep their marks in composed form (NFC), so
    Japanese `が` is not `か` and Russian `й` is not `и` (and `ё` is not `е`, which a
    Russian reader may miss). Every kind of space and control character is one space;
    soft hyphens, zero-width spaces and variation selectors vanish. Folding is done
    one user-perceived character at a time, so a match in folded text is always whole
    characters of the original (A11): a match of `s` in `ß` highlights the `ß`.
73. **The index** (`index.rs`, `engine.rs`; carries out A13 and A53 for search). Per
    document: every cell's ID, the cell it is in and its place there, its texts as they
    are and folded, and the properties filters ask about; and a table from every pair
    of neighbouring folded characters (and every single character of a script written
    without spaces) to the cells that have it. **The table only proposes cells; whether
    a cell matches is always decided by looking at its text.** So the table may name
    too many cells and never too few, and nothing about it can make a result wrong:
    - an edit only adds to it (what it makes stale is counted, and the table is built
      again when half of it is stale: work that the edits before it pay for);
    - a key that more than an eighth of a document's cells have gets no list at all,
      and a search made of such keys only looks at every cell, which it nearly did
      anyway; this halves the table;
    - a word that must match exactly (`case:`) is looked for in every cell.
    The index holds where cells are (parent and place), unlike the link index (A53),
    because results of documents that are not open need their location trails and
    "within a branch" needs ancestors. Following a change report therefore costs in
    proportion to the cells it names **plus the cells directly inside a grid that
    gained, lost or reordered cells** (their places are read again); measured 2.5 µs
    per edit in a workspace of 100,000 cells. **Trails, depths and the order of the
    document are not stored per cell:** a trail is read off the parents when a result
    is made (so renaming or moving a cell changes the trails of everything inside it
    at no cost), and the order of a document is worked out when a search first needs
    it after cells moved (about a millisecond for 5,000 cells) and kept until they
    move again.
74. **What matches, and in what order** (`engine.rs` has the rule in full). A word
    matches a cell if it is found anywhere in the folded text, note, image file name or
    link target of the cell, or in the text of a cell that one of its links leads to.
    Anywhere, not only at the start of a word: that is what TreeSheets does, it is what
    text without spaces needs, and it makes every prefix a match while typing; how
    good the match is goes into the order instead. Points per word: 4 x field (text 0,
    displayed result 1, note 2, linked cell's text 3, image name 4, link target 5) +
    kind (whole field 0, whole word 1, start of a word 2, inside a word 3); a word
    that is absent (possible under OR) 32; added up; then shorter text, smaller
    depth, document name, document order. Recency only when asked (`sort:`). No
    statistics of the collection (how rare a word is) take part: the order of results
    for a query depends on the matching cells alone, so it does not shift when other
    documents change, and it can be explained to a person.
75. **A query is a value; the text syntax is one way to make it** (`query.rs`). Words,
    filters, and, or, not. Limits, so that no query can take unbounded time or memory:
    4,096 bytes of text, 64 words and filters, nesting 16, 1,024 bytes a word. Finding
    is plain substring search; there is no pattern language and so nothing that can
    blow up. The parser never fails on punctuation (a quote or bracket left open is
    closed at the end, as when typing); it fails when a known filter has a value that
    means nothing, saying what would. A word with a colon that is not a filter
    (`https://...`, `12:30`) is a word.
76. **What of a cell is searched comes through one function**
    (`wombat_search::index::searchable`), used by the index and by the reference
    scanner alike. It returns the text, the note, the image name, the link targets,
    and `shown`: what the cell displays where that is not its stored text, the result
    of a formula cell. The field is built all the way through (indexed, ranked just
    after the text, written to the cache, highlighted). Where the results come from
    and how the field stays right under editing is A90.
77. **Replace is `ReplaceText` commands in one group** (`replace.rs`). Every place where
    the word is found in a cell's text is replaced, left to right, no place overlapping
    another; notes are not touched (as in TreeSheets). One call is one unit of undo for
    the document. If any command is refused (a replacement that would join up with
    the text beside it into one user-perceived character, A33), all are put back.
78. **No tolerance for typing mistakes, no search by meaning, nothing inside images.**
    Typo tolerance was tried on paper only: with matching anywhere in a word it would
    make most short queries match most cells, and doing it well (by edit distance on
    whole words, ranked below exact matches) needs a list of the words of the
    workspace that the index does not otherwise need. It stays on the future list.
79. **The speed target is asserted.** `tests/large.rs` builds a workspace of 100,000
    cells in a release build (20,000 in a debug build) and fails if the median query
    takes over 10 ms or the worst over 50 ms (ten times that in a debug build), if
    following one edit takes over 0.5 ms, or if the index takes over 16 times the
    text. The measured numbers are in `milestones/M6.md`.

Numbers 80 to 89 are not used: 90 on are where milestones 4, 5 and 6, built apart, meet.

90. **An index gets formula results from the editor, and works them out itself for a
    document that nobody edits** (A61, A76; `Summary` in `summary.rs`). Results are
    beside the document, never in it, so an index that keeps what cells show cannot be
    made from a document alone.
    - *An open document.* `SearchIndex::insert_document(doc, calc, name)` and
      `SearchIndex::apply(doc, calc, changes)` take the editor's `Calc`
      (`Editor::calc()`) with the document: it has the results, and
      `Calc::recalculated()` says which formula cells show something else since the
      last command, undo, redo or group. The index reads those cells again like the
      cells that the change report names. The change report itself was not widened: it
      says what changed in the document (A35), and a recalculated formula cell did
      not change, so a report that named it would make every other reader of reports
      (undo sizes, the link index, a view) do work for nothing. The cost of following
      an edit is the cells of the report plus the recalculated cells.
    - *A document that is not open.* `Summary::of(doc)`, which the scan of a workspace
      calls, works the results out as after loading (`Calc::new`, no variables of a
      run) and the cache keeps them, so a cached scan calculates nothing. `Summary::update`
      takes the `Calc`; the link index ignores it, since it keeps targets and no texts.
    - *Cost* (release build, measured): for documents without formula cells 0.28 ms
      per 1,000 cells on top of 1.6 ms of indexing and about 2.3 ms of parsing (under a
      tenth more); for a document of 22,000 cells with 10,000 formulas 44 ms on top of
      53 ms of parsing and 17 ms of indexing.
    - *Hostile documents* are bounded by the evaluator's own limit on work (A67), which
      holds for a scan as for an editor, so that a document is indexed the same open
      or not: a document made to reach the limit costs a scan what it costs an editor
      to open (4.9 s at the limit of 50 million cells; the limit is the place to
      change that).
    - *Variables of a run* are in the editor's results and in no file, so a formula
      that reads one is indexed by its number while the document is open and by `#N/A`
      after it was saved and scanned, as it shows in each case.
91. **Replace works on what is stored: of a formula cell the formula, never the result**
    (A77, A65). A result is not text that anyone typed; replacing in it would mean
    turning the formula into a constant. `replace` looks at `cell.text` only, so a
    search for a computed value finds the cell and a replace of the same word changes
    nothing in it. A replace that changes a formula is `ReplaceText` like any other:
    the cell stays a formula cell (text commands never change the kind, A65), is in
    the change report, and its result and those of its readers are worked out again
    before the command returns. Replacing a table's name in the whole document renames
    it in the formulas that name it too, since both are text.
92. **Where search shows or follows a cell, it goes by what the cell shows** (A66,
    as `links::place_shown` does for links): the location trail of a result, and the
    field "text of a linked cell", are the results of formula cells, not their
    formulas. A hit carries both of its own cell: `text` (the formula) and `shown` (the
    result), and the tool prints them as `wombat show` does, `"=SUM(Costs)" = "1050.5"`.
    Filters that ask about a cell's text (`text:`, `tag:`, `has:text`) and the "shorter
    text first" of the ranking still go by the stored text.

## B. Deliberate departures from TreeSheets

1. **Links point at cell IDs**, not at "the nearest cell with this text". Renaming or moving a cell
   cannot break or redirect a link. When importing a `.cts` file, each text link is resolved once
   by TreeSheets' rule and stored as an ID.
2. **Backlinks exist.** Any cell can list the cells that link to it.
3. **Search covers a whole workspace**, shows where each result sits, and treats tags and filters
   as parts of a search, where TreeSheets has find-and-replace inside one document.
4. **View state is outside the document** (decision A8). TreeSheets saves the selection, zoom and
   fold state in the file.
5. **Undo is a list of steps** (decision A9). TreeSheets keeps copies of the changed part.
6. **No `.cts` writing.** The importer is read-only and exists for testing.
7. **Lookup-style export** with ID columns placed for XLOOKUP is new.
8. **Colours are red-green-blue** written `#rrggbb`. TreeSheets stores blue-green-red numbers;
   the importer converts.
9. **An image's display scale belongs to the cell that shows it**, not to the image, and images
   are files beside the document instead of being embedded.
10. **"Text and grid vertical" is a property of the grid**, since it means nothing without one.
    TreeSheets keeps it on the cell.
11. **Last-edit times are milliseconds since 1970 (UTC)** and may be absent.
12. **The model accepts any Unicode text in a cell**, including line breaks and control
    characters. TreeSheets turns control characters into spaces as text is entered; whether
    Wombat's editing commands do the same is for milestone 3.
13. **A selection is its top-left cell's ID plus a size, and zoom is the ID of the cell zoomed
    into.** TreeSheets saves a zoom depth and marks the selected cell in the file.
14. **A link to a missing cell is kept by its text.** A `.cts` link names its target by text
    and TreeSheets looks for it each time the link is followed. Where the importer finds no
    cell with that text, the link keeps its range, has no target (`LinkTarget::Unknown`), and
    carries the text in the field `cts_target`, which the file format keeps as an unknown
    field. Nothing re-resolves it later; milestone 4 may.
15. *(Replaced by A66 in milestone 5: a formula cell is a target by what it shows.)*
    **At import, a formula cell is never the target of a link.** In TreeSheets the text of a
    formula cell is its calculated result, so a link can go to a cell by what it shows. The
    importer has the formula, not the result (milestone 5), and would rather leave such a link
    without a target (B14) than point it at the wrong cell.
16. **The view state of an imported file is what TreeSheets shows after loading it**, not the
    raw header values: the selection is clamped to its grid and is at least one cell, a missing
    one is the first cell of the root's grid, and the zoom is the cell on the way to the
    selection at the saved depth (B13).
17. **An image's display scale is TreeSheets' number, unchanged**: larger means drawn smaller.
18. **B12 no longer holds:** cell text has no control characters, as in TreeSheets (A32).
19. **Text commands refuse bad positions.** A position outside the text, or one that cuts a
    character made of several code points, makes the command fail (A33). TreeSheets clamps
    ranges to the text and cuts wherever the cursor is.
20. **Colouring a cell never recolours a tag.** In TreeSheets, setting the colour of a cell
    whose text is a tag changes the tag's colours instead of the cell's, and "add tag" takes
    the colours of the selected cell. In Wombat `SetProps` always sets the cell, and tags
    have their own commands (`SetTag` with explicit colours, `RemoveTag`).
21. **Paste makes an exact copy** (A39). TreeSheets' paste keeps the target's text size and
    alignment (it takes the size of a neighbouring cell), widens the target's column to the
    source's, and when pasting a grid into a cell without text merges the grid's cells into
    the surrounding grid. `CopyCell` does none of the three: the target becomes what the
    source is. Seen in the real app with `referee/run_script`; the two paste cases that are
    compared are ones where the difference does not show.
22. **Deleting every row or column of the root's grid with `DeleteRows`/`DeleteColumns` is
    refused.** TreeSheets silently does nothing. (`DeleteCells` on everything in the root
    empties the cells and keeps the grid, as TreeSheets does; elsewhere both remove the
    grid.)
23. **Edit times are stamped only when the caller gives a time** (A37), and a new cell
    without one has no time. TreeSheets stamps every new cell with the current time.
24. **A new grid's "vertical" flag comes from the grid around it** (B10 moved the flag from
    the cell to the grid). In TreeSheets a new cell inherits its parent cell's flag.
25. **`SetRangeColor` with no colour** removes the colour from a range only. TreeSheets can
    only remove the colours of a whole text.
26. **Sorting names its column and rows.** TreeSheets sorts by the selection (one column
    wide and several rows high) and refuses other shapes with a message. `SortRows` is
    given the column and the rows, so there is nothing to refuse but numbers out of range;
    fewer than two rows is not an error and changes nothing. The order is TreeSheets'
    (text without regard to case, numbers as text, ties by the next columns, stable).
    Letter case is folded one code point at a time by Unicode's lower-case mapping where
    that is one code point; TreeSheets uses the C library's `towlower`, which gave the same
    in the cases compared but can differ for rare letters and between systems.
27. **New grids made by restructuring have the "vertical" flag of the grid around them**
    (B24 again): the grids that hierarchify, wrap, split and hierarchy swap make. In
    TreeSheets the flag is the cell's own. Flatten keeps the flag of the grid it replaces.
28. **Flatten gives new columns the default width of 80.** TreeSheets uses its "default
    column width" setting, which Wombat does not have.
29. **A square grid does not remember whether it was a row.** Where hierarchy swap adds a
    cell to a grid, TreeSheets adds a column to a row and a row to a column, and a grid
    that has become square (one cell) counts as what it was before, for as long as the app
    runs; the file does not keep that. Wombat remembers it within one command only (which
    is what the compared cases need); between commands a square grid counts as a column,
    as it does in TreeSheets after loading.
30. **Relative sizes stop at -32 and 8.** TreeSheets widens these limits by the depth the
    view is zoomed to; `AdjustRelSize` has no view. `SetProps` can still set any size.
31. **A checkbox that is part of a longer character is not cut off it.** `ToggleCheck` is
    refused for a text that starts with U+2611 followed by a variation selector (the emoji
    form), by A33; TreeSheets cuts. The mouse form of the action (a click on a ticked box
    unticks it) is not a command: it is `ReplaceText` of one character.
32. **Restructuring is refused where it would nest grids deeper than 32** (A20);
    TreeSheets has no such limit when editing.
33. **The root keeps its grid, and cannot be cleared or pasted over.** In TreeSheets the
    root is never a selected cell (the selection is always cells inside a grid, and
    `Grid::DelSelf` returns without doing anything for the root's grid), so no action
    there removes the root's grid, clears the root or pastes over it. Wombat's commands
    name any cell, so the rule is stated: `RemoveGrid` of the root, `ClearCells` with the
    root among the cells, `CopyCell` onto the root, and `DeleteRows` or `DeleteColumns` of
    all of the root's rows or columns (B22) are refused, with nothing done. `DeleteCells`
    on everything in the root empties the cells and keeps the grid (B22, as TreeSheets).
    The restructuring commands never leave the root without a grid. So once the root has
    a grid, no command takes it away (a new document's root has none until `CreateGrid`),
    which the random tests check for every command. Setting the root's text and
    properties stays possible; TreeSheets' interface cannot do that, but its script API
    can.

Numbers 34 to 39 are not used.

40. **Following a link gives its target; nothing is opened by the core.** TreeSheets'
    "open link in browser" and "open file" launch a browser or a program. The core
    says what a link points at (`LinkIndex::resolve`), which link the cursor is on
    (`links::link_at`, by TreeSheets' rule) and whether an external target is a URL or
    a file path (`links::external`, TreeSheets' `HasURLScheme`); opening is the
    interface's. TreeSheets opens the text of every selected cell when there is no
    link; in Wombat that is the interface passing the text to the same function.
41. **"Go to matching cell" is not a link.** TreeSheets' F6 and F7 jump to the next
    cell with the same text or image. With links by ID (B1) going to a link's target
    is resolution, and going back is the backlinks (B2); finding cells by equal text or
    image is a search and belongs to milestone 6.
42. **A deleted link target is reported, not searched for.** In TreeSheets a link whose
    cell is gone silently goes to the next nearest cell with that text, or nowhere. In
    Wombat it is a broken link, listed as such (A55).
43. **Cells can move between documents and keep their identity** (A51). TreeSheets
    moves cells between files only by cut and paste, which makes new cells.

Numbers 44 to 49 are not used.

50. **A formula cell that stops being one shows its formula again.** In TreeSheets the
    text of a formula cell is its result, and stays so when the cell is marked as data
    (or `ts.set_formula("")`): the formula is lost. In Wombat the text is the formula
    (A61), so the cell shows the formula again and nothing is lost.
51. **A formula cell with a grid is named by its result of now.** A name in a formula
    can stand for a formula cell that has a grid, by its result. TreeSheets compares with
    the result the cell showed last, which depends on what was calculated before (right
    after loading it is the formula itself). Wombat works that result out when a name is
    looked up, which is what TreeSheets settles on; a cell whose result is still being
    worked out at that moment (it is looking up a name itself) has no name. The
    referee's four cases agree.
52. **Limits on formulas** (A67): length, nesting and work. TreeSheets has none; it would
    run out of stack or time instead.
53. **Limits on a run, and a run that TreeSheets cannot survive** (A67). TreeSheets has
    no limits; a grid inside a grid is run up to three times per level, so a deep
    document takes it very long. And where a variable-assign cell is reached with
    nothing in hand (at the start of a row, after an operation that failed), TreeSheets
    reads through an empty value and crashes; Wombat then takes the result of the cell's
    own grid, or goes on with nothing in hand if it has none.
54. **Formula cells inside a variable's value count by the result they had at the run.**
    TreeSheets would work such a copy out again, looking up names inside the copy. And
    the names that count as "assigned by the program" (`#N/A` instead of `#NAME?` before
    a run) are taken from the texts of the cells in a variable-assign cell's grid; for a
    formula cell there, Wombat takes the formula, TreeSheets the result.
55. **A run that changes nothing is nothing** (A44, A64); TreeSheets adds an undo step
    for every run.
56. **Where a TreeSheets on another system calculates differently from Wombat** (A60).
    In the plain `C` locale (TreeSheets started without a language setting on Linux)
    only ASCII letters and digits can be in a name without quotes and only ASCII letters
    change case. On Windows text is counted in UTF-16 units, so `LEN` of an emoji is 2
    there and 1 in Wombat (and in TreeSheets on Linux). Which characters are letters
    follows the Unicode tables of the C library there and of the Rust standard library
    here, which can be of different Unicode versions. The median of more than three
    numbers that has both `0` and `-0` in the middle can come out as either zero in
    TreeSheets; Wombat takes them in the order of the cells.
57. **Typing over a formula cell.** In TreeSheets, typing into a selected formula cell
    replaces it and makes it a data cell at the first key; `ts.set_text` also makes a
    formula cell a data cell, even if the new text starts with `=`. Wombat has one rule
    for text that was typed (`EnterText`, A65), and its plain text commands never change
    the kind.

60. **A filter is part of a query, not a state of the document** (milestone 6; numbers
    B58 and B59 are not used). TreeSheets' filters
    mark cells as hidden until the filter is turned off, and three settings say how
    (show whole rows, show sub-grids, keep previous matches). In Wombat a filter is a
    condition in a search that returns cells; nothing is hidden and nothing is kept.
    Showing the row or the grid around a result is the interface's choice; "keep
    previous matches" is `OR`.
61. **"Show N% of last edits" includes ties, and counts per document.** TreeSheets
    sorts the cells by edit time and shows the first (cells x N / 100) + 1, whichever
    order equal times happen to come in. `recent:N%` is every cell edited at or after
    the time of the cell at that position; cells without a time count as oldest.
62. **Replace covers the cell it starts from, and finds folded text.** TreeSheets'
    "replace all" leaves the text of the root (or of the cell the view is zoomed into)
    alone and replaces inside it; Wombat's scope is the cell and everything inside it,
    the same cells that a search within that branch finds. A word that is not marked
    exact also finds accented and full-width forms, which TreeSheets' lower-casing
    does not.
63. **"Go to matching cell" has no preference for looks** (settles B41). TreeSheets'
    F6 and F7 go to the next cell with the same text or image, preferring one whose
    style and colours differ from the current cell's. In Wombat it is "next match" of
    the query `text:"..."` or `image:...`: the next such cell in document order.
64. **"Next match" does not skip folded grids.** TreeSheets, with "restrict to view"
    on, skips what is inside folded cells. Folding is view state (A8), which the core's
    search does not see; "restrict to view" is otherwise `in:` with the cell the view
    shows.
65. **Search finds a word in any field of any document, folded** (B3 in detail). TreeSheets looks in the text of the cells of one document, by lower-casing
    or exactly (one switch, exact by default). Wombat looks in text, note, image name,
    link targets and linked cells' texts of every document of the workspace, ignoring
    case, accents and width (A72) unless the word says `case:`.
