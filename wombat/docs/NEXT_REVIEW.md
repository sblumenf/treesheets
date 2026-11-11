# Review of the bridge (milestone B1)

This note is for a session that did **not** write the bridge, person or AI. It says what to
review, what to look for and what to hand back. `NEXT_BRIDGE.md` asks for this review before
anything is built on the bridge; each review of the core found real problems its author's
tests had missed.

## What to review

The work of milestone B1, which is these commits on the branch `claude/focused-cannon-r4a117`:

```sh
git log --oneline d250d84..32da19f
git diff d250d84..32da19f --stat -- wombat .github/workflows/wombat.yml
```

- **The core additions:** `crates/wombat-core/src/edit/read.rs` (`Editor::grid_view`),
  `Clip::as_document` and `Clip::from_document` in `crates/wombat-core/src/edit/paste.rs`,
  and `crates/wombat-core/tests/read.rs`.
- **The bridge crate:** everything written by hand in `crates/wombat-bridge`: `Cargo.toml`,
  `src/lib.rs`, `src/api/document.rs`, `src/api/wire.rs`, `src/convert.rs`, `src/record.rs`
  and `tests/`.
- **The bare app:** `app/lib/main.dart`, `app/lib/demo.dart`, `app/test/`,
  `app/integration_test/`, `app/pubspec.yaml`.
- **The CI job** "Bridge and bare app" in `.github/workflows/wombat.yml`.
- **The documents:** `docs/milestones/B1.md` and decisions A122 to A131 in
  `docs/DECISIONS.md`.

Later commits on the branch (a simple grid screen, measuring, Android) are not part of this
review unless you are told otherwise.

## What not to review

Generated or vendored files, except to check that they are what the tools would write:
`crates/wombat-bridge/src/frb_generated.rs`, `app/lib/src/rust/`, `app/rust_builder/`,
`app/android/`, `app/linux/`, `app/pubspec.lock`, `Cargo.lock`.

## Read first

`../AGENTS.md` (the rules the work must keep), `NEXT_BRIDGE.md` (what was asked),
`milestones/B1.md` (what the author says was built, run and left out), and decisions A1 to
A15, A28, A31, A90 and A121 to A131 in `DECISIONS.md`.

Form your own view from the code. `B1.md` and the decisions are the author's account: check
them against the code, do not take them as findings.

## What to look for

1. **Defects:** inputs or sequences of calls that give a wrong result, a panic, a document
   that breaks the model's rules, or a document and a file that no longer agree.
2. **Ways round the core's rules.** Documents change only through commands; nothing panics
   on bad input; no file access in the core outside its storage layer. In particular: can
   anything that comes over the bridge (a command, a clip, a table, an ID, unknown fields)
   make the core hold or save something it would refuse from a file?
3. **The serialised form.** Does every one of the 45 commands mean the same on both sides?
   A field swapped in both directions survives a round trip, so the round-trip test cannot
   see it: compare `api/wire.rs` and `convert.rs` with `Command` in
   `crates/wombat-core/src/edit/mod.rs`. Is anything lost or changed on the way (numbers,
   text, order, unknown fields)?
4. **The open document:** what is saved and when, the saved flag, creating over an existing
   file, the copy with fresh IDs, the clock and the ID source, what happens to the document
   when a call fails.
5. **Threads and timing:** what happens when Dart makes two calls on one document without
   waiting, or lets go of a document while a call runs. The author did not test this.
6. **Behaviour without a test,** and tests that would still pass if the code were wrong.
   Changing a line of the code and seeing whether a test fails is a good way to find these.
7. **Claims in `B1.md` and the decisions that the code does not bear out,** and things left
   out that are not listed there.
8. **The pattern.** Later work will copy how commands cross, how errors are reported and
   how a document is held. Say so if one of these will be costly to live with, and why.

## How to check

From `wombat/`:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The app's tests need Flutter 3.47.9 (`B1.md`, "How to try it"). Reading alone is a valid
review; say which of these you ran.

## What to hand back

A list of findings, most serious first. For each: the file and line, what is wrong, an
input or a sequence of calls that shows it (as a failing test if you can), and how serious
it is. Keep what you verified apart from what you suspect. Say also what you looked at and
found sound, and what you did not look at.

**Do not fix anything.** The fixes are a separate step, each starting with a test that
fails because of the finding, and they end as a "Fixes after review" section in `B1.md`, as
in the earlier milestones.
