# The `.wombat` file format, version 1.0

This describes the files Wombat reads and writes, precisely enough to write another reader or
writer. The implementation is `crates/wombat-core/src/format.rs` (documents), `view.rs` (view
state) and `storage.rs` (where files go). The reasons are in `DECISIONS.md` (A7, A8, A12, A16-A22,
A26-A30, A32).

## 1. Files on disk

For a document saved as `notes.wombat` in some folder:

```text
folder/
  notes.wombat                    the document (section 3)
  notes.wombat.images/            its images, one ordinary image file each (section 5)
  .wombat-view/<document id>.json its view state on this device (section 6)
```

- A **workspace** is a folder; its documents are the files directly in it whose names end in
  `.wombat` and do not start with a dot. Subfolders are not searched.
- The image folder is the document's file name with `.images` appended. It exists only if the
  document has images.
- View state is keyed by the document's ID, not its file name, so renaming a document keeps it.
  The `.wombat-view` folder differs per device and can be left out of sync and backups.
- While saving `x`, a writer may create `.x.tmp` in the same folder and rename it over `x`. A
  leftover `.x.tmp` is rubbish from an interrupted save and may be deleted. Wombat deletes
  whatever is at that name before it saves and creates the temporary file new, so that a link
  placed there is never written through (decision A27).
- If `x` is a symbolic link, Wombat saves to the file the link leads to (the temporary file is
  beside that file) and leaves the link in place; a link that leads nowhere is an error. The
  image folder and the view state stay beside the name the document was opened by.
- On Unix a saved file keeps the permissions of the file it replaces. Its owner and group become
  those of whoever saved it.
- **Copying a document** is not copying its file: the copy would have the same document ID and
  cell IDs, which must be unique (section 2). A copy is made by loading the document, giving it a
  new document ID and new cell IDs, with its links to its own cells following (section 3), and
  saving that, with its images and view state (`wombat copy`, decision A28). Two files of one
  workspace with the same document ID are an error that tools should report. Moving or renaming
  a file, with its image folder, is fine.

## 2. Common rules

- A file is one JSON value (RFC 8259) in UTF-8, without a byte order mark. A reader refuses
  anything else, and anything after the value except white space.
