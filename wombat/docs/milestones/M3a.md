# Milestone 3a: the command engine, text editing, properties, basic structure

Status: finished, as the first half of milestone 3. Milestone 3b (sort, transpose, hierarchify,
flatten, hierarchy swap, wrap, merge and split) is not started. All three checks pass
(`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`): 183 tests (143 before) and 1 that only runs by hand.
`cargo check -p wombat-core -p wombat-cts` passes for `wasm32-unknown-unknown`,
`aarch64-linux-android` and `aarch64-apple-ios` (it compiles there; nothing was run there).

## What was built

- **The editor** (`crates/wombat-core/src/edit/`). `Editor` owns a document. `apply(command,
  now, ids)` carries out one command; `undo()` and `redo()` walk the history; `begin_group()` /
  `end_group()` make several commands one undo unit; `set_history_limit` bounds the history.
  Each of `apply`, `undo` and `redo` returns a **change report**: the cells whose content
  changed, the cells created, the cells removed, the cells whose grid changed shape or
  children, and the tags changed. A command that cannot be done returns an error and leaves the
  document and the history exactly as they were.
- **25 commands** (list below): text editing, styling and linking of text ranges, properties,
  grids, rows and columns, clearing and deleting, moving, swapping, copying, tags.
- **Steps.** Inside, every command is a short list of small steps of ten kinds, each of which
  returns the step that undoes it. The history is those steps; no copies of the document.
- **Text rules** (`text.rs`): what happens to styled ranges and links when text is inserted,
  deleted or replaced, taken from TreeSheets' `src/textruns.h` and checked against that code;
  positions in code points; a command that would cut an emoji, an accented letter, a flag or a
  Hangul syllable made of several code points is refused; control characters become spaces.
- **Folding** as view state: `ViewState::set_folded`, outside the document and the history,
  and `ViewState::apply_changes`, which drops folds, selection and zoom of cells that are gone.
- **The construction layer** (`construct.rs`): the old unrestricted `cell_mut` and `add_grid`
  now exist only for code that imports the trait `construct::Construct` (reader, importer,
  tests); a document inside an `Editor` cannot be reached by them.
- **`wombat edit <file> --script <file>`**: editing commands as lines of text.
- **`referee/ops/`**: 44 operations done by the real TreeSheets and compared, and 3,304 edits
  compared with TreeSheets' own run and link code.

## How to try it

From `wombat/`:

```sh
cargo build
printf 'Shopping\n  Fruit | Price\n  Apple | 1.20\n    red\n    green\n  Pear | 2\n' > /tmp/shop.txt
rm -f /tmp/shop.wombat /tmp/before.wombat
target/debug/wombat new /tmp/shop.wombat --from /tmp/shop.txt
cp /tmp/shop.wombat /tmp/before.wombat

# A series of edits, shown, then all undone and saved: the file is byte for byte as before.
cat > /tmp/edits.txt <<'END'
text /1,2 "Apples"
style /1,2 0 5 +bold
insert-rows / 2 1
text /1,2 "Banana"
copy /1,3 /2,2
changes
show
undo all
save
END
target/debug/wombat edit /tmp/shop.wombat --script /tmp/edits.txt
cmp /tmp/shop.wombat /tmp/before.wombat && echo "identical to the original"

# The same edits kept: remove the "undo all" line.
grep -v '^undo' /tmp/edits.txt > /tmp/edits2.txt
target/debug/wombat edit /tmp/shop.wombat --script /tmp/edits2.txt
target/debug/wombat show /tmp/shop.wombat
target/debug/wombat edit --help          # every command, with the syntax
```

