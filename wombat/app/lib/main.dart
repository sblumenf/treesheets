import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'demo_screen.dart';
import 'grid/grid_screen.dart';
import 'grid/stage.dart';
import 'session.dart';
import 'src/rust/frb_generated.dart';

/// The Wombat app as far as it goes: a simple grid screen on one document, which starts
/// as the example, and the plain-text check of the bridge behind a menu entry. Not yet
/// the interface that `docs/UI_SPEC.md` specifies.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const WombatApp());
}

class WombatApp extends StatelessWidget {
  const WombatApp({super.key, this.folder});

  /// Where the app keeps its document; see [StartScreen.folder].
  final Directory? folder;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Wombat',
    home: StartScreen(folder: folder),
  );
}

/// Opens the app's document, at first a copy of the example that comes with the app,
/// and shows it on the grid screen.
class StartScreen extends StatefulWidget {
  const StartScreen({super.key, this.folder});

  /// Where the document is kept. Without one it is a folder of fixed name in the
  /// system's temporary folder, which the app may write to on every platform but which
  /// the system may clear: where documents really live is still to decide (A148).
  /// Tests give a folder of their own.
  final Directory? folder;

  @override
  State<StartScreen> createState() => _StartScreenState();
}

class _StartScreenState extends State<StartScreen> {
  late final Future<Stage> _stage = _open();

  /// The document is the one that the last start left, with everything done to it, or
  /// if there is none yet, a copy of the example. Many installs have the same example
  /// file, so the core makes the copy, with IDs of its own (decision A28).
  Future<Stage> _open() async {
    final folder =
        widget.folder ?? Directory('${Directory.systemTemp.path}/wombat');
    await folder.create(recursive: true);
    final path = '${folder.path}/example.wombat';
    if (File(path).existsSync()) return Stage.open(await Session.open(path));
    final example = await rootBundle.load('assets/example.wombat');
    // The core copies from a file, so the example is one for a moment.
    final original = File('${folder.path}/example-original.wombat');
    await original.writeAsBytes(example.buffer.asUint8List(), flush: true);
    try {
      return await Stage.open(
        await Session.openCopy(from: original.path, to: path),
      );
    } finally {
      await original.delete();
    }
  }

  /// Saves and closes the document when the screen goes; there is no one left to show
  /// an error to, and the stage has let go of the document either way.
  @override
  void dispose() {
    _stage.then((stage) => stage.close()).catchError((Object _) {});
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<Stage>(
      future: _stage,
      builder: (context, snapshot) {
        final stage = snapshot.data;
        if (stage != null) {
          return GridScreen(
            stage: stage,
            menu: [
              PopupMenuItem(
                key: const Key('bridge-check'),
                value: () => Navigator.of(context).push(
                  MaterialPageRoute<void>(builder: (_) => const DemoScreen()),
                ),
                child: const Text('Bridge check'),
              ),
            ],
          );
        }
        final error = snapshot.error;
        return Scaffold(
          body: Center(
            child: error == null
                ? const CircularProgressIndicator()
                : Text(describeError(error), key: const Key('start-error')),
          ),
        );
      },
    );
  }
}
