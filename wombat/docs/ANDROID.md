# Running the app on Android

For a person at a machine with an Android phone or emulator. The app builds for Android (it
was built on a Linux cloud machine that could not run it); **nothing has been seen to run
on Android yet.** What you find goes into `milestones/B1.md` and `milestones/UI1.md`.

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

```sh
git fetch origin claude/focused-cannon-r4a117
git checkout claude/focused-cannon-r4a117
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

The grid screen opens on the example. Try: tap a cell and type; the tick; Return (it should
move to the cell below); "Open ›" under "Costs"; the arrow and the names in the bar at the
bottom; undo and redo at the top; the menu, and "Bridge check" in it with its three buttons.
Nobody has seen this screen yet, on any device: say what looks wrong, what is too small to
tap, and what the on-screen keyboard covers.

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
phone and nothing more. Its application ID is still `com.example.wombat_app`.
