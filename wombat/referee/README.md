# Referee

The real TreeSheets app, run headless, is the referee for Wombat: what it reads from a `.cts` file,
and what it does to a document, is the reference that the Rust code is compared with. This folder
has the scripts that drive it and the reference ("golden") data it produced. Tests read `golden/`
and never need the app. Only regenerating needs it.

| Path | Contents |
| ---- | -------- |
| `export_golden` | Opens every example `.cts` in the real app and writes its exports to `golden/` |
| `run_script` | Runs one Lobster script against one `.cts` in a fresh app and prints the result |
| `make_fixtures` | Has the app build `fixtures/*.cts` from `fixtures/*.lobster` |
| `referee.py` | What the scripts share: private X server, app instance, agent socket, JSON writer |
| `ui.py` | Drives the app's menu actions by pressing keys on the private display (`xdotool`), for what no script function reaches |
| `dump_tree.lobster` | The script that walks a document and reads every property (for the JSON kind) |
| `fixtures/` | Test documents of our own, in the current file format, which no example uses |
| `golden/` | The reference data, and `MANIFEST.json` |
| `ops/` | Operation cases: one edit done by the real app and the same edit as Wombat commands. `make_ops` runs `cases/*.lobster` through the script API; `make_ui_ops` runs `ui/cases/*.ui` through `ui.py` |

The scripts are Python 3.8+ with the standard library only. They need `Xvfb` and a TreeSheets
built with Lobster; `ui.py` and `ops/make_ui_ops` also need `xdotool`. They never use an existing display, the user's settings or a running
TreeSheets: each file is opened as a copy in a new temporary directory, in a new instance
(`-i -p -a`) with its own `HOME`, on a private `Xvfb`. Nothing is left running afterwards.

## Building the referee

From the repository root, on Debian/Ubuntu (about 6 minutes on 4 cores):

```sh
apt-get install -y cmake g++ libgtk-3-dev libgl1-mesa-dev libglu1-mesa-dev libcurl4-openssl-dev \
    libsecret-1-dev libnotify-dev gettext xvfb xdotool
cmake -S . -B _build -DCMAKE_BUILD_TYPE=Release -DTREESHEETS_BUNDLE_WXWIDGETS=ON -DENABLE_IPO=OFF
cmake --build _build -j4
```

The scripts use `_build/TreeSheets`, or the binary that `TREESHEETS_BIN` names.

CMake downloads wxWidgets, Lobster, wxPdfDocument, Monocypher and libunibreak. Where the network
does not allow GitHub's source archives (`github.com/<owner>/<repo>/archive/...` gave 403 in the
Claude Code cloud container, while release downloads and `git clone` worked), clone the pinned
tags and point CMake at them by adding these to the first `cmake` command:

```sh
git clone --depth 1 --branch v2026.8 https://github.com/aardappel/lobster ../aardappel/lobster
git clone --depth 1 --branch v1.4.0 https://github.com/utelle/wxpdfdoc ../utelle/wxpdfdoc
git clone --depth 1 --branch 4.0.3 https://github.com/LoupVaillant/Monocypher ../loupvaillant/monocypher
#   -DFETCHCONTENT_SOURCE_DIR_LOBSTER=$PWD/../aardappel/lobster
#   -DFETCHCONTENT_SOURCE_DIR_WXPDFDOC=$PWD/../utelle/wxpdfdoc
#   -DFETCHCONTENT_SOURCE_DIR_MONOCYPHER=$PWD/../loupvaillant/monocypher
```

The tags are those in `cmake/Lobster.cmake`, `cmake/WxPdfDoc.cmake` and `cmake/Monocypher.cmake`.
Use the same ones when those files change.

## Regenerating

```sh
wombat/referee/export_golden            # all of golden/, about 15 seconds
wombat/referee/export_golden -o /tmp/g TS/examples/sales.cts    # some files, elsewhere
wombat/referee/make_fixtures            # only after changing a fixtures/*.lobster
wombat/referee/ops/make_ops             # ops/golden/, after changing ops/cases/*.lobster
wombat/referee/ops/make_ui_ops          # ops/ui/golden/, after changing ops/ui/cases/*.ui
wombat/referee/ops/make_ui_ops NAME     # one case
```

`export_golden` exports every `.cts` under `TS/examples` (all subdirectories) and `fixtures/`. The
output is the same on every run, so after regenerating, `git diff` shows what the app now does
differently. Regenerate when TreeSheets changes how it loads, exports or calculates, and when
examples are added. It exits with 1 and lists the file if one cannot be opened or exported.

`make_fixtures` is different: cells get the time they were made, so each run gives another
`.cts`. That is why the fixtures are checked in. Run `export_golden` after it.