A cell is named by its place as `wombat show` prints it (`/` is the root, `/2,1` column 2,
row 1 of the root's grid, `/2,1/1,3` a cell of the grid inside that one) or by its ID. In the
command-line tool columns and rows count from 1; positions in text count from 0. Nothing is
written until a `save` line; a line that fails stops the run.

## The commands

Core name (`wombat_core::Command`) and the `wombat edit` line:

| Command | `wombat edit` | What it does |
| ---- | ---- | ---- |
| `InsertText` | `insert CELL POS "text"` | Insert text at a position |
| `DeleteText` | `delete CELL POS LEN` | Delete a range of text |
| `ReplaceText` | `replace CELL POS LEN "text"` | Replace a range; the new text takes the style and link of the first replaced character |
| `SetText` | `text CELL "text"` | Replace the whole text; drops styled ranges and links, as TreeSheets does |
| `SetRangeStyle` | `style CELL POS LEN +bold -italic` | Switch bold, italic, fixed, underline, strike on or off for a range |
| `SetRangeColor` | `color CELL POS LEN #rrggbb\|none` | Colour a range, or give it the cell's colour again |
| `SetLink` | `link CELL POS LEN url "x"` / `cell CELL` / `none` | Link a range to an address or to a cell (by ID), or unlink it |
| `SetProps` | `set CELL... name=value...` | Cell colour, text colour, relative size, alignment, vertical alignment, cell type, note, draw style, image, image scale, last-edit time; of the cell's grid: border colour, outer spacing, vertical layout. Any number of cells, one undo unit |
| `SetStyle` | `cell-style CELL... +bold` | Style of whole cells (base style and every range) |
| `SetColumnWidth` | `width CELL COLUMN WIDTH` | Width of one column |
| `CreateGrid` | `grid CELL COLUMNS ROWS` | Give a cell a grid of empty cells |
| `RemoveGrid` | `remove-grid CELL` | Remove a cell's grid with all in it |
| `InsertRows`, `InsertColumns` | `insert-rows CELL ROW COUNT`, `insert-columns` | Insert empty rows or columns |
| `DeleteRows`, `DeleteColumns` | `delete-rows CELL ROW COUNT`, `delete-columns` | Delete rows or columns with all in them; all of them removes the grid |
| `ClearCells` | `clear CELL...` | Empty cells: text, image and grid go, looks stay |
| `DeleteCells` | `delete-cells CELL COLUMN ROW COLUMNS ROWS` | TreeSheets' Delete key on a block: empty the cells, delete rows or columns left empty, remove the grid if nothing is left |
| `MoveRows`, `MoveColumns` | `move-rows CELL ROW COUNT TO`, `move-columns` | Reorder rows or columns |
| `SwapCells` | `swap CELL CELL` | Two cells trade places, also across grids |
| `MoveCell` | `move CELL EMPTY-CELL` | Move a cell (text, properties, grid, ID) to where an empty cell is; refused into its own inside |
| `CopyCell` | `copy CELL CELL` | Paste: the second cell becomes a copy of the first, new IDs inside, links inside the copy follow |
| `SetTag`, `RemoveTag` | `tag "name" #cell #text`, `untag "name"` | Add, recolour or remove a document tag |

Not commands: `fold`/`unfold` (view state), `undo [N|all]`, `redo [N|all]`, `begin`/`end`
(group), `now MILLISECONDS` (the time that edits stamp), `show [ids]`, `changes`, `check`,
`save`.

New rows, columns and grids take after their neighbours as in TreeSheets: a new cell gets the
colours, style, alignments, draw style and size of the cell before it in the row or column (the
first one when inserted in front), a new grid's cells those of the cell they are in; a new
grid gets the border colour and spacing of the grid around it, and new columns the width of
the column their grid's cell is in. Which commands stamp the last-edit time is decision A37.

## How the engine works (decisions A31-A39)

- **Steps** (`edit/step.rs`): splice text; set one field of a cell or its grid; set a tag; add
  cells; remove cells; insert lines (rows or columns); remove lines; move lines; replace a
  cell's whole grid; make two cells trade places. `Step::apply` changes the document and
  returns the undoing step. A step that removes cells holds those cells, since nothing else
  does any more; no step holds anything proportional to the rest of the document.
- **History**: a unit is the undoing steps of one command (or one group). Undo applies them
  and collects the redoing steps. Redo therefore puts back the very same cells, same IDs.
- **Failure**: a command function may return an error at any point; the editor then undoes
  the steps already applied.
- **Change report**: every step notes what it touches; the notes become the report. For one
  command the report is exact (the tests check it in both directions); for a group it may
  name a cell that was changed and changed back.

## Tests

183 tests pass; 40 are new.

- **28 unit tests** of the commands and text rules (`edit/tests.rs`, `text.rs`): every command
  with its edge cases and refusals, each successful command checked to undo and redo exactly,
  each refusal checked to leave document and history untouched.
- **Random-sequence tests** (`wombat-core/tests/commands.rs`, shared code in
  `tests/support/`): random commands of all 25 kinds on random documents of up to about 170 cells whose
  texts are full of characters made of several code points. Two properties, 1,000 cases each per run:
  1. after every command the document is valid, saves and loads back equal, and its texts
     are still whole characters; a refused command changed nothing; every change report is
     complete and tight, compared against a before/after snapshot; undoing everything gives
     the original document and the original bytes, with every stop on the way and every
     undo's report checked; redoing everything gives the final document, bytes and IDs;
  2. commands, groups, undo and redo in any order, against a model that keeps a snapshot per
     history unit.
  A third test counts that the generator gets every kind of command both carried out and
  refused (of 9,000 commands: 3,892 carried out, 3,346 refused, the rest asking for
  nothing new). Run once with 30,000 cases per property: no failure.
- **The same on real documents** (`wombat-cts/tests/commands.rs`): all 50 `.cts` files at hand
  (37 examples, 2 fixtures, 11 quirk files; 8,298 cells) get 40 random commands each (1,083
  carried out, 558 refused), view state kept in step; plus the two properties with 96 cases
  each on a random one of the 50. Run once with 20,000 cases each: no failure.
- **Large document** (`tests/edit_large.rs`): numbers below.
- **Command-line tool**: 2 new tests (a script of every sort of edit; undo all gives the
  original bytes; 14 kinds of bad line stop the run and write nothing) and the referee test.

The random tests found three real defects while this was being built, all in the change report
and all fixed: inserting or deleting columns did not report the changed list of column widths;
replacing text by the same text was reported (and recorded) as a change; pasting over a cell
that had a grid did not report that the grid's properties changed.

## Compared with the real TreeSheets

**Through the real app** (`referee/ops/`, test `wombat-cli/tests/ops.rs`): 44 cases. Each is a
Lobster script that does one operation in the real TreeSheets on `fixture.cts`, the app's
resulting document as golden JSON, and the same operation as `wombat edit` lines. The test
imports the fixture, runs the lines with the real `wombat` program, and compares every cell:
text, note, type, colours, size, style, alignments, per character style, colour and link, grid
size, border colour, column widths, vertical layout. 44 cases, 767 cells, no mismatches.
Regenerate with `referee/ops/make_ops` (needs the app; the test does not).

| Compared through the app | Cases |
| ---- | ---- |
| Set whole text (also with control characters) | 2 |
| Style a range: plain text, across ranges, inside one, joining two, back to plain, in a bold cell, non-ASCII | 7 |
| Colour a range | 2 |
| Link a range: plain, over links, joining, unlinking part, to a cell, non-ASCII | 6 |
| Insert a row or column: start, middle, end, in a nested grid; with undo, undo and redo | 9 |
| Delete cells: one, a row, a column, a block, a cell with a grid, all of a nested grid, the last cell of a row, everything, and two tie cases | 11 |
| Create a grid (three places) | 3 |
| Cell style, a set of properties (colours, size, alignments, note, column width, border colour) | 2 |
| Paste a subtree into an empty cell, paste a cell over a cell | 2 |

**Against TreeSheets' own code, without the app** (`referee/ops/textruns.cpp`, test
`wombat-core/tests/textruns.rs`): the script API of TreeSheets cannot insert or delete text in
the middle of a cell, so that could not go through the app. Instead `src/textruns.h`, the file
where TreeSheets keeps styled ranges and links in step with the text, is compiled as it is
into a small program that prints what it does for 3,304 edits of a 7-character text (8
starting states; every insert, deletion and replacement position; styling, colouring and
linking every range), and the Wombat commands are compared with that: no mismatches. Two
small operations in that program (`cellstyle`, `uncolor`) repeat four lines of `src/text.h`
by hand because that file cannot be compiled alone.

