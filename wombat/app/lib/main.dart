import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'demo_screen.dart';
import 'grid/grid_screen.dart';
import 'grid/stage.dart';
import 'session.dart';
import 'src/rust/frb_generated.dart';

/// The Wombat app as far as it goes: a simple grid screen on the example document, and
/// the plain-text check of the bridge behind a menu entry. Not yet the interface that
/// `docs/UI_SPEC.md` specifies.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const WombatApp());
}

class WombatApp extends StatelessWidget {
  const WombatApp({super.key});

  @override
  Widget build(BuildContext context) =>
      const MaterialApp(title: 'Wombat', home: StartScreen());
}

/// Opens the example that comes with the app and shows it on the grid screen.
class StartScreen extends StatefulWidget {
  const StartScreen({super.key});

  @override
  State<StartScreen> createState() => _StartScreenState();
}

class _StartScreenState extends State<StartScreen> {
  late final Future<Stage> _stage = _open();

  /// Many installs have the same example file, so the core makes a copy of it with IDs
  /// of its own (decision A28), and that copy is what is opened. It goes to a folder the
  /// app may write to on every platform; where documents really live is still to decide.
  Future<Stage> _open() async {
    final example = await rootBundle.load('assets/example.wombat');
    final folder = await Directory.systemTemp.createTemp('wombat');
    final original = File('${folder.path}/example-original.wombat');
    await original.writeAsBytes(example.buffer.asUint8List(), flush: true);
    final doc = await Session.openCopy(
      from: original.path,
      to: '${folder.path}/example.wombat',
    );
    return Stage.open(doc);
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
