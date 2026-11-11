> Background research, not instructions. The interface specification is `../UI_SPEC.md`; where this file differs from it, the specification wins.

# Wombat on React Native: a second look at the interface toolkit

Date: 2026-11-11. Companion to `nested-grids-mobile.md` (the "Flutter report"), which this
reuses and does not repeat. Scope: would React Native (RN) serve Wombat better than Flutter (decision A121)?
Wombat is a phone-first app for nested grids of short text cells, with desktop apps later, and a Rust core
that reports changed cells after every command.

Labels: **[Evidence]** = backed by a cited primary source (registry, repository, official docs or blog);
**[Secondary]** = trade press or third-party blog; **[Speculation]** = my reasoning, untested.
All version numbers and dates were checked on 2026-11-10/11 against the npm registry, pub.dev, crates.io,
GitHub and official blogs.

---

## 0. TL;DR

- **Text input: RN is better in kind, but it has bugs of its own.** RN's `TextInput` *is* a
  `UITextField`/`UITextView` (iOS) or `EditText` (Android), so autocorrect bubbles, the loupe, dictation,
  Writing Tools, text replacement, the native edit menu and hardware-keyboard behaviour come from the OS.
  Flutter's fields are drawn by Flutter, and some features have been missing for years (autocorrect
  tooltips missing on iOS since 2017, flutter#12920, open). But RN's New Architecture glue has open,
  serious iOS regressions in 2026: **CJK IME composition broken on Fabric** (#56463, fix PR waiting
  since March 2026), **dictated text duplicated in controlled inputs** (#59406, RN 0.88), a dropped
  keystroke after the first character (#58904). "Native fields" does not mean "no text bugs". It means
  the bugs are in a thin layer that can be patched, or avoided by using uncontrolled inputs.
- **Desktop: Flutter clearly wins.** react-native-macos is at 0.83 (published 2026-09-30) while core RN
  is at 0.88 (2026-10-15). That is about 10 months behind, and it skipped 0.80 and 0.82. **Expo SDK 58
  dropped macOS** "until react-native-macos reaches 0.86". react-native-windows is at 0.85, about 6 months
  behind, and Fabric-only since 0.82. Linux has no official port: three young community ports, one
  pre-alpha. Key libraries (Reanimated, Gesture Handler, keyboard-controller, Skia) list no Windows
  support, and the Rust binding tool (uniffi-bindgen-react-native) supports neither desktop fork.
  Realistic RN desktop means **react-native-web inside Tauri or Electron**, on all three OSes. That is
  workable, but it is a second host.
- **Grid rendering: Flutter wins.** RN has no maintained 2D virtualized grid, Android's `ScrollView`
  scrolls one axis only, and there is no synchronous text-measurement API for content-sized columns.
  Flutter has `TableView` with pinned rows and columns, and `TextPainter`.
- **Rust: Flutter is more mature.** flutter_rust_bridge 2.13.0 supports all six platforms.
  uniffi-bindgen-react-native is still a pre-release (0.31.0-7, 2026-11-10), its docs say "should not
  yet be used in production", and it covers iOS, Android, web (WASM) and Node, but not
  react-native-macos or react-native-windows.
- **Verdict: keep Flutter (A121) provisionally, about 60/40, and let a 3–4 day bake-off on a real iPhone
  decide.** Wombat edits one cell at a time, so Flutter's weak spot is confined to one widget. That
  widget can be swapped for a native text view if needed, which the Flutter report already proposed.
  RN's weak spots (desktop, 2D grid, Rust binding on desktop) touch the whole app. **Switch to RN (phones)
  + react-native-web in Tauri (desktop)** if the bake-off shows Flutter's editor, including the
  native-view variant, is clearly worse to type in. Section 9 lists what would change the verdict.

---

## 1. Text input: how good RN's `TextInput` really is

### What RN gets for free [Evidence for the mapping, Speculation for user impact]

RN's `TextInput` renders a real `UITextField` (single-line) or `UITextView` (multiline) on iOS and an
`EditText` on Android. Whatever the OS does in its own text views, RN gets without extra code:
autocorrect with its underline and revert bubble, predictive and inline predictions, the selection
loupe, the space-bar trackpad, dictation, Scribble on iPad, Writing Tools, text replacement shortcuts,
the system edit menu (Look Up, Translate), spell-check, Live Text, autofill, and the hardware-keyboard
text shortcuts. These are the gaps the Flutter report flagged. On iOS, Flutter's matching issues are
real and old:

| Flutter issue (iOS text input) | Opened | 👍 | State |
|---|---|---|---|
| #57609 Let iOS keyboard dismissal synchronize with scroll | 2020-05 | 94 | open |
| #12920 Autocorrect tooltips don't appear on iOS | 2017-11 | 81 | open |
| #72625 Add the native keyboard actions bar (up/down/done) | 2020-12 | 28 | open |
| #160725 Laggy first keyboard open (physical iOS device) | 2024-12 | 21 | open |
| #179482 Semi-transparent keyboard on iOS 26 reveals widgets not drawn under it | 2025-12 | 17 | open, **P1** |

Source: GitHub search API on `flutter/flutter`, labels `a: text input` + `platform-ios`, sorted by 👍,
2026-11-10. **Open counts: 1,069 `a: text input` issues, 207 of them iOS** [Evidence].

### RN's own open text-input issues (so the comparison is fair) [Evidence]

From `facebook/react-native` issue search (2026-11-10): **89 open issues labelled `Component: TextInput`**,
**149 open issues mentioning TextInput**, and 97 open issues mentioning dictation, autocorrect, IME,
composition or keyboard. Out of 746 open issues in total.

**The counts are not comparable with Flutter's.** RN runs a stale bot: issues with no activity for 180
days are closed after 7 more days, and "Needs: Author Feedback" issues after 24 + 7 days
([stale-bot.yml](https://raw.githubusercontent.com/facebook/react-native/main/.github/workflows/stale-bot.yml)).
Flutter keeps issues open for years. Read RN's numbers as "recent, active problems" and Flutter's as
"the backlog". The content matters more than the counts. Notable open RN issues relevant to a cell editor:

| RN issue | Opened | What breaks | Relevance to Wombat |
|---|---|---|---|
| [#56463](https://github.com/facebook/react-native/issues/56463) CJK IME input "fundamentally broken" on iOS Fabric | 2026-04 | Missing composition underline, lost composition, `maxLength` cuts composition, controlled `value` interferes. Reported since 0.76, in both controlled and uncontrolled inputs. Community fix PR #56082 waiting since 2026-03-13. A related Meta commit landed 2026-11-02, but the issue is still open | **High** for Japanese, Chinese and Korean users. Must be tested on day 1 |
| [#59406](https://github.com/facebook/react-native/issues/59406) Controlled single-line input duplicates dictated text (RN 0.88, New Arch) | 2026-10 | Dictation inserts its text twice. Uncontrolled inputs are not affected | Use an uncontrolled editor (`defaultValue` + `onChangeText`) |
| #58904 TextInput drops a keystroke right after the first character in an empty field (iOS, Fabric) | 2026-10 | Lost keystroke | High if real; needs repro |
| #58807 TextInput interrupts Japanese composition with the Simeji keyboard | 2026-10 | IME | Same family as #56463 |
| #59386 Multiline input shifts its text while typing Korean on iOS 27 | 2026-10 | Layout jump | Medium |
| #59157 Multiline input loses its custom font after an autocorrect | 2026-10 | Style lost | Medium if custom fonts are used |
| #59354 Recycled `InputAccessoryView` left visible and blocks touches (0.88 regression) | 2026-10 | Toolbar above the keyboard | Use keyboard-controller instead of the core `InputAccessoryView` |
| #59305 `automaticallyAdjustKeyboardInsets` ignores changes while the keyboard is up | 2026-10 | Keyboard avoidance | Use keyboard-controller |
| #58918 NSRangeException on Undo after pasting past `maxLength` | 2026-10 | Crash | Avoid `maxLength` |
| [#59534](https://github.com/facebook/react-native/issues/59534) Android repaints the whole root on every caret blink | 2026-10 | Wasted GPU fill (no measured frame cost) | Low |
| #58990 `onSubmitEditing` fires twice on Android with a hardware keyboard (0.88 RC) | 2026-10 | Double commit | Medium for "Return = next cell" |

**Pattern [Speculation, from the list]:** RN's problems sit in its glue between JS state and the native
field: controlled-value syncing, attributed-string comparison in Fabric, recycling. They are not in the OS
field itself. Many can be avoided by design: an **uncontrolled** editor, no `maxLength`, and no
re-render of the editor while typing. CJK IME is the exception (#56463 covers uncontrolled inputs too).
Flutter's problems are missing platform behaviours, by design. They are harder to work around except
by embedding a native view.

### Keyboard handling, toolbar, avoidance [Evidence]

- **react-native-keyboard-controller 1.23.0** (2026-11-05, ~14.6M downloads per month) is the de-facto
  standard. It provides `KeyboardToolbar` (prev, next, done, fully customizable), `KeyboardStickyView`
  (any view glued to the keyboard: Wombat's structure toolbar), `KeyboardAwareScrollView`,
  `KeyboardAvoidingView`, and keyboard-tracking animations on Reanimated
  ([docs](https://kirillzyusko.github.io/react-native-keyboard-controller/docs/api/components/keyboard-toolbar)).
  Platforms: iOS and Android only ([React Native Directory](https://reactnative.directory)).
- Margelo (May 2026) calls RN's built-in `KeyboardAvoidingView` "structurally broken on every modern
  device" and recommends keyboard-controller
  ([Margelo](https://margelo.com/blog/deep-dive-in-keyboard-handling)) [Secondary].
- iOS interactive dismissal (`keyboardDismissMode="interactive"`) is the native `UIScrollView` behaviour,
  which addresses Flutter's #57609. keyboard-controller has an issue about iOS 26.x keyboard and layout
  animation glitches inside expo-router screens (#1215, unresolved in the snippet) [Secondary].
- iOS 26's translucent (Liquid Glass) keyboard wraps native apps too. Native views drawn behind the
  keyboard are expected there, so RN content extends behind it like any UIKit app's. I found no RN
  equivalent of Flutter's P1 #179482 [Speculation, absence of evidence].

### Rich text inside a cell

TreeSheets now stores **runs** inside a cell's text: bold, italic, fixed-width and colour ranges
(`src/text.h`, `runs`). If Wombat keeps that, RN's `TextInput` is weak at it. Nested `<Text>` inside
`TextInput` works only partly, and the community answer is a native editor:
[react-native-enriched](https://www.npmjs.com/package/react-native-enriched) 0.8.1, pre-1.0,
`UITextView`/`EditText` based. Notion's engineer on HN says RN `TextInput` "doesn't support rich text
features" well, and mobile rich editors (Notion, Docs, Quip) mostly use WebViews
([HN](https://news.ycombinator.com/item?id=34197613)) [Secondary]. Flutter's `TextField` can style
ranges through `TextEditingController.buildTextSpan` [Evidence: Flutter API]. **If cell-level styling
is enough (Wombat decision pending), this is a non-issue. If per-run styling while editing is
required, it favours Flutter.**

---

## 2. Architecture and performance

### New Architecture status [Evidence]

- RN 0.88.1 is latest (0.88.0 on 2026-10-15). The 0.88 blog says it is the third release in a row
  since 0.83 **without user-facing breaking changes**. It also brings W3C-style event dispatch, variable
  fonts and iOS SceneDelegate ([blog](https://reactnative.dev/blog/2026/10/19/react-native-0.88)).
- Expo SDK 55 removed the Legacy Architecture entirely ([changelog](https://expo.dev/changelog/sdk-55)).
  FlashList v2, Reanimated 4 and Gesture Handler 3 are **New-Architecture-only**. Fabric, JSI,
  TurboModules and bridgeless mode are the platform now, not an option.
- Cadence: core RN releases about every two months (0.84 Feb, 0.85 Apr, 0.86 Jun, 0.87 Aug, 0.88 Oct
  2026, from npm). Expo is moving towards adopting the non-breaking releases almost immediately
  ([SDK 57](https://expo.dev/changelog/sdk-57)). Expo noted 601 commits touching 1,552 files between
  0.85.0 and 0.86.0, even in a "non-breaking" step.

### Rendering a level of cells as native views

- A level shows about 50–150 visible cells, each a `View` + `Text`. That is well within normal RN list
  screens. The Flutter report's point holds: only one level plus previews is ever built, whatever the
  document's depth [Speculation, to measure].
- **2D virtualization does not exist off the shelf.** FlashList 2.3.3 (2026-10-01, JS-only, New Arch,
  masonry) and Legend List 3.7.2 (2026-11-07, pure TS, runs on iOS, Android, web, macOS and Windows) are
  1D lists. My search found no maintained RN 2D virtualized grid. Web grids (RevoGrid, Handsontable,
  react-datasheet-grid) do not run in RN [Evidence: READMEs, search].
- **Android `ScrollView` scrolls one axis only**, so a 2D level needs a horizontal `ScrollView` around a
  vertical list. That gives axis-locked scrolling, not diagonal. iOS `UIScrollView` can pan both ways,
  but RN's `ScrollView` component exposes one axis per instance [Evidence: RN docs]. Pinned header rows
  are easy. A pinned first column needs a scroll offset copied on the UI thread (Reanimated
  `useAnimatedScrollHandler` + transform) [Speculation: standard technique, some jank risk on Android].
- RN's experimental **`VirtualView`** (docs from 0.82 to 0.88, "don't use in production") hides and may
  unmount children that leave a `ScrollView`'s viewport. It could become a cheap way to cull cells in a
  2D layout ([docs](https://reactnative.dev/docs/virtualview)).
- **Column sizing.** TreeSheets sizes columns to their content. RN has no synchronous JS text-measure
  API that I could find, so the options are a two-pass layout (`onLayout`, risks a flash), Skia's
  `Paragraph` measure (iOS, Android and web only), or a small native module (Nitro) calling
  `NSAttributedString`/`StaticLayout`. Flutter's `TextPainter` does this in one call
  [Speculation for RN options, Evidence for Flutter].

**Verdict:** Flutter's `TableView` (pinned rows and columns, merged cells, diagonal or axis-locked
scrolling) plus `TextPainter` is a clear advantage for the grid itself. In RN, a level view with pinned
header row and column and content-sized columns is about a week of custom work [Speculation].

### Gestures and animation [Evidence]

- **react-native-gesture-handler 3.5.0** (2026-11-05; 3.x is New-Arch-only with a hook-based API, needs
  RN ≥ 0.82; 2.x supports up to RN 0.87) and **react-native-reanimated 4.7.1** (worklets on the UI
  thread; compatibility table: 4.7/4.8 need RN 0.86–0.88)
  ([compat](https://docs.swmansion.com/react-native-reanimated/docs/guides/compatibility/)). Pinch, pan,
  long-press and drag run on the UI thread with no JS round-trip. A gesture-driven, reversible dive
  (follow the pinch, cancel midway) is a natural fit: measure the cell rect on the UI thread, interpolate
  a transform. That is comparable to Flutter's custom `AnimationController` [Speculation on effort:
  similar].
- **Shared element transitions** in Reanimated 4 are still behind `ENABLE_SHARED_ELEMENT_TRANSITIONS`,
  "not yet production ready". 4.7 "brings the feature closer to a stable release"
  ([feature flags](https://docs.swmansion.com/react-native-reanimated/docs/guides/feature-flags/)). Flutter's
  `Hero` is stable. **For the dive, don't rely on SET. Animate within one screen instead** (the current
  level and the incoming level are both views in the same screen).
- **Native navigation.** react-native-screens gives a real `UINavigationController` and Android stack,
  so the iOS edge-swipe back is the platform's own interactive gesture. If each level is a native stack
  screen, "system back = up one level" is exactly native. But then the custom "grow from the cell"
  animation fights the native push animation. Choose one model [Speculation].

### Skia for previews [Evidence + Speculation]

`@shopify/react-native-skia` 2.14.0 (2026-10-01) gives a Flutter-like canvas with a `Paragraph` text API.
It could paint child-grid previews as pictures (the Flutter report's approach). But React Native
Directory lists it for iOS, Android, web and tvOS, **not macOS or Windows**. It also adds about 4–6 MB
per platform [Speculation, from typical Skia sizes]. **Prefer plain `View`/`Text` previews** (a few
lines of small text plus a shape glyph): portable to every RN host and accessible. Keep Skia for the
desktop-only zoomable canvas (concept 6), if ever.

---

## 3. Rust core integration

| Option | Status (2026-11) | Platforms | Notes |
|---|---|---|---|
| **uniffi-bindgen-react-native** (ubrn) | 0.31.0-7 (pre-release tag, 2026-11-10); first published 2024-12; ~1.5M npm downloads per month (likely pulled in transitively, e.g. by Matrix SDK bindings) | iOS and Android (JSI C++ + TurboModule), **web via a generated wasm-bindgen crate**, Node via `@ubjs/node` | Docs: "still in early development, and should not yet be used in production" ([site](https://jhugman.github.io/uniffi-bindgen-react-native)). Users listed in the README: Matrix SDK for RN, ChessTiles, Whip, russh bindings. Funded by Filament and Mozilla, led by James Hugman ([Mozilla Hacks, Dec 2024](https://hacks.mozilla.org/2024/12/introducing-uniffi-for-react-native-rust-powered-turbo-modules/)). **No react-native-windows support** (issue #186 open since 2024-12) and **no macOS issue at all**. Tracks uniffi 0.31 while uniffi is at 0.32.2 |
| Nitro Modules + a C ABI from Rust | react-native-nitro-modules 0.37.2, ~11M downloads per month | iOS, Android | Statically compiled JSI bindings. You would write a thin C++ layer over a `cbindgen` header by hand. Mature and fast, but manual |
| Hand-written JSI C++ module | — | Any host with JSI (Hermes) | Most control, most work. The same C++ could in principle target RNW and react-native-macos [Speculation] |
| Craby (Rust TurboModules) | `craby-modules` 0.1.0-rc.8 | iOS, Android | Too young |
| WASM in RN | `@callstack/polygen` 0.2.1 (AOT-compiles WASM to C), `react-native-webassembly` (2023) | — | Hermes has no WASM runtime. Not a serious route on phones |

Compare: **flutter_rust_bridge 2.13.0** (2026-08-23), a Flutter Favorite, all six platforms [Evidence:
pub.dev].

### How change reports should flow into React [Speculation, design]

1. The core runs in-process. A command is a **synchronous** JSI call (`core.apply(cmd)`) returning a
   small record: `{changedCells: Id[], changedGrids: Id[], structure: bool}`. This is decision A10.
   Long work (search, load) uses ubrn async calls and callbacks.
2. JS keeps a **normalized external store**: `Map<Id, {version, text, style, gridPreview}>`. It is
   filled lazily per level: ask the core for "level L with previews", not the whole document.
3. Each cell component subscribes by ID with `useSyncExternalStore` (or a selector store such as
   Zustand or Legend State). After a command, invalidate only the changed IDs. Only visible cells that
   changed re-render. Previews are keyed by grid ID + version, as in the Flutter plan.
4. The editor is **uncontrolled**. Keystrokes go to the core debounced, or on commit. They are not
   echoed back into `value`. This avoids #59406 and the classic controlled-input cursor jumps.
5. Keep the JS↔Rust boundary coarse. Per-cell-per-frame calls are the thing to avoid in both stacks.

Bonus: ubrn's WASM output means the **same Rust API annotations** could serve a react-native-web
desktop build in a webview (Section 4). That would revive the WebAssembly target that A121 dropped from A2.

---

## 4. Desktop: macOS, Windows, Linux

### Release lag (from npm publish dates) [Evidence]

| Core RN minor | Core date | react-native-macos | react-native-windows |
|---|---|---|---|
| 0.80 | 2025-06-12 | skipped | 2025-10-28 |
| 0.81 | 2025-08-12 | 2026-01-17 | 2025-12-20 |
| 0.82 | 2025-10-08 | **skipped** | 2026-03-12 (Paper removed, Fabric-only) |
| 0.83 | 2025-12-10 | **2026-09-30** (latest) | 2026-05-26 |
| 0.84 | 2026-02-11 | — | 2026-06-18 |
| 0.85 | 2026-04-07 | — | **2026-10-19** (latest) |
| 0.88 | 2026-10-15 | — | — |

- **macOS lags about 10 months and 5 minors**, and skips versions. Its Fabric port is unfinished: open
  issues in Oct 2026 include "`<Modal>` unimplemented" and crashes on Fabric (#3155, #3154). The 0.81
  backports still validate on Paper (#3135) [Evidence: GitHub]. **Expo SDK 58 (2026-10-15): "SDK 58 does
  not support macOS yet. Support will return when react-native-macos is updated to at least React
  Native 0.86."** ([changelog](https://expo.dev/changelog/sdk-58)).
- **Windows lags about 6 months**, ships regularly, and is Fabric-only since 0.82 (April 2026). It
  requires Visual Studio 2026 since 0.84
  ([Microsoft devblog](https://devblogs.microsoft.com/react-native/2026/)).
- **Library consequence:** Reanimated 4.7 needs RN 0.86+, so an app on mobile 0.88 and macOS 0.83
  cannot share the same Reanimated, Gesture Handler 3.x or keyboard-controller versions. React Native
  Directory lists **no Windows support** for Reanimated, Gesture Handler, Skia, keyboard-controller or
  FlashList. Legend List and react-native-svg cover all hosts [Evidence: reactnative.directory API].
- **Who ships on these:** Microsoft Office (comments, dialogs and other parts of Word, Excel and
  PowerPoint on Windows and macOS), the Xbox app on Windows, and Microsoft Store on Xbox
  ([showcase](https://microsoft.github.io/react-native-windows/resources-showcase)). The showcased
  Messenger desktop app was **shut down in December 2025**, with users sent to the web
  ([CyberInsider](https://cyberinsider.com/meta-to-shut-down-messenger-desktop-apps-on-december-15-redirects-users-to-web/))
  [Secondary]. Usage is mostly Microsoft embedding RN *inside* large native apps, not whole
  third-party RN desktop apps.
- **Menus, windows, keyboard:** react-native-macos and RNW expose key events and native modules, but app
  menus and multi-window need native code per OS [Speculation from docs; not verified in depth].

### Linux [Evidence]

No official port. Community efforts, all young:

| Project | Approach | Status |
|---|---|---|
| [react-native-gtkx](https://github.com/itsmepetrov/react-native-gtkx) | RN API rendered as GTK4/Adwaita widgets, `run-linux` | 0.4.0 alpha, created 2026-07, 70 downloads per month |
| [react-native-gtk4](https://github.com/curiosity26/react-native-gtk4) | Fabric C++ host over GTK4, Pango text, Orca accessibility | Versions tracking 0.83–0.87 stable, created 2026-10, ~5k downloads per month |
| [react-native-linux](https://github.com/react-native-linux/react-native-linux) | Fabric + Skia/Vulkan on Wayland | "pre-alpha", `TextInput` "in verification", no accessibility yet |

None is something to bet a product on in 2026.

### The realistic RN desktop: react-native-web in Tauri or Electron [Speculation, design]

React Native Web 0.21.4 (2026-10-08) renders RN components to the DOM, and many apps ship iOS, Android
and web from one codebase (e.g. Bluesky). A **Tauri 2.12** (2026-11) shell would give macOS, Windows and
**Linux** from one desktop build, with the Rust core either native in the Tauri process or as WASM via
ubrn's generated wasm-bindgen crate. Browser text inputs have mature IME and spell-check on desktop.
Costs: webview differences (WebKit on macOS and Linux, WebView2 on Windows), a less native look, a
second host with its own gesture, scroll and keyboard code paths, and libraries that are mobile-only
(keyboard-controller, Gesture Handler 3 on web is partial).

**Is "one codebase" realistic?** With RN: **one TypeScript UI codebase, two to three hosts**: the RN
mobile app, a react-native-web desktop app (or react-native-windows plus react-native-macos at
different RN versions), and platform-specific files for keyboard, menus and gestures. With Flutter:
one project, five platform runners, the same widgets everywhere. On desktop, Flutter's "one codebase"
claim is materially more true.

---

## 5. Tooling and shipping

- **Expo 58.0.23** (SDK 58 released 2026-10-15, RN 0.88, built for iOS 27). EAS Build (cloud iOS builds
  without a Mac), EAS Submit (store upload), EAS Update (over-the-air JS updates, within store rules),
  Continuous Native Generation with config plugins, and Expo Modules 2.0 (beta) for plain Swift and
  Kotlin modules ([changelog](https://expo.dev/changelog/sdk-58)). That is a real productivity
  advantage over Flutter's tooling for store shipping and OTA. Flutter has no sanctioned OTA path
  (Shorebird is third-party) [Evidence for Expo, Secondary for Shorebird].
- **Rust under Expo:** ship prebuilt `.xcframework` and `.so`/AAR artifacts in a local Expo module, or
  install rustup in an EAS pre-install hook. Doable, but not documented by ubrn ("Expo: not mentioned")
  [Speculation].
- **Expo and desktop:** no Windows support, and macOS dropped in SDK 58 (see above).
- **Upgrade pain:** Shopify (5 years, Jan 2025): RN "version upgrades require significant work and
  often code restructuring, so a rotating group handles them"; more third-party libraries mean
  supply-chain risk; debugging was flaky ([Shopify](https://shopify.engineering/five-years-of-react-native-at-shopify)).
  The streak of non-breaking releases since 0.83 is a real improvement. Dependency churn remains: Gesture
  Handler 3 (new API, 2026), Reanimated 4 + worklets, FlashList v2 all landed in about the last 18 months.
- **AI coding assistants:** TypeScript became GitHub's most-used language in August 2025, and GitHub
  links this partly to typed languages working better with AI agents
  ([Octoverse 2025](https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1/))
  [Evidence]. React has a huge training corpus. **But** RN-specific APIs churn (Gesture Handler 2 vs 3,
  Reanimated 2/3/4, old vs new architecture), so assistants often produce outdated code. Dart and
  Flutter have a smaller corpus but a stable API and a strict analyzer [Speculation]. Net: a slight edge
  to RN/TypeScript for "assistant writes code that compiles and looks idiomatic", but none for
  "assistant knows the right 2026 library versions". Expo now ships an `@expo/agent-cli` for agent
  workflows [Evidence: SDK 58 changelog].

---

## 6. Accessibility

- **RN uses native views, so VoiceOver and TalkBack see real platform elements**, and text fields are
  the platform's fully accessible ones. That is a structural advantage for the editor [Evidence: RN
  docs].
- **But RN's role vocabulary has no `table`, `row`, `cell` or `tree` roles.** The `role` list has `grid`
  ("adds the in/out of grid announcements to Android's GridView"), `list`, `listitem`, `heading`,
  `button`, `toolbar`, `tab` and so on
  ([accessibility.md](https://raw.githubusercontent.com/facebook/react-native-website/main/docs/accessibility.md))
  [Evidence]. Flutter's `SemanticsRole` has `table`, `row`, `cell` and `columnHeader`, also without a tree
  role. Its platform mapping is undocumented (Flutter report).
- Both support **custom actions** (RN: `accessibilityActions` + `onAccessibilityAction`; iOS rotor
  actions such as "Open grid" and "Up one level") and `accessibilityState` (expanded, selected). RN's
  `experimental_accessibilityOrder` lets reading order follow the grid row by row. Open RN issues include
  VoiceOver announcing row elements in the wrong order (#55139) and focus order breaking on async updates
  (#58145) [Evidence].
- **Design (either toolkit):** announce a nested cell as "Project plan, grid 3 by 4, row 2 of 5, column 1"
  with the actions "Open grid", "Peek" and "Up one level". List mode (concept 2) doubles as the
  screen-reader-friendly view. For full iOS data-table semantics in RN, a tiny native module could set
  `accessibilityContainerType = .dataTable` [Speculation].

---

## 7. The UX concepts in React Native

The earlier report's ranking and reasoning hold unchanged. The toolkit choice doesn't alter what users
need. I agree with its core rules, with one challenge: its concept 3 relies on long-press for peek, and
on iOS that gesture belongs to the **native context menu with preview**, which RN can use directly. This
is a genuine RN advantage (below).

### Concept 1 (Level view with child previews)

- **Build:** one screen. The current level is a horizontal `ScrollView` wrapping a FlashList or Legend
  List of rows (or a plain absolute layout for small levels), with a pinned header row and first column
  via Reanimated scroll sync. Cells are `Pressable` + `Text`. Previews are 2–4 lines of small `Text` plus
  a shape badge ("3×4"), drawn from a core "preview" record. The chevron dive target is a 44 pt
  `Pressable`. The location bar sits at the bottom. The dive animation measures the cell (`measure` on
  the UI thread), mounts the next level under a transform, and animates scale and translate with
  Reanimated. Pinch-to-dive uses a Gesture Handler `Pinch` gesture driving the same shared value.
  Android back and the iOS edge swipe use `BackHandler` plus a custom edge-pan, or native-stack screens
  per level.
- **Easier than Flutter:** native back semantics if you use native-stack screens; native text in cells
  (Dynamic Type, system fonts, VoiceOver).
- **Harder:** no `TableView`, so 2D scrolling, pinned headers and content-sized columns are custom; no
  `CustomPainter`-style cheap previews unless you accept Skia (not on desktop); shared element
  transitions are experimental.

### Concept 2 (Grid/List toggle per level)

- **Build:** a native segmented control (`@react-native-segmented-control/segmented-control`) in the level
  header. List mode is a FlashList or Legend List of cards. Remember the mode per grid per device in
  local storage (MMKV), as A8 requires.
- **Easier than Flutter:** list virtualization is RN's best-trodden path (FlashList v2 handles dynamic
  heights without estimates). Native segmented control.
- **Harder:** keeping scroll position across the mode switch (map the top visible cell ID to the other
  mode) is custom in both.

### Concept 3 (Peek)

- **Build, iOS-native variant (new, RN-enabled):** long-press a nested cell to get **`UIContextMenuInteraction`
  with a live preview** of the child grid, plus menu items (Open, Move to…, Copy link, Fold), the way
  Mail and Files peek. Tapping the preview commits the dive, the platform's own "pop". Available through
  libraries such as zeego, react-native-ios-context-menu or Expo UI's context menu [Speculation on exact
  library fit; the UIKit API is Evidence]. The **bottom-sheet variant** uses native sheets with detents
  (react-native-screens `formSheet` or `@gorhom/bottom-sheet`). Dragging to full height dives.
- **Easier than Flutter:** the preview and menu are the real iOS component users already know.
  Flutter's `CupertinoContextMenu` is an imitation. This turns long-press, which NN/g calls hidden, into
  a platform convention with a familiar look.
- **Harder:** native context-menu previews are limited in interactivity (read-only snapshot), so
  "edit in peek" needs the sheet variant.

### Concept 4 (Move to…, pocket, keyboard toolbar)

- **Build:** a multi-select bottom toolbar (plain views). "Move to…" opens a native sheet that reuses the
  level view in picker mode. The pocket is a small persistent bar above the tab area. The **structure
  toolbar above the keyboard is `KeyboardStickyView`** from keyboard-controller (insert row/column, wrap
  in grid, unwrap, undo, redo, link, tag). Single-cell editor: an uncontrolled multiline `TextInput`
  docked above the keyboard. Return commits and moves down (`blurOnSubmit={false}`, `onSubmitEditing`).
- **Easier than Flutter:** the editor is the native text view, so autocorrect, dictation and the loupe
  are exactly right, and keyboard tracking is frame-synced (keyboard-controller). This is the main
  reason to consider RN.
- **Harder:** #58990 (double submit with a hardware keyboard on Android) and the CJK IME bug need
  workarounds. Hardware-keyboard shortcuts (Tab between cells, ⌘-arrows on iPad) need a key-command
  library or a native module.

### New concepts RN makes possible or cheaper

1. **Native context-menu peek** (above).
2. **Native toolbars and menus per platform:** iOS 26 Liquid Glass toolbars and sheets come for free when
   using native components (Expo Router's `NativeTabs` and `Stack` use the system bars in SDK 58). The
   app looks current without redesign [Evidence: SDK 58 changelog].
3. **App Intents, widgets, Spotlight** via Expo modules (`expo-app-intents` alpha, `expo-widgets`): "Add
   to my Inbox grid" from Siri or Shortcuts, a home-screen widget showing a pinned grid. Flutter can do
   these through platform code too, but Expo packages make them cheap [Evidence: SDK 58 changelog;
   effort is Speculation].

---

## 8. Real-world evidence

| App / company | Stack | What it says for Wombat |
|---|---|---|
| **Shopify** (all mobile apps on RN since ~2020) | RN | Sub-500 ms P75 screen loads, >99.9% crash-free; upgrades costly; more third-party libraries ([Shopify, Jan 2025](https://shopify.engineering/five-years-of-react-native-at-shopify)) |
| **Microsoft** Office, Outlook, Teams, Xbox | RN on mobile, RN embedded in parts of Office desktop | Desktop RN works when a large company maintains the platform and embeds RN in a native app ([showcase](https://microsoft.github.io/react-native-windows/resources-showcase), [DevClass 2024](https://devclass.com/2024/04/11/react-native-and-why-microsoft-uses-it-for-its-own-cross-platform-development/)) |
| **Notion** | Moved *away* from an RN wrapper around a webview to a native shell plus a web editor (per a Notion engineer, 2022/23) | Rich editors on mobile are mostly webviews; RN didn't solve editor cost for them ([HN](https://news.ycombinator.com/item?id=34197613)) [Secondary] |
| **Standard Notes** | Published "React Native is not the future" (Dec 2022) when consolidating on web tech | The webview-editor-in-RN pattern adds layers without benefit ([HN thread](https://news.ycombinator.com/item?id=34197613)) [Secondary] |
| **Airbnb** | Left RN in 2018 ("Sunsetting React Native") | Old architecture; most cited reasons (bridge, three platforms of expertise) are partly addressed by the New Architecture [Secondary, dated] |
| **Messenger desktop** (RN for Windows/macOS showcase) | Shut down Dec 2025, moved to the web | A showcase RN desktop app is gone |
| **Bluesky** | RN + Expo, iOS, Android and web from one codebase | Shows RN + react-native-web can be one TypeScript codebase across mobile and web [Evidence: open-source repo] |
| **Outliners / grids on RN** | None found that edit nested structures with native `TextInput`. Workflowy's mobile stack is undocumented; Logseq and Obsidian use Capacitor webviews (Flutter report) | No direct precedent either way. Note the *webview* outliners are the ones users call laggy (Logseq) |

The prior art supports one real positive: for **plain-text, one-field-at-a-time editing** (Wombat's
case), RN's native fields are as good as typing gets on a phone. The negative evidence (Notion,
Standard Notes) is about RN wrapping *web editors*, which Wombat wouldn't do.

---

## 9. Head to head for Wombat

| Criterion | Flutter (3.47.x, flutter_rust_bridge 2.13) | React Native (0.88, Expo 58) | Edge |
|---|---|---|---|
| **Typing feel on iPhone** (autocorrect, dictation, loupe, edit menu, keyboard sync) | Self-drawn. Autocorrect bubble missing (#12920, 2017), no native accessory bar, keyboard/scroll dismissal not synced, first-open lag, iOS 26 P1. Mitigation: one native `UITextView` platform view for the single editor | Native fields. Keyboard-controller is excellent. Fabric glue bugs: CJK IME (#56463), dictation duplication in controlled inputs (#59406), keystroke drop (#58904). Mostly avoidable except CJK | **RN** (clear for Latin scripts; unclear for CJK until tested) |
| Rich text runs inside a cell | `buildTextSpan` styling in the field | Weak; native editor libraries pre-1.0 | Flutter (only if runs are needed) |
| Grid rendering (2D scroll, pinned headers, content-sized columns, previews) | `TableView` + `TextPainter` + `CustomPainter` | All custom; Android single-axis `ScrollView`; no sync text measure | **Flutter** |
| Gestures and animation (pinch dive, reversible transitions) | Mature, custom controllers, stable `Hero` | Gesture Handler 3 + Reanimated 4 on the UI thread, excellent; shared element transitions experimental | Tie |
| Native look (iOS 26 Liquid Glass, context menus, sheets) | Imitated widgets; must chase each iOS redesign | Real platform components | **RN** |
| Desktop macOS / Windows | Stable, same code | macOS fork about 10 months behind, Fabric gaps, dropped by Expo 58; Windows about 6 months behind; key libraries missing on Windows | **Flutter** (strong) |
| Linux | Stable (Canonical-maintained embedder) | None official; react-native-web in Tauri or Electron is the practical answer | **Flutter** (strong) |
| Rust integration | frb 2.13, all six platforms, mature | ubrn pre-release, "not for production", mobile + WASM only, no desktop forks | **Flutter** |
| Accessibility | Semantics with table/row/cell roles; custom actions; editor accessibility is Flutter's own | Real native views and fields; no table/cell roles; custom actions | Tie (RN better for the editor, Flutter better for grid roles) |
| Tooling and shipping | Good CLI; no sanctioned OTA | Expo EAS Build/Submit/Update, config plugins; cloud iOS builds | **RN** |
| Maintenance burden | One framework, few dependencies, quarterly releases | Release every ~2 months (now non-breaking), many third-party native libraries, desktop forks at other versions | **Flutter** |
| AI-assistant friendliness | Smaller corpus, stable API | TypeScript is #1 on GitHub; API churn trips assistants | Slight RN |

### Verdict

**Keep Flutter as the working choice (about 60/40), conditional on a short iPhone bake-off focused on
typing.** Reasons:

1. Wombat's goals include **desktop on three OSes, ideally Linux**. Flutter covers that from one
   project today. RN covers it only through a second host (react-native-web + Tauri) or through forks
   that lag core by 6–10 months and lack key libraries. Phone-first does not make desktop free.
2. The **grid itself** is Wombat's core screen, and Flutter has the right primitives (`TableView`,
   `TextPainter`, painters). In RN these are custom work, and Android's one-axis scrolling is a real
   constraint.
3. The **Rust bridge** is production-grade in Flutter and pre-release in RN, with no desktop forks.
4. Flutter's biggest weakness, iOS typing, is **contained**: Wombat uses one editor at a time (Flutter
   report, "editing one cell at a time"). If Flutter's field fails on device, a single native
   `UITextView` platform view is a bounded fix. RN's weaknesses are not contained in one widget.

But RN's advantage on the thing users do most, typing into cells, is real, and it is backed by
Flutter's own long-open issues. **This is why the bake-off matters more than any table.**

### What would change the verdict

- **Towards RN:** on a real iPhone, testers clearly prefer RN's typing (Section 10 criteria), *and* the
  Flutter native-view editor variant is janky (keyboard and toolbar desync, focus bugs, or a
  platform-view performance hit while scrolling). Or the team decides desktop can be **web-tech
  (Tauri)** and Linux is low priority. Or ubrn reaches 1.0 with an explicit production-ready statement.
- **Towards Flutter (firmly):** RN's CJK IME bug (#56463) reproduces in an uncontrolled input on RN 0.88
  and isn't fixed by 0.89. Or Wombat keeps rich-text runs inside cells. Or the RN 2D level view can't hit
  zero dropped frames on a mid-range Android.
- **Watch items:** react-native-macos catching up to 0.86+ (Expo restoring macOS); RNW support in ubrn
  (#186); Flutter fixing #12920, #57609 or #179482; Reanimated shared element transitions reaching
  stable.

---

## 10. Prototype plan (RN mirror of the Flutter plan) and the bake-off

### RN prototype (mirrors Flutter report §7; same tasks, measures and criteria)

Throwaway Expo app (SDK 58, dev client), TypeScript in-memory model: cells with ID, text and an optional
child grid. Load the same converted TreeSheets examples plus the synthetic deep/wide stress document
(depth 8, a 200×10 level).

- **Day 1:** Level view (horizontal `ScrollView` + Legend List rows), location bar, up/back, tap-to-dive
  with a Reanimated transform. Single-cell uncontrolled `TextInput` editor with `KeyboardStickyView`
  toolbar. Run on a real iPhone and one Android; check typing.
- **Day 2:** Text previews, pinned header row and first column (Reanimated scroll sync), search-jump to a
  deep target, per-level scroll memory (MMKV).
- **Day 3:** List-mode toggle (segmented control + FlashList), Peek (native context menu with preview,
  plus a sheet variant), pocket + Move to… sheet, undo.
- **Day 4:** Gesture-driven dive (pinch), variant A (Grid only) and variant B (Grid + List + Peek) behind
  a switch.
- **Day 5:** First usability round.

**Tasks 1–8, measures (success, time, errors, lostness, SEQ, preference), participants (5 per round,
2–3 rounds) and success criteria: identical to the Flutter report.** Additional RN-specific technical
checks:

- Typing battery (below) passes, including CJK and dictation.
- Zero dropped frames during dive and scroll on a 2–3-year-old iPhone and a mid-range Android
  (Perf Monitor / Android GPU profiling, release build).
- JS-thread headroom: a command round-trip into a stub Rust core (ubrn) plus re-render of changed cells
  in under 8 ms for a 150-cell level.

**RN kill or pivot signals:** the 2D level view janks on Android, or the pinned column visibly lags; the
CJK IME breaks in an uncontrolled field on 0.88; ubrn can't build the stub core for both phones within a
day; keyboard-controller toolbar glitches on iOS 26/27 that can't be worked around.

### The bake-off: smallest test that decides, on a real iPhone, in 3–4 days

**Build the same tiny app twice** (Flutter and RN), stripped to the parts where the two toolkits differ:

1. One level of about 60 cells (10×6) with text previews in 8 nested cells, a pinned header row and
   first column, and diagonal or axis-locked scrolling.
2. Tap to select, chevron to dive (animated), Up at the bottom, system back = up.
3. The single-cell editor docked above the keyboard with a 6-button structure toolbar; Return = commit
   and move down.
4. A stub Rust crate exposing `apply(cmd) -> changeset` and `level(id) -> cells`, bound with
   flutter_rust_bridge and with ubrn. Every edit goes through it.
5. **Flutter gets two editor variants:** the stock `TextField`, and a native `UITextView` platform view.
   Without the second, the comparison isn't fair to the plan.

**Day 1–2:** build both (with an AI assistant, which also tests "assistant friendliness"). **Day 3:**
technical battery on one iPhone (2–3 years old, current iOS) and one mid-range Android. **Day 4:** 5
users, blind to toolkit, counterbalanced: edit 10 cells by typing a given sentence each, including one
dictated and one with a deliberate typo left for autocorrect. Then a forced choice: "which felt more like
typing on your phone?" and why.

**Technical battery (pass/fail per item, both stacks):**

| # | Check |
|---|---|
| T1 | English typing with autocorrect: underline, bubble, revert-tap |
| T2 | Dictation into a cell: no duplicated or lost text |
| T3 | Japanese (Kana/Romaji), Chinese Pinyin and Korean: composition underline, candidates, commit; no lost characters |
| T4 | Loupe and cursor drag, space-bar trackpad, double-tap and select-all, copy/paste, edit menu (Look Up/Translate) |
| T5 | Emoji and ZWJ sequences: cursor never splits a grapheme (decision A11) |
| T6 | Keyboard open latency (first and later), toolbar glued to the keyboard during interactive dismissal, no content hidden behind the iOS 26/27 translucent keyboard |
| T7 | Hardware keyboard (iPad or Bluetooth): Return, Tab, arrows, ⌘Z |
| T8 | VoiceOver: read a level, open a nested grid via custom action, edit a cell |
| T9 | Frame timing: scroll the level and dive 20 times; count dropped frames |
| T10 | Cold start to first level, and release binary size |
| T11 | Half-day desktop spike: Flutter macOS + Linux build of the same app vs RN-web in Tauri on macOS + Linux. Does it run, does typing work? |

**Decision rule:**

- If **4 or 5 of 5 users prefer RN's typing** *and* RN passes T3, T6 and T9, *and* the Flutter
  native-view variant fails T6 or T9 → switch to RN for phones, plan react-native-web + Tauri for desktop,
  and record a new decision that replaces A121 (and revisits A2's WebAssembly target).
- If preference is split (3/2 or worse for RN), or RN fails T3 or T9 → keep Flutter. Adopt the
  native-view editor if it beat the stock `TextField` on T1/T2/T6.
- If both fail T3 → CJK becomes a known limitation to fix upstream, and the decision rests on
  desktop and grid (Flutter).

---

## 11. Biggest risks (RN path)

1. **Desktop is a second product.** Forks lag 6–10 months, and Expo dropped macOS. A Tauri +
   react-native-web host is the likely real answer, and it has its own input and gesture paths.
2. **Rust binding maturity:** ubrn is pre-release, has no desktop-fork support, and its pre-releases
   track uniffi with a lag.
3. **Fabric text-input regressions** (CJK, dictation), found by the community and patched slowly (the
   CJK fix PR waited 8 months).
4. **2D grid is custom work,** especially on Android (one-axis `ScrollView`, pinned-column sync) and for
   content-sized columns.
5. **Dependency churn:** a release every 2 months, plus Reanimated/worklets/Gesture Handler majors in
   2025–26. Each library pins RN version ranges.
6. **Evidence base is thin:** no shipping RN app edits nested grids. As in the Flutter report, the user
   tests are the real evidence.

---

## Sources (primary unless marked)

- npm registry (versions, dates, downloads), queried 2026-11-10: react-native, react-native-macos,
  react-native-windows, expo, react-native-keyboard-controller, @shopify/flash-list, @legendapp/list,
  react-native-reanimated, react-native-gesture-handler, @shopify/react-native-skia,
  uniffi-bindgen-react-native, react-native-nitro-modules, react-native-web, react-native-gtkx,
  @curiosity26/react-native-gtk4, electron. pub.dev: flutter_rust_bridge, two_dimensional_scrollables.
  crates.io: tauri, uniffi. Flutter release JSON (3.47.9, 2026-10-29).
- RN 0.88 blog: https://reactnative.dev/blog/2026/10/19/react-native-0.88
- RN issues: https://github.com/facebook/react-native/issues (searches above), #56463, #59406, #59534;
  stale bot: https://raw.githubusercontent.com/facebook/react-native/main/.github/workflows/stale-bot.yml
- RN accessibility docs: https://raw.githubusercontent.com/facebook/react-native-website/main/docs/accessibility.md
- RN VirtualView: https://reactnative.dev/docs/virtualview
- Flutter issues via GitHub search API: flutter/flutter #12920, #57609, #72625, #160725, #179482
- Expo changelogs: https://expo.dev/changelog/sdk-58, https://expo.dev/changelog/sdk-57, https://expo.dev/changelog/sdk-55
- Reanimated compatibility and feature flags: https://docs.swmansion.com/react-native-reanimated/docs/guides/compatibility/,
  https://docs.swmansion.com/react-native-reanimated/docs/guides/feature-flags/
- Gesture Handler 3: https://swmansion.com/changelog/react-native-gesture-handler/ (Secondary aggregation of releases)
- keyboard-controller docs: https://kirillzyusko.github.io/react-native-keyboard-controller/docs/api/components/keyboard-toolbar
- Margelo keyboard deep dive (Secondary): https://margelo.com/blog/deep-dive-in-keyboard-handling
- FlashList README: https://github.com/Shopify/flash-list; Legend List README: https://github.com/LegendApp/legend-list
- React Native Directory API: https://reactnative.directory
- uniffi-bindgen-react-native: https://github.com/jhugman/uniffi-bindgen-react-native, https://jhugman.github.io/uniffi-bindgen-react-native,
  https://hacks.mozilla.org/2024/12/introducing-uniffi-for-react-native-rust-powered-turbo-modules/, issues #186, #376
- react-native-macos releases and issues: https://github.com/microsoft/react-native-macos/releases, #3135, #3154, #3155
- RNW blog: https://devblogs.microsoft.com/react-native/2026/; showcase: https://microsoft.github.io/react-native-windows/resources-showcase
- Linux ports: https://github.com/itsmepetrov/react-native-gtkx, https://github.com/curiosity26/react-native-gtk4,
  https://github.com/react-native-linux/react-native-linux
- Shopify: https://shopify.engineering/five-years-of-react-native-at-shopify
- Notion / Standard Notes discussion (Secondary): https://news.ycombinator.com/item?id=34197613
- Messenger desktop shutdown (Secondary): https://cyberinsider.com/meta-to-shut-down-messenger-desktop-apps-on-december-15-redirects-users-to-web/
- Microsoft RN usage (Secondary): https://devclass.com/2024/04/11/react-native-and-why-microsoft-uses-it-for-its-own-cross-platform-development/
- Octoverse 2025: https://github.blog/news-insights/octoverse/octoverse-a-new-developer-joins-github-every-second-as-ai-leads-typescript-to-1/
- TreeSheets rich-text runs: /home/user/treesheets/src/text.h (`runs`)
