# Running the app on Android

For a person at a machine with an Android phone or emulator. The app builds for Android (it
was built on a Linux cloud machine that could not run it); **nothing has been seen to run
on a real Android phone yet** (the integration tests run on an emulator in CI). What you
find goes into `milestones/B1.md` and `milestones/UI2.md`.

## Once: set up the machine

1. **Rust**, with rustup (<https://rustup.rs>). The build installs the Android targets it
   needs by itself.
2. **Flutter 3.47.9** (<https://docs.flutter.dev/get-started/install>).
3. **The Android SDK and NDK.** Android Studio installs them; or, with the command-line
   tools alone, what worked on Linux was:

   ```sh
   sdkmanager --licenses
   sdkmanager "platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;28.2.13676358"
   flutter config --android-sdk <where the SDK is>
   flutter doctor --android-licenses
   ```

4. `flutter doctor -v` should show a tick at "Android toolchain". It also says what is
   missing.

On Linux the first build took about three minutes and the tools about 11 GB of disk (the
SDK and NDK 2.7 GB, Gradle's downloads 3.7 GB, the build folder 4.3 GB).

## Each time: get the code and a device

The branch this work is on, `claude/affectionate-dijkstra-bx8xxw` when this was written
(use the branch you were given if it has moved on):

```sh
git fetch origin claude/affectionate-dijkstra-bx8xxw
git checkout claude/affectionate-dijkstra-bx8xxw
cd wombat/app
flutter pub get
flutter devices
```

`flutter devices` lists the phone (with USB debugging on) or the emulator, with the ID that
the commands below take as `<device>`.

## 1. Look at it

```sh
flutter run -d <device>
```

The phone screen opens on the example. `PHONE_TEST.md` is the list of what to try, written
for someone with a phone and nothing else; it covers the screen, typing and cancelling,
rows and columns, the app put away in the middle of typing, rotating, the large test
documents and the prototype log. Nobody has seen this screen on a phone yet: say what
looks wrong, what is too small to tap, and what the on-screen keyboard covers.

There is no Save: the app saves by itself (`milestones/UI2.md`). **This has never run on a
phone, so try on purpose** what `PHONE_TEST.md` has under "Put the app away in the middle
of typing", and besides:

- The back button and the back swipe: with a nested grid open they should go up one level,
  with the editor open they should close the editor, keeping what was typed, and nothing
  else (A184), and at the root they should leave the app. Say what the keyboard's own back
  does.
- Return on the on-screen keyboard (it is shown as "next"): the editor should move to the
  cell below and keep the keyboard (A180).
- Turn the phone off and on while typing.

The document is kept in the app's own data folder (decision A165):
`/data/user/0/io.github.sblumenf.burrow/files/documents/example.wombat`, by the plugin's
code; nobody has looked. (`adb shell run-as io.github.sblumenf.burrow ls -lR files` should
show it for a debug build.) It should still be there after the app is installed again over
itself; before A165 it was in the app's `code_cache` folder, which Android emptied then.
Say in `milestones/UI2.md` where it really was. If the app ever starts with "The document
could not be opened", it offers to put the file aside: say what led to it.

## 2. Let the tests press the buttons

```sh
flutter test integration_test/app_test.dart -d <device>
flutter test integration_test/grid_test.dart -d <device>
```

Each should end with "All tests passed!".

## 3. Measure

```sh
flutter drive --profile --driver=test_driver/integration_test.dart \
  --target=integration_test/measure_test.dart -d <device>
```

Run it twice and keep the report of the second run: the first builds the 100,000-cell
document, the second only opens it. The report is printed on the computer. Use a real phone
if you can; an x86 emulator may refuse a profile build, and its numbers say little about a
phone.

Write the report into `milestones/B1.md` beside the desktop numbers, with the name of the
phone. Three things to look at in particular:

- **Was the app killed while opening?** Opening peaked at about 16 times the size of the
  file on desktop.
- **How long an edit takes with the formula** (40 and 80 ms on desktop).
- **Whether memory at the end is far above memory after the first open** (it was on
  desktop Linux, which looked like its allocator and not a leak).

## If the build fails

- "Plugin org.jetbrains.kotlin.android was not found" on the first build was, on the cloud
  machine, a download refused for the moment by Maven Central (HTTP 429). Running the same
  command again got further each time.
- "cannot find -lwombat_bridge": the package in `crates/wombat-bridge/Cargo.toml` must be
  named `wombat_bridge`, with an underscore (decision A131).
- The build compiles the Rust library with rustup's `stable`, not the version pinned in
  `rust-toolchain.toml` (A131). If a newer stable fails where the pin works, say so in
  `B1.md`.

## To install without building

```sh
flutter build apk --release --target-platform android-arm64,android-x64
adb install build/app/outputs/flutter-apk/app-release.apk
```

The package is signed with Android's debug key, which is enough to install it on your own
phone and nothing more. Its application ID is `io.github.sblumenf.burrow` (decision A164)
and its name on the phone is Wombat.

## The emulator, as CI runs it

CI runs the integration tests on an emulator; the same by hand, with the Android SDK and its
command-line tools installed and KVM available (written from the CI job; not run on the
machine these documents were written on, which has no SDK):

```sh
sdkmanager emulator platform-tools "system-images;android-34;default;x86_64"
avdmanager create avd --name wombat --package "system-images;android-34;default;x86_64" --device pixel_6
emulator -avd wombat          # CI adds: -no-window -gpu swiftshader_indirect -memory 4096 -cores 2
rustup target add --toolchain stable x86_64-linux-android i686-linux-android
cd wombat/app
flutter test integration_test/app_test.dart -d emulator-5554
flutter test integration_test/grid_test.dart -d emulator-5554
flutter test integration_test/measure_test.dart -d emulator-5554
```

One file per call. Leave "Remove animations" and the developer options' animation scales
alone: with the transition animation scale at 0 Flutter shortens its own animations to a
twentieth, and a test that waits for the screen to stop drawing then reads it too early
(the first CI run failed that way; the tests have since been changed to wait for what they
expect).

What it showed (CI run 43272689795, of the app before decisions A160 to A165): the library
loads and the three test files pass on Android 14, x86-64. The app's temporary folder is
`/data/user/0/<application id>/code_cache` (`/data/user/0` is a link to `/data/data`), and
Android empties it when the app is installed again over itself, which is why the document
is no longer kept there. What it does not show: anything of a real phone — the on-screen
keyboard, the back gesture, the app being stopped out of sight, arm64, speed (the
emulator's times are of an unoptimised build: open 2.0 s, save 0.65 s, one edit with a
whole-document formula 0.14 s for 101,001 cells; peak memory about 550 MiB with the
document built in the same process). `milestones/B1.md` has the whole report.

The measuring keeps its large document in the app's data folder on a phone
(`files/wombat-measure/`), so the second of two runs opens what the first built, if the app
was not removed in between (`flutter test --no-uninstall` for the first).