- **IDs** are 128-bit values written as a string of 36 characters in UUID form,
  `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, with **lower-case** hexadecimal digits. No other
  spelling is accepted. Wombat generates random (version 4) UUIDs, but any 128 bits are valid.
  Every cell and every document has an ID that never changes and is never reused, in this
  document or any other.
- **Colours** are strings `"#rrggbb"`. Readers accept upper- or lower-case digits; writers write
  lower case.
- **Positions in text** (`start`, `len`, the cursor) count Unicode code points (scalar values) —
  not bytes, and not UTF-16 units. `"a😀"` has length 2.
- **Whole numbers** must be JSON numbers without fraction or exponent (`2`, not `2.0`).
- **Defaults are omitted.** A field with its default value is left out by writers; readers accept
  it either way. Where this document says "required", the field has no default.
- **Unknown fields are kept.** In every object defined here, a field whose name is not listed is
  kept by a reader and written back when the document is saved, at the same place. It is kept as
  a JSON *value*: the order of the keys inside it and the spelling of its numbers may change
  (keys are written sorted; `1e300` may become `1e+300`). Numbers in unknown fields that fit
  neither a signed or unsigned 64-bit integer nor a double are not preserved exactly, so writers
  of future fields must stay within those. Unknown *values* of known fields (for example a
  `kind` this version does not list) are errors, not unknown fields.
- A JSON object must not repeat a key. A reader refuses a file in which any object does, at
  every level, also inside the value of an unknown field.

### Versions

Every file starts with `format` (what kind of file it is), `version` (the major version, a whole
number of at least 1) and `minor` (default 0), followed by the ID (`id`, in view state `doc`).
Writers put these four first, in this order, so that a reader can tell what a file is, and
which document, from its first bytes. A reader accepts them anywhere in the top-level object.

- A reader **refuses** a file whose `version` is greater than the one it knows, with a clear
  error and without reading the rest. Version 1 readers refuse 2 and above. (Wombat decides this
  from the header when the header comes first; what follows it need not even be JSON.)
- A reader that only wants a document's ID, for example to list a folder, may stop after the
  header. Wombat's listing does, so it does not notice damage further on in a file.
- A reader **accepts** a file whose `minor` is greater than the one it knows. A newer minor
  version may only *add fields* (which older readers keep as unknown fields). It may not add
  values to existing fields, change meanings or tighten rules; any such change needs a new major
  version.
- A writer writes the greater of its own minor version and the one the file was loaded with.

### Limits

A reader refuses a file that is over any of these. Writers never produce one: Wombat refuses to
save a document whose file would be over a limit, with an error that says which (decision A22).

| Limit | Value |
| ---- | ---- |
| File size | 33,554,432 bytes (32 MiB) |
| Cells in one document | 700,000 |
| Nesting depth of grids (root cell is depth 0; a cell at depth 32 cannot have a grid) | 32 |
| Text or note of one cell | 16,777,216 code points |
| Image file name | 255 bytes |
| Nesting of arrays and objects inside the value of one unknown field (`1` is 0 deep, `[]` is 1, `[{"a": []}]` is 3) | 24 |

The limits on nesting are chosen together, so that no file nests JSON arrays and objects more
than 127 deep in all, which common JSON parsers accept: the document object is 1 level, each
level of grid costs 3 (grid object, `cells` array, cell object), a run or link of a cell 2 more,
and 1 + 3 × 32 + 3 + 24 = 124. The limit on cells is what a file of the largest size can hold
and no less: a cell takes at least 48 bytes (its ID, one space of indentation, the line end),
and 700,000 × 48 is just over 32 MiB. So no file is refused for its number of cells alone; the
limit bounds documents built in memory, in line with what can be saved (decision A43).

**Editing stays within the limits** (decision A43). A document that editing commands built can
always be saved: the editor keeps an upper bound on the size of the file (every cell counted as
if it were indented as deeply as allowed, so 81 bytes for an empty cell; at most 414,000 empty
cells) and refuses a command that would take the bound over 32 MiB. A file close to the limit
can therefore load and still be over the bound; it can be edited, more slowly, as long as
writing it out shows that it fits.

## 3. The document file

```json
{
"format": "wombat", "version": 1, "minor": 0, "id": "0e6c1f0a-5c0e-4c57-9d5a-3b0f6f1f7a11",
"tags": {
 "todo": {"text_color": "#ff0000"}
},
"root": {"id": "9dd0919c-a9ab-4114-ab50-48e1075436b4", "text": "Shopping", "grid": {"xs": 2, "ys": 2, "cells": [
 {"id": "113d7972-8660-403f-a42c-29d1e0b61b42", "text": "Fruit", "bold": true},
 {"id": "5a084319-df59-48cd-bae9-932ec3c1b1da", "text": "Price", "bold": true},
 {"id": "e5cc7c31-1e95-442a-92a8-446ee3cf1fda", "text": "Apple", "grid": {"xs": 1, "ys": 2, "cells": [
  {"id": "0c1279f2-5c3a-46b6-be21-ff38df1048f8", "text": "red"},
  {"id": "0ec2f6b7-4ca1-484e-b6b5-b4707e610c3f", "text": "green"}
 ]}},
 {"id": "53aca59b-b416-4162-bede-a2ddffb6fdc4", "text": "1.20"}
]}}
}
```

### Document object

| Field | Type | Default | Meaning |
| ---- | ---- | ---- | ---- |
| `format` | string | required | Must be `"wombat"`. |
| `version` | whole number | required | Major format version: `1`. |
| `minor` | whole number 0..2^32-1 | 0 | Minor format version. Wombat always writes it. |
| `id` | ID | required | The document's ID. |
| `tags` | object | empty | The document's tags: tag name → tag object. |
| `root` | cell object | required | The root cell. |

**Tag object.** A cell "has" a tag when its text equals the tag's name (as in TreeSheets). A
tag's name, like a cell's text, has no characters below U+0020.

| Field | Type | Default | Meaning |
| ---- | ---- | ---- | ---- |
| `color` | colour | `#ffffff` | Background colour for cells with the tag. |
| `text_color` | colour | `#000000` | Text colour for cells with the tag. |

### Cell object

