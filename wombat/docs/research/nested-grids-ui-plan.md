> Background research, not instructions. The interface specification is `../UI_SPEC.md`; where this file differs from it, the specification wins.

# Wombat's nested grids on phone, tablet and desktop: a design plan

Date: 11 November 2026. Written for the product owner. It is a plan, not a specification and not code.

It builds on two earlier reports in the same folder. Read them for detail this plan doesn't repeat:
- `nested-grids-mobile.md`: prior art, a Workflowy deep dive, six ranked concepts and a test plan.
- `react-native-comparison.md`: the toolkit comparison, plus the idea of an iOS-style
  long-press preview.

What's new here: fresh research on Tana and on Workflowy's 2025–2026 changes, a direct answer to the
"Workflowy with tables" worry, user journeys, ten new ideas, a ranking that covers everything, how the
design grows to larger screens, and a step-by-step plan.

**How claims are labelled.**
- **[Evidence]**: research, a standard or official documentation, with a link.
- **[Product]**: how a shipping product behaves, from its docs, changelog or user reviews. User
  reviews are anecdotal.
- **[Idea]**: my own design reasoning. It is untested, and the usability tests are what will prove
  or disprove it.

---

## 0. The plan in one page

**The main problem.** TreeSheets' magic is that you can see the nesting at a glance: grids inside
grids, several levels deep, on one screen. A phone can't show five levels at a readable size. The
easy fix is one level per screen, but that throws away what makes Wombat different and turns it into
"Workflowy with tables".

**The answer: a stage with a depth budget, not one level per screen.**
- One grid is always **the stage**: the level you are working in, at full size.
- Nested grids inside it are drawn **as deeply as they stay readable and tappable**:
  - shown in full, if they fit;
  - otherwise as a small but legible **preview**;
  - otherwise as a **shape thumbnail**, a tiny sketch of the child grid's rows, columns and colours.
- That rule is the **depth budget**. A phone usually affords one and a half to two levels, a tablet
  two to three, and a desktop three to five, which is what TreeSheets already shows.
- There is **one way down: dive.** Tap the nested grid drawn inside a cell (its preview, shape or
  badge) and it grows into the screen and becomes the stage. Tapping the cell's own text selects
  it, and a second tap edits it. "Tap what you want to work on" is the whole rule. Small children
  already show in full and can be edited where they are, so there's no separate "open in place".
- Long-press gives the cell's menu, which shows a larger preview on top: a free look without
  leaving, but not a separate verb anyone has to learn.
- A thumb-reachable **trail bar** at the bottom always says where you are, and the system back
  gesture always takes you up one level.

The structure stays visible. Only the amount of detail adapts to the screen.

**What to borrow, in one line each:**
- **Workflowy**: zoom into any item, breadcrumbs, search that filters in place, the toolbar above the
  keyboard, multi-select with a bottom bar and "Move to", and "layout is a property of a node".
- **Tana**: the same data shown through different views, "fields are like the columns of a
  database", typed templates, and live searches that act like tables. Also its biggest lesson: keep
  power out of the way, because its learning curve and its weak phone app are the top complaints.

**The first build is boring and solid:** the stage, previews, diving by tapping a nested grid, the
trail bar, the keyboard toolbar, a single editor for one cell, "Move to…" with a pocket, search
that jumps to the right level, and undo.

**After that:** lenses (list, card and board views of a grid), a mini-map, recent places, split
view on tablets, and live search grids. "Show here" (opening a child in place beyond what the
depth budget allows) and a visible peek control come back only if the tests ask for them (§3.3).

**Experiments:** fisheye, a continuous zoomable canvas on phones, Tana-style typed templates, and
drag-to-insert.

**Steps:**
1. Paper sketches
2. A clickable prototype in Flutter on a real iPhone, with the typing test on day one
3. Three rounds of usability tests with five people each
4. Build on the Rust core
5. Widen to tablet and desktop
6. A beta

Each step has questions to answer and a pass mark (§8).

---

## 1. Design principles

Each principle comes with the reason behind it. When two ideas conflict, these decide.

