# The app

The Wombat app as far as it goes: a simple grid screen on one document, which starts as a
copy of the example, and behind a menu entry the plain-text check of the bridge. It calls
the Rust core through `../crates/wombat-bridge`. It is not yet the interface that
`../docs/UI_SPEC.md` specifies.

- The grid screen: `../docs/milestones/UI1.md`.
- The bridge, how to run and test, what is left to do and what is still open:
  `../docs/milestones/B1.md`.
- What is generated and how to regenerate it: `../AGENTS.md`.

Four things to know before changing it:

- **A document is reached through `Session`** (`lib/session.dart`) and nothing else. It
  sends the calls to the core in the order they were made; the generated `OpenDocument`
  underneath does not. Its comments say what the calls do and refuse.
- **A screen reaches the document through its `Keeper`** (`lib/keeper.dart`), which owns
  the `Session` and is the one place that saves. **There is no Save**: the document is
  saved a second after a change, again by itself after a save that failed, when the app
  is no longer in front (`Kept` in `lib/main.dart` tells the keeper), and when its screen
  goes. A screen shows what the keeper says and does not save.
- **Everything the user does on the grid screen is one action in one queue**
  (`lib/grid/grid_screen.dart`), and an action looks at the editor and the stage when its
  turn comes, not when the finger came down. A new control goes through `_do`.
- **Where the document is kept**: `wombat/example.wombat` in the user's data folder on
  Linux, macOS and Windows (`documentFolder` in `lib/main.dart`), and in the app's
  temporary folder, which the system may clear, on Android and iOS. Where documents
  really live, and choosing one, is not decided.

`flutter test` runs the tests in `test/` without a device; it builds the bridge's library
first and asks cargo where it is. The tests in `integration_test/` run the built app, one
file per call: `flutter test integration_test/grid_test.dart -d linux` (under `xvfb-run -a`
without a display).
