# The first slice of the interface: the phone screen (UI2)

Status: **built; run by tests without a device, in the built Linux app and, by its test
of the built app, on an Android emulator in CI; looked at in pictures of the Linux app at
the size of a phone; never used by a person, and nothing of it has run on a phone or on
iOS.** It is step 5 of the way to the first test on the owner's hardware
(`PLAN.md`): `UI_SPEC.md` sections 3 to 8 (the depth budget, the phone screen, dive and up,
tapping, the editor with its toolbar, adding rows and columns), with the prototype's
variant switches, its log and its test documents, covering flows 1, 2, 3, 7, 9 and 10 of
section 17. It replaces the stand-in grid screen (`UI1.md`), which is deleted. Decisions
A181 to A195 (A191 is what the screen's first run on the emulator showed of its test,
A192 the iOS project, A193 to A195 what the fifth review led to). It was reviewed
independently once, by the fifth review ("After the fifth review" below). `PHONE_TEST.md` is the list for
the owner's test.

## What it does

- **The stage's grid, scrolling both ways**, with the text and the colour of the level's
  cell as the title. Only the rows near what is seen are built. The first row and the
  first column stay in view (a switch).
- **A grid inside a cell is drawn by the depth budget** (A182): cell by cell where its
  columns each hold a short word (Full), as its first texts in small print with its size
  (Preview), as a miniature of its rows, columns and colours (Shape), or as a count
  (Badge, for a grid that is folded on this device). Larger text of the system makes
  grids step down sooner. Cells are drawn with their colours, their styles, the ranges
  styled otherwise, their links, their alignment and relative size; a formula cell shows
  its result; text from right to left is laid out so.
- **Dive and up.** A tap on a Preview, Shape or Badge goes into that grid; "Open ›" goes
  into the grid of the cell that is edited or selected. The arrow of the trail bar, a
  level's name there, and the system's back go up. Each level comes back where it was
  left: the row and column at the top left, and the selected cell (the core's view
  state, also after the app was closed). The level is drawn growing or shrinking into
  place, and the cell the screen came up out of flashes once.
- **Tapping.** A cell's text: the editor opens on it. With the editor open, a tap
  anywhere else only closes it, keeping what was typed. The other variant selects first.
- **The editor**, docked at the bottom above the keyboard, with the cross and the tick
  beside it and a toolbar under it: Aa (Bold, Italic, Link, Tag), the line break, →,
  +Row, +Col and Open ›. Return saves and moves to the cell below, → to the right; at the
  edge a row or column is added. Nothing is sent while typing: the cell that is edited
  is drawn from the editor. One editing is one step to undo; the cross leaves no trace,
  also after the app was away (A177).
- **Rows and columns** by Return and → at the edge, +Row and +Col, the "+" bars after the
  last row and column, and "+ Add" of the bottom bar beside the selected cell. A document
  that starts empty offers its first cell.
- **Undo and redo** in the title bar.
- **No Save**: the word under the title says how the document stands with its file, and
  a line that stays says what to do when it could not be saved or another program
  changed the file ("Keep mine", "Open theirs"), as on the stand-in (A155, A159).
- **The prototype's settings** behind the menu: the two variant switches of the
  specification, the switch for the first row and column, the two thresholds of the depth
  budget, and the log, which can be exported as a file or copied (A187).
- **Five test documents** behind the menu (A188): the example, "Depth forms", "Six levels
  (12×20)", "Large (100,000 cells)" and "Empty".

## How it is put together

`app/lib/phone/`, on `Keeper`, `StageReader`, `Editing` and `CellEditor`:

- `phone_screen.dart`: `PhoneScreen`. The one queue of the user's actions, the reader, the
  editing and the editor; every action (tap, dive, up, back, Return, →, +Row, +Col, the
  bars, the styles, undo, redo, keep mine, open theirs); the title bar, the lines for
  trouble and messages, the trail, the bottom bar; what the keeper asks when the app is
  put away; what happens when the screen goes.
