# Quirk files

Hand-built `.cts` files for what TreeSheets' loader does that none of its example files
exercises: sizes that cells without text take from their parent, runs and links that have to be
repaired, text that is not UTF-8, the selection and zoom after loading, images without a length,
a fold that takes a cell's image away (versions 16 and 17), and so on.

- `*.cts` are written by `quirk_files()` in `../../src/tests.rs`. A unit test checks that the
  files here are byte for byte what that code builds. To write them again:
  `cargo test -p wombat-cts --lib -- --ignored write_quirk_files`
- `golden/*.json` is what the real TreeSheets read from them, made with the referee (see
  `../../../../referee/README.md`), from the repository root:
  `wombat/referee/export_golden -o wombat/crates/wombat-cts/tests/quirks/golden wombat/crates/wombat-cts/tests/quirks/*.cts`
  and then deleting the `.txt` and `.xml` files it also writes, which no test uses.

`tests/referee.rs` compares the importer with these exactly as it does with the examples.
