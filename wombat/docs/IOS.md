# Running the app on an iPhone

For a person with a Mac and an iPhone. The app has an iOS project (`app/ios`, decision
A192); it was made on a Linux machine that cannot build for iOS, so **nobody has built
it on a Mac of their own, and nothing has been seen to run on an iPhone.** CI builds it
for a simulator on a Mac and runs one test there (below). What you find goes into
`milestones/B1.md` and `milestones/UI2.md`.

## What exists

- `app/ios`: the project as `flutter create --platforms=ios` of Flutter 3.47.9 writes it,
  with two changes by hand: the bundle identifier is `io.github.sblumenf.burrow`, as the
  application ID on Android (A164), and the name on the home screen is Wombat.
- `app/rust_builder/ios/wombat_bridge.podspec`: the build glue that `flutter_rust_bridge`
  2.13.0 put there (cargokit). During the Xcode build it compiles the bridge with cargo
  into a static library and puts it into a framework named `wombat_bridge`, which is where
  the Dart side loads it from. It uses rustup's `stable`, not the pinned version (A131).
- No `Podfile`. Flutter writes it from its own template at the first `flutter pub get` on
  a machine with Xcode, and the first build then writes `Podfile.lock` and changes
  `ios/Runner.xcodeproj/project.pbxproj` and the two files in `ios/Flutter`. **Commit all
  of those after the first build on a Mac**, so that later builds use the same pods. (CI
  makes them in every run and throws them away; that the files are exactly these is from
  the documentation, since the job does not print them.)
- The lowest iOS version is 15.0, which is what Flutter 3.47.9 itself needs; no package
  the app uses asks for more.
