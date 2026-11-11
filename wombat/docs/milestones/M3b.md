# Milestone 3b: restructuring

Status: finished; with it milestone 3 is built. Thirteen commands were added to the engine of
3a, 73 operations were carried out by the real TreeSheets and compared, and one fault of the 3a
engine that this work would have run into was repaired (below). All checks pass
(`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`): 210 tests (183 before) and 1 that only runs by hand, in 64 seconds.
`cargo check -p wombat-core --target wasm32-unknown-unknown` passes.

Other findings of the review of 3a (file size limits when saving, stamping of commands that
change nothing, protection of the root, memory of the history) were left for a separate pass,
as agreed; they are not touched here.

## What was built

- **13 commands** (`crates/wombat-core/src/edit/restructure.rs`, listed in `edit/mod.rs`):

  | Command | `wombat edit` | TreeSheets action | What it does |
  | ---- | ---- | ---- | ---- |
  | `SortRows` | `sort CELL COLUMN ROW COUNT [desc]` | Sort Ascending / Descending | Reorders whole rows by the text in one column |
  | `Transpose` | `transpose CELL...` | Transpose | Rows become columns in the grids of the cells |
  | `Hierarchify` | `hierarchify CELL` | Hierarchify | A table becomes a hierarchy: rows that start alike are merged under one cell |
  | `Flatten` | `flatten CELL` | Flatten | A hierarchy becomes a table: one row per innermost cell, its parents to its left |
  | `HierarchySwap` | `hierarchy-swap CELL` | Hierarchy Swap | The cells with this text move up a level and take their parents inside them |
  | `Wrap` | `wrap CELL COLUMN ROW COLUMNS ROWS` | Wrap in new parent | A block of cells moves into a new cell in its place |
  | `Collapse` | `collapse CELL COLUMN ROW COLUMNS ROWS` | Collapse Cells | The texts of a block are joined in its first cell |
  | `SplitCell` | `split CELL POSITION` | Split cell at the cursor | The text is cut into two cells inside the cell |
  | `TagCells`, `UntagCells` | `tag-cells CELL...`, `untag-cells CELL...` | Add Cell Text as Tag, Remove Cell Text from Tags | The texts of cells become tags with the cells' colours, or stop being tags |
  | `AdjustRelSize` | `size-by DELTA [deep] CELL...` | Increase / Decrease text size, with or without sub-grids | Adds to the relative size, within -32..8 |
  | `AdjustColumnWidths` | `width-by CELL COLUMN COUNT DELTA [deep]` | Increase / Decrease column width, with or without sub-grids | Adds to column widths, not below 5 |
  | `ToggleCheck` | `toggle-check CELL...` | Toggle todo checkbox | Adds open checkboxes, ticks them, or removes them |

  There are now 38 commands. `wombat edit --help` describes all of them.
- **A repair of the engine** (decision A42): undoing a step that moved cells into another
  grid did not give them back to the parent they had. No 3a command moved cells that way,
  but the recipe that 3a wrote down for 3b did, and a command that failed half-way could
  have left a broken document. The review of 3a found it; it was fixed first, with three
  tests that failed before the fix. It needed an eleventh kind of step, used only for undoing.
- **A way to drive the real app's menus** (`referee/ui.py`, `referee/ops/make_ui_ops`,
  `referee/ops/ui/`), see "Compared with the real TreeSheets".

## How to try it

From `wombat/`:

```sh
cargo build
printf 'Shop\n  Fruit | Apple | 10\n  Veg | Leek | 9\n  Fruit | Pear | 100\n  Fruit | apple | 9\n' > /tmp/shop.txt
rm -f /tmp/shop.wombat /tmp/before.wombat
target/debug/wombat new /tmp/shop.wombat --from /tmp/shop.txt
cp /tmp/shop.wombat /tmp/before.wombat
cat > /tmp/restructure.txt <<'END'
sort / 3 1 4
show
hierarchify /
show
hierarchy-swap /1,1/1,1
show
flatten /
show
transpose /
wrap / 1 1 2 1
changes
undo all
save
END
target/debug/wombat edit /tmp/shop.wombat --script /tmp/restructure.txt
cmp /tmp/shop.wombat /tmp/before.wombat && echo "identical to the original"
target/debug/wombat edit --help
```