| Field | Type | Default | Meaning |
| ---- | ---- | ---- | ---- |
| `id` | ID | required | The cell's ID. |
| `text` | string | `""` | The text. For a formula cell, the formula (for example `=SUM(Expenses)`). No characters below U+0020: no line breaks, tabs or other control characters (decision A32). |
| `kind` | string | `"data"` | `data`, `code` (operation), `var_assign`, `var_read`, `view_h`, `view_v` or `formula`. |
| `color` | colour | `#ffffff` | Background colour. |
| `text_color` | colour | `#000000` | Colour of text outside coloured runs. |
| `draw` | string | `"grid"` | How the cell's grid is drawn: `grid`, `bubbles` or `lines`. |
| `align` | string | `"auto"` | `auto` (right for right-to-left text, else left), `left`, `center`, `right`. |
| `valign` | string | `"auto"` | `auto`, `top`, `middle`, `bottom`. |
| `size` | whole number, 32-bit signed | 0 | Text size relative to the default; positive is *smaller* (as in TreeSheets). |
| `bold`, `italic`, `fixed`, `underline`, `strike` | `true`/`false` | `false` | The base style of the text. |
| `runs` | array of run objects | empty | Rich text, see below. |
| `links` | array of link objects | empty | Links, see below. |
| `note` | string | `""` | A note attached to the cell. |
| `edited` | whole number, 64-bit signed | absent | When the text was last edited: milliseconds since 1970-01-01T00:00:00 UTC. |
| `image` | image object | absent | An image shown in the cell, see section 5. |
| `grid` | grid object | absent | The grid inside the cell. |

**Run object**: a range of the text styled differently from the base style.

| Field | Type | Default | Meaning |
| ---- | ---- | ---- | ---- |
| `start` | whole number 0..2^32-1 | required | Position of the first code point. |
| `len` | whole number 1..2^32-1 | required | Number of code points. |
| `bold`, `italic`, `fixed`, `underline`, `strike` | `true`/`false` | `false` | The **complete** style of the range, not relative to the cell's. |
| `color` | colour | absent | The colour of the range; absent means the cell's `text_color`. |

**Link object**: a range of the text that is a link. Links are independent of runs.

| Field | Type | Default | Meaning |
| ---- | ---- | ---- | ---- |
| `start`, `len` | as for runs | required | The range. |
| `url` | non-empty string | absent | An external target: a URL with a scheme, or else a file path, absolute or relative to the document. |
| `cell` | ID | absent | An internal target: the ID of a cell. |
| `doc` | ID | absent | With `cell`: the ID of the document the cell is in. Absent means this document; a writer never writes this document's own ID here. |

A link has `url`, or `cell` (optionally with `doc`), or neither; `url` together with `cell` or
`doc`, and `doc` without `cell`, are errors. A link with neither is one whose kind of target this
version does not know; its unknown fields say what it is. The target cell need not exist (it may
have been deleted, or be in a document that is not present).

A link to a cell of the document it is in has one form: `cell` without `doc` (decision A47). A
reader that finds `doc` equal to the document's own `id` treats it as absent, and it is written
back without. (So a copy of a document, which has a new ID, needs no change to such links, and
"the links to this cell" can be found by comparing cell IDs alone.)

One such unknown field is in use: the `.cts` importer (`crates/wombat-cts`) writes a TreeSheets
link to a cell by text, for which no cell with that text exists, as a link with neither `url` nor
`cell` and with `"cts_target": "<the text>"` (decision B14). The core keeps it like any unknown
field and does not interpret it.

**Rules for runs and for links** (each list separately): sorted by `start`; `len` at least 1; no
two ranges overlap; every range ends at or before the end of the text. A file that breaks them is
refused.

### Grid object

| Field | Type | Default | Meaning |
| ---- | ---- | ---- | ---- |
| `xs` | whole number 1..2^32-1 | required | Number of columns. |
| `ys` | whole number 1..2^32-1 | required | Number of rows. |
| `border` | colour | `#a0a0a0` | Border colour. |
| `spacing` | whole number 0..2^32-1 | 3 | Outer spacing. |
| `vertical` | `true`/`false` | `true` | `true`: the grid is below the cell's text; `false`: beside it. |
| `widths` | array of `xs` whole numbers 0..2^32-1 | all 80 | The width at which text wraps, per column. |
| `cells` | array of cell objects | required | Exactly `xs` × `ys` cells, row by row (row-major). |

