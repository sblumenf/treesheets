# A simple grid screen (UI1)

Status: **built, and run as a Linux desktop app by a test; nobody has looked at it or
touched it.** This is not a milestone of `PLAN.md` and not the screen that `UI_SPEC.md`
specifies. It is a first, small step towards that screen, built so that the bridge is used
by something that draws a grid, and so that the Android build and the measurements have
something real to run. It was built before the independent review of the bridge
(`NEXT_REVIEW.md`) was done. Three reviews and an evaluation changed it: it reaches the
document through `Session` and a `Keeper` that saves, keeps what is typed, and takes its
actions one at a time (decisions A144 to A148 and A154 to A159; `B1.md`, "Fixes after
review", has each finding with its test). A fourth review was of what is under it
(`Session`, `InTurn`, `Keeper`, `main.dart`; A160 to A163), which the real interface keeps
when this screen, a stand-in, is replaced: `lib/grid/`, `GridOnKeeper` in `main.dart`, and
the tests named below as the screen's go then. For the first screen of the real interface
those layers then gained the stage read, the editing of a cell and the view state (plan
step 1, A175 to A180; `B1.md`). Of that the stand-in uses two things, so that they have a
user before the real screen exists: it edits through the keeper's `Editing`, in place of
its own workaround for the cross (A157, replaced by A177), and its text field is the
`CellEditor` (A180). It still reads one grid at a time and keeps its own trail. This page
says how it is now.

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
  goes through the keeper's editing (A177): only the part of the text that differs is
  replaced, so the styles and links of the rest stay (none of which this screen draws),
  and a text that starts with `=` makes a formula cell. The editor keeps the keyboard
  while the core is at work on Return. The field shows up to five lines (a text may have
  line breaks, A174); this screen has no key that makes one.
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
  that is not saved yet, at once when the app is no longer in front, and when the app
  goes. When the app goes out of sight, what is in the editor is entered as well; when it
  only stops being in front (a window that loses the focus), it is not.
- **Closing the window waits for the document** (A162): what is in the editor is entered,
  the document saved and closed, and then the app exits. If the document cannot be saved,
  the window stays once and the line below says why; closed again, the app exits.
- **The cross still means "not this" after the app was away** (A177): the text that went
  into the document then is held there, in a group of commands that the cross cancels and
  the tick ends. After the cross the document, what can be undone and what can be redone
  are exactly as before the editor opened; after the tick the editing is one step to undo.
  While text is held so, nothing else can change the document: the keeper refuses it.
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
and at every later start that same file as it was left. The file is `documents/example.wombat`
in the app's own data folder on every system (A165): on Linux
`~/.local/share/io.github.sblumenf.burrow/documents/` unless `XDG_DATA_HOME` says
otherwise, on Android the app's `files` folder. If what is there cannot be opened at a
start, the app says so and offers to try again or to put it aside, with its pictures and
view states, and start with the example (A163).

## How it is put together

- `app/lib/session.dart`: `Session`, the open document as the app holds it: every call to
  the core in the order it was made (A144). Nothing else touches the bridge's handle.
  `InTurn`, which does the ordering there and in the three places below, reports an error
  that nobody waited for and refuses a piece of work that waits for its own queue, or for
  a queue that waits for its own (A154, A160).
- `app/lib/keeper.dart`: `Keeper`, which owns the `Session` for every screen on the
  document and is the one place that saves: after a change, again after a failure, when
  the app is put away, at the close. It says how the document stands with its file
  (`state`), holds "another program changed the file" and "ended after a fault", and is
  what a screen attaches itself to, to be told and to be asked to finish what is being
  typed (A155, A161). It also has what the real screen will draw from and edit through
  (`StageReader`, `Editing`, the view state; A176 to A178), of which this screen uses the
  editing.
- `app/lib/grid/stage.dart`: `Stage`, the logic with no widgets: which cell is on the
  stage, the trail, each action as a call through the keeper followed by reading the stage
  again, and a cell's editing opened, held, entered and dropped (`edit`, `holdText`,
  `enterText`, `drop`). It keeps nothing of the document but what it last read, and does not save. Its
  steps are taken one at a time in the order they were asked for, each judged by the stage
  as it is then (A146). If the cell on the stage is gone after an undo, it gives way to
  the nearest cell above that is still there; any other error leaves the stage where it
  was.
