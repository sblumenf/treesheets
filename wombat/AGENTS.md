# AGENTS.md (Wombat)

Rules for anyone, person or AI, changing code under `wombat/`. The parent `../AGENTS.md` covers the
TreeSheets C++ code; its style rules (unity build, wx naming, clang-format) do **not** apply here.

Read `docs/PLAN.md` and `docs/DECISIONS.md` before changing anything. They are the project's
memory: sessions do not remember each other, so anything agreed must be written there.

## Layout

| Path | Contents |
| ---- | -------- |
| `docs/PLAN.md` | The milestones, their status, scope and the future list |
| `docs/POC_SUMMARY.md` | What the proof of concept achieved, how it was checked, known limits, and what comes next |
| `docs/DECISIONS.md` | Design decisions and deliberate departures from TreeSheets |
| `docs/FILE_FORMAT.md` | Specification of the `.wombat` format, the image folder and the view-state file |
| `docs/milestones/` | One summary per finished milestone |
| `docs/UI_SPEC.md` | Specification of the Flutter interface |
| `docs/NEXT_BRIDGE.md` | The next piece of work: a minimal bridge between Flutter and the core |
| `docs/ANDROID.md` | How to run, test and measure the app on an Android device |
| `docs/NEXT_REVIEW.md` | The brief for the independent review of the bridge |
| `docs/research/` | Background research behind the interface; not instructions |
| `crates/wombat-core` | The core library: document model (`model.rs`), commands, undo and redo (`edit/`: `mod.rs` the editor and the list of commands, `command.rs` what each does, `restructure.rs` the same for sort, transpose, hierarchify, flatten, hierarchy swap, wrap and the like, `step.rs` the reversible steps, `size.rs` the running bound on the size of the saved file and the memory of the history, `program.rs` where editing meets calculating: the `Run` and `EnterText` commands), formulas and the program of a document (`eval/`: `num.rs` numbers and characters, `formula.rs` one formula, `calc.rs` the results kept up to date, `run.rs` Program > Run), text rules (`text.rs`), the construction layer for building documents (`construct.rs`), file format (`format.rs`), view state (`view.rs`), files and workspaces (`storage.rs`, `workspace.rs`), links and backlinks (`links/`: the link index, resolution, copying and moving between documents; `edit/paste.rs` the commands for that), indexes over a workspace (`summary.rs` what an index keeps per document, `scan.rs` reading a folder with a cache on disk), import and export (`convert/`: `delimited.rs` writing and reading CSV and TSV, finding the delimiter of a file, and the guard against formula injection, `export.rs` one grid, flattened, indented text, `lookup.rs` one table per grid with ID columns to join them, `names.rs` file names from cell text, `import.rs` reading files into a table of texts; `edit/import.rs` the command that puts one into a cell; `export_files.rs` writing an export to a file or folder, part of the file layer); of its tests, `tests/formulas_memory.rs` is the one that bounds the peak memory of its whole test process, alone in its file so that no other test's memory is counted, and `tests/storage_bridge.rs` has what the storage layer does for the bridge (file states, copies that fail) |
| `crates/wombat-cts` | Read-only importer of TreeSheets `.cts` files (all format versions up to 31), a testing tool; `tests/referee.rs` compares it with `referee/golden/`, `tests/quirks/` holds hand-built files with their own golden data |
| `crates/wombat-search` | The search service: folding of text for comparison (`fold.rs`), queries and their text syntax (`query.rs`), what is indexed per document and how it follows edits (`index.rs`), searching and ranking (`engine.rs`), the slow reference scanner that the tests compare with (`naive.rs`), find and replace (`replace.rs`), the index of a workspace folder (`workspace.rs`, the only file access, through `wombat_core::scan`); `tests/support/` has the query generator and the made-up workspace of the size tests |
| `crates/wombat-bridge` | The bridge to the Flutter interface (package `wombat_bridge`), a thin layer over the core: `api/document.rs` the open document that Dart holds (open, create, read one grid, apply a command, undo, redo, save, overwrite, close), `api/wire.rs` the serialised form of commands, change reports and errors, `convert.rs` between that and the core's types, `record.rs` the same as JSON text, `open_files.rs` the set of files that are open, by every name each has, so that none is open twice, `panic_tests.rs` what an open document does when something panics (inside the crate, since nothing outside can cause a panic); `frb_generated.rs` is generated and never edited; `tests/roundtrip.rs` every command through the serialised form and what is refused on the way in, `tests/fields.rs` every field of every command against a table written by hand, each direction on its own, `tests/deep.rs` the flat form of a table, `tests/out.rs` what goes out (reports, results, every error), `tests/session.rs` an open document as the app uses it, `tests/disk.rs` an open document and its file (when saving writes and when it refuses), `tests/open_files.rs` one open document per file, `tests/working_folder.rs` a relative path after the working folder changed (one test, alone in its file because it changes the working folder of the process), `tests/close.rs` closing a document and dropping one unclosed, `tests/randseq.rs` random commands through an open document beside an editor of the core (it takes the core's generator from `crates/wombat-core/tests/support` by path), `tests/common/` what those share, with the list of the core's commands that does not compile when one is added |
| `app/` | The Flutter app as far as it goes: `lib/session.dart` the open document as Dart holds it, the only way to the bridge's handle: calls in the order they were made, closing, the text for errors, and `InTurn`, the queue that keeps the order (A144, A154, A160); `lib/keeper.dart` what owns the open document for every screen on it, is the one place that saves and says how the document stands with its file (A155, A161); a simple grid screen that stands in for the real interface until that is built (`lib/grid/stage.dart` its logic without widgets, `lib/grid/grid_screen.dart` the widgets and the one queue of the user's actions; `docs/milestones/UI1.md`), and the plain-text check of the bridge from milestone B1 (`lib/demo.dart` its three steps, `lib/demo_screen.dart` the screen); `lib/main.dart` says where the app's document is kept (A165), opens it, at first a copy of the example (`lib/example.dart` makes the copy), puts aside one that cannot be opened (A163), shows on it the screen it is given (the grid screen, `GridOnKeeper`), and in `Kept` tells the keeper when the app is put away and holds the app's exit until the document is saved (A162); `lib/src/rust/` is generated; `rust_builder/` is the build glue that `flutter_rust_bridge` put there (cargokit), not ours to edit; `assets/example.wombat` the example; `lib/measure.dart` times the app on a 100,000-cell document it builds itself; `test/` runs without a device (`support.dart` builds the bridge's library and loads it from where cargo says it is, `support_test.dart` tests that; `fake_document.dart` is a stand-in for the core that can be held up and made to fail, and `Watched`, the same switches over a document of the real core; `probe_screen.dart` is the least screen there can be on a keeper, for testing what is under every screen; `session_test.dart`, `keeper_test.dart` (on the stand-in and on the real core) and `main_test.dart` (the app's start and end, on the probe screen) test the layers that are kept; `stage_test.dart`, `grid_screen_test.dart` on the stand-in and `grid_screen_real_test.dart` on the real core test the grid screen and go with it; `demo_test.dart`, `demo_screen_test.dart`, `wire_test.dart`, `measure_test.dart`), `integration_test/` on one, one file per call (`test_driver/` prints the measurements) |
| `crates/wombat-cli` | The `wombat` command-line tool, a thin layer over the core; `edit.rs` reads the editing commands of `wombat edit`; `tests/ops.rs` compares commands with `referee/ops/` |
| `referee/` | Scripts that drive the real TreeSheets app, and the reference ("golden") data it produced |
| `referee/ops/` | Operation cases: one edit done by the real app (`cases/*.lobster`) and as Wombat commands (`cases/*.edit`) on `fixture.cts`, the app's result (`golden/`), and `make_ops` to regenerate |
| `referee/ops/ui/` | The same for menu actions that no script function reaches: `cases/*.ui` says which keys were pressed in the real app (`referee/ui.py`, `make_ui_ops`), on its own `fixture.cts`; also text typed into cells and Program > Run (`type`, `retype`, `action run`) |
| `referee/formulas/` | What the real TreeSheets calculates: documents with formulas, programs and series of edits (`cases.py`, by hand and random), the app's answers (`golden/*.jsonl`), `make_formulas` to regenerate; `crates/wombat-core/tests/formulas_referee.rs` compares; `golden/review.jsonl` holds the cases added after review, so that the older files stay as they are |
| `referee/exports/` | What the real TreeSheets exports and imports: ten cases as CSV, TSV and indented text, the CSV of every example, and 25 files with the documents the app made of them; `make_exports` to regenerate (it answers the app's file dialogs by key presses), `README.md` there; `check_lookup_libreoffice` opens a lookup export in a real LibreOffice and enters the README's formulas (optional, by hand); `crates/wombat-cts/tests/export.rs` and `crates/wombat-core/tests/import_referee.rs` compare |

New crates (for example the `.cts` importer or the search service) go under `crates/` and are
listed here when added.

## Checks that must pass before every commit

Run from `wombat/`:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

After changing anything under `crates/wombat-bridge/src/api/`, regenerate both sides of the
bridge and commit them, then run the app's checks. From `wombat/app/`, with Flutter and
`flutter_rust_bridge_codegen` 2.13.0 installed (`docs/milestones/B1.md` says how):

```sh
flutter_rust_bridge_codegen generate
(cd .. && cargo fmt --all && cargo build -p wombat_bridge)
dart format lib/*.dart lib/grid test integration_test test_driver
flutter analyze
flutter test
```

On a machine with a display or `xvfb-run`, also run the built app's tests, one file per call
(the whole folder in one call fails):

```sh
for test in integration_test/*_test.dart; do xvfb-run -a flutter test "$test" -d linux; done
```

In Dart, reach a document only through `Session` (`app/lib/session.dart`), never through the
generated `OpenDocument`: its calls are kept apart but not in order (A127, A144). A screen
on the app's document does not hold a `Session` either: it works through the `Keeper` of the
document (`app/lib/keeper.dart`), which saves for every screen (A155). (The bridge check's
screen has a document of its own, through `Demo`.)

What a screen on a `Keeper` must do, which the code does not enforce:

- **Put everything the user does through one queue** (`InTurn`, A158), with `post`, and give
  the queue a handler for errors (`onUnheard`) that shows them.
- **Attach itself** (`Keeper.attach`) for as long as it is there, with that queue and with a
  `finish` that puts into the document what is being typed, and detach when it goes. Without
  it the app is put away, closed and exited without what is on the screen. `finish` does
  the work itself: the keeper runs it in the queue's turn, so it must not ask that queue.
  It shows what the core refuses instead of throwing it, and it has two seconds.
- **Show `Keeper.state`**, and what to do for `failed` and `changedOnDisk`: a document that
  is not in its file holds the app's exit back once (A162), and the user has to see why.
- **Start again at the root when `Keeper.opens` changes**: the document is another.
- **Wait for what it starts inside an action**, or set it apart (`InTurn.apart`): work that
  is merely started counts as part of the action for as long as the action runs (A160).

The code does enforce the rest: calls in order, no call after the end, no wait of a queue
for itself or of two for each other, saving, and what `close` and `openTheirs` wait for.

CI (`.github/workflows/wombat.yml`) runs on every push that touches `wombat/`, in four jobs.
"Format, lint and test" runs the same three with `--locked`, and checks that the core and the
search service build for the web, Android and iOS. "Bridge and app on Linux" runs the
generator and fails when the committed generated code is not what it writes (regenerate as
above and commit; the check removes all generated code first, so a file the generator no
longer writes fails it too), then the app's format check, analyzer and device-free tests, and
then builds the app for Linux and runs each file of `app/integration_test/` on it, one call
per file, under `xvfb-run`. "App for Android" builds the debug package for arm64 and runs
nothing. "App on an Android emulator" starts an x86-64 emulator of Android 14
(`system-images;android-34;default;x86_64`, `pixel_6`, 1080x2400, 4 GB, 2 processors, software
drawing) with the SDK's own tools, builds a debug package for `android-x64` and runs each file
of `app/integration_test/` on it in a call of its own with a time limit, the measurement last
and twice. It takes about ten minutes, four of them the build; nothing of the emulator is
cached. Android's animation settings are left as a phone has them, on purpose: with the
transition animation scale at 0 Flutter runs its own animations at a twentieth of their
length. A green emulator job says that the bridge's library loads on Android, that the app
opens, changes and saves a document in its own storage there and draws at the size of a phone.
It says nothing about the on-screen keyboard, the back button or gesture, or what Android does
to an app out of sight: the tests act inside Flutter. A test of the built app waits for the
thing it expects, not for a screen that has stopped drawing: the screen draws nothing while
the core works, and nothing at all while the app is hidden. Cargo and pub may not change their lock files
in CI: commit `Cargo.lock` and `app/pubspec.lock` with the change that needs them. The versions
of Flutter, the generator and `cargo-expand` are pinned at the top of the workflow; raise them
there together with the lock files. The Rust version is pinned in `rust-toolchain.toml` so that
CI and every machine use the same compiler and lints; the app's own build of the bridge
(cargokit) uses rustup's `stable` all the same (A131).

## Engineering rules

- **The core has no interface code and no global state.** No printing, no reading the clock or
  random numbers except through values passed in, no file access outside the storage module.
- **Documents change only through commands** addressed by cell ID. Every command is reversible and
  returns what it changed (decisions A9, A10, A34, A35). Do not add a second way in. Building a
  document that nobody edits yet (reader, importer, tests) uses `construct::Construct` (A31).
- **No panics on bad input.** Library code does not use `unwrap`, `expect`, `panic!`, unchecked
  indexing or unchecked arithmetic on anything that came from a file or a caller. Return errors.
  `unsafe` is forbidden (`#![forbid(unsafe_code)]`).
- **Bound everything read from a file:** lengths, counts, nesting depth, decompressed size.
- **Deterministic output.** Saving, exporting and iteration order never depend on hash-map order,
  the clock or the machine.
- **Dependencies are a cost.** Add one only when it clearly earns its place, prefer widely used
  crates, and check that it compiles for phones and desktops (iOS, Android, macOS, Windows, Linux).
- **Keep it small.** No speculative features, options or abstractions. Three layers is enough.
- **Public items have a doc comment** that says what they do. Comments explain why, not what.

## Testing rules

- **Every behaviour has a test**, written with the code and not afterwards.
- **Use rules as well as examples** (property tests with `proptest`): for example "any sequence of
  commands followed by undoing them all gives back the original document", "save then load gives
  an equal document", "loading arbitrary bytes never panics".
- **Use the referee where TreeSheets defines the behaviour.** Compare against the files in
  `referee/golden/`, which the real TreeSheets produced. Tests must not need the real app at test
  time; only regenerating the golden files does.
- **Never weaken, skip or delete a failing test to get a pass.** Fix the code, or if the test is
  wrong, say why in the commit message.
- **A bug fix starts with a test that fails** because of the bug.

## TreeSheets as the reference

- Where behaviour is not specified in `docs/`, do what TreeSheets does. Read `../src/` to find
  out, and treat it as a description of behaviour to redesign, not as code to translate line by
  line. Its structure (model, drawing and input in the same types) is what Wombat exists to avoid.
- A deliberate difference from TreeSheets is added to section B of `docs/DECISIONS.md` in the same
  commit.

## Finishing a milestone

1. All checks pass.
2. The command-line tool does what the plan's "what you can try" column says.
3. `docs/milestones/M<n>.md` says, in plain English: what was built, how to try it, what was left
   out, what went wrong, and what the next milestone should know.
4. The status in `docs/PLAN.md` is updated.

Report results as they are. If something does not work or was skipped, say so in the summary.

## Git

- Commit messages: imperative, sentence-case subject starting with "Wombat:", with a body that
  explains why.
- Do not commit `target/` or scratch files.
