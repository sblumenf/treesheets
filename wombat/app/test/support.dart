import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:wombat_app/src/rust/api/document.dart';
import 'package:wombat_app/src/rust/frb_generated.dart';

/// Loads the bridge as the tests use it, without a device: the library that
/// `cargo build -p wombat_bridge`, run in `wombat/`, built.
Future<void> loadBridge() => RustLib.init(
  externalLibrary: ExternalLibrary.open('../target/debug/libwombat_bridge.so'),
);

/// A copy of the example that comes with the app, in `folder`, opened.
Future<OpenDocument> openExample(Directory folder, String name) =>
    OpenDocument.openCopy(
      from: 'assets/example.wombat',
      to: '${folder.path}/$name.wombat',
    );
