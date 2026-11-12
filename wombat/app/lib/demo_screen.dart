import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'demo.dart';
import 'example.dart';
import 'session.dart';

/// The screen of milestone B1: plain text and three buttons, to show that a document
/// opens, reads, changes and undoes through the Rust core. Kept as a check of the bridge.
class DemoScreen extends StatefulWidget {
  const DemoScreen({super.key, this.demo});

  /// The steps behind the buttons; a test gives its own, to see what became of them.
  final Demo? demo;

  @override
  State<DemoScreen> createState() => _DemoScreenState();
}

class _DemoScreenState extends State<DemoScreen> {
  late final _demo = widget.demo ?? Demo();
  String _text = 'Press Open.';
  bool _busy = false;

  /// A folder for the copies this screen makes, made at the first Open and removed
  /// with the screen: they are for looking at, not for keeping.
  Directory? _folder;

  /// The step that runs, or the last one that did.
  Future<void> _step = Future.value();

  /// Runs one step and shows its text, or the error it ended with. A press while a
  /// step runs does nothing: the steps build on each other's text on the screen.
  void _run(Future<String> Function() step) {
    if (!_busy) _step = _show(step);
  }

  Future<void> _show(Future<String> Function() step) async {
    setState(() => _busy = true);
    String text;
    try {
      text = await step();
    } catch (error) {
      text = describeError(error);
    }
    if (!mounted) return;
    setState(() {
      _text = text;
      _busy = false;
    });
  }

  Future<String> _open() async {
    final example = await rootBundle.load('assets/example.wombat');
    final folder = _folder ??= await Directory.systemTemp.createTemp(
      'wombat-check',
    );
    return _demo.open(folder, bytesOf(example));
  }

  /// Closes the document, and only then removes its folder; both after a step that is
  /// still running, which may be about to make the one or the other.
  @override
  void dispose() {
    _step.whenComplete(
      // There is no one left to show an error of these two to.
      () => _demo
          .close()
          .catchError((Object _) {})
          .whenComplete(() => _folder?.delete(recursive: true))
          .catchError((Object _) {}),
    );
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // `_run` drops a press while a step runs whatever the buttons look like; they
    // are switched off as well so that the user sees that nothing can be pressed.
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