1. **Show the shape, adapt the detail.**
   - Nesting is always visible in some form: the full grid, a preview, or a shape thumbnail. It is
     never simply hidden.
   - *Why:* it is the product's reason to exist. Recognition beats recall: people hold only about
     four chunks in working memory, so contents they can see beat contents they must remember
     ([Nielsen heuristic 6](https://www.nngroup.com/articles/ten-usability-heuristics/)).
2. **One stage at a time, never one level at a time.**
   - Only one grid is fully interactive, but its children show as much as the depth budget allows.
   - There is one way into a child that isn't fully shown: dive. One verb is easier to learn and
     harder to confuse than several that differ only by how far they go.
   - *Why:* it keeps tapping precise (one clear set of targets) without hiding the structure. Every
     extra verb is another control to find, another thing to explain and another way to tap the
     wrong one (§3.3).
3. **You always know where you are, and one tap gets you back.**
   - The trail bar is on every screen. The system back gesture goes up one level. Animated
     transitions show where you came from.
   - *Why:* "desert fog", getting lost in zoomable spaces, is the best-documented failure of nested
     and zoomable interfaces
     ([Jul & Furnas, via Bederson](https://cs.umd.edu/~bederson/images/pubs_pdfs/p23-bederson.pdf));
     some users get lost after zooming on small screens
     ([Büring & Reiterer, MobileHCI 2005](https://typeset.io/pdf/usability-of-overview-supported-zooming-on-small-screens-58zou4wwcf.pdf)).
4. **Every action has a visible button; gestures are shortcuts.**
   - Pinch, long-press and swipes speed things up for people who know them. Nothing depends on them.
   - *Why:* hidden gestures lack signifiers
     ([NN/g on contextual swipe](https://www.nngroup.com/articles/contextual-swipe/)), and Apple says
     context-menu actions should also be reachable elsewhere
     ([HIG, context menus](https://developer.apple.com/design/human-interface-guidelines/context-menus)).
     Workflowy users hit accidental swipe-completes, and Logseq and Obsidian users couldn't find indent
     (earlier report §2).
5. **Big thumbs, bottom controls.**
   - Targets are at least 44 points; main controls sit in the bottom third of the screen.
   - *Why:* Apple's and Google's guides ask for 44 pt and 48 dp
     ([summary](https://tetralogical.com/blog/2022/12/20/foundations-target-size/)), WCAG 2.2 sets 24 px
     as the floor and 44 px as the enhanced level
     ([W3C](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)), and most phone use
     is one-handed or cradled
     ([Hoober](https://uxmatters.com/mt/archives/2013/02/how-do-users-really-hold-mobile-devices.php)).
6. **Tables stay tables.**
   - A grid shows as a grid by default. Other views are a choice, never an automatic reflow.
   - *Why:* reflowing tables into blocks harms scanning and comparison
     ([Hoober](https://www.uxmatters.com/mt/archives/2020/07/designing-mobile-tables.php),
     [NN/g](https://www.nngroup.com/articles/mobile-tables/)).
7. **Views change the look, never the data.**
   - Lenses, folding, zoom and selection are per-device view state (decision A8). Switching a lens
     never edits the document.
   - *Why:* Tana and Workflowy both promise that a view only changes how things look
     ([Tana view toolbar](https://outliner.tana.inc/blog/a-flexible-alternative-to-rigid-databases-meet-the-new-view-toolbar-in-tana),
     [Workflowy tables](https://community.workflowy.com/t/tables/1647)), and it is what makes
     switching views feel safe.
8. **Restructuring is explicit first, magic second.**
   - Moving cells between levels works with "Move to…" and a pocket before it works by drag.
   - *Why:* restructuring is the weakest part of every mobile outliner, and Workflowy's 2025 answer was
     multi-select with an explicit Move action
     ([Workflowy what's new](https://workflowy.com/whats-new/)).
9. **Everything can be undone, and undo is easy to reach.**
   - *Why:* people explore more boldly when mistakes are cheap (Nielsen heuristic 3). The core already
     records every change as a reversible step (A9).
10. **Calm by default, power on request.**
    - A new user sees cells, grids and a search box. Tags, formulas, lenses and templates appear when
      used.
    - *Why:* Tana's power is loved, but "a huge learning curve" is its most common complaint
      ([review roundup](https://blog.saner.ai/tana-reviews)), and a G2 reviewer said their first week
      was "mostly configuring the tool"
      ([summary of G2/DEV reviews](https://dev.to/pickuma/tana-review-an-outliner-based-pkm-tool-that-thinks-in-graphs-8l8)).
11. **Native manners on each platform.**
    - iOS back swipe, Android back, long-press menus, standard sheets, Dynamic Type and system
      haptics.
    - *Why:* platform consistency (Nielsen heuristic 4). It also matters for app-store reviews.
12. **Fast feels trustworthy.**
    - A dive starts within one frame, and typing never lags.
    - *Why:* Workflowy spent 2025–2026 fixing toolbar lag and the keyboard covering the edited
      item, which shows that users notice
      ([what's new](https://workflowy.com/whats-new/)).

---

## 2. Inspiration: what to borrow, what to avoid, and how it maps onto grids

The earlier report covers Workflowy in depth. This section adds Workflowy's 2025–2026 changes, a full
Tana study and the others, always asking: **how does this work with rows and columns, not just a
list?**

### 2.1 Workflowy (updated)

**What's new since the earlier report [Product, [what's new](https://workflowy.com/whats-new/)]:**
- **Multi-select on mobile.**
  - Launched 17 September 2025 and open to everyone from 7 October 2025.
  - The bulk actions are change type, duplicate, move, mirror and export, plus "Move to Date"
    (23 October 2025).
  - Since 7 October 2025, long-pressing a selected item starts a drag.
  - Fixes followed: items keeping their order after a move (4 November 2025) and more reliable drag
    of a multi-selection (19 November 2025).
- **Tables launched** on 30 June 2026.
  - Each child is a row, and the row's children are its cells.
  - A row called "Header" names the columns.
  - Cells are full items and can hold nested items, but a cell with nested content can't be edited
    inside the table and is marked with a left border.
  - Column widths can be changed only on desktop
    ([tables guide](https://community.workflowy.com/t/tables/1647)).
- **Panes** (side-by-side views) arrived on desktop on 7 September 2026, with "no mobile support
  (yet)".
- **Mobile polish:**
  - Board columns got a minimum width (November 2025).
  - Board scrolling was fixed (April 2026).
  - Swipe left now offers "Move to Tomorrow" and "Next Week" (May 2026).
  - A crash when zooming into an item with very many children was fixed (May 2026).
  - Toolbar buttons "now update immediately. No more lag" (September 2026).

**Borrow:**
- Zoom with breadcrumbs.
- Search that filters the view in place.
- The toolbar above the keyboard.
- The selection bar with Move and Duplicate.
- "Layout is a property of a node": boards and tables are layouts of an item, not a different kind
  of data.
- Marking cells that hold nested content.
- The "+" row at the bottom of a table.

**Avoid:**
- Swipe actions on cells. Grids need horizontal scrolling, and Workflowy's own boards had
  swipe-versus-scroll conflicts.
- Relying on drag alone.
- Desktop-only features in a phone-first product (panes, column resizing).

**On grids:** Workflowy's table is a one-level grid whose nested cells are opened rather than shown.
Wombat goes further: every grid can be nested, and nested grids are *shown* (as a preview or a
thumbnail), not just marked with a border. That is the visible difference from Workflowy.

### 2.2 Tana

**What it is [Product]:**
- **Everything is a node.** Even a supertag's definition is a node
  ([supertags](https://outliner.tana.inc/learn/features/supertags)).
- **Supertags turn a node into an object.**
  - Applying `#person` or `#task` stamps the tag's template of fields onto the node.
  - All nodes with that tag form a collection. The tag's page, by default, is a search for all of
    them shown as a table: "Each row in a database represents one node. Each column represents the
    Fields of those nodes."
  - A tag can **extend** another tag ("think of it as a projection"), and a node can carry several
    tags (same source).
- **Fields** are "like the columns of a database."
  - In the outline they appear nested under a node when it is expanded.
  - In a table each field is a column.
  - There are nine field types, including options, date, number and checkbox. Number fields can be
    totalled in tables
    ([fields](https://outliner.tana.inc/learn/features/fields)).
- **Views of the same node:**
  - Outline, Table, Cards, Lanes, List (two panels), Calendar, Side menu and Tabs.
  - In a table, children are rows and fields are columns. Adding a column adds a field. Columns can
    show totals, averages and counts
    ([views](https://outliner.tana.inc/learn/features/views)).
  - A view toolbar under the node offers Search, Filter, Sort, Group and Display. It is hidden by
    default. Changing a view "changes only how information is shown, not the underlying data"
    ([view toolbar post](https://outliner.tana.inc/blog/a-flexible-alternative-to-rigid-databases-meet-the-new-view-toolbar-in-tana)).
- **Search nodes** are live searches placed anywhere in the outline.
  - Their results update as matching nodes change.
  - They can be shown in any view, which makes them dashboards
    ([search nodes](https://outliner.tana.inc/search-nodes)).
- **The mobile app**
  ([Tana mobile](https://outliner.tana.inc/learn/features/tana-mobile)):
  - It is built around **capture**: writing, voice, camera and files sent to Today, Inbox or a
    pinned node, plus widgets, Siri, Shortcuts and the Action Button.
  - You can browse, search, tag and fill fields.
  - "View options are not yet available on mobile." Editing is an early-access feature on iOS. Search
    doesn't support every query, because complex searches need the whole graph loaded, which "isn't
    feasible on mobile for performance reasons".

**What users say [Product, anecdotal; the sources are review sites of uneven quality]:**
- Praise:
  - "I love the Supertags and I love how you can use your database in different views."
  - Tana "is the first tool that truly adapts to how my brain works"
    ([Saner roundup](https://blog.saner.ai/tana-reviews), a competitor's page).
- Complaints:
  - "Huge learning curve", with estimates of two to three weeks before feeling productive.
  - A first week spent "mostly configuring the tool"
    ([DEV review](https://dev.to/pickuma/tana-review-an-outliner-based-pkm-tool-that-thinks-in-graphs-8l8)).
  - Price.
  - Mobile "lags the desktop experience". If you need to edit complex supertag structures on the go,
    "the mobile experience disappoints"
    ([vantaige](https://vantaige.io/ai-tool/tana)).
  - "There is still no easy way to navigate to the inbox" (Saner roundup).
  - Some reports of slowdowns with dense links.

**Borrow:**
- **Same data, many lenses.** A grid can be shown as a grid, a list, cards or a board on this device,
  without changing the document.
- **Header row as fields.** When a grid has a header row, each row below is a record and the header
  names its fields. Card and list lenses can then label each value, and a column can show a total
  (TreeSheets already has formulas for this).
- **Live search as a grid.** A saved search shown as a grid of results with their location trail,
  editable in place (§5, idea F7).
- **Capture on the phone.** A share-sheet and widget path into an "Inbox" grid is cheap and loved.
- **Pinned fields.** In a wide grid, let the user pin the columns that matter, as Tana pins fields.

**Avoid:**
- Front-loading a schema. Templates must be optional and appear late.
- Hiding view options on mobile. If lenses exist, they must work on the phone.
- Search that can't work on a phone. Wombat's core indexes on the device (milestone 6), which can be
  a real advantage over Tana here.
- Two products with similar names (Tana versus Tana Outliner confused reviewers).

**On grids:** Tana makes rows and columns *out of* a tree: children become rows, fields become
columns. Wombat starts from grids, so the mapping runs the other way.
- A Wombat grid with a header row *is* a Tana-style table.
- A cell holding a grid *is* a node with fields.
- A tag that stamps a sub-grid template *is* a lightweight supertag (idea F9, an experiment).

The important difference is that Wombat's columns are free-form and positional, not typed fields. Keep
it that way at first: it is TreeSheets' charm, and it avoids Tana's schema burden.

### 2.3 Other products worth borrowing from

**Notion [Product].**
- How it works:
  - On mobile, pages open in a stack, and the home tab shows nested pages with toggles.
  - Database pages can switch between their views.
  - "Move to" opens a page picker ([Notion help](https://www.notion.com/help/workspaces-on-mobile)).
  - Multi-column layouts collapse to one column on phones
    ([third-party guide](https://www.getinflow.io/post/notion-for-overwhelmed-beginners-mobile-phone-advice)).
- **Borrow:** the "Move to" picker.
- **Avoid:** silent collapse of layouts, and slow loading of deep pages.
- **On grids:** Notion's toggles reveal nesting in place, which suits a list. In a grid, the depth
  budget's Full form does that job without a toggle on every cell (§3.3).

**Logseq and Roam [Product].**
- How it works:
  - Block references link to a block, and embeds show it inline.
  - Logseq caps embed rendering at two levels deep
    ([Logseq forum](https://discuss.logseq.com/t/the-difference-between-logseqs-block-embeds-and-block-references/8459)).
  - Linked references collect backlinks under a page.
- **Borrow:**
  - A backlinks panel under a cell, with each backlink shown with its location trail.
  - A depth cap on embedded content, the same idea as the depth budget.
- **Avoid:** the missing indent and toolbar problems on phones.

**Apple Numbers and Excel on phones [Product].**
- How it works:
  - In Numbers you tap a cell to select it, drag blue dots to extend the selection, tap a row number
    or column letter to select a row or column, and double-tap to edit
    ([Apple guide](https://support.apple.com/guide/numbers-iphone/select-cells-rows-and-columns-tan4deb6ba50/ios)).
  - Numbers also has a form view, one row at a time (earlier report).
  - Excel's formula bar is the safe way to edit (earlier report).
- **Borrow:**
  - Tap to select and tap again to edit.
  - Selection handles.
  - Row and column header bars as targets for whole-row and whole-column actions.
  - A form view as the "card" lens.
- **Avoid:** editing inside tiny cells, and the extra tap before typing that users complain about.

**MindNode [Product].**
- How it works: focus mode "highlights only the branch you're working on while fading out the
  rest" and zooms the canvas to it
  ([MindNode guide](https://www.mindnode.com/support/guides/focus-mode),
  [TapSmart](https://tapsmart.com/features/deep-dive-mindnode)). Its makers added an outline because
  finding things on a canvas on a small screen is hard (earlier report).
- **Borrow:** dim instead of hide (desktop focus spotlight, idea F8), and an alternative
  one-dimensional view.

**Muse and Heptabase [Product].**
- How they work:
  - In Muse, tapping a board enters it, landing where you tapped (earlier report).
  - Heptabase has supported card editing on mobile whiteboards since 2024
    ([roadmap/changelog](https://wiki.heptabase.com/changelog/2024)), but reviewers call arranging a
    whiteboard on a phone limited.
- **Borrow:** "enter where you tapped" continuity.
- **Avoid:** free spatial arrangement as the main phone interaction.

**iOS Files, Mail and Photos [Product].**
- How they work: long-press shows a live preview plus a short menu; tapping the preview opens it. Apple
  advises keeping previews lightweight and keeping actions reachable elsewhere (HIG, cited above).
- **Borrow:** tap a container to open it; long-press for the platform's preview plus menu (idea C3).

**Things 3 [Product].**
- How it works:
  - The **Magic Plus** button can be tapped or dragged: dropped into a list, it inserts a to-do at
    that exact spot; dropped in the left margin, it makes a heading; dropped on a day, it schedules
    the to-do
    ([Cultured Code](https://culturedcode.com/things/features/),
    [MacStories](https://www.macstories.net/?p=48965)).
  - Its reviews praise its delight: careful motion and satisfying haptics.
- **Borrow:**
  - A single, thumb-placed "+" that can also be dragged to say *where* (idea F10, an experiment).
  - Polish as a feature.

**TreeSheets itself [Product, from `src/`].**
- How it works:
  - Discrete zoom by path, an ancestor header, auto-zoom to tiny targets, folding, and filters by
    tag, text and style.
  - Structural commands such as Wrap, Flatten, "Hierarchify", Hierarchy swap and Transpose.
  - Three drawing styles: grid, bubble and line.
- **Borrow:** all of these as the command vocabulary. They are proven on desktop. On a phone they
  live in the selection bar's "More" menu.

---

## 3. The "Workflowy with tables" problem, answered

The question: **how much nesting can a phone show at once while staying readable and tappable?**

### 3.1 The numbers that set the limit [Evidence + Idea]

- **Width.**
  - A typical iPhone is about 390 points wide; Android phones in portrait are under 600 dp
    ([window size classes](https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes)).
  - With 44 pt targets, that leaves room for **3–4 readable text columns** at the stage.
- **Text size.**
  - Apple's smallest standard text style is about 11 points.
  - Below roughly 9–10 points, text stops being readable for many people at arm's length.
    [Idea: these thresholds should be tuned in testing, and they must grow with the user's
    large-text setting.]
- **What follows:**
  - A child grid drawn inside a stage cell at about 70% scale is still readable, as a **preview**.
  - At 50% it is readable only for very short text.
  - Below that it is a **shape**, not text.
  - Taps inside a preview can't target individual child cells; the whole preview is one target.

### 3.2 The depth budget [Idea, core]

Draw each nested grid at the richest form that still passes two tests:
1. **Can people read it?** Its text stays at or above the smallest readable size.
2. **Can people tap it?** Every tappable thing stays at least 44 points.

If a nested grid fails the tests, step down to the next form:

| Form | What you see | Tappable? |
|---|---|---|
| **Full** | The child grid in place, at near-full size, editable | Yes, each cell |
| **Preview** | The first few child cells' text, smaller, with "3×4" and a fade if cut off | One target: tap to dive |
| **Shape** | A miniature of the grid's rows and columns, colours and how full it is, with no text | One target: tap to dive |
| **Badge** | Just a small grid icon and a count ("12") | A 44-point target at the cell's edge: tap to dive |

**What each device typically affords:**
- **Phone:** Full for small children (a 2×2 of short words), Preview for most, Shape for
  grandchildren. That is about **one and a half to two levels visible at once**: more than Workflowy
  shows, and honest about the screen.
- **Tablet:** two to three levels.
- **Desktop:** three to five, which is TreeSheets territory. It is the **same rule** with a bigger
  budget. TreeSheets already shrinks nested grids and auto-zooms into tiny ones (`relsize`,
  `ZoomTiny` in `src/document.h`), so this generalises what TreeSheets does rather than inventing
  something new.

**Why it beats both extremes:**
- Versus one level per screen: you still see the structure and what's inside, which keeps the "aha"
  of TreeSheets.
- Versus shrinking everything (TreeSheets on a phone): nothing is ever drawn smaller than you can
  read or tap. Deeper content becomes a shape, not illegible text.

### 3.3 Going deeper: one verb, dive

**The rule: tap what you want to work on.** [Idea, core]

A cell that holds a grid is drawn as two parts: its own text on top, and its child grid below in
whatever form the depth budget chose (TreeSheets draws a cell's text above its grid the same way).

| What you tap | What happens |
|---|---|
| The cell's text | Selects the cell. Tap again (or Edit) to edit its text. Same as any cell |
| A **Full** child's cells | Selects that child cell directly. A Full child is already "open", so no dive is needed |
| A **Preview** or **Shape** | **Dive**: the cell grows into the screen and its grid becomes the stage |
| A **Badge** | **Dive**, the same way |
| "Open ›" in the bottom bar, with the cell selected | **Dive**. The labelled twin of tapping the child, and the way in when the cell is too small to split |
| Long-press anywhere on the cell | The cell's menu, with a larger preview of the child on top (see below) |

This is how TreeSheets already works with a mouse: clicking inside a nested grid acts on that grid.
Diving is the phone's way of making a too-small grid big enough to click into. TreeSheets' own
"zoom in" (`A_ZOOMIN`, Ctrl+PgUp or Ctrl+mouse wheel in `src/tsframe.h`) does the same thing: it
makes a deeper grid the root of the view.

**Where things sit on a phone.** [Idea, to tune in paper tests]
- The text part and the child part are each at least 44 points tall, and the child part is drawn
  with a visible frame (a mini-grid or shape), so the two read as different things.
- A Badge sits at the cell's trailing edge with a 44-point hit area, even when the icon is smaller.
- If a cell is too small to give both parts 44 points, it becomes **one target**: tap selects, and
  "Open ›" appears in the bottom bar. No chevron or triangle is drawn on the cell, so columns keep
  their width (the stage only has room for 3–4 columns, §3.1).
- *Evidence:* when an icon sits next to a label, people tap either one: with most icons there was no
  preference, and even with the best icon 29% of taps landed on the label, so NN/g advises against
  giving them different actions ([NN/g, accordion icons](https://www.nngroup.com/articles/accordion-icons/)).
  That is why there is no small "dive" icon beside the text: the split is between two large,
  visibly different regions, not between a word and an icon.

**Editing a cell that has a grid, without accidental dives.**
- Tap the text to select, tap again to edit. The editor docks above the keyboard (J6); the child
  stays drawn in the cell. The editor toolbar has "Open grid ›" for diving straight from editing.
- **While editing, a tap outside the editor only ends editing.** It never dives. Dives start only
  from the resting state.
- *Evidence:* Dynalist users on phones "constantly unwantedly" zoom in by hitting the bullet while
  trying to edit or tick a checkbox, lose their place, and asked for a way to turn tap-to-zoom off
  ([Dynalist forum](https://talk.dynalist.io/t/prevent-accidental-zoom-when-tapping-a-checkbox-on-mobile/4586),
  [request for zoom options](https://talk.dynalist.io/t/android-app-request-options-to-set-zoom-in-settings/8260))
  [Product, anecdotal]. A large framed target, no dive while editing, and an instant way back
  (system back, which restores the exact scroll position, §3.4) are the three defences.

**Long-press: a look without a verb.**
- Long-press opens the cell's menu (Move to…, Group, Fold, Style, Delete). The top of that menu shows
  the child grid larger than the preview, so you can read it without leaving.
- **iOS:** the platform context menu, which since iOS 13 replaced 3D Touch "peek and pop": a
  preview plus actions, opened by long-press on every device
  ([Use Your Loaf](https://useyourloaf.com/blog/adding-context-menus-in-ios-13/),
  [Apple deprecated symbols](https://developer.apple.com/documentation/uikit/deprecated-symbols))
  [Evidence]. Flutter ships this as `CupertinoContextMenu`
  ([Flutter API](https://api.flutter.dev/flutter/cupertino/CupertinoContextMenu-class.html)).
  Tapping the preview dives.
- **Android and others:** Android has no preview-on-press convention, so it is a bottom sheet with the
  same actions and the larger preview at its top. Tapping the preview dives.
- This is not taught as a separate "peek" action and needs no icon on the cell. It costs nothing
  extra, because the menu exists anyway for Move to… and the other actions.

**Fold is view state, not a way down.** [Idea, from TreeSheets]
- Fold (in the long-press menu; Ctrl+F10 or Ctrl+double-click on desktop, as in TreeSheets) shrinks a
  child to a Badge on this device, to declutter. Unfold returns it to whatever the depth budget
  chooses. It never makes a child bigger than the budget allows.

**Gestures are accelerators.** Pinch out on a cell dives; pinch in goes up one level (later also to
the overview, F12). Both have visible twins (the child itself, "Open ›", ‹ in the trail bar).
Double-tap is not used for diving.

**Why one verb, and the alternatives weighed.**

| Option | For | Against | Verdict |
|---|---|---|---|
| **Three verbs:** open in place (triangle), peek (icon or long-press), dive (chevron) | Each depth of commitment has its own control | Three controls per nested cell on a 3–4-column screen; on a phone the triangle would sometimes open and sometimes dive (when the child is too wide), the same control doing two things; open in place reflows rows *and* columns in a grid, not just lines as in an outline | Cut |
| **Open in place as the main verb, dive as escalation** (Workflowy, Notion toggles) | Familiar from outliners; keeps siblings in view | In a list, expanding pushes lines down; in a grid it changes row heights and column widths, so the whole stage jumps. Most children don't fit a phone's width, so it would fall back to dive most of the time | Cut on phones; on larger screens the bigger depth budget already shows children in full |
| **Two verbs: dive and peek** | Peek checks contents without losing place | Previews already show the contents; going back is one gesture and lands exactly where you were; Android has no peek convention; a visible peek icon costs a target per cell | Peek kept only as the long-press menu's preview, not as a verb |
| **One verb: tap the nested grid to dive; tap text to edit; long-press menu with a preview** | One rule, no extra icons, matches TreeSheets' "click where you want"; the menu is needed anyway | Loses quick edits of a medium-sized child while seeing its neighbours; relies on the text and child regions being clearly different | **Chosen** |

Evidence behind the choice:
- **Tana's phone app** uses a similar split: "Tap on a node to open it/zoom in", "Tap on the content
  of a node to go into edit mode", "Long-press for all options", with a separate ">" to expand in
  place ([Tana mobile](https://outliner.tana.inc/learn/features/tana-mobile)) [Product]. Its
  editing on phones is still early access, so it says little about edit-heavy use.
- **Workflowy** has both: clicking the bullet zooms, the triangle shows or hides children
  ([Workflowy bullets help](https://community.workflowy.com/raw/1663)) [Product]. That works in a
  one-dimensional list, and its bullet is the small target behind Dynalist-style accidental zooms.
- **iOS Files, Finder's column view, Things 3 projects and Apple Numbers** [Product, from using
  them; Numbers via the Apple guide cited in §2.3]: in file and
  project browsers, tapping a container opens it, and long-press gives a preview and menu; Numbers
  keeps "tap selects, tap again edits" for cells. Wombat combines the two by region.
- **Fewer choices are faster and less error-prone** [Evidence, general]: decision time grows with the
  number of options (Hick's law), and mode errors happen when the same action means different things
  in different states, so commands should be distinct and states visible
  ([Nielsen on progressive disclosure](https://jakobnielsenphd.substack.com/p/progressive-disclosure),
  [MIT 6.813 on safety and modes](https://web.mit.edu/6.813/www/sp17/classes/04-safety)).
- **Apple advises one disclosure button per view** because more add complexity
  ([HIG, disclosure controls](https://developer.apple.com/design/human-interface-guidelines/disclosure-controls))
  [Evidence]; a triangle on every nested cell is the opposite.

**What is lost, honestly.**
- **Editing a medium child next to its siblings on a phone.** A child too big for Full but small
  enough to fit if pushed (say a 3×3 list) must be dived into to edit. Each edit is dive, edit, back:
  two extra steps, and the neighbours are out of sight while editing.
- **Checking without moving, for people who never long-press.** They must dive and come back.
- **A familiar outliner gesture.** Workflowy and Notion users may look for an expand arrow.

**What is deferred, and what would bring it back.**
- **"Show here"** (open a child in place, one step richer than the budget, where it fits). It returns
  first as an item in the long-press menu, and only later, if needed, as a visible control. Bring it
  back if the tests show any of: frequent round trips (dive, small edit, back within about
  20 seconds) on small children; people tapping a preview and saying they expected it to open "here";
  or people asking to see a child next to its siblings while editing.
- **A visible peek control.** Bring it back if many dives are "look and leave" (back within a few
  seconds with no action) and people find that tiring, or if few people ever find the long-press
  preview.

Tests check that people predict what tapping a preview, a shape and a badge will do, how often they
dive by accident, and the signals above (§8).

### 3.4 Where am I? Never getting lost

- **The trail bar**, at the bottom of the screen within thumb reach:
  - It shows `‹  Parent name › Current name` plus a small depth indicator: dots, one per level.
  - Tap ‹ to go up one level. Tap the names to see the full path as a list and jump to any ancestor.
  - Long-press ‹ to see recent places (the iOS back-button convention).
  - Breadcrumbs that wrap onto several lines hurt on mobile
    ([NN/g](https://www.nngroup.com/articles/breadcrumbs/)), so it is one line, and the full path is
    one tap away.
- **The stage title** at the top is the dived-into cell's text, like Workflowy's zoomed title, with
  its colour as a thin accent stripe, so levels look different.
- **Animated continuity:**
  - When diving, the cell grows into the screen. When going up, the screen shrinks back into its
    cell, which flashes briefly so your eye finds it.
  - Workflowy users asked to keep the zoom animation because it makes the app "come alive" (earlier
    report).
- **Ghost of the parent:** a faint strip at the top edge shows a sliver of the parent grid around
  the current cell, to remind you that you are inside something. [Idea, experiment]
- **Remembered places:** each level remembers its scroll position and selection on this device (A8),
  so going up lands you exactly where you left.
- **Jumps** (from search, a link or a backlink) land on the deepest level that shows the target
  readably (TreeSheets' rule). The trail bar animates to the new path, and the target cell pulses
  once.
- **Mini-map** (idea F4): one tap shows the whole document as nested boxes, with "you are here" lit
  up.

---

## 4. The core experience, as journeys

The persona for these journeys: Sam keeps a "Trip planning" document. The top grid has Days down the
side and Morning, Afternoon and Evening across. Many cells hold small grids: a list of options, a
budget table, a packing list.

**J1. Opening a document.**
- Sam opens the workspace and taps "Trip planning". It opens where Sam last was on this phone (view
  state, A8). A first-time open starts at the top.
- The top grid fills the screen: three columns of readable cells, with the first column and header
  row pinned when the grid scrolls.
- Cells with children show previews ("Louvre · Orsay · Rodin", plus a small 1×3), or shapes for
  bigger ones.
- The trail bar says "Trip planning".
- *Principles:* status visible, shape shown.

**J2. Getting oriented.**
- Sam scrolls. The pinned headers keep the axes visible (NN/g on mobile tables).
- A cell with a busy shape stands out. It is a 6×4 budget table, and its thumbnail shows six rows of
  numbers in green.
- Sam pinches in a little. The stage becomes an **overview** in which every cell is drawn as its shape
  thumbnail, like semantic zoom on a map. Sam spots the packing list in Day 5 Evening and taps it to
  return to normal size centred on that cell.
- *Evidence:* semantic zoom can provide richer navigation cues than a separate overview
  ([Hornbæk et al., TOCHI 2002](https://scilit.com/publications/ad4f4bae6e3d5d7df2180827b57daa71)).
  This is an accelerator, with "Overview" also in the view menu.

**J3. Diving in and out.**
- "Day 2 Morning" holds a 1×3 list of short words, so it is already drawn in full. Sam taps one item
  in it and edits it there, without diving.
- Sam taps the preview inside "Budget". The cell grows into the screen, the title reads "Budget", and
  the trail reads "‹ Trip planning › Budget".
- A swipe from the left edge (iOS) or the back gesture (Android) goes up, and the Budget cell
  flashes in its place.
- Back at the top, Sam long-presses "Day 4 Evening". The menu shows its restaurant list larger, Sam
  reads it, and dismisses the menu without leaving the stage.

**J4. Seeing context while deep inside.**
- Four levels down, Sam wonders which day this belongs to and taps the trail. A sheet lists the path
  ("Trip planning › Day 3 › Evening › Restaurants") with each level's shape thumbnail.
- On a tablet, the parent is simply visible in a pane on the left (§6).

**J5. Finding something.**
- **Search:**
  - Sam taps the search button in the bottom bar and types "museum".
  - The current stage **filters in place**: non-matching cells dim, rows without matches collapse,
    and cells whose children contain matches get a count badge ("2 inside").
  - Below, a "Elsewhere in this document / workspace" list shows other matches, each with its
    location trail. Tapping one jumps there (journey rule in §3.4).
  - This is Workflowy's filter-in-place adapted to two dimensions.
- **Tags:** tapping a `#booked` chip in any cell starts the same filter. Filters combine, and
  `-#booked` excludes.
- **Backlinks:** the cell's info sheet shows "Linked from 3 places", each with its trail, the way
  Logseq lists linked references.
- **Quick switcher:** type a few letters of any grid's title to jump there (Obsidian style).

**J6. Editing a cell.**
- Tap selects (a clear outline and a light haptic tick). Tap again, or tap Edit, to edit. With a
  hardware keyboard, typing on a selected cell starts editing straight away.
- If the cell holds a grid, tapping its text edits the text; the child stays drawn below it. While
  editing, tapping elsewhere only ends editing and never dives (§3.3).
- The **single editor** docks above the keyboard, with a **toolbar** above it: undo, redo, new row,
  new column, open the cell's grid, link, tag and style.
- The cell stays visible above the keyboard and updates live. Workflowy needed a fix so the
  keyboard stops covering the edited item, so treat this as a must-have.
- Return commits and moves down (the spreadsheet habit). A toolbar arrow moves right. A new line
  inside a cell is an explicit button.

**J7. Adding rows and columns.**
- When a grid is selected, faint "+" bars appear after the last row and column, as in Workflowy's
  table and Tana's placeholder row. Tapping one adds a row or column and starts editing its first
  cell.
- In the middle of a grid: select a row's header bar (Numbers style) and choose "Insert above" or
  "Insert below".
- Experiment: drag a floating "+" to the exact gap, Things' Magic Plus (idea F10).

**J8. Restructuring.**
- **Move to another level:**
  - Select cells (tap, then "Select" for more, with handles to extend). The bottom bar offers
    Move to…, Cut, Duplicate, Link, Style and More.
  - **Move to…** opens a picker that browses the document with the same stage-and-preview look and
    offers clear drop slots: "Into this cell (new grid)", "Before or after this row", "As a new
    column".
  - Or **Cut** puts the cells in the **pocket**, a small chip above the bottom bar ("2 cells") that
    follows you as you navigate. Paste where you want them.
- **Wrap:** select a block of cells and choose "Group into grid". An animation shrinks them into one
  new cell whose child is that block (TreeSheets' Wrap).
- **Unwrap:** on a cell with a child grid, "Ungroup" spills the child's cells into the parent and
  shows a preview of the result before confirming, because it can reshape the parent.
- **Reorder within a level:** long-press a row or column header bar and drag it. Single-cell
  two-dimensional drag is left for later (swap or shift is ambiguous).
- **Power commands** live under More: Transpose, Flatten, Hierarchy swap and Sort.

**J9. A wide grid on a narrow screen.**
- The default is a real grid with pinned header row and first column and horizontal scrolling,
  signposted by a fade at the edge and a column count ("columns 1–3 of 9").
- Options, from lightest to heaviest:
  1. **Pin columns**: choose which columns stay visible, like Tana's pinned fields.
  2. **Rotate to landscape** for more columns. The layout reflows only the column count, not the
     structure.
  3. **Card lens**: one row at a time as a form, Numbers style, with header names as labels.
  4. **List lens**: an outline of rows.
- The choice is remembered per grid on this device.

**J10. Undo.**
- Undo and redo sit in the keyboard toolbar while editing and in the bottom bar's overflow
  otherwise. On iOS, shake-to-undo and three-finger swipe are left on.
- After a structural change, a brief **toast** offers "Moved 2 cells to Budget · Undo".
- Undo that changes another level **takes you there** and pulses the restored cells, so you can see
  what came back. TreeSheets also moves the view to undone changes.

---

## 5. Ideas and concepts, ranked

**Practicality** blends effort, risk and evidence (1 is most practical).
- **Core**: in the first build.
- **Later**: after the core is validated.
- **Experiment**: prototype and test before committing.
- **Deferred**: left out on purpose; comes back only if a named test signal appears.

### 5.1 The earlier six, refined

| # | Concept | Refinement in this plan | Tag | Practicality |
|---|---|---|---|---|
| C1 | **Level view with previews** | Becomes the **stage with a depth budget** (§3.2): Full, Preview, Shape or Badge, with one way down (tap the child to dive, §3.3). No longer strictly one level per screen | Core | 1 |
| C4 | **Explicit restructuring** (Move to…, pocket, keyboard toolbar, multi-select bar) | Adds Wrap and Ungroup with animated previews, header-bar drag for rows and columns, and an undo toast | Core | 2 |
| C2 | **Grid/List toggle** | Generalised into **lenses** (F2): Grid, List, Card (form), later Board. Lenses only show when a grid is wider than the screen, or from the view menu | Core (Grid, List), Later (Card, Board) | 3 |
| C3 | **Peek** | Not a separate verb: the long-press menu shows a larger preview on top (the iOS context menu; a bottom sheet on Android and elsewhere), and tapping it dives. No peek icon on cells. A visible peek control is deferred until tests show "look and leave" dives (§3.3) | Core (menu preview), Later (visible control, only if needed) | 3 |
| C5 | **Fisheye level** | Kept as an experiment for very wide grids on phones (focus column full, others squeezed to strips). Evidence is mixed: one PDA study found no speed difference but most users preferred fisheye ([TVCG 2006](https://cs.lnu.se/isovis/courses/spring09/da4094/papers/User%20Interaction%20with%20Scatterplots%20on%20Small%20Screens.pdf)) | Experiment | 8 |
| C6 | **Continuous zoomable canvas** | Not on phones. On desktop and tablet it is just the depth budget plus smooth zoom between budgets; the phone's "overview" pinch (J2) is a snapped, two-step version | Later (desktop), Experiment (phone) | 9 |

### 5.2 Fresh ideas

**F1. Shape thumbnails ("the shape of a child grid").** [Idea, core, practicality 1]
- When a child is too small to read, draw it as a miniature: its row and column lines, cell colours,
  and bars showing how much text each cell holds.
- A budget table looks like a budget table; a long list looks like a tall stripe; an empty grid
  looks empty.
- *Why:* it keeps TreeSheets' "see structure at a glance" even at four levels deep, and people use
  shape and colour for recognition before reading.
- *Weakness:* may look like noise if overused. Test whether people can tell two thumbnails apart.

**F2. Lenses (Tana's views, for grids).** [Product inspiration, core/later, practicality 3]
- Any grid can be seen through a lens on this device:
  - **Grid** (the default).
  - **List**: rows as an outline.
  - **Card**: one row as a labelled form, with arrows for next and previous.
  - **Board**, later: a one-row grid of lists becomes columns.
- If the grid's first row is marked as a header, Card and List label values with header names, as
  Tana's fields do. Lenses never change the document.
- *Weakness:* mode confusion and lost scroll position. Mitigation: keep the selected cell in view
  across lens switches, and keep the lens switch in one consistent place.

**F3. Trail bar with recent places.** [Product inspiration, core, practicality 1]
- The bottom trail bar (§3.4), plus a history: long-press ‹ for the last ten places across documents.
- A "jump back" button appears for a few seconds after any jump (search, link or backlink) to
  return to where you were.
- *Why:* jumping around is where people get lost; a one-tap way home costs almost nothing.

**F4. Mini-map.** [Idea, later, practicality 5]
- A sheet (phone) or corner panel (tablet and desktop) shows the whole document as nested boxes
  drawn from shape thumbnails, with the current stage lit up. Tapping any box jumps there.
- *Evidence:* overviews can help, but semantic zoom may replace the need for them (Hornbæk, above).
  Test whether people use it.
- *Weakness:* large documents become a blur; it needs smart cropping around the current place.

**F5. Split view and parent pane on tablets.** [Product, later, practicality 4]
- In wide windows, show the parent stage on the left and the dived-into child on the right, like
  Finder's column view or Tana's two-panel list view. Diving shifts the panes left.
- A second mode puts two unrelated grids side by side, to compare branches.
- *Why:* it fixes the main weakness of the stage model (comparing branches), using standard
  two-pane layouts for medium and expanded widths
  ([Android window size classes](https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes)).

**F6. Compare tray (phone).** [Idea, experiment, practicality 6]
- "Pin to compare" puts a cell, with its preview, in a small floating tray that stays visible while
  you navigate elsewhere. It answers test task 5 (compare two values in different branches) on a
  phone without split view.
- *Weakness:* another floating thing on a small screen. It could share space with the pocket.

**F7. Live search grids (Tana search nodes).** [Product, later, practicality 6]
- Save any search (`#todo -#done` inside "Trip planning") as a grid. Each row is a matching cell,
  with columns for its text, its trail and its tags.
- Editing a result edits the original. The grid updates as documents change.
- *Why:* Tana users love searches that act as dashboards, and Wombat's search runs on the device, so
  it can do on phones what Tana says it cannot.
- *Weakness:* needs core support for showing a cell in two places (not in the current plan). Start
  read-only with links.

**F8. Focus spotlight (desktop and tablet).** [Product, later, practicality 5]
- MindNode-style: with a cell selected, everything outside its branch dims a little. On large
  screens where many levels show at once, it tells you what you are working on without hiding
  context.

**F9. Shape templates (Tana supertags, lightweight).** [Idea, experiment, practicality 8]
- Tag a cell `#contact`, and it can stamp a child-grid template (Name, Phone, Email). Every
  `#contact` cell in the workspace can then be shown as one table (a live search grid with the
  template's header).
- This is optional and appears only after the user tags things.
- *Why:* it gets Tana's "typed objects and collection table" without its up-front schema.
- *Weakness:* a core-model decision (templates, typed cells are out of scope for the proof of
  concept), and the learning curve Tana suffers from. Experiment only.

**F10. Draggable "+" (Things' Magic Plus for grids).** [Product, experiment, practicality 7]
- One floating "+" in the thumb zone:
  - Tap it to add a row at the end.
  - Drag it, and gap markers appear between rows and columns, plus "inside this cell" targets. Drop
    it to insert exactly there.
- *Weakness:* two-dimensional drop targets are dense; it may conflict with scrolling. Delight, if it
  works.

**F11. Haptics and sound of structure.** [Platform practice, core, practicality 2]
- Light haptic ticks for: selecting, crossing a level boundary (dive or up), snapping into a drop
  slot, and the overview pinch snapping between steps. One firmer tap when an action can't happen.
- Always subtle, and respectful of the system setting.
- *Why:* it confirms an action without demanding the eyes, as Things' praised polish does. It is
  cheap.

**F12. Overview pinch (semantic zoom, snapped).** [Evidence-informed idea, later, practicality 4]
- Pinch in on the stage to switch to "shapes only" (every cell drawn as its thumbnail), then once
  more for the parent level. Pinch out on a cell to dive. Every step snaps; there is no continuous,
  free zoom.
- A visible "Overview" button does the same.
- *Why:* map-style semantic zoom gives navigation cues; snapping avoids desert fog and unreadable
  in-between states.

**F13. Capture inbox.** [Product (Tana mobile), later, practicality 3]
- The share sheet, a home-screen widget and a Shortcuts action add a cell to a chosen "Inbox" grid.
- *Why:* Tana's mobile app is built around capture, and that's what people do on phones.

**F14. Screen-reader outline mode.** [Accessibility practice, core, practicality 3]
- The List lens doubles as the spoken representation. Each nested cell is announced as "Restaurants,
  grid 4 by 2, row 3 of 7", followed by the first few child cells' text (so no peek is needed). Its
  custom actions are "Open grid" (dive) and "Fold"; "Up one level" is the platform's back or escape
  gesture (earlier reports).

### 5.3 Everything ranked

| Rank | Idea | Tag | Why this rank |
|---|---|---|---|
| 1 | C1 Stage with depth budget, with tap-to-dive as the one way down | Core | The answer to the main problem; generalises TreeSheets |
| 1 | F1 Shape thumbnails | Core | Makes the depth budget work at all depths |
| 1 | F3 Trail bar and recent places | Core | Cheap, essential for not getting lost |
| 2 | C4 Move to…, pocket, keyboard toolbar, multi-select bar, Wrap and Ungroup | Core | Restructuring is where others fail; explicit first |
| 2 | F11 Haptics | Core | Cheap and confirms actions |
| 3 | C2/F2 Lenses: Grid and List now, Card and Board later | Core / Later | Workflowy and Tana both prove "views of the same data" |
| 3 | F14 Screen-reader outline mode | Core | Accessibility, built on the List lens |
| 3 | C3 Peek as the long-press menu's preview | Core | Free with the menu that exists anyway; platform-standard on iOS |
| 3 | F13 Capture inbox | Later | High value on phones, low risk |
| 4 | F5 Split view and parent pane on tablets | Later | Fixes cross-branch comparison on big screens |
| 4 | F12 Overview pinch | Later | Evidence-backed semantic zoom, snapped |
| 5 | F4 Mini-map | Later | Useful if people use it; test first |
| 5 | F8 Focus spotlight | Later | Desktop and tablet nicety |
| 6 | F7 Live search grids | Later | Powerful; needs core support for showing a cell twice |
| 6 | F6 Compare tray | Experiment | Phone answer to comparing; unproven |
| 7 | F10 Draggable "+" | Experiment | Delightful if dense targets work |
| 7 | "Show here" (open in place beyond the budget) and a visible peek control | Deferred | Each adds a verb; back only if the tests show the signals in §3.3 |
| 8 | C5 Fisheye level | Experiment | Mixed evidence; complex |
| 8 | F9 Shape templates | Experiment | Needs core model work; schema burden |
| 9 | C6 Continuous zoom on phones | Experiment | Legibility and desert-fog risk; fine on desktop |

---

## 6. From phone to tablet to desktop

**What stays the same everywhere:**
- the stage and the depth budget rule;
- one way down: tap the nested grid (or "Open ›") to dive, tap text to select and edit, long-press
  or right-click for the menu with its preview. Larger screens simply show more children in full,
  so there is less need to dive;
- the trail and the history;
- search that filters in place;
- Move to… and the pocket;
- lenses, undo, and the same words and icons.

Someone who learns Wombat on the phone already knows it on desktop.

| Window width | Typical device | What changes |
|---|---|---|
| Under 600 | Phone portrait | One stage. Trail bar and actions at the bottom. Depth budget about 1.5–2 levels. Single editor above the keyboard |
| Phone landscape (short height) | Phone | More columns on the stage. No two-pane layout, because the height is too small ([Android guidance](https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes)). The trail collapses into the top bar |
| 600–840 | Tablet portrait, foldables | Optional parent pane (F5). Depth budget about 2 levels. The long-press menu's preview can be larger |
| 840–1200 | Tablet landscape, small laptop | Parent pane plus stage, or two grids side by side. Depth budget about 2–3 levels. Trail moves to the top as a full breadcrumb. Editing in the cell itself becomes an option |
| Over 1200 | Desktop | TreeSheets-like: depth budget 3–5 levels, so a big part of a document shows at once. Smooth zoom between budgets. Focus spotlight. Mini-map in a corner. Several documents in tabs |

**Keyboard on desktop, tablets with keyboards and phones with Bluetooth keyboards:**
- Keep TreeSheets' habits where sensible: arrows move the selection, Enter or F2 edits, Escape stops
  editing and then selects the parent cell (TreeSheets' "Select Parent"), and Tab and Shift+Tab move
  across cells.
- **Going down and up** uses TreeSheets' keys (`src/tsframe.h`):
  - Shift+Enter moves into the selected cell's grid (TreeSheets' "Select First Child"). If that grid
    is drawn in full, it selects its first cell; if it is a preview, shape or badge, it dives. One
    key, and it adapts to what is on screen.
  - Ctrl+PgUp and Ctrl+PgDn (Cmd on Mac), also Ctrl+mouse wheel, dive into the selected cell and go
    up one level: TreeSheets' Zoom In and Zoom Out.
  - Both appear in the menus next to "Open grid" and "Up one level".
- Fold keeps TreeSheets' Ctrl+F10 and Ctrl+double-click.
- Cmd/Ctrl+F searches, Cmd/Ctrl+K opens the quick switcher and command palette, Cmd/Ctrl+Z and
  Shift+Cmd/Ctrl+Z undo and redo, and Cmd/Ctrl+G groups into a grid.
- Discoverability:
  - On iPad, holding Cmd shows the shortcut list (system feature).
  - On desktop, every shortcut appears in the menu next to its command.
  - The command palette lists them too.

**Mouse and trackpad:**
- Clicking works like tapping: click the text to select, click a preview, shape or badge to dive.
  Double-click edits and selects a word, as in TreeSheets; it never dives.
- Hover shows a cell's preview at a larger size after a short delay (TreeSheets has hover-zoom).
  This is the desktop's look-without-leaving.
- Right-click opens the same menu as long-press, with the same preview on top.
- The mouse's back button goes up one level.
- Drag and drop between levels is safe here: drop slots highlight, and hovering on the breadcrumb
  goes up a level.
- Scrolling with a modifier, or pinching on the trackpad, zooms between budgets.

**Avoid:** desktop-only features in the core experience. Workflowy's panes and column resizing are
desktop-only, so phone users can't do things desktop users can. Wombat should offer every action on
every platform, even if desktop has faster ways.

---

## 7. Best practices and pitfalls

**Touch and reach.**
- Targets: at least 44×44 pt (48 dp on Android), with at least 8 pt between targets. A *visible*
  badge can be small if its tappable area is large.
- A cell with a child has two large regions, its text and its child; never a small icon beside the
  text with a different action (§3.3).
- No dive while editing: a tap outside the editor only ends editing.
- Put the primary controls (trail, search, +, pocket) in the bottom third. Keep the top bar for the
  title and rare actions.
- Avoid horizontal swipes on cells: grids need horizontal scrolling. Reserve the left screen edge for
  the system back gesture.

**Gestures and discoverability.**
- Every gesture has a visible twin (principle 4).
- Teach gestures with a gentle hint *after* someone uses the button version three times ("Tip: pinch
  out on a preview to dive"), not in a tutorial up front.
- Long-press is a platform convention for menus and previews; don't use it for anything else.

**Animation and orientation.**
- Dives and returns should take about 250–350 ms, with motion that comes from the cell and goes back
  to it.
- Follow the system's Reduce Motion setting: cross-fades with the cell highlighted instead of zooms.
- Rotation keeps the selected cell in view and only changes how many columns fit.

**Accessibility.**
- Screen readers:
  - Use the outline mode (F14) and a sensible reading order (row by row).
  - Announce depth and grid size.
  - Provide a custom action "Open grid" (dive) on every cell with a child, plus "Fold". Going up
    is the platform's own gesture: Android back, and VoiceOver's two-finger scrub (escape) on iOS.
  - Read the first few child cells in the cell's label, so nobody needs to dive just to hear what
    is inside.
- Large text:
  - The depth budget must use the user's text size. At very large sizes, previews shrink to shapes
    sooner, and the List lens becomes the default for wide grids.
  - Never cap text size.
- Contrast:
  - User-chosen cell colours must keep text readable. Automatically pick dark or light text against
    the cell colour, and warn in the style picker when a pair fails 4.5:1 (WCAG AA).
  - Shape thumbnails need non-colour cues (lines, fill bars) for colour-blind users.
- Test with VoiceOver and TalkBack in every usability round, not just at the end. Flutter has no
  built-in "tree" role, so nesting must be carried by labels and actions (earlier reports).

**How fast it feels.**
- Show the next level immediately, even before every preview has been drawn, and fill previews in
  without moving anything.
- Typing must never wait for the core.
- Search shows first results as you type.
- Never show a spinner for navigation inside a document.

**Onboarding.**
- The first document is a small, real example: a trip planner, or a reading list with a nested
  budget. It shows a small child in full, a preview and a shape. The best teacher is a document
  worth poking at; TreeSheets' own tutorial is a document too.
- Two one-line coach marks at most, each shown once at the moment it applies: "Tap a grid inside a
  cell to go into it" (the first time a preview is on screen) and "‹ takes you back up" (after the
  first dive). The long-press preview and pinch are taught later as tips, after a few dives.
- Introduce power (tags, formulas, lenses) the first time it is relevant, not before (principle 10,
  Tana's lesson).

**Empty states.**
- An empty grid shows a "+" row and column and one line ("Type anything. Any cell can hold its own
  grid.").
- An empty search shows recent searches and tags.
- An empty pocket is invisible.

**Platform conventions.**
- iOS: back swipe, sheets with detents, long-press previews, Dynamic Type, SF Symbols-style icons.
- Android: predictive back, Material bottom sheets, 48 dp targets.
- Desktop: menus, shortcuts, right-click.
- Flutter draws everything itself, so these must be built deliberately. That is the price of one
  codebase.

**Pitfalls to avoid.**
- Tiny cells that can't be hit.
- Editing in place on a phone.
- Several ways down that differ only in how far they go, or one control that does different things
  depending on width.
- Gestures with no visible twin.
- Animations that block input.
- A breadcrumb at the top, out of thumb reach.
- Views that silently differ between phone and desktop (Tana's mobile gap).
- A schema before the first sentence (Tana's learning curve).
- Swipe actions that fight scrolling (Workflowy boards).

---

## 8. The step-by-step plan

Each step lists what to decide, the questions to answer and how to judge success. The core
milestones are not started, so steps 1–3 use a throwaway prototype with a fake in-memory model and do
not wait for Rust.

### Step 1: Framing and paper (about a week)

**Do:**
- Pick two or three real test documents:
  - a TreeSheets example such as the tutorial, converted;
  - a personal planning document;
  - a stress document: eight levels deep, and one level that is 200×10.
- Write the eight test tasks below.
- Sketch on paper or a whiteboard, at phone size:
  - the stage, the four depth-budget forms, and the split of a cell into its text and its child;
  - the trail bar, Move to… and the pocket;
  - the search filter and the lenses.
- Run quick paper tests with three or four people: "Where would you tap to see what's in Budget?",
  "…to change Budget's title?", and, pointing at a preview, "What will happen if you tap here?"

**Decide:** the visual language for previews and shapes; how the text and child regions look
different (frame, tint, spacing); the minimum cell size for the split; wording ("Open" or "Go in"?
"Group into grid" or "Wrap"?).

**Questions:**
- Do people understand that a preview means "there is a grid inside"?
- Do they expect tapping a preview to go into it, to open it here, or to select the cell?
- Do they expect tapping the text to edit it?

**Success:** at least three of four people predict that tapping a preview, a shape or a badge goes
into it and that tapping the text edits it, without being told. If most expect a preview to open
"here", note it as an early "Show here" signal (§3.3).

### Step 2: Feel prototype in Flutter on real phones (about two weeks)

**Day one, before anything else: the typing test.**
- Build one screen with a cell editor in two versions:
  - **A:** Flutter's own text field.
  - **B:** a native iOS text view embedded in the Flutter screen. The earlier reports note that one
    native editor is affordable precisely because Wombat edits one cell at a time.
- On a real iPhone, check: autocorrect and its suggestion bubbles, dictation, the magnifier and
  cursor dragging, text replacement shortcuts, Writing Tools, Chinese and Japanese input, emoji,
  shake-to-undo, hardware keyboard shortcuts, and the keyboard toolbar staying put.
- Have three to five people type and correct a short paragraph in each version, without knowing
  which is which.
- **Pass:** people can't tell A from the Notes app, or prefer A. Otherwise use B for the editor and
  carry on.
- This is the cheapest moment to learn it, and the React Native report's bake-off uses the same
  test.

**Then build:**
- The stage with pinned headers.
- The depth budget (Full, Preview, Shape, Badge), and tap-to-dive with its animation, "Open ›" in
  the bottom bar and pinch out as an accelerator.
- The long-press menu with its larger preview (the iOS context menu; a bottom sheet on Android).
- The trail bar with history.
- Search filtering in place, with jumps.
- The single editor with its toolbar.
- Multi-select, Move to… and the pocket.
- Wrap and Ungroup, and undo with a toast.
- The Grid and List lenses.
- Haptics.
- Two variants behind a switch:
  - **A:** depth budget on, with tap-to-dive.
  - **B:** strict one level per screen with previews only, the earlier report's baseline.
- Built but switched off: "Show here" in the long-press menu, so it can be turned on in round 2 if
  round 1 shows the signals in §3.3.
- Logging of every dive and up, with timestamps, to count accidental, round-trip and "look and
  leave" dives.

**Decide:** the depth-budget thresholds (smallest readable text, preview scale); the animation
timing; the default lens rules.

**Questions:**
- Is a preview at phone size readable enough to avoid blind dives?
- Can the text and child regions of a cell be told apart and hit reliably at phone size, including
  with large text?
- How small can a cell get before it must become one target with "Open ›"?
- Is the dive animation smooth on a two- to three-year-old iPhone and a mid-range Android?

**Success:**
- No dropped frames during dives and scrolling on the test phones.
- Typing passes the day-one test.
- The team can do all eight tasks without workarounds.

### Step 3: Usability rounds (three rounds over about three weeks)

**Format:** five participants per round. Find most problems in each round, fix them, then retest
([NN/g](https://nngroup.com/articles/why-you-only-need-to-test-with-5-users/)). Mix spreadsheet and
outliner users with general phone users, and include at least one screen-reader or large-text user.
Each person tries both variants, in a balanced order, in round 1. Later rounds test only the winner.

**Tasks** (from the earlier report, plus three new ones):
1. Find the cell "X" three levels deep, starting at the top, without search.
2. Change the text of a cell two levels deep, then return to the top.
3. Add a row to a sub-grid and fill in two cells.
4. Move a cell from one sub-grid to a sub-grid in another branch.
5. Compare two values in two sibling sub-grids and say which is larger.
6. Turn a cell into a 2×2 grid and fill it, then group three existing cells into a new grid.
7. Undo the last three changes.
8. Find every cell tagged #todo inside one branch and open one.
9. *(New)* Describe what's in the document after 30 seconds of looking. This tests whether "seeing
   the shape" works, which is the anti-Workflowy test.
10. *(New)* In a wide grid (9 columns), find the value in row 12, column 7.
11. *(New)* Fix a typo in three items of a small list inside a cell (a 3×3 child that shows as a
    preview), then carry on with the top grid. This is the task where "Show here" would help most,
    so it shows what cutting it costs.

**Measures:**
- Task success, time, and errors (wrong cell, accidental dive, wrong destination).
- **Drill-down signals**, from the logs:
  - *Accidental dives:* a dive followed by going up within about 2 seconds with no other action,
    or a dive the person says they didn't mean.
  - *Round trips:* dive, a small edit, up again within about 20 seconds, on the same small child.
  - *Look-and-leave dives:* a dive, nothing changed, up within a few seconds.
  - *Long-press preview use:* whether anyone finds and uses it without being told.
- Before the first tap on a preview in round 1, ask "What do you think will happen?" and note the
  answer.
- **Getting lost:** "where am I?" moments, and steps taken compared with the shortest path.
- The single ease question after each task (1–7).
- For task 9: how many of the document's main parts people can name.
- Which variant people prefer, and why.

**Success criteria** (adjust after round 1):
- Tasks 1–3 done unaided by at least four of five people, each in about a minute.
- Task 4 done unaided by three of five in round 1, and four of five by round 3.
- Lostness under about 1.5 on task 1, and at most one "where am I?" per person per task.
- Task 9: variant A beats variant B on the number of parts named. This is the evidence that the
  depth budget is worth it.
- People explain previews and shapes without being told.
- At most one accidental dive per person across the whole session, and none while editing.
- Task 11 done by four of five people in about a minute, without them asking to open the list
  "here".
- Screen-reader users complete tasks 1–3 using the "Open grid" action.

**Kill or pivot signals:**
- **Shapes read as noise:** keep previews and badges only.
- **People keep doing round trips, or say they want to open a child "here":** turn on "Show here" in
  the long-press menu for round 2. Make it a visible control only if round 2 still shows the need.
- **Many look-and-leave dives, and nobody finds the long-press preview:** test a visible peek
  control in round 2.
- **Accidental dives above the bar:** enlarge the text region, require the cell to be selected
  before its child dives, or move diving to "Open ›" only, and retest.
- **Cross-level moves fail even with Move to…:** rethink restructuring before building anything
  else.
- **People keep asking for "the whole picture":** pull the mini-map (F4) and the overview pinch (F12)
  forward.

### Step 4: Build on the core (in step with core milestones 1, 3, 4 and 6)

- Replace the fake model with the Rust core.
- Redraw only the cells the core reports as changed (decision A10). The stage only ever shows a
  bounded number of cells, so this stays fast regardless of document size.
- Store view state (lens, folds, scroll and history) per device (A8).
- Add backlinks (milestone 4) and workspace search (milestone 6) to the journeys.
- **Success:**
  - The prototype's journeys work on real documents.
  - Opening a large document shows the first screen in under a second on the test phones.
  - Typing latency is unchanged.

### Step 5: Widen to tablet and desktop (in parallel with the end of step 4)

- Raise the depth budget, add the parent pane and split view, keyboard shortcuts, hover and drag,
  the focus spotlight and the mini-map.
- Test with two or three TreeSheets desktop users. They are the toughest judges of "seeing structure
  at a glance".
- **Success:** TreeSheets users say the desktop view shows "enough" structure, and they can do the
  tasks with keyboard and mouse at least as fast as in TreeSheets.

### Step 6: Beta (TestFlight and Play testing)

- 20–50 real users, with in-app feedback.
- Light, privacy-respecting usage counts: dives, ups, quick round trips, use of the long-press
  preview, use of the trail history and lens switches.
- Then decide on the "later" and "deferred" ideas ("Show here", a visible peek control, capture
  inbox, live search grids) and run the experiments
  (compare tray, draggable "+", shape templates) as A/B prototypes.
- **Success:**
  - People return to their documents weekly.
  - No more than about one "lost" report per 100 sessions.
  - App-store-grade crash-free rate.

---

## 9. Open questions and risks, honestly

1. **The depth budget is unproven.**
   - No shipping phone app draws nested two-dimensional grids at reduced size. The thresholds may
     turn out so strict that phones rarely show more than previews, making Wombat on the phone close
     to one level per screen anyway.
   - That would still be acceptable, but it must be known early (task 9).
2. **One way down may be too few, or too easy to trigger.**
   - Tap-to-dive depends on the text and child regions of a cell looking and feeling different. In
     small cells, or with large text, people may dive when they meant to select, the Dynalist
     problem. The defences (large regions, no dive while editing, instant back) are untested in grids.
   - Without "Show here", editing small lists inside cells costs a dive and a return each time, and
     outliner users may miss the expand arrow they know. Task 11 and the round-trip count measure
     this; "Show here" is built and ready to switch on.
   - The long-press preview may simply go unnoticed, which is acceptable as long as diving and
     coming back stay cheap.
3. **Comparing across branches** stays awkward on phones. Split view solves it on tablets. The compare
   tray is a guess.
4. **Restructuring on touch** is everyone's weak spot. Move to… and the pocket are safe but slow for
   experts, and drag across levels is deliberately deferred.
5. **Typing quality in Flutter on iOS.** The day-one test exists for this. The fallback (one native
   editor) has its own integration risks around keyboard toolbar placement and focus.
6. **Tana-style typing (templates, fields) could pull Wombat toward complexity.** Its users love it
   and its non-users bounce off it. Keeping it optional and late is a judgement call, not evidence.
7. **Free-form grids rarely have header rows,** so Card and List lenses may often have nothing to
   label values with. They may then look like plain outlines, which is still useful but less
   impressive.
8. **Accessibility of nesting** depends on careful labels and custom actions. Screen-reader users
   may find two-dimensional grids plus depth heavy. The outline mode is the mitigation, and it must
   be tested with real users.
9. **Flutter draws its own controls,** so iOS and Android "feel" (back gestures, sheets, haptics, long
   press previews) must be rebuilt. Imitations can feel slightly off. The React Native report weighed
   this; the bake-off decides.
10. **The evidence base is thin.** Most product evidence is vendor documentation, changelogs and
    reviews of uneven quality, including competitor-written review roundups. Academic work on zooming
    and fisheye views is old and done on PDAs. The usability rounds are the real evidence.
11. **Live search grids and shape templates need core features** (a cell shown in two places,
    templates, typed values) that are outside the proof of concept. They should stay ideas until the
    core can support them.

---

## Appendix: the short technical notes a builder will need

- The stage is a lazily built two-dimensional scrolling grid with pinned header row and column.
  Flutter has an official package for this. It is pre-1.0, so expect changes (earlier report §5).
- Previews and shape thumbnails should be painted as cached pictures keyed by grid and version, not
  built from many small widgets. The core's "changed cells" list says when to repaint.
- The depth budget is one measuring pass per stage: it estimates the text size each nested grid
  would need and chooses Full, Preview, Shape or Badge.
- Diving and going up share one animation driven by the gesture, so a pinch can be cancelled
  halfway. A tap plays the same animation to its end.
- Hit testing on a cell with a child returns one of two regions, text or child (or a single region
  when the cell is too small to split), and the stage ignores child-region taps while the editor is
  open.
- On iOS the long-press menu uses Flutter's `CupertinoContextMenu`, with the child's cached preview
  picture as its preview; elsewhere a bottom sheet shows the same picture. Tapping the preview starts
  the dive animation from the preview's position.
- "Show here", if switched on, is one more depth-budget override in view state, next to fold: it
  asks for the next richer form where it fits, and nothing else changes.
- The editor is a single field, which can be swapped for a native iOS text view if the day-one test
  says so.
- View state (lens, folds, scroll, history) lives in a per-device store, outside the document (A8).

## Sources used for the first time in this plan (others are in the earlier reports)

- Workflowy what's new: https://workflowy.com/whats-new/
- Workflowy tables: https://community.workflowy.com/t/tables/1647
- Tana mobile: https://outliner.tana.inc/learn/features/tana-mobile
- Tana views: https://outliner.tana.inc/learn/features/views
- Tana view toolbar: https://outliner.tana.inc/blog/a-flexible-alternative-to-rigid-databases-meet-the-new-view-toolbar-in-tana
- Tana fields: https://outliner.tana.inc/learn/features/fields
- Tana supertags: https://outliner.tana.inc/learn/features/supertags
- Tana search nodes: https://outliner.tana.inc/search-nodes
- Tana reviews: https://blog.saner.ai/tana-reviews (a competitor's roundup),
  https://dev.to/pickuma/tana-review-an-outliner-based-pkm-tool-that-thinks-in-graphs-8l8,
  https://vantaige.io/ai-tool/tana
- Notion on mobile: https://www.notion.com/help/workspaces-on-mobile,
  https://www.getinflow.io/post/notion-for-overwhelmed-beginners-mobile-phone-advice
- Logseq embeds: https://discuss.logseq.com/t/the-difference-between-logseqs-block-embeds-and-block-references/8459
- Numbers selection: https://support.apple.com/guide/numbers-iphone/select-cells-rows-and-columns-tan4deb6ba50/ios
- MindNode focus: https://www.mindnode.com/support/guides/focus-mode, https://tapsmart.com/features/deep-dive-mindnode
- Heptabase changelog: https://wiki.heptabase.com/changelog/2024
- Things 3: https://culturedcode.com/things/features/, https://www.macstories.net/?p=48965
- Apple HIG context menus: https://developer.apple.com/design/human-interface-guidelines/context-menus
  (the page didn't render for fetching; the guidance is paraphrased from general HIG knowledge)
- WCAG 2.2 target size: https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html
- Android window size classes: https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes
- Zooming research:
  - Hornbæk, Bederson, Plaisant: https://scilit.com/publications/ad4f4bae6e3d5d7df2180827b57daa71
  - Büring & Reiterer: https://typeset.io/pdf/usability-of-overview-supported-zooming-on-small-screens-58zou4wwcf.pdf
  - Small-screen fisheye versus zoom: https://cs.lnu.se/isovis/courses/spring09/da4094/papers/User%20Interaction%20with%20Scatterplots%20on%20Small%20Screens.pdf
- Nielsen heuristics: https://www.nngroup.com/articles/ten-usability-heuristics/
- Going deeper (§3.3):
  - NN/g accordion icons (tap on label vs icon): https://www.nngroup.com/articles/accordion-icons/
  - Dynalist accidental zooms: https://talk.dynalist.io/t/prevent-accidental-zoom-when-tapping-a-checkbox-on-mobile/4586,
    https://talk.dynalist.io/t/android-app-request-options-to-set-zoom-in-settings/8260
  - Workflowy bullets: https://community.workflowy.com/raw/1663
  - Context menus replacing peek and pop: https://useyourloaf.com/blog/adding-context-menus-in-ios-13/,
    https://developer.apple.com/documentation/uikit/deprecated-symbols
  - Flutter CupertinoContextMenu: https://api.flutter.dev/flutter/cupertino/CupertinoContextMenu-class.html
  - Apple HIG disclosure controls: https://developer.apple.com/design/human-interface-guidelines/disclosure-controls
  - Progressive disclosure and Hick's law: https://jakobnielsenphd.substack.com/p/progressive-disclosure
  - Modes and mode errors: https://web.mit.edu/6.813/www/sp17/classes/04-safety
  - TreeSheets zoom, fold and selection keys: `src/tsframe.h` (`A_ZOOMIN`, `A_ZOOMOUT`, `A_FOLD`,
    `A_ENTERGRID`, `A_CANCELEDIT`)