**Do these comparisons bite?** Both passed on the first full run, so they were tested by
breaking the code on purpose. Of 8 breakages, the app comparison caught 5 with its first 42
cases; two cases were added that catch two more (which rows or columns go when both could);
the eighth (links to the same target merging) cannot be seen in the app's dump and is caught
by the `textruns.h` comparison, which caught 6 of 6 breakages of its own. The breakages were
picked by hand, so this shows the comparisons are not empty, not that they are complete.

**Differences found:** TreeSheets' paste is not an exact copy (it keeps the target's size
and alignment, widens the column, and merges a text-less grid into the surrounding grid);
Wombat's is (decision B21). Nothing else differed in what was compared.

**Resting on Wombat's own tests only** (the script API does not reach them, or they have no
TreeSheets equivalent): inserting, deleting and replacing text *in the app* (only the
`textruns.h` comparison); cell style and text colour on cells that have styled ranges (the
script API's versions clear the ranges; Wombat follows the menu's, by reading the C++);
`DeleteRows`/`DeleteColumns`, `RemoveGrid`, `ClearCells` on their own; moving rows and
columns, swapping and moving cells; tags; folding; which commands stamp the edit time; image,
cell type, draw style, outer spacing and vertical layout properties; the grapheme rule.

## Measurements

One document of 101,001 cells (three levels), best of 20, on this development container:

