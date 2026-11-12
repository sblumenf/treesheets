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
    *Since changed:* A112 moved the cache out of the workspace, made `Verify` the default and changed the first line.
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
    *Since changed:* A111 changed the table.
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
    *Since changed:* A108 changed several points of the syntax.
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
    *Since changed:* A105 and A106 limit what is replaced; A107 is replace across a folder.
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

Numbers 80 to 89 are from the review of milestones 4 and 5 (they stand before 90 on, which
are where milestones 4, 5 and 6, built apart, meet).

80. **A text that a formula makes is at most 1 MiB** (adds to A67; found in review of
    milestone 5). Texts had no bound: `SUBSTITUTE` seven deep on 32 characters, or 41
    cells that each join the one before to itself, asked for gigabytes, and the process
    was aborted, also when such a document was only loaded. Now every text value of a
    formula (what `&`, `SUBSTITUTE`, `UPPER`, `LEFT` and the like make, a literal, and
    the text of a cell that a name or `SUBSET` stands for) is at most
    `eval::MAX_TEXT_BYTES` = 1,048,576 bytes: a sixteenth of the 16,777,216 code points
    that the text of a cell may have (A20). A cell can hold a pasted document; a
    calculated text is for showing, and each one a formula holds while it is worked out
    counts. A longer one is `#LIMIT!`, an error like `#NUM!` that carries over to
    whoever reads it; the length is worked out before anything is allocated. And making
    text is work (A81): 16 bytes count as one cell read, so that all the texts of one
    recalculation, results included, stay under 32 MB. Running the program does not
    join texts; there the bytes of copied texts count as steps (16 bytes as one) and
    `graph` counts its bars four times.
81. **What a recalculation may cost: 2,000,000 units of work, each formula's cost the
    same however it is reached** (replaces the 50,000,000 cells of A67, and how A63
    decides between part and all; found in review). Measured, 45,000,000 cell reads
    took 13 seconds; a part of the formulas was tried and thrown away first (30
    seconds); and an edit of a cell that no formula reads worked everything out, because
    the check on the total came after the attempt and did not ask whether anything had
    been done. Now:
    - *Work* (`eval::MAX_WORK`) is: one per cell of a grid that is read (all inside it
      for `TREE`), counted when the reading starts, so that it is the same whether the
      reading ends early at an error; one for a grid that was read before in the same
      recalculation (its contents are shared between the formulas, and each aggregate
      of it is worked out once), unless the formula is inside that grid, since its own
      cell does not count for it; one per 16 bytes of text made; 6 per formula and one
      per 32 bytes of its text (finding it, reading it and keeping its result takes as
      long as reading six cells); and at every name, once more what the texts made so
      far cost and a sixteenth of the position in the formula, which is what reading
      the formula again from its start costs if that name has to wait for another
      formula. Measured, a unit is at most about 0.45 microseconds, so the limit is 0.9
      seconds on this machine; `Calc::stats().work` says what the last update took.
      (3,000,000 was tried first and took 1.3 seconds.)
    - *What a formula costs does not depend on who waited for whom*: while an attempt
      only looks for the formulas it has to wait for, nothing is counted, and it is
      counted when the attempt is made again. So "over the limit" is a property of the
      document, and keeping results up to date gives what working everything out gives
      at any limit (tested with limits of 10 to 410 units on random documents).
    - *A change that no formula reads works nothing out*, whatever the total; when
      the total is over half the limit, everything is worked out straight away, once.
      Circles and `#LIMIT!` still mean everything after every change (A63).
    - Beside the limit stands the time that does not depend on what the formulas do:
      before everything is worked out, the document is walked once to know what names
      mean, 0.8 microseconds per cell (0.55 seconds for the 700,000 cells a document may
      have; it was two walks and 1.1 microseconds), and a formula that is over the
      limit still takes 0.6 microseconds to say so. `milestones/M5.md` has the worst
      case measured.
82. **A run holds at most twice a document** (adds to A67; found in review). The
    20,000,000 steps of A67 let one row with a grid of 100,000 cells and 150
    variable-assign cells copy 15 million cells into variables: 2.5 GB, which the
    editor then kept. The cells that a run holds in its copy of the document, in what
    its views were filled with, and in its variables may together be at most
    `eval::MAX_RUN_CELLS` = 1,400,000 (twice A43's limit) with at most
    `eval::MAX_RUN_TEXT` = 64 MiB of text; it is asked before a value is copied into a
    view or a variable, and the run is refused with nothing changed. Variables are still
    copies, not shared: a value in hand is changed in place by the operations after it,
    and sharing would need every such place to copy first. What the editor keeps after
    a run (the variables, and the value of the root cell) is within the same bound.
83. **Hierarchify, flatten, collapse and hierarchy swap go by what cells show**
    (completes A66; the real app was asked, `referee/ops/ui`). Hierarchify groups rows
    by what their first cells show. Flatten leaves, for a formula cell, a data cell with
    its result; so does the copy of a parent in a hierarchy swap, which finds cells by
    what they show. Collapse appends what the cells show; if the first cell of the block
    is a formula cell, it stays as it is and the other texts are gone (TreeSheets
    appends them to the result and then works the formula out again, which comes to
    the same). And `EnterText` with the text that the cell has already (typed with the
    text commands, A65) leaves it alone, styles and links included, unless the cell is
    a formula cell now: TreeSheets changes only the kind when typing ends. Found by
    typing into the real app.
84. **A moved cell can come back** (replaces the last rule of A52; found in review of
    milestone 4). `AdoptCells` refused an ID that the history of the document held, so
    a cell moved from A to B could not move back to A, or on to C and back to B, until
    the history was cleared. The refusal protected nothing: IDs are never given twice
    (A5, A36, A48), so an ID in the history is that of the very cell that arrives, and
    the unit that holds it is older than the command that brings it in. Undo is last
    in, first out: it reaches the older unit only after the newer one took the cell
    out again, so the cell is never in the document twice. `AdoptCells` now refuses only
    an ID that is in the document. The editor counts how often each ID is held, since
    two units can hold one cell now. Tested for A to B to A and A to B to C to B, with
    undo and redo in every order of the editors.
85. **Links are corrected in place** (replaces the last sentence of A51; found in
    review). A new command, `RetargetLinks`, changes the document that links to given
    cells name and nothing else; `move_between` and `LinkIndex::repairs` use it where
    they made new links with `SetLink` before, which lost unknown fields and joined two
    neighbouring links to one cell into one. 45 commands now, with `Import` of milestone 7. `resolve_unknown` still
    makes a new link: it gives a target to a link that had none.
86. **What "nothing is lost" means after a move undone on one side** (corrects A52).
    Undone in the document the cells arrived in, the cells are in neither document;
    they are in that document's redo and in the other's undo. A new command in the first
    document ends its redo, and then the cells exist only in the history of the
    document they left, until that is undone, forgotten or closed. `Editor::holds(cell)`
    says that a cell is in an editor's history and not in its document, so that an
    interface can warn before such a history goes; the core does not know which
    editors are open together, and warns of nothing itself.
87. **Nothing is written through a link at a name the program chose** (adds to A27;
    the second finding of this kind). A27 lets a save follow a link at the document's
    own path, because someone put it there on purpose. Every other name is the
    program's: temporary and backup files, the image folder and the files in it, the
    folder of view states and its files. A link at such a name is planted: files are
    written beside it and renamed over it (`storage::write_own`), and a folder that is a
    link is refused. An image that is a link in the source's folder is not copied out as
    if it were the file. Names that come from a document cannot leave its folder
    (A30). The cache of an index is not covered here (it is reworked in milestone 6).
88. **Images, tags and folds go with cells between documents** (adds to A51; found in
    review). `storage::carry_images` copies the image files that a clip's cells show
    into the image folder of the other document: used as it is if the same bytes are
    there under the name, under a new name if another picture has the name (and the
    clip is changed to say so), refused with nothing copied if a file is missing, since
    after a move the picture would be gone for good. The tags that the cells have are
    added to the other document if it lacks them (B70). On a move the folds of the
    cells go to the other document's view state; a copy has new cells and no folds,
    as a copy within a document. `storage::transfer_cell` does all of it for two files
    and then removes from the source's image folder what nothing there shows any more;
    `links::paste_clip` and `links::move_clip` are the same for editors.
89. **Two documents are saved together** (`storage::save_two`; found in review). Both
    are written beside their files first; the old first file is kept under a second
    name; then each is put in place. If the second cannot be, the first is put back, and
    both are as they were. If that fails too, the error (`Error::HalfSaved`) names the
    file that is new, the one that is old, and where the old content of the first is.
    Between the two renames a crash leaves the first new and the second old, with the
    backup beside them: two files cannot be replaced in one step.

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
      change that, and A81 did: under a second since).
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
    *Since changed:* A106: formula cells are replaced only when asked.
92. **Where search shows or follows a cell, it goes by what the cell shows** (A66,
    as `links::place_shown` does for links): the location trail of a result, and the
    field "text of a linked cell", are the results of formula cells, not their
    formulas. A hit carries both of its own cell: `text` (the formula) and `shown` (the
    result), and the tool prints them as `wombat show` does, `"=SUM(Costs)" = "1050.5"`.
    Filters that ask about a cell's text (`text:`, `tag:`, `has:text`) and the "shorter
    text first" of the ranking still go by the stored text.
95. **Import and export are conversions between values; files are another layer**
    (milestone 7; the numbers A93 and A94 are not used here). `convert` turns a document
    into text and text into a `Table` of texts, and touches no file. An export reads a
    `Source`: the document and the results of its formula cells (`Calc`), both by
    reference, so an export cannot change a document and makes no entry in a history. An
    import gives a `Table`, which becomes a new document (`to_document`) or goes into a
    cell through the editing engine. Writing is `export_files`, reading is the caller's
    (`storage::read_bounded`). The same document and options give the same bytes.
96. **What a cell exports as.** What it shows: for a formula cell the result, as
    TreeSheets exports it and as A66 says for whatever shows a cell. The option
    `formulas` exports the formula instead. Only text is exported, and with the option
    `notes` the note as a column of its own after each column; styles, colours, images,
    links and the kind of a cell have no place in delimited text. The complete form of a
    document is its `.wombat` file.
97. **Defaults of delimited text: comma, quotes only where needed, a UTF-8 byte-order
    mark, CR LF.** Each is what the most common reader, a spreadsheet, takes without
    questions. *The mark:* without it Excel reads a CSV file in the system's old encoding
    and shows every non-ASCII letter wrong, silently; with it, a few other programs show
    three junk bytes in the first field, visibly, and Wombat's own importer drops it. The
    silent damage is the worse one. *CR LF* is what RFC 4180 says. *Quoting* only where
    needed keeps the file readable; a row that is one empty cell is written `""`, since
    an empty line is no row to most readers. All can be switched (`--no-bom`, `--lf`,
    `--quote-all`, `--delimiter`), and `ExportOptions::treesheets` is the set that gives
    TreeSheets' own bytes.