### What makes a document invalid

A reader refuses the whole file (it does not repair or skip) when: a required field is missing; a
known field has the wrong type or an unlisted value; an ID is not in canonical form; two cells
have the same ID, or a cell has the document's ID; `cells` does not hold exactly `xs` × `ys`
cells; `widths` does not hold `xs` numbers; runs or links break their rules; a cell's text or a
tag's name has a character below U+0020 (a note may); an image name or
scale is invalid (section 5); an object repeats a key; or a limit is exceeded.

## 4. Canonical form (what Wombat writes)

Any JSON layout that follows section 3 is a valid file. Wombat itself always writes the layout
below, so that saving the same document twice gives **byte-identical** files:

- Line 1 is `{`. Line 2 holds `format`, `version`, `minor`, `id`, separated by `, `, ending `,`.
- If there are tags: a line `"tags": {`, one line per tag (` "name": {fields}`, sorted by name as
  Unicode code points, joined by `,`), a line `},`.
- One line per unknown document field, sorted by name: `"name": value,`.
- `"root": ` and the root cell, then a line `}`. The file ends with a newline (`\n` only).
- A **cell** is written on one line: `{`, its fields in the order of the table in section 3
  (unknown fields of the cell, sorted by name, come after `image` and before `grid`), `}`.
  Fields are separated by `, ` and written `"name": value`. Unknown fields of runs, links,
  images, tags and grids come last in their object (in a grid: before `cells`).
- A **grid** continues the cell's line up to `"cells": [`; then each cell of the grid is on its
  own line, indented by one space per nesting level, lines joined by `,`; then a line with the
  parent's indentation and `]}`, directly followed by the `}` that closes the parent cell.
