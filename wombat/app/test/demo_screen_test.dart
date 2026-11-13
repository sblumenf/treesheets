// The screen of the plain-text check without a device, on the real core. The core
// answers in real time, which a widget test does not let pass by itself: `until` does.
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/demo.dart';
import 'package:wombat_app/demo_screen.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'support.dart';

void main() {
  setUpAll(loadBridge);

  Finder key(String name) => find.byKey(Key(name));

  String output(WidgetTester tester) =>
      tester.widget<SelectableText>(key('output')).data!;

  bool enabled(WidgetTester tester, String name) =>
      tester.widget<FilledButton>(key(name)).onPressed != null;

  /// Lets real time pass until `done` says so, ten seconds at most: the core answers
  /// when it answers, and the test waits for that and not for a time.
  Future<void> until(WidgetTester tester, bool Function() done) async {
    final watch = Stopwatch()..start();
    while (!done() && watch.elapsed < const Duration(seconds: 10)) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 2)),
      );
      await tester.pump();
    }
    expect(done(), isTrue, reason: 'waited ten seconds');
  }

  /// A press, and then until the step it started has ended: Open can be pressed
  /// again.
  Future<void> press(WidgetTester tester, String name) async {
    await tester.tap(key(name));
    await tester.pump();
    await until(tester, () => enabled(tester, 'open'));
  }

  /// The folders that the screen made for its copies and has not removed.
  Set<String> folders() => {
    for (final entry in Directory.systemTemp.listSync())
      if (entry.path.contains('wombat-check')) entry.path,
  };

  testWidgets('two presses in one frame are one step', (tester) async {
    final before = folders();
    final demo = Demo();
    await tester.pumpWidget(MaterialApp(home: DemoScreen(demo: demo)));
    expect(
      (enabled(tester, 'change'), enabled(tester, 'undo')),
      (false, false),
    );
    await press(tester, 'open');
    expect(output(tester), contains('"=SUM(Costs)" = "10"'));

    // The second press comes before the button could be drawn as switched off.
    await tester.tap(key('change'));
    await tester.tap(key('change'));
    await tester.pump();
    // While the step runs nothing can be pressed: the steps build on each other.
    expect(
      (
        enabled(tester, 'open'),
        enabled(tester, 'change'),
        enabled(tester, 'undo'),
      ),
      (false, false, false),
    );
    await until(tester, () => enabled(tester, 'open'));
    expect(output(tester), contains('Set the first cost to 31'));
    await press(tester, 'undo');
    expect(output(tester), contains('"=SUM(Costs)" = "10"'));

    // A second Open uses the same folder, and leaves the first copy in it.
    final made = folders().difference(before);
    expect(made.length, 1);
    await press(tester, 'open');
    expect(folders().difference(before), made);
    expect(Directory(made.single).listSync().length, 2);

    // Taking the screen away closes its document and removes the folder of its copies.
    expect(demo.isOpen, isTrue);
    await tester.pumpWidget(const SizedBox());
    await until(tester, () => folders().difference(before).isEmpty);
    expect(demo.isOpen, isFalse);
  });

  testWidgets('a step that ends after the screen is gone is not shown', (
    tester,
  ) async {
    final before = folders();
    final demo = Demo();
    await tester.pumpWidget(MaterialApp(home: DemoScreen(demo: demo)));
    await tester.tap(key('open'));
    await tester.pumpWidget(const SizedBox());
    // The step went on to make its folder and open its document; both go.
    await until(
      tester,
      () => !demo.isOpen && folders().difference(before).isEmpty,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('what goes wrong is shown as the app shows errors', (
    tester,
  ) async {
    final before = folders();
    final demo = _Failing();
    await tester.pumpWidget(MaterialApp(home: DemoScreen(demo: demo)));
    await press(tester, 'open');
    await press(tester, 'change');
    expect(output(tester), 'Error (refused): not so');
    // After a fault, the fixed sentence and not the text of the exception.
    await press(tester, 'undo');
    expect(output(tester), faultSentence);
    // A close that fails when the screen goes is no one's to see, and the folder
    // is removed all the same.
    await tester.pumpWidget(const SizedBox());
    await until(tester, () => folders().difference(before).isEmpty);
    expect(demo.closes, 2);
    expect(tester.takeException(), isNull);
    await tester.runAsync(demo.reallyClose);
  });
}

/// The steps of the check, with two that fail and a close that fails.
class _Failing extends Demo {
  int closes = 0;

  @override
  Future<String> change() async =>
      throw const BridgeError(kind: ErrorKind.refused, message: 'not so');

  @override
  Future<String> undo() async =>
      throw PanicException('panicked at /home/builder/core.rs:1:1');

  @override
  Future<void> close() async {
    closes++;
    if (isOpen) throw StateError('cannot close');
  }

  Future<void> reallyClose() => super.close();
}