- `stage_view.dart`: `StageView`, the grid of the stage: the rows as a lazy list laid out
  from the remembered row, the first row and column kept in view, the "+" bars, scrolling
  the edited cell into view, and saying where the view is.
- `cell_view.dart`: `CellView` (a cell: its text, and its grid in one of the four forms),
  `CellText` (the text with its styles, runs and links), `PreviewView`, `ShapeView`, and
  `Scene`, what every cell of one drawing shares.
- `budget.dart`: `formOf`, which form a grid gets, and `columnWidths`, how wide the
  stage's columns are.
- `editor_bar.dart`: `EditorBar`, the field with the cross and the tick and the toolbar.
- `place.dart`: where a cell is in a stage read, which ranges of a text are styled, the
  editor's selection as a range of code points, and the request for an unfolded grid.
- `settings_sheet.dart`, `variants.dart`, `log.dart`: the prototype's sheet, its switches
  and thresholds, and its log.
- `words.dart`: every text the screen and the start screen show.

Beside it: `app/lib/test_documents.dart` makes and opens the test documents;
`app/lib/main.dart` shows the phone screen on the app's document and opens the others
from its menu; `app/lib/measure.dart` gave up its two document builders to be used for
that (`largeTable`, `writeDocument`).

**What the screen needed from the bridge and did not have: nothing.** Everything is a
read of the stage or a command that was there. No Rust was changed.

## Against `UI_SPEC.md` sections 3 to 8

Built as written, or with what "Decided when the first slice was built" in the
specification says: the four forms and their tap behaviour; the thresholds as settings;
the title with its colour stripe; the grid scrolling both ways with the first row and
column in view; text on top and the grid in a frame below, each at least 48 tall; dive
and up with the level's place and selection restored; the system's back; the trail; tap
to edit, tap a Full grid's cell to edit it, tap a Preview, Shape or Badge to dive; a tap
outside the open editor only closing it; both variant switches; one editor docked above
the keyboard; Return, →, the line break, +Row, +Col, Aa with bold, italic, link and tag,
Open ›, the tick, and the cross; the edited cell updating as one types; the "+" bars; the
log.

Built differently, and why (the decisions have the reasoning):

- **Full needs columns of 72 dp, not 48** (A182): at 48 every text broke after three or
  four letters, seen in the running app.
- **A cell with a grid is always two targets**; "a cell too small for both parts is one
  target" does not come about, because rows grow (A182).
- **Back with the editor open closes the editor only** (A184); the stand-in also went up.
- **The dive does not grow out of the tapped cell**: the level grows into place as a
  whole (A183).
- **A level comes back at a row, not at a distance** (A183): the core remembers a row
  and a column.
- **"Unfold in place" goes one level** (A185).
- **The toolbar** has the tick and the cross beside the field, and Aa opens a row, not a
  menu (A186).
- **The bottom bar** has no ⋯ of its own; undo, redo and the one ⋯ are at the top (A184).

Not built, though sections 3 to 8 name it:

- **The long press and the cell menu** (section 6 names it, section 9 specifies it), and
  with it "New grid inside", inserting above or to the left, and deleting rows and
  columns (section 8). Nothing deletes anything; undo takes back what was added.
- **"Find" in the title bar** (section 12), and Clean mode's hiding of the bars (11).
- **The trail's dots, its list of the whole way, and "recent places"** (section 5).
- **Jumps** (section 5): there is nothing yet to jump from.
- **The pulse of a jump's target**; the flash after a way up is built.
- Images and notes in cells, vertical alignment, a grid's draw styles, a grid beside its
  owner's text.

## Tests

Counts are of test cases as `flutter test` reports them.

- `app/test/phone_parts_test.dart` (27 tests, no device): the depth budget at each of its
  thresholds, the widths of columns, where a cell is in a read, which ranges are styled,
  the direction of a text, the pieces a styled text is drawn in and a cell's base style,
  the switches and their ranges, the log (its file, a file that cannot be written, too
  many entries, the export).