98. **Exported text cannot run as a formula in a spreadsheet, by default** ("CSV
    injection"). A spreadsheet that opens a CSV file treats a field that starts with `=`,
    `+`, `-`, `@`, a tab or a carriage return as a formula, which can read other cells,
    fetch from the network, or on old systems start a program. A document may hold text
    from anywhere, so every field of cell content (texts, notes, column names taken from
    cells, the trail and text in `index.csv`) that starts with one of those gets a single
    quote in front, as OWASP recommends. **Numbers are left alone:** a sign followed by
    digits with at most one decimal point or comma and an optional exponent (`-5`,
    `+2.5`, `-1,5`, `-2e10`) is a number or text to every spreadsheet, never more.
    The price: the quote is visible in the spreadsheet's cell, and a file exported this
    way and imported again has it in the text (`'=1+1`). `--raw` (`guard: false`)
    exports the text as it is, for files that go to a program and not to a spreadsheet.
    IDs, counts and file names are made by the program and are not guarded: an ID starts
    with a hexadecimal digit, a file name with a letter or digit.
99. **The flattened export is by rows, with one block of columns per level of nesting.**
    One line per innermost row (a row none of whose cells has a grid), after the rows it
    is inside of, outermost first. Level 1 is the exported grid; each level's block is as
    wide as the widest grid at that level, so the table is a rectangle and a column means
    the same on every line. Rows of narrower grids are padded on the right; a row nested
    less deep than others has empty fields in the deeper blocks; a row with a nested grid
    has no line of its own (its cells, the text of the cell that has the grid included,
    are on every line from that grid); a row with several nested grids gives the lines of
    each in turn after the same outer row; empty cells and entirely empty rows are kept.
    Where every grid has one column this is exactly the table that `Flatten` makes (a
    test compares them). There is no header line: grids have none.
100. **The lookup-style export (B7): one table per grid, joined by IDs.** A folder with
    one CSV file per grid of the exported cell and below, `index.csv` and `README.txt`.
    - *Columns:* the content first (`A`, `B`, `C`, ..., the names the columns have in the
      spreadsheet they land in; with notes, `A_note` after each), then `row_id`, then
      `A_id`, `B_id`, ... (the ID of every cell), then `<column>_table` for each column
      that has nested grids (the file of the table inside the cell, or empty), then, in
      every table but the top one, `parent_table`, `parent_row_id`, `parent_cell_id`.
      XLOOKUP does not need the key in the first column, so what a person reads is first.
    - *`row_id` is the ID of the row's first cell.* Cells have permanent IDs (A5) and rows
      have none; a nested grid sits in one cell, whose row is found by that row's first
      cell. The key is the same in every export for as long as that cell exists and is
      first in its row; moving columns or deleting the first cell changes it. Giving rows
      IDs of their own in the model was not done for an export.
    - *The join:* `parent_row_id` of any row of a nested table is found exactly once in
      the `row_id` column of the table named in `parent_table`. `parent_cell_id` says
      which cell of that row, and is found in one of its `_id` columns.
    - *`index.csv`:* per table its file, trail (the texts on the way to its cell; a cell
      without text is called by the first text of its row), rows, columns, whether the
      first row was taken as names, parent table, parent row, parent column, and the ID
      and text of the cell it sits in.
    - *`README.txt`* explains this and gives an XLOOKUP formula for the first nested
      table and its parent, with the real sheet names and column letters.
    - *Column names:* letters by default, and every row is data, because a grid has no
      header row and guessing one would silently drop a row of data from some tables.
      With `header_row`, a table takes its names from its first row and leaves that row
      out, unless it has only one row or a nested grid in that row (whose rows would
      lose their parent); names that repeat, or that are those of key columns, get a
      number. The key columns are found by position, not by name.
    - *Nothing is lost:* with the default names, the tree of texts and IDs is rebuilt
      from the folder alone by a test. A single "long" table of one row per cell was
      considered as a second lossless form and not built: the tables already are one.
101. **File names of exported tables come from cell text and are trusted with nothing.**
    `<words>-<suffix>.csv`: the words are the trail with every character that is not a
    letter or digit (of any script) made `-`, at most 22 characters, the end kept; the
    suffix is the first eight digits of the ID of the cell the grid sits in. So a name
    has no separator, no dot, no space; is never `.`, `..`, hidden, an option, or a
    reserved name of Windows; is at most 31 characters, the limit of a sheet name in
    Excel, so that a file opens as a sheet of its own name and the README's formula
    fits; and is unique by its ASCII suffix alone, whatever a file system does with case
    or accents. A table keeps its name between exports while its cell and trail stay.
    If two suffixes of one export are equal (which random IDs do not do; a crafted file
    can) every table is numbered instead (`n1`, `n2`, ...). The writer checks names
    again and refuses anything that is not one plain name.
102. **Import: a table of texts, bounded, plain by default; into a cell it is one
    command.**
    - Delimited text is read as RFC 4180 says, and where a file is not valid, as
      TreeSheets reads it (B82 has the two exceptions). Rows are padded to a rectangle.
      Line breaks inside a quoted field become one space and other control characters
      spaces, since cell text has none (A32). Indented text is read exactly as TreeSheets
      does.
    - Bytes are taken as UTF-8 (with or without a mark), as UTF-16 if they start with
      its mark, and otherwise as Windows-1252, which is what Excel on Windows writes as
      plain "CSV" and cannot fail.
    - **Text that starts with `=` stays plain text** unless the import is asked for
      formulas. A file from somewhere else was written for another program; its formulas
      would mean something else here, or start calculating unasked. TreeSheets' import
      does the same. With the option it is as if each text had been typed (A65).
    - *Bounds:* the file is at most 32 MiB (the caller reads it with
      `storage::read_bounded`); rows times columns after padding, and lines of an
      outline, are under the limit on cells, checked before the rectangle is made; a
      field is at most as long as a text may be; nesting stops at the depth a document
      may have (deeper lines become rows of the last level, as in TreeSheets). Reading
      takes time in proportion to the file.
    - *`Command::Import { cell, table }`* gives the cell the table as its grid, in place
      of the grid it may have, and leaves the cell's own text. The cells are new, with
      IDs from the `IdSource`, and look like the cells of a new grid of that cell
      (`CreateGrid`). It may be the root. It is one unit of undo, reports what changed
      like every command, and is refused if the document would have too many cells, nest
      too deep or be too large to save.
103. **An export writes only where it was told, and never through a link.** Two reviews
    found writes that followed a planted symbolic link. For exports (`export_files`):
    an existing file, or a folder with anything in it, is not written to without
    `overwrite`; a symbolic link at the file, at the folder or at a file in the folder is
    refused, with or without `overwrite`, and is neither followed nor removed; a file is
    written as a new temporary file beside it (created so that an existing name fails,
    after removing what was there) and renamed into place, which replaces a name and
    follows nothing; every name of a folder export is checked to be one plain name, and
    all of this is checked before the first file is written. With `overwrite`, files in
    the folder that the export does not name are left alone, so tables of an earlier
    export whose grids are gone stay there; `index.csv` says which tables are current.
    Links in the folders above the one named are the user's own and are followed.
104. **An export is bounded too.** At most 512 MiB of text per export (a flattened
    export repeats the outer rows on every line, so it can be far larger than the
    document) and at most 10,000 tables in a lookup export; beyond that it is refused
    with the advice to export a part by its cell.
    *Since changed:* A110: filters and the tie-break go by what is shown too.
105. **Replace changes only whole characters** (review of milestone 6; numbers 93 to 104
    are used by other work). A match in folded text can cover part of a character: `s`
    is found in the `ß` of `Straße` (folded `ss`), `f` in `ﬁ`, `1` in `½`, one person in
    a family emoji. Search still finds and highlights those cells. Replace takes a
    place only where the match is the whole of one or more user-perceived characters;
    any other place is left alone, counted, and listed with the characters it touches
    (`replace::Counts::skipped`), by the dry run as by the run. After such a place the
    search goes on one character further, so `ss` still replaces the `ß` of `sß`.
    Before this, `s -> X` made `XtraXe` of `Straße` without a word.
106. **Replace leaves formula cells alone unless asked** (replaces the second half of
    A91). `ReplaceOptions::formulas` (the tool's `--formulas`, `formulas` in
    `replace-all`) includes them; then, as A91 says, the formula is what is replaced,
    and the cell calculates again. Without it formula cells are skipped and the result
    and the dry run say how many of them had how many places
    (`Counts::formula_cells`, `formula_places`). Replacing the word "sum" in a folder
    of notes must not turn every `=SUM(` into something else. Renaming a table in the
    whole document therefore needs the option.
107. **Replace across a folder prepares everything, then saves, and is not atomic**
    (`wombat_search::workspace::replace_in_folder`). Every document is read and changed
    in memory first; a replacement refused in any of them saves nothing. Then the
    changed documents are saved one by one (each file whole or not at all); if
    one fails, saving stops and the result names the files saved, the one that failed
    and the ones not reached. There is no undo across files: a second run finishes the
    job, since what was replaced is no longer found. The changed documents are all in
    memory until they are saved.
108. **The text syntax forgives ordinary typing** (changes A75 in these points; review
    of milestone 6).
    - `-` before a digit (`-5`, `-0.5`, `-.5`) is part of the word: a negative number
      is a thing to search for in a document of figures. "Not 5" is `NOT 5` or
      `-"5"`.
    - `AND` and `NOT` are operators like `OR`, in upper case only; `a AND b` is `a b`.
      The words themselves are searched in quotes (`"AND"`); in lower case they are
      words. Left hanging (`a AND`, `a NOT`) they count for nothing, like `a OR`.
    - The curly quotes `“ ” „ ‟` are quotes: phones type them in place of `"`.
    - A word with a colon is a filter only if its first part is a filter's name.
      Anything else is text, as before (`12:30`, a web address); if it reads like an
      attempt at a filter (`status:done`), `Parsed::warnings` says that it was
      searched as text. `is:` and `in:` are words that anyone types before a colon, so
      they are filters only with a value they can mean (`is:tagged`, `in:` with a cell
      ID) and text with a warning otherwise (`is:open`, `in:progress` used to be
      errors). Every other filter with a value that means nothing is still an error
      that says what would, because there the writer meant a filter.
    - A word with nothing to search for (empty, or folding to nothing: a lone accent,
      a soft hyphen, a variation selector) matches **no** cell and `Results::warnings`
      says so. It used to match every cell.
    The `Query` value and the text agree: everything above is how text becomes a value;
    only the last point is about values.
109. **`tag:` is the row of a tag cell; `is-tag:` is the tag cell** (B90). In text,
    `tag:NAME` is `Filter::RowTag`: the cells in a row of a grid in which some cell is
    the tag NAME (the root cell, which is in no grid: itself). `tag:todo apple` is
    therefore "rows tagged todo that mention apple", which is what the words say, and
    `tag:todo` alone is every cell of those rows. `is-tag:NAME` is `Filter::Tag`,
    TreeSheets' own filter: the cells that are the tag. `has:tag` stays "is any tag".
    Only the row counts, not the grid inside a cell of the row, nor the rows above or
    below: narrowing further is `in:`. The index keeps the number of columns of each
    cell's grid for this (4 bytes a cell; the summary's version is 3).
110. **Whatever asks what a cell is to the eye goes by what the cell shows** (replaces
    the last sentence of A92). `text:`, `tag:`, `is-tag:`, `has:tag`, `has:text` and
    the "shorter text first" of the ranking go by the result of a formula cell, as the
    trail and the linked-text field already did. Words still find a formula cell by
    its formula *and* by its result (A90), as two fields. `formula:TEXT` finds formula
    cells whose formula contains the text (folded, like a word); `kind:formula` finds
    them all.
111. **The table of the index proposes exactly, in bounded space; a search reads a cell
    once** (replaces the description of the table in A73 and the engine's part of A79;
    review of milestone 6).
    - *The table.* Every folded character and every pair of neighbours is a key (A73
      had single characters only for scripts without spaces, and dropped the list of
      any key that an eighth of the cells had, so that common words looked at every
      cell). Keys are not stored: each is hashed into one of a fixed number of lists
      (twice the number of cells, a power of two), built packed in one array; a list
      that more than one cell in 32 is on is kept as one bit per cell. Keys that share
      a list share its cells, which costs a look at a few more cells and can never
      give a wrong result, since every proposed cell is checked against its text.
      Edits add to a small table beside the packed one; when half is stale, or it has
      doubled, it is built again. On varied text the index took 29 times the text and
      now takes 8 (cells of 12 characters) or 1.4 (cells of 400).
    - *A search.* Each word gets, from the table, one bit per cell: may have it or
      cannot. The cells looked at are those the whole query allows (and of bits, or
      of bits); a word that the bits rule out for a cell is not looked for there, which
      is what makes negations and words that are absent cheap. A word is prepared for
      searching once per query (`memchr`), not once per cell. From four words on, the
      third word that needs a cell read makes the cell be read once for all of them
      (`aho-corasick`). The same word twice is one needle counted twice. Only the
      results asked for are kept while searching, and the order of the document does
      not work out points for results it does not return. Both crates are pure Rust,
      build for the web, and are in most Rust programs already.
    - *What cannot be had:* stopping early for "the best N". The points of a cell are
      only known by reading it, and nothing cheaper bounds them, so every matching
      cell is read; a query that matches most cells stays linear in the workspace.
    - *A hit carries at most 1,000 highlight ranges* (`MAX_RANGES`) and the number of
      all (`Hit::range_count`). The limits of A75 on a query stay as they were.
    - *Following a document that the index does not have is an error*
      (`SearchIndex::apply` returns `NotIndexed`). It used to read the document in under
      an empty name, so that a caller's mistake showed up later as results from a
      document called "".