- `app/lib/grid/grid_screen.dart`: the widgets, and one queue for everything the user does
  (A158). It attaches itself to the keeper with that queue and with how to hold what is
  in the editor, for when the app is put away or closed; when the screen itself goes, it
  enters the text, which ends its editing. Its text field is a `CellEditor`
  (`app/lib/cell_editor.dart`).
- `app/lib/main.dart`: where the document is (`documentFolder`), opening it
  (`openDocument`), putting aside what cannot be opened (`putAside`), the start screen,
  which shows on the keeper whatever screen it is given (`GridOnKeeper` for this one: it
  opens the stage and has the bridge check in its menu), and `Kept`, which watches the
  app's lifecycle and its exit for the keeper; the start screen closes the document when
  it goes. `app/lib/example.dart` makes the copy of the example;
  `app/lib/demo_screen.dart` is the screen of milestone B1, moved.

**This screen draws from little of the bridge**: it reads one grid at a time, and keeps
the trail on the Dart side. What the bridge has for the real screen since plan step 1
(the stage and everything nested below it in one read, the view state) it does not use.

## Against `UI_SPEC.md`

Taken from it: one stage (section 3), the title and trail bars (4 and 5), tap text to edit
and tap the child grid to dive, and a tap outside the open editor only closing it (6), one
editor at a time with Return moving down (7), the system's back as "up one level" (10), no
Save for documents, and targets of at least 48 dp.

Not in it when this was built, and here: the cross that closes the editor without entering
its text (A147). The specification has since been given the cross (its section 7).

Left out, all of it for the real screen: the Full, Preview and Shape forms of a nested grid
(a nested grid is always the "Open ›" target, with no count); the first row and column
staying in place; the dive animation; restoring scroll position and selection per level;
the keyboard toolbar; adding rows, columns and grids; the cell menu; gestures; Clean mode;
search; tablet and desktop layouts; accessibility work; colours, styles, alignment, images,
notes and links of cells, none of which are drawn; drawing only the cells on screen;
choosing, making or naming a document; Android's animation that shows where back leads.

## Tests

What is kept when this screen is replaced:

- `app/test/session_test.dart` (33 tests, no device): the session, and the
  queue by itself (`B1.md` lists them).
- `app/test/keeper_stage_test.dart` (36), `keeper_editing_test.dart`
  (28), `stage_read_test.dart` (13), `cell_editor_test.dart`
  (8) and `fake_document_test.dart` (4): what plan step 1
  added under every screen (`B1.md` says what each holds).
- `app/test/keeper_test.dart` (68 tests, no device): on the stand-in for the
  core, with the clock of the test: saving a second after the first change, trying again
  after a failure and how often, how the document stands with its file and what listeners
  are told, what is attached, the app put away, closing and what cannot finish, every
  call after the end, with no one left to ask, the other program's file, and a fault. On
  the real core: a change in the file soon after, closing, keep mine, open theirs, theirs
  unreadable, a file that nobody changed, after a fault, and that a file can be read
  while it is open and nothing is left of the look.
- `app/test/main_test.dart` (36 tests, no device, the app on `ProbeScreen`): where
  the document is kept, the copy of the example, the first start and the next with what
  was typed, a document that cannot be opened and the way out (a file, a folder, a link
  that leads nowhere; with its pictures and view states), one that is merely open, one
  that another start made, putting aside twice in the same instant, the system's request
  to exit (all in the file first; held back once; never longer than five seconds), and
  the lifecycle. Its last group, three tests, is of the app with this screen.

What goes with this screen:

