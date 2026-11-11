# A simple grid screen (UI1)

Status: **built, and run as a Linux desktop app by a test; nobody has looked at it or
touched it.** This is not a milestone of `PLAN.md` and not the screen that `UI_SPEC.md`
specifies. It is a first, small step towards that screen, built so that the bridge is used
by something that draws a grid, and so that the Android build and the measurements have
something real to run. It was built before the independent review of the bridge
(`NEXT_REVIEW.md`) was done; what the review finds may change the code underneath it.

## What it does

- **One grid on the stage**, as a table that scrolls both ways, with the text of the cell
  it belongs to as the title. A formula cell shows its result (in italics; in the error
  colour if it failed).
- **Into a nested grid and back out.** A cell that has a grid shows an "Open ›" target
  under its text. The trail bar at the bottom goes up one level by its arrow, or to any
  level by its name.
- **Editing the text of one cell.** Tapping a cell's text opens one editor docked at the
  bottom, with the cell's text (the formula, for a formula cell). The tick saves, the
  cross cancels, Return saves and moves the editor to the cell below. Saving sends
  `EnterText`, so a text that starts with `=` makes a formula cell.
- **Undo and redo** in the title bar. The cells that the last command, undo or redo
  changed or recalculated are tinted until the next action.
- **Save**, and the plain-text check of the bridge from milestone B1, in the menu.
- **Messages** (an error from the core, "Nothing to undo") have a line of their own above
  the trail bar.

It opens a copy of the example that comes with the app, a new copy at every start.

## How it is put together

- `app/lib/grid/stage.dart`: `Stage`, the logic with no widgets: which cell is on the
  stage, the trail, and each action as a call to the core followed by reading the stage
  again. It keeps nothing of the document but what it last read. If the cell on the stage
  is gone after an undo, it gives way to the nearest cell above that is still there.
- `app/lib/grid/grid_screen.dart`: the widgets. Every action goes through one function that
  drops a second action while the first runs, and shows what went wrong.
- `app/lib/main.dart` opens the example and shows the screen; `app/lib/demo_screen.dart` is
  the screen of milestone B1, moved.

**The bridge did not have to grow.** The screen needs only what milestone B1 built: read
one grid, apply a command, undo, redo, save. The trail is kept on the Dart side.

## Against `UI_SPEC.md`

Taken from it: one stage (section 3), the title and trail bars (4 and 5), tap text to edit
and tap the child grid to dive (6), one editor at a time with Return moving down (7), and
targets of at least 48 dp.

Left out, all of it for the real screen: the Full, Preview and Shape forms of a nested grid
(a nested grid is always the "Open ›" target, with no count); the first row and column
staying in place; the dive animation; restoring scroll position and selection per level;
the keyboard toolbar; adding rows, columns and grids; the cell menu; gestures; Clean mode;
search; tablet and desktop layouts; accessibility work; colours, styles, alignment, images,
notes and links of cells, none of which are drawn; drawing only the cells on screen.

## Tests

- `app/test/stage_test.dart` (9 tests, no device, the real core): the root, into a grid
  and out, jumping along the trail, typing with the report and the recalculated formula,
  undo and redo, making and unmaking a formula cell, the title following its text, a stage
  taken away by undo, a refused command, saving. CI runs these.
- `app/integration_test/grid_test.dart` (the real app, here on Linux under Xvfb): into
  "Costs", type and save, cancel, Return to the cell below, back by the trail, the editor
  of a formula cell, undo to the end with its message, redo, up by the arrow, and the
  bridge check from the menu. CI does not run this one.

The on-screen test found two faults that the logic tests could not: Return saved but did
not move the editor down (the move was dropped because the save was still counted as
running), and the pop-up message for "Nothing to undo" lay over the trail bar and swallowed
taps on it. Both are fixed, and the test now checks both.

## Not verified

- **How it looks, and how it feels.** No person has seen the screen: taking a picture of
  the window under Xvfb gave an empty image. The test finds widgets and reads their text;
  it does not judge layout, sizes or colours.
- Anything running on Android, iOS or a phone; the on-screen keyboard; touch. The app with
  this screen builds for Android (`flutter build apk --release`); `docs/ANDROID.md` says how
  to run it there.
- A large grid. All of a grid's cells are built at once; with a thousand cells that is
  untried, and with many more it will be slow.
- Text that needs more room than six lines is cut off with an ellipsis and can only be
  read in the editor, which has one line.