112. **Index caches are outside the workspace, and checked by content unless asked
    otherwise** (replaces the place, the entry's first line and the default of A71,
    and the place in A54; review of milestone 6).
    - *Why.* A cache entry cannot prove where it came from: whoever can write the
      folder can write an entry that fits a document and says something else, and it
      was believed, also with every file checked. A workspace folder comes from
      archives, shared drives and sync services. Signing entries would need a secret
      kept somewhere else, which is the same as keeping the cache somewhere else.
    - *Where.* `scan` takes a `Cache`: the folder and the mode. `Cache::in_base(base,
      workspace, mode)` is the folder `<name>-<checksum of the full path>` under a base
      that the interface chooses: the cache directory an app is given on a phone; for
      the tool `$WOMBAT_CACHE_DIR`, else `wombat` under `$XDG_CACHE_HOME` or `~/.cache`,
      `~/Library/Caches`, `%LOCALAPPDATA%`, or `--cache-dir`. A cache folder that is
      inside the workspace (also through a link) is not used. `.wombat-index` in a
      workspace is never read, written or deleted. This also takes the cache out of
      synced folders, where it doubled the number of files.
    - *An entry is of one file.* Its first line ends in one checksum over the line, the
      name of the document file and the body, so an entry made of parts of two, or
      put under another name, is refused. This is against accidents, not forgery: the
      checksum is not secret, and the defence against forgery is the place.
    - *`Verify` is the default everywhere*: every document is read and its checksum
      compared; only parsing is saved. On 5,000 small documents `Fast` saved 33 ms of
      115, and it trusts what a sync client or an unpacker can restore. `Fast` remains
      for those who ask (`--cache=fast`), with the risk stated at `CacheMode::Fast`,
      and now also compares, on Unix, the inode and the time of last change, which a
      program cannot set back. `scan::clear_cache` ("rebuild the index", `--rebuild`)
      discards the cache of a workspace.
    - *The tool keeps a cache by default* (`--no-cache` to keep none), since with
      `Verify` it cannot make a result stale: `search`, `links`, `backlinks` and
      `broken` alike, with the same options.
    - The symbolic-link rules of A71 hold for the new place: the cache folder of the
      workspace, the folders in it and the entries must be real.

113. **A field with a comma, a semicolon or a tab is quoted, whichever is the
    delimiter** (fixes after the review of milestone 7; changes A97. The numbers A105 to
    A112 are not used here). LibreOffice's import dialog proposes all three as separators
    at once, so a cell `Ana; Bob` in a comma separated file shifted its row by a column
    and the lookup join gave `#N/A`. `Quoting::Minimal` now quotes for all three, plus the
    quote and line breaks as before; `Quoting::Narrow` is the old rule (only the
    delimiter), kept because `--treesheets` TSV is compared with TreeSheets' bytes. The
    tests read every export a second time with a reader that splits at all three, and
    get the same table.
114. **The guard against formula injection guards what could run, and says that its
    quote is visible** (changes A98). The old rule put `'` before every text that
    started with `=`, `+`, `-` or `@` unless it was a number, which damaged list items,
    dashes, telephone numbers and `@names`; and its claim that spreadsheets take the
    quote as "this is text" was false: LibreOffice (verified) and Excel (documented) show
    it as a character when they open a CSV file. The rule now, by the first character
    (a full-width `＝ ＋ － ＠` counts as the plain one):
    - a tab or carriage return: guarded;
    - `=`: guarded, except the text `=` alone (LibreOffice evaluates `==` and
      `= in prose` to an error);
    - `+` or `-`: guarded unless the text is only signs, dashes and spaces; or a sign
      followed by digits, spaces and `( ) . , / -` with at least one digit; or a number
      with an exponent; or a sign, a space and text with no parenthesis, no `|` and no
      `!` before a letter, digit, `$`, `'`, `_` or `#`;
    - `@`: guarded only with a parenthesis, a `|` or such a `!` in the text.
    A parenthesis is what a function call needs, `|` what a DDE link needs, `!` what a
    reference to another sheet or workbook needs; without them a formula can only
    calculate or fail. Where unsure the rule guards (`-item`, `-A1`, `+1+1`,
    `- call (x)`). What is left alone can still be *shown* differently by a
    spreadsheet (Excel is documented to make `=+1-555-0100` of `+1-555-0100`, and
    `#NAME?` of `- item`; LibreOffice does neither): that is the same kind of change as
    `00123` becoming 123, not an attack, and is listed with it in M7.md.
    **Taking the guard off again:** a text that starts with `'` before something the
    rule guards gets one more quote (`'=1+1` is written `''=1+1`), so
    `unguard(guard(t)) == t` for every text, and `Table::unguarded` (`--unguard`) gives
    back exactly what was exported. It is an option and off by default, because in a
    file from anywhere else `'=1+1` cannot be told from a text its author wrote so.
115. **The encoding of an imported file is never guessed** (changes A102). One byte
    that was not UTF-8 made the whole file Windows-1252, silently: `café` became `cafÃ©`.
    Now `decode` reads UTF-8 (a byte-order mark accepted and dropped), or UTF-16 if the
    file starts with its byte-order mark, and anything else is an error that names the
    first byte that does not belong and the encodings that can be asked for: `utf-8`,
    `utf-16le`, `utf-16be`, `windows-1252`. Half a UTF-16 unit or pair is an error too,
    not a replacement character. `decode` returns the encoding used and the tool prints
    it.
116. **An export replaces a regular file and nothing else, however the path is
    written** (extends A103). `--overwrite` replaced a device node with a regular file
    (the temporary file was renamed over it), and `link/` was written through where
    `link` was refused, because a path that ends in a separator is resolved through the
    link. Now a path is first reduced to its components (no separator or `.` at the
    end); at the place of a file only nothing or a regular file is accepted, and a
    device, FIFO, socket, folder or link is refused with or without `overwrite`; the same
    for every file of a folder export, checked before anything is written. The tool's
    `new_path` (used by `new`, `import`, `copy`) refuses a name that ends in a separator
    and anything that is there, a link to nothing included. Tests use FIFOs, sockets
    and links in a temporary folder.
117. **Writing a lookup folder again removes the tables of the export before, and the
    names of tables fit a sheet name exactly** (changes A101 and A103).
    - With `overwrite`, a folder that has anything in it must have an `index.csv` of a
      lookup export (its first line is the column names, with any delimiter), else it is
      refused: it is not Wombat's. The tables that index lists and the new export does
      not write are removed first: only plain names ending in `.csv` that are regular
      files in that folder. Other files stay. Before, tables of grids that were gone
      stayed beside the new index and still joined.
    - The files are written tables first, then `README.txt`, then `index.csv` last. A
      folder is not replaced atomically: a folder that is not empty cannot be renamed
      over, and removing the old one whole would remove files the export did not write.
      If writing stops half way, the folder has the old index or none; exporting into
      it again with `overwrite` puts it right in the first case, and in the second the
      folder is refused and has to be removed by hand.
    - A name is at most 31 UTF-16 units, which is how Excel counts a sheet name (22
      letters outside the BMP were 53). The suffix is eight digits of the cell's ID;
      among 10,000 tables two share them in about one export in a hundred, and then
      only those tables get sixteen digits, and only if those are equal too (a crafted
      file) a number. Before, one collision renamed every table. Names still hold only
      letters, digits and `-`, so none of `[ ] : * ? / \ '`, and differ in a lower-case
      ASCII suffix, so also without regard to case.
118. **A flattened row with several nested grids says which cell each line is inside
    of, and exports can say how many error values they wrote** (extends A99).
    `P,Q,p1 / P,Q,p2 / P,Q,q1` did not say that `q1` is inside `Q`. When some row has
    more than one nested grid (or always, or never: `parent_columns`), every line ends
    with one column per level below the first: the letter that the column of the parent
    cell has in the flattened table itself (`P,Q,p1,A`, `P,Q,q1,B`), empty where the line
    is not nested that deep. Letters of the same table can be checked by eye and used in
    a filter; at the end they shift no other column. `error_cells` counts the formula
    cells in the exported grids whose result is an error value (`#LIMIT!`, `#NAME?`),
    and the tool prints the count after an export.
119. **An import is refused early if a document could not hold it, and replaces
    content only when asked** (changes A102). `Table::new` adds up what the cells would
    at least take in a saved document (49 bytes a cell, and its text with 12 more) and
    refuses a table over the 32 MiB of a file with the number of cells offered and the
    number there is room for; before, 600,000 one-letter rows were made into a document
    and refused at saving, with a message about bytes. (The 700,000-cell limit cannot
    be reached through an import: 684,784 empty cells fill a file.)
    `Command::Import` has `replace`: without it, a grid any of whose cells has text, a
    note, an image or a grid is not replaced. The cells removed are in the change
    report; the links that pointed at them are what `LinkIndex::would_break` gives for
    the cells of the grid before the command, which the tool prints for the document
    itself.
120. **The delimiter of a `.csv` file is looked for in the file** (`delimited::detect`).
    The first 100 lines are read as the importer reads them (delimiters in quoted
    fields do not count). A candidate occurs in every line the same number of times.
    One candidate: that one. A tab among several: the tab. Comma and semicolon both:
    the semicolon if every comma stands between two digits (decimal commas), else no
    answer. Fewer than two lines, or no candidate: no answer, and the caller takes what
    the file's name says (comma). The tool does this for `.csv` only, `--delimiter`
    overrides it, and it prints which delimiter it used and why.
121. **Flutter for the interface**, one codebase for the iPhone, Android, macOS, Windows and Linux
    apps, calling the Rust core natively through `flutter_rust_bridge`. Replaces the interface part
    of A2: WebAssembly is not a target. Flutter builds every app from one project with no separate
    wrapper, draws identically on every platform, and gives smooth scrolling and gestures on
    phones. The interface is designed for the phone first and widened for desktop.
    (Recorded as A16 on the interface branch, where that number was still free; renumbered when
    the two branches were merged. The core is still checked for WebAssembly in CI because it
    costs nothing and catches file or thread use creeping into the pure core; it is no longer a
    requirement and can be dropped if it gets in the way.)
122. **The bridge crate may hold `unsafe` in one generated file.** Calling between Rust and
    Dart needs it, and `flutter_rust_bridge` writes that code (`crates/wombat-bridge/src/
    frb_generated.rs`). So that crate cannot take the workspace's lints, which forbid `unsafe`:
    it writes the same lints out with `unsafe_code` denied instead of forbidden, and lets off
    only the module `frb_generated`, in `lib.rs`. The generated file also trips
    `clippy::unwrap_used`, let off in the same place, and no other lint: the list there is
    those two, each checked to be needed. Everything written by hand in the crate is
    under the same rules as the rest of Wombat. Both generated sides (that file and
    `app/lib/src/rust/`) are committed, so that the three checks pass without the generator.
123. **The serialised form of commands and reports is a set of mirror types in the bridge crate
    with `serde`** (`api/wire.rs`), not a `serde` derive on the core's types. One command of the
    core is one variant with the same name and fields; `flutter_rust_bridge` generates the Dart
    classes from the same types, so Dart, the bridge and JSON share one form. The core gains no
    dependency (it still uses `serde_core` only), and a derive on it would have let a caller
    state what the core computes (the height of a clip, the cell count of a table). What
    differs from the core's types:
    - IDs are their canonical text; anything else is refused.
    - Unknown fields of an image are the text of a JSON object (empty for none). It is read with
      `serde_json`, which keeps the last of a repeated key where the file reader refuses; the
      command still checks the fields when it is applied.
    - A scale that is not a finite number is refused, in both directions: JSON cannot write it.
    - A table to import goes through `Table::new` and `Entry::new`, which check and count.
      It is carried as a flat list (A135).
    - As JSON (`record.rs`) it is what `serde` writes by default: `{"SetText": {"cell": "...",
      "text": "..."}}`, and `"Run"` for a command without fields.
    This form is independent of the script language of `wombat edit`, which stays what the
    referee cases are written in.
124. **A clip crosses a boundary as a document.** `Clip::as_document` gives the cells as a
    document whose root is the top cell, and `Clip::from_document` makes the clip again, checked
    as `Clip::of` checks. The bridge carries the text of that document and the cell the top
    cell was in (A132: the ID of the document says where the cells came from). So the two paste commands are
    in the serialised form like the other 43, and there is still no way to make a clip of cells
    that break the model's rules. (The bridge has no call yet that gives Dart a clip; that comes
    with copy and paste in the interface.)
125. **Reading one grid is a query of the core**, `Editor::grid_view`: a cell and the cells of
    its grid, each with its text, kind, what a formula cell shows, whether that is an error
    value, and whether it has a grid. A cell without a grid gives 0 by 0 cells, so a new
    document, whose root has no grid, reads without an error. The bridge only converts it.
126. **What crosses the bridge after a command is the change report with the recalculated
    cells in it** (`Report.recalculated`). The core's `Changes` stays as decision A90 left it;
    the bridge adds `Editor::recalculated()`, because an interface that redraws only what a
    report names would otherwise show old results of formulas.