- `app/test/phone_screen_test.dart` (151 since the fifth review, no device, on the stand-in for the core,
  which can be held up and made to fail): what is shown (the level, the state, formulas,
  names where there are none, right-to-left text, several lines, a root of one cell, a
  root without a grid); each depth form on the screen; five sizes of screen with text at
  the usual size and twice that, and the screen turned while a cell is edited (without
  a keyboard: until the fifth review only one test, upright, gave the screen a
  keyboard's inset; four more do now, three of them on its side); taps (the
  editor, a Full grid's cell, a dive and the three ways back, a tap outside the editor in
  five places, both variant switches, the bottom bar, the first row and column kept in
  view); each level where it was left, and the flash; editing (drawn from the editor,
  the tick and the cross, messages, Return and →, text the core refuses, the line break,
  Aa, Open ›, the bars); more typed while the core is at work on the tick and on four
  actions of the bars; what is typed while the editor moves on (A193); actions held up behind one another; taps that come close
  together, on every button in a row, and a screen that goes while an action runs;
  saving after a second, when the app is no longer in front, hidden, paused and
  detached; the cross, the tick and the screen going after the app was away; every line
  for a save that failed, for a file another program changed, for a stage that could not
  be read and for a view state that could not be written; a fault; a root that cannot be
  read; the log's entries; the settings sheet.
- `app/test/phone_screen_real_test.dart` (36 since the fifth review, no device, on the real core): **flows
  1, 2, 3, 7, 9 and 10** (1 and 9 on the generated six-level document, 2, 3 and 7 on a
  small nested one, 10 on the example); Return in the last row as two steps to undo; →
  in the last column; +Row and +Col; the "+" bars; "+ Add"; a row added to an unfolded
  grid and to a Full grid that then is too large to stay Full; a document that starts
  empty, down to no grid again by undo; bold and italic on a selection and off again, on
  the whole cell, and taken back by the cross; a link on a selection, on all of the
  text, removed, and put aside; a tag on and off, and one the core refuses; line breaks,
  line ends of another system, right-to-left text; a formula and the cell that reads it;
  an entering the core refuses; keep mine, open theirs, theirs unreadable; the app out of
  sight with the file read back, and one editing with the app away in the middle; a
  stage that could not be read again; and the place of a level there again after the app
  was closed and started.
- `app/test/main_test.dart`, group "with the phone screen" (8 of its 41): the app showing its
  document and keeping what was typed when it goes, with the log in its folder; the test
  documents opened from the menu, each made once, with what was typed entered into the
  document that is left; the six-level and the large document being what they say; a
  test document that cannot be opened; where an exported log goes; the switches staying
  when another document is shown; the bridge check.
- `app/integration_test/grid_test.dart` (the built app; the name is the stand-in's,
  because the emulator job names the file): the example as a copy with IDs of its own;
  flow 10; the file having the edits a moment later; the cross; into "Costs" and back by
  the trail with the editor open; a formula cell's editor; flow 7 down to "Nothing to
  undo", and redo; the system's back; the app out of sight with something half typed,
  and the cross after it; the six-level document from the menu, with what was typed
  entered into the example first; flow 9; flows 1 and 2; flow 3; flow 7 again; up and to
  the top; the settings and the bridge check; the system asking the app to exit with the
  editor open; and the app started again on the document as it was left. It names no
  size and goes into a grid by its cell and "Open ›", so that it does not depend on
  which form a grid has on the screen it runs on.

**Where the stand-in's tests went.** The fourth review listed the tests that were the
only ones of something in the kept layers. Each assertion, and its home now:

