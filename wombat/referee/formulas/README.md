# Referee for formulas and programs

What the real TreeSheets calculates, recorded for the tests of Wombat's evaluator
(`crates/wombat-core/src/eval/`). The tests read `golden/*.jsonl` and never need the app; only
regenerating does.

| Path | Contents |
| ---- | -------- |
| `cases.py` | The cases: hand-written ones for every function, operation and error path of `src/evaluator.h`, and random ones from a grammar with a fixed seed |
| `app.py` | Builds a document in the real app through its script API and reads back what it shows |
| `make_formulas` | Records all cases into `golden/` |
| `probe` | Asks the app about one document, for whoever works on the evaluator |
| `start.cts` | The document that the edit sequences begin with, saved by the app (`make_formulas sequences` rewrites it) |
| `golden/formulas.jsonl` | A document and what every cell shows: 36 hand-written documents, 400 random ones |
| `golden/fgrid.jsonl` | The same for formula cells that have a grid of their own (see below) |
| `golden/programs.jsonl` | A document, what `ts.evaluate` returned for its root, and the document after it: 62 hand-written, 300 random |
| `golden/sequences.jsonl` | The document after each step of a series of edits: 8 hand-written series, 30 random |

The test that compares is `crates/wombat-core/tests/formulas_referee.rs`. The examples of
TreeSheets itself are compared by `crates/wombat-cts/tests/formulas.rs`, from `../golden/`.

## Regenerating

```sh
export TREESHEETS_BIN=/path/to/_build/TreeSheets    # see ../README.md for building it
wombat/referee/formulas/make_formulas               # all four files, about three minutes
wombat/referee/formulas/make_formulas programs      # one of formulas, fgrid, programs, sequences
wombat/referee/formulas/probe '["",0,1,2,[["A",0,1,2,[["1",0],["2",0]]],["=SUM(A)",6]]]'
```

Every case is built twice, in different instances of the app, and must come out the same.
The output is the same on every run, so `git diff` after regenerating shows what the app now
does differently. Regenerate when `src/evaluator.h` changes or cases are added.

The random cases come from `random.Random` with the seed in `cases.py`; Python promises the
same sequence for the same seed across versions for the functions used (`random`,
`randint`, `choice`, `sample`). The cases themselves are in the golden files, so a test never
depends on that.

## How a document is written

As nested lists, the same way in a case (`"doc"`) and in what the app showed (`"shown"`,
`"after"`):

```
[text, kind]                                   a cell without a grid
[text, kind, columns, rows, [cells, row by row]]
```

`kind` is TreeSheets' number: 0 data, 1 operation, 2 variable assign, 3 horizontal view,
4 variable read, 5 vertical view, 6 formula. In a case the text of a formula cell is its
formula; in what the app showed it is the result. The root cell has no text.

A step of a sequence is `["text", path, text]` (`ts.set_text`), `["formula", path, formula]`
(`ts.set_formula`), `["kind", path, kind]`, `["insert-row", path, row]`,
`["insert-column", path, column]`, `["delete", path, column, row, columns, rows]`, `["undo"]`,
`["redo"]`, and, by pressing the key of the menu item on the private display (`../ui.py`),
`["sort", path, column, row, rows, descending]` and `["run"]`. A path is a list of
`[column, row]` from the root; everything counts from 0.

## What to know

- **`ts.evaluate` is not Program > Run.** The script function runs the program and fills the
  views, but throws the variables away; the menu item keeps them for the formulas (a formula
  can read a variable by name). So `programs.jsonl` shows formulas without variables, and the
  test compares with a calculation without variables. The sequences press the menu item's key
  (`Ctrl+Alt+F5` never arrives on the private display, so `app.py` binds `Ctrl+Alt+N` through
  TreeSheets' own key bindings) and so do cover variables in formulas.
- **A variable-assign cell with nothing in hand crashes the app** (`Grid::EvalGridCell` reads
  through an empty value): at the start of a row, after an operation that failed, after a grid
  that gave nothing. The random programs assign only right after a plain data cell or a
  variable read, and no hand-written case does it. Wombat's behaviour there (see `run.rs`) is
  therefore not compared.
- **An operation cell can only be made from a text that is an operation** (`ts.set_cell_type`
  turns anything else into data). `app.py` marks the cell while its text is `+` and changes
  the text afterwards, which is how a file can have such a cell.
- **Formula cells with a grid** are named by their result, and TreeSheets goes by the result it
  showed last, which depends on what was calculated before. The app's script functions
  recalculate several times before the dump, so `fgrid.jsonl` shows the settled state; Wombat
  defines the settled state as the rule (decision B51). The random documents have no such
  cells.
- **Steps that find their cell gone** are left out of a random sequence when recording (an
  earlier random step deleted it), so the golden file has only steps that happened.
- **A step that Wombat refuses** (a row inserted into a cell without a grid) does nothing in
  the app; the test treats the refusal as nothing.
- **The locale** is `C.UTF-8` (`../referee.py`). In the plain `C` locale TreeSheets takes only
  ASCII letters for names and upper case; that is not recorded.
- Text only: the golden data has what cells show, their kinds and the sizes of grids, not
  colours or styles. What a view copies besides text (`Cell::Clone`) rests on reading the C++.