## Golden kinds

For `TS/examples/<name>.cts` the files are `golden/<name>.<kind>`, with the subdirectory kept
(`golden/contrib/help/...`), and for `fixtures/<name>.cts` they are `golden/fixtures/<name>.<kind>`.
All are UTF-8 with LF line ends (`.gitattributes` keeps them so).

| Kind | What it is |
| ---- | ---------- |
| `.json` | Every property the script API can read, per cell. The most complete kind |
| `.xml` | TreeSheets' XML export (File > Export > XML) of the whole document |
| `.txt` | TreeSheets' indented text export: one line per cell, two spaces per level |
| `.run.txt` | The `.txt` export after Program > Run. Only for documents with operation cells |
| `.run-result.txt` | What that run returned: the text export of the value of the root cell |

`MANIFEST.json` lists, per document: the `.cts` format version, its size and SHA-256 (to notice
that an example changed), and counts (cells, grids, formulas, operations, notes, images, tags,
folded grids, cells with runs or links). It does not name the TreeSheets version, since a local
build's version is the time it was configured.

### `.json`

`{"selection", "autoexport_html", "autoexport_pdf", "root"}`, then one line per cell, in the order
of the file (row by row, a cell before the cells in its grid):

| Key | Meaning |
| --- | ------- |
| `path` | `[x, y]` in each grid from the root down. `[]` is the root cell |
| `text` | The text as shown. For a formula cell this is the calculated result |
| `formula` | The formula (`"=SUM(Numbers) - 10"`) if `type` is `formula`, else `null` |
| `note` | The cell's note, `""` if none |
| `type` | `data`, `operation`, `variable_assign`, `view_horizontal`, `variable_read`, `view_vertical`, `formula` (0 to 6 in the file) |
| `bg`, `fg` | Cell and text color as `#RRGGBB` |
| `relsize` | Text size steps, positive is bigger. The file has the opposite sign |
| `stylebits` | Bold 1, italic 2, fixed 4, underline 8, strikethrough 16 |
| `align`, `valign` | `auto`/`left`/`center`/`right` and `auto`/`top`/`middle`/`bottom` |
| `render_style` | How the cell's grid is drawn: `grid`, `bubble`, `line` |
| `vertical_layout` | Grid below the text (true, the default) or next to it. The file has it in the grid record, so it only means something for cells with a grid |
| `filtered` | Text hidden by a filter. Not in the file: always false after loading |
| `is_tag` | The text is one of the document's tags |
| `has_image` | The cell shows an image |
| `last_edit` | Milliseconds since 1970 UTC, or `null` where the file has no time: all cells of files older than version 14, and cells without text (the app gives those the time of loading) |
| `runs` | Spans whose style or color differs from the cell's: `start`, `len` in Unicode code points, as in the file, `start_utf8`, `len_utf8` in bytes of the UTF-8 text, `stylebits`, `fg` |
| `links` | Linked spans, with the same positions: `target`, and `internal` (target is the text of a cell) or not (a URL or file) |
| `grid` | `null`, or `cols`, `rows`, `folded`, `border_color`, `col_widths` (pixels, one per column) and `cells` |

`selection` is the saved selection as `x`, `y`, `cols`, `rows` in its grid, but not which grid.
`autoexport_html` is 0 (off), 1 (with images) or 2 (without).

### `.xml`

The same text as the menu export writes, minus its first lines: it starts at the root's `<grid>`,
without the XML prolog and the root's own `<cell>` element (the script API exports the grid of
the root, so the root cell's own properties are only in the `.json`).

It has: nesting (`<grid>`, `<row>`, `<cell>`), the text (escaped `&amp; &lt; &gt; &quot;`), and
as attributes, only where they differ from the default: `type` (number), `colorfg` and `colorbg`
(as `0xBBGGRR`: blue first, unlike the `.json`), `stylebits`, `relsize` (positive is bigger),
`align` and `valign` (numbers), and on grids `folded`, `bordercolor` (default `0xA0A0A0`) and
`outerspacing` (default 3). Styled spans are `<run stylebits=".." colorfg="..">` inside the text.
A formula cell has its formula as text, not the result.

It does not have: notes, links, images, tags, render style, vertical layout, column widths, edit
times, calculated formula results, the selection.

### `.txt`

The text of every cell, also the empty ones, indented by two spaces per level starting at two.
Formula cells show their result. A cell with an image has one space after its text. Nothing else.

## What no golden kind has