- Strings escape only `"` as `\"`, `\` as `\\`, newline, carriage return and tab as `\n`, `\r`,
  `\t`, and other characters below U+0020 as `\u00xx` (lower-case digits). Everything else,
  including non-ASCII, is written as itself.
- Numbers of known fields are written in the shortest decimal form. Unknown values are written
  compactly (no spaces), object keys sorted, numbers as the shortest form that reads back equal.

## 5. Images

A cell's `image` object refers to a file in the document's image folder (section 1).

| Field | Type | Default | Meaning |
| ---- | ---- | ---- | ---- |
| `file` | string | required | The file's name in the image folder. |
| `scale` | number | 1 | Display scale; finite and greater than 0. |

- `file` is 1 to 255 characters from `A-Z a-z 0-9 . _ -` and does not start or end with a dot.
  It is a name, never a path, so a document cannot refer to a file outside its image folder.
- `file` is not a name that Windows reserves for devices: `CON`, `PRN`, `AUX`, `NUL`, `COM0` to
  `COM9` and `LPT0` to `LPT9`, in upper or lower case, alone or followed by a dot and anything
  (`nul.png` is refused, `nul_.png` and `com10.png` are not). So every valid name can be a file
  on every system.
- Wombat names an imported image `<new ID>.<extension>`, the extension being the original one in
  lower case if it is 1 to 8 letters or digits. Several cells may refer to the same file.
- The image file is in whatever image format it was imported in (PNG, JPEG…); the core does not
  decode it. A missing image file does not make the document invalid.
- Moving a document means moving its image folder with it; a copy of a document (section 1) gets
  copies of its images in its own image folder. Files in the image folder that no cell refers to
  are not removed by this version.

## 6. View state

Which grids are folded, the selection and the zoom are not in the document. They are in
`.wombat-view/<document id>.json`:

```json
{
"format": "wombat-view", "version": 1, "minor": 0, "doc": "0e6c1f0a-5c0e-4c57-9d5a-3b0f6f1f7a11",
"selection": {"cell": "113d7972-8660-403f-a42c-29d1e0b61b42", "xs": 2, "ys": 1, "cursor": [0, 5]},
"zoom": "e5cc7c31-1e95-442a-92a8-446ee3cf1fda",
"folded": [
 "e5cc7c31-1e95-442a-92a8-446ee3cf1fda"
]
}
```

| Field | Type | Default | Meaning |
| ---- | ---- | ---- | ---- |
| `format` | string | required | Must be `"wombat-view"`. |
| `version`, `minor` | whole numbers | required, 0 | As in section 2; the current version is 1.0. |
| `doc` | ID | required | The document this belongs to; must match the file name. |
| `selection` | object | absent | The selection. |
| `zoom` | ID | absent | The cell that is zoomed into and shown as if it were the root. |
| `folded` | array of IDs | empty | The cells whose grid is folded. Written sorted; duplicates are ignored. |

**Selection object**: `cell` (ID, required) is the top-left selected cell; `xs` and `ys` (whole
numbers 0..2^32-1, required) are how many columns and rows are selected from there (0 means an
insertion point before the cell, as in TreeSheets); `cursor` (optional) is `[start, end]`, the
selected range of the cell's text in code points while it is being edited, with `start` ≤ `end`.

View state can be older than the document, so it may name cells that no longer exist. That is not
an error; a reader drops those references. Unknown fields are kept as in section 2, and the
limits of section 2 on file size and on nesting inside unknown fields apply; `folded` holds at
most 700,000 IDs (the limit on cells). A copy of a document (section 1) gets a copy of the view state under its
own ID, naming its own cells. The canonical
layout is as in the example: line 2 the header, then `selection`, `zoom`, unknown fields and
`folded` (one ID per line), each on its own line.

## 7. Index caches

An index over a workspace (of links, of words for search) can be kept on disk, one file per
document, so that unchanged documents need not be parsed again (decisions A53, A71, A112).
It is **not part of the format, and not in the workspace folder**: it holds nothing that is
not in the documents, may be deleted at any time, and a reader that does not know it loses
nothing. A workspace can come from an archive, a shared drive or a sync service, and a cache
that came with it could not be trusted, so the cache is kept where the program that reads
the workspace says: a folder of the user's own. The command-line tool uses
`$WOMBAT_CACHE_DIR`, or else `wombat` in the system's cache directory (`$XDG_CACHE_HOME` or
`~/.cache`; `~/Library/Caches` on macOS; `%LOCALAPPDATA%` on Windows), or the folder given
with `--cache-dir`. A folder `.wombat-index` inside a workspace is where the cache was up
to milestone 6: it is never read, written or deleted, and can be removed by hand.

Its layout may change with any version of Wombat. For the record, the current one:

- Under the base folder, each workspace has a folder `<name>-<16 hexadecimal digits>`: the
  last part of the workspace folder's full path (letters, digits, `-` and `_` of it, at
  most 32) and a checksum of the whole path. In it, `workspace.txt` says in words which
  folder it is the cache of, and there is one folder per kind of index: `links` (the link
  index alone) and `search` (words and links together).
- The entry of `notes.wombat` is `<kind>/notes.wombat.idx`.
- The first line is `wombat-index <kind> <version> <rules> <length> <time> <inode> <changed>
  <checksum> <body length> <entry checksum>` and a newline: the kind of index; the version
  of its layout; a number (16 hexadecimal digits) for the rules the index was made by; of
  the document file its length in bytes, its modification time in nanoseconds since 1970,
  on Unix its inode number and the time of its last change of any kind (`-` for each that
  is not known), and the checksum of its bytes; the length of everything after the line;
  and a checksum over the line up to there, the name of the document file and everything
  after the line. A checksum is 32 lower-case hexadecimal digits (`summary::digest`).
- The rest is the index's summary of the document as bytes: numbers of 7 bits a byte, low
  bits first; texts with their length in bytes first; IDs as 16 bytes. What they hold is
  described at `DocLinks::encode` and `DocSearch::encode`.
- A reader ignores an entry unless its first line is exactly what the reader would write for
  the rest, for this document file name and for the checksum of the document file as it is,
  and then reads the document instead. Only if asked to (`CacheMode::Fast`) may a reader
  take the file to be unchanged without reading it: when length, times and inode are what
  the entry says and the entry was written at least two seconds after the modification
  time. An entry over 64 MiB is neither read nor written, and neither is anything in a
  cache folder that is a symbolic link, nor a cache folder that is one or that is inside
  the workspace.