| The stand-in's test | What it alone showed | Where that is shown now |
| --- | --- | --- |
| `stage_test.dart`, "after a fault the document is let go of and not used again" | `Keeper` ends the work at a fault; every call after the end is refused, `openTheirs` too | `keeper_test.dart`: "after a fault the document is let go of, and nothing saved"; "after … every call fails the same way" |
| `stage_test.dart`, "the refusal of every call after a fault ends the work too" | A save that meets the bridge's refusal after a fault ends the work | `keeper_test.dart`: "the refusal of save / overwrite / close after a fault ends the work too" |
| `stage_test.dart`, "a file that is changed again while it is opened ends the work" | `openTheirs` when the file cannot be opened after all | `keeper_test.dart`: "that is changed again while it is opened ends the work"; "a fault while it is opened is said as one" |
| `stage_test.dart`, "a root that cannot be read is an error, not a crash" | Nothing is left with the keeper by a screen that could not start | `phone_screen_test.dart`: "a root that cannot be read is said, and nothing is left behind"; `keeper_stage_test.dart`: "a stage that cannot be read at the start is no reader" |
| `stage_test.dart`, the rest of "saving by itself" on the real core | Saved soon after, undo too; keep mine; open theirs; theirs unreadable | `keeper_test.dart`, group "on the real core"; with the screen: `phone_screen_real_test.dart`, group "the file" |
| `grid_screen_test.dart`, "when the app is no longer in front, but not what is being typed" | `Kept` saves on `inactive` and does not ask screens to finish | `main_test.dart`: "saved when the app is no longer in front, without what is typed"; with the screen: `phone_screen_test.dart`, the test of the same name |
| `grid_screen_test.dart`, "at once when the app is hidden / paused / detached, with what is being typed" | `Kept` puts the document away at each of the three | `main_test.dart`, the tests for each state; with the screen: `phone_screen_test.dart`, the three tests of the same name |
| `grid_screen_test.dart`, "a screen that has gone is no longer asked or told" | `Kept` stops watching the lifecycle; the screen detaches | `main_test.dart`: "what has gone is no longer told that the app is put away"; `phone_screen_test.dart`: the test of the same name, which also shows that the reader is closed |
| `grid_screen_test.dart`, "text that the core refuses when the app leaves is kept and said" | A finisher that shows a refusal instead of throwing it | `phone_screen_test.dart`: the test of the same name |
| `grid_screen_test.dart`, the cross and the tick after the app was away (seven tests) | An editing held, then entered or dropped: histories, saves, a second writer refused | `phone_screen_test.dart`, group "the document is saved without being asked", the tests of the same names; `keeper_editing_test.dart` for the keeper's part |
| `grid_screen_real_test.dart`, "the app out of sight: the text is in the file, and the cross takes it out again" | The same on the real core, with the file read back | `phone_screen_real_test.dart`: the test of the same name |
| `grid_screen_real_test.dart`, "one editing with the app out of sight in the middle is one undo" | The same | `phone_screen_real_test.dart`: the test of the same name |
| `integration_test/grid_test.dart`, the lifecycle in the built app | The only run of it in the built app | The same file, "The app goes out of sight with something half typed" |
| `main_test.dart`, "with the stand-in screen" | The app showing its screen, and the screen leaving nothing behind | `main_test.dart`, "with the phone screen"; `phone_screen_test.dart` |

What the stand-in's tests said of its own logic (its `Stage`, its trail kept in Dart, its
tints) went with it. What they said of taps in any order and of text never thrown away
is said of the phone screen, test for test, in `phone_screen_test.dart`.

**Changes that no test would notice.** The new code (`lib/phone/`, and what this
work added to `main.dart` and `test_documents.dart`) was changed one thing at a time, 452
changes in all, and the tests run after each, in three passes with fixes between them. The
driver and its logs are not in the repository.

- **408 were noticed** by a test.
- **22 were of code that is no longer there**: the first pass showed that it did nothing
  (a check before a read whose own failure covers it, a frame asked for that scrolling
  asks for anyway, guards for things that cannot be missing where they stood, the held
  cell cleared in four places where one sets it), and it was removed.
