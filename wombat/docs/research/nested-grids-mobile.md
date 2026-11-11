> Background research, not instructions. The interface specification is `../UI_SPEC.md`; where this file differs from it, the specification wins.

# Nested grids on a phone: research and recommendations for Wombat

Date: 2026-11-11. Scope: how to make arbitrarily nested grids pleasant on a phone with touch, building on
the settled Wombat decisions (Flutter UI + Rust core via flutter_rust_bridge, phone first, Obsidian-style
UX, per-device view state, changed-cell reports from the core).

Labels used below: **[Evidence]** = backed by a cited source; **[App]** = observed in a shipping product
(documentation or reviews, usually anecdotal); **[Speculation]** = my own reasoning, untested.

---

## 0. TL;DR

- **No shipping phone app does arbitrarily nested *grids*.** Outliners nest lists (Workflowy, Logseq,
  Dynalist), Notion nests pages, Confluence and Coda nest tables only one level or with lost features,
  and no TreeSheets-like mobile app turned up. The gap is real, but so is the reason: every product that
  tried "tables in tables" on small screens backed off to one level or to lists.
- **Recommendation: a "Level view" (one grid level per screen) with *visible previews* of child grids
  inside cells, an always-reachable Up control plus breadcrumb at the bottom, a one-level "Peek" sheet,
  a per-level Grid/List toggle, and an explicit "Move to..." picker for restructuring.** Every part of it
  maps to an established pattern (Workflowy zoom, breadcrumbs, keyboard toolbar, multi-select Move
  toolbar and per-node layouts; MindNode focus mode; Muse tap-to-enter; Numbers forms; Files-app move
  pickers) and to standard heuristics (visibility of system status, recognition
  over recall, user control and freedom, platform consistency). Novel gestures (pinch to dive, swipe for
  siblings) are *accelerators* layered on top of visible buttons, never the only way.
- **Workflowy is the closest precedent**: its Table layout already shows a grid whose cells can hold
  nested content, marked with a border and opened rather than edited inline, and its boards/tables are
  layouts of a node, not a different data model. Zoom, breadcrumbs, the keyboard toolbar, multi-select +
  Move, tags and in-place search filtering carry over; swipe actions and free 2D drag mostly do not (§2b).
- **Continuous zoom (TreeSheets/Muse/Prezi style) and fisheye grids rank lowest for phones**: legibility
  floors, "desert fog" disorientation, and hard implementation. Keep them for iPad/desktop later.
- **Prototype in pure Dart now** (the core's milestones are all "not started"), on a real iPhone,
  with a fake in-memory model fed from a TreeSheets example converted to JSON. Two concepts head to head
  (Level view vs. Level view + List mode), 5 users per round, 2–3 rounds, measured on task time, errors
  and "getting lost" counts.

---

## 1. What TreeSheets does today (and what carries over)

Read from `/home/user/treesheets/src/document.h`, `cell.h`, `grid.h`:

- **Zoom is discrete and path-based.** `Document::drawpath` is the path from the root to the cell
  currently shown as the drawing root; `Zoom(dir)` pushes or pops one level (`document.h` ~1160) and
  re-lays out only that root (`currentdrawroot->LazyLayout`, ~1354). This is already "one grid level as
  the screen", the same idea as the Wombat level view.
- **TreeSheets already draws a breadcrumb.** `Layout()` adds one text line of height per non-empty
  ancestor (`hierarchysize`, ~1358–1362), and those ancestors' texts are drawn above the zoomed grid.
- **Nesting below the root is drawn smaller, not hidden.** Nested grids are laid out inline at reduced
  text size (`relsize`, `MinRelsize`, `pathscalebias`). On a 27" monitor this shows 3–5 levels at once;
  on a 6" phone it cannot, because text becomes illegible after about one level of shrink
  **[Speculation, but follows from the touch/legibility numbers in §3]**.
- **"Tiny" cells auto-zoom.** `ScrollOrZoom(zoomiftiny)` / `ZoomTiny()` (~796–830) zooms in when
  jumping to a cell that would be drawn too small, and zooms *out* only as far as needed to show the
  target. That is exactly the rule a phone needs for search results, links and backlinks.
- **Folding** (`Grid::folded`) hides a sub-grid but keeps the cell; `InView()` treats folded grids as
  cut-offs. On a phone, "folded" and "shown only as a preview" become nearly the same thing.

Takeaway: TreeSheets' *model* of navigation (a stack of zoom levels plus an ancestor header plus
zoom-to-target) transfers well. What does not transfer is the *rendering* choice of showing many levels
at once by shrinking them.

---

## 2. Prior art

