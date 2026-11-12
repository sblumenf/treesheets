# The app

The Wombat app as far as it goes: a simple grid screen on one document, which starts as a
copy of the example, and behind a menu entry the plain-text check of the bridge. It calls
the Rust core through `../crates/wombat-bridge`. It is not yet the interface that
`../docs/UI_SPEC.md` specifies: the grid screen (`lib/grid/`) stands in for it, and what is
under the screen (`lib/session.dart`, `lib/keeper.dart`, `lib/stage_read.dart`,
`lib/cell_editor.dart`, `lib/main.dart`) is what that interface will be built on. Its application ID is `io.github.sblumenf.burrow`; the name it
shows is Wombat.

- The grid screen: `../docs/milestones/UI1.md`.
- The bridge, how to run and test, what is left to do and what is still open:
  `../docs/milestones/B1.md`.
- What is generated and how to regenerate it: `../AGENTS.md`.

Seven things to know before changing it:

- **A document is reached through `Session`** (`lib/session.dart`) and nothing else. It
  sends the calls to the core in the order they were made; the generated `OpenDocument`
  underneath does not. Its comments say what the calls do and refuse.
- **A screen reaches the document through its `Keeper`** (`lib/keeper.dart`), which owns
  the `Session` and is the one place that saves. **There is no Save**: the document is
  saved a second after a change, again by itself after a save that failed, when the app
  is no longer in front and before the app exits when its window is closed (`Kept` in
  `lib/main.dart` tells the keeper), and when the app goes. A screen shows what the keeper
  says (`Keeper.state`) and does not save.
- **A screen draws from a `StageReader`** (`Keeper.read`): the stage, the trail and every
  grid nested below it in one read, which the keeper reads again after every change and
  compares, so that the screen is told which cells to draw again. Where the screen is
  (level, scroll position, selection, folds) belongs to the device; the reader is told and
  the keeper writes it. The stand-in grid screen does not use this yet.
- **A cell's text is edited through an `Editing`** (`Keeper.edit`) and a `CellEditor`
  (`lib/cell_editor.dart`): nothing is sent while the user types, the tick makes one step
  to undo, the cross leaves no trace, and Return is the screen's action, not a line
  break.
- **A screen attaches itself to the keeper** (`Keeper.attach`) with the queue of its
  actions and with how to hold what is being typed, and detaches when it goes.
  `../AGENTS.md` has the short list of what a screen must do; `test/probe_screen.dart` is
  the least screen that does it.
- **Everything the user does on the grid screen is one action in one queue**
  (`lib/grid/grid_screen.dart`), and an action looks at the editor and the stage when its
  turn comes, not when the finger came down. A new control goes through `_do`.
- **Where the document is kept**: `documents/example.wombat` in the app's own data folder
  on every system, which the `path_provider` plugin finds (`documentFolder` in
  `lib/main.dart`, decision A165). On Linux that is
  `~/.local/share/io.github.sblumenf.burrow/documents/` unless `XDG_DATA_HOME` says
  otherwise; on Android the app's `files` folder. Tests give the app a folder of their
  own (`WombatApp(folder:)`), so none writes there. Choosing, making and sharing documents
  is not designed.

`flutter test` runs the tests in `test/` without a device; it builds the bridge's library
first and asks cargo where it is. The tests in `integration_test/` run the built app, one
file per call: `flutter test integration_test/grid_test.dart -d linux` (under `xvfb-run -a`
without a display).