127. **An open document is an opaque handle** (`OpenDocument`: editor, file and what it looked
    like when last read or written, ID source, clock, whether saved), not something looked up
    by document ID: two files can have the same ID (A28). Every call is asynchronous on the
    Dart side, and the core's work runs off the thread that draws. (Not all of a call does:
    Dart writes the arguments and reads the answer on its own thread, and the arguments are
    read on the Rust side inside the call itself, before the work is handed over. For an
    import of 100,000 cells the evaluation after the review measured 34 to 130 ms of the
    caller's time, of 464 to 736 ms in all; so much data belongs in a call that takes a
    path or bytes.)
    `flutter_rust_bridge` keeps two calls on one document from running at the same moment,
    with a lock. It does not keep them in order: each call goes to a pool of threads, and of
    two that wait either may go first. The review saw 40 insertions into one cell, made
    without waiting, end with the wrong text in 6 of 20 rounds. So the caller sends one call
    at a time and waits for its answer before the next; the comment of `OpenDocument` says
    so, and the app is to do it in one place on the Dart side, where the order of calls is
    known. A test in the core asserts that `Editor` can move between threads. Only plain
    `apply` is bridged; joining typing into one undo unit and groups come with the real
    interface.
128. **The bridge supplies what the core takes as arguments:** new IDs from a generator seeded
    from the operating system once per open document (`getrandom`), and the time from the
    device's clock. Tests give both.
129. **Errors cross as `{kind, message, path}`; a panic is caught, and what it leaves is not
    used.** The kind mirrors the variants of the core's error, for the interface to switch
    on; the message is the core's English text. The bridge adds kinds of its own:
    `BadRequest` for what is wrong with something sent over it (A133), `ChangedOnDisk`
    (A140), `AlreadyOpen` (A141), `Closed` (A142) and `Internal` (A143).
    A panic is a fault in the program; the core is written not to have any. What happens
    if there is one all the same was tried by the evaluation after the review, in a copy
    of the bridge with a function that panics on purpose:
    - In a call, the panic is caught by `flutter_rust_bridge` where the work runs
      (`catch_unwind`), and Dart gets a `PanicException`, not a crash. Its text can hold a
      backtrace with the paths of the machine that built the app, so an interface shows a
      sentence of its own and not that text.
    - The lock around a document is not poisoned by a panic (it is tokio's, which has no
      poisoning). After a panic in a call that changes a document, the document was still
      there, half changed, readable, called saved, with the command in its history.
      Decision A143 is about that.
    - A panic while a handle is dropped is not caught: the generated function that drops
      it is a plain `extern "C"` function, and a panic that reaches the end of one ends the
      process. What the bridge does in a drop is therefore guarded by itself (A142).
    - All of this needs panics to unwind. Nothing in the workspace sets `panic = "abort"`;
      a comment at the end of the workspace's `Cargo.toml` says that nothing may, and
      `crates/wombat-bridge/src/lib.rs` does not compile if something does (tried once with
      `CARGO_PROFILE_DEV_PANIC=abort`).
    - No test in the repository sends a panic across to Dart: that would need a function
      that Dart can call and that panics, in the library that ships. The tests of what the
      document does after a panic are inside the crate (`src/panic_tests.rs`) and use
      switches that exist only in its test build.
    The library is started with `PanicBacktrace::setup()` only, not with
    `flutter_rust_bridge`'s `setup_default_user_utils`. That also set `RUST_BACKTRACE` in
    the environment of the process, while other threads of the app run and may read the
    environment, which C does not make safe; and on Android, iOS and macOS it sent whatever
    any library logs, down to the finest level, to the system's log, in release builds too.
    Nothing in Wombat logs. The price is that a panic is reported with a backtrace only
    when the app was started with `RUST_BACKTRACE=1`. A test holds that starting the
    library leaves that variable alone.
130. **The app opens the example through a copy with fresh IDs** (`OpenDocument::open_copy`,
    which is `storage::copy_document_with_state`): every install has the same example file,
    and A28 says a copy gets its own IDs. A copy that fails leaves nothing behind (A139), and
    the editor that is made of the copy afterwards cannot fail, because it refuses only what
    the save, which has just written that very document, refuses too; so there is never a
    file of a copy without an open document for it. The same holds for a new document. A new document is saved where it is created, as `Workspace::create`
    does. The bare app writes into the system's temporary folder; where documents live, opening
    from bytes, and Android's picked documents (which are not paths) are for the real
    interface.
131. **Three things the build tools decide.** The package is named `wombat_bridge`, with an
    underscore unlike the other crates, because the app's build (cargokit, in
    `app/rust_builder`) looks for a library with the package's name. That build compiles the
    bridge with rustup's `stable` toolchain, not the version pinned in `rust-toolchain.toml`:
    cargokit knows only stable, beta and nightly. The checks and CI use the pin. And the crate,
    the Dart package and the generator must be the same version of `flutter_rust_bridge`
    (2.13.0), pinned exactly in all three places.
132. **A clip says once where its cells came from** (from the review of the bridge; changes
    A124). The carried form had the ID of the source document twice, as a field `source` and
    as the ID of the document that carries the cells, and when the two differed the field won
    without a word. `ClipData` has no `source` now, and `Clip::from_document(doc, parent)`
    takes the source from the ID of `doc`, which is what `Clip::as_document` writes there.
    The check in `as_document` stays: a document under construction is not checked, and a
    clip of one can break rules that only a whole document has (a test builds two such).
133. **What the bridge itself refuses has its own kind of error, `BadRequest`** (from the
    review; changes A129). `Invalid` meant both "the file is broken", which a user can be told,
    and "what the interface sent is malformed", which is a mistake in the interface. Now
    `BadRequest` is everything the bridge refuses before the core is asked: text that is no ID,
    a number out of range or not finite, unknown fields that are no JSON object, JSON text
    that is no command. `Invalid` is only what the core means by it, which includes the
    document that carries a clip, since the core checks that. (A table to import that is
    malformed was `Invalid` too; since the second review it is `BadRequest`, A153.)
134. **Whole numbers that Dart sends cross in 64 bits and are checked on arrival** (from the
    review; adds to A123). A Dart `int` has 64 bits. The mirror types had the core's widths
    (`u32`, `i32`, `u8`), and the generated Dart wrote only that many bits, so row 4294967296
    arrived as row 0 and a relative size of 4294967299 as 3, with no error. Every whole number
    in `Command` and what it holds (`Prop`, `Rgb`, `TableData`) is now `i64` in `api/wire.rs`,
    and `convert.rs` turns each into the core's type with a checked conversion, refusing
    what does not fit as `BadRequest`. JSON follows the same rule, since it is the same types.
    Numbers that only go from Rust to Dart (`GridInfo.xs` and `ys`) stay as they are: a
    narrower number always fits a wider one. In Dart these numbers are plain `int`:
    `app/flutter_rust_bridge.yaml` sets `type_64bit_int`. Without it the generator writes
    `PlatformInt64`, a name that is `int` on phones and desktops and a big-number type on
    the web, so that every signature shows a type that means nothing where the app runs.
    With it, phones and desktops send exactly the same eight bytes as before (the generated
    conversion is the identity there; the tests of numbers that do not fit still pass); on
    the web, which is not a target (A121), numbers beyond 2 to the 53rd would lose
    precision, where before they would have been exact.
135. **A table to import crosses as one flat list** (from the evaluation after the review;
    changes A123). `TableData` was a table whose cells could hold tables. The generated decoder,
    the conversion and the freeing of such a value all call themselves once per level, and the
    first two run before any check of ours can: a table nested a few thousand deep overflowed
    the stack of the thread, which ends the process and is not a panic that anything catches.
    Now `TableData.cells` is every cell in document order, each followed at once by the cells of
    the grid inside it, and a cell with a grid says only the size of it (`GridSize`: columns and
    number of cells). No type in `api/wire.rs` contains itself any more, so nothing generated
    recurses. `convert.rs` puts the core's `Table` together with a list of the grids that are
    open; `Table::new` refuses one nested deeper than a document may be (`Limit`) as soon as it
    is made, so nothing deeper is ever built. A list that is no table is a `BadRequest`, refused
    by the bridge before the core sees it: a grid that says it has more cells than follow, and a
    grid, the outermost or one inside, with no cells, no columns or a row that is not whole
    (A153). `Limit` stays for a table nested too deep, with too many cells or with a text too
    long. JSON has the same flat form.
136. **The JSON form is strict** (from the review; adds to A123). As `serde` reads by default, a
    field that may be none was none when left out, and unknown fields were passed over: a
    recorded `SetRangeColor` without its `color` removed the colour, and a misspelt field was
    read as if it were not there. Now every type of `api/wire.rs` refuses unknown fields, and a
    field that may be none must be there, as `null`. Unknown and missing fields were not all
    that `serde` reads and never writes: it also reads an object from a list of its values, and
    `{"Run": null}` for `"Run"`. Both are refused since A152, which compares what was read with
    what is written. None of this touches what crosses between Dart and Rust: the generated code
    writes every field in order, without names.
137. **Half a surrogate pair in a Dart string is documented, not refused** (from the review).
    A Dart string may hold one, which is no character; Dart's encoder writes U+FFFD in its
    place before Rust sees the string, so the bridge cannot tell it from a U+FFFD that was
    meant. Refusing would need a check of every string on the Dart side. `api/wire.rs` says
    so where a Dart developer reads it, and a Dart test pins it. And because the generator
    does not carry the comment of an enum that holds data (`Command`, `Prop`, `LinkTarget`)
    into Dart, the rules a Dart developer needs are stated on variants, fields and `Report`.
    The comment of the handle `OpenDocument` is not carried over either; what it says is
    repeated on `open`, and the rules for every command are on `apply`.
138. **An open document knows its file by length, time and, on Unix, which file it is** (from
    the review of the bridge). A program that keeps a document open must know, when it saves,
    whether another program changed the file. Reading a file of up to 32 MiB at every save to
    compare it is too dear, so the storage layer notes a `FileState` when it loads, saves or
    copies a document: the length, the time of modification and, on Unix, the device and the
    file's number on it. `FileState::check` compares that with what is at the path, following
    a symbolic link as saving does, and answers unchanged, gone or changed. The state of a
    saved file is taken from the new file before it is given its name, and that of a loaded
    file from the open file before its bytes are read, so that a change by someone else in
    that moment is reported and never remembered as one's own. The file's number matters
    because most programs save by putting a new file under the old name, which this sees
    whatever the length and clock say. Not seen: the same file rewritten in place with the
    same length and the same time of modification (within one tick of the file system's
    clock, or by a program that puts the old time back), and on Windows, where the standard
    library does not give the file's number yet, any change that keeps length and time. The
    time of the last change of permissions, owner and attributes is left out on purpose:
    backup, search and synchronisation tools change those on files they do not edit, and a
    false "changed on disk" teaches people to overwrite without looking. Between checking
    and writing, another program can still save and be overwritten; closing that gap would
    take locking, which is not attempted (`write_atomic` already says that two programs
    saving at once are not protected from each other). The same gap is in creating: `create`
    looks whether a file is there and then writes, and a file that another program makes at
    the path between the two is replaced unseen; so is the file that a copy makes. The
    comment of `create` says so (from the second review).
139. **A copy of a document that fails removes what it made** (from the evaluation after the
    review). `copy_document` copies the images, then the view state, then saves the document,
    so that a copy that is there is complete. When a later step failed, the image folder with
    its pictures and the view state of a document that does not exist stayed, and nothing
    ever removed them. The copy now notes, before it starts, which of those files and folders
    are not there yet, and removes exactly those when it fails; a folder is removed only if
    it is empty. What was there before the call is never removed. An image file of the same
    name that was already in the copy's image folder has been replaced by then, as before
    this decision, and is not put back.
140. **Saving an open document never goes over another program's work without being told
    to** (from the review; uses A138). `OpenDocument` keeps the `FileState` of its file from
    opening, creating, copying and every save, and `save()` looks at the file first:

    | | The file is as it was | The file is gone | The file was changed | It cannot be looked at |
    | ---- | ---- | ---- | ---- | ---- |
    | Something was done since | written | written | refused: `ChangedOnDisk` | `Io` |
    | Nothing was done since | left alone | written | refused: `ChangedOnDisk` | `Io` |

    Before, `save()` trusted its own flag: after the file was deleted or replaced it answered
    success and wrote nothing, and with something to save it replaced whatever was there. A
    file that is gone is written again because the document that is open is the only copy
    left. A file that was changed is refused also when nothing was done to the document, so
    that "saved" is never answered about a file that holds something else. A path that is a
    link leading nowhere is an error as it is for every save of the core (`write_atomic`),
    and stays one for `overwrite()`.
    `overwrite()` writes without looking, for when the user chose this document over the
    file. It is a second call, not an argument of `save`: in Dart every argument is named
    and required, so an argument would have put `overwrite: false` on every ordinary save
    and made the dangerous call differ from the safe one by one word; a separate name can be
    searched for and is never called by a wrong value.
    `is_saved()` says whether anything was done to the document since its file was read or
    written. It does not look at the file, so it stays true when the file is deleted; what
    happened to the file is found by `save()`. It is a flag that every change clears, undo
    and redo included, not a comparison: undo and then redo after a save lead back to what
    was saved and the document is still called unsaved; saving then writes the same bytes
    again. A command that changes nothing in the document leaves it as it is: one that asks
    for nothing new (an empty report), and a `Run` after which formula cells show something
    else but no cell is different (a report with only `recalculated`; A151).