- **6 did not compile** (four of them were made again in a form that does, and noticed).
- **16 were not noticed**, and are left:
  - *The same behaviour by another way* (2): which of typed text and a formula's result
    is taken first in `CellText` (the result is not looked at for typed text anyway); a
    row kept alive out of sight being passed over when the top row is looked for (its
    place comes out as not-a-number, which no comparison takes for the top).
  - *Guards that the screen cannot reach by its own buttons* (6): "+" bars and "Open ›"
    on a stage or a cell without a grid, Return without an editor, "Edit" of the bottom
    bar with the editor open, a remembered column that the grid no longer has (the core
    keeps it inside the grid), and the screen being keyed by its keeper in `main.dart`.
  - *Depends on timing that a test here cannot fix* (4): the keeper's question being run
    in the turn of the screen's queue (said here to give the same result as the
    keeper's own order "in every case that was built". **That was wrong**: the fifth
    review put the app away while Return was at work, and without the queue the text
    of the next cell was held in the cell that was left. The queue is needed, and the
    test "the app put away while Return is at work" now notices its removal); the log being read before entries made meanwhile are
    written; the document put away before another is shown (the close that follows
    wrote the place too, in every run); a second tap on "Add the first cell".
  - *Not tested* (4): the editor given back a text that the core kept differently (it
    takes half a surrogate pair to make one); the selection being taken before the link
    dialog opens and not after (two changes); and the styles drawn for text typed while
    the tick was at work.

The first pass also found three faults that are fixed: a row kept alive by a tap's
ripple was measured where it had last been, so that turning the phone could leave the
edited cell out of sight; the place of a level was read before the rows were laid out
where scrolling had ended, and handed to the wrong view when a level changed; and three
taps on switches in the settings without a frame between them each started from the
settings as last drawn.

A fourth was found when the screen first ran on the emulator of CI, by reading the
screen's code for what differs at that size, and not by the run itself: **a cell edited
in the first row was not scrolled into view** while that row is kept in view, where it
scrolls in two fifths of the height. In the example that is "Bread" and "Cheese" of the
shopping list as soon as the keyboard is up. The row is now scrolled to the edited cell
like the list below it; the test has the emulator's screen (1080 by 2400 at 420 dpi) and
a keyboard's inset, and failed first with the cell 53 logical pixels below what was
seen.

## What was run, and where

All of it on the cloud machine these documents were written on (Linux x86-64, 4
processors, Flutter 3.47.9, no Android SDK, no Mac), on the commits of this work:

- `cargo fmt --all -- --check`, `cargo clippy --locked --workspace --all-targets -- -D
  warnings`, `cargo test --locked --workspace --no-fail-fast`: passed (no Rust changed).
- `dart format --output=none --set-exit-if-changed lib/*.dart lib/phone test
  integration_test test_driver`, `flutter analyze`: clean.
- `flutter test`: 441 tests, passed (440 before the test of the fourth fault above).
- Each of the three files of `integration_test/` under `xvfb-run -a … -d linux`: passed.
  `grid_test.dart` was also run once with the app's window made 412 by 915, the size of
  the emulator's screen in CI, which is how a drag that started outside what a phone
  shows was found.
- **Pictures of the running Linux app**, with its window made 420 by 900 under Xvfb, by a
  driver that is not in the repository: the root of the example, the editor open and
  with the row of styles, a dived level, a line break being typed, the menu, the
  settings, "Depth forms" with each form, the same with the smallest text raised so that
  Previews become Shapes, the six-level document at its root, scrolled sideways, with a
  grid unfolded in place, dived once and twice, the editor two levels down, the large
  document and the empty one. The set was taken three times as the screen changed; every
  picture was looked at in at least one set, and seven of the last. What was found and
  changed: the
  banner of the debug build lay over the menu; the frame of a Full grid was painted over
  by its cells; a Badge's count was cut in a narrow cell; and Full grids with columns of
  48 dp broke every text after a few letters (A182). No overflow stripes and no clipped
  bars were seen in the last set.

**Not run there:** the Android build, the emulator and anything of iOS, which that machine
cannot do; anything on a phone.

**In CI** (`B1.md`, "What was run, and where", has the runs in full):