| Operation | Debug build | Release build | Cells in the report |
| ---- | ---- | ---- | ---- |
| Insert text in one cell / undo | 4.8 µs / 2.6 µs | 0.41 µs / 0.27 µs | 1 |
| Delete text in one cell / undo | 4.4 µs / 2.3 µs | 0.37 µs / 0.26 µs | 1 |
| Style a range / undo | 2.7 µs / 1.5 µs | 0.27 µs / 0.22 µs | 1 |
| Insert a row in a 10 x 10 grid / undo | 32 µs / 11 µs | 5.8 µs / 1.2 µs | 11 |
| Insert a row in the 10 x 100 root grid / undo | 33 µs / 11 µs | 5.4 µs / 1.2 µs | 11 |
| Insert a column in the root grid / undo | 327 µs / 128 µs | 56 µs / 17 µs | 102 |
| Delete a root row holding 1,010 cells / undo | 1.4 ms / 1.5 ms | 0.15 ms / 0.22 ms | 1,011 |
| Move a root row / undo | 9.9 µs / 9.6 µs | 1.0 µs / 1.0 µs | 1 |
| Swap two cells of different grids / undo | 7.1 µs / 3.5 µs | 0.7 µs / 0.4 µs | 2 |
| For comparison: checking the whole document | 246 ms | 58 ms | |

The test fails above 20 ms (debug) or 5 ms (release) per operation. Costs that are not
constant, with the reason: a text command reads the cell's text once (positions are code
points); inserting, deleting or moving lines rewrites the ID list of that one grid; deleting,
copying, swapping or moving a cell with a grid walks what is inside it (to remove it, copy
it, or check the nesting limit); `DeleteCells` looks at every cell of the one grid, as
TreeSheets does. Nothing looks at the rest of the document. `Editor::new` validates the whole
document once.

## Dependencies

`wombat-core` gained `unicode-segmentation` 1.13 (no dependencies of its own, `no_std`), for
the boundaries of user-perceived characters. `wombat-cli` gained `serde_json` for tests only.

## Not done, doubtful, or different from the brief

- **Text edits inside a cell were not compared with the running app**, only with its
  `textruns.h` (above). That file is the whole of the logic, but it is not the app.
- **The guarantee about user-perceived characters has two holes** (decision A33): ranges
  that come from imported files may already cut one, and an edit can make two neighbours
  join into one character with an old range end inside. Commands never cut one.
- **Formula cells**: text commands leave the cell type alone. In TreeSheets typing over a
  formula cell makes it a data cell; in Wombat the text of a formula cell is the formula, so
  what editing it should mean is left to milestone 5. `ClearCells` does make it a data cell.
- **Paste** is within one document, always replaces the target, and does not adjust the text
  sizes inside a pasted grid as TreeSheets does. Pasting from another document is not built.
