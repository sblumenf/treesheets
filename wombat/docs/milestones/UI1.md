# A simple grid screen (UI1)

Status: **built, and run as a Linux desktop app by a test; nobody has looked at it or
touched it.** This is not a milestone of `PLAN.md` and not the screen that `UI_SPEC.md`
specifies. It is a first, small step towards that screen, built so that the bridge is used
by something that draws a grid, and so that the Android build and the measurements have
something real to run. It was built before the independent review of the bridge
(`NEXT_REVIEW.md`) was done. Three reviews and an evaluation changed it: it reaches the
document through `Session` and a `Keeper` that saves, keeps what is typed, and takes its
actions one at a time (decisions A144 to A148 and A154 to A159; `B1.md`, "Fixes after
review", has each finding with its test). This page says how it is now.

## What it does

- **One grid on the stage**, as a table that scrolls both ways, with the text of the cell
  it belongs to as the title. A formula cell shows its result (in italics; in the error
  colour if it failed).
- **Into a nested grid and back out.** A cell that has a grid shows an "Open ›" target
  under its text. The trail bar at the bottom goes up one level by its arrow, or to any
  level by its name. The system's back (Android's button or edge swipe) goes up one level
  too, and leaves the app only from the root.
- **Editing the text of one cell.** Tapping a cell's text opens one editor docked at the
  bottom, with the cell's text (the formula, for a formula cell). The tick enters the text
  and closes the editor, Return enters it and moves the editor to the cell below (in the
  last row it closes the editor), the cross closes the editor without entering. Entering
  sends `EnterText`, so a text that starts with `=` makes a formula cell. The editor keeps
  the keyboard while the core is at work on Return.
- **What is typed is kept** (A147). With the editor open, a tap on a cell or on "Open ›"
  enters the text and closes the editor, and does nothing else (`UI_SPEC.md` section 6).
  The up arrow, the trail, the system's back, Undo and Redo enter the text and then do
  what they are for. If the core refuses the text, the editor stays open with it and the
  message line says why. Keys typed while the core is at work on Return or the tick stay
  in the editor; if that happens under an action of the bars, the action is not done and
  the message line says so.
- **Actions are taken one at a time, in the order they were made** (A158): each finds the
  editor and the stage as the one before left them. A tap on a cell that is no longer on
  the stage when its turn comes does nothing.
- **Undo and redo** in the title bar. The cells that the last command, undo or redo
  changed, made or recalculated are tinted until the next action.
- **No Save: the document saves itself** (A145, A155): a second after the first change
  that is not saved yet, at once when the app is no longer in front, and when the screen
  goes. When the app goes out of sight, what is in the editor is entered first; when it
  only stops being in front (a window that loses the focus), it is not.
- **The cross still means "not this" after the app was away** (A157): the text that went
  into the document then is taken back, and the tick after it leaves one step to undo,
  not two.
- **When saving did not work**, a line that stays says so, with "Try again"; the app also
  tries again by itself, later each time (A155). If another program changed the file, the
  line has "Keep mine" and "Open theirs"; "Open theirs" first makes sure that file can be
  read, and gives nothing up if it cannot (A159). The same line says when the grid could
  not be read again after a change.
- **After a fault in the program** the screen shows one fixed sentence in place of the
  grid, and nothing that reaches the document can be pressed.
- **Messages** that pass (an error from the core, "Nothing to undo") have a line of their
  own above the trail bar, until the next action or a tap on them.
- The plain-text check of the bridge from milestone B1 is in the menu.

It has one document: at the first start a copy of the example that comes with the app,
and at every later start that same file as it was left. On Linux, macOS and Windows the
file is in the user's data folder (`~/.local/share/wombat/example.wombat` on Linux unless
`XDG_DATA_HOME` says otherwise); on Android and iOS it is still in the app's temporary
folder, which the system may clear (A156). If that file cannot be opened at a start, the
app says so and offers to try again or to put the file aside and start with the example.

## How it is put together

- `app/lib/session.dart`: `Session`, the open document as the app holds it: every call to
  the core in the order it was made (A144). Nothing else touches the bridge's handle.
  `InTurn`, which does the ordering there and in the three places below, reports an error
  that nobody waited for and refuses a piece of work that waits for its own queue (A154).
- `app/lib/keeper.dart`: `Keeper`, which owns the `Session` for every screen on the
  document and is the one place that saves: after a change, again after a failure, when
  the app is put away, at the close; it also holds "another program changed the file" and
  "ended after a fault" (A155).
- `app/lib/grid/stage.dart`: `Stage`, the logic with no widgets: which cell is on the
  stage, the trail, each action as a call through the keeper followed by reading the stage
  again. It keeps nothing of the document but what it last read, and does not save. Its
  steps are taken one at a time in the order they were asked for, each judged by the stage
  as it is then (A146). If the cell on the stage is gone after an undo, it gives way to
  the nearest cell above that is still there; any other error leaves the stage where it
  was.
- `app/lib/grid/grid_screen.dart`: the widgets, and one queue for everything the user does
  (A158). It tells the keeper how to enter what is in the editor when the app is put away.
