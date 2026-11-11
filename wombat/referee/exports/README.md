# Exports and imports by the real TreeSheets

What the real app writes when it exports, and what it makes of a file it imports: the
reference for Wombat's import and export (milestone 7). The tests read the files here and never
need the app; `make_exports` regenerates them. The general notes on the referee are in
`../README.md`.

| Path | Contents |
| ---- | -------- |
| `make_exports` | Drives the app and writes everything below |
| `fixture.lobster`, `fixture.cts` | A document of ten cases, each a cell with a grid: quotes, empty rows, text that looks like a formula, formula cells, nested grids, an outline |
| `golden/NAME.csv`, `.txt` | Per case: what the script function `ts.get_subtree_text` gives for the cell (1: CSV, 0: indented text) |
| `golden/NAME.tsv` | Per case: what File > Export > Tab delimited text writes with the view zoomed into the cell |
| `golden/document.txt`, `document.csv`, `plain.menu.csv` | The menu exports of the whole document as indented text and CSV, and of one case as CSV |
| `golden/examples/NAME.csv` | The CSV of the root's grid of each of the 38 documents that `../export_golden` exports |
| `imports/NAME.csv`, `.ssv`, `.tsv`, `.txt` | 25 files to import: comma, semicolon and tab separated values, indented text |
| `imports/golden/NAME.EXT.json` | The document the app made of each through File > Import: per cell its text, its type and its grid |

```sh
wombat/referee/exports/make_exports             # everything, about 100 seconds
wombat/referee/exports/make_exports imports     # one part: exports, examples or imports
wombat/referee/exports/make_exports --fixture   # also rebuild fixture.cts (new edit times)
```

## How it is done

- **CSV and indented text** of a cell come from the script API, as the golden files of
  `../golden/` do.
- **TSV** has no script function. The menu action is triggered with a key press on the private
  display (`../ui.py`, with key bindings of its own that this script adds), and the file dialog
  that opens is answered with Return: it proposes the document's name with the right extension
  in the document's folder, which is a private temporary one. The menu exports what the view
  shows, so the view is first zoomed into the case (`ts.zoom`).
- **Import** is a menu action too, with a file dialog: the path of a copy of the file is typed
  into it. The import opens in a new tab, which `../dump_tree.lobster` then reads.

## Bytes are kept as the app wrote them

`.gitattributes` here turns line-end conversion off for `golden/` and `imports/`:

- The app ends the lines of **indented text** with CR LF on Linux and Windows and with LF on
  macOS (`LINE_SEPARATOR` in `src/tools.h`). The files here have the CR LF it wrote. (The `.txt`
  files of `../golden/` were normalised to LF.)
- The rows of **CSV and TSV** end with LF on Linux (and would with CR LF on Windows).
- The files in `imports/` have CR LF, CR, a byte-order mark and no final line end where their
  names say so.

## What the app does, in short

- CSV: every field in double quotes, quotes doubled, nested grids left out (the cell's own text
  is exported), every row and column kept. No byte-order mark.
- TSV: a field is quoted only if it starts with a quote or holds a tab. So `say "hi"` and `a"b`
  are written bare, and a row of one empty cell is an empty line.
- Import: lines are split at CR and LF and empty lines dropped, before anything else. A quoted
  field that reaches the end of a line continues on the next, joined by a space. After a closing
  quote one character is skipped, whatever it is. A delimiter at the end of a line is not
  followed by an empty field. Text that starts with `=` stays plain text.
- Indented text: each leading space or tab is one step; the first line sets the top level; a line
  indented further than the level it is in goes into the grid of the line before it.