- *Run 53* (43347537203, commit `009486b`), the first with this screen: the Linux job
  passed, and the Android package built. The emulator job failed in `grid_test.dart`
  after seven seconds, at the third cell of flow 10: the cell showed "Cheese" where
  "Butter" had been typed and ticked. The cause was the test and Android's keyboard, not
  the screen (A191). In the same run a random test of the core failed
  (`sorting_is_a_stable_permutation_of_rows`): the test compared stored text where the
  sort compares what cells show (A66). The test was corrected (`dcff413`); the core is
  unchanged, and nothing of it touches this screen.
- *Run 54* (43352269035, commit `4af64c7`): **all five jobs passed.** 441 tests without a
  device; the three files of `integration_test/` on the built Linux app; and on the
  Android emulator (job 131645914091: x86-64, Android 14, 1080 by 2400, debug build)
  `app_test.dart`, **`grid_test.dart`, which is this screen's first pass on Android**,
  and the measurement twice. `grid_test.dart` took 2 min 9 s there with building and
  installing. The emulator's times for the read the screen draws from are in `B1.md`
  ("Measurement reports"): about 265 ms for the whole stage of the 1,000-cell root of
  the large document, in a debug build.
- **What the emulator run does not show.** The test is its own keyboard there (A191),
  so the on-screen keyboard never came up: what it covers, its Return key and what the
  back gesture does with it up are as untried as before. The test presses inside
  Flutter; nobody looked at the emulator's screen, and no picture of it was taken.
- **iOS**: the job builds the app, with this screen in it, for a simulator and runs only
  the bridge check there (`app_test.dart`; `docs/IOS.md`). The phone screen has never
  run on iOS, on a simulator either.
- The Android job keeps the package of every run as the artifact
  `wombat-android-arm64-debug`, which is what `PHONE_TEST.md` has the owner install.

## Not verified

- **Anything on a phone**: touch, the on-screen keyboard and what it covers, whether its
  Return key says and sends "next" for a field of several lines (A180), the back button
  and the back gesture with the keyboard up, what Android does to the app out of sight,
  how fast a debug build scrolls and reads on a phone's processor, the 100,000-cell
  document there, the export folder (A187), the phone's own text size and display size.
- **The on-screen keyboard, on the emulator too.** `integration_test/grid_test.dart`
  failed in its first run there and has passed since ("In CI" above), but as its own
  keyboard (A191): the emulator's keyboard is not connected while it runs. So that the
  edited cell stays in sight above a keyboard is known only from the test without a
  device that gives the screen a keyboard's inset.
- **iOS**: the screen has not run there at all; only the bridge check has, on a
  simulator (`docs/IOS.md`).
- **How it feels.** Nobody has used it. The pictures show layout, not whether a tap lands
  where it was meant, nor how the dive looks in motion.
- **The read of a very large stage on a phone.** The screen reads the whole grid of the
  stage after every change (A181). For the root of the large document that was about 250
  ms in a debug build here.
- **Dark theme and contrast** (section 14): the grid is drawn in the document's own
  colours on a light theme; nothing was checked against WCAG.
- **A screen reader**: buttons have labels and a nested grid says "contains a … by …
  grid"; nothing was run with TalkBack.
- macOS and Windows: not built anywhere. iOS: built in CI for a simulator (A192), with
  this screen never shown there.

## After the fifth review

The fifth review (independent; the Dart wiring of the wider bridge and this screen) found
five things, each fixed with a test that failed first:

- **Keys typed while the editor moved to the next cell were lost, or joined the cell that
  was left** (high). What is typed after Return, →, +Row or +Col is now the next cell's
  (A193). Tests: the group "typed while the editor moves on" in `phone_screen_test.dart`
  (each of the two moments of Return and of →, two Returns in a row, text the core
  refuses, no cell to move to, the app put away, the cross, a tick before it), and three
  on the real core in "rows and columns" (typed while the new row is made, two Returns
  in the last row without waiting, typed right after +Row and +Col).