- **Moving a cell** means trading places with an empty cell, so that the ID follows the
  content. TreeSheets' drag-and-drop was not studied.
- **Commands are plain data but have no serialised form yet** (no JSON for them, no `serde`
  derives). The command-line syntax is the tool's, not an interchange format.
- **The command-line tool counts columns and rows from 1** (like `wombat show`); the core
  counts from 0.
- **History is unbounded by default** and lives in memory only; closing loses it.
- **Undo restores cells but not view state** about them (folds, selection): decision A38.
- **Stricter files**: a `.wombat` file with a line break or tab in a cell's text, valid
  before, is now refused (decision A32, which also says why format 1.0 was tightened in
  place). `Document::tags` and `Document::extra` are now methods; `cell_mut` and `add_grid`
  need `use wombat_core::construct::Construct`.
- **Limits of the random tests**: documents are small (the large-document test covers size,
  not variety); the generator never makes unknown fields, images on disk or links across
  documents; 1,000 cases per run is a smoke level, the 30,000-case run was done once by hand.
- **`referee/ops/golden/textruns.jsonl` is 600 KB** of generated text in the repository.
- **Unknown fields of cells** survive every command; `CopyCell` copies them to the target,
  which is a guess about fields this version cannot understand.
- The CI file (`.github/workflows/wombat.yml`) is outside `wombat/` and was not touched.

## What milestone 3b should know

**Adding a command:**

1. Add a variant to `Command` in `edit/mod.rs` (plain data, cells by ID, a doc comment).
2. In `edit/command.rs` add a function `fn my_command(tx: &mut Tx, ...) -> Result<(), Error>`
   and a line in `run`. Read the document with `tx.doc()`, check everything you can first
   (return `refused(...)`, `Error::NotFound`, `Error::Limit`), then call `tx.step(Step::...)`
   for each change. Returning an error after some steps is safe: they are undone.
3. Do not add step kinds. Restructuring is: `Step::SetGrid { cell, grid }` to give a cell a
   new grid (shape, list of cell IDs, widths) made of cells that already exist, which also
   makes it their parent; `Step::AddCells` before a grid lists new cells; `Step::RemoveCells`
   after no grid lists them any more. Between steps the document may be invalid; at the end
   it must pass `validate` (the tests check after every command). A sort is one `SetGrid`
   with the same cells in another order; a transpose one `SetGrid` with the sizes swapped;
   wrapping is `AddCells`, then `SetGrid` on the new cell and on the old owner. Build the new
   `Grid` with `Grid::new` and copy the properties you want to keep. See the test
   `the_general_grid_step_rearranges_and_reverses`.
4. Use `fresh.next(doc)` for new IDs, `new_cell` for cells that take after a neighbour,
   `descendants`, `height`, `check_depth`, `check_room` for the limits.
5. Tests: a unit test in `edit/tests.rs` (`Fixture::ok` checks undo and redo for you,
   `Fixture::bad` checks that a refusal changes nothing); a new kind in
   `tests/support/mod.rs` (`KINDS`, and an arm in `command`), which puts it into every
   random-sequence test at once; a line in `wombat-cli/src/edit.rs` and its `HELP`.
6. The change report needs nothing: steps note what they touch.

**The referee and restructuring:** TreeSheets' script API has no sort, transpose, hierarchify,
flatten, hierarchy swap, wrap or merge; they are menu actions (`Document::Action` in
`src/document.h`). `referee/run_script` cannot reach them as it is. Either they are driven
another way (keys through `xdotool` on the private display, or new script functions in
TreeSheets, which is outside `wombat/`), or 3b rests on reading the C++ and on its own tests,
and must say so.

**Also:** `DeleteCells` already contains TreeSheets' rule for which rows or columns disappear
after cells are emptied; `clear_cell` is `Cell::Clear`. Text sizes relative to a parent
(`Grid::RelSize` in TreeSheets) are not adjusted anywhere yet.

## Fixes after review

An independent review of the editing engine (milestones 3a and 3b) reproduced the problems
below. Each fix began with a test that failed because of the problem (they are in
`crates/wombat-core/tests/review.rs` unless another place is named); the decisions are A43 to
A48 and B33. The reviewer's finding about the parents of cells in `SetGrid` and `InsertLines`
was fixed with milestone 3b (A42) and is not repeated here.

