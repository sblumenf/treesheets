// The phone screen of the real app on a real device (or desktop): the flows of
// UI_SPEC section 17 that the first slice covers (1, 2, 3, 7, 9 and 10), on the
// example and on the generated six-level document; that the document is in its file
// without anyone saving it, also what was half typed when the app went out of sight;
// and that it is there again at the next start. Nothing here depends on the size of
// the screen: a grid is gone into by its cell and "Open ›", whatever form it is
// drawn in, and a cell is scrolled to before it is tapped.
// Run with: flutter test integration_test/grid_test.dart -d linux   (or -d <a device>)
import 'dart:convert';
import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:wombat_app/example.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/main.dart';
import 'package:wombat_app/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  Finder key(String name) => find.byKey(Key(name));

  bool there(String name) => key(name).evaluate().isNotEmpty;

  String? line(WidgetTester tester, String name) =>
      there(name) ? tester.widget<Text>(key(name)).data : null;

  /// What a cell shows, by its place: `x-y` for column x and row y of the grid on
  /// the stage, `x-y/i-j` for a cell of the grid drawn inside that one.
  String shown(WidgetTester tester, String cell) => tester
      .widget<Text>(
        find.descendant(of: key('cell-$cell'), matching: find.byType(Text)),
      )
      .textSpan!
      .toPlainText();

  String editor(WidgetTester tester) =>
      tester.widget<TextField>(key('editor')).controller!.text;

  Keeper keeperOf(WidgetTester tester) =>
      tester.widget<Kept>(find.byType(Kept)).keeper;

  /// Waits, for some seconds at most, until `done` says so. Real time passes
  /// whether or not the app draws: out of sight it draws nothing, and a wait for
  /// a frame would not end. What `done` throws (a widget that is not there to be
  /// asked) is "not yet".
  Future<void> until(
    WidgetTester tester,
    bool Function() done, {
    String? what,
  }) async {
    bool isDone() {
      try {
        return done();
      } catch (_) {
        return false;
      }
    }

    for (var i = 0; i < 600 && !isDone(); i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 50)),
      );
      if (!const [
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
        AppLifecycleState.detached,
      ].contains(tester.binding.lifecycleState)) {
        await tester.pump();
      }
    }
    expect(
      isDone(),
      isTrue,
      reason:
          'waited for ${what ?? 'something'}; the title is '
          '${line(tester, 'title')}, the message ${line(tester, 'message')}',
    );
  }

  /// Lets what moves on the screen come to rest: the level that is arrived at,
  /// a menu. For a time, and not until nothing is drawn: a frame may always be
  /// due for something else.
  Future<void> rest(WidgetTester tester) async {
    for (var i = 0; i < 8; i++) {
      await tester.pump(const Duration(milliseconds: 60));
    }
  }

  /// Scrolls the thing with this key to the middle of what is seen.
  Future<void> reveal(WidgetTester tester, String name) async {
    await until(tester, () => there(name), what: name);
    await Scrollable.ensureVisible(tester.element(key(name)), alignment: 0.5);
    await rest(tester);
  }

  /// A finger drawn across the middle of the grid, as far as `by`. (The middle of
  /// what is seen of it: the grid itself can be wider than the screen.)
  Future<void> swipe(WidgetTester tester, Offset by) async {
    await tester.dragFrom(tester.getCenter(key('stage-across')), by);
    await rest(tester);
  }

  /// Taps, and waits until the screen shows what `then` expects: the core answers
  /// when it answers, and no frame is due while the screen waits for it.
  Future<void> press(
    WidgetTester tester,
    String name,
    bool Function() then, {
    String? what,
  }) async {
    await reveal(tester, name);
    await tester.tap(key(name));
    await tester.pump();
    await until(tester, then, what: what ?? 'what $name does');
    await rest(tester);
  }

  /// Opens the editor on a cell, on the text it is to open with.
  Future<void> edit(WidgetTester tester, String cell, String text) => press(
    tester,
    'cell-$cell',
    () => editor(tester) == text,
    what: 'the editor on "$text"',
  );

  Future<void> type(WidgetTester tester, String text) async {
    await tester.enterText(key('editor'), text);
    await tester.pump();
  }

  Future<void> tick(WidgetTester tester) =>
      press(tester, 'save', () => !there('editor'));

  Future<void> cross(WidgetTester tester) =>
      press(tester, 'cancel', () => !there('editor'));

  /// Into the grid of a cell, by the cell and "Open ›", which works whatever form
  /// the grid is drawn in at this size of screen.
  Future<void> into(WidgetTester tester, String cell, String title) async {
    await edit(tester, cell, title);
    await press(tester, 'tool-open', () => line(tester, 'title') == title);
  }

  /// An entry of the menu of the title bar.
  Future<void> choose(WidgetTester tester, String entry) async {
    await tester.tap(key('menu'));
    await until(tester, () => there(entry), what: 'the menu');
    await rest(tester);
    await tester.tap(key(entry));
    await rest(tester);
  }

  /// Shows a test document, and waits until its first level is drawn.
  Future<void> document(WidgetTester tester, String name, String title) async {
    await choose(tester, 'document-$name');
    await until(
      tester,
      () =>
          !there('opening') &&
          !there('reading') &&
          line(tester, 'title') == title &&
          (there('stage-across') || there('empty-root')),
      what: 'the document $title',
    );
    await rest(tester);
  }

  /// Takes the app off the screen and waits until its document is closed: for
  /// that itself, not for a time.
  Future<void> end(WidgetTester tester) async {
    final keeper = keeperOf(tester);
    await tester.pumpWidget(const SizedBox());
    await until(tester, () => keeper.ended != null, what: 'the close');
    expect(keeper.ended, 'The document was closed.');
  }

  testWidgets('the flows of the first slice; saved without asking', (
    tester,
  ) async {
    // A folder of this run's own: the app keeps its documents between starts.
    final folder = await Directory.systemTemp.createTemp('wombat-grid-test');
    final file = File('${folder.path}/example.wombat');
    bool inFile(String text) =>
        file.readAsStringSync().contains('"text": "$text"');
    await tester.pumpWidget(WombatApp(folder: folder));
    await until(tester, () => there('stage-across'), what: 'the first level');
    await rest(tester);

    // The root of the example: two columns, three rows, a formula that shows its
    // result, and two small grids drawn cell by cell.
    expect(line(tester, 'title'), 'Example');
    expect(line(tester, 'state'), 'Saved');
    expect(shown(tester, '0-0'), 'Shopping');
    expect(shown(tester, '0-1'), 'Costs');
    expect(shown(tester, '1-2'), '10');
    expect(
      [for (var y = 0; y < 3; y++) shown(tester, '0-1/0-$y')],
      ['3', '2', '5'],
    );
    expect(tester.widget<IconButton>(key('up')).onPressed, isNull);
    expect(find.text('Save'), findsNothing);

    // What is open is a copy with IDs of its own, not the file that every install
    // has (decision A28): no ID of the example is in the copy's file.
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
    expect(mine, contains(keeperOf(tester).root));
    expect(Directory(folder.path).listSync().map((e) => e.path), [file.path]);

    // Flow 10: fix the text of three cells in a small nested grid, then continue
    // at the top. Return saves and moves to the cell below.
    await edit(tester, '0-0/0-1', 'Apples');
    await type(tester, 'Pears');
    expect(shown(tester, '0-0/0-1'), 'Pears');
    await tester.testTextInput.receiveAction(TextInputAction.next);
    await until(
      tester,
      () => editor(tester) == 'Bread',
      what: 'the cell below',
    );
    await type(tester, 'Rolls');
    await tester.testTextInput.receiveAction(TextInputAction.next);
    await until(
      tester,
      () => editor(tester) == 'Cheese',
      what: 'the cell below',
    );
    await type(tester, 'Butter');
    await tick(tester);
    expect(
      [for (var y = 0; y < 4; y++) shown(tester, '0-0/0-$y')],
      ['Item', 'Pears', 'Rolls', 'Butter'],
    );
    expect(line(tester, 'title'), 'Example');
    await edit(tester, '1-0', 'what to buy');
    await type(tester, 'what was bought');
    await tick(tester);
    expect(shown(tester, '1-0'), 'what was bought');

    // Nobody saved, and a moment later it is in the file.
    await until(tester, () => inFile('Butter'), what: 'the save');
    await until(tester, () => line(tester, 'state') == 'Saved');

    // The cross leaves the cell as it is.
    await edit(tester, '1-1', 'what it costs');
    await type(tester, '999');
    expect(shown(tester, '1-1'), '999');
    await cross(tester);
    expect(shown(tester, '1-1'), 'what it costs');

    // Into "Costs", a number changed, and back out by the trail with the editor
    // open: what is in it is kept, and the formula has followed.
    await into(tester, '0-1', 'Costs');
    expect(
      [for (var y = 0; y < 3; y++) shown(tester, '0-$y')],
      ['3', '2', '5'],
    );
    await edit(tester, '0-0', '3');
    await type(tester, '30');
    await press(tester, 'trail-0', () => line(tester, 'title') == 'Example');
    expect(there('editor'), isFalse);
    expect(shown(tester, '1-2'), '37');

    // The editor of a formula cell shows the formula, not the result.
    await edit(tester, '1-2', '=SUM(Costs)');
    await cross(tester);

    // Flow 7: undo the last three changes; then two more, to the end, with its
    // message; redo.
    await press(tester, 'undo', () => shown(tester, '1-2') == '10');
    await press(tester, 'undo', () => shown(tester, '1-0') == 'what to buy');
    await press(tester, 'undo', () => shown(tester, '0-0/0-3') == 'Cheese');
    await press(tester, 'undo', () => shown(tester, '0-0/0-2') == 'Bread');
    await press(tester, 'undo', () => shown(tester, '0-0/0-1') == 'Apples');
    await press(
      tester,
      'undo',
      () => line(tester, 'message') == 'Nothing to undo',
    );
    // The message goes with the next action, and never covers the bars.
    await press(
      tester,
      'redo',
      () => !there('message') && shown(tester, '0-0/0-1') == 'Pears',
    );

    // The system's back goes up one level, and at the root it is left alone
    // here: it would end the app under the test.
    await into(tester, '0-1', 'Costs');
    await tester.binding.handlePopRoute();
    await until(tester, () => line(tester, 'title') == 'Example');
    await rest(tester);

    // The app goes out of sight with something half typed: that is in the file at
    // once, and the editor is as it was when the app comes back. The cross then
    // still means "not this": the cell and the file are as before.
    await edit(tester, '1-1', 'what it costs');
    await type(tester, 'not this');
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    await until(tester, () => inFile('not this'), what: 'the text in the file');
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await rest(tester);
    expect(editor(tester), 'not this');
    await cross(tester);
    expect(shown(tester, '1-1'), 'what it costs');
    await until(tester, () => !inFile('not this'), what: 'the file put right');

    // The generated document of the specification: six levels, 12 by 20 at the
    // root. What was typed and not said yes to in the example goes with it.
    await edit(tester, '1-1', 'what it costs');
    await type(tester, 'left for another document');
    await document(tester, 'nested', 'Six levels');
    expect(inFile('left for another document'), isTrue);

    // Flow 9: in a grid of more than nine columns, reach row 12, column 7: down
    // and sideways by hand, the first row and column staying in view.
    for (var i = 0; i < 40 && !there('cell-6-11'); i++) {
      await swipe(tester, const Offset(0, -250));
    }
    await reveal(tester, 'cell-6-11');
    expect(shown(tester, '6-11'), 'Level 0 cell 6.11 text');
    expect(shown(tester, '0-0'), 'Level 0 cell 0.0 text');
    expect(shown(tester, '0-11'), 'Level 0 cell 0.11 text');
    await edit(tester, '6-11', 'Level 0 cell 6.11 text');
    await cross(tester);
    await swipe(tester, const Offset(0, 100000));

    // Flows 1 and 2: down to a cell three levels deep without search, editing one
    // two levels deep on the way.
    await into(tester, '3-0', 'Level 0 cell 3.0 text');
    expect(shown(tester, '3-0'), 'Level 1 cell 3.0 text');
    await into(tester, '3-0', 'Level 1 cell 3.0 text');
    await edit(tester, '1-0', 'Level 2 cell 1.0 text');
    await type(tester, 'edited two levels down');
    await tick(tester);
    expect(shown(tester, '1-0'), 'edited two levels down');
    await into(tester, '0-1', 'Level 2 cell 0.1 text');
    expect(shown(tester, '1-1'), 'Level 3 cell 1.1 text');
    expect(tester.widget<IconButton>(key('up')).onPressed, isNotNull);

    // Flow 3: add a row to a nested grid and fill two cells. Return in the last
    // row adds the row; → moves to the cell on the right.
    expect(there('cell-0-3'), isFalse);
    await edit(tester, '0-2', 'Level 3 cell 0.2 text');
    await tester.testTextInput.receiveAction(TextInputAction.next);
    await until(
      tester,
      () => there('cell-0-3') && editor(tester) == '',
      what: 'the new row',
    );
    await type(tester, 'first');
    await press(
      tester,
      'tool-right',
      () => shown(tester, '0-3') == 'first' && editor(tester) == '',
    );
    await type(tester, 'second');
    await tick(tester);
    expect((shown(tester, '0-3'), shown(tester, '1-3')), ('first', 'second'));

    // Flow 7 again: the last three changes are the second text, the first, and
    // the row.
    await press(tester, 'undo', () => shown(tester, '1-3') == '');
    await press(tester, 'undo', () => shown(tester, '0-3') == '');
    await press(tester, 'undo', () => !there('cell-0-3'));
    await press(tester, 'redo', () => there('cell-0-3'));

    // Up by the arrow, and to the top by the trail (flow 2): each level is where
    // it was left.
    await press(
      tester,
      'up',
      () => line(tester, 'title') == 'Level 1 cell 3.0 text',
    );
    expect(shown(tester, '1-0'), 'edited two levels down');
    await press(tester, 'trail-0', () => line(tester, 'title') == 'Six levels');
    expect(shown(tester, '0-0'), 'Level 0 cell 0.0 text');

    // The settings of the prototype and its log are behind the menu; so is the
    // plain-text check of the bridge.
    await choose(tester, 'menu-settings');
    await until(tester, () => there('variant-tap-text'));
    expect(line(tester, 'log-count'), isNot('Prototype log: 0 entries'));
    await press(tester, 'settings-close', () => !there('variant-tap-text'));
    expect(File('${folder.path}/$logName').existsSync(), isTrue);
    await choose(tester, 'bridge-check');
    await until(tester, () => find.text('Press Open.').evaluate().isNotEmpty);
    await tester.binding.handlePopRoute();
    await until(tester, () => line(tester, 'title') == 'Six levels');
    await rest(tester);

    // Back on the example, with the editor open, the system asks whether the app
    // may exit, as it does on a desktop when the window is closed. When the
    // answer comes, everything is in the file.
    await document(tester, 'example', 'Example');
    expect(shown(tester, '1-1'), 'left for another document');
    await edit(tester, '1-0', 'what to buy');
    await type(tester, 'typed and left');
    final keeper = keeperOf(tester);
    expect(await tester.binding.handleRequestAppExit(), AppExitResponse.exit);
    expect(inFile('typed and left'), isTrue);
    expect(keeper.ended, 'The document was closed.');

    // The app ends and starts again: the document is as it was left.
    await tester.pumpWidget(const SizedBox());
    await tester.pumpWidget(WombatApp(folder: folder));
    await until(tester, () => there('stage-across'), what: 'the next start');
    await rest(tester);
    expect(there('start-error'), isFalse);
    expect(line(tester, 'title'), 'Example');
    // With what was undone still undone, and what was redone redone.
    expect(shown(tester, '1-0'), 'typed and left');
    expect(shown(tester, '1-1'), 'left for another document');
    expect(shown(tester, '1-2'), '10');
    expect(
      [for (var y = 0; y < 4; y++) shown(tester, '0-0/0-$y')],
      ['Item', 'Pears', 'Bread', 'Cheese'],
    );

    await end(tester);
    await folder.delete(recursive: true);
  });
}