- **Images:** only `has_image`. Not which image, its bytes, type or display scale.
- **Tags:** only `is_tag` per cell. Not the list of tag names in the file or their colors.
- **The zoom level** saved in the file, and which grid the selection is in.
- **Grid outer spacing** is only in the `.xml`, column widths only in the `.json`.
- **A run that has a color equal to the cell's text color** and the same style looks like no run.
  Adjacent links to the same target read as one link.
- **Password protected files:** `export_golden` reports them as failed (the app would ask for the
  password in a dialog). No example or fixture is one.
- **Layout and rendering:** no sizes, positions, wrapping or pictures.

## What the importer author should know

- The examples use format versions 6, 7, 8, 13, 15, 17, 19, 23, 24, 28 and 30. None uses the
  current version 31, and none has runs (27+), links (31), text alignment (28+) or vertical
  alignment (29+) set. `fixtures/features.cts` (version 31) is there to cover those, with
  non-ASCII and non-BMP text in the spans. The 11 tutorials have formula cells (15 each).
- The loader turns control characters in text (below U+0020, so also line breaks and tabs) into
  spaces. The golden text shows that.
- The app refuses files newer than its own version, and still reads all older ones.
- Program > Run changes the document (it fills the view cells). `.run.txt` is the state after.

## Running a script against a document

```sh
wombat/referee/run_script FILE.cts SCRIPT.lobster               # prints ts.agent_result(...)
wombat/referee/run_script --dump json FILE.cts SCRIPT.lobster   # the document after the script
wombat/referee/run_script --json FILE.cts SCRIPT.lobster        # {"ok", "error", "result"}
```

The script works on a copy, so the `.cts` never changes. It starts at the root cell, and returns
a string with `ts.agent_result(s)`. The API is in `TS/docs/script_reference.html`, and the
`treesheets-agent` skill (`.claude/skills/treesheets-agent/SKILL.md`) has the practical notes.
`--dump xml|txt|json` prints the document as it is after the script, in a golden kind, to compare
with what the Rust core gives for the same operation. In that `json`, every `last_edit` is `null`,
since the script may have edited any cell. Exit code 1, with the error on stderr, if the script
does not compile or fails. A script that opens a dialog (for example `ts.save_document(true)`)
blocks until `--timeout` (300 seconds).

## Menu actions, by pressing keys

The script API has no function for most menu actions (sort, transpose, hierarchify, flatten,
hierarchy swap, wrap, collapse, split and others), and neither it nor the agent socket can run
an action by name. `ui.py` reaches them the way a person does: the app runs on the private
`Xvfb` display as always, a Lobster script sets the selection (`ts.select_range`), `xdotool key`
presses the action's shortcut on that display only, and the document is read back with
`dump_tree.lobster`. Actions without a shortcut get one through TreeSheets' own custom key
bindings, which in portable mode it reads from `TreeSheets.ini` in its working directory (the
table `ACTIONS` in `ui.py`). Nothing in TreeSheets is changed, and no existing display is touched.

A case is `ops/ui/cases/NAME.ui` (the selection and the actions, each with whether the document
must change), `NAME.edit` (the same as `wombat edit` lines) and `ops/ui/golden/NAME.json` (what
the app gave), all on `ops/ui/fixture.cts`, which `fixture.lobster` builds. `make_ui_ops` runs
every case twice in fresh apps and demands the same result, checks that the document changed or
not as the case says, and after reading the result presses one more key that always changes the
document, so that "nothing changed" means the action was refused and not that the key was lost.
The test that compares (`crates/wombat-cli/tests/ops.rs`) needs none of this.

What it took to make keys arrive, for whoever adds an action: the window must be given the
keyboard (`xdotool windowfocus`; there is no window manager); Ctrl+Alt+F-keys never arrive (the
X server takes them); and a new binding must not be a key that another menu item has.

## Known limitations

- Linux only (X11 through `Xvfb`, Unix socket in `/tmp`).
- One app start per document (under half a second each).
- The process is ended with SIGTERM, since there is no window manager to close it. The scripts
  remove its socket and token files themselves.
- Menu actions are reached only through their keyboard shortcuts (`ui.py`). Still out of reach:
  anything that needs the mouse (dragging cells, clicking a checkbox, resizing a column by
  hand) and dialogs (colour pickers, file choosers, search and replace). Text can be typed
  into a cell (`type` and `retype` in a `.ui` case: xdotool types, Enter ends the editing;
  Escape would cancel it); inserting and deleting text in the middle of a cell is compared
  with TreeSheets' own `src/textruns.h` instead, compiled into `ops/textruns.cpp`.
- What the dump cannot show cannot be compared: which cell is which (TreeSheets has no IDs), a
  tag's colours, grid spacing, image scale, edit times (see "What no golden kind has").