| # | Finding | What was done | Test (seen to fail first) |
| ---- | ---- | ---- | ---- |
| 1 | Commands could build a document that cannot be saved: a grid of 1,000 by 1,000 cells, a text of 9 million emoji | The editor keeps an upper bound on the saved size, updated by every step, and refuses a command that takes it over 32 MiB. The limit on cells is 700,000 (was 5,000,000). A43 | `a_grid_too_large_to_save_is_refused...`, `a_text_too_large_to_save_is_refused` (both failed: accepted, then not saveable) |
| 2 | A command that asks for nothing new was not always nothing: a colour set to what it is stamped the edit time and entered the history; a colour set and set back was reported | `SetProps` makes steps for the net effect only; the stamp follows a real colour change; `TagCells` sets each tag once. A44 | `setting_the_colour_a_cell_has...`, `a_colour_set_and_set_back...`, `two_cells_that_tag_one_name...` (all failed) |
| 3 | `RemoveGrid(root)` and `ClearCells([root])` wiped the document while `DeleteRows` refused | One rule: the root keeps its grid and cannot be cleared or pasted over. B33 | `no_command_takes_the_grid_of_the_root_away` (failed) |
| 4 | History without bound; a keystroke held all runs and links of its cell; typing could not join the unit before it | Default bounds of 1,000 units and about 64 MiB; `Editor::apply_typing`; text edits hold only the runs and links they touch. A46 | `the_history_is_bounded...`, `typing_joins_the_unit...` (new functions, so no "before"); `a_keystroke_holds_only_the_runs_it_touches` (failed with the old steps: 288,784 bytes, the test allows 1,500) |
| 5 | An ID of a removed cell that the history still holds could be issued again | The editor keeps the IDs of the cells in its history; such an ID is refused. A48 | `the_id_of_a_cell_held_in_the_history_is_not_issued_again` (failed: accepted) |
| 6 | `SetLink` stored a link to an own cell with and without the document's ID as two different targets | One form, without the ID: in commands, when loading, in copies with fresh IDs, and `validate` refuses the other. A47 | `a_link_into_the_own_document_is_stored_without_the_document_id` (failed) |
| 7 | A group of commands had no report | `Editor::end_group` returns it, net of cells made and removed inside the group. A45 | `a_group_reports_what_it_changed_as_a_whole`, and the random walk checks every group's report for completeness |
| 8 | The random generators carried out some commands in under half of the tries | Arguments are taken from the document; every command is carried out in at least 60% of its tries, printed and asserted | `the_random_commands_reach_every_kind_both_ways` |
| 9 | `referee/README.md` said menu actions are out of reach | Rewritten to describe `ui.py`, `ops/` and what is still out of reach | |

### 1. Size

- **The limits now:** a file at most 32 MiB (unchanged); at most 700,000 cells (what such a
  file can hold: a cell takes at least 48 bytes); a document that commands build has a bound
  of at most 32 MiB, where the bound counts an empty cell as 81 bytes (its 48 and the
  deepest indentation), so commands stop at about 414,000 empty cells (20.3 MB as a file).
  Texts and notes still at most 16,777,216 code points each, and now also within the bound.
- **How it is kept cheap:** the bound is a sum over cells and tags (`edit/size.rs`). Every
  kind of step adds and subtracts the parts it touches: a text edit the bytes of the text
  that goes and comes, a property its old and new value, cells that are added or removed
  their own sum. No command looks at the whole document for this. A unit test compares the
  running sum with a fresh count for every kind of step, and the random tests do so after
  every command, undo and redo (and check that the saved file is not longer than the bound).
- **Where it is checked:** in `Editor::apply`, once, when the command is complete; so every
  command is covered, the 3b ones included, without a check of its own. Commands that make
  many cells (new grid, inserted rows and columns, flatten, wrap, split, and the three that
  work on a copy of a subtree) are also refused before they make more cells than a file can
  hold, and hierarchy swap stops before its copies pass the limit on cells.
