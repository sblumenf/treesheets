# Wombat interface specification (phone first)

## 1. Product

A Wombat document is a grid of cells. Any cell can hold another grid, nested to any depth. Cells
hold short text and can be styled, folded, tagged and linked. This spec covers the phone interface,
and how it widens to tablet and desktop.

## 2. Technical frame

- **Flutter**, one codebase for iOS, Android, macOS, Windows and Linux.
- **Document model:** the interface changes documents only by sending commands (add row, edit text,
  move cells, …). Every command is undoable and returns the cells it changed. Only those are
  redrawn. The prototype uses an in-memory model with this command interface. The real model is a
  Rust core called through `flutter_rust_bridge`, which replaces it later.
- **One action layer:** each user action exists once. Buttons, gestures, keys and screen-reader
  actions all call the same action.
- **View state** (current level, scroll positions, selection, folding, settings) is stored per
  device, separately from the document.
- **The cell editor is a replaceable component** with two implementations behind a switch: Flutter's
  text field, and a native iOS text view.

## 3. Depth budget

One grid is **the stage**, drawn at full size. Each grid nested inside it is drawn in the richest
form that passes both tests:

- **Readable:** text of at least 11 pt, scaled with the system text size.
- **Tappable:** every target at least 44 pt (iOS) or 48 dp (Android).

```
   FULL               PREVIEW             SHAPE              BADGE
 ┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
 │ Trip        │    │ Trip        │    │ Trip        │    │ Trip     ▦12│
 │┌─────┬─────┐│    │┌───────────┐│    │┌───────────┐│    └─────────────┘
 ││Day 1│Rome ││    ││Day1 Rome  ││    ││▇▇ ▇ ▇▇▇   ││
 │├─────┼─────┤│    ││Day2 Flor… ││    ││▇ ▇▇ ▇     ││
 ││Day 2│Flor.││    ││     3×4 ░░││    ││▇▇ ▇       ││
 │└─────┴─────┘│    │└───────────┘│    │└───────────┘│
 └─────────────┘    └─────────────┘    └─────────────┘
```

| Form | Content | Tap behaviour |
|---|---|---|
| Full | The child grid at near-full size | Each child cell can be tapped |
| Preview | The first cells' text at about 70% scale, the size ("3×4"), and a fade when cut off | One target |
| Shape | A miniature of the rows, columns and colours, with no text | One target |
| Badge | A grid icon and the cell count | One target, with a hit area of 44 pt / 48 dp |

The thresholds (minimum text size, preview scale) are settings in the prototype, so they can be
tuned.

