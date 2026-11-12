// The screen of the plain-text check without a device, on the real core. The core
// answers in real time, which a widget test does not let pass by itself: `settle` does.
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/demo.dart';
import 'package:wombat_app/demo_screen.dart';

import 'support.dart';

void main() {
  setUpAll(loadBridge);

  String output(WidgetTester tester) =>
      tester.widget<SelectableText>(find.byKey(const Key('output'))).data!;

  /// Lets real time pass until the core has answered and the screen has caught up.
  Future<void> settle(WidgetTester tester) async {
    for (var i = 0; i < 25; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 20)),
      );
      await tester.pump();
    }
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
    await tester.tap(find.byKey(const Key('open')));
    await settle(tester);
    expect(output(tester), contains('"=SUM(Costs)" = "10"'));

    // The second press comes before the button could be drawn as switched off.
    await tester.tap(find.byKey(const Key('change')));
    await tester.tap(find.byKey(const Key('change')));
    await settle(tester);
    expect(output(tester), contains('Set the first cost to 31'));
    await tester.tap(find.byKey(const Key('undo')));
    await settle(tester);
    expect(output(tester), contains('"=SUM(Costs)" = "10"'));

    // Taking the screen away closes its document and removes the folder of its copies.
    expect(folders().difference(before).length, 1);
    expect(demo.isOpen, isTrue);
    await tester.pumpWidget(const SizedBox());
    await settle(tester);
    expect(demo.isOpen, isFalse);
    expect(folders().difference(before), isEmpty);
  });

  testWidgets('a step that ends after the screen is gone is not shown', (
    tester,
  ) async {
    await tester.pumpWidget(const MaterialApp(home: DemoScreen()));
    await tester.tap(find.byKey(const Key('open')));
    await tester.pumpWidget(const SizedBox());
    await settle(tester);
    expect(tester.takeException(), isNull);
  });
}