The first `show` has the rows in the order 10, 100, 9, 9: numbers sort as text, as in
TreeSheets. To see a case as the real app did it and as Wombat does it:
`cat referee/ops/ui/cases/hswap_basic.ui referee/ops/ui/cases/hswap_basic.edit`, and
`cargo test -p wombat-cli --test ops -- --nocapture`.

## Every TreeSheets action, and where it went

TreeSheets has 265 actions (the `A_` list in `src/main.cpp`, without the end marker). A script
put each into one class and checked that none is missing or named twice. "Covered by 3a
commands" means the action is the same as one command that existed, or a few of them as one
undo group, with the selection turned into a list of cells by whoever calls; where the real
app was asked, the case is named. The bold, italic, underline, strike-through, cut, copy,
paste, undo, redo and run actions are not in the `A_` list (they have standard menu numbers):
styles, undo and redo are 3a commands, cut is `CopyCell` and `DeleteCells`, paste is
`CopyCell` (B21), run is milestone 5.

### Built in 3b: 20 actions

| Actions | Command |
| ---- | ---- |
| `A_COLLAPSE` | `Collapse` |
| `A_TRANSPOSE` | `Transpose` |
| `A_TOGGLECHECK` | `ToggleCheck` |
| `A_SPLITCELL` | `SplitCell` |
| `A_INCSIZE` `A_DECSIZE` `A_INCSIZENH` `A_DECSIZENH` | `AdjustRelSize` (with / without the cells inside) |
| `A_INCWIDTH` `A_DECWIDTH` `A_INCWIDTHNH` `A_DECWIDTHNH` | `AdjustColumnWidths` (with / without the grids inside) |
| `A_SORT` `A_SORTD` | `SortRows` |
| `A_HSWAP` | `HierarchySwap` |
| `A_TAGADD` | `TagCells` |
| `A_TAGREMOVE` | `UntagCells` |
| `A_WRAP` | `Wrap` |
| `A_HIFY` | `Hierarchify` |
| `A_FLATTEN` | `Flatten` |

### Covered by 3a commands: 68 actions

