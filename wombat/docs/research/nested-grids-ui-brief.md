> Background research, not instructions. The interface specification is `../UI_SPEC.md`; where this file differs from it, the specification wins.

# Wombat nested grids on a phone: the build brief

This is the plan to build from. `nested-grids-ui-plan.md` holds the background research and
sources. Where the two disagree, this brief wins.

## 1. The problem

TreeSheets shows grids inside grids, several levels deep, on one screen. A phone can't show five
levels at a readable size. Showing one level per screen would make Wombat "Workflowy with tables"
and lose what makes it different.

## 2. The answer: a stage with a depth budget

- **The stage** is the one grid you are working in, drawn at full size.
- **The depth budget:** each grid nested inside the stage is drawn as richly as it stays
  **readable** (text of about 11 pt or more, growing with the phone's text-size setting) and
  **tappable** (targets of 44 pt or more). When it can't pass both tests, it steps down a form.

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
  child cells        first cells,       rows, columns,     icon and
  tappable           smaller, one       colours, no text   cell count
  and editable       tap target         one tap target     one tap target
```

How many levels fit at once. It's the same rule everywhere, with a bigger budget on bigger screens:

```
 Phone   ██ stage   ▓ preview   ░ shape                    ≈ 1.5–2 levels
 Tablet  ██ stage   ██ full   ▓ preview   ░ shape          ≈ 2–3 levels
 Desktop ██ stage   ██ full   ██ full   ▓ preview   ░      ≈ 3–5 levels (TreeSheets today)
```

## 3. The phone screen

```
 ┌───────────────────────────────┐
 │ ▌Trip planning     Find    ⋯  │  title = the cell you dived into;
 ├───────┬───────────┬───────────┤  the colour stripe tells levels apart
 │       │ Plan      │ Budget    │  ← header row stays pinned
 ├───────┼───────────┼───────────┤
 │ Italy │┌─────────┐│ 2,400     │
 │       ││Day1 Rome││           │  ← a nested grid as a preview
 │       ││Day2 Fl… ││           │
 │       │└─────────┘│           │
 ├───────┼───────────┼───────────┤
 │ Spain │ ▇▇ ▇ ▇▇   │ 1,900     │  ← a deeper grid as a shape
 ├───────┼───────────┼───────────┤
 │ Notes │ Visa?   ▦4│           │  ← a badge
 ├───────┴───────────┴───────────┤
 │ ‹ Home › Trips › Trip pl… ••• │  trail bar: ‹ goes up, dots show depth
 │  + Add    Edit    Open ›   ⋯  │  bottom bar: within thumb reach
 └───────────────────────────────┘
```

- 3–4 columns fit across a phone. Wider grids scroll sideways, with the first column pinned.
- Every control sits in the bottom third of the screen. Every gesture also has a visible button.

## 4. Going down: one action, dive

**The rule is "tap what you want to work on".** A cell holding a grid has two parts: its own text,
and its child grid in a frame below it.

```
 ┌───────────────────────┐
 │ Italy                 │ ← tap the text: select. Tap again: edit.
 │┌─────────────────────┐│
 ││ Day1 Rome  Day2 Fl… ││ ← tap the child: DIVE
 │└─────────────────────┘│
 └───────────────────────┘
     long-press anywhere: menu (Move to…, Fold, …) with a larger preview on top
```

| You tap | Result |
|---|---|
| The cell's text | Select it; tap again to edit |
| A cell inside a **Full** child | Select that cell directly (no dive needed) |
| A **Preview**, **Shape** or **Badge** | Dive |
| **Open ›** (with the cell selected) | Dive. The labelled alternative, also used when a cell is too small to split |
| Long-press | The cell's menu, with a larger preview on top |
| Pinch out on a child | Dive (a shortcut) |

- Each part of the cell gets at least 44 pt. A cell too small to split becomes one target: a tap
  selects it, and you use **Open ›**.
- No triangles or chevrons are drawn on cells, so columns keep their width.
- **While editing, a tap outside the editor only ends editing. It never dives.**

**The dive:** the cell grows to fill the screen in about 300 ms. Its grid becomes the stage, and
its text becomes the title.

```
 ┌──────────────┐      ┌──────────────┐      ┌──────────────┐
 │ Italy ┌────┐ │      │ ┌──────────┐ │      │ ▌Italy       │
 │       │▦▦▦▦│ │ ───► │ │ ▦▦▦▦▦▦▦▦ │ │ ───► │ Day1 │ Rome  │
 │       └────┘ │      │ │ ▦▦▦▦▦▦▦▦ │ │      │ Day2 │ Flor. │
 └──────────────┘      └─┴──────────┴─┘      └──────────────┘
      tap child            grows                new stage
```

## 5. Going up, and never getting lost

```
 │ ‹  Home › Trips › Italy   ••• │
   │        └──────┬──────┘   └── one dot per level
   │               └── tap: the full path as a list; jump to any level
   └── tap: up one level.  Long-press: recent places
```

- The phone's back gesture (edge swipe on iPhone, back on Android) also goes up one level.
- Going up plays the dive in reverse: the screen shrinks back into its cell, which flashes once.
- Each level remembers its scroll position and selection on this device.
- **Jumps** from search, links and backlinks land straight on the level that shows the target
  readably. The trail bar updates and the target pulses once.

```
  search "visa" ──► Home › Trips › Italy › Notes    (one jump, no step-by-step drilling)
                                              ▲
                                    target cell pulses
```

## 6. Editing

One cell is edited at a time, in a single editor docked above the keyboard:

```
 ├───────────────────────────────┤
 │ Visa? Check embassy site_     │ ← the editor (one text field)
 │ Aa  ↵  →  +Row  +Col  Open › ✓│ ← the toolbar above the keyboard
 ├───────────────────────────────┤
 │  q  w  e  r  t  y  u  i  o  p │
 │   a  s  d  f  g  h  j  k  l   │
```

- The editor is the only text field in the app. If Flutter's own field doesn't feel right on
  iPhone, this one piece is swapped for a native iOS text view (the day-one test, §13).
- ✓ or tapping outside ends editing. Aa holds bold, italic, links and tags. ↵ is a line break
  inside the cell.

## 7. Adding rows, columns and grids

**While typing (the main way, as in Workflowy):**
- **Return** saves the cell and moves down. In the last row, it adds a row.
- **→** moves right. In the last column, it adds a column.
- **+Row** and **+Col** insert next to the current cell.

**At the end of the grid:** faint **+** bars on the stage grid only (never on previews). A tap
adds a row or column and starts editing its first cell.

```
 ┌───────┬────────┬───┐
 │ Italy │ Rome   │ + │  ← new column
 ├───────┼────────┤   │
 │ Spain │ Madrid │   │
 ├───────┴────────┴───┤
 │         +          │  ← new row
 └────────────────────┘
```

**Long-press menu:** Insert row above/below, Insert column left/right, New grid inside (dives into
an empty grid, ready to type), Delete row/column (with an Undo toast).

## 8. Restructuring

Restructuring uses explicit commands. Drag and drop comes later, if ever.

```
 select cells ──► bottom bar: [Move to…] [Cut to pocket] [Group into grid] [Ungroup] [Delete]
                         │                 │
                         ▼                 ▼
                 pick a destination   navigate anywhere, then
                 from the tree        "Paste from pocket ▾"
```

- **Multi-select:** a long-press menu item starts select mode, then tap cells to add them.
- **Move to…:** choose the destination from a list of places, like iOS Files.
- **Pocket:** cut cells stay in a tray while you navigate, shown as a chip above the bottom bar.
- **Group into grid / Ungroup:** wraps the selected cells into a new nested grid, or flattens one.
- **Undo:** every change shows a toast with Undo. Undo and redo are always in the ⋯ menu.

## 9. Search, tags and links

- Find filters the current stage in place. Non-matching cells dim, and branches with matches stay
  visible, as in Workflowy.
- "Search everywhere" lists results with their trail (`Trips › Italy › Notes`). Tapping a result
  jumps there (§5).
- Tags (`#todo`) and backlinks use the same results list.

## 10. Tablet and desktop

Same model, bigger budget. Tablets add a parent pane:

```
 ┌───────────────┬───────────────────────────────┐
 │ Trips         │ ▌Italy                        │
 │ ┌───┬───────┐ │ ┌──────┬────────┬──────────┐  │
 │ │It.│▓▓▓▓▓▓▓│◄┼─┤ Day1 │ Rome   │ ┌──────┐ │  │
 │ │Sp.│░░░░   │ │ │ Day2 │ Flor.  │ │▓▓▓▓▓▓│ │  │
 │ └───┴───────┘ │ └──────┴────────┴─└──────┘─┘  │
 │  parent pane  │  stage, with 2–3 levels shown │
 └───────────────┴───────────────────────────────┘
```

Desktop keeps TreeSheets' keys and adds mouse equivalents:

| Action | Desktop |
|---|---|
| Dive / go up | Ctrl+PgUp / Ctrl+PgDn, or Ctrl+mouse wheel |
| Go into the child grid | Shift+Enter (selects the first cell if the child is Full, otherwise dives) |
| Edit | Double-click, or start typing |
| End editing, then go to the parent | Esc |
| Menu / larger preview | Right-click / hover |
| Add a row or column | As on the phone (Return, → at the edge), or the + bars on hover |

## 11. Principles

1. **Show the shape, adapt the detail.** The structure is always visible; only the detail shrinks.
2. **One stage at a time, never one level at a time.**
3. **Always know where you are.** The trail bar is always there, and back always goes up.
4. **Visible first.** Every gesture has a button.
5. **Thumbs:** 44 pt targets, controls in the bottom third.
6. **Tables stay tables.** A grid never reflows into a list unless the user asks.
7. **Everything undoable**, with undo one tap away.
8. **Calm by default.** Power features stay out of the way until asked for. Tana's top complaints
   are its learning curve and its weak phone app.
9. **Native manners:** back gestures, menus, sheets and haptics behave as on each platform.

## 12. What to build, in order

| Scope | Contents |
|---|---|
| **First build** | Stage, depth budget, tap to dive and Open ›, trail bar, long-press menu, single editor with keyboard toolbar, adding rows/columns (§7), search with jumps, multi-select, Move to…, pocket, Group/Ungroup, undo |
| **Later** | List/card/board views of a grid (Tana-style), mini-map, split view on tablet, live search grids, a tray to pin cells for comparing |
| **Only if tests ask** | "Show here" (opening a child in place beyond the budget), a visible peek button |
| **Experiments** | Fisheye view, free zoomable canvas, typed templates (Tana supertags), drag to insert |

## 13. Steps and pass marks

```
 1 Paper ──► 2 Flutter prototype ──► 3 Test ×3 rounds ──► 4 Build on core ──► 5 Tablet/desktop ──► 6 Beta
   ~1 wk      ~2 wks, real phones     ~3 wks, 5 people      with core          parallel            TestFlight
                                       each                  milestones                             and Play
```

**Step 2, day one: the typing test.** Build one cell editor two ways: (A) Flutter's own text field,
(B) a native iOS text view. On a real iPhone, check autocorrect bubbles, dictation, cursor dragging,
Chinese/Japanese input, emoji and the keyboard toolbar. 3–5 people type in both without knowing
which is which. If A isn't as good as B, use B.

**Then the prototype**, with an in-memory model (throwaway, not the Rust core). Behind a switch:
- **Variant A:** the depth budget.
- **Variant B:** strict one level per screen.
- **Variant C:** one tap edits text directly, with selection by long-press.

It logs every dive and every up.

**Step 3: usability tasks** (five people per round, both variants in round 1):

| # | Task |
|---|---|
| 1 | Find a cell three levels deep without search |
| 2 | Edit a cell two levels deep, return to the top |
| 3 | Add a row to a sub-grid, fill two cells |
| 4 | Move a cell to a sub-grid in another branch |
| 5 | Compare values in two sibling sub-grids |
| 6 | Turn a cell into a 2×2 grid; group three cells into a new grid |
| 7 | Undo the last three changes |
| 8 | Find every `#todo` in one branch and open one |
| 9 | After 30 seconds of looking, describe what's in the document |
| 10 | In a 9-column grid, find row 12, column 7 |
| 11 | Fix typos in three cells of a small nested list, then carry on at the top |

**Pass marks:**
- Tasks 1–3: 4 of 5 people unaided, about a minute each.
- Task 4: 3 of 5 in round 1, 4 of 5 by round 3.
- Task 9: variant A beats variant B on the parts named. This is the proof the depth budget is
  worth it.
- At most one accidental dive per person per session, and none while editing.
- At most one "where am I?" per person per task.
- Task 11: 4 of 5 without asking to open the list "here". If they fail, add "Show here".
- Variant C versus the default: keep whichever gets fewer errors and complaints about the
  extra tap.

## 14. Risks

- **The depth budget is unproven.** On a phone it may show little more than previews. Task 9
  decides it.
- **One way down may be too few** (round trips on small children) or too easy to trigger
  (accidental dives). The logs show which.
- **Moving cells by touch is weak in every app.** Move to… and the pocket are safe but slow.
- **Comparing across branches stays awkward on phones.** Split view solves it on tablets.
- **Typing quality in Flutter on iPhone.** The day-one test settles it; the fallback is contained.
- **The product evidence is thin.** It's mostly vendor docs and reviews, so the usability rounds
  carry the weight.
