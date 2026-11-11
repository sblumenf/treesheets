# Wombat proof of concept: closing summary

The proof of concept set out to show that a Rust core with no interface can create, edit,
calculate, link, search and save nested-grid documents. All eight milestones in `PLAN.md` are
built, each was reviewed independently where it was large enough to matter, and the fixes from
those reviews are in. This page says what exists, how far it can be trusted, and what a phone
interface will need next.

## What exists

| Milestone | What you can do today |
| ---- | ---- |
| 1. Documents and file format | Create nested documents and save them as readable `.wombat` files, with permanent cell IDs |
| 2. TreeSheets importer | Import any TreeSheets `.cts` file (format versions 1 to 31) |
| 3. Editing, undo and redo | Change documents with 45 commands, including sort, flatten, hierarchify and transpose, and undo back to an identical file |
| 4. Links and backlinks | Link cells within and across documents; links survive renaming, moving and restructuring; list what links to a cell |
| 5. Formulas | Formula cells such as `=SUM(Costs) * 1.2` that recalculate on edits, and TreeSheets' "Run" programs |
| 6. Search | Search a whole workspace as you type, with tags, links, dates and branches as parts of the query; find-next and replace |
| 7. Import and export | Export a grid, a flattened table or a lookup folder for XLOOKUP; import CSV, TSV and indented text |

Everything is reachable from the `wombat` command-line tool; each milestone summary under
`milestones/` has a demo with exact commands.

Size: about 35,000 lines of Rust in four crates (`wombat-core`, `wombat-search`, `wombat-cts`,
`wombat-cli`) and about 24,000 lines of tests. The core depends on three small libraries and the
search service on four; both build for the web (WebAssembly), Android and iOS.

## How it was checked

- **473 automated tests**, run on every push, taking about four minutes.
- **Comparison with the real TreeSheets.** A headless build of TreeSheets is the referee
  (`../referee/`). The importer matches it cell for cell on 38 documents; editing operations match
  on 117 recorded cases; formulas match on 168 example cells, 436 generated documents and 362
  programs; text export matches byte for byte on 38 documents.
- **Rules as well as examples.** Random command sequences check that undoing everything gives back
  the original bytes, that change reports are complete, that the link and search indexes kept up
  to date edit by edit equal ones rebuilt from scratch, and that search agrees with a slow scan of
  every cell.
- **Deliberate breakage.** For each milestone from 4 on, the code was broken on purpose in 20 to
  27 ways to confirm the tests notice. Where a breakage slipped through, a test was added.
- **Independent reviews** of milestones 1, 3, 4, 5, 6 and 7 by reviewers who did not write the
  code. Every one found real problems the author's tests had missed; all findings rated critical
  or major were fixed, each starting from a test that failed.

## What the reviews found, in kind

It is worth knowing the pattern, because the same kinds of problem will come up again:

- **Writes that followed a planted link.** Found three separate times (saving, the index cache,
  view state and images, exports). Every place that writes a file has now been audited.
- **Limits that did not agree.** A document could be edited into a state that could not be saved;
  a 579-byte document with a text-doubling formula crashed the program on opening.
- **Operations that changed more than was asked.** Replace destroyed characters it had only
  partly matched and rewrote formulas.
- **Tests weaker than they looked.** Random tests that mostly generated refused commands, or
  compared two things that were wrong in the same way.

## What is not verified, and known limits

- **Nothing has been used by a person through an interface.** The proof of concept shows the
  foundation is sound, not that the product is pleasant to use.
- **Excel and Google Sheets have not opened the exports.** The lookup join was verified in
  LibreOffice only.
- **Windows and macOS have not run the code.** It was built and tested on Linux, and
  compile-checked for the web, Android and iOS.
- **Memory when opening a document** is about 13 times the file size, and the file size limit
  was lowered to 32 MiB to contain it. This is the limit most likely to hurt on a phone.
- **Search index memory** is around 8 times the size of the text for short cells.
- **The slowest search queries** (dozens of one-letter alternatives) take 80 to 94 ms at 100,000
  cells, over the 50 ms target, and about ten times that at a million cells. Typical queries take
  under a millisecond.
- **Document size ceilings:** about 414,000 empty cells per document through editing, and about
  330,000 formula cells before results show a limit error.
- **Formulas refer to cells by name**, as TreeSheets does, so renaming a table changes what a
  formula means. Links do not have this weakness. `milestones/M5.md` has a proposal.
- **A move between two documents is two undo steps**, one in each document.
- **No tolerance for typing mistakes in search**, no search by meaning, no search inside images.
- **Not built from TreeSheets:** XML and HTML export, XML import, images and PDF output,
  encryption, scripting.

## What a phone interface needs from the core next

The interface will be built in Flutter, starting with the nested grid on a phone (decision A121).
`UI_SPEC.md` specifies it and `research/` holds the background. The first piece of work, a minimal
bridge, is described in `NEXT_BRIDGE.md`.

1. **A bridge.** Commands and results are plain data shaped to cross a language boundary, but
   they have no serialised form yet, and no Flutter-to-Rust binding exists.
2. **A "what is on screen" query.** The core can hand over a whole grid; it cannot yet cheaply
   answer "the rows visible in this grid" for a very large one.
3. **Lower memory when opening a document**, by reading the file without building an
   intermediate copy.
4. **A first screen that tests all three at once:** one Flutter screen on a phone showing a real
   nested document, with drill-down, back, and editing one cell.

## Where things are

- `PLAN.md`: the milestones and the future list.
- `DECISIONS.md`: every design decision (section A) and every deliberate difference from
  TreeSheets (section B).
- `FILE_FORMAT.md`: the `.wombat` format.
- `milestones/`: one summary per milestone, with demos and each review's fixes.
- `../AGENTS.md`: the working rules for anyone changing the code.
