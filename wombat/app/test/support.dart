import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/frb_generated.dart';

/// Loads the bridge as the tests use it, without a device. It builds the library
/// first, so that a test never passes or fails on what some earlier build left behind;
/// cargo does nothing when it is up to date.
Future<void> loadBridge() async {
  final build = await Process.run('cargo', [
    'build',
    '--locked',
    '-p',
    'wombat_bridge',
  ], workingDirectory: '..');
  if (build.exitCode != 0) {
    throw StateError('cargo build -p wombat_bridge failed:\n${build.stderr}');
  }
  final target = Platform.environment['CARGO_TARGET_DIR'] ?? '../target';
  final name = Platform.isWindows
      ? 'wombat_bridge.dll'
      : Platform.isMacOS
      ? 'libwombat_bridge.dylib'
      : 'libwombat_bridge.so';
  await RustLib.init(
    externalLibrary: ExternalLibrary.open('$target/debug/$name'),
  );
}

/// A copy of the example that comes with the app, in `folder`, opened.
Future<Session> openExample(Directory folder, String name) => Session.openCopy(
  from: 'assets/example.wombat',
  to: '${folder.path}/$name.wombat',
);