- **Undo and redo are never refused for size.**
- **A file close to the limit** can have a bound over the limit (the bound is loose by up to
  about 33 bytes per cell). Such a document can still be edited; each command is then
  followed by writing the document out once to see that it still fits. That is the one case
  where a command costs time in proportion to the document; it ends when the bound is under
  the limit again.
- **The rule that is tested** (`what_commands_build_near_the_size_limit_can_be_saved`, 240
  random documents, 40 commands each, with the limit set just above the document in three
  ways): every accepted command leaves a document that saves within the limit and loads
  back equal; a refusal changes nothing; undo and redo of everything work. 5,033 commands
  were carried out and 2,390 refused as too large. Taking out the write-out check, or
  counting a part too short, makes tests fail (tried). One test works at the real limit
  (414,231 cells, 10 seconds in a debug build).

### 2 and 3. Nothing new is nothing; the root

The random tests now assert for every command of every kind: if the document is equal before
and after, the report is empty and the history untouched; and if the root had a grid before,
it has one after. `commands_that_ask_for_what_is_already_so_do_nothing` lists 25 such requests
by hand (sorting sorted rows, transposing one cell, adjusting by zero, and so on); those
passed before the fix too, the defect was in `SetProps` and `TagCells` only. The helper of the
3b unit tests removed the root's grid to set up its tables; it now starts from a root without
a grid.

### 4. History

- **Default:** 1,000 units and about 64 MiB, the oldest forgotten first, the newest always
  kept. 1,000 is TreeSheets' number (it also stops at 100 MB of copies); with typing joined
  into units it is a long session. The memory is an estimate: steps, and the cells, texts,
  runs, links and lists they hold.
- **Typing:** `apply_typing` joins a text edit to the newest unit if that is typing in the
  same cell. The caller decides when; the command-line tool does not use it.
- **Cost of a keystroke** in a cell with 2,000 runs, three keystrokes (at the start, inside
  a run, at the end): 288,784 bytes held before, 928 after; that is 96 KB against 0.3 KB per
  keystroke. What is left is the steps themselves: the text, the edit time, and the one or
  two runs that the keystroke touches.
- **Left as it was:** undo is refused while a group is open; styling a range (not typing)
  still holds the cell's whole list of runs; the redo history has no bound of its own.

### 7. Reports for groups

`end_group` returns the report of the outermost group; `wombat edit`'s `changes` shows it
after `end`. It is complete and exact about created and removed cells; it may name a cell that
was changed and changed back.

### 8. The random tests

How often each command is carried out, of about 240 tries each (300 documents, 30 mixed
commands each, one try in eight with numbers out of range on purpose; the test prints this
table and fails if any command is below 60%):

| Command | Carried out | Nothing to do | Refused | Carried out |
| ---- | ---- | ---- | ---- | ---- |
| `InsertText` | 178 | 47 | 15 | 74% |
| `DeleteText` | 202 | 11 | 17 | 87% |
| `ReplaceText` | 185 | 20 | 18 | 82% |
| `SetText` | 226 | 24 | 0 | 90% |
| `SetRangeStyle` | 203 | 28 | 14 | 82% |
| `SetRangeColor` | 152 | 62 | 29 | 62% |
| `SetLink` | 171 | 41 | 19 | 74% |
| `SetProps` | 152 | 37 | 27 | 70% |
| `SetStyle` | 184 | 50 | 0 | 78% |
| `SetColumnWidth` | 219 | 1 | 17 | 92% |
| `CreateGrid` | 221 | 0 | 23 | 90% |
| `RemoveGrid` | 200 | 0 | 41 | 82% |
| `InsertRows` | 216 | 14 | 13 | 88% |
| `InsertColumns` | 224 | 14 | 13 | 89% |
| `DeleteRows` | 211 | 0 | 51 | 80% |
| `DeleteColumns` | 161 | 0 | 51 | 75% |
| `ClearCells` | 220 | 9 | 20 | 88% |
| `DeleteCells` | 212 | 12 | 30 | 83% |
| `MoveRows` (was 49%) | 145 | 47 | 34 | 64% |
| `MoveColumns` (was 41%) | 150 | 68 | 28 | 60% |
| `SwapCells` | 191 | 24 | 6 | 86% |
| `MoveCell` | 137 | 0 | 62 | 68% |
| `CopyCell` | 181 | 12 | 30 | 81% |
| `SetTag` | 204 | 0 | 28 | 87% |
| `RemoveTag` | 190 | 0 | 83 | 69% |
| `SortRows` (was 30%) | 192 | 44 | 10 | 78% |
| `Transpose` | 200 | 56 | 0 | 78% |
| `HierarchySwap` (was 29%) | 135 | 0 | 81 | 62% |
| `Hierarchify` (was 42%) | 186 | 38 | 20 | 76% |
| `Flatten` | 201 | 13 | 36 | 80% |
| `Wrap` | 180 | 0 | 27 | 86% |
| `Collapse` (was 42%) | 157 | 3 | 68 | 68% |
| `SplitCell` | 242 | 0 | 9 | 96% |
| `TagCells` | 166 | 58 | 0 | 74% |
| `UntagCells` (was 36%) | 162 | 87 | 0 | 65% |
| `AdjustRelSize` | 197 | 24 | 0 | 89% |
| `AdjustColumnWidths` | 191 | 18 | 35 | 78% |
| `ToggleCheck` | 239 | 0 | 0 | 100% |