- `app/lib/main.dart`: where the document is (`documentFolder`), opening it, the start
  screen with its way out, and `Kept`, which watches the app's lifecycle for the keeper
  and closes the document when the screen goes. `app/lib/example.dart` makes the copy of
  the example; `app/lib/demo_screen.dart` is the screen of milestone B1, moved.

**The bridge grew by what the reviews asked for**, not by what the screen needs to draw:
the screen reads one grid, applies a command, undoes, redoes, saves, overwrites and closes.
The trail is kept on the Dart side.

## Against `UI_SPEC.md`

Taken from it: one stage (section 3), the title and trail bars (4 and 5), tap text to edit
and tap the child grid to dive, and a tap outside the open editor only closing it (6), one
editor at a time with Return moving down (7), the system's back as "up one level" (10), no
Save for documents, and targets of at least 48 dp.

Not in it, and here: the cross that closes the editor without entering its text (A147).

Left out, all of it for the real screen: the Full, Preview and Shape forms of a nested grid
(a nested grid is always the "Open ›" target, with no count); the first row and column
staying in place; the dive animation; restoring scroll position and selection per level;
the keyboard toolbar; adding rows, columns and grids; the cell menu; gestures; Clean mode;
search; tablet and desktop layouts; accessibility work; colours, styles, alignment, images,
notes and links of cells, none of which are drawn; drawing only the cells on screen;
choosing, making or naming a document; Android's animation that shows where back leads.

## Tests

- `app/test/keeper_test.dart` (22 tests, no device): on the stand-in for the core,
  with the clock of the test: saving a second after the first change, trying again after a
  failure and how often, the app put away, closing, a close that cannot save, letting go,
  and the other program's file; on the real core, that a file can be read while it is open
  and that nothing is left of the look.
- `app/test/stage_test.dart` (41 tests, no device): on the real core, the root, into a
  grid and out, jumping along the trail, typing with the report and the recalculated
  formula, undo and redo, making and unmaking a formula cell, the title and the trail
  following a rename, a stage taken away by undo, a refused command, what a redo makes,
  text entered in place of the command before it, steps asked for at once, saving by
  itself and when asked, and a file that another program changed, kept, given up, or
  unreadable. On a stand-in for the core that can be made to fail
  (`app/test/fake_document.dart`): a save that fails, a grid that cannot be read after an
  edit or on the way in or up, and a fault in the program.
- `app/test/grid_screen_test.dart` (71 tests, no device, the widgets on the stand-in,
  which can also be held up): what a formula cell looks like, the tints, the names where
  the document has none, actions that come while the core is at work, the system's back,
  every way typed text used to be thrown away, Return and the keyboard, saving after a
  second and when the app is put away, the cross and the tick after that, a screen that
  goes with its editor open, the lines for a save that failed and their buttons, and the
  sentence after a fault.
- `app/test/grid_screen_real_test.dart` (7 tests, no device, the widgets on the real
  core): keep mine, open theirs, open theirs with a file that cannot be read, an entering
  that the core refuses, the app out of sight and the cross, one editing with the app away
  in the middle, and a grid that could not be read again.
- `app/test/main_test.dart` (12 tests, no device): where the document is kept on each
  system, the copy of the example, the document going with its screen, and the app itself
  on the real core: the first start, the next start with what was typed, a document that
  cannot be opened with the way out, and one that is merely open already.
- `app/integration_test/grid_test.dart` (the real app, here on Linux under Xvfb): that the
  document is a copy with IDs of its own, into "Costs", type and enter, the file having
  the edit a moment later, cancel, Return to the cell below, back by the trail with the
  editor open, the editor of a formula cell, undo to the end with its message, redo, up by
  the arrow and by the system's back, the app going out of sight with something half
  typed and the cross after it, the bridge check from the menu, and the app ended with
  the editor open and started again on the document as it was left.

CI runs all of them.

The on-screen test found two faults that the logic tests could not, when the screen was
first built: Return saved but did not move the editor down (the move was dropped because
the save was still counted as running), and the pop-up message for "Nothing to undo" lay
over the trail bar and swallowed taps on it. Both are fixed, and the test checks both.

## Not verified

- **How it looks, and how it feels.** No person has seen the screen: taking a picture of
  the window under Xvfb gave an empty image. The test finds widgets and reads their text;
  it does not judge layout, sizes or colours.
- Anything running on Android, iOS or a phone; the on-screen keyboard; touch; what a phone
  really does when the app is put away or ended (the saving was tried with the lifecycle
  events of the test binding, on Linux); Android's back button and edge swipe (the test
  sends what Flutter makes of them); whether "next" on a phone's keyboard behaves as
  Return does here. The app with this screen builds for Android (the debug package in CI);
  `docs/ANDROID.md` says how to run it there.
- macOS and Windows: not built, not run; where the document is kept there was worked out
  from their conventions and tested with made-up environments only (A156).
- The app ended outright, without being put away first: what is in the editor is lost, and
  up to a second of entered changes (A145). The same when it is ended while only
  `inactive`.
- A large grid. All of a grid's cells are built at once; with a thousand cells that is
  untried, and with many more it will be slow.
- Text that needs more room than six lines is cut off with an ellipsis and can only be
  read in the editor, which has one line.
- Two screens on one document. `Keeper` is made for it and nothing uses it that way; what
  A157 says about the cross with a second screen has no test.
