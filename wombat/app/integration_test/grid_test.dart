// The simple grid screen of the real app on a real device (or desktop): goes into a
// nested grid, types into a cell, comes back out and undoes.
// Run with: flutter test integration_test/grid_test.dart -d linux   (or -d <a device>)
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:wombat_app/main.dart';
import 'package:wombat_app/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  Finder key(String name) => find.byKey(Key(name));

  String title(WidgetTester tester) => tester.widget<Text>(key('title')).data!;

  /// The text that a cell of the stage shows, by its place in the grid.
  String shown(WidgetTester tester, int index) {
    final text = find.descendant(
      of: key('cell-$index'),
      matching: find.byType(Text),
    );
    return tester.widget<Text>(text).data!;
  }

  Future<void> press(WidgetTester tester, String name) async {
    await tester.tap(key(name));
    await tester.pumpAndSettle();
  }

  testWidgets('into a grid, type, back out, undo', (tester) async {
    await tester.pumpWidget(const WombatApp());
    await tester.pumpAndSettle();

    // The root: two columns, three rows, a formula that shows its result.
    expect(title(tester), 'Example');
    expect(shown(tester, 0), 'Shopping');
    expect(shown(tester, 2), 'Costs');
    expect(shown(tester, 5), '10');
    expect(key('open-2'), findsOneWidget);
    expect(key('open-1'), findsNothing);
    expect(tester.widget<IconButton>(key('up')).onPressed, isNull);

    // Into "Costs".
    await press(tester, 'open-2');
    expect(title(tester), 'Costs');
    expect([for (var i = 0; i < 3; i++) shown(tester, i)], ['3', '2', '5']);
    expect(key('editor'), findsNothing);

    // Tap the first cost: the editor opens with its text. Type and save.
    await press(tester, 'cell-0');
    expect(tester.widget<TextField>(key('editor')).controller!.text, '3');
    await tester.enterText(key('editor'), '30');
    await press(tester, 'save');
    expect(key('editor'), findsNothing);
    expect(shown(tester, 0), '30');

    // Cancel leaves the cell as it is.
    await press(tester, 'cell-1');
    await tester.enterText(key('editor'), '999');
    await press(tester, 'cancel');
    expect(shown(tester, 1), '2');

    // Return saves and moves the editor to the cell below.
    await press(tester, 'cell-1');
    await tester.enterText(key('editor'), '20');
    await tester.testTextInput.receiveAction(TextInputAction.next);
    await tester.pumpAndSettle();
    expect(shown(tester, 1), '20');
    expect(tester.widget<TextField>(key('editor')).controller!.text, '5');
    await press(tester, 'cancel');

    // Back out by the trail: the formula has followed.
    await press(tester, 'trail-0');
    expect(title(tester), 'Example');
    expect(shown(tester, 5), '55');

    // The editor of a formula cell shows the formula, not the result.
    await press(tester, 'cell-5');
    expect(
      tester.widget<TextField>(key('editor')).controller!.text,
      '=SUM(Costs)',
    );
    await press(tester, 'cancel');

    // Undo twice, then once more with nothing left; redo.
    await press(tester, 'undo');
    expect(shown(tester, 5), '37');
    await press(tester, 'undo');
    expect(shown(tester, 5), '10');
    await press(tester, 'undo');
    expect(tester.widget<Text>(key('message')).data, 'Nothing to undo');
    // The message goes with the next action, and never covers the bars.
    await press(tester, 'redo');
    expect(key('message'), findsNothing);
    expect(shown(tester, 5), '37');

    // In again and up by the arrow.
    await press(tester, 'open-2');
    expect(shown(tester, 0), '30');
    await press(tester, 'up');
    expect(title(tester), 'Example');

    // The plain-text check of the bridge is behind the menu.
    await press(tester, 'menu');
    await press(tester, 'bridge-check');
    expect(find.text('Press Open.'), findsOneWidget);
  });
}
