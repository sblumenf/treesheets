# A simple grid screen (UI1)

Status: **built, and run as a Linux desktop app by a test; nobody has looked at it or
touched it.** This is not a milestone of `PLAN.md` and not the screen that `UI_SPEC.md`
specifies. It is a first, small step towards that screen, built so that the bridge is used
by something that draws a grid, and so that the Android build and the measurements have
something real to run. It was built before the independent review of the bridge
(`NEXT_REVIEW.md`) was done. The review and the evaluation after it changed it: it now
reaches the document through `Session`, saves by itself, keeps what is typed, and takes
its steps in order (decisions A144 to A148; `B1.md`, "Fixes after review", has each
finding with its test). This page says how it is now.

## What it does

- **One grid on the stage**, as a table that scrolls both ways, with the text of the cell
  it belongs to as the title. A formula cell shows its result (in italics; in the error
  colour if it failed).
- **Into a nested grid and back out.** A cell that has a grid shows an "Open ›" target
  under its text. The trail bar at the bottom goes up one level by its arrow, or to any
  level by its name.
- **Editing the text of one cell.** Tapping a cell's text opens one editor docked at the
  bottom, with the cell's text (the formula, for a formula cell). The tick enters the text
  and closes the editor, Return enters it and moves the editor to the cell below, the
  cross closes the editor without entering. Entering sends `EnterText`, so a text that
  starts with `=` makes a formula cell.
- **What is typed is kept** (A147). With the editor open, a tap on a cell or on "Open ›"
  enters the text and closes the editor, and does nothing else (`UI_SPEC.md` section 6).
  The up arrow, the trail, Undo and Redo enter the text and then do what they are for. If
  the core refuses the text, the editor stays open with it. Keys typed while the core is
  at work on Return or the tick stay in the editor.
- **Undo and redo** in the title bar. The cells that the last command, undo or redo
  changed, made or recalculated are tinted until the next action.
- **No Save: the document saves itself** (A145): a second after the first change that is
  not saved yet, at once when the app is no longer in front (with what is in the editor
  entered first), and when the screen goes.
- **When saving did not work**, a line that stays says so, with "Try again"; if another
  program changed the file, with "Keep mine" and "Open theirs". The same line says when
  the grid could not be read again after a change.
- **After a fault in the program** the screen shows one fixed sentence in place of the
  grid, and nothing that reaches the document can be pressed.
- **Messages** that pass (an error from the core, "Nothing to undo") have a line of their
  own above the trail bar, until the next action.
- The plain-text check of the bridge from milestone B1 is in the menu.

It has one document: at the first start a copy of the example that comes with the app,
and at every later start that same file as it was left. The file is still in the system's
temporary folder, which the system may clear (A148).

## How it is put together

- `app/lib/session.dart`: `Session`, the open document as the app holds it: every call to
  the core in the order it was made (A144). Nothing else touches the bridge's handle.
- `app/lib/grid/stage.dart`: `Stage`, the logic with no widgets: which cell is on the
  stage, the trail, each action as a call to the core followed by reading the stage
  again, and the saving. It keeps nothing of the document but what it last read. Its steps
  are taken one at a time in the order they were asked for, each judged by the stage as it
  is then (A146). If the cell on the stage is gone after an undo, it gives way to the
  nearest cell above that is still there; any other error leaves the stage where it was.
- `app/lib/grid/grid_screen.dart`: the widgets. Nothing is dropped while an action runs.
  It watches the app's lifecycle to save.
- `app/lib/main.dart` opens the document and shows the screen, and closes the document
  when the screen goes; `app/lib/demo_screen.dart` is the screen of milestone B1, moved.

**The bridge grew by what the review asked for**, not by what the screen needs to draw:
the screen reads one grid, applies a command, undoes, redoes, saves, overwrites and closes.
The trail is kept on the Dart side.

## Against `UI_SPEC.md`

Taken from it: one stage (section 3), the title and trail bars (4 and 5), tap text to edit
and tap the child grid to dive, and a tap outside the open editor only closing it (6), one
editor at a time with Return moving down (7), no Save for documents, and targets of at
least 48 dp.

Not in it, and here: the cross that closes the editor without entering its text (A147).

Left out, all of it for the real screen: the Full, Preview and Shape forms of a nested grid
(a nested grid is always the "Open ›" target, with no count); the first row and column
staying in place; the dive animation; restoring scroll position and selection per level;
the keyboard toolbar; adding rows, columns and grids; the cell menu; gestures; Clean mode;
search; tablet and desktop layouts; accessibility work; colours, styles, alignment, images,
notes and links of cells, none of which are drawn; drawing only the cells on screen.

## Tests

- `app/test/stage_test.dart` (27 tests, no device): on the real core, the root, into a
  grid and out, jumping along the trail, typing with the report and the recalculated
  formula, undo and redo, making and unmaking a formula cell, the title following its
  text, a stage taken away by undo, a refused command, what a redo makes, steps asked for
  at once, saving by itself and when asked, and a file that another program changed, kept
  or given up. On a stand-in for the core that can be made to fail
  (`app/test/fake_document.dart`): a save that fails, a grid that cannot be read after an
  edit or on the way in or up, and a fault in the program.
- `app/test/grid_screen_test.dart` (25 tests, no device, the widgets on the stand-in, which
  can also be held up): what a formula cell looks like, taps that come while the core is
  at work, every way typed text used to be thrown away, saving after a second and when the
  app is put away, the lines for a save that failed and their buttons, and the sentence
  after a fault.
- `app/integration_test/grid_test.dart` (the real app, here on Linux under Xvfb): that the
  document is a copy with IDs of its own, into "Costs", type and enter, the file having
  the edit a moment later, cancel, Return to the cell below, back by the trail with the
  editor open, the editor of a formula cell, undo to the end with its message, redo, up by
  the arrow, the app going out of sight with something half typed, the bridge check from
  the menu, and the app started again on the document as it was left.

CI runs all three.

The on-screen test found two faults that the logic tests could not: Return saved but did
not move the editor down (the move was dropped because the save was still counted as
running), and the pop-up message for "Nothing to undo" lay over the trail bar and swallowed
taps on it. Both are fixed, and the test now checks both.

## Not verified

- **How it looks, and how it feels.** No person has seen the screen: taking a picture of
  the window under Xvfb gave an empty image. The test finds widgets and reads their text;
  it does not judge layout, sizes or colours.
- Anything running on Android, iOS or a phone; the on-screen keyboard; touch; what a phone
  really does when the app is put away or ended (the saving was tried with the lifecycle
  events of the test binding, on Linux). The app with this screen builds for Android
  (`flutter build apk --release` on the cloud machine before the review; the debug package
  in CI since); `docs/ANDROID.md` says how to run it there.
- The app ended outright, without being put away first: what is in the editor is lost, and
  up to a second of entered changes (A145).
- A large grid. All of a grid's cells are built at once; with a thousand cells that is
  untried, and with many more it will be slow.
- Text that needs more room than six lines is cut off with an ellipsis and can only be
  read in the editor, which has one line.
