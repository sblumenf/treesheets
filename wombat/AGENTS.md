# AGENTS.md (Wombat)

Rules for anyone, person or AI, changing code under `wombat/`. The parent `../AGENTS.md` covers the
TreeSheets C++ code; its style rules (unity build, wx naming, clang-format) do **not** apply here.

Read `docs/PLAN.md` and `docs/DECISIONS.md` before changing anything. They are the project's
memory: sessions do not remember each other, so anything agreed must be written there.

## Layout

| Path | Contents |
| ---- | -------- |
| `docs/PLAN.md` | The milestones, their status, scope and the future list |
| `docs/DECISIONS.md` | Design decisions and deliberate departures from TreeSheets |
| `docs/FILE_FORMAT.md` | Specification of the `.wombat` format, the image folder and the view-state file |
| `docs/milestones/` | One summary per finished milestone |
| `crates/wombat-core` | The core library: document model (`model.rs`), commands, undo and redo (`edit/`: `mod.rs` the editor and the list of commands, `command.rs` what each does, `restructure.rs` the same for sort, transpose, hierarchify, flatten, hierarchy swap, wrap and the like, `step.rs` the reversible steps), text rules (`text.rs`), the construction layer for building documents (`construct.rs`), file format (`format.rs`), view state (`view.rs`), files and workspaces (`storage.rs`, `workspace.rs`); later links, formulas |
| `crates/wombat-cts` | Read-only importer of TreeSheets `.cts` files (all format versions up to 31), a testing tool; `tests/referee.rs` compares it with `referee/golden/`, `tests/quirks/` holds hand-built files with their own golden data |
| `crates/wombat-cli` | The `wombat` command-line tool, a thin layer over the core; `edit.rs` reads the editing commands of `wombat edit`; `tests/ops.rs` compares commands with `referee/ops/` |
| `referee/` | Scripts that drive the real TreeSheets app, and the reference ("golden") data it produced |
| `referee/ops/` | Operation cases: one edit done by the real app (`cases/*.lobster`) and as Wombat commands (`cases/*.edit`) on `fixture.cts`, the app's result (`golden/`), and `make_ops` to regenerate |
| `referee/ops/ui/` | The same for menu actions that no script function reaches: `cases/*.ui` says which keys were pressed in the real app (`referee/ui.py`, `make_ui_ops`), on its own `fixture.cts` |

New crates (for example the `.cts` importer or the search service) go under `crates/` and are
listed here when added.

## Checks that must pass before every commit

Run from `wombat/`:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI (`.github/workflows/wombat.yml`) runs the same three on every push that touches `wombat/`.

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
  crates, and check that it can compile to WebAssembly and for phones, since the core must.
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
