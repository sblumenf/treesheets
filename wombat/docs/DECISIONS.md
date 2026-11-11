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
    with exactly one such cell (never the root, the linking cell or a formula cell,
    B15) and leaves the others. TreeSheets' "nearest cell" rule is applied once, at
    import (B1), and not again: on a document that has since been edited, "nearest"
    would be a guess. Any `SetLink` on the range converts the link by hand.

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
15. **At import, a formula cell is never the target of a link.** In TreeSheets the text of a
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
