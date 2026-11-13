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
| `docs/IOS.md` | How to build the app for an iPhone and put it on one, what CI shows of iOS and what nobody has tried |
| `docs/PHONE_TEST.md` | What the owner tries on a phone with the first slice of the interface, and how to get the app from a CI run |
| `docs/NEXT_REVIEW.md` | The brief for the independent review of the bridge |
| `docs/research/` | Background research behind the interface; not instructions |
| `crates/wombat-core` | The core library: document model (`model.rs`), commands, undo and redo (`edit/`: `mod.rs` the editor (commands, groups that can be ended or cancelled, undo and redo, `retype` for the text of an editor) and the list of commands, `command.rs` what each does, `restructure.rs` the same for sort, transpose, hierarchify, flatten, hierarchy swap, wrap and the like, `step.rs` the reversible steps, `size.rs` the running bound on the size of the saved file and the memory of the history, `program.rs` where editing meets calculating: the `Run` and `EnterText` commands, `read.rs` one grid as plain data and `stage.rs` what the first screen of an interface reads: the stage, the trail and the grids nested below, in full and in reduced form), formulas and the program of a document (`eval/`: `num.rs` numbers and characters, `formula.rs` one formula, `calc.rs` the results kept up to date, `run.rs` Program > Run), text rules (`text.rs`, with the part of a text that differs, `tests/text_difference.rs`), the construction layer for building documents (`construct.rs`), file format (`format.rs`), view state (`view.rs`: folds, selection, the level shown and what each level remembers), files and workspaces (`storage.rs`, `workspace.rs`), links and backlinks (`links/`: the link index, resolution, copying and moving between documents; `edit/paste.rs` the commands for that), indexes over a workspace (`summary.rs` what an index keeps per document, `scan.rs` reading a folder with a cache on disk), import and export (`convert/`: `delimited.rs` writing and reading CSV and TSV, finding the delimiter of a file, and the guard against formula injection, `export.rs` one grid, flattened, indented text, `lookup.rs` one table per grid with ID columns to join them, `names.rs` file names from cell text, `import.rs` reading files into a table of texts; `edit/import.rs` the command that puts one into a cell; `export_files.rs` writing an export to a file or folder, part of the file layer); of its tests, `tests/formulas_memory.rs` is the one that bounds the peak memory of its whole test process, alone in its file so that no other test's memory is counted, `tests/storage_bridge.rs` has what the storage layer does for the bridge (file states, copies that fail), `tests/view.rs` has what each level remembers and which level is shown (the view state of the first screen: its file form, what is refused, and how it follows commands, undo and redo), `tests/stage.rs` the read of the first screen (by example, against a second slow implementation on random documents, and what it costs on large ones), and `tests/line_breaks.rs` what every command, the model and the file format do with a text of several lines |
| `crates/wombat-cts` | Read-only importer of TreeSheets `.cts` files (all format versions up to 31), a testing tool; `tests/referee.rs` compares it with `referee/golden/`, `tests/quirks/` holds hand-built files with their own golden data |
| `crates/wombat-search` | The search service: folding of text for comparison (`fold.rs`), queries and their text syntax (`query.rs`), what is indexed per document and how it follows edits (`index.rs`), searching and ranking (`engine.rs`), the slow reference scanner that the tests compare with (`naive.rs`), find and replace (`replace.rs`), the index of a workspace folder (`workspace.rs`, the only file access, through `wombat_core::scan`); `tests/support/` has the query generator and the made-up workspace of the size tests, `tests/line_breaks.rs` searching and replacing in text of several lines |
| `crates/wombat-bridge` | The bridge to the Flutter interface (package `wombat_bridge`), a thin layer over the core: `api/document.rs` the open document that Dart holds (open, create, read one grid, read the stage and what is nested below it, read one cell, apply a command, groups of commands and entering an editor's text, undo, redo, the view state of this device, save, overwrite, close), `api/wire.rs` the serialised form of commands, change reports and errors, and what a stage read asks and answers, `convert.rs` between that and the core's types, `record.rs` the same as JSON text, `open_files.rs` the set of files that are open, by every name each has, so that none is open twice, `panic_tests.rs` what an open document does when something panics (inside the crate, since nothing outside can cause a panic); `frb_generated.rs` is generated and never edited; `tests/roundtrip.rs` every command through the serialised form and what is refused on the way in, `tests/fields.rs` every field of every command against a table written by hand, each direction on its own, `tests/deep.rs` the flat form of a table, `tests/out.rs` what goes out (reports, results, every error), `tests/session.rs` an open document as the app uses it, `tests/disk.rs` an open document and its file (when saving writes and when it refuses), `tests/open_files.rs` one open document per file, `tests/working_folder.rs` a relative path after the working folder changed (one test, alone in its file because it changes the working folder of the process), `tests/close.rs` closing a document and dropping one unclosed, `tests/stage.rs` every field of a stage read against values written by hand, and the range of every number sent for one, `tests/stage_read.rs` the stage read through an open document (the example, what is refused, random documents against the core, and what it costs on large ones), `tests/groups.rs` groups of commands and `retype`: what each reports and what a group means for the saved flag, saving, closing and dropping, `tests/view.rs` the view state: read at opening, kept in step, written at closing, and never in the way of the document, `tests/line_breaks.rs` a text of several lines through every way in and out, `tests/randseq.rs` random commands, groups (ended and cancelled), entered texts, undo and redo through an open document beside an editor of the core (it takes the core's generator from `crates/wombat-core/tests/support` by path), `tests/common/` what those share, with the list of the core's commands that does not compile when one is added |
| `app/` | The Flutter app as far as it goes: `lib/session.dart` the open document as Dart holds it, the only way to the bridge's handle: calls in the order they were made, closing, the text for errors, positions between Dart's counting and the core's (`codePoints`, `units`), the request a screen starts from (`phoneAsk`), and `InTurn`, the queue that keeps the order (A144, A154, A160, A175); `lib/keeper.dart` what owns the open document for every screen on it, is the one place that saves and says how the document stands with its file (A155, A161), and has for every screen the stage kept up to date (`StageReader`, A176), the editing of a cell's text (`Editing`, A177) and where the screen is on this device (A178); `lib/stage_read.dart` one answer of a stage read with the ways through it, and what differs between two (`StageRead`, `StageChange`); `lib/cell_editor.dart` the editor of a cell as a screen and its toolbar know it, and the one kind there is, made of Flutter's text field (A180); the phone screen of `docs/UI_SPEC.md`, its first slice (`lib/phone/`, A181 to A189, `docs/milestones/UI2.md`): `phone_screen.dart` the screen and the one queue of the user's actions, `stage_view.dart` the stage's grid scrolling both ways with the first row and column kept in view, `cell_view.dart` a cell and the four forms of a grid inside it, `budget.dart` which form a grid gets and how wide the columns are, `editor_bar.dart` the editor docked at the bottom with its toolbar, `place.dart` where a cell is in a stage read and which ranges are styled, `settings_sheet.dart`, `variants.dart` and `log.dart` the prototype's switches and its log, `words.dart` every text the screen shows; and the plain-text check of the bridge from milestone B1 (`lib/demo.dart` its three steps, `lib/demo_screen.dart` the screen); `lib/main.dart` says where the app's documents are kept (A165), opens its document, at first a copy of the example (`lib/example.dart` makes the copy), puts aside one that cannot be opened (A163), shows the phone screen on it (or the screen a test gives), opens the prototype's test documents from that screen's menu (`lib/test_documents.dart` makes them, A188), and in `Kept` tells the keeper when the app is put away and holds the app's exit until the document is saved (A162); `lib/src/rust/` is generated; `rust_builder/` is the build glue that `flutter_rust_bridge` put there (cargokit), not ours to edit; `ios/` is the iOS project as Flutter generated it, with the app's identifier and name set by hand (A192), `android/` and `linux/` the projects for those systems; `assets/example.wombat` the example; `lib/measure.dart` times the app on a 100,000-cell document it builds itself, and the read a screen draws from on that and on a document nested six levels deep (the two are also test documents of the prototype); `test/` runs without a device (`support.dart` builds the bridge's library and loads it from where cargo says it is, `support_test.dart` tests that; `fake_document.dart` is a stand-in for the core that can be held up and made to fail, and `Watched`, the same switches over a document of the real core; `fake_document_test.dart` holds the stand-in to the real core (A179); `probe_screen.dart` is the least screen there can be on a keeper, for testing what is under every screen; `session_test.dart`, `keeper_test.dart`, `keeper_stage_test.dart` (the stage kept up to date, and the view state), `keeper_editing_test.dart` (the editing), each on the stand-in and on the real core, `stage_read_test.dart` (what differs between two reads, on the real core), `cell_editor_test.dart` and `main_test.dart` (the app's start and end, on the probe screen; its last group the app with the phone screen and the test documents) test the layers under every screen; `phone_parts_test.dart` (the depth budget, column widths, places, styled ranges, how text is drawn, the log), `phone_screen_test.dart` on the stand-in and `phone_screen_real_test.dart` on the real core (the flows of UI_SPEC section 17 that the slice covers, rows and columns, styles, the file) test the phone screen, with `phone_support.dart` what they share; `demo_test.dart`, `demo_screen_test.dart`, `wire_test.dart`, `measure_test.dart`), `integration_test/` on one, one file per call (`test_driver/` prints the measurements) |
| `crates/wombat-cli` | The `wombat` command-line tool, a thin layer over the core; `edit.rs` reads the editing commands of `wombat edit`; `tests/ops.rs` compares commands with `referee/ops/`, `tests/line_breaks.rs` has text of several lines through the built program |
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
dart format lib/*.dart lib/phone test integration_test test_driver
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
document (`app/lib/keeper.dart`), which saves for every screen (A155), reads the stage for
it and holds its editing. (The bridge check's
screen has a document of its own, through `Demo`.)

What a screen on a `Keeper` must do, which the code does not enforce
(`app/test/probe_screen.dart` is the least screen that does it, and
`app/lib/phone/phone_screen.dart` the one the app shows):

- **Put everything the user does through one queue** (`InTurn`, A158), with `post`, and give
  the queue a handler for errors (`onUnheard`) that shows them.
- **Draw from a `StageReader`** (`Keeper.read`, A176), and from nothing it keeps itself: the
  keeper reads the stage again after every change, by whomever, before the call that made
  the change answers. Listen to the reader and draw again what the change names
  (`StageChange`); close the reader when the screen goes. Go to another level with
  `StageReader.go`, and say where the screen is with `at` as it scrolls, which costs
  nothing: the level left remembers it, and it is in the file with the next save. A screen
  that asks for a window of rows (`phoneWindowAsk`) says which rows it shows (`see`), or
  the rows after the fortieth are never read.
- **Edit a cell's text through an `Editing`** (`Keeper.edit`, A177), and touch the text
  field only through a `CellEditor` (A180). Nothing is sent while the user types. The tick,
  Return and every other action enter it (`enter`) before they do anything else; the cross
  drops it (`drop`). While an editing holds something in the document, the keeper refuses
  `apply`, `undo`, `redo` and `edit` with a `StateError`. Take the cell's text from what
  `hold` and `style` answer, and count positions in that: the core may keep another text
  than was sent (CR LF is one line break there).
- **Attach itself** (`Keeper.attach`) for as long as it is there, with that queue and with a
  `finish` that holds in the document what is being typed (`Editing.hold`), and detach when
  it goes. Without it the app is put away, closed and exited without what is on the
  screen. `finish` does the work itself: the keeper runs it in the queue's turn, so it must
  not ask that queue. It shows what the core refuses instead of throwing it, and it has
  two seconds.
- **End its editing when it goes, with one call** (`Editing.enter`, or `drop`), on an
  editing that is open already. An editing that holds something and is forgotten keeps
  every other change out until the document is closed. One that is opened only in
  `dispose` and entered after that is two calls, and the second comes after the close that
  the app asks in between: the text is lost. So an editor that stays open after its text
  was entered (more was typed while the core worked) gets a new editing at once, inside
  the same action (A186).
- **Say where it is while it is there.** `StageReader.at` hands nothing on by itself: the
  keeper does, with a save, when the app is put away, and when a level is left. A screen
  that is taken away closes its reader, and what it said last is lost unless the keeper
  was asked first: whoever takes a screen off a document that stays open calls
  `Keeper.putAway` before (the start screen does, A188). Measure the place after a frame
  was drawn: scrolling comes to rest before the rows are laid out where it ended.
- **Give every part a key that says what it is** (`cell-<x>-<y>`, `child-…`, `tool-…`; the
  class comments of `lib/phone/` list them, A181): tests find things by them, and so does
  a report of a fault.
- **Put every text it shows in `lib/phone/words.dart`**, so that it can be translated.
- **Show `Keeper.state`**, and what to do for `failed` and `changedOnDisk`: a document that
  is not in its file holds the app's exit back once (A162), and the user has to see why.
  `Keeper.viewError` is not for the screen: it says nothing about the document (A178).
- **Close its editor without entering when `Keeper.opens` changes**: the document is
  another, and the editing ended with the one before (`Editing.open` is false). The reader
  starts again by itself, at the level the view state of the other document names, and
  says that everything differs.
- **Wait for what it starts inside an action**, or set it apart (`InTurn.apart`): work that
  is merely started counts as part of the action for as long as the action runs (A160).

The code does enforce the rest: calls in order, no call after the end, no wait of a queue
for itself or of two for each other, saving, what `close` and `openTheirs` wait for, the
stage read again after every change, no change beside an editing that holds something,
and the view state written after the document and never in its way.

CI (`.github/workflows/wombat.yml`) runs on every push that touches `wombat/`, in five jobs.
"Format, lint and test" runs the same three with `--locked`, and checks that the core and the
search service build for the web, Android and iOS. "Bridge and app on Linux" runs the
generator and fails when the committed generated code is not what it writes (regenerate as
above and commit; the check removes all generated code first, so a file the generator no
longer writes fails it too), then the app's format check, analyzer and device-free tests, and
then builds the app for Linux and runs each file of `app/integration_test/` on it, one call
per file, under `xvfb-run`. "App for Android" builds the debug package for arm64 and runs
nothing, and keeps the package as the artifact `wombat-android-arm64-debug`
(`docs/PHONE_TEST.md`). "App on an Android emulator" starts an x86-64 emulator of Android 14
(`system-images;android-34;default;x86_64`, `pixel_6`, 1080x2400, 4 GB, 2 processors, software
drawing) with the SDK's own tools, builds a debug package for `android-x64` and runs each file
of `app/integration_test/` on it in a call of its own with a time limit, the measurement last
and twice. It takes about twelve minutes, four of them the build; nothing of the emulator is
cached. Android's animation settings are left as a phone has them, on purpose: with the
transition animation scale at 0 Flutter runs its own animations at a twentieth of their
length. A green emulator job says that the bridge's library loads on Android, that the app
opens, changes and saves a document in its own storage there and draws at the size of a phone.
It says nothing about the on-screen keyboard, the back button or gesture, or what Android does
to an app out of sight: the tests act inside Flutter, and the test of the phone screen is its
own keyboard, so the on-screen one does not come up (A191).
"App for iOS (simulator build)" runs on a Mac (`macos-15`): it builds the debug app for an
iPhone simulator with nothing signed, fails if the bridge's library is not in the app, then
makes a simulator (the newest iOS the runner's Xcode has, the newest plain iPhone, chosen by
rule because the names change with the runner's image) and runs
`app/integration_test/app_test.dart` on it with a time limit. It takes about fifteen minutes.
A green job says that the iOS
project builds, that the bridge compiles for iOS and that its library loads and answers on a
simulator. It says nothing of a real iPhone, of signing, or of the other two test files,
which do not run there: the phone screen has never run on iOS. No `Podfile` is committed yet
(`docs/IOS.md`), so the job's pods are whatever CocoaPods resolves that day. A test of the built app waits for the
thing it expects, not for a screen that has stopped drawing: the screen draws nothing while
the core works, and nothing at all while the app is hidden. Cargo and pub may not change their lock files
in CI: commit `Cargo.lock` and `app/pubspec.lock` with the change that needs them. The versions
of Flutter, the generator and `cargo-expand` are pinned at the top of the workflow; raise them
there together with the lock files. The iOS job has a checksum of its own, of Flutter's archive
for a Mac (`FLUTTER_MACOS_SHA256` in the job); raise it with the version. The Rust version is pinned in `rust-toolchain.toml` so that
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
