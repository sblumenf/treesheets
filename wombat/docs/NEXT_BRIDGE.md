# Next piece of work: a minimal bridge between Flutter and the Rust core

**Status:** built and run on Linux; the Android build, the measurements and the independent
review are still to do. See `milestones/B1.md`, which says what exists and what is left. The
rest of this note is the brief as it was written.

This note is for whoever starts the bridge work, person or AI, in a new session. It says what to
build, what not to build, and what "done" means. Read `../AGENTS.md`, `POC_SUMMARY.md`,
`DECISIONS.md` (at least A1 to A15 and A121) and `UI_SPEC.md` section 2 first.

## Where things stand

- The Rust core is finished as a proof of concept: documents, editing with undo, links, formulas,
  search, import and export. It has no interface. `POC_SUMMARY.md` says what works and what does
  not.
- The interface will be Flutter, calling the core through `flutter_rust_bridge` (decision A121).
  `UI_SPEC.md` specifies it, phone first. Its prototype uses an in-memory model behind a command
  interface, which the Rust core replaces later.
- **Nothing connects Flutter to the core yet.** There is no Flutter project in the repository and
  no binding code. Commands and results are plain data shaped to cross a language boundary, but
  they have no serialised form.

## The goal

Prove the plumbing end to end with the least code: a bare Flutter app that opens a real document
through the Rust core, shows it as text, changes it, and undoes the change. No real interface.

The nested-grid screen in `UI_SPEC.md` is the piece of work **after** this one. It will grow the
bridge as it needs to; do not try to anticipate it here.

## What to build

1. **A bridge crate** (for example `crates/wombat-bridge`) that depends on `wombat-core` and exposes,
   through `flutter_rust_bridge`, only this:
   - open a document from a path, and create a new one; get its ID;
   - read one grid: for a cell ID, its size and its cells (ID, text, what it shows if it is a
     formula cell, whether it has a grid);
   - apply one command and get back the change report;
   - undo and redo, each returning its change report;
   - save.
   Errors cross the bridge as values the Flutter side can show; nothing panics across it.
2. **A serialised form for commands and change reports**, for all commands at once (there are 45;
   see `crates/wombat-core/src/edit/mod.rs`). It is mechanical, it is what crosses the bridge, and
   it also allows recording and replaying commands in tests. Decide the form (derive `serde` on the
   command types, or let `flutter_rust_bridge` generate mirror types) and record the decision.
   Property test: every command survives a round trip through the serialised form unchanged.
3. **A bare Flutter app** (for example `app/` beside `crates/`) with one screen of plain text and
   three buttons: it opens a bundled example document, prints the root grid, sets the text of one
   cell, prints the change report and the grid again, and undoes.
4. **A test that runs without a device:** a Dart test (or a Rust test of the bridge crate) that
   performs the same sequence and checks the results, so CI can run it. Add it to
   `.github/workflows/wombat.yml`.
5. **A short summary** at `docs/milestones/B1.md` in the same form as the other milestone
   summaries, and the decisions taken, appended to `DECISIONS.md` from A122 and B100.

## What not to build

- No real interface: no grid drawing, no gestures, no navigation.
- No "rows visible on screen" query. The core cannot yet answer it cheaply; its right shape
  depends on how the grid is drawn, so it belongs to the next piece of work.
- No bridge for search, links, formulas' programs, import or export beyond what the 45 commands
  and the grid read already give. Those are widened later, once the pattern is proven.
- No iOS build as a condition of being done (see below).

## What "done" means

- `cargo fmt`, `cargo clippy -D warnings` and `cargo test --workspace` pass, as before, with the
  new crate in the workspace.
- The device-free test passes locally and in CI.
- The bare app **builds for Android** and runs either on an Android emulator or as a Flutter Linux
  desktop app, showing the sequence above. Say plainly which of the two was actually run.
- Measured and written in the summary: how long opening the document and one edit take across the
  bridge, and the memory the app uses with a 100,000-cell document open (the core's memory when
  loading is a known concern; see `POC_SUMMARY.md`).
- The summary says what was not verified.

## Things that will get in the way

- **Flutter is not installed** in a fresh cloud session, and the environment's network policy may
  need to allow its downloads (the Flutter SDK, pub.dev, Android SDK and Gradle hosts).
- **iOS builds need a Mac.** They cannot be produced in a Linux cloud session. Building for iOS
  on GitHub's macOS runners is possible but is a separate step, and needs an Apple developer
  account for anything that runs on a real iPhone.
- **Nobody can feel a phone from here.** Gestures, the on-screen keyboard and real performance
  need a person with a device. That matters for the next piece of work more than for this one.
- **The pinned Rust version** (`rust-toolchain.toml`) must be one `flutter_rust_bridge` supports.
  If it has to be raised, do it as its own commit and fix whatever the newer lints find.
- **The referee** (the real TreeSheets build used for comparisons) is not needed for this work.

## Rules that still apply

Everything in `../AGENTS.md`. In particular: the core stays free of interface code and of file
access outside its storage layer, documents change only through commands, nothing panics on bad
input, and every behaviour has a test. The bridge is a thin layer: if logic is needed, it goes in
the core, with tests there.

Each review of the core found real problems its author's tests had missed. Have this work
reviewed by someone who did not write it before building the next piece on top.

## Which model to use

This is advice from the session that built the core, not a measurement.

- **Use the most capable model available (Fable) for this bridge work.** It is small in lines of
  code but it sets patterns everything later copies: how commands cross the boundary, how errors
  are reported, how memory is shared. It also spans two languages and a build system, where a
  wrong early choice is expensive to undo. The core was built and reviewed at this level.
- **Opus is a reasonable second choice** if cost matters more than the last bit of care.
- **Sonnet is suited to the later, repetitive step** of widening the bridge to the remaining
  commands and queries once the pattern exists and is tested, and to routine fixes.
- Whatever builds it, **use a separate session for the review**, at the most capable level.