141. **A file is open in one document at a time** (from the review). Two `OpenDocument`s on one
    file each saved over the other's work without a word. The bridge keeps a set of the files
    that documents of this process have open (`open_files.rs`); `open`, `create` and `open_copy`
    (for the file it makes) first claim the file and are refused with `AlreadyOpen`, with the
    path as it was asked for, when a document has it. The claim is part of the document and is
    given up when the document is closed or dropped, and with it when opening fails half-way, so
    a file is never held by a document that does not exist. Checking and claiming are one step
    under one lock, so of two opens at the same moment one is refused. What is copied from may
    be open: a copy only reads it. A file is known by up to three names, however the path is
    spelt, and is open if a document holds any of them (A150): the real name of the file that a
    save would replace (`fs::canonicalize`), the name below its real folder (or below the
    nearest folder above that is there), and the path as asked, from the root. A file that is
    not there yet (`create`, the target of a copy, a file removed while it was open) has the
    last two. A path through a link that leads nowhere has them too and no real name; nothing is
    opened or made through such a link anyway. Not seen to be the same file: a second hard link
    to it, and, for a file that is not there yet, a name that differs only in a way the file
    system ignores (case on macOS and Windows). A claim stays on the names the document had when
    it was opened: on the file that the path named then, and on the path itself. If a link on
    the way is changed while the document is open, saving follows the new link; the path is
    still held, so nothing else is opened or made through it, but the file the link now leads to
    is not held under its own name. The set is the bridge's one piece of state shared by the
    whole process; it is not in the core, which has none. It knows nothing of other processes:
    what another program does to the file is seen at saving (A140). A lock that a panicking
    thread left poisoned is used all the same, since no step on the set can leave it half done.
142. **An open document is ended by closing it, and saved when it is dropped unclosed** (from
    the review). There was no close: a handle that Dart let go of took its unsaved edits with
    it. The interface has no notion of throwing edits away (`UI_SPEC.md` has no Save, Close
    or Discard at all, which this does not contradict), so:
    - `close()` saves what is unsaved, as `save()` does and with the same refusals, and then
      gives the document up: the file may be opened again, the memory is freed, and every
      later call on the handle, a second `close` included, is refused with `Closed`. If the
      save fails, the error is returned and the document stays open exactly as it was, so
      nothing is lost; from there `overwrite()` and `close()` keep this document, and
      disposing of the handle keeps the other program's file.
    - With nothing unsaved, `close()` writes nothing and does not look at the file; that
      includes a document on which only a `Run` that changed no cell was carried out (A151). So a
      document that was only read always closes; a file that was removed meanwhile is not
      brought back, and one that someone else changed is not complained about. (`save()`
      does both, A140: it is asked to make the file hold the document. `close()` is asked
      to lose nothing.)
    - A document that is dropped without having been closed, unsaved, is saved if its file
      is as it was or gone. If someone else changed the file, or it cannot be looked at,
      nothing is written: a drop has no one to ask and no one to tell. This is a safety
      net, not the way to save. It runs when Dart disposes of the handle or collects it, on
      the thread that does so, which it blocks for the time of the save (about 0.2 s for
      100,000 cells); it can report nothing; and when the system kills the app it does not
      run. The interface saves by calling `save` and `close`.
    - The generated function that drops a handle has no guard, and a panic that leaves an
      `extern "C"` function ends the process (A129). So the save in the drop is inside
      `catch_unwind`, the payload of a caught panic is forgotten and not dropped, and
      giving up the claim on the file cannot panic (A141).
    After `close()` the handle holds nothing; it is still Dart's to dispose of. `id()`,
    `root()`, `path()` and `is_saved()` can now fail, with `Closed`.
143. **A document that a panic may have left half changed is not used again** (from the
    evaluation after the review; see A129 for what a panic does). The open document notes
    that a call which changes it (`apply`, `undo`, `redo`) is under way, from before the
    core is asked until the report is made, and clears the note when the call ends, with a
    value or with an error (the core leaves the document as it was when it refuses). If a
    call finds the note set, an earlier one never ended: a panic unwound through it. Then
    every call, reads and `is_saved()` included, is refused with `Internal`; `save`,
    `overwrite` and `close` write nothing, and neither does the drop of the handle, which
    still gives up the file. So the file stays as it was last saved, and the way on is to
    dispose of the handle and open the file again; what was not saved is lost, which is
    better than saving half a command over what was whole. Calls that only read, and
    saving, set no note: they do not change the document, and a save that panics has not
    replaced the file (the new file gets its name in one last step).
144. **The app reaches a document through one Dart class that sends its calls in order** (from
    the review; the Dart part of A127). `Session` (`app/lib/session.dart`) holds the handle
    `OpenDocument` and is the only thing the app and its tests use. Each call starts when the
    one made before it has ended, well or badly, whether or not the caller waited: a chain of
    futures, on the Dart side because that is where the order of calls is known. An error goes
    to the caller of its own call and the calls after it are carried out all the same. (An error
    that no caller waited for was dropped; A154.) The review's test (40 insertions into one cell
    without waiting, 20 rounds) was wrong in 2 of 20 rounds on the bare handle when it was run
    again here, and is right through the class. What else the class decides:
    - `id`, `root` and `path` are read once when the document is opened and are plain
      values from then on: they never change, and so nothing has to wait for them.
    - `close()` saves, and on success disposes of the handle at once: nothing is left in
      it to save then, so letting go costs the thread that draws nothing (A142). After
      it every call throws a `BridgeError` of kind `Closed`, made on the Dart side, since
      the handle is gone; a call made while the close was still under way comes after it
      and is refused the same way.
    - `discard()` lets go of the handle without a close, for when a save was refused
      with `ChangedOnDisk` and the user chose the other program's file, and for after a
      fault. It is not a way to throw edits away: the bridge's safety net still saves
      what it may (A142).
    - `isFatal`, `describeError` and `faultSentence` are there too: after a panic
      (`PanicException`, then `Internal` on every call) the screen shows one fixed
      sentence and never the text of the exception (A129).
    - `codePoints(text, units)` turns a position in a Dart string, counted in UTF-16
      units, into the code points the core counts (A11), and refuses a position outside
      the text (A154). Nothing sends a position yet.
    - The class carries the comments a Dart developer needs, because the generator drops
      those of the handle's type and of `Command` (A137).
    `InTurn`, the few lines that do the ordering, is a class of its own because the grid
    screen's logic uses the same thing for its own steps (A146), and since the third review
    the keeper of the document and the screen's actions do too (A155, A158).
145. **The app saves the document by itself; there is no Save** (from the evaluation after the
    review; `UI_SPEC.md` has no Save, Close or Discard for documents). The simple grid screen
    had a menu entry "Save" and nothing else, so the back button, swiping the app away or the
    system ending it lost every edit. Now the document is saved without being asked. (As first
    built, by `Stage`, `app/lib/grid/stage.dart`; since A155 by `Keeper`, `app/lib/keeper.dart`,
    which owns the document for every screen on it. What follows is as it is now.) It is saved:
    - **one second after the first change that is not saved yet.** Later changes do not put
      the clock back, so steady typing cannot keep the file old; a change is in the file
      a second later at the latest, plus the time of the save, while the app runs and the
      save works (one that fails is tried again by itself, A155). Not
      after every command: on the 100,000-cell document of `measure.dart` (profile build,
      Linux desktop, measured again for this decision) a save takes 213 ms (worst of five
      302 ms) and an edit that no formula depends on 1.2 ms. Saving after each command
      would make such an edit about 170 times dearer, and every next edit would wait for
      the save before it, since calls go one at a time (A144).
    - **at once when the app is no longer in front**, because after that the system may end
      the app without another word. When it is out of sight (`AppLifecycleState` hidden,
      paused or detached), what is in the editor is entered into the cell first (the editor
      stays open), so that half a sentence is not lost either. When it is only `inactive` (a
      window that lost the focus, a dialog of the system), what is in the document is saved
      and the editor is left alone. As first built, every state but `resumed` entered the
      editor's text, from an observer in the grid screen; the third review showed what that
      did to the cross (A157). The observer is now where the app owns its document (A155). A
      save with nothing to save only looks at the file.
    - **when the screen goes** (`Keeper.close`, which is `Session.close` after what is in the
      editor was entered; A155). A save that fails is not swallowed: `Keeper.saveError` (which
      `Stage` passes on) holds it until a save works, and the screen shows it in a line that
      stays, with "Try again". For `ChangedOnDisk` (A140) the line offers the choice: "Keep
      mine" (`overwrite`) or "Open theirs" (let go of this document, open the file again, start
      at the root; what was not saved here is lost, which is the choice; since A159 only once
      that file is known to be readable). After a fault in the program (`PanicException`, or
      `Internal` on any call; A143) the keeper lets go of the document, nothing more is saved,
      every control that reaches the document is gone and the screen shows one fixed sentence.
      The bridge's own saving when a handle is dropped (A142) stays what it was, a safety net.
      Not done: nothing is written while the user types into the editor (typing is not bridged
      as commands yet, A127), so an app that is killed outright, without being moved out of
      sight first, loses what is in the editor and at most the last second of entered changes.
146. **The steps of the grid screen are taken in order, and none is dropped** (from the
    evaluation after the review). The screen dropped every action that came while another
    ran, a second tap on Undo as well as, before A145, Save. And nothing kept two steps of
    `Stage` apart: two taps on the way into a grid in one frame put the cell on the trail
    twice. `Stage` now takes its steps one at a time in the order they were asked for
    (`InTurn`, as `Session` does for calls), and judges each by the stage as it is when
    its turn comes: the second of those two taps finds the cell no longer on the stage and
    does nothing, two taps on Undo undo twice, and a cell tapped while an undo runs opens
    the editor on the text the undo left (`Stage.cell`). The screen has no "busy" flag
    any more. Also: going into a grid or up reads first and moves the trail after, so an
    error leaves the stage where it was; and an edit that the core made is not reported as
    failed when reading the stage again fails (`Stage.readError`, shown with "Try again").
    The third review found that "judges each by the stage as it is when its turn comes" was
    true of `Stage` and not of the screen, which still decided when the finger came down
    whether an editor was open, and that "none is dropped" did not hold with the editor
    open. A158 gives the screen's actions a queue of their own.
147. **Text in the editor is never thrown away by a tap elsewhere.** The simple grid screen
    closed the editor without a word when one went into a grid, up, along the trail, or
    tapped another cell. `UI_SPEC.md` section 6 says that a tap outside the editor only
    closes it and never dives or selects, and section 7 has a tick that closes the editor
    and no cancel, so closing means keeping. Hence:
    - a tap on a cell or on the way into a grid, while the editor is open, enters the text
      and closes the editor, and does nothing else (section 6);
    - the up arrow, the trail, Undo and Redo enter the text, close the editor and then do
      what they are for. The spec does not say; they are bars, not the grid, and doing
      nothing would make them need two taps. Undo therefore first undoes what was just
      entered. (If more was typed while the text was entered, the action is not done and
      the message line says so; A158.)
    - if the core refuses the text, the editor stays open with it and the action is not
      done;
    - if keys were typed while the core worked on Return or the tick, the editor stays
      where it is with those keys, and is not moved or closed over them;
    - the cross (cancel), which the spec does not have, stays for now: it is the one way
      to say "not this", and it is asked for by name. (After the app had been put away
      the text was in the document already and the cross kept it; A157 makes the cross
      take it back.)
