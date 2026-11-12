// The simple grid screen of the real app on a real device (or desktop): goes into a
// nested grid, types into a cell, comes back out and undoes; then that the document is
// in its file without anyone saving it, and is there again at the next start.
// Run with: flutter test integration_test/grid_test.dart -d linux   (or -d <a device>)
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:wombat_app/example.dart';
import 'package:wombat_app/grid/grid_screen.dart';
import 'package:wombat_app/grid/stage.dart';
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

  /// Waits, for a few seconds at most, until `done` says so.
  Future<void> until(WidgetTester tester, bool Function() done) async {
    for (var i = 0; i < 100 && !done(); i++) {
      await tester.pump(const Duration(milliseconds: 50));
    }
    expect(done(), isTrue);
  }

  /// Takes the app off the screen and waits until the document of `stage` is closed:
  /// for that itself, not for a time.
  Future<void> end(WidgetTester tester, Stage stage) async {
    await tester.pumpWidget(const SizedBox());
    await until(tester, () => stage.ended != null);
    expect(stage.ended, 'The document was closed.');
  }

  testWidgets('into a grid, type, back out, undo; saved without asking', (
    tester,
  ) async {
    // A folder of this run's own: the app keeps its document between starts.
    final folder = await Directory.systemTemp.createTemp('wombat-grid-test');
    final file = File('${folder.path}/example.wombat');
    await tester.pumpWidget(WombatApp(folder: folder));
    await tester.pumpAndSettle();

    // The root: two columns, three rows, a formula that shows its result.
    expect(title(tester), 'Example');
    expect(shown(tester, 0), 'Shopping');
    expect(shown(tester, 2), 'Costs');
    expect(shown(tester, 5), '10');
    expect(key('open-2'), findsOneWidget);
    expect(key('open-1'), findsNothing);
    expect(tester.widget<IconButton>(key('up')).onPressed, isNull);

    // What is open is a copy with IDs of its own, not the file that every install
    // has (decision A28): no ID of the example is in the copy's file, and the stage
    // shows the cells of that file.
    final example = utf8.decode(
      bytesOf(await rootBundle.load('assets/example.wombat')),
    );
    final ids = RegExp(r'"id": "([0-9a-f-]{36})"');
    final theirs = ids.allMatches(example).map((m) => m[1]!).toSet();
    final mine = ids
        .allMatches(file.readAsStringSync())
        .map((m) => m[1]!)
        .toSet();
    expect(theirs.length, 15);
    expect(mine.length, 15);
    expect(mine.intersection(theirs), isEmpty);
    final stage = tester.widget<GridScreen>(find.byType(GridScreen)).stage;
    expect(mine, contains(stage.grid.cell.id));
    expect(mine.containsAll(stage.grid.cells.map((c) => c.id)), isTrue);
    expect(Directory(folder.path).listSync().map((e) => e.path), [file.path]);

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

    // Nobody saved, and a moment later it is in the file.
    await until(tester, () => file.readAsStringSync().contains('"text": "30"'));

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

    // Back out by the trail, with the editor open: what is in it is kept, and the
    // formula has followed.
    await tester.enterText(key('editor'), '7');
    await press(tester, 'trail-0');
    expect(key('editor'), findsNothing);
    expect(title(tester), 'Example');
    expect(shown(tester, 5), '57');

    // The editor of a formula cell shows the formula, not the result.
    await press(tester, 'cell-5');
    expect(
      tester.widget<TextField>(key('editor')).controller!.text,
      '=SUM(Costs)',
    );
    await press(tester, 'cancel');

    // Undo three times, then once more with nothing left; redo.
    await press(tester, 'undo');
    expect(shown(tester, 5), '55');
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

    // The system's back goes up one level too, and at the root it is left alone
    // here: it would end the app under the test.
    await press(tester, 'open-2');
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(title(tester), 'Example');

    // The app goes out of sight with something half typed: that is in the file at
    // once, and the editor is as it was when the app comes back. The cross then
    // still means "not this": the cell and the file are as before.
    await press(tester, 'cell-1');
    await tester.enterText(key('editor'), 'not this');
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    await until(
      tester,
      () => file.readAsStringSync().contains('"text": "not this"'),
    );
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(
      tester.widget<TextField>(key('editor')).controller!.text,
      'not this',
    );
    await press(tester, 'cancel');
    expect(shown(tester, 1), 'what to buy');
    await until(
      tester,
      () => !file.readAsStringSync().contains('"text": "not this"'),
    );

    // Once more, and this time the app is ended with the editor open: the text is
    // entered and saved as the screen goes.
    await press(tester, 'cell-1');
    await tester.enterText(key('editor'), 'typed and left');

    // The plain-text check of the bridge is behind the menu, and there is no Save.
    await press(tester, 'menu');
    expect(find.text('Save'), findsNothing);
    await press(tester, 'bridge-check');
    expect(find.text('Press Open.'), findsOneWidget);

    // The app ends and starts again: the document is as it was left.
    await end(tester, stage);
    expect(file.readAsStringSync(), contains('"text": "typed and left"'));
    await tester.pumpWidget(WombatApp(folder: folder));
    await tester.pumpAndSettle();
    expect(key('start-error'), findsNothing);
    expect(title(tester), 'Example');
    expect(shown(tester, 1), 'typed and left');
    expect(shown(tester, 5), '37');
    await press(tester, 'open-2');
    expect([for (var i = 0; i < 3; i++) shown(tester, i)], ['30', '2', '5']);

    await end(tester, tester.widget<GridScreen>(find.byType(GridScreen)).stage);
    await folder.delete(recursive: true);
  });
}