| Product | How nesting works on phone | What works | What fails | Source quality |
|---|---|---|---|---|
| **Workflowy** | Zoom ("hoist") into any bullet; breadcrumb; left/right swipe on items for complete/menu; "boards" turn a level into kanban columns, and boards and lists can contain each other | Zoom is the core navigation and users ask other apps to copy it | Accidental swipe-completes reported by users; later update reduced swipe/scroll conflicts; crash fix for zooming into nodes with many children | [App] [blog](https://blog.workflowy.com/swipe-right-on-us-mobile-shortcuts/), [what's new](https://workflowy.com/whats-new/), [boards help](https://workflowy.com/help/boards), [Reflect request](https://roadmap.reflect.app/feature-requests/p/ability-to-zoom-into-a-node) |
| **Dynalist** | Click bullet to zoom, breadcrumb trail | Same pattern as Workflowy | — | [App] [forum](https://talk.dynalist.io/t/a-quick-way-to-go-to-inbox/1479/12) |
| **Logseq** | Outliner; block zoom only recently added to the new mobile app | — | No Tab key on phones: users can't find how to indent; toolbar above keyboard missing or not staying put in some builds; a community member built a separate native Android client because the official one felt unreliable | [App] [indent thread](https://discuss.logseq.com/t/how-to-indent-list-item-on-andoird/26379), [toolbar issue](https://fr.github.com/logseq/logseq/issues/11290), [DB changelog](https://discuss.logseq.com/t/logseq-db-changelog/30013/22), [FastLog](https://discuss.logseq.com/t/a-fast-native-android-app-for-logseq-og-markdown-version/35052) |
| **Obsidian** | (Text outliner via plugin) | Toolbar above keyboard | Indent/outdent buttons were opt-in and hard to discover | [App] [forum](https://forum.obsidian.md/t/bug-indent-unindent-buttons-on-mobile-toolbar-are-opt-in-and-hard-to-discover/16286) |
| **Tana** | Outliner with supertags; tables on desktop | Capture | Mobile lacks view settings (table view not confirmed on mobile); editing complex structures on mobile "disappoints" | [App] [Tana mobile docs](https://outliner.tana.inc/learn/features/tana-mobile), [review](https://vantaige.io/ai-tool/tana) |
| **Notion** | Pages in pages, revealed with toggles in the sidebar; databases shown as views | Familiar page-push navigation | Slow loads of uncached databases; users tap cards several times; deep nesting adds load time; can't drag a page into another page on mobile | [App, anecdotal] [blog](https://nerdymomocat.github.io/posts/my-annoyances-with-notion), [Fabric](https://developers.fabric.so/blog/why-notion-is-slow), [Notion help](https://www.notion.com/help/workspaces-on-mobile) |
| **Google Sheets / Excel mobile** | Flat grid, pinch zoom, formula bar for editing | Formula bar is the "safest" edit path | Tapping the wrong tiny cell, overwriting; long text hard to edit; extra tap before typing annoyed users | [App] [Excel touch guide](https://support.microsoft.com/en-us/excel/excel-for-android-touch-guide), [Bricks guide](https://thebricks.com/resources/guide-how-to-make-google-sheets-mobile-friendly), [reviews](https://justuseapp.com/en/app/842849113/google-sheets/reviews) |
| **Apple Numbers (iPhone)** | **Form view**: one table row shown as a form, arrows for prev/next row, edits sync both ways | Fast record entry on a phone | Calculated fields not editable in form; separate tab | [Evidence/App] [Apple guide](https://support.apple.com/guide/numbers-iphone/tan66fd732f9/ios) |
| **Airtable** | Grid on phone; record cards; expanded record | Scrollable cards so you can compare fields without opening each record | Expanding a record was "an extra step" and made comparison hard (Airtable's own words); expanded view ignored hidden fields | [App] [Airtable blog](https://blog.airtable.com/the-new-ios-grid-view/), [community](https://community.airtable.com/legacy-product-ideas-75/the-mobile-android-or-ios-app-of-airtable-doesn-t-follow-the-configuration-of-hidden-fields-in-the-view-41499) |
| **MindNode** | Mind map + editable outline; on iPhone the outline is modal (slides up full screen); **Focus mode** shows only the selected branch | Developers say finding content on an infinite canvas on a small screen "is not always easy", which is why they added an outline | Landscape split is cramped | [App] [MindNode blog](https://www.mindnode.com/blog/outlining-on-ios), [MacStories](https://www.macstories.net/reviews/mindnode-for-ipad-and-iphone-adds-editable-outline-mode/), [Mac Life](https://www.pressreader.com/australia/mac-life/20210817/281736977513261) |
| **Muse** (iPad/Mac) | Boards in boards; tap a board card to enter it, landing where you tapped; breadcrumb bar to exit | Spatial "zig, zag, zoom" navigation | iPad-first; no evidence about phones | [App] [Muse guide](https://museapp.com/guide), [Ink & Switch](https://www.inkandswitch.com/muse/) |
| **Heptabase** | Whiteboards on mobile, cards editable | Reading and capture | "Arranging a whiteboard on a small screen is awkward"; mobile is capture-first | [App] [DEV review](https://dev.to/pickuma/heptabase-review-visual-note-taking-for-thinking-through-problems-5daa), [changelog](https://wiki.heptabase.com/changelog/2024) |
| **Nested tables generally** | Confluence Cloud: one level only, fewer features; Coda: detail tables can't nest, nested tables lose deletion/filtering; GitBook: nested tables "difficult to edit, resize, navigate"; wikis advise avoiding them on mobile | — | Every vendor capped depth | [App] [Confluence](https://community.atlassian.com/forums/Confluence-articles/Nested-Tables-are-now-supported-in-Confluence-Cloud/ba-p/2980588), [Coda](https://community.coda.io/t/content-formatting-text-nesting-dynamic-text-generation-infinite-section-nesting/13612), [GitBook](https://gitbook.com/docs/~gitbook/embed/page/creating-content/blocks/table), [Fandom](https://community.fandom.com/wiki/Help:Avoiding_nested_tables) |
| **TreeSheets-like on mobile** | None found. TreeSheets is desktop only; TreeTime (proprietary, hierarchical trees, Android) is the closest | — | — | [AlternativeTo](https://alternativeto.net/software/treesheets), [strlen.com](https://strlen.com/treesheets) |

**Patterns that recur across successful phone apps** (synthesis):

1. **One focus level at a time + a way back** (outliner zoom, MindNode focus, Muse boards, Notion pages).
   This is the most consistent pattern; I found no successful phone app that shows several levels of a
   2D structure at once at full detail.
2. **An alternate 1D representation** for small screens (MindNode outline, Numbers form, Airtable cards).
3. **Editing in a dedicated, large field** (spreadsheet formula bar, Numbers form) rather than in tiny cells.
4. **Hidden gestures cause trouble** (Workflowy accidental swipes, Logseq/Obsidian indent discoverability).
5. **Structure editing is the weakest part everywhere on mobile** (Notion can't drag into pages, Logseq
   indent, Heptabase arranging, Tana complex structures). This is where Wombat could genuinely lead.

Source caveat: most of the above is vendor docs, forums and reviews, not controlled studies. I found no
published usability study of nested tables on phones.

---

## 2b. Workflowy in depth, and what carries over to nested grids

Workflowy is the primary inspiration alongside Obsidian, because it has shipped "zoom into any node"
on phones for over a decade and has since added non-list layouts (boards, tables) without breaking its
outline model. That last point is the closest existing precedent for Wombat.

### How Workflowy works on mobile (as documented)

| Pattern | What Workflowy does | Praise / complaints | Source |
|---|---|---|---|
| **Zoom into any bullet** | Tapping a bullet's dot makes it "your whole world": everything else is hidden, its text becomes the view's title, children below | The defining feature; other apps' users ask for it (Reflect, NotePlan) | [Get started](https://community.workflowy.com/t/get-started/1650), [beta help](https://beta.workflowy.com/help/bullets), [Reflect](https://roadmap.reflect.app/feature-requests/p/ability-to-zoom-into-a-node), [NotePlan](https://feedback.noteplan.co/general-feature-request/p/workflowy-like-zooming-or-hoisting) |
| **Breadcrumbs + zoom out** | Trail at the top; tap any crumb to zoom out to that level; Home icon to root; Back button; star (bookmark) lists | One user complained when a redesign truncated the breadcrumb into the menu bar ("navigation harder") | [Navigate around](https://beta.workflowy.com/help/navigate-around), [iOS app post](https://blog.workflowy.com/workflowys-new-mobile-app-for-ios-is-live/), [design comments](https://blog.workflowy.com/workflowy-has-a-new-design/comment-page-1/) |
| **Zoom animation** | Animated zoom in/out between levels | Users asked Workflowy to keep it: "the right amount of it makes a software come alive"; one suggested the same animation for moves | [design comments](https://blog.workflowy.com/workflowy-has-a-new-design/comment-page-1/) (anecdotal, ~2016) |
| **Collapse / expand** | Arrow beside a bullet toggles children in place; zoom (dot) vs expand (arrow) are separate targets | iPad bug needing double-taps to expand/collapse/zoom fixed Jan 2026, showing the two small targets are touch-sensitive | [beta help](https://beta.workflowy.com/help/bullets), [what's new](https://workflowy.com/whats-new/) |
| **Swipe actions** | Swipe right = complete (again = uncomplete); swipe left = bullet menu (share, duplicate, delete, full menu); 2026: swipe left also "Move to Tomorrow / Next Week" | One-hand completion praised (shopping lists). Accidental completions, no setting to disable; swipes fire when scrolling boards horizontally (official advice: scroll boards with two fingers); one user lost the visible checkmark so swipe was the only way | [swipe post + comments](https://blog.workflowy.com/swipe-right-on-us-mobile-shortcuts/), [share/duplicate post](https://blog.workflowy.com/you-can-now-share-and-duplicate-items-from-the-mobile-apps/) |
| **Toolbar above keyboard** | Since the 2018 iOS rewrite, edit controls sit above the keyboard "for improved one-hand usage": indent, outdent, undo, redo, @, #tag, link, mirror | 2026 release notes fix toolbar lag and keyboard covering the edited node, so it's heavily used and still being tuned | [iOS app post](https://blog.workflowy.com/workflowys-new-mobile-app-for-ios-is-live/), [what's new](https://workflowy.com/whats-new/) |
| **Drag to reorder** | Historically: hold the dot ~2 s, then drag; desktop: drag right to indent | Long-standing complaints: handle hard to hit, gesture undiscoverable, a "cluster" of drag bugs (Workflowy's own words, 2018), unclear drop targets | [reviews thread](https://talk.dynalist.io/t/moving-items-in-ios-app-is-difficult-usability-issue/1405), [Workflowy iOS posts](https://blog.workflowy.com/workflowy-now-starts-twice-as-fast-on-ios/), [drag-to-indent](https://blog.workflowy.com/moving-bullets-is-now-easier-than-ever/) |
| **Multi-select + Move/Mirror (2025)** | Long-press to enter multi-select; drag handle moves the selection; a **bottom toolbar** offers type change, duplicate, **move**, **mirror**, export; long-press a selected node starts drag | Shipped Sep–Nov 2025 with several reliability fixes after: the newest, most considered answer to "moving between levels on a phone" | [what's new](https://workflowy.com/whats-new/), [forum](https://community.workflowy.com/t/mobile-multi-select/662) |
| **Move To** | Desktop: bullet menu "Move To…" or `/move` (Dec 2022). Older Android move flow described as "cumbersome, several pop-ups" | Shows that a destination picker is the explicit fallback to drag | [Move To post](https://blog.workflowy.com/move-on-up-move-to-is-now-available/) |
| **Mirrors** | Live copies of a bullet in several places; edits sync; bullet menu shows "N mirrors, see them"; if one of two is deleted, the other reverts to a normal bullet; search can include/exclude mirrors | Praised for tasks belonging to several projects, boards showing one card in two columns | [mirrors post](https://blog.workflowy.com/mirrors-create-live-copies-of-any-bullet/), [support](https://workflowy.zendesk.com/hc/en-us/articles/4410177946004) |
| **Search filters the tree in place** | Typing filters the current view in real time; matches shown with ancestors (and descendants); edit directly in results; tap a #tag to filter; combinable filters, `-#tag` to exclude; star a search as a shortcut; later added expand-while-searching | Users had complained they couldn't browse while a tag filter was active, hence expand-in-search | [Get started](https://community.workflowy.com/t/get-started/1650), [search help](https://workflowy.com/help/search/), [expand in search](https://blog.workflowy.com/workflowy-feature-expand-while-in-search-mode/) |
| **Boards** | Any node can be a board: first level = columns, second = cards. Moving a card between columns is just moving a node, so the outline stays intact. Lists can contain boards and vice versa | Mobile: column minimum width added (Nov 2025), board scrolling fix (Apr 2026), swipe vs horizontal scroll conflict | [boards](https://community.workflowy.com/t/boards/1670), [what's new](https://workflowy.com/whats-new/) |
| **Tables** | Any node can be a table: children = rows, their children = cells; a "Header" row names columns; **cells are full nodes and can hold nested bullets, but cells with nested content can't be edited inline and are marked with a left border**; convertible back to a list | Early users: no way to see whether a cell had children (since fixed with the border) | [tables](https://community.workflowy.com/t/tables/1647), [tables announcement](https://community.workflowy.com/t/new-bullet-type-tables/1156) |

General quality notes: older reviews said the iOS app lagged the web app's features; Workflowy has spent
2025–2026 on mobile multi-select, drag, toolbar latency and keyboard fixes
([what's new](https://workflowy.com/whats-new/)). Animation and breadcrumb evidence is thin (a handful
of user comments).

### The key insight: "layout is a property of a node"

Workflowy never changed its data model to add boards and tables: a node's children stay a tree, and the
node chooses how to **lay them out** (list, board, table). Its table layout is *literally a grid whose
cells may hold nested structure*, shown with a marker and opened rather than edited inline. That is
the closest shipping precedent for Wombat, and it suggests:

- In Wombat the model is the reverse (every node is a grid; a 1-column grid is a list), but the same
  idea applies: **each grid can be *displayed* as Grid or List (or Board) on this device**, as per-device
  view state, without changing the document. This strengthens Concept 2.
- Workflowy's choice to make nested-content cells "open, don't edit inline" with a visible marker is
  exactly the Concept 1 preview + dive rule, validated in a shipping product (desktop and mobile).

### Mapping each pattern onto nested grids

| Workflowy pattern | Verdict for Wombat | How |
|---|---|---|
| Zoom into any bullet, it becomes the whole screen | **Carries over directly** | Dive into any cell's grid; the cell's text becomes the screen title (same as TreeSheets' ancestor header). Concept 1. |
| Breadcrumb trail, tap crumb to zoom out, Home | **Carries over, adapted to thumb reach** | Keep a full breadcrumb (users complained when Workflowy truncated it), but put the Up control and a compact trail at the bottom; tap trail for the full path; Home = document root. |
| Zoom animation | **Carries over, harder in 2D** | A list item grows into a page in 1D; a cell must grow from an arbitrary rect, and its preview must morph into a full grid. Reuse the preview painter at interpolated scale. |
| Collapse/expand in place | **Needs adapting** | In List mode, identical. In Grid mode, "expand in place" means showing the child grid inline, which only works when it fits legibly; otherwise fold = show summary badge only. Separate visible targets for "select" and "open", as Workflowy separates dot and arrow, at 44 pt. |
| Swipe right/left on items | **Mostly doesn't fit** | Grids need horizontal scrolling, and Workflowy's own boards suffer swipe/scroll conflicts. No "complete" concept in Wombat. Allow swipe-left-for-menu only in List mode rows; in Grid mode use tap-select + toolbar/long-press menu. |
| Toolbar above keyboard | **Carries over directly** | Same placement and purpose; replace indent/outdent with grid-structure buttons (insert row/column, wrap in grid, unwrap, move row/col), plus undo/redo, link, tag. |
| Indent / outdent | **Needs adapting (2D)** | "Indent" = wrap the selected cells into a new sub-grid (or move into the neighbouring cell's grid); "outdent" = move the cell(s) out to the parent level next to their container. Must name direction (into which cell? after which row/column?), so make these explicit buttons with a destination preview. |
| Drag to reorder | **Adapt carefully** | Within a level: long-press a row/column header to drag it (1D, like Workflowy). Dragging single cells in 2D is ambiguous (swap? insert and shift right or down?), so start with rows/columns and use the Move picker for cells. Workflowy's history of drag complaints argues for not making drag the only way. |
| Multi-select + bottom toolbar (Move, Mirror, Duplicate) | **Carries over directly** | Selection in a grid is a rectangle (TreeSheets `Selection`), plus a bottom toolbar: Move to…, Duplicate, Link, Style, Delete. Concept 4. |
| Move To picker | **Carries over** | Picker browses levels with the same dive UI and offers "place here" targets. Concept 4. |
| Mirrors (live copies) | **Doesn't fit yet** | Wombat's core plan has links and backlinks by cell ID, not transclusion. Links + backlinks cover "this belongs in two places" partly; a mirrored cell (one cell shown in two grids) is a core-model decision for later, not a UI one. |
| Tags | **Carries over** | Tags are already part of Wombat's planned search (milestone 6); show them as tappable chips in cells that start a filter. |
| Search filters the tree in place | **Needs adapting (2D)** | In List mode, identical (matches + ancestors). In Grid mode: dim non-matching cells and hide rows with no match in the current level; cells whose sub-grids contain matches get a badge ("3 inside") so you can dive straight to them; a global results list (with location trail) for matches outside the current level. Keep editing directly in filtered views. |
| Star (bookmark) a list or a search | **Carries over** | Pinned grids and saved searches in the left drawer (Obsidian-style). |
| Boards layout | **Carries over as a layout option** | A grid with one row of cells, each holding a 1-column grid, is a board. Could be a third display mode later; low priority. |
| Tables layout (nested content marked, not edited inline) | **Carries over directly** | It is Wombat's preview rule. |

### Effect on the ranking

Workflowy evidence raises confidence in Concept 1 (zoom = one level per screen, with breadcrumbs and
an animation users like), in Concept 2 (layouts per node without changing the model), and in
Concept 4 (its 2025 answer to moving between levels is multi-select + bottom toolbar + Move/drag, after
years of drag complaints). It lowers confidence in swipe-based cell actions for grids. The ranking in §4
is unchanged in order, but Concepts 1, 2 and 4 now carry Workflowy-derived details.

---

## 3. Principles and research to design against

| Principle | What it says | Implication for nested grids |
|---|---|---|
| **Touch targets** | Apple HIG 44×44 pt; Material 48×48 dp (~7–9 mm) ([summary](https://tetralogical.com/blog/2022/12/20/foundations-target-size/), [ishadeed](https://ishadeed.com/article/target-size/)) | A 390-pt-wide iPhone fits at most ~8 tappable columns, realistically 3–4 readable text columns. Child grids drawn inside a cell are **not** individually tappable unless the cell is huge, so a nested preview must be a single target ("open this grid"), not a field of tiny cells. |
| **Thumb reach / Fitts's law** | Hoober's 1,333 street observations: 49% one-handed, 36% cradled, 15% two-handed, grips change every few seconds ([UXmatters](https://uxmatters.com/mt/archives/2013/02/how-do-users-really-hold-mobile-devices.php)); his own caveat: not scientific | Put Up/breadcrumb and the main actions at the **bottom** (in the thumb zone), not in a top bar as TreeSheets and most breadcrumbs do. |
| **Visibility of system status / wayfinding** (Nielsen heuristic 1) | Users must always know where they are | Persistent location indicator (breadcrumb + depth) on every level; animated transitions that show where you came from. |
| **Recognition over recall** (heuristic 6), working-memory limits | People hold only a few chunks in working memory (Cowan's ~4) | Showing previews of child grids beats making users remember what's inside a cell; one level at a time is fine *if* the level shows hints of what's below. Toggles that force remembering values across views are bad for comparison (NN/g, below). |
| **User control and freedom** (heuristic 3), error recovery | Undo, easy exit | Every dive must be reversible with one obvious control and with the platform back gesture (iOS edge swipe, Android back). Undo for every structural edit (core already guarantees, A9). |
| **Consistency and platform conventions** (heuristic 4) | Use standard patterns | Navigation-stack semantics (push/pop) for diving; bottom sheets for peek; long-press for context menus; system back gesture = up one level. |
| **Gesture discoverability** | NN/g: contextual swipe lacks signifiers; keep hidden swipes for non-critical actions; avoid conflicts with navigation swipes ([NN/g](https://www.nngroup.com/articles/contextual-swipe/)) | Pinch-to-dive and swipe-to-sibling are accelerators only; each has a visible button equivalent. Avoid horizontal swipes on cells, because horizontal scrolling is needed for wide grids. |
| **Mobile tables** | NN/g: horizontal scrolling is "somewhat acceptable" for big tables if more content is signposted; lock the row-header column and header row; let users pick which columns to show ([NN/g](https://www.nngroup.com/articles/mobile-tables/)). Hoober: "tables need to be tables"; reflowing into blocks harms scanning ([UXmatters](https://www.uxmatters.com/mt/archives/2020/07/designing-mobile-tables.php)) | Keep a real grid as the default; add a List mode as an *option*, not an automatic reflow; pin first row/column when a grid is wider/taller than the screen. |
| **Breadcrumbs on mobile** | NN/g: costs can outweigh benefits on mobile; never wrap to multiple lines; show last tier or truncate into overflow ([NN/g](https://www.nngroup.com/articles/breadcrumbs/)) | Single-line breadcrumb that shows the parent name and a depth count, tap to expand the full path in a sheet. |
| **Zoomable UIs** | Pad++ and semantic zoom: objects render differently by scale ([Bederson & Hollan](https://www.lri.fr/~mbl/Teach/SituatedComputing/2017/papers/ZUI/pad++-uis94-p17-bederson.pdf)); Raskin's ZoomWorld argued spatial memory beats folder mazes ([Wikipedia](https://en.wikipedia.org/wiki/The_Humane_Interface)); **"desert fog"**: users get lost when the view has no navigational cues ([Jul & Furnas UIST'98, via Bederson retrospective](https://cs.umd.edu/~bederson/images/pubs_pdfs/p23-bederson.pdf)) | Semantic zoom is right for previews (a child grid becomes a summary at small size). Free continuous zoom on a phone risks desert fog and illegible text; snap to levels instead. |
| **Focus+context / fisheye** | DateLens (fisheye calendar on PDAs) was significantly faster for complex tasks, but satisfaction was mixed ([TOCHI 2004](https://microsoft.com/en-us/research/wp-content/uploads/2004/03/tochidatelens.pdf)); DOI trees fit hierarchies into bounded space by interest ([Card & Nation AVI'02](https://homes.cs.washington.edu/~jheer/files/2004-DOITree-AVI.pdf)) | Real evidence that fisheye *can* help on small screens, for specific tasks. Worth an experiment, not the foundation. |
| **Test size** | NN/g: 5 users find most problems in a qualitative round; prefer several small rounds; quantitative needs ≥20 ([NN/g](https://nngroup.com/articles/why-you-only-need-to-test-with-5-users/)) | Plan 2–3 rounds of 5 users. |

---

## 4. Interaction concepts, ranked by practicality

Each concept lists: how it works, principles it rests on, evidence for/against, failure modes, scaling,
desktop widening. Concepts 1–4 are complementary and together form the recommended design; 5 and 6 are
alternatives for larger screens.

### Concept 1 (rank 1): Level view with child previews ("Dive", Workflowy zoom for grids)

**How it works.** The screen shows one grid: the current level. Cells are full-size and readable. A cell
that holds a grid shows a **preview** inside it: the first few child cells' text at reduced size if
they fit legibly, otherwise a compact summary (dimensions like "3×4", first cell's text, a mini-map of
the shape). Tapping a cell selects it; tapping the preview area (or a chevron badge, ≥44 pt) **dives**:
an animated transition grows that cell to fill the screen and its grid becomes the current level. At the
bottom, a one-line location bar shows "‹ Parent name · depth 3"; tap ‹ (or the system back gesture) to go
up one level; tap the name to open the full path as a list and jump to any ancestor. Pinch-out on a cell
dives, pinch-in goes up: accelerators that mirror the visible buttons. Search results, links and
backlinks jump straight to the target level (TreeSheets' `ScrollOrZoom` rule: show the deepest level
that contains the target), and the location bar shows the new path.

**Principles.** Visibility of status (location bar always on), recognition over recall (previews show
what's inside), user control (one-tap up and system back), platform conventions (navigation stack),
Fitts/thumb zone (controls at the bottom), touch targets (one big target per nested preview).

**Workflowy-derived details.** Cell text becomes the screen title on dive; separate visible targets
for select and open (Workflowy's dot vs arrow); Home and starred grids in the drawer; a full path
available in one tap (a user objected when Workflowy truncated it); keep an animated zoom (users said it
makes the app "come alive"). In-place search: dim non-matching cells, hide non-matching rows, badge cells
whose sub-grids contain matches.

**Evidence.** Strongest of all concepts: it is the pattern of Workflowy/Dynalist zoom (and Workflowy's
table layout, where nested-content cells are marked and opened, not edited inline), MindNode focus
mode, Muse tap-to-enter (which lands where you tapped), Notion page push, and TreeSheets' own
`drawpath` zoom. Users of other apps explicitly request outliner-style zoom (Reflect roadmap).
Against: none of these apps nests a *2D* grid, so evidence that previews are readable enough is missing.

**How it could fail.** (a) Users lose the "big picture" and feel tunnel vision, especially when
comparing two sub-grids in different branches. (b) Previews too small to read, so users dive blind.
(c) Tap-to-select vs. tap-to-dive confusion. (d) Too many dives for a simple lookup (each dive costs
~300 ms of animation plus re-orientation).

**Mitigations.** Peek (concept 3) for compare-without-committing; a visible chevron as the only
dive target on a cell, so a tap elsewhere just selects; quick switcher/search to skip levels; remember
scroll position per level (per-device view state, A8).

**Scaling.** Depth: unlimited, because only one level plus one preview level is ever built. Breadth:
a level with 200×20 cells is an ordinary 2D scroll with pinned header row/column. Deep paths: the
location bar truncates; full path in a sheet.

**Desktop.** Same model, but the extra width shows previews at larger size (two legible levels), and
the location bar moves to the top as a full breadcrumb. Optionally, a side pane shows the parent level
(like a column browser). This is close to what TreeSheets does now, which is reassuring.

### Concept 2 (rank 2): Grid/List toggle per level ("Hybrid", Workflowy-style layouts)

**How it works.** Any level can be shown as a **Grid** or as a **List**. List shows rows as cards: the
first column as the card title, other columns as labelled lines below (Numbers form view / Airtable
card style), and nested grids as preview rows that open with the same dive. A single-column grid (very
common in TreeSheets documents, which are often used as outlines) is effectively a list already, and can
default to List. The choice is remembered per grid per device (view state, A8). A segmented control
in the level's header switches mode: visible, not a hidden gesture.

**Principles.** Flexibility and efficiency of use (heuristic 7): each representation suits different
tasks. Legibility: List mode never needs horizontal scrolling. Consistency: one dive model in both modes.

**Evidence.** For: Workflowy added boards and tables as *layouts of a node* without changing the outline
model, and both convert back to lists; MindNode added an outline because canvas search on small screens is hard; Numbers'
form view and Airtable cards exist for phone record editing. Against: NN/g and Hoober both warn that
reflowed tables hurt comparison and scanning; Airtable itself says expanded records made comparison
hard. So List must be a *choice*, not an automatic transform of wide grids.

**How it could fail.** Users don't understand the two views show the same data; mode switches cause
disorientation (lost scroll position); column labels are often absent in free-form grids (TreeSheets
grids rarely have a header row), so cards have nothing to label lines with.

**Scaling.** List handles long, narrow data very well; wide data with many columns makes long cards.
**Desktop.** Mostly Grid; List remains useful for outline-shaped grids.

### Concept 3 (rank 3): Peek sheet ("look without leaving")

**How it works.** Long-press a cell with a sub-grid (or tap a "peek" icon) to open a **bottom sheet**
showing that sub-grid at full size, editable. Drag the sheet up to full height to commit and dive;
swipe it down to return. Only one peek at a time; inside a peek, the dive control dives for real (no
nested sheets). Long-press also shows the standard context menu (cut, copy, move to..., fold, style).

**Principles.** User control (cheap to undo the navigation), recognition (see contents in place,
parent still visible above the sheet), platform conventions (iOS context menus and sheets, Notion's
"side peek" on desktop).

**Evidence.** Bottom sheets and long-press previews are platform-standard. Airtable's lesson (expanding
records is an extra step) suggests peeking should be cheap and not modal-feeling.
Against: long-press is a hidden gesture (NN/g), so it needs a visible equivalent; sheets stacked on
sheets are confusing.

**Failure.** Long-press conflicts with text selection and drag; users confuse peek with dive.
**Scaling.** Independent of depth (always one level). **Desktop.** Becomes a hover/side panel or
split view.

### Concept 4 (rank 4): Explicit restructuring (multi-select, "Move to..." and the pocket)

**How it works.** Restructuring across levels is where every mobile outliner struggles, so make it
explicit and boring first:
- **Select** one or several cells (tap, then a "select" mode with a visible checkbox bar, as Files/Mail).
- **Move to...** opens a picker that browses the tree level by level (same dive UI) with
  "Place here" buttons: into a cell as a new sub-grid, before/after a row, into a new column. Same as the
  iOS Files "Move" sheet.
- **Pocket**: Cut puts cells into a visible tray at the bottom ("2 cells in pocket"); navigate anywhere;
  Paste. The pocket persists across dives, so cut-and-paste across levels needs no drag.
- **Toolbar above the keyboard** (Obsidian-style) shows nest/unnest-like actions as buttons: "Wrap in
  grid", "Unwrap", "Insert row", "Insert column", "Move row up/down". Always visible while editing,
  addressing Logseq's missing-Tab problem.
- **Drag and drop** within a level (reorder rows/columns, long-press then drag) as an accelerator;
  dragging onto the location bar's ‹ moves the item up a level after a hover delay (like springboard
  folders). Treat cross-level drag as phase 2.

**Principles.** Recognition over recall (destination shown, not remembered), error prevention
(explicit destination), visible controls over hidden gestures, undo for every move.
**Evidence.** Workflowy's 2025 mobile answer to the same problem is long-press multi-select, a drag
handle, and a bottom toolbar with Move/Mirror/Duplicate, after years of complaints that its 2-second-hold
drag was hard to discover and hit; its desktop has Move To. Files/Mail move sheets are familiar; Logseq/Obsidian complaints show discoverability of
structure ops is the pain point; Notion mobile lacks drag-into-page. Cut/paste as a cross-context move
is universal. Against: a picker is slower than drag for experts; no study that the pocket is understood.
**Scaling.** Picker uses search + quick switcher for deep targets. **Desktop.** Real drag and drop,
keyboard shortcuts (TreeSheets has these), pocket stays as the clipboard.

### Concept 5 (rank 5): Focus+context grid ("fisheye level")

**How it works.** Within the current level, the selected row and column get full size; others are
compressed to narrow strips showing only first characters, the way DateLens compresses days. Dragging
the focus scrubs across the grid. Could also expand a nested cell inline (DOI-tree style) while
compressing siblings, showing 2 levels at once around the focus.
**Principles.** Focus+context reduces navigation for overview+detail tasks.
**Evidence.** DateLens/FishCal: faster on complex tasks on PDAs, mixed satisfaction; DOI trees scale
to large hierarchies on desktop. Against: distortion makes scanning and spatial memory harder; touch
precision on compressed strips is poor (below 44 pt); much more complex to build.
**Failure.** Jittery layout on every selection change; can't read compressed cells; motion sickness
during scrubbing. **Scaling.** Decent breadth; depth only 2 levels. **Desktop.** Useful on big grids;
maybe as an option.

### Concept 6 (rank 6): Continuous zoomable canvas ("whole document ZUI")

**How it works.** TreeSheets' whole-document rendering with free pinch zoom; semantic zoom swaps
detail levels as you zoom (small child grids become summary blocks, then expand to full cells).
**Principles.** Spatial memory (Raskin), semantic zoom (Pad++).
**Evidence.** Muse and Heptabase show it works on iPad; Heptabase reviewers find arranging on a phone
awkward; MindNode added an outline because finding things on a canvas on a small screen is hard; desert
fog is a known ZUI failure. Against for phones: text legibility and touch targets break after one level
of shrink.
**Failure.** Users get lost; accidental zooms; editing at non-1:1 scale is awkward; heavy rendering.
**Scaling.** Needs level-of-detail culling (hard). **Desktop/iPad.** This is where it belongs; it
is what TreeSheets desktop users already know. Consider it for the desktop widening, sharing the
same preview renderer as concept 1.

### Cross-cutting: editing one cell at a time

The early idea of a **single ordinary text field** for editing is well supported: it matches the
spreadsheet formula bar (the "safest and most precise" edit path per guides,
[Excel touch guide](https://support.microsoft.com/en-us/excel/excel-for-android-touch-guide)) and
Numbers' form fields. Suggested behaviour:
- Tap a selected cell (or the edit button) to open the editor docked above the keyboard, with the
  formatting/structure toolbar above it. The cell scrolls into view above the keyboard and is
  highlighted; text updates live in the cell.
- Return = commit and move to the cell below (spreadsheet convention); a toolbar "→" moves right;
  a "new line" key is explicit. Avoid Google Sheets' complaint of an extra tap before typing:
  typing on a selected cell with a hardware keyboard starts editing immediately.
- A big benefit of single-editor: if Flutter's own text input falls short on iOS (see §5), **one**
  native `UITextView` as a platform view is affordable, while making every cell a native view
  would not be. **[Speculation]**

---

## 5. Flutter feasibility (late 2026)

| Need | Tool | Maturity / notes |
|---|---|---|
| Lazy 2D grid with pinned header row/column, merged cells | `two_dimensional_scrollables` **TableView** | v0.5.6, verified publisher flutter.dev, published ~early Nov 2026; supports pinned rows/columns, merged cells, infinite rows/cols, diagonal or axis-locked scrolling; **TreeView** also included ([pub.dev](https://pub.dev/packages/two_dimensional_scrollables)). Pre-1.0: expect API churn; 0.4.x fixed merged/pinned interaction bugs ([changelog](https://pub.dev/packages/two_dimensional_scrollables/versions/0.4.2/changelog)). Row/column extents must be supplied by you, so content-sized columns (TreeSheets-style) need a Dart measuring pass with `TextPainter` and caching. |
| Pinch/pan over a large canvas | `InteractiveViewer.builder` | Gives the visible viewport `Quad`; the official sample builds only visible table cells ([API](https://api.flutter.dev/flutter/widgets/InteractiveViewer/builder.html)). Culling is your job. Mainly needed for concept 6. |
| Child-grid previews inside cells | `CustomPainter` / `RepaintBoundary` | Paint previews as pictures, not widget trees: cheap, and cacheable by grid ID + version. |
| Dive animation | Custom transition from cell rect to full screen (`AnimatedBuilder` + `Matrix4`/`RectTween`), or the `animations` package's container transform; `Hero` works for route pushes | Making the transition **gesture-driven and reversible** (follow the pinch, cancel mid-way) needs a custom `AnimationController` driven by the gesture. **[Speculation on effort: a few days.]** |
| Redraw only changed cells | Map cell ID → `ValueNotifier`/keyed widget; each core command returns changed IDs (A10) | Natural fit: only visible cells exist as widgets, so most changed IDs are no-ops. Previews keyed by grid ID invalidate when any descendant within preview depth changes. |
| Rust core binding | `flutter_rust_bridge` 2.13.0 (Flutter Favorite; all six platforms) ([pub.dev](https://pub.dev/packages/flutter_rust_bridge)) | Mature. Keep calls coarse (one call per command, returning a changeset), not per cell per frame. |
| Text editing on iOS | `TextField`/`CupertinoTextField` (Flutter-drawn, talks to the iOS text input system) | Known gaps historically: autocorrect tooltip issues ([flutter#12920 referenced](https://docs.flutter.dev/release/breaking-changes/add-showAutocorrectionPromptRect)), some IME candidate issues on iPad; packages exist wrapping native `UITextView` because Flutter fields "don't use native UI components" ([flutter_native_text_input](https://pub.dev/packages/flutter_native_text_input/versions/0.1.1)). Status of individual issues in 2026 unverified. **Test typing on a real iPhone on day 1**, including autocorrect, dictation, CJK input and cursor dragging. |
| Accessibility | `Semantics` + `SemanticsRole` | Roles include `table`, `row`, `cell`, `columnHeader`, `list`, `listItem` but **no tree/treeItem** role ([API](https://api.flutter.dev/flutter/dart-ui/SemanticsRole.html)). Platform mapping detail per role is undocumented. The level view is a good fit: each level is a table; a nested cell is announced as "Project plan, contains grid 3 by 4" with custom semantic actions "Open grid" and "Up one level". List mode doubles as an accessible representation. Test with VoiceOver and TalkBack early. |

**Performance reasoning [Speculation, to measure]:** with one level built at a time, a phone builds
perhaps 50–150 visible cell widgets plus painted previews, independent of document size or depth.
The real costs are text measurement for column sizing of large levels (cache by cell version) and
the initial load of huge documents across the bridge (load lazily by level, if the core API allows).
Continuous ZUI (concept 6) is the only concept that needs serious level-of-detail work.

---

## 6. Recommended approach

Build **Concept 1 (Level view with previews)** as the backbone, plus **Concept 2 (Grid/List toggle)**,
**Concept 3 (Peek)** and **Concept 4 (Move to... + pocket + keyboard toolbar)**, with the single-field
editor. Rules of thumb:

1. Every action has a visible control; gestures (pinch dive, edge-swipe up) only accelerate.
2. Location always visible at the bottom; system back = up one level.
3. Only one level is ever fully interactive; one level below is a readable preview, deeper is a summary.
4. Real grids by default (with pinned headers when wider/taller than the screen); List mode as a choice.
5. Jumps (search, link, backlink) land on the deepest level containing the target, as TreeSheets does.

What would genuinely be new: legible previews of nested 2D content, a gesture-driven reversible dive
that preserves spatial continuity, and restructuring across levels that is easy on a phone. Those
three are the "pioneering" part; the rest is deliberately familiar.

---

## 7. Prototype plan (days, not weeks)

The Wombat core has no milestones done yet, so **don't wait for it**: build a throwaway Flutter app
with an in-memory Dart model (cells with text, optional child grid, IDs). Load a realistic document:
convert one or two TreeSheets examples (e.g. `TS/examples/tutorial.cts`, or a user's real `.cts`)
to JSON with a small script, plus a synthetic deep/wide stress document (depth 8, a 200×10 level).

**Day 1:** Level view with TableView, location bar, up/back, tap-to-dive with a simple animation; the
single-field editor with toolbar. Run on a real iPhone (and one Android) and check typing quality.
**Day 2:** Previews (painted), pinned header row/column, search-jump to deep target, per-level scroll
memory. **Day 3:** List mode toggle; Peek sheet; pocket + "Move to..." picker; undo. **Day 4:** Polish
transitions, gesture-driven dive; build variant A (Level view, Grid only) and variant B (Level view +
List mode + Peek) behind a switch. **Day 5:** First usability round.

**Tasks** (each user, both variants, order counterbalanced):
1. Find the cell "X" three levels deep, starting at the top (no search).
2. Change the text of a cell two levels deep, then return to the top.
3. Add a row to a sub-grid and fill in two cells.
4. Move a cell from one sub-grid to a sub-grid in another branch (a different level).
5. Compare two values that live in two sibling sub-grids, and say which is larger.
6. Turn a cell into a 2×2 grid and fill it.
7. Undo the last three changes.
8. Using search, find every cell tagged #todo inside one branch and open one of them (tests the
   in-place filter with badges, Workflowy-style).

**Measures:**
- Task success (yes / with help / no) and time on task.
- Errors: wrong cell edited, accidental dive, accidental edit/selection, wrong destination.
- **Getting lost**: count of "where am I?" moments (verbal or observed back-and-forth), number of
  ups/dives versus the optimal path (lostness ratio = actual steps / minimum steps).
- After each task, Single Ease Question (1–7); at the end, which variant they prefer and why.
- Technical: frame times in DevTools profile mode on the real phone (target no janky frames during
  scrolling and dive), keyboard latency, VoiceOver walkthrough of tasks 1–3.

**Participants:** 5 per round (NN/g: about 85% of problems in a qualitative round), mixing 2–3
spreadsheet/outliner users and 2–3 general phone users; 2–3 rounds with fixes between. Do not
compute statistics on 5 users; for a quantitative comparison later, use 20+.

**Success criteria (proposed, adjust after round 1):**
- Tasks 1–3 completed unaided by ≥4 of 5 users, each in under ~1 minute.
- Task 4 (cross-level move) completed unaided by ≥3 of 5 in round 1, ≥4 of 5 by round 3.
- Lostness ratio under ~1.5 on task 1; at most one "where am I?" per user per task.
- Users can explain what a preview means without being told.
- No dropped frames during dive transitions on a 2–3 year old iPhone; typing feels native.

**Kill or pivot signals:**
- Users routinely misread previews or dive blind → enlarge previews, try showing two levels inline on
  tap (a concept 5 lite).
- Cross-level moves fail even with the picker → rethink structure editing before anything else.
- Flutter typing quality is unacceptable on iOS → swap the single editor for a native platform view.
- Users repeatedly ask for "the whole picture" → prototype concept 6 on iPad as a companion view.

---

## 8. Biggest risks

1. **Previews may be unreadable** at phone sizes, turning the level view into blind navigation
   (no evidence either way; test first).
2. **Comparing across branches** is inherently awkward in a one-level-at-a-time model; Peek and split
   views only partly help.
3. **Structure editing on touch** is the weakest point of every prior app; Wombat may hit the same wall.
4. **Flutter iOS text input** quality, historically imperfect; verify on device on day 1.
5. **Accessibility of nesting**: no tree role in Flutter's `SemanticsRole`; depends on good custom
   semantics labels and actions.
6. **Pre-1.0 TableView package**: API churn; a custom `RenderObject` may be needed if content-sized
   columns or previews don't fit its model.
7. **Evidence base is thin**: almost all prior-art evidence is vendor docs, forums and reviews; no
   published study of nested grids on phones was found, so the user tests are the real evidence.