At the start of this pass, with the rule for the root in place, 20 of the 38 were under 60%
(the table of 3b showed only the new ones). What
changed: a sort picks a column in which two neighbouring rows differ and the direction that
turns them round; ranges for deleting, styling, colouring and linking are not empty; rows,
columns and blocks are taken from the grid's real size; swap, move and copy avoid the root
and cells inside each other; tags are named after cell texts; and most random documents get a
small hierarchy of single columns and a table with repeating first cells. `MoveColumns`,
`HierarchySwap` and `UntagCells` stay near 60% because after a few destructive commands a
small document often has no grid with two columns, no two levels of single-column grids, or
no cell whose text is a tag.

New in what the generators cover: units limits on the history (the walk sets them and the
model forgets with the editor); typing that joins; groups inside groups; a `SetProps` with
several properties, also set and set back; unknown fields at every level (document, tag, cell,
run, link, image, grid) in one document in three; texts of characters made of several code
points were there already. The budget in bytes is tested by hand-written cases only.

### Numbers

226 tests pass (210 before) and 1 runs by hand; `cargo test --workspace` takes about 1 minute
45 seconds in a debug build on this container (64 seconds before: the random tests check the
size bound, write the document out after every command, and work on larger documents).
`cargo fmt`, `cargo clippy -D warnings` and `cargo check -p wombat-core --target
wasm32-unknown-unknown` pass. The referee comparisons (38 files, 44 and 73 operation cases)
are unchanged and pass.

### Changed for callers

- `Editor::end_group` returns `Changes`. New: `apply_typing`, `set_history_budget`,
  `history_bytes`, `size_bound`, `set_size_limit`, `edit::size_bound`,
  `edit::DEFAULT_HISTORY_UNITS`, `edit::DEFAULT_HISTORY_BYTES`.
- The history is bounded unless told otherwise (`set_history_limit(None)` and
  `set_history_budget(None)` give the old behaviour).
- `limits::MAX_CELLS` is 700,000. A `.cts` file with more cells is refused by the importer.
- Refused now: `RemoveGrid`, `ClearCells` and `CopyCell` on the root; any command that would
  make the document too large to save; an `IdSource` that gives an ID held by the history.
- A document with a link that names its own document ID is invalid when built by hand; a
  file with one loads (the ID is dropped).

### Doubtful or left undone

- The size bound is loose (81 bytes for a cell that takes 48), which is why commands stop at
  about 60% of the cells a file could hold, and why files near the limit fall back to being
  written out after each command. Exact indentation would cure both at the price of steps
  that walk the cells they move; not done.
- `Editor::set_size_limit` is public mainly so that the rule can be tested near the limit.
- A refused command can still take the memory of the cells it made before the refusal: at
  most about 150 MB (414,000 cells), where it was 1.8 GB.
- Undo inside an open group is still refused.
- The combined report of a group is not tight (see 7).