| Actions | How it maps |
| ---- | ---- |
| `A_ENTERGRID` `A_ENTERGRIDN` | `CreateGrid` (1 x 1, or the size asked for) when the cell has no grid; otherwise only a selection move |
| `A_CLRVIEW` | `ClearCells` on the cells whose kind is a view (which cells: the caller's list) |
| `A_MARKDATA` `A_MARKVIEWH` `A_MARKVIEWV` `A_MARKFORMULA` `A_MARKVARD` `A_MARKVARU` | `SetProps` kind |
| `A_IMAGE` `A_LASTIMAGE` | `SetProps` image (reading the picture file is the storage layer's) |
| `A_DELETE` `A_BACKSPACE` | On cells `DeleteCells`; in text `DeleteText`; on a line between rows or columns `DeleteRows` / `DeleteColumns` |
| `A_DELETE_WORD` `A_BACKSPACE_WORD` | `DeleteText`; where the word ends is cursor logic, left to the interface |
| `A_DELGRID` | `RemoveGrid` for each selected cell, as a group (compared with the app: `mapped_delete_grid`) |
| `A_MLEFT` `A_MRIGHT` `A_MUP` `A_MDOWN` | Outside text: the selected cells trade places with their neighbours, wrapping round: `SwapCells` (or `MoveRows` / `MoveColumns`) as a group. In text: cursor movement |
| `A_TT` | `SetStyle` (as bold, italic, underline, strike-through, which are not `A_` actions) |
| `A_CANCELEDIT` | Leaving text editing; when the typing is abandoned, `Editor::undo` |
| `A_CELLCOLOR` `A_TEXTCOLOR` `A_BORDCOLOR` `A_LASTCELLCOLOR` `A_LASTTEXTCOLOR` `A_LASTBORDCOLOR` | `SetProps` colour, text colour, border colour (of the grid the cells are in), or `SetRangeColor` for selected text; a tag's colours are not changed this way (B20) |
| `A_BORD0` `A_BORD1` `A_BORD2` `A_BORD3` `A_BORD4` `A_BORD5` | `SetProps` spacing on the cell that has the grid |
| `A_TEXTGRID` `A_V_GS` `A_V_BS` `A_V_LS` `A_H_GS` `A_H_BS` `A_H_LS` `A_GS` `A_BS` `A_LS` | `SetProps` draw on the cells (and those inside), and `SetProps` vertical on those of them that have a grid (B10): two commands in a group |
| `A_HYPERLINK` | `SetLink` |
| `A_PASTESTYLE` | `SetProps` (colours, alignments, image, note) and `SetStyle` with the values of the copied cell. Not exact: TreeSheets sets the cell's base style and leaves styled ranges alone, `SetStyle` changes the ranges too |
| `A_IMAGER` | `SetProps` image=none (compared with the app: `mapped_remove_image`) |
| `A_RESETSIZE` | `SetProps` size=0 on the cells and those inside (compared with the app: `mapped_reset_size`) |
| `A_RESETWIDTH` | `SetColumnWidth` to the default for each selected column |
| `A_RESETSTYLE` | `SetStyle` all off, `SetRangeColor` none over the text, `SetProps` alignments auto, as a group (compared with the app: `mapped_reset_style`) |
| `A_RESETCOLOR` | `SetProps` colours to the defaults (compared with the app: `mapped_reset_color`); for a cell whose text is a tag TreeSheets resets the tag instead, Wombat does not (B20) |
| `A_FOLD` `A_FOLDALL` `A_UNFOLDALL` | `ViewState::set_folded` (view state, not a command: A38) |
| `A_IMAGESCF` `A_IMAGESCN` | `SetProps` image-scale per cell; the scale is the cell's in Wombat (B9), so the new value is worked out by the caller |
| `A_DEFBGCOL` | `SetProps` colour on the cells that have the old background colour (which cells: the caller's list) |
| `A_MINISIZE` | `SetProps` size; the value TreeSheets sets depends on the display font setting, so the caller supplies it |
| `A_EDITNOTE` | `SetProps` note |
| `A_ALIGNAUTO` `A_ALIGNLEFT` `A_ALIGNCENTER` `A_ALIGNRIGHT` `A_VALIGNAUTO` `A_ALIGNTOP` `A_ALIGNMIDDLE` `A_ALIGNBOTTOM` | `SetProps` align / valign |
| `A_TAGSET` | `SetText` with the tag's name |

### Milestone 4 (links): 6 actions

| Actions | What |
| ---- | ---- |
| `A_BROWSE` `A_BROWSEF` | Opening what a link or a cell's text names |
| `A_LINK` `A_LINKREV` `A_LINKIMG` `A_LINKIMGREV` | Finding the cell a link or a text points at |

### Milestone 5 (formulas): 1 actions

| Actions | What |
| ---- | ---- |
| `A_MARKCODE` | Makes a cell an operation by looking its text up among the operations |

### Milestone 6 (search and filter): 30 actions

| Actions | What |
| ---- | ---- |
| `A_SEARCH` `A_RESTRICTVIEW` `A_CASESENSITIVESEARCH` `A_CLEARSEARCH` `A_CLEARREPLACE` `A_REPLACE` `A_REPLACEONCE` `A_REPLACEONCEJ` `A_REPLACEALL` `A_SEARCHNEXT` `A_SEARCHPREV` | Search and replace |
| `A_FILTER5` `A_FILTER10` `A_FILTER20` `A_FILTER50` `A_FILTERM` `A_FILTERL` `A_FILTERS` `A_FILTEROFF` `A_FILTERNOTE` `A_FILTERBYTAGS` `A_FILTERBYCELLBG` `A_FILTERBYSTYLE` `A_FILTERBYTEXT` `A_FILTERMATCHNEXT` `A_FILTERSHOWROWS` `A_FILTERSHOWSUBGRIDS` `A_FILTERUNION` `A_FILTERRANGE` `A_FILTERDIALOG` | Filters |

### Milestone 7 (import and export): 17 actions

| Actions | What |
| ---- | ---- |
| `A_EXPXML` `A_EXPHTMLT` `A_EXPHTMLTI` `A_EXPHTMLTE` `A_EXPHTMLO` `A_EXPHTMLB` `A_EXPTEXT` `A_EXPCSV` `A_EXPTSV` | Export as text formats |
| `A_IMPXML` `A_IMPXMLA` `A_IMPTXTI` `A_IMPTXTC` `A_IMPTXTS` `A_IMPTXTT` | Import |
| `A_PASTETEXT` `A_PASTETSV` | Paste text as cells (an import from the clipboard) |

### Not core: 123 actions

| Actions | What |
| ---- | ---- |
| `A_SAVEALL` `A_SETPASSWORD` `A_CHECKUPDATES` `A_MAKEBAKS` `A_TOTRAY` `A_AUTOSAVE` `A_SAVEDISMISS` `A_PREVFILE` `A_NEXTFILE` `A_MINCLOSE` `A_SINGLETRAY` `A_STARTMINIMIZED` `A_GLOBALKEY` `A_FSWATCH` `A_OPENTEMPLATE` `A_ADDTEMPLATE` `A_ADDCURTEMPLATE` `A_DETTEMPLATE` `A_TEMPLATE` | Files, windows, the app |
| `A_EXPIMAGE` `A_EXPSVG` `A_EXPPDF` | Export as a picture: drawing |
| `A_ZOOMIN` `A_ZOOMOUT` `A_INCFONTSIZE` `A_DECFONTSIZE` `A_RESETFONTSIZE` `A_SHOWSBAR` `A_SHOWTBAR` `A_BOTTOMTABS` `A_TRADSCROLL` `A_COLORS` `A_COLCELL` `A_FULLSCREEN` `A_SCALED` `A_ROUND0` `A_ROUND1` `A_ROUND2` `A_ROUND3` `A_ROUND4` `A_ROUND5` `A_ROUND6` `A_FASTRENDER` `A_INVERTRENDER` `A_INNERBORDERCOLOR` `A_ZOOMSCR` `A_CENTERED` `A_SHRINKTEXT` `A_RESETPERSPECTIVE` `A_THINSELC` `A_HOVERZOOM` `A_HOVERSHADOW` `A_FLATCOPY` `A_CUSTKEY` `A_SETLANG` `A_DRAGANDDROP` `A_DEFAULTMAXCOLWIDTH` `A_SET_FIXED_FONT` | Zoom, view and options |
| `A_LEFT` `A_RIGHT` `A_UP` `A_DOWN` `A_SLEFT` `A_SRIGHT` `A_SUP` `A_SDOWN` `A_ALEFT` `A_ARIGHT` `A_AUP` `A_ADOWN` `A_SCLEFT` `A_SCRIGHT` `A_SCUP` `A_SCDOWN` `A_HOME` `A_END` `A_CHOME` `A_CEND` `A_NEXT` `A_PREV` `A_ENTERCELL` `A_ENTERCELL_JUMPTOSTART` `A_ENTERCELL_JUMPTOEND` `A_PROGRESSCELL` `A_SCOLS` `A_SROWS` `A_SHOME` `A_SEND` | Cursor, selection and scrolling |
| `A_TUTORIALWEBPAGE` `A_SCRIPTREFERENCE` `A_HELP_OP_REF` `A_HELP_EXAMPLES` | Help |
| `A_PAGESETUP` `A_PRINTSCALE` | Printing |
| `A_FORMULABAR` `A_OPENCELLCOLOR` `A_OPENTEXTCOLOR` `A_OPENBORDCOLOR` `A_OPENIMGDROPDOWN` `A_DDIMAGE` `A_NOP` | Toolbar controls and menus, not actions on the document |
| `A_IMAGESCP` `A_IMAGESCW` `A_SAVE_AS_JPEG` `A_SAVE_AS_PNG` | Changing the pixels or the file type of a picture: pictures are files beside the document (B9), their bytes are not the core's |
| `A_IMAGESVA` `A_IMAGECOPY` `A_COPYCT` `A_COPYBM` `A_COPYSVG` `A_COPYWI` | Copying to the clipboard or to a file: the document does not change |
| `A_AUTOEXPORT_HTML_NONE` `A_AUTOEXPORT_HTML_WITH_IMAGES` `A_AUTOEXPORT_HTML_WITHOUT_IMAGES` `A_AUTOEXPORT_PDF` `A_DEFAULTIMAGE_PNG` `A_DEFAULTIMAGE_JPEG` | Export on every save, and the format of pasted pictures: settings Wombat has no feature for (A25) |
| `A_RUNSCRIPT` `A_RUNSCRIPTCODE` `A_ADDSCRIPT` `A_DETSCRIPT` `A_AGENTSERVER` `A_SCRIPT` | Scripting (out of scope of the proof of concept) |

Unsure: `A_PASTESTYLE` (the mapping is close, not exact, and was not compared with the app);
`A_MINISIZE`, `A_DEFBGCOL` and `A_CLRVIEW` (their logic for choosing values or cells is left
to the caller; one could argue for commands of their own); `A_MLEFT` and its three siblings
(moving cells with Ctrl+arrows is a rotation of the selection within its grid, which
`SwapCells` in a group can do but nothing here does for the caller); `A_REPLACE*` (classed
with search, though replacing changes text: the change itself is `ReplaceText`).

## Compared with the real TreeSheets

**How the app was driven.** The script API and the agent socket have no way to run an action
by number or name (read: `script_interface.h`, `treesheets_impl.h`, `lobster_impl.cpp`,
`agent_server.h`). So the keys are pressed: the app runs as before on a private `Xvfb`
display, a Lobster script sets the selection (`ts.select_range`), `xdotool key` presses the
action's shortcut on that display only, and the document is read back with the same dump
script as before. Actions without a shortcut (sort, hierarchify, flatten, the two tag
actions, remove image) get one through TreeSheets' own custom key bindings, which in portable
mode it reads from `TreeSheets.ini` in its working directory. Nothing in TreeSheets was
changed. What it took: the window has to be given the keyboard (`xdotool windowfocus`; there
is no window manager); Ctrl+Alt+F-keys never arrive (the X server takes them for switching
consoles); and a new binding must not be a key some other menu item has (Ctrl+Alt+L and
Ctrl+Alt+R were, and silently did something else).

**Safeguards.** Every case is run twice, in two fresh apps, and must give the same document.
Every action line says whether the document must change; the driver checks that. After the
result is read, one more key press that always changes the document shows that keys arrive,
so that "changed nothing" means refused, not lost. A full second regeneration of all cases
gave byte-identical golden files.

**Result.** 73 cases (`referee/ops/ui/cases/NAME.ui` and `.edit`, `golden/NAME.json`,
`fixture.cts` built by `fixture.lobster`), 6,497 cells compared (text, note, kind, colours,
size, style, alignments, per character style, colour and link, tag or not, image or not, grid
size, border colour, column widths), no mismatches. 68 of the first 70 matched on the first
run; the two that did not were mistakes in the cases (a wrong selection, and the Ctrl+Alt+R
clash). The test is `menu_actions_give_what_the_real_treesheets_gave` in
`crates/wombat-cli/tests/ops.rs`; it does not need the app.

| Operation | Cases | What they cover |
| ---- | ---- | ---- |
| Sort | 9 | ascending, descending, numbers as text, part of the rows, a block selection, ties (rows equal but for letter case keep their order), accented capitals, cells with grids, one row and one cell (nothing happens) |
| Transpose | 6 | whole grid, a column, grids inside a partial selection, the grid of one selected cell, twice, widths lost by turning twice |
| Hierarchy swap | 10 | basic, a chain of single cells (parents disappear), merging of cells and notes, across the document into a wide table (empty cells left), an emptied row taken along, a row that gets a column, twice, a parent with the same text, two refusals |
| Hierarchify | 6 | three columns, after sorting, two columns, one column with repeats, nothing to do, refused (not a table) |
| Flatten | 8 | a tree, a table, uneven depths, a whole-grid selection, the whole document (with image, styles, sizes), refused (no grid), after hierarchify, before hierarchify |
| Wrap | 8 | one cell, a row, a block, the whole grid, rows of a column, last columns, last rows, the looks of the first cell |
| Collapse | 5 | a row, a block, a column with grids, a styled first cell with a grid, refused (one cell) |
| Split | 4 | middle, start, end with an image, a styled cell that has a grid |
| Tags | 2 | add, add and remove |
| Sizes and widths | 6 | with and without the cells inside, both limits |
| Checkboxes | 4 | add, tick, remove, three times round |
| Mapped actions | 5 | delete grid, reset size, reset style, reset colours, remove image, each as the 3a commands the table above names |

**Do the comparisons bite?** After they passed, the code was broken on purpose in 22 ways
(sort with case, without the tie-break, reversing ties when descending; wrong widths after
transpose, hierarchify and wrap; flatten keeping colours or dropping sizes; and so on). The
cases caught 19. For the other three, three cases were added (a row that gets a column in a
hierarchy swap, a row taken along, the looks of a wrap's new cell), which catch them; the
first of these showed a real difference, since fixed (B29). The breakages were chosen by
hand: this shows the comparisons are not empty, not that they are complete.

**What the app cannot show**, and so rests on reading the C++ and on Wombat's own tests:
which cell keeps which ID (TreeSheets has none); the colours of a tag (the dump only says
whether a cell's text is a tag); grid spacing, image scale and edit times; the "vertical"
flag of new grids (every cell of the fixture has the default); the wording of refusals.
`AdjustColumnWidths` and the size limits were compared, but only at the top zoom level.

## Which cell keeps its ID (decision A41)

| Operation | Rule |
| ---- | ---- |
| Sort, transpose | All cells keep their IDs |
| Hierarchify | All cells keep their IDs; the first cell of a row that repeats an earlier one is removed; nothing is created |
| Flatten | Every cell keeps its ID: a leaf on its row, a parent on the first row it stands on; its repeats further down and the empty cells are new |
| Hierarchy swap | Swapped cells and what they held keep their IDs; of cells that merge, the first one there stays; the copy of a parent is new, unless the parent disappears, in which case the copy takes its ID |
| Wrap | The wrapped cells keep their IDs; the new parent and the empty cells left behind are new |
| Collapse | The first cell keeps its ID; the others are emptied or deleted |
| Split | The cell keeps its ID and holds the parts, which are new |

Each rule has a unit test (`edit/tests/restructure.rs`).

## Tests

210 tests pass; 27 are new.

- **3 tests of the engine repair** (`edit/tests.rs`): the two failures the review found, and
  every kind of step followed by its inverse and by the inverse of that, starting from cells
  that are children of other cells. All three failed before the fix.
- **18 unit tests** of the new commands (`edit/tests/restructure.rs`): what each does, the ID
  rules, every refusal, the nesting limit, undo and redo of each successful command.
- **The random-sequence tests cover all 38 commands** (`tests/support/mod.rs`): valid after
  every command, a refusal changes nothing, exact change reports, undo all gives the original
  bytes, redo all the final bytes and IDs, save and load; on random documents (1,000 cases
  twice) and on the 51 imported `.cts` files (1,292 commands carried out, 424 refused, and
  96 cases twice). They passed with the new commands on the first run, after the engine
  repair. The random documents now mostly have a root grid with several cells and more
  cells with text. How often each new command is carried out, of about 240 tries each in
  the test that counts (sequences of 30 mixed commands, which also delete grids; one try
  in six uses numbers out of range on purpose):

  | Command | Carried out | Nothing to do | Refused | Carried out |
  | ---- | ---- | ---- | ---- | ---- |
  | `SortRows` | 73 | 88 | 85 | 30% |
  | `Transpose` | 162 | 94 | 0 | 63% |
  | `HierarchySwap` | 62 | 0 | 154 | 29% |
  | `Hierarchify` | 103 | 62 | 79 | 42% |
  | `Flatten` | 129 | 54 | 67 | 52% |
  | `Wrap` | 148 | 0 | 59 | 71% |
  | `Collapse` | 95 | 2 | 131 | 42% |
  | `SplitCell` | 245 | 0 | 6 | 98% |
  | `TagCells` | 145 | 79 | 0 | 65% |
  | `UntagCells` | 89 | 160 | 0 | 36% |
  | `AdjustRelSize` | 197 | 24 | 0 | 89% |
  | `AdjustColumnWidths` | 147 | 14 | 83 | 60% |
  | `ToggleCheck` | 239 | 0 | 0 | 100% |
  | `MoveRows` (was 12%) | 111 | 51 | 64 | 49% |
  | `MoveColumns` (was 12%) | 101 | 62 | 83 | 41% |

  This is not "most of the time" for sort, hierarchy swap, hierarchify, collapse and
  untag: they need rows that differ, two levels of single-column grids, a table, a block
  with text, a cell whose text is a tag, and the small random documents often lack those
  after a few destructive commands. Every command is carried out at least 62 times per
  run of that test, and the test fails below 20.
- **3 rules of the operations** (`tests/commands.rs`, 300 random documents each): transposing
  twice puts every cell back (and the whole document, if the grid had one width for all
  columns; otherwise only widths differ); sorting is a permutation of whole rows, in order,
  stable, touches nothing else, and a second sort does nothing; flatten keeps every cell and
  its text, puts each leaf in the column of its level with its parents to the left, and its
  result can be hierarchified; wrap keeps the block's cells, in order, in one new cell.
- **Flatten then hierarchify**: TreeSheets promises nothing here, and in general the
  original does not come back (empty cells stay, colours and notes are gone, equal texts
  merge). One unit test shows the case where it does: leaves all at the same level and
  texts that differ under each parent give back the same shape, texts and IDs.
- **Large grid** (`tests/edit_large.rs`): numbers below; the test fails over 3 s (debug)
  or 0.5 s (release) per command.
- **Command-line tool**: one new test (every new line once, each named in the help, undo all
  gives the original bytes, 11 bad lines), and the comparison with the app.

## Measurements

A grid of 3 columns and 10,000 rows (30,001 cells), on this development container, one run:

| Operation | Debug build | Release build |
| ---- | ---- | ---- |
| Sort by a column with 50 different texts | 199 ms | 38 ms |
| Sort by a column with 10,000 different texts | 106 ms | 23 ms |
| Transpose | 30 ms | 3.5 ms |
| Hierarchify (to 50 groups of 200) | 362 ms | 119 ms |
| Flatten (back to 10,000 rows) | 447 ms | 143 ms |
| Wrap 9,000 of the rows | 346 ms | 86 ms |
| Undo of all seven | 531 ms | 94 ms |

Sorting compares texts case-folded each time it compares; hierarchify and flatten copy the
subtree they work on and compare it with the document afterwards. None of them looks at
cells outside the grid they work on. A first version of wrap took 6 s here (it searched the
grid once per cell); the test caught it.

## Different from TreeSheets, or doubtful

Decisions B26 to B32 record the deliberate differences. In short, and with what is merely
unresolved:

- **Sort** takes a column and rows, and has nothing to refuse for selection shapes (B26).
  Case folding uses Unicode's one-to-one lower case; TreeSheets uses the C library's, which
  gave the same for the accented capitals compared, but was only tried on Linux.
- **The "vertical" flag of new grids** comes from the grid around (B27), not compared.
- **Hierarchy swap** remembers that a grid was a row only within one command (B29).
- **Sizes** stop at -32 and 8 whatever the zoom (B30). **Checkboxes** in emoji form are
  refused, and the mouse form of the action is not built (B31).
- **Formula cells**: in TreeSheets the text of a formula cell is its result, in Wombat the
  formula. Hierarchify (which compares texts), flatten, collapse and hierarchy swap (which
  copy texts and make the copies data cells) therefore see other texts for such cells.
  Left for milestone 5.
- **Unknown fields** of a cell stay with the cell that keeps the ID; copies and empty
  leftovers do not get them. A guess, as in 3a.
- **Collapse** appends text without the grapheme check of the text commands: a text that
  starts with a combining mark joins the space before it (the second hole of A33).
- **Hierarchify** finds repeated first cells with a hash map; TreeSheets compares each row
  with all rows above it. Same result, not the same cost.
- **Colours of tags** set by `TagCells`, spacing of new grids and edit times were not
  compared with the app (it cannot show them).
- **`referee/README.md` was not changed** (the brief allowed only additions to `ops/` and a
  helper script): its sentence "Edits that only the user interface can do are out of reach"
  is no longer true, and its table lacks `ui.py`. `ui.py` and `ops/make_ui_ops` say how to use
  them. Regenerating needs `xdotool` (`apt-get install xdotool`) besides what the README lists.
- **Typing over a selection of several cells** wraps them first in TreeSheets; that is the
  interface calling `Wrap`, not built here.
- **The acceptance rates of the random tests** for five commands are low (table above).

## What the next milestone should know

- Links (milestone 4) can rely on A41: after any restructuring a surviving cell has its ID.
  Flatten and hierarchy swap copy texts with their links into new cells; those copies link
  to the same targets.
- A command that rearranges a subtree: load it into a `Scratch` (`restructure.rs`), change
  the copy freely, `commit`. A command that moves a few cells: `SetGrid` steps directly,
  as `wrap` and `split_cell` do; undo is exact whatever the order (A42).
- A new menu action in the referee: add it to `ACTIONS` in `referee/ui.py` (with a key no
  menu item has), write `NAME.ui` and `NAME.edit`, run `referee/ops/make_ui_ops NAME`.
