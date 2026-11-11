> Background research, not instructions. The interface specification is `../UI_SPEC.md`; where this file differs from it, the specification wins.

# Wombat on a phone: a gesture-driven mode alongside the buttons

The question: *"What if, on mobile, we have 100% gesture support instead of buttons? Both options
should be built."* This note designs a complete gesture way to work Wombat on a phone, sets it next
to the button design in `nested-grids-ui-brief.md`, and works out how both can live in one app.

Labels used below: **[E]** is evidence from a source, **[I]** is my own idea or judgement.

---

## 1. The short answer

- **Build both, but not as two apps.** Gestures are always on. Buttons are on by default. A setting
  called **Clean mode** hides the bottom bar and the "+" bars for people who have learned the
  gestures. **[I]**
- **Some things stay visible in Clean mode too:** the trail bar (it tells you where you are, so it
  isn't just a button), the editor's keyboard toolbar, the long-press menu and undo toasts. With these,
  nobody gets stuck. **[I]**
- **"100% gestures" means every action can be *reached* by a gesture.** It doesn't mean every action
  has its *own* gesture. About 8 gestures are worth learning. Everything else goes in the long-press
  menu, which is itself a gesture. **[I]**
- **If a screen reader or Switch Control is on, the buttons come back by themselves.** Every action is
  also offered as a screen-reader action. **[I]**, following Apple's rule below **[E]**.
- **Gestures and buttons call the same commands.** Each action is built once, so the extra cost is
  the gesture layer itself plus testing it, not a second app. **[I]**

---

## 2. Evidence: gesture-first apps and what happened to them

| Product | What it did | What happened | Lesson |
|---|---|---|---|
| **Clear** (2012) | A to-do list with no buttons. Pull down to add, pinch apart to insert between items, swipe to finish or delete, pinch to go up a level. | Much praised. Reviewers said it broke Norman and Nielsen's rules on visibility and discoverability, but its first-run walkthrough helped. Years later a user asked for the standard swipe-right to go back, and the team replied that edge swipes kept ticking off rows by mistake and were unreliable with phone cases. Back swipe would "have to be an opt-in setting". **[E]** [Fast Company](https://www.fastcompany.com/1668915/clear-a-to-do-list-app-with-a-ui-from-the-future), [iMore](https://www.imore.com/clear-review), [Clear forum](https://club.useclear.com/t/please-just-let-me-swipe-right-to-go-back-swipe-down-is-not-normal/3452) | Gesture-only can delight in a small app with few actions. Swipes on rows clash with system edge swipes, and that is still true a decade later. |
| **Mailbox** (2013–16) | Short and long swipes on a message to archive, snooze or list it. | Closed by Dropbox for business reasons. Its swipes became standard in Outlook, Spark and others, and those apps keep buttons as well. **[E]** [9to5Google](https://9to5google.com/2015/12/07/dropbox-mailbox-carousel-shutdown/), [Beebom](https://beebom.com/mailbox-alternatives/amp/) | Swipes on rows are a lasting accelerator, but they are rarely the only way. |
| **Paper by FiftyThree** | Undo by "rewinding": a two-finger circle. | User reports call it charming but say a sloppy pinch could be read as a rewind. Later versions promised a "quicker undo". **[E, weak: forum posts]** [MacRumors forum](https://forums.macrumors.com/threads/fiftythree-gives-behind-the-scenes-look-at-developing-papers-zoom-feature.1585592) | Clever gestures that look alike get mixed up. |
| **Procreate** | Two-finger tap undoes, three-finger tap redoes, hold to repeat. The gestures can be remapped. | A success with a professional, daily audience. It keeps visible undo and redo buttons, and it offers a *single-touch* option for people who can't tap with several fingers. **[E]** [Procreate Handbook](https://help.procreate.com/procreate/handbook/interface-gestures/gestures) | Gestures work for heavy daily users, with buttons kept and an accessible alternative. |
| **Things 3** | The "Magic Plus" is a visible button: tap it to add, or drag it to *where* you want the new item. **[E]** [Cultured Code](https://culturedcode.com/things/features/), [MacStories](https://www.macstories.net/reviews/things-3-beauty-and-delight-in-a-task-manager/) | Loved. | A visible control can also *be* a gesture. That is a good model for Wombat's "+". |
| **Workflowy / Dynalist mobile** | Swipe right to complete, swipe left for the bullet menu. Users ask for swipe-to-indent and can't find how to indent by touch. **[E]** [Workflowy blog](https://blog.workflowy.com/swipe-right-on-us-mobile-shortcuts/), [Dynalist forum](https://talk.dynalist.io/t/ability-or-option-for-swipe-to-indent/8666) | Even the main inspiration uses only two or three swipes, with a menu behind them. |
| **Superhuman mobile** | Remappable swipes, plus a command search reached by a gesture, which it calls "the most important" one to remember. Onboarding was led by people. **[E]** [Superhuman help](https://help.superhuman.com/article/461-mobile-customizing-swipes-and-triage-bar), [First Round](https://review.firstround.com/superhuman-onboarding-playbook/) | One gesture that opens "everything" is easier to remember than many gestures. |
| **Apple's own** | iOS 13 added three-finger swipe to undo/redo and three-finger pinch to copy/paste. Shake to undo is older. A three-finger tap shows a bar of buttons as the fallback. | John Gruber called shake to undo "awful and indiscoverable". The three-finger gestures clash with apps such as Obsidian, and iOS 16 made them fire too eagerly, even in games. **[E]** [Daring Fireball](https://daringfireball.net/2018/12/ios_still_hasnt_gotten_undo_right), [Obsidian forum](https://forum.obsidian.md/t/paste-gesture-on-ios-and-ipados-clashes-with-obsidian-swipe-gesture/45494), [9to5Mac](https://9to5mac.com/2022/09/15/ios-16-bug-three-finger-games/) | Even Apple pairs hidden gestures with a visible fallback, and system gestures take touches away from apps. |
| **Android gesture navigation** | Back is a swipe from *either* side edge. An app can opt out of only 200 dp per edge. **[E]** [Android docs](https://developer.android.com/training/gestures/gesturenav), [.NET API mirror](https://learn.microsoft.com/dotnet/api/android.views.view.systemgestureexclusionrects) | Both side edges belong to the system on Android. |

**Research [E]:**
- Norman and Nielsen, *Gestural Interfaces: A Step Backwards in Usability* (2010): gestures have no
  visible signs that they exist and no shared standards. [jnd.org](https://jnd.org/gestural-interfaces-a-step-backwards-in-usability/)
- NN/g on contextual swipe: swipe has no visible signs, so "burying key actions" behind it stops
  people finding them. Use it for destructive actions, keep one meaning per app, and make undo easy
  to see. [NN/g](https://www.nngroup.com/articles/contextual-swipe/)
- NN/g on hidden navigation: on phones, hidden navigation was used in 57% of cases and partly
  visible navigation in 86%. Hiding it made it about 20% less discoverable and 15% slower on mobile.
  This measured hidden menus, not gestures, but it shows what hiding costs. [NN/g](https://www.nngroup.com/articles/hamburger-menus/)
- Nacenta et al., CHI 2013: people remembered gestures they designed themselves better, by up to 24%,
  on the same day and the next. Most errors were *which gesture meant what*, not doing the movement
  wrong. [Paper](https://research-repository.st-andrews.ac.uk/bitstream/10023/3486/5/NacentaKamberQiangKristensson_GestureMemorability.pdf)
- Kurtenbach and Buxton, marking menus: the best way from beginner to expert is a visible menu whose
  *use* rehearses the expert movement. Marks were 3.5 times faster than the menu, and experts still
  went back to the menu to refresh their memory. [Buxton](https://www.billbuxton.com/MMUserLearn.html)
- Findlater and McGrenere and later work: menus that rearrange themselves often hurt speed and
  satisfaction, because things move. [Gajos review](https://kgajos.seas.harvard.edu/papers/AIMag09-AUIs.pdf)
- Apple, WWDC21 "Discoverable design": "use gestures as a shortcut, not a replacement"; people skip or
  forget tutorials. [WWDC notes](https://wwdcnotes.com/documentation/wwdc21-10126-discoverable-design/)

**What the evidence says, in one line:** gesture-only works in small, single-purpose apps with daily
users. In apps with many actions, gestures last as accelerators on top of visible controls.

---

## 3. Hard constraints on a phone

**What the platforms say [E].** Apple's HIG says a custom gesture must be "discoverable,
straightforward to perform, distinct from other gestures" and "not the only way to perform an
important action". It also says to "use shortcut gestures to supplement standard gestures, not
replace them" and to avoid gestures that clash with system ones
([HIG Gestures](https://developer.apple.com/design/human-interface-guidelines/gestures)). Material
gives the same advice. So, by the platforms' own rules, a gesture-only Wombat is allowed only as an
*option*, not as the only design.

**Areas the app can't use [E/I]:**

```
 ┌───────────────────────────────┐
 │▒▒▒▒▒ status bar: Notification ▒│ ← top edge: Notification Centre / shade.
 │▒▒▒▒▒ and Control Centre pulls ▒│   Start "pull to search" below this.
 │▒│                           │▒│
 │▒│                           │▒│ ← left edge: back on iOS and Android.
 │▒│      SAFE FOR WOMBAT      │▒│   Right edge: back on Android too.
 │▒│        GESTURES           │▒│   Keep about 24 pt free on both sides.
 │▒│                           │▒│
 │▒│                           │▒│
 │▒▒▒▒▒▒ home indicator / nav ▒▒▒│ ← bottom edge: home and app switching
 └───────────────────────────────┘
```

**Other constraints:**

1. **One finger already scrolls the grid in both directions.** Wide grids scroll sideways, so a
   sideways swipe *on a cell* is ambiguous. **[I]** Resolution: row swipes start only on the
   **pinned first column**, which never scrolls sideways.
2. **The editor owns text gestures.** Cursor dragging, double-tap to select a word, and the iOS
   three-finger undo/copy/paste all happen there. **[E]** (Obsidian clash) No Wombat gesture works
   inside the editor. **[I]**
3. **Screen readers take over gestures.** With VoiceOver or TalkBack on, one-finger swipes move
   between items and double-tap activates. Custom gestures are unreachable unless they are also
   offered as *custom actions* (VoiceOver rotor "Actions", TalkBack actions menu). Flutter supports
   this. **[E]** [Flutter a11y guide](https://dev.to/gdg/a-practical-guide-to-flutter-accessibility-part-2-hiding-noise-exposing-actions-2f7i)
4. **One hand.** Multi-finger taps need two hands or a phone on a table. Pinch is awkward one-handed
   on a large phone. **[I]** Every multi-finger gesture needs a one-finger route.
5. **Accidental triggers.** Pockets, phone cases, a palm on the screen, and gestures that look alike
   (pinch vs two-finger tap vs two-finger drag). **[E]** (Clear, Paper) Destructive gestures need an
   undo toast. **[E]** (NN/g)
6. **Android makers add their own gestures**, for example a three-finger swipe down for a screenshot
   on several brands. **[I, check per device]**

---

## 4. The gesture vocabulary

**How many is realistic?** Standard gestures cost nothing to learn: tap, double-tap, long-press,
drag to scroll, and system back. On top of those, **about 8 custom gestures** is the realistic
ceiling for a non-professional audience. **[I]** The reasoning:
- Recall errors are about *which gesture means what* (Nacenta), and they grow with the number of
  gestures.
- Procreate's well-known core is about that size.
- Clear managed with about 6.
- Everything past 8 goes in the long-press menu, where its name is visible. This works like a marking
  menu: you see the command, and you learn the gesture next to it.

**One idea runs through the set [I]: pinch is depth.** Spread your fingers to go *in* (dive, or make a
new grid inside). Pinch to come *out* (go up). This matches the zoom metaphor of the stage, and it
leaves sideways swipes free for rows.

### Cheat sheet

```
 ╔═══════════════════════════════════════════════════════════════════╗
 ║  WOMBAT GESTURES                                    (Clean mode)  ║
 ╠═══════════════════════════════════════════════════════════════════╣
 ║  STANDARD (nothing to learn)                                      ║
 ║   tap ............... select cell  ·  tap a preview/shape: DIVE   ║
 ║   tap again/2×tap ... edit cell                                   ║
 ║   hold .............. menu + larger preview (everything else)     ║
 ║   drag .............. scroll the grid                             ║
 ║   edge back ......... up one level                                ║
 ╠═══════════════════════════════════════════════════════════════════╣
 ║  CORE (8)                                                         ║
 ║ 1 spread on a child ........ DIVE   (on a plain cell: new grid    ║
 ║                                      inside, then dive)           ║
 ║ 2 pinch on the stage ....... UP one level                         ║
 ║ 3 2-finger tap ............. UNDO   (hold = keep undoing)         ║
 ║   3-finger tap ............. REDO                                 ║
 ║ 4 pull up past last row .... ADD ROW                              ║
 ║   pull left past last col .. ADD COLUMN                           ║
 ║ 5 swipe row handle ←  ...... DELETE ROW (undo toast)              ║
 ║   swipe row handle →  ...... SELECT ROW / start multi-select      ║
 ║ 6 2-finger drag over cells . SELECT A BLOCK                       ║
 ║ 7 hold + drag a cell ....... MOVE  (drop on a child = into it;    ║
 ║                                     hover child = dive; hover     ║
 ║                                     trail bar = up)               ║
 ║ 8 pull down at top ......... FIND  (pull further = everywhere)    ║
 ╠═══════════════════════════════════════════════════════════════════╣
 ║  EXTRAS (optional, off until tests say yes)                       ║
 ║   spread between two rows / two columns .... insert in the middle ║
 ║   swipe along the trail bar ................ back/forward history ║
 ║   3-finger pinch / spread on a selection ... cut to pocket/paste  ║
 ╠═══════════════════════════════════════════════════════════════════╣
 ║  IN THE EDITOR: Return = next row (adds one at the end).          ║
 ║  Swipe the keyboard down or tap outside = done.                   ║
 ║  No Wombat gestures inside the text.                              ║
 ║  MENU ONLY: group/ungroup, fold, style, colour, move to…,         ║
 ║  insert left/right/above/below, delete column                     ║
 ╚═══════════════════════════════════════════════════════════════════╝
```

### Every action in the brief

| Action | Gesture | Button (brief) |
|---|---|---|
| Dive | Tap a preview/shape/badge; spread on a child | Open › |
| Go up | Pinch; edge back; tap ‹ on the trail | ‹ |
| Jump history | Long-press the trail (recent places); swipe along the trail (extra) | Trail list |
| Select | Tap | — |
| Multi-select | Two-finger drag (a block); swipe the row handle right, then tap more cells | Menu: Select |
| Edit / end editing | Tap again or double-tap / swipe the keyboard down, or tap outside | Edit / ✓ |
| Add row/column at the end | Pull past the edge; Return / → while typing | + bars, +Row/+Col |
| Insert in the middle | Spread between rows/columns (extra); menu | Menu, +Row/+Col |
| New nested grid | Spread on a plain cell | Menu: New grid inside |
| Group / ungroup | Menu | Bar: Group / Ungroup |
| Move | Hold and drag (spring-loaded dive and up) | Move to… |
| Cut / paste (pocket) | Three-finger pinch / spread (extra); menu | Cut to pocket / Paste |
| Delete | Swipe the row handle left; menu | Delete |
| Undo / redo | Two- / three-finger tap; undo toast | Toast, ⋯ menu |
| Search | Pull down at the top | Find |
| Fold | Menu | Menu |
| Peek / preview | Hold (preview on top of the menu) | Same |
| Style | Aa in the editor; text selection menu; menu | Aa |

### Conflicts and how each is resolved

| Conflict | Resolution |
|---|---|
| Edge swipe for back vs a row swipe | The row handle is the pinned first column, which starts about 24 pt in from the edge. Left-edge swipes always go to the system. Clear's report shows this is a real problem, so the logs must count misfires. |
| Android back from the *right* edge vs "pull left past the last column" | Treat the pull as an overscroll that starts inside the grid. Never start it at the edge. |
| Sideways scroll vs row swipe | Row swipes only on the pinned column. Scrolling happens everywhere else. |
| Spread to dive vs spread to insert between rows (extra) | Dive needs both fingers on the same child. Insert needs the fingers in different stage rows. If the gesture is ambiguous, do nothing and show a hint. Keep it an extra until it's tested. |
| Pinch up vs two-finger tap vs two-finger drag | Told apart by movement: fingers apart/together = pinch; fingers parallel = drag; little movement and under about 250 ms = tap (as Procreate does). Tune it in the prototype. |
| Pull down to search vs Notification shade | Search only when the grid is already scrolled to the top and the pull starts below the title. |
| Pull up for a new row vs the home indicator | Only at the end of the grid's scroll, and the pull starts above the bottom safe area. |
| Hold for the menu vs hold and drag to move | Platform standard: hold shows the menu, and moving the finger turns it into a drag (as with iOS context menus). |
| iOS three-finger system gestures and three-finger redo/cut | iOS may take three-finger taps and pinches before Wombat sees them. Needs a day-one check like the typing test. Fallback: redo stays on the undo toast and in the menu, and cut/paste stay extras. |
| Gestures inside the editor | None. The editor belongs to the system's text handling. |
| Tap outside while editing | Ends editing only. It never dives (brief rule, kept). |

---

## 5. A phone screen in Clean mode

```
 ┌───────────────────────────────┐
 │ ▌Trip planning                │ ← title; pull down here for Find
 ├───────┬───────────┬───────────┤
 │ Italy │┌─────────┐│ 2,400     │
 │ ┆     ││Day1 Rome││           │ ← spread on it: dive
 │ ┆     ││Day2 Fl… ││           │
 │ ┆     │└─────────┘│           │
 ├───────┼───────────┼───────────┤
 │ Spain │ ▇▇ ▇ ▇▇   │ 1,900     │
 ├───────┼───────────┼───────────┤
 │ Notes │ Visa?   ▦4│           │
 ├───────┴───────────┴───────────┤
 │   ‹ Home › Trips › Trip pl… •••│ ← slim trail bar stays (pinch = up)
 └───────────────────────────────┘
   ┆ = the row handle (pinned column): swipe ← delete, → select
   No bottom bar, no "+" bars: pull past the edge instead.
   An undo toast still appears after every change.
```

Compared with the brief's screen, Clean mode gains about one row of grid, roughly 10% more cells on a
small phone. **[I]** That is the honest size of the reward. It is a calmer screen, not a new
ability.

---

## 6. How both options can live in one app

| Option | Good | Bad | Verdict |
|---|---|---|---|
| **(a) Gestures always on, buttons always shown** (brief today) | Discoverable; meets HIG; one layout to test; gestures as accelerators is what lasted (Mailbox, Procreate) | Doesn't reward experts with space; some people find it cluttered | **Keep as the default** |
| **(b) Setting hides buttons ("Clean mode")** | Experts get the space and the calm; the user controls it, so nothing moves unexpectedly; cheap (layout only) | Two layouts to test; a user can hide something they still need | **Add as an option** |
| **(c) Buttons fade as the app sees you use the gestures** | Smooth on paper | Things move without warning, and adaptive interfaces that move items often hurt (Findlater); hard to explain ("where did Add go?"); many states to test; bad for people who share a phone or come back after a break | **Reject.** Use a gentle *one-off suggestion* instead (below) |
| **(d) Separate modes** (gesture mode vs button mode with different behaviour) | Each can be pure | Two apps to design, document and test; gestures learned in one mode don't carry to the other; splits the user base | **Reject** |

**Recommendation: (a) by default, plus (b) as Clean mode, plus a light touch of (c).** **[I]**

- **Gestures work in both modes.** Clean mode only hides the bottom bar and the "+" bars. Nothing
  else changes, so whatever you learned still applies.
- **Teach gestures in the moment, not with a tutorial** (people skip tutorials **[E]**, WWDC21):
  - The undo toast reads "Undone · tip: two-finger tap".
  - Long-press menu items show their gesture next to the name (the marking-menu idea **[E]**).
  - The "+" bars animate a small pull when tapped, showing that you can pull the grid instead
    (Apple's "hint at the gesture" advice **[E]**).
- **A one-off suggestion:** after someone has used, say, 5 of the core gestures several times, offer
  once: "You're using gestures. Hide the buttons?" They can accept or ignore it. The layout never
  changes by itself. **[I]**
- **Accessibility can't be switched off:**
  - With VoiceOver, TalkBack or Switch Control on, the buttons show whatever the setting says.
  - Every command is also a screen-reader custom action.
  - Multi-finger gestures always have one-finger alternatives, as Procreate does **[E]**.
  - The long-press menu is reachable by every input method.
- **The way back from Clean mode is always visible:** the "⋯" in the trail bar opens the full menu,
  including Settings. **[I]**

---

## 7. What building both costs, and how to keep it cheap

**One command layer [I].** The Rust core already models every change as a command. On top of it, the
app keeps one list of *user actions*: Dive, Up, Undo, AddRow, DeleteRow, Move, Find, and so on. Each
action exists once and has:

- a name and an icon
- a check for when it's available ("can I delete here?")
- what it does (it calls the core command)
- its gesture, its button, its menu entry, its screen-reader action and its desktop shortcut, all as
  *bindings* to the same action

```
  tap/spread/pinch/swipe ──┐
  bottom-bar buttons ──────┤
  long-press menu ─────────┼──►  ACTION (one place) ──► Rust core command ──► undo stack
  screen-reader actions ───┤
  desktop keys / menus ────┘
```

What this gives you:
- A new action costs one entry plus its bindings.
- The menu, screen-reader actions and desktop menus come almost for free.
- Logs can record *which way* each action was triggered (button, gesture or menu). That is the main
  measure for the tests and the beta.

**The extra cost of gestures [I, rough]:**

| Item | Estimate |
|---|---|
| Gesture recognition and conflict tuning, in the prototype | 2–3 weeks |
| Clean mode layout and setting | A few days |
| In-the-moment hints | About 1 week |
| Screen-reader custom actions | About 1 week, and needed anyway |
| Ongoing: testing doubles on the dimensions that matter (2 layouts × 2 OSes × screen reader on/off) | Recurring |

Keep it cheap by:
- recording real touch traces once, and replaying them to check that gesture tuning hasn't broken
  anything (no new devices needed);
- shipping the extras off by default;
- dropping any gesture that fails the tests rather than polishing it.

---

## 8. Testing: comparing the two in the planned rounds

Use the brief's 11 tasks and pass marks unchanged, and add a **gesture arm**. **[I]**

**Set-up:**
- **Round 1:** the brief's variants as planned, plus one extra session per person: 10 minutes in
  Clean mode with **no coaching**. Note every gesture they find on their own.
- **Round 2: a dedicated gesture round, 8–10 people.**
  - Day 1: show a 60-second cheat card, then all 11 tasks in Clean mode. Half start in button mode
    and half in Clean mode, then they swap tasks of similar difficulty.
  - **Day 2 (24 h later, remote):** recall test ("go up a level", "undo twice", …), with no card.
    Repeat tasks 1–4 and 7 and time them.
- **Round 3:** fix what failed, and confirm with 5 people. Include 1–2 regular VoiceOver or TalkBack
  users and one person who mostly uses the phone one-handed.

**Extra measures:**

| Measure | How |
|---|---|
| Discoverability without coaching | Share of the 8 core gestures found in 10 minutes |
| Recall after a day | Correct gestures out of 8 when asked by name |
| Accidental triggers | Logged: a gesture command undone within 3 s, a system back during a row swipe, a dive or up nobody meant |
| Time once learned | Day-2 task times, Clean vs buttons |
| Mix-ups | Right movement, wrong meaning (Nacenta's main error type) |
| Preference | Which mode would you keep, and why |

**Decisions:**

- **Gestures become the default (Clean mode on from the start)** only if all of these hold. I expect
  it won't happen, and that's fine:
  - 4 of 5 people find dive, up and undo uncoached;
  - day-2 recall is at least 7 of 8 for most people;
  - accidental triggers are no higher than with buttons;
  - day-2 times are at least 20% faster.
- **Keep Clean mode as an option** if, after the cheat card:
  - day-2 recall is at least 6 of 8 for 4 of 5 people;
  - there is at most 1 accidental trigger per person per session, and none while editing (the
    brief's own bar);
  - no task pass mark falls below the button arm.
- **Drop a single gesture** if 3 or more people forget it by day 2, or if it causes more than one
  accidental trigger per session. Move it to the menu.
- **Drop Clean mode** if fewer than 5 of the 8 core gestures pass. Gestures then stay as accelerators
  in (a), which costs almost nothing because they share the action layer.
- **In the beta:** watch the share of actions triggered by gesture among people who have used a
  gesture at least once, how many people turn Clean mode on, and how many turn it back off. Many
  turning it off is a warning sign.

Five people per arm finds problems but can't prove speed differences. The day-2 timing is a sanity
check, not statistics. If the decision rests on speed, the beta logs settle it.

---

## 9. Risks

1. **Clashes with system edge swipes.** Clear's team couldn't fix this in a simple list. Wombat
   adds 2D scrolling and Android's back from both sides. *Mitigation:* row handle only, safe margins,
   log misfires.
2. **Gestures that look alike get confused** (pinch, two-finger tap, two-finger drag, spread to
   dive vs spread to insert). *Mitigation:* prototype tuning; extras off by default.
3. **iOS keeps three-finger gestures for itself**, and some Android makers add their own.
   *Mitigation:* a day-one check; redo and the pocket always in the menu.
4. **Accessibility drifts.** Clean mode gets designed and tested by sighted two-handed people.
   *Mitigation:* buttons return automatically with assistive tech; screen-reader users in round 3;
   every action exists once in the action layer, so none can be gesture-only.
5. **Two layouts double the testing and the support questions** ("I can't find Add"). *Mitigation:*
   one layout difference only; gestures identical in both modes; ⋯ always visible.
6. **Learning drops off after a break.** People forget gestures they don't use daily. *Mitigation:*
   the menu shows gesture names; Clean mode is opt-in.
7. **Precious engineering time.** Gesture polish can eat weeks that the depth budget needs.
   *Mitigation:* the decision rules above; extras ship only on evidence.
8. **"100% gestures" sets a promise the platforms advise against.** *Mitigation:* describe Clean
   mode as "fewer buttons, same app", not "no buttons".
9. **Weak evidence base.** Much of the product evidence is reviews and forums. *Mitigation:* the
   test rounds carry the weight, as in the brief.

---

**Change to the brief's principle 4 [I]:** "Visible first. Every gesture has a button, and every
button can be hidden once you know its gesture, except where you need it to find your way."
