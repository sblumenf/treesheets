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
