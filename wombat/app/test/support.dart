import 'dart:convert';
import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/frb_generated.dart';

/// Loads the bridge as the tests use it, without a device. It builds the library
/// first, so that a test never passes or fails on what some earlier build left behind;
/// cargo does nothing when it is up to date. Where the library is, cargo says itself:
/// that depends on its configuration and environment, which are not for guessing.
Future<void> loadBridge() async {
  final build = await Process.run('cargo', [
    'build',
    '--locked',
    '-p',
    'wombat_bridge',
    '--message-format=json',
  ], workingDirectory: '..');
  if (build.exitCode != 0) {
    throw StateError('cargo build -p wombat_bridge failed:\n${build.stderr}');
  }
  await RustLib.init(
    externalLibrary: ExternalLibrary.open(bridgeLibrary('${build.stdout}')),
  );
}

/// The file of the bridge's library that can be loaded, from what
/// `cargo build --message-format=json` printed: one JSON object per line, of which the
/// one for the bridge names the files it made, from the root.
String bridgeLibrary(String messages) {
  for (final line in const LineSplitter().convert(messages)) {
    if (!line.startsWith('{')) continue;
    final message = jsonDecode(line);
    if (message case {
      'reason': 'compiler-artifact',
      'target': {'name': 'wombat_bridge'},
      'filenames': final List<Object?> files,
    }) {
      for (final file in files.whereType<String>()) {
        if (const ['.so', '.dylib', '.dll'].any(file.endsWith)) return file;
      }
    }
  }
  throw StateError('cargo did not say where it built the bridge:\n$messages');
}

/// A copy of the example that comes with the app, in `folder`, opened.
Future<Session> openExample(Directory folder, String name) => Session.openCopy(
  from: 'assets/example.wombat',
  to: '${folder.path}/$name.wombat',
);