- **One grid unfolded in place made every read of the large document 101,000 cells**
  (medium). Not unfolded where the read would grow by more than 500 cells (A194); two
  tests on the real core, one on the large test document.
- **On its side with the keyboard up the grid had 25 pixels and the field scrolled out
  of sight** (medium-low). The title bar, the trail, a row of buttons and the first row
  kept in view give way (A195); four tests at the review's sizes.
- **Two test documents chosen one right after the other** left one open unseen and
  refused the other (low). They are shown one after the other (`main_test.dart`).
- Small: a comment that named a file that does not exist; a test for what a making of
  "Depth forms" that was stopped half-way leaves behind.

Its random taps on the real core (`rv5_monkey_test.dart` in the review's scratch folder,
12 runs of 140 actions) were run three times on the fixed screen and passed each time.
They are not part of the suite: they wait on real time and take about 70 seconds.

Run on the tree with these fixes, on the same machine as above: the format check and
`flutter analyze` clean; `flutter test` four times, 464 tests: three runs passed, and
one had a single failure, in `phone_parts_test.dart` "the prototype log keeps the newest
entries when there are too many", which did not fail again in six runs of that file
alone (that test does not wait for the log's last write before its folder is deleted,
which is the likely cause; it is not changed here); each of the three files of
`integration_test/` under `xvfb-run -a … -d linux`, passed. No Rust changed. Nothing of it has run on a phone, an emulator or iOS: CI will say for the
emulator when the branch is pushed.

## Still open

- **Deleting**: rows, columns, grids, cells. The commands exist; the cell menu that
  offers them is section 9.
- **A grid inside a cell** can only be made in an empty document.
- **The window of rows** for very long stages (`phoneWindowAsk`, A176), if a phone shows
  that the whole read is too slow.
- **The switches are forgotten at every start, and every start shows the example**
  (A187, A188).
- **A Full grid needs its cells given by the read**: at most 24 cells, one level down. A
  wide screen could show more levels cell by cell (section 13) with another request.
- **The edited cell is drawn plain once its text is changed**, until the text is held in
  the document again, and the editor's own field never shows styles (A177, "Left").
- **Text in a cell is cut at eight lines** unless the cell is edited.
- **The first row kept in view** takes up to two fifths of the height when it is tall,
  also with the keyboard up: on a phone that is then about 150 logical pixels, three
  lines of a nested grid, and the rest of the row is reached by scrolling it (a cell
  edited there is scrolled to). It can be switched off.
- **A list anchored at its last rows** (after the edited cell was scrolled into view by
  starting the list there) shows empty space below them until it is scrolled.
- The messages of the core and of `StateError`s are English and not in `words.dart`.
- **A request that names the grids to give cell by cell.** The bridge's request has one
  bound for all grids of a stage, which is why unfolding in place is refused where many
  grids are near the size of the one unfolded (A194). It needs a change to the bridge.
- **For the owner to judge** (the fifth review concluded these by reading; none is
  changed):
  - A double tap on a cell opens the editor and closes it again, and so does a second
    tap on "Add the first cell": each tap is judged when its turn comes (section 6 as
    written).
  - Undo and redo act on the whole document while the screen shows one level: a step
    that changed another level is undone with nothing to see and nothing said.
  - Undo with something typed in the open editor enters it and undoes that. Redo with
    something typed enters it, which empties the redo history, and then says "Nothing
    to redo".
  - An action that takes more than two seconds keeps the app's leaving from holding the
    editor's text in time (B1, "given two seconds"); switching the test document then
    can lose it.
- **Not noticed by any test** (the fifth review's sample): the input method's text under
  way not being made final before the text is read (no test types with one); the
  document not being put away before another is shown; `CellEditor.selection` with no
  selection at all.
- **A screen taken away in the middle of a move of the editor** enters the text of the
  cell that is left; what was typed for the next cell is lost with the screen (A193).
- Everything `B1.md` lists under "Still open after the fixes" that is not a screen's.