> **Decided since** (not a change of the specification's intent; `DECISIONS.md` A166): the "cell
> count" of a Badge is the number of cells directly in the child grid, columns times rows, as
> the figure has it (a 3×4 grid is ▦12). The core keeps no count of the cells further down.
>
> **Decided when the first slice was built** (A182):
>
> - **"Near-full size" means that a column holds a short word.** The two tests alone would
>   let a Full grid have columns of 48 dp, in which every text breaks after a few letters.
>   A Full grid's columns are at least four and a half times the size of a cell's text (72
>   dp at the usual text size), and the grid has at most 24 cells and is one level below
>   the stage. Otherwise it steps down.
> - A Preview needs room for a few letters of each of its columns, a Shape a target's
>   width. A Badge is what is left, and what a grid folded on this device shows.
> - The prototype's settings are the smallest readable text and the scale of a Preview.

## 4. Phone screen

```
 ┌───────────────────────────────┐
 │ ▌Trip planning     Find    ⋯  │  title bar: current level's cell text, colour stripe
 ├───────┬───────────┬───────────┤
 │       │ Plan      │ Budget    │  first row and first column stay visible when scrolling
 ├───────┼───────────┼───────────┤
 │ Italy │┌─────────┐│ 2,400     │
 │       ││Day1 Rome││           │
 │       │└─────────┘│           │
 ├───────┼───────────┼───────────┤
 │ Spain │ ▇▇ ▇ ▇▇   │ 1,900     │
 ├───────┴───────────┴───────────┤
 │ ‹ Home › Trips › Trip pl… ••• │  trail bar
 │  + Add    Edit    Open ›   ⋯  │  bottom bar (hidden in Clean mode, §11)
 └───────────────────────────────┘
```

- The grid scrolls in both directions. Phones show 3–4 columns.
- A cell holding a grid draws its own text on top and the child grid in a frame below. Each part is
  at least 44 pt / 48 dp tall. A cell too small for both parts is one target.

> **Decided when the first slice was built** (A183, A184):
>
> - A row grows to hold its tallest cell, so no cell is too small for both parts: a cell
>   that holds a grid is always two targets.
> - The bottom bar has "+ Add", "Edit" and "Open ›", which act on the selected cell. Undo,
>   redo and the one ⋯ are in the title bar, with a word under the title for how the
>   document stands with its file. "Find" comes with section 12.
> - Keeping the first row and column in view is a third switch of the prototype, on to
>   start with. A first row taller than two fifths of the screen scrolls inside that much.

## 5. Navigation

### Going down (dive)

The tapped cell grows to fill the screen in 250–350 ms. Its grid becomes the stage, and its text
becomes the title.

```
 ┌──────────────┐      ┌──────────────┐      ┌──────────────┐
 │ Italy ┌────┐ │      │ ┌──────────┐ │      │ ▌Italy       │
 │       │▦▦▦▦│ │ ───► │ │ ▦▦▦▦▦▦▦▦ │ │ ───► │ Day1 │ Rome  │
 │       └────┘ │      │ │ ▦▦▦▦▦▦▦▦ │ │      │ Day2 │ Flor. │
 └──────────────┘      └─┴──────────┴─┘      └──────────────┘
```

### Going up

The animation reverses: the screen shrinks into its cell, and that cell flashes once.

```
 │ ‹  Home › Trips › Italy   ••• │
   │        └──────┬──────┘   └── one dot per level
   │               └── tap: full path as a list, jump to any level
   └── tap: up one level. Long-press: recent places
```

- Each level restores its last scroll position and selection.
- **Jumps** (search results, links, backlinks) open the level that shows the target readably,
  update the trail bar, and pulse the target cell once.

> **Decided when the first slice was built** (A183): a level remembers the column and the
> row at the top left of what was seen, not a distance, so it comes back with that row at
> the top from its first line. The level grows or shrinks into place as a whole, not out
> of the tapped cell's own rectangle. Each level of the trail is tapped by its name; the
> dots, the list of the whole way and "recent places" are not built.

## 6. Tapping cells

| Tap on | Result |
|---|---|
| A cell's text | Edit it (the editor opens, §7) |
| A cell inside a Full child | Edit that cell |
| A Preview, Shape or Badge | Dive |
| Long-press on a cell | The cell menu (§9), with a larger preview of its child grid on top (iOS context menu; bottom sheet on Android) |

- While the editor is open, a tap outside it only closes the editor. It never dives or selects.
- **Variant switches** in the prototype:
  - "Tap text" = **Edit** (default) or **Select first, tap again to edit**.
  - "Tap child grid" = **Dive** (default) or **Unfold in place** (the child is drawn Full inside
    its cell and the rows below move down; diving then uses Open › or a gesture).

> **Decided when the first slice was built** (A184, A185):
>
> - Closing the editor by a tap outside it enters what was typed (A147).
> - The system's back with the editor open closes the editor, as a tap outside does, and
>   does not also go up a level.
> - A grid that is drawn Full is gone into by "Open ›", of the toolbar or of the bottom bar.
> - "Unfold in place" goes one level: a grid inside an unfolded grid is dived into. An
>   unfolded grid scrolls sideways inside its cell, and a bar above it folds it again.
> - The long press and the cell menu (section 9) are not built yet.

## 7. Editing

One cell is edited at a time, in a single editor docked above the keyboard.

```
 ├───────────────────────────────┤
 │ Visa? Check embassy site_     │  editor
 │ Aa  ↵  →  +Row  +Col  Open › ✓│  keyboard toolbar
 ├───────────────────────────────┤
 │  q  w  e  r  t  y  u  i  o  p │
```

| Control | Action |
|---|---|
| Return | Save, move to the cell below. In the last row, add a row |
| → | Save, move to the cell on the right. In the last column, add a column |
| ↵ | Line break inside the cell |
| +Row / +Col | Insert a row below / a column to the right of the current cell |
| Aa | Bold, italic, link, tag |
| Open › | Dive into this cell's grid |
| ✓ | Close the editor |

- The edited cell stays visible above the keyboard and updates as you type.
- The editor has no custom gestures. The system's text gestures (cursor dragging, selection, undo)
  work unchanged.

> **Decided since**, by the owner or on the owner's behalf; the table above is as it was written.
>
> - **The editor also has a cross (✕): "not this".** It closes the editor and leaves the cell, and
>   everything that could be undone or redone, exactly as before the editor opened, also when
>   the app was put away in between (`DECISIONS.md` A147, A177).
> - **Line breaks are allowed in a cell's text**, so the ↵ key is real (A174). This is a
>   departure from TreeSheets, whose cells have none.
> - **Return in the last row is two steps of the undo history**: the text, then the new row.
> - **"Tag" under Aa is TreeSheets' tag for now**: the whole text of a cell is the name of a tag
>   of the document, and the cell is drawn in the tag's colours. Tags written inside a longer
>   text (`#todo`, section 12) are not built.
> - Nothing is sent to the document while the user types: the edited cell is drawn from the
>   editor's text, and the document has it at the tick, at Return, and when the app is put away
>   (A145, A177).
>
> **Decided when the first slice was built** (A186):
>
> - The tick and the cross are beside the field, always in sight; the other keys are a
>   strip under it that scrolls sideways on a narrow screen. Aa shows a second row with
>   Bold, Italic, Link and Tag: a row and not a menu, which would take the keyboard away.
> - +Row and +Col move the editor into the new cell beside the edited one.
> - Bold and Italic act on what is selected, or on the whole cell without a selection.
> - The editor moves only to a cell that is drawn cell by cell: where a row makes a Full
>   grid too large to stay Full, the row is added and the editor closes.

## 8. Adding rows, columns and grids

- Return and → at the edge, and +Row / +Col (§7).
- **"+" bars** after the last row and the last column of the stage grid only. A tap adds a row or
  column and opens the editor on its first cell.

```
 ┌───────┬────────┬───┐
 │ Italy │ Rome   │ + │  ← new column
 ├───────┼────────┤   │
 │ Spain │ Madrid │   │
 ├───────┴────────┴───┤
 │         +          │  ← new row
 └────────────────────┘
```

- Cell menu: Insert row above/below, Insert column left/right, New grid inside (creates an empty
  grid, dives into it, and opens the editor), Delete row/column.

> **Decided when the first slice was built** (A184): the cell menu comes with section 9, so
> nothing deletes a row or a column yet (undo takes back what was added) and no grid can
> be made inside a cell, except the first cell of an empty document, which the screen
> offers in place of the grid.

## 9. Cell menu and restructuring

**Cell menu (long-press):** Select, Insert…, New grid inside, Move to…, Cut, Copy, Paste, Group into
grid, Ungroup, Fold/Unfold, Style, Delete.

**Selection mode** (from Select, or by swiping the first column right, §10):

```
 select cells ──► bottom bar: [Move to…] [Cut] [Group into grid] [Ungroup] [Delete] [Done]
```

- In selection mode, a tap adds or removes a cell.
- **Move to…:** a picker browses the document in the same stage-and-preview style. Drop targets
  are "Into this cell (new grid)", "Before/after this row" and "As a new column".
- **Pocket:** cut cells stay in a tray, shown as a chip above the bottom bar, while you navigate.
  Tapping the chip offers "Paste here".
- **Group into grid:** wraps the selected cells into a new grid inside one cell. **Ungroup**
  replaces a cell's grid with its cells.
- **Undo:** every change shows a toast with Undo for 5 seconds. Undo and Redo are always in ⋯.

## 10. Gestures

| Gesture | Where | Action |
|---|---|---|
| Spread (two fingers apart) | On a child grid | Dive |
| Pinch (two fingers together) | On the stage | Up one level |
| Swipe left | On the first column of a row | Delete the row (with Undo toast) |
| Swipe right | On the first column of a row | Select the row, enter selection mode |
| Hold and drag | On a cell | Move it. Hold over a child grid to dive into it; hold over the trail bar to go up. Drop shows the same targets as Move to… |
| System back (edge swipe / Android back) | Anywhere | Up one level |

- Swipe gestures apply only on the first column, so they never conflict with scrolling the grid
  sideways.
- Every gesture has a button or menu equivalent and a screen-reader action.
- Gestures behave the same in normal mode and in Clean mode.

## 11. Clean mode

A setting. When on:
- **Hidden:** the bottom bar and the "+" bars.
- **Kept:** the title bar, trail bar, keyboard toolbar, cell menu, selection-mode bar, toasts
  and ⋯.

Clean mode switches off automatically while VoiceOver, TalkBack or Switch Control is running.

## 12. Search

- **Find** filters the current stage in place. Non-matching cells dim, and branches containing
  matches stay visible.
- **Search everywhere** lists results across the workspace with their trail
  (`Trips › Italy › Notes`). Tapping a result jumps there (§5).
- Tags (`#todo`) and backlinks use the same results list.

## 13. Tablet and desktop

Same model. The larger screen raises the depth budget, so more levels show as Full and Preview.

```
 ┌───────────────┬───────────────────────────────┐
 │ Trips         │ ▌Italy                        │
 │ ┌───┬───────┐ │ ┌──────┬────────┬──────────┐  │
 │ │It.│▓▓▓▓▓▓▓│◄┼─┤ Day1 │ Rome   │ ┌──────┐ │  │
 │ │Sp.│░░░░   │ │ │ Day2 │ Flor.  │ │▓▓▓▓▓▓│ │  │
 │ └───┴───────┘ │ └──────┴────────┴─└──────┘─┘  │
 │  parent pane  │  stage                        │
 └───────────────┴───────────────────────────────┘
```

- **Wide windows** show the parent level in a left pane.

| Action | Desktop |
|---|---|
| Dive / up | Ctrl+PgUp / Ctrl+PgDn, Ctrl+mouse wheel |
| Go into the child grid | Shift+Enter (selects the first cell if the child is Full, otherwise dives) |
| Edit | Double-click, or start typing on a selected cell |
| Close editor, then select parent | Esc |
| Cell menu / larger preview | Right-click / hover |
| Add row / column | Return / → at the edge, or the "+" bars (shown on hover) |

- On desktop, a single click selects and double-click edits.

## 14. Accessibility

- Every action is available as a screen-reader action: Edit, Open grid, Up one level, Select,
  Move to…, Delete, and the rest of the cell menu.
- A cell is announced as its text, its position ("row 3 of 7, column 2 of 4") and, if it holds a
  grid, the grid's size ("contains a 3 by 4 grid").
- Large text sizes apply to the depth budget, so children step down to Preview or Shape sooner.
- Contrast meets WCAG AA in light and dark themes.

## 15. Performance

- Only visible cells are built. After a command, only the changed cells are redrawn.
- Scrolling and the dive animation run without dropped frames on a three-year-old iPhone and a
  mid-range Android phone.
- Typing has no visible lag.

## 16. Prototype logging

Log every dive, up, jump, edit, gesture and undo with a timestamp and the variant settings. This
is used to count accidental dives and gesture use. The data stays on the device and can be
exported as a file.

> **Decided when the first slice was built** (A187): an entry says what was done, when,
> under which settings, how deep the level is and at which column and row. It holds no
> text of any cell and no cell ID. The log is exported as a file or copied to the
> clipboard from the prototype's settings.

## 17. Flows the prototype must support

1. Find a cell three levels deep without search.
2. Edit a cell two levels deep, then return to the top.
3. Add a row to a nested grid and fill two cells.
4. Move a cell into a nested grid in another branch.
5. Compare values in two sibling nested grids.
6. Turn a cell into a 2×2 grid; group three cells into a new grid.
7. Undo the last three changes.
8. Find every `#todo` in one branch and open one.
9. In a 9-column grid, reach row 12, column 7.
10. Fix the text of three cells in a small nested grid, then continue at the top.
11. Do flows 1–4 in Clean mode, and with VoiceOver on.

Test documents: a few hand-made documents, plus a generated one at least 6 levels deep, with grids
up to 12×20.
