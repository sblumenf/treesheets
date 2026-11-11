import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'demo.dart';
import 'src/rust/frb_generated.dart';

/// The bare app of milestone B1: one screen of plain text and three buttons, to show that
/// a document opens, reads, changes and undoes through the Rust core. No real interface.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const WombatApp());
}

class WombatApp extends StatelessWidget {
  const WombatApp({super.key});

  @override
  Widget build(BuildContext context) =>
      const MaterialApp(title: 'Wombat bridge', home: DemoScreen());
}

class DemoScreen extends StatefulWidget {
  const DemoScreen({super.key});

  @override
  State<DemoScreen> createState() => _DemoScreenState();
}

class _DemoScreenState extends State<DemoScreen> {
  final _demo = Demo();
  String _text = 'Press Open.';
  bool _busy = false;

  /// Runs one step and shows its text, or the error it ended with.
  Future<void> _run(Future<String> Function() step) async {
    setState(() => _busy = true);
    String text;
    try {
      text = await step();
    } catch (error) {
      text = describeError(error);
    }
    setState(() {
      _text = text;
      _busy = false;
    });
  }

  Future<String> _open() async {
    final example = await rootBundle.load('assets/example.wombat');
    // A folder the app may write to on every platform; where documents really go is for
    // the real interface to decide.
    final folder = await Directory.systemTemp.createTemp('wombat');
    return _demo.open(folder, example.buffer.asUint8List());
  }

  @override
  Widget build(BuildContext context) {
    final ready = !_busy && _demo.isOpen;
    return Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.all(8),
              child: Wrap(
                spacing: 8,
                children: [
                  FilledButton(
                    key: const Key('open'),
                    onPressed: _busy ? null : () => _run(_open),
                    child: const Text('Open'),
                  ),
                  FilledButton(
                    key: const Key('change'),
                    onPressed: ready ? () => _run(_demo.change) : null,
                    child: const Text('Change'),
                  ),
                  FilledButton(
                    key: const Key('undo'),
                    onPressed: ready ? () => _run(_demo.undo) : null,
                    child: const Text('Undo'),
                  ),
                ],
              ),
            ),
            Expanded(
              child: SingleChildScrollView(
                padding: const EdgeInsets.all(8),
                child: SizedBox(
                  width: double.infinity,
                  child: SelectableText(
                    _text,
                    key: const Key('output'),
                    style: const TextStyle(fontFamily: 'monospace'),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