- `app/test/stage_test.dart` (45 tests, no device): on the real core, the root,
  into a grid and out, jumping along the trail, typing with the report and the
  recalculated formula, undo and redo, making and unmaking a formula cell, the title and
  the trail following a rename, a stage taken away by undo, a refused command, what a redo
  makes, text held for an editing and then entered or dropped, steps asked for at once, saving
  by itself and when asked, and a file that another program changed, kept, given up, or
  unreadable. On a stand-in for the core that can be made to fail
  (`app/test/fake_document.dart`): a save that fails, a grid that cannot be read after an
  edit or on the way in or up, a fault in the program, and a stage that is left.
- `app/test/grid_screen_test.dart` (74 tests, no device, the widgets on the
  stand-in, which can also be held up): what a formula cell looks like, the tints, the
  names where the document has none, actions that come while the core is at work, the
  system's back, every way typed text used to be thrown away, Return and the keyboard,
  saving after a second and when the app is put away, the cross and the tick after that
  (with what could be redone before, and a second writer refused meanwhile), a
  screen that goes with its editor open, the lines for a save that failed and their
  buttons, and the sentence after a fault.
- `app/test/grid_screen_real_test.dart` (7 tests, no device, the widgets on the real
  core): keep mine, open theirs, open theirs with a file that cannot be read, an entering
  that the core refuses, the app out of sight and the cross, one editing with the app away
  in the middle, and a grid that could not be read again.
- `app/integration_test/grid_test.dart` (the real app, on Linux under Xvfb and on the
  Android emulator in CI): that the document is a copy with IDs of its own, into "Costs",
  type and enter, the file having the edit a moment later, cancel, Return to the cell
  below, back by the trail with the editor open, the editor of a formula cell, undo to the
  end with its message, redo, up by the arrow and by the system's back, the app going out
  of sight with something half typed and the cross after it, the bridge check from the
  menu, the system asking the app to exit with the editor open (the text is in the file
  when the answer comes), and the app started again on the document as it was left. Each
  press waits until the screen shows what the test expects next.

CI runs all of them.

The on-screen test found two faults that the logic tests could not, when the screen was
first built: Return saved but did not move the editor down (the move was dropped because
the save was still counted as running), and the pop-up message for "Nothing to undo" lay
over the trail bar and swallowed taps on it. Both are fixed, and the test checks both.

## Not verified

- **How it looks, and how it feels.** One picture of the window was looked at, taken under
  Xvfb in the run that closed the window on an unsaved document (`B1.md`, "What was run,
  and where"): the grid, the line "Not saved: another program changed the file." with its
  two buttons, the trail and the editor were where they should be. (An earlier attempt
  gave an empty image; `xvfb-run -a -s '-screen 0 1400x900x24'` and `import -window root`
  give one.) Nobody has used the screen. The tests find widgets and read their text; they
  do not judge layout, sizes or colours.
- Anything on a phone or on iOS; the on-screen keyboard; touch; what a phone really does
  when the app is put away or ended (the saving was tried with the lifecycle events of
  the test binding); Android's back button and edge swipe (the test sends what Flutter
  makes of them); whether "next" on a phone's keyboard behaves as Return does here. The
  integration test of this screen has passed on an Android emulator in CI, as the app was
  before A160 to A165 (`B1.md`); `docs/ANDROID.md` says how to run it on a phone.
- macOS and Windows: not built, not run; where the document is kept there is the
  plugin's business (A165), and whether Windows asks before the window closes is not
  known (A162).
- The app ended outright, without being put away first and without its window being
  closed: what is in the editor is lost, and up to a second of entered changes (A145). The
  same when it is ended while only `inactive`.
- A large grid. All of a grid's cells are built at once; with a thousand cells that is
  untried, and with many more it will be slow.
- Text that needs more room than six lines is cut off with an ellipsis and can only be
  read in the editor, which shows five lines and scrolls.
- The editor's field of several lines under a real keyboard: whether a phone's keyboard
  shows and sends "next" for it, and what a hardware Return does on a desktop (A180). The
  tests send the action as Flutter's test keyboard does.
- Two screens on one document. `Keeper` is made for it and nothing uses it that way. That
  a second writer is refused while text is held (A177) is tested through the stage of
  this screen, not with a second screen.
