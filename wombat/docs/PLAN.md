# Wombat proof of concept: the plan

Wombat is a new product inspired by TreeSheets: documents are grids of cells, and any cell can
hold another grid, nested to any depth. It is not a continuation of TreeSheets and there are no
existing users to migrate.

**Goal of the proof of concept:** a Rust core with no interface that can create, edit, calculate,
link, search and save nested-grid documents. It is proven by automated tests and usable from a
small command-line tool.

The long-term aim is an app that is first-class on both desktop and mobile, in the app stores. The
interface will be built with Flutter on top of the core (decision A121).

## Milestones

| # | Milestone | What gets built | What you can try | Status |
|---|---|---|---|---|
| 0 | Ground rules | The plan, design decisions, future list and working rules in the repository; tests that run automatically on every change | Read the plan in the repository | done |
| 1 | Documents and file format | Nested-grid documents with permanent cell IDs; the `.wombat` file format; workspaces (a folder of documents); safe handling of damaged files | Create a document, save it, open the file in a text editor | done ([summary](milestones/M1.md)) |
| 2 | TreeSheets importer | Read-only import of the example `.cts` files, as a testing tool, checked against the real TreeSheets | Convert an example and print it as an outline | done ([summary](milestones/M2.md)) |
| 3 | Editing, undo and redo | Add, delete, move, sort and restructure cells; edit and style text; fold; every change recorded as a reversible step; the core reports what changed | Make a series of edits, undo them all, get the original back | done ([3a](milestones/M3a.md), [3b](milestones/M3b.md)); the findings of the review are fixed (end of [3a](milestones/M3a.md)) |
| 4 | Links and backlinks | Links that point at cell IDs, within and across documents; a list of what links to any cell | Rename a linked cell and see the link survive | done ([summary](milestones/M4.md)); the findings of the review are fixed (end of the summary) |
| 5 | Formulas | TreeSheets' formula system, checked against the real app | Change a number and watch dependent cells recalculate | done ([summary](milestones/M5.md)); the findings of the review are fixed (end of the summary) |
| 6 | Search service | Instant search across a workspace; results shown with their location trail; search within a branch; filters by tag, link and date; best matches first; a measured speed target | Search a large workspace and narrow it by tag | done ([summary](milestones/M6.md)); the findings of the review are fixed (end of the summary) |
| 7 | Import and export | Single-grid export; flattened export with the trail of parents; lookup-style export (one CSV per grid, ID columns placed for XLOOKUP); CSV and plain-text import | Export a nested document and join two tables with XLOOKUP | done ([summary](milestones/M7.md)); the findings of the review are fixed (end of the summary) |

Each milestone builds on the ones before it. When a milestone is finished its status changes here
and a summary is added under `docs/milestones/`.

All eight milestones are done. [`POC_SUMMARY.md`](POC_SUMMARY.md) says what exists, how far it
can be trusted, and what an interface needs next.

## How a milestone is judged finished

- The automated tests pass, including random-sequence tests and comparisons against the real
  TreeSheets where they apply.
- The command-line tool does what the "what you can try" column says.
- A short written summary (`docs/milestones/M<n>.md`) records what was built, what was left out
  and what went wrong.

## Known risks

- **Milestone 3 is the largest** and may be split in two.
- **Milestone 5 is the riskiest**, because the formula system is intricate.
- **Milestone 6 had an open question:** whether the Tantivy search library suits phones and web
  interfaces. It was tested first and does not (decision A70); the simpler purpose-built index
  is what was built.
- **No engineer reviews the code.** The tests and the comparisons against TreeSheets are the main
  safeguard. They reduce this risk without removing it.
- **Nothing here is visual.** The proof of concept shows the foundation is sound, not that the
  product is pleasant to use. How nested grids work on a phone screen is still an open question.

## Out of scope for the proof of concept

- Any interface, on desktop or phone.
- Sync, plugins, collaboration and AI features.
- File encryption, scripting and the Markdown converter.
- Search by meaning (embeddings) and search inside images.
- Typed cells such as dates. The file format leaves room for them.

## Future list

- A minimal bridge between Flutter and the core (`NEXT_BRIDGE.md`): built, run on Linux and on an
  Android emulator in CI, and reviewed four times, with what the reviews found fixed; running it
  on an Android phone and the measurements there are still to do, and what else is open is listed
  ([summary](milestones/B1.md)).
- A simple grid screen on the bridge, a first step towards the interface: one grid, into a nested
  grid and back, editing the text of a cell, undo and redo, and a document that saves itself. Run
  on Linux by tests; not yet seen by a person or tried on a phone. It is a stand-in, to be
  replaced by the first slice of the interface ([summary](milestones/UI1.md)).
- What the first screen of the interface needs under it (step 1 of the way to the first test on
  the owner's hardware): one read of the stage and everything nested below it, groups of commands
  that can be cancelled, the view state of the device, line breaks in cell text, and in the app
  the stage kept up to date for every screen, the editing of a cell, and the editor as a part
  that can be exchanged. Built and tested without a device; not reviewed, and nothing of it has
  run on a phone (decisions A166 to A180; [summary](milestones/B1.md)).
- The Flutter interface (`UI_SPEC.md`), Android first: its first slice is sections 3 to 8 (the
  depth budget, the phone screen, diving and going up, tapping, the editor with its toolbar,
  adding rows and columns), then a test by the owner on a phone. A native text editor on iOS
  is kept possible and not built.
- Sync between devices.
- A spreadsheet import/export wizard: a single Excel workbook with linked sheets, ready-made
  XLOOKUP formulas, and flattening of any number of tables.
- A Markdown converter.
- Plugins, once the core's instructions have settled.
- Search by meaning.
- Tolerance for typing mistakes in search (it did not arrive with milestone 6: decision A78).
- A trademark and app store check on the name "Wombat" before anything goes public.

Design decisions are recorded in `DECISIONS.md`. Working rules for anyone (person or AI) changing
the code are in `../AGENTS.md`.