148. **Where the app keeps its document is still temporary** (changes the last sentence of A130;
    **replaced by A165**: the document is in the app's own data folder on every system, found
    with `path_provider`. What follows is how it was). The app made a new folder in the system's
    temporary folder at every start and the bridge check at every press of Open, and removed
    none. Now the app has one document, `wombat/example.wombat` in the system's temporary
    folder: the copy of the example at the first start, and the same file, with everything done
    to it, at every later start, which is what makes saving mean something. The bridge check has
    one folder per visit of its screen and removes it when the screen goes. A place that the
    system does not clear needs the `path_provider` plugin (there is no way to ask Android or
    iOS for the app's own folder from Dart alone). A dry run of adding it (2.1.6) showed 15 new
    packages in `pubspec.lock`, among them `jni` and build hooks that compile native code for
    Android, and registration code generated into `android/` and `linux/`; the Android build
    could not be run on the machine this was written on (no SDK). It was not added; it belongs
    with the decision where documents live (also Android's picked documents, which are not
    paths). Until then the system may clear the folder: on Linux at a restart, and on Android,
    where it should be the app's cache folder (not tried), when storage is short. If it does so
    under an open document, the next save writes the file again when the folder is still there
    (A140) and is shown as failed when it is not.
149. **An open document knows its file by the path from the root that it had when it was
    opened** (from the second review, R1; adds to A140 and A141). `OpenDocument` kept the
    path as it was given and worked it out again at every save, while the claim on the file
    (A141) was made absolute once. After the program's working folder changed, a document
    opened as `rel.wombat` saved to a new `rel.wombat` in the new folder and answered
    success; the first file kept its old content, and the new one could be opened beside the
    document that had just written it. Now the path is made absolute when the document is
    opened (`std::path::absolute`: against the working folder of that moment, `.` taken
    out, `..` and links left as they are) and that one path is what is read, saved to,
    checked and answered by `path()`. Links are deliberately not followed for it: saving
    through a link goes where the link leads at the time of the save, which is the storage
    layer's rule (`write_atomic`). A path that cannot be made absolute (the empty path) is an
    `Io` error with the path as asked. The test changes the working folder, which belongs to
    the whole process, so it is alone in its test file (`tests/working_folder.rs`).
150. **A claim holds every name the document's file has** (from the second review, R3, and a
    case the Dart side reported; changes A141). A claim held one name: where links lead. But
    an open document is two things that can come apart: the file it read, and the name it
    saves to. A document opened through a link `L` to `T` held `T`; when `L` was removed,
    `create(L)` succeeded while the first document went on saving to `L`. A claim now holds
    up to three names, and a file is open if any name of the newcomer is held by a document:
    1. the real name of the file that a save would replace (links followed, no `.` or `..`),
       if there is such a file: a second open through another link, or of the file itself,
       meets this one;
    2. the name below its real folder, without following a link that the name itself is. If
       the folder is not there (it was removed under the open document and may be made
       again), it is the name below the nearest folder above that is there;
    3. the path as asked, from the root, which stays when a link on the way to the folder is
       changed.

    So nothing is opened or made at the name of an open document when its link was removed
    or made to lead elsewhere, when its file is gone, or when its folder is gone or was
    removed and made again, however the path is spelt. Opening the link's target directly,
    or through a second link, is refused as before. All names are given up together when the
    document is closed or dropped; checking and claiming are still one step under one lock.
    Still not seen to be the same file: a second hard link, and for a file that is not there
    yet a name that differs only in a way the file system ignores. Still possible: a link is
    made to lead to a file that a second document already has open by its real name;
    `overwrite()` on the first then writes into the second's file (`save()` refuses with
    `ChangedOnDisk`).
151. **Only a change to the document makes it unsaved** (from the second review, R4; changes
    the last sentences of A140). The flag "saved" was cleared by every report that was not
    empty, and a report also lists the formula cells that show something else
    (`recalculated`). Program > Run on a saved document whose program changes no cell gives
    a report with only that list: nothing joins the undo history and a save would write the
    same bytes, but the document counted as unsaved, and `close()` then answered
    `ChangedOnDisk` if another program had changed the file, although a document with
    nothing unsaved always closes (A142). The flag now follows the core's `Changes`
    (changed, created, removed, grids, tags): what formula cells show is worked out from the
    document and is in no file.
152. **Only the form of JSON that is written is read** (from the second review, R5; replaces
    the exception in A136). `serde` reads more forms than it writes. Besides `{"Run": null}`
    for `"Run"`, which A136 allowed, it reads any object from a list of its values in order:
    `{"InsertText": ["<id>", 1, "x"]}`, a colour as `[1, 2, 3]`, a report as six lists. A
    list has no names, so a value in the wrong place is read as another field, which is what
    strict reading is there to prevent. `record.rs` now reads the text a second time as JSON
    of any kind and compares it with what is written for the value it read; text in any
    other form is a `BadRequest`. What may differ is what JSON itself takes for the same:
    spaces and line ends, the order of the fields of an object, and a whole number where a
    fraction may stand (`1` for `1.0`). `{"Run": null}` is refused with the rest. The other
    way, a deserializer that hands `serde` only maps for objects, has to wrap the
    deserializer, the visitor, the seed and the accesses to lists, maps, enums and variants
    for the rule to reach nested values: an estimated 180 lines of forwarding (it was not
    written) against 35 here. The price is time: a text is read twice and written once,
    about three times the cost of reading, which a record that is played back can afford.
    Nothing of this touches what crosses between Dart and Rust.
153. **A list that is no table is a bad request, never `Invalid`** (from the second review,
    R8; changes the end of A135 and completes A133). A malformed table got two kinds: a grid
    short of cells or a number out of range was `BadRequest`; no cells, no columns and rows
    that are not whole were `Invalid`, the core's word for a file that breaks a rule of the
    format. A caller could not read `Invalid` as "the file is broken", which is what A133
    made `BadRequest` for. `convert.rs` now checks the shape of every grid of the flat list,
    the outermost and each one inside, before the core is asked: at least one column, at
    least one cell, only whole rows, and as many cells as it says. What is left for the core
    to refuse is a table past a limit of documents, and that stays `Limit`: nested too deep,
    too many cells, a text too long. This is a deliberate change of behaviour: lists that
    were answered `Invalid` are answered `BadRequest`.

    Also from the second review, about CI (R12): the check that the generated bridge code is
    fresh removes the generated Dart folder (`app/lib/src/rust`) and `frb_generated.rs`
    before it runs the generator, so that a file the generator writes no more shows as
    deleted; run over the files in place, such a file was neither changed nor new and
    stayed unseen. The generator runs without them (tried with 2.13.0: no difference on the
    committed code; a stale file committed under `app/lib/src/rust/api/` passes the old
    check and fails the new one). Still as it was: the bridge is never built for iOS, and
    for Android only with rustup's `stable` (A131); actions are pinned by tag, not by commit.