- The document is kept where it is on the other systems, by the same rule (A165): on iOS
  `Library/Application Support/documents/example.wombat` inside the app's own container
  (by the plugin's code; nobody has looked). That folder is private to the app: it is not
  in the Files app, it is in the phone's backups, and it goes when the app is deleted. No
  key in `Info.plist` is needed for it, and none was added. The app asks for no
  permission.

## What CI shows, and what it does not

The job "App for iOS (simulator build)" (`.github/workflows/wombat.yml`) runs on a Mac
(`macos-15`): it builds a debug app for a simulator, checks that the bridge's library is
in it, makes a simulator of the newest iPhone and iOS the runner has, and runs
`integration_test/app_test.dart` on it. Nothing is signed.

What it showed. The job's first run (run 52 of the workflow, 43310447110) was cancelled
after fifteen minutes without ever being given a Mac to run on; why is not known. The
next (run 53, 43347537203, commit `009486b`) was the first time the bridge was compiled
for iOS and loaded there, and every step passed; so did run 54 (43352269035, commit
`4af64c7`, job 131645914588), which is the one read for the figures here:

- The runner: macOS 15.8.2 on Apple's processor (image `macos-15-arm64` 20261104.0514),
  Xcode 16.4 with the simulator SDK 18.5, CocoaPods 1.17.0, Flutter 3.47.9, and rustup's
  `stable`, which was Rust 1.100.0 that day (A131).
- The build: `flutter build ios --simulator --debug` took 3 min 27 s, of which the Xcode
  build 155 s and `pod install` 2 s. The app is 186 MB. In it is
  `Frameworks/wombat_bridge.framework/wombat_bridge`, 41 MB, for both processors a
  simulator may run on (x86-64 and arm64); the identifier is
  `io.github.sblumenf.burrow`, the name Wombat and the lowest iOS version 15.0.
- The simulator: an iPhone 16 with iOS 18.5. The runner also had iOS 18.6 and 26.0 to
  26.2; the job's rule takes the newest that is not newer than the Xcode in use. Making
  and starting it took 3 min 43 s.
- The test: `app_test.dart` passed, its one test ("open, change and undo on the screen").
  The step took 3 min 5 s, of which 80 s were building the app again for that simulator.
- The whole job took a little over 15 minutes.
- Flutter warned in both builds that the plugin `wombat_bridge` "does not support Swift
  Package Manager" and that this "will become an error in a future version of Flutter".
  The plugin is the build glue of `flutter_rust_bridge`; nothing was done about it.

What a green job says: the iOS project builds, the bridge compiles for iOS and is linked
into the app, and the library loads and answers on a simulator. What it does not say:
anything of a real iPhone. A simulator runs on the Mac's processor and the Mac's files, so
its speed and memory say nothing. The test presses inside Flutter: the on-screen keyboard,
the back swipe, what iOS does to an app out of sight, and whether the document is still
there after the app is installed again are untried. Nothing is signed in CI, so signing
and installing on a phone are untried too. **Only the bridge check runs there**: the phone
screen (`grid_test.dart`) and the measurement have never run on iOS, on a simulator
either, so nobody has seen the app's real screen drawn by iOS.

## Once: set up the Mac

1. **Xcode**, from the App Store (15 at least; 16 is what Flutter 3.47.9 recommends).
   Start it once so that it installs its parts, then:

   ```sh
   sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
   sudo xcodebuild -runFirstLaunch
   ```

2. **CocoaPods** (`brew install cocoapods`, or `sudo gem install cocoapods`).
3. **Rust**, with rustup (<https://rustup.rs>). The build installs the iOS targets it
   needs by itself.
4. **Flutter 3.47.9** (<https://docs.flutter.dev/get-started/install>).
5. `flutter doctor -v` should show a tick at "Xcode". It also says what is missing.

## An Apple account

A simulator needs none. A real iPhone runs only what is signed:

- **A free Apple ID** is enough to put the app on your own iPhone. What Xcode signs with it
  runs for 7 days; after that, build and install again (the document stays, since the app
  is installed over itself). Apple allows a few such apps per phone and a small number of
  new identifiers a week.
- **A paid developer account** (99 US dollars a year) signs for a year, and is what
  TestFlight and the App Store need. Nothing here needs it yet.

## Each time: get the code

```sh
git fetch origin claude/affectionate-dijkstra-bx8xxw
git checkout claude/affectionate-dijkstra-bx8xxw
cd wombat/app
flutter pub get
```

(Use the branch you were given if it has moved on.)

## 1. On a simulator first

```sh
open -a Simulator
flutter devices
flutter run -d <device>
```

This needs no account and shows whether the build works on your Mac at all. The first
build compiles the Rust core twice (once for each processor a simulator may run on) and
takes some minutes.

## 2. On your iPhone

1. Connect the iPhone by cable, unlock it, and answer "Trust this computer".
2. On the iPhone switch on Settings › Privacy & Security › Developer Mode (iOS 16 and
   later; the phone restarts). The switch may only appear after Xcode has seen the phone.
3. `open ios/Runner.xcworkspace` (the workspace, not `Runner.xcodeproj`). In Xcode:
   Settings › Accounts, add your Apple ID. Then select the project "Runner" at the top of
   the list on the left, the target "Runner", the tab "Signing & Capabilities": tick
   "Automatically manage signing" and choose your name as the Team.
   - If Xcode says the bundle identifier is not available, another account has
     registered `io.github.sblumenf.burrow`. Say so before changing it: the identifier is
     a decision (A164).
4. Back in the terminal:

   ```sh
   flutter devices
   flutter run --release -d <device>
   ```

   `--release` on purpose: a debug build on an iPhone starts only from Xcode or
   `flutter run`, not from its icon, and asks for access to the local network. (Or press
   the Run button in Xcode, with the scheme's build configuration set to Release.)
5. The first time, the iPhone refuses to open the app: Settings › General › VPN & Device
   Management, tap your Apple ID under "Developer App", and trust it. Then open the app.

Then go through `PHONE_TEST.md` from "What to try" on: it is written for an Android
phone, and the screen is the same. Where it says the phone's back button, an iPhone has
only the swipe. On an iPhone in particular:

- There is no back button. The swipe from the left edge should go up one level in a
  nested grid; say what it does with the editor open and at the root.
- Swipe up to the home screen while typing, then swipe the app away in the app switcher
  and start it again: what was typed should be in the cell.
- The keyboard: what it covers, what its Return key says and does, and whether the bar
  above it (the suggestions) gets in the way.
- Dictation and the keyboard's own undo (shaking the phone, or a three-finger swipe).

## 3. Let the tests press the buttons

```sh
flutter test integration_test/app_test.dart -d <device>
flutter test integration_test/grid_test.dart -d <device>
```

One file per call. Each should end by saying that its tests passed. On a simulator this
is what CI does for the first file; the second has never run on iOS, and on a real iPhone
nobody has run either.

## 4. Measure

As in `ANDROID.md`, "Measure", with the iPhone as the device (a profile build needs a real
phone: a simulator takes debug builds only). Write the report into `milestones/B1.md` with
the model of the phone.

## If the build fails

- **"No Podfile" or pods out of date**: `flutter pub get`, then `cd ios && pod install`.
- **A cargo error inside the step "Build Rust library"**: the full output is in Xcode's
  report navigator, or in `flutter build ios -v`. The build runs `rustup run stable cargo
  build` for the bridge; if a newer stable fails where the pinned version works, say so in
  `B1.md` (A131). `xcrun` not found means Xcode's command-line tools are not selected
  (step 1 of the set-up).
- **"Signing for Runner requires a development team"**: step 3 above.
- **The app starts and stops with "Failed to load dynamic library
  wombat_bridge.framework/wombat_bridge"**: the Rust library was not put into the app.
  `ios/Podfile` must have `use_frameworks!` (Flutter's template has), and the package in
  `crates/wombat-bridge/Cargo.toml` must be named `wombat_bridge` (A131).

## What is unverified

Everything on this page except what the CI runs above name. The steps for a Mac of your
own and for a real iPhone are written from memory of Flutter's and Apple's documentation
for this version; no page was open while they were written, and nobody followed them. In
particular: the first `pod install` on a Mac of one's own, signing with a free account,
the 7-day limit in practice, the limits on apps and identifiers, where the menus are in
Xcode and in the iPhone's settings, where the document really is on a phone, and every
item under "On your iPhone".
