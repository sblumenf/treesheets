# The app

The Wombat app as far as it goes: a simple grid screen on one document, which starts as a
copy of the example, and behind a menu entry the plain-text check of the bridge. It calls
the Rust core through `../crates/wombat-bridge`. It is not yet the interface that
`../docs/UI_SPEC.md` specifies.

- The grid screen: `../docs/milestones/UI1.md`.
- The bridge, how to run and test, what is left to do and what is still open:
  `../docs/milestones/B1.md`.
- What is generated and how to regenerate it: `../AGENTS.md`.

Three things to know before changing it:

- **A document is reached through `Session`** (`lib/session.dart`) and nothing else. It
  sends the calls to the core in the order they were made; the generated `OpenDocument`
  underneath does not. Its comments say what the calls do and refuse.
- **There is no Save.** The document is saved a second after a change, when the app is no
  longer in front, and when its screen goes (`lib/grid/stage.dart`).
- **The document is kept in the system's temporary folder** (`wombat/example.wombat`
  there), which the system may clear. Where documents really live is not decided.

`flutter test` runs the tests in `test/` without a device; it builds the bridge's library
first. The tests in `integration_test/` run the built app, one file per call:
`flutter test integration_test/grid_test.dart -d linux` (under `xvfb-run -a` without a
display).