154. **The queue of calls loses no error and does not wait for itself** (from the third
    review, of the Dart side; adds to A144; **changed by A160**, which says what "inside a
    piece" is, sets timers apart and sees two queues that wait for each other). `InTurn` chained every piece of work to the one
    before, and so that one failure would not end the chain, the chain took every error;
    for Dart that counted as the error having been handled, so the error of a call that
    nobody awaited vanished. And a piece that awaited another piece of its own queue waited
    for ever, since that one comes after it. Now:
    - The answer that `run` gives knows whether anyone has taken it (`await`, `then`,
      `catchError`, `whenComplete`, `timeout`, `asStream`). If someone has when the piece
      fails, the error goes to that caller alone. If no one has, it goes once to the
      handler the owner of the queue gave (`onUnheard`; the grid screen shows it on its
      message line), or without one to the zone as an error nobody caught, which is where
      Dart sends such errors and what fails a test. "At the moment it fails" is the rule
      Dart itself has for futures: someone who takes the answer only afterwards finds the
      error in it as well.
    - Each piece runs in a zone that carries a mark of its own, and the queue remembers the
      mark of the piece that runs. `run` called from inside that piece fails at once with a
      `StateError` that says so, and runs nothing; waited for or not, since the two cannot
      be told apart. Something a piece left behind (a timer, a listener) that runs after it
      may ask the queue again. The mark is taken off just after
      the piece has ended (in the next microtask), so in that instant such a call is refused
      too; nothing in the app does that.
    - The chain is made of plain callbacks. With `whenComplete` or an `async` closure in
      it, 16,000 queued pieces took 90 seconds in a test (each one a little longer than
      the one before); as it is they take about a quarter of a second.

    Three smaller things from the same review. `Session.over` lets go of the handle when
    the handle cannot say its ID, root or path, instead of leaving a file taken by
    something nobody holds. `codePoints` refuses a position below 0 or beyond the text with
    a `RangeError`, as the core refuses a position and never moves it (A11); that includes
    -1, which a `TextSelection` gives when there is no selection, so a caller asks
    `selection.isValid` first. Clamping was the other choice and would have put text at the
    start of a cell for a selection that is none. And the Dart tests ask cargo where it
    built the library they load (`--message-format=json`) instead of working the folder
    out, which was wrong for a relative `CARGO_TARGET_DIR` and silent for other settings.
155. **Saving belongs to what owns the document, not to a screen** (the coordinator's
    decision after the third review; changes A145 and A146; **changed by A161**: `addFinisher`
    is now `attach`, putting away saves first and waits no longer than two seconds, a close
    that cannot finish leaves the document open, and listeners are told apart from the
    keeper's own work; and by A162, which makes `endKeeping` run when the window is closed).
    The clock that saves after a
    change was in `Stage`, the watching of the app's lifecycle in the grid screen's widget,
    and "ended after a fault" a state of each `Stage`: a second screen on the same document
    would have had none of it, or a second copy of all of it. `Keeper`
    (`app/lib/keeper.dart`) now owns the `Session` for every screen on the document:
    - Screens read and change the document through it (`grid`, `apply`, `undo`, `redo`),
      one call at a time (`InTurn`). A change starts the clock (A145); a fault ends the
      work for everyone (`ended`); `saveError`, `ended` and `opens` are what a screen
      shows, and it is told when they change. `Stage` holds no `Session` any more, only
      passes those on, and asks (`save`, `keepMine`, `openTheirs`).
    - **A save that failed is tried again by itself** (the review's finding: after one
      failure nothing was tried until the next change or the user). The wait starts at the
      delay of saving, one second, and doubles with every failure up to a minute: 1, 2, 4,
      8, 16 and 32 seconds after the failure before, then every 60. A save that works puts
      it back to one second. Not tried again: `ChangedOnDisk`, which only the user can
      answer. It stops when the document is closed, let go of or ended. While a save is
      waiting to be tried again, a new change does not bring it forward.
    - **The app's lifecycle is watched where the app owns its document**: `Kept` in `main.dart`,
      a widget around the screen. `inactive` calls `Keeper.save`; `hidden`, `paused` and
      `detached` call `Keeper.putAway`, which first lets everything that works on the document
      finish what it has begun and then saves. What "finish" means each says itself
      (`addFinisher`): a stage waits for the steps asked of it, the grid screen enters what is
      in its editor (A157). A screen that goes away with text in its editor enters it as one
      more step of its stage, which the keeper waits for like the others. If something cannot
      finish, the document is saved all the same.
    - **A close that cannot save leaves the document open** (changes A145's "when the
      screen goes"). `Stage.close` let go of the document when the save failed, which made
      the bridge try the same save again on the thread that draws (A142) and told no
      screen. `Keeper.close` now throws, keeps the document in use, puts the error in
      `saveError` and tries again like any save that failed, so that a screen can say so.
      Only an owner with no screen left calls `letGo` (`endKeeping` in `main.dart`, when
      the app's last screen goes), which is the bridge's safety net and never writes over
      another program's work.
    - The keeper does not close itself and `Stage` does not close it: whoever made it does.
    Four queues are now one inside the other (the screen's actions, the stage's steps, the
    keeper's calls, the session's calls). Each asks only the one below it, and the guard
    of A154 turns a mistake in that order into an error instead of a wait without end.
    The price of the move: something that calls `Keeper.save` directly is not ordered after
    a stage's steps that have not reached the keeper yet; `putAway` and `close` are, through
    the finishers, and the save by the clock need not be.
156. **On desktops the document is in the user's data folder; a document that cannot be
    opened can be put aside** (the coordinator's decision after the third review; changes
    A148; **where the folder is was replaced by A165, and what is put aside and when by
    A163**. What follows is how it was). `wombat/` in the system's temporary folder is `/tmp/wombat` on Linux: one folder
    for every user of the machine, which the system clears. Without a new dependency,
    `documentFolder` in `app/lib/main.dart` works the folder out from the environment:
    - Linux: `$XDG_DATA_HOME/wombat` if that is set and starts at the root (the
      specification says to pass over one that does not), else `$HOME/.local/share/wombat`;
    - macOS: `$HOME/Library/Application Support/wombat`;
    - Windows: `%APPDATA%\wombat`;
    - Android, iOS, anything else, and a desktop whose environment does not say:
      `wombat` in `Directory.systemTemp`. On a phone that is the app's own cache folder,
      **which the system may clear** when storage is short; a real documents folder there
      needs the `path_provider` plugin or a picker (A148 says what adding it brings), and
      is still to decide.

    The function takes the system's name, the environment and the temporary folder as
    arguments, and a test goes through every branch; the macOS and Windows branches have
    never run on those systems. A document that an earlier build left in `/tmp/wombat` on a
    desktop is not moved: there is none anyone keeps.
    The app has one fixed file, so a file there that cannot be read (a save cut short by
    a crash of the machine, another program's file) showed the same error at every start
    and offered nothing. The start screen now says that the document could not be opened,
    names the file, and has two buttons: "Try again", and "Put that file aside and start
    with the example", which gives the file the name `example-put-aside-<time>.wombat` in
    the same folder and opens a new copy of the example. Nothing is deleted. The second is
    not offered when the file is merely open already (`AlreadyOpen`), which is not the file's
    doing, or when there is no file to put aside (the folder could not be made). It is
    offered after a fault while opening: a file that makes the core fail is exactly what to
    put aside.
157. **Text that went into the document because the app was put away is not yet said yes
    to** (the coordinator's decision after the third review; changes A147). When the app
    goes out of sight, what is in the editor is entered so that it is in the file (A145).
    The user has not pressed the tick, though, and after coming back the cross closed the
    editor and left the text in the cell, and the tick made a second step of the undo
    history out of one editing. The bridge has no groups of commands yet (A127), so with
    what there is:
    - The screen remembers the text that went in that way (only if it changed the cell).
    - **The cross undoes it.** That entering is the latest command, so one undo puts the
      cell, and the undo history, back as they were before the editor opened.
    - **The next entering takes its place**: the tick, Return, a tap elsewhere, an action
      of the bars, or the app put away once more first undo that command and then enter
      the text as it is now (`Stage.enterText` with `insteadOfLast`), so one editing is one
      step whatever happened in the middle. If the text is the one that went in, nothing
      is sent. If it is the text the cell had before, the undo is all, and no step is left.
      If the core refuses the new text, the undone command is redone and the editor stays.
    - Only `hidden`, `paused` and `detached` enter the text; `inactive` does not (A155), so
      a window that loses the focus leaves nothing to take back.

    What this cannot do, and what remains:
    - After the cross, Redo brings the cancelled text back: the entering is in the redo
      history. And whatever could be redone before the app was put away cannot any more:
      the entering emptied the redo history, as every command does.
    - It rests on that entering being the latest command when the cross or the tick comes.
      That holds while one screen works on the document, since every other action of the
      screen closes the editor first. A second screen that changed the document in between
      would have its command undone instead. Entering as one group of commands, which the
      core has (`begin_group`, `end_group`) and the bridge does not carry, would remove
      both this and the point above.
    - Between the undo and the new entering the document holds the old text; a save by the
      clock that falls exactly there writes that, and the next save, a second later,
      writes the new text.
    - An app that is ended while it is only `inactive` loses what is in the editor, as an
      app that is ended outright does (A145).
158. **Every action on the grid screen goes through one queue** (the coordinator's decision
    after the third review; completes A146). `Stage` took its steps in order, but the
    screen read "is an editor open?" when the finger came down and acted on it when the
    step before had ended. A cell tapped and then "Open ›", the up arrow or the trail,
    while any step was still at work (the document's own save is enough), opened the editor
    and then moved the stage away from under it; the second of two taps on Undo with the
    editor open was dropped, and so was an action when more had been typed while the
    editor's text was entered. Now a tap on a cell or on "Open ›", up, the trail, Undo,
    Redo, Return, the tick, the cross, the buttons of the line that says what is wrong, the
    system's back and the app being put away each become one action in one queue
    (`InTurn`), and an action looks at the editor and the stage when its turn comes. The
    three sequences of the review, and the same with up and the trail, now end as if the
    taps had come one after the other: the editor opens, and the tap after it, being a tap
    outside the editor, closes it (A147).
    - **No action is dropped without a word.** It is done, or the message line says why
      not: what the core refused; "Not done: more was typed in the meantime. It is still in
      the editor." when an action of the bars could not close the editor because the user
      typed on while its text was entered (what was sent is in the document); and an error
      nobody expected, through the queue's handler (A154). After a fault the sentence that
      takes the place of the grid is the answer to every action still in the queue.
    - **One thing is done without a word, on purpose:** a tap on a cell or on "Open ›"
      whose cell is no longer on the stage when its turn comes does nothing. That is the
      second of two taps on "Open ›", and a cell tapped while the way up was still being
      read. The stage has visibly moved; a message would be noise after every double tap,
      and opening whatever is at that place now would act on a cell the user never saw.
    - The message line is cleared when the user acts, not when the action's turn comes, so
      what one action said is not wiped by the next one in the queue before it was seen.
    - **The system's back is "up one level"** (`UI_SPEC.md` section 10; it was not handled,
      so Android's back left the app from any level). The screen never lets the route pop
      by itself (`PopScope`); back is the same action as the arrow, what is typed is kept
      first, and at the root the action asks the system to leave the app
      (`SystemNavigator.pop`). Deciding in the action and not when the frame was built is
      what keeps two quick backs from leaving the app from a level that was only about to
      be left. The price: Android's animation that shows where back leads is off for this
      screen. After a fault back is the system's again.
    - **Return keeps the keyboard.** The editor's action is "next", and Flutter's answer to
      that is to move the focus to the next widget unless the field says what Return does
      (`onEditingComplete`). The focus only came back when the editor moved to the cell
      below; when the core refused the text, the editor stayed open without it. The test
      that was to show keys typed while Return is at work passed because
      `tester.enterText` gives the field the focus first; the screen's tests now type into
      whatever has the keyboard, as a user does.
    - The screen takes another stage when it is given one (`didUpdateWidget`), and enters
      what is in its editor when it goes (`dispose`); neither was handled.
159. **"Open theirs" looks before it lets go** (the coordinator's decision after the third
    review; changes A145; **A161 adds** that what is attached finishes first and that the
    choice is refused when no other program changed the file). The choice let go of this document and then opened the file. If
    the other program's file could not be read, the unsaved work was gone, nothing was
    open, and since the app has one fixed file every later start failed too. The bridge
    opens a file once (A141), so the file cannot be opened beside the document that holds
    it; but a copy only reads what it copies from. `readable` in `app/lib/keeper.dart` asks
    the core for a copy of the file in a folder of its own in the system's temporary
    folder, closes it and removes the folder: if that works, the file can be read. Only
    then is this document let go of and the file opened. If it does not work, nothing
    changed: the document is open and unsaved, the line still offers "Keep mine" and "Open
    theirs", and the message says that the other file cannot be opened and why.
    The copy costs reading and writing the document once (and its pictures). A gap is
    left: the file can be changed again between the look and the opening. If it then
    cannot be read, no document is open; the screen says so, and that at the next start
    the app offers to put the file aside (A156). Closing the gap needs a call that opens a
    file in place of the document a handle holds, or one that reads a file without opening
    it, in the bridge.
160. **A piece of a queue is what it does and waits for; timers are apart; two queues that
    wait for each other are refused** (from the fourth review, G3, G8 and G10; changes A154).
    The guard of A154 marked a piece by its zone, and a zone goes with everything the piece
    starts. So a timer made in a piece (the document's save clock, made by the step that
    changed the document) was refused when it fired while that piece still ran, although
    nothing waited for it. And a piece of queue A that waited for queue B, while a piece of B
    waited for A, was not seen: both hung. Now, in `InTurn` (`app/lib/session.dart`):
    - **Each piece knows which queues it has asked and not heard from.** `run` called from
      inside a piece is refused, with a `StateError` that says which of the two it is, when
      the piece that runs now in the queue asked is the caller itself, or has (through any
      number of queues) asked the caller's queue for something that has not ended. That is
      exact for the wait it can see and costs a walk over a handful of queues.
    - **Dart cannot tell `await f()` from `unawaited(f())`**: both are a call that returns a
      future. So everything a piece starts is inside it, waited for or not, and a call that
      would wait for ever if it were waited for is refused also when it is not. That rule
      cannot hang; what it costs is a refusal that says so, of work that would have been
      safe. Two things are set apart, because they are known not to be waited for by the
      piece: **the callbacks of timers** the piece makes (the zone of a piece hands a timer's
      callback on to a zone without the mark), and what is run through **`InTurn.apart`**,
      which is for telling listeners and the like. The price: a wait for ever that goes
      through a timer's callback (`Future.delayed(d, () => queue.run(…))`, awaited) is not
      seen. Nothing in the app does that.
    - **`join`** runs work in its turn, or at once when the caller is inside the piece whose
      turn it is: for work that belongs in a queue and may be asked for by a piece of that
      queue (A161 uses it).
    - **`post`** runs work for no one and gives no answer. `run` answers with a future that
      knows whether anyone took it (A154); an `async` function that returns that answer has
      taken it, and if nobody listens to *that* function, the error goes to the zone and not
      to `onUnheard`. That is Dart's rule for every `async` function and no queue can see
      past it; the comment of `InTurn` now says exactly what "heard" means. `post` is the way
      to do something for no one (a user's action): there is no answer to pass on, so an
      error, the refusal above included, always goes to `onUnheard`.
161. **The keeper says how the document stands, bounds what it waits for, and its rules are in
    the code** (from the fourth review, G2 to G4, G7, G9, G11 and what it read; changes A155
    and A159). `Keeper` (`app/lib/keeper.dart`) is what the next screens are built on, and a
    screen had to know its insides to be safe: not to ask it from a listener, not to close
    from its own queue, to register a finisher. Now:
    - **`state`**: `saved`, `unsaved` (a change the clock has not written yet), `saving`,
      `failed` (tried again by itself), `changedOnDisk` (the user chooses) or `ended`. What
      is wrong outranks what is under way, so a save that is tried again shows as `failed`
      until one works. A command that changed nothing, by its report, leaves the document
      `saved` and starts no clock (what formula cells show does not count, A151). Leaving is
      safe when the state is `saved`.
    - **Listeners are told once of each change** of `state` or `saveError` (`ended` and
      `opens` change only when `state` does), and never of the same twice: two errors are
      the same when their kind and message are, which is how the core's errors compare
      (the core makes a new object for every failure; the keeper asked whether it was the
      same object, the stand-in for the core threw one constant, and so the tests did not
      see that the real app was told at every try). They are told apart from the keeper's own
      piece (`InTurn.apart`, A160), when its work is done and when a write begins, so a
      listener may ask the keeper at once.
    - **`attach`** takes the place of `addFinisher` and `addListener` for a screen's logic:
      what to tell (`onChange`), how to finish what has been begun (`finish`), and the
      queue that orders the screen's work (`queue`), with one `Attachment` to `detach`. The
      keeper runs `finish` in the turn of that queue, or as part of the piece when it is
      asked from inside that queue. So a "close" action in a screen's own queue of actions
      works, where it used to close without the editor's text and then throw.
    - **Putting away saves first.** `putAway` saves what is in the document, then lets
      everything attached finish, all at once and each within **two seconds**, then saves
      again if that changed anything. It answers with what kept anything from finishing (a
      `TimeoutException` for one that took too long) and does not throw for it. Two seconds:
      a finisher is a few calls to the core, milliseconds each, and a phone gives an app
      that is put away only a few seconds in all.
    - **`close` throws exactly when the document was not closed.** If something attached
      cannot finish, `close` saves what is in the document, leaves it open and throws
      `Unfinished` with the causes; if the document cannot be saved, as before (A155). It
      is never closed and then thrown about. An owner with no one to ask calls `letGo`, as
      `endKeeping` does. This was chosen over closing regardless: text that could not be
      entered is still on a screen, and closing under it would lose it without a word.
    - **`openTheirs`** lets what is attached finish first, so that a step begun on this
      document does not go on in the other program's (`Unfinished`, and nothing changed,
      if it cannot). And it is refused (`Refused`) unless the state is `changedOnDisk`:
      letting go of a document whose file is as it left it makes the bridge save it (A142),
      which wrote this document over the file and then opened that. **What the bridge
      lacks**: a call that gives a document up without saving, or better one that reads the
      file again into the handle (which would also close the gap of A159). With it the
      check could go. What is left: a file that goes back to exactly what this document
      last read between the failed save and the choice would still be written; and a step
      that a screen asks for after the choice was made lands, call by call, on whichever
      document is open then, which `opens` tells it.
    - **After the end every call fails the same way**, with a `BridgeError` of kind `Closed`
      ("the document is no longer in use"): reads, changes, `save`, `keepMine`, `putAway`,
      `openTheirs`, `close` and `letGo`, also one that was made before the end and whose
      turn came after. Six of them used to return as if they had worked. `endKeeping`
      (now in `keeper.dart`) is the one call that is safe on a keeper that may have ended.
    - `readable` gives the copy's handle back also when closing the copy fails, and a
      failure to make the folder for the copy is a `FileSystemException`, which the
      stand-in screen no longer reports as the other program's file being unreadable.
      `faultSentence` now says "Its file holds what was last saved, or more": after a
      fault in a call that only reads or saves, the document is whole and letting go of it
      saves it.
162. **Closing the window waits for the document** (from the fourth review, G1; adds to A145
    and A155). The app saved when it was put away and when its screen went, and on Linux
    neither happens when the window is closed: the process ends. Typing, Return and closing
    the window at once lost the text, four times of four. `Kept` in `app/lib/main.dart` now
    answers the system's request to exit (`WidgetsBindingObserver.didRequestAppExit`):
    - It puts the document away (A161: saved, what is typed entered, saved) and closes it
      (`endKeeping`), and answers "exit" when that is done.
    - **At most five seconds**, after which it answers "exit" all the same, so that a save
      that hangs cannot make the app impossible to close; a line on the standard output
      says that whether the document was saved is not known. Five seconds: a save of
      100,000 cells takes about 0.2 s on a desktop and 0.65 s on the emulator, the
      finishers have two seconds, and a window that does not close for longer looks hung.
    - **If the document is then not in its file** (another program changed the file, the
      save failed, a fault), or what was typed could not be entered, the exit is held back
      **once**: the window stays, and the screen says what is wrong. Asked again while
      nothing is better, the app lets go of the document (which never writes over another
      program's work) and exits. After a save that worked the next refusal is the first
      again.
    - Which systems ask: Linux, seen (below); macOS, by Flutter's documentation; Windows,
      the documentation of Flutter 3.47 names only macOS and Linux, and it was not tried.
      Android and iOS do not ask: there the app is put away first (A145). An app that is
      killed is not asked anywhere.
    - Seen on the Linux debug build under Xvfb, with the reviewer's driver: a cell clicked,
      a word typed, with and without Return, the window closed 0, 200, 300 and 500 ms
      later: seven of seven runs exit with the word in the file. With the file changed by
      another program in between, the first close leaves the window with "Not saved:
      another program changed the file", the second exits, and the file is the other
      program's.
163. **Whatever is in the document's place is the document; what is put aside takes its
    pictures and view states along, into a folder that is new** (from the fourth review,
    G5, G6 and what it read; changes A156).
    - The start looked for a *file* of the document's name. A folder there, or a link that
      leads nowhere, was "no file", so the app tried to copy the example to that name, got
      "already exists" and offered no way out. It now takes whatever has the name for the
      document, opens it, and offers to put it aside when that fails.
    - "Put that file aside" renamed the file and left its image folder
      (`example.wombat.images`, `docs/FILE_FORMAT.md`) under the name of the new document,
      and the name it gave came from the clock, which `rename` would have let replace an
      earlier one. `putAside` now makes a new folder `put-aside-<date>-<random>` in the
      document's folder with the system's call for a folder that did not exist
      (`createTemp`), and moves into it, under their own names, the document, its image
      folder and the folder of view states (`.wombat-view`, which is this one document's,
      since the app keeps no other there). Each is moved as what it is: a link is moved,
      not followed. Nothing is deleted or replaced, also for two calls in the same instant.
      View states are named by the document's ID, which a file that cannot be read does not
      give, which is why the whole folder goes.
    - Putting aside is not offered when the file is merely open already (A156) or when it
      was made by another start of the app between this one's look and its copy of the
      example ("already exists"): that file is whole and someone's. "Try again" then opens
      it. (Two running copies of the app on one document are still not kept apart: each
      process has its own set of open files, A141. The second one to save is told that
      another program changed the file.)
    - `WombatApp` and `StartScreen` are given the screen to show on the keeper
      (`screen`), so that what the app does at its start and end is tested without the
      stand-in grid screen, on the least screen there can be (`test/probe_screen.dart`).
164. **The application ID is `io.github.sblumenf.burrow`; the name people see is Wombat** (the
    owner's decision). It replaces the template's `com.example.wombat_app` in the Android
    namespace and application ID, the package of `MainActivity` and its folder, and
    `APPLICATION_ID` of the Linux build, which is also what GTK knows the app by and what
    names the folder of A165 on Linux. The Android label and the title of the Linux window
    said `wombat_app` and say "Wombat". Not changed: the Dart package and the name of the
    Linux executable (`wombat_app`), which nobody sees and every import and script uses;
    signing (the release build is still signed with the debug key); `allowBackup`. iOS,
    macOS and Windows have no project folder yet.
165. **Documents are in the app's own data folder, on every system by one rule** (the owner's
    decision; replaces A148 and the folder of A156). The folder is `documents` in what
    `path_provider`'s `getApplicationSupportDirectory` gives (`documentFolder` in
    `app/lib/main.dart`, with the asking passed in so that tests give their own):
    - Android: `/data/user/0/io.github.sblumenf.burrow/files/documents` (the app's
      `files` folder, by the plugin's code; not run here). Before, it was
      the system's temporary folder, which on Android is the app's `code_cache`, and
      Android empties that at every update of the app: seen on the emulator in CI, where
      installing the package again removed the file.
    - Linux: `$XDG_DATA_HOME/io.github.sblumenf.burrow/documents`, without that variable
      `~/.local/share/io.github.sblumenf.burrow/documents` (seen: the real-app runs of A162
      wrote there). The plugin follows the same XDG rule the app's own code did; the
      folder is now named by the application ID and not `wombat`.
    - iOS and macOS: `Library/Application Support` of the app; Windows: the app's folder
      under the roaming application data. By the plugin's documentation; never run.
    - Why "application support" and not "documents": on desktops the plugin's documents
      folder is the user's `Documents`, which is not the app's own; application support is
      private to the app on a phone, is not cleared by the system and stays over updates.
      The price on a phone: the user cannot see the file from another app. Choosing,
      making and sharing documents is still to be designed.
    - **No second place.** The app's own rule fell back to the temporary folder when the
      environment did not say (a Linux session without `HOME` got `/tmp/wombat`), and took
      a relative `HOME` as it was. Now a path that is not from the root is refused, and
      when the system cannot say, the start screen says so and offers to try again.
    - Nothing is moved from where earlier builds kept the document: no installs exist.
    - **The cost of the plugin.** `path_provider` 2.1.6 with its five platform packages,
      and through them `ffi`, `xdg_directories`, `objective_c`, `hooks`, `code_assets` and
      `record_use`: twelve packages in `pubspec.lock`. Its Android part is **held below
      2.3.0** (`path_provider_android: ">=2.2.23 <2.3.0"`): 2.3.0 was rewritten on JNI and
      brings three more packages and a native library (`libdartjni.so`) that is then also
      compiled into the Linux and Windows builds, which need a JDK for it and use none of
      it (seen: the Linux bundle gained the library). 2.2.23 talks to Android over a
      channel, as the plugin always did. This has to be looked at again when the plugin
      stops serving the older part. On Linux the plugin is Dart alone: nothing new is
      compiled, and no generated file in the repository changed. The Android build with
      the plugin was not run here (no SDK); CI's two Android jobs are what shows it.
    - The app's tests give `WombatApp` a folder of their own; none writes where a user's
      documents are. The measuring (`integration_test/measure_test.dart`) keeps its large
      document in the same data folder on Android and iOS, so that a second run finds what
      the first built, and in the temporary folder on a desktop, so that a developer's
      measuring leaves nothing among the installed app's data. The bridge check
      (`demo_screen.dart`) stays in the temporary folder: its folder lasts one visit of
      the screen and is removed with it.

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

Numbers 66 to 69 are not used. 70 to 79 are from the review of milestones 4 and 5.

70. **Tags go with cells to another document** (A88). TreeSheets keeps tags per document
    and has no rule for pasting between documents: a pasted cell is a tag only if the
    document it arrives in has a tag of its text. Wombat adds the tags that the copied or
    moved cells have to the other document, with their colours, if it has none of that
    name, so that a tagged cell stays tagged.
71. **Limits on calculating, tightened** (A80 to A82; changes B52 and B53). A text that a
    formula makes is at most 1 MiB; one recalculation is at most 2,000,000 units of work,
    of which every formula takes at least 6, so a document cannot have more than about
    330,000 formula cells that are worked out; a run holds at most twice a
    document in views and variables. TreeSheets has none of these and would run out of
    memory or time.

80. **Exported CSV is made for spreadsheets, not for TreeSheets' bytes** (milestone 7;
    numbers B66 to B69 are not used; B70 to B79 are from the review of milestones 4 and 5). TreeSheets quotes every field, ends rows with
    the system's line end, writes no byte-order mark and exports text as it is. Wombat by
    default quotes where needed, writes CR LF and the mark (A97), and guards text that a
    spreadsheet would run (A98). `--treesheets` gives the bytes TreeSheets writes on
    Linux and macOS: all 38 examples and the 10 cases of the referee compare equal.
81. **Where TreeSheets' TSV is not valid delimited text, Wombat's is.** TreeSheets
    quotes a field of its TSV only if it starts with a quote or holds a tab, so `say
    "hi"` and `a"b` are written bare (a quote in a field that is not quoted), and a row
    that is one empty cell is an empty line, which no reader takes for a row, its own
    included. Wombat quotes any field that holds a quote and writes such a row as `""`.
    Three of the ten cases differ for these two reasons and no other. (TreeSheets' CSV
    is valid, apart from the line end being LF on Linux.)
82. **Two things are read differently when importing delimited text.** A delimiter at
    the end of a line is followed by an empty field, as RFC 4180 says; TreeSheets drops
    it, so that a table whose last column is empty comes back one column short. And
    text between a closing quote and the next delimiter stays with the field
    (`"abc"x,def` is `abcx` and `def`); TreeSheets drops one character there, whatever
    it is, and so reads `abc`, an empty field and `def`. In the other 23 of the 25
    files compared the result is the same. A file with no line at all is refused;
    TreeSheets silently does nothing.
83. **An import can go into a cell, and then replaces its grid.** TreeSheets' import
    always makes a new document. Its paste of text does go into the selected cell, and
    if that cell has no text merges the new grid into the grid around it; Wombat's
    `Import` never changes anything outside the cell it is given.
84. **Imports are bounded** (A102): TreeSheets reads a file of any size into memory,
    nests indented text up to 500 deep, and has no limit on cells. Wombat stops at 32
    MiB, 700,000 cells, 16 million characters per cell and 32 levels.
85. **Flattening for export goes by rows and changes nothing.** TreeSheets' Flatten
    (and `Command::Flatten`) changes the document and makes one row per innermost
    *cell*, so a table inside a cell comes out with one row per cell of the table. The
    flattened export makes one line per innermost *row* and keeps the columns of a table
    side by side (A99); for grids of one column the two are the same.
86. **A file not in UTF-8 is read as Windows-1252, or as UTF-16 if it says so**
    (A102). TreeSheets leaves it to wxWidgets' guess, which depends on the system.
90. **`tag:` narrows to rows** (A109; numbers B87 to B89 are not used). TreeSheets'
    "filter by tags" shows the cells that are a tag, and its "show entire row" setting
    draws their rows. In a search a result is a cell, so the setting has to become
    part of the query: `tag:todo` is the cells of the rows that have the tag, and
    `is-tag:todo` is exactly what TreeSheets' filter marks.
95. **A file that is not UTF-8 is refused unless its encoding is said** (A115; changes
    B86; the numbers B87 to B89 and B91 to B94 are not used). TreeSheets leaves the encoding to
    wxWidgets' guess; Wombat guessed Windows-1252 and now guesses nothing.
96. **A `.csv` file's delimiter is found in the file** (A120). TreeSheets has one menu
    item per delimiter and reads what it is told; so does Wombat when told
    (`--delimiter`, `--format semicolon`, `.tsv`).
97. **An import into a cell replaces content only when asked** (A119; changes B83).
98. **Other separators are quoted too, and the guard is narrower** (A113, A114; adds
    to B80). TreeSheets quotes every field of CSV and guards nothing; `--treesheets`
    still gives its bytes.
99. **The flattened export may end its lines with parent columns** (A118; adds to
    B85). TreeSheets' Flatten has nothing like it; where every grid has one column they
    never appear unasked, and the export equals Flatten as before.
