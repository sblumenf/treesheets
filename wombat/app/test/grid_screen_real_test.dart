// The widgets of the simple grid screen, without a device, on the real core: what the
// tests on the stand-in show once more where the stand-in could differ from the core
// (a file that another program changed, text that the core refuses, the app put away,
// a grid that could not be read again). The core answers in real time, which a widget
// test does not let pass by itself: `until` does, and waits for the thing.
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/grid/grid_screen.dart';
import 'package:wombat_app/grid/stage.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/main.dart' show Kept;
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'fake_document.dart';
import 'support.dart';

void main() {
  late Directory folder;
  var count = 0;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-screen');
  });

  tearDownAll(() => folder.delete(recursive: true));

  Finder key(String name) => find.byKey(Key(name));

  String shown(WidgetTester tester, int index) => tester
      .widget<Text>(
        find.descendant(of: key('cell-$index'), matching: find.byType(Text)),
      )
      .data!;

  String title(WidgetTester tester) => tester.widget<Text>(key('title')).data!;

  String editor(WidgetTester tester) =>
      tester.widget<TextField>(key('editor')).controller!.text;

  String? line(WidgetTester tester, String name) =>
      key(name).evaluate().isEmpty ? null : tester.widget<Text>(key(name)).data;

  /// Lets real time pass until `done` says so, ten seconds at most.
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

  /// Does something that asks the core, and waits for it as `until` waits. Not
  /// `tester.runAsync`: what was made on the clock of the test (the keeper and the
  /// stage, as in the app) does not move while that waits.
  Future<T> real<T>(WidgetTester tester, Future<T> Function() work) async {
    late T value;
    Object? error;
    var done = false;
    work().then(
      (answer) {
        value = answer;
        done = true;
      },
      onError: (Object failure) {
        error = failure;
        done = true;
      },
    );
    await until(tester, () => done);
    if (error != null) throw error!;
    return value;
  }

  /// A tap, and then until `done`.
  Future<void> press(
    WidgetTester tester,
    String name,
    bool Function() done,
  ) async {
    await tester.tap(key(name));
    await tester.pump();
    await until(tester, done);
  }

  Future<void> lifecycle(
    WidgetTester tester,
    List<AppLifecycleState> states,
  ) async {
    for (final state in states) {
      tester.binding.handleAppLifecycleStateChanged(state);
      await tester.pump();
    }
  }

  const out = [AppLifecycleState.inactive, AppLifecycleState.hidden];
  const back = [AppLifecycleState.inactive, AppLifecycleState.resumed];

  /// A test with the screen on a copy of the example, kept as the app keeps its own;
  /// the core is under a watcher that can make one call fail.
  void realTest(
    String name,
    Future<void> Function(
      WidgetTester tester,
      Stage stage,
      Session doc,
      Watched watch,
    )
    body,
  ) {
    testWidgets(name, (tester) async {
      final (doc, watch) = await real(
        tester,
        () => watchExample(folder, 'screen-${count++}'),
      );
      final keeper = Keeper(doc);
      final stage = await real(tester, () => Stage.open(keeper));
      await tester.pumpWidget(
        MaterialApp(
          home: Kept(
            keeper: keeper,
            child: GridScreen(stage: stage),
          ),
        ),
      );
      await body(tester, stage, doc, watch);
      await real(tester, () => endKeeping(keeper));
    });
  }

  /// Types "mine" into the first cell, lets another program put `theirs` in the file,
  /// and waits for the save that comes by itself to find that out.
  Future<void> changedByAnother(
    WidgetTester tester,
    Session doc,
    String Function(String file) theirs,
  ) async {
    await press(tester, 'cell-0', () => key('editor').evaluate().isNotEmpty);
    tester.testTextInput.enterText('mine');
    await press(tester, 'save', () => shown(tester, 0) == 'mine');
    final file = File(doc.path);
    file.writeAsStringSync(theirs(file.readAsStringSync()));
    await tester.pump(const Duration(seconds: 1));
    await until(tester, () => line(tester, 'trouble') != null);
    expect(
      line(tester, 'trouble'),
      'Not saved: another program changed the file.',
    );
  }

  String renamed(String file) => file.replaceFirst('"Shopping"', '"Theirs"');

  realTest('a file that another program changed: keep mine', (
    tester,
    stage,
    doc,
    watch,
  ) async {
    await changedByAnother(tester, doc, renamed);
    expect(File(doc.path).readAsStringSync(), contains('"Theirs"'));
    await press(tester, 'keep-mine', () => line(tester, 'trouble') == null);
    expect(File(doc.path).readAsStringSync(), contains('"text": "mine"'));
    expect(File(doc.path).readAsStringSync(), isNot(contains('"Theirs"')));
    expect(shown(tester, 0), 'mine');
  });

  realTest('a file that another program changed: open theirs', (
    tester,
    stage,
    doc,
    watch,
  ) async {
    await tester.tap(key('open-2'));
    await until(tester, () => title(tester) == 'Costs');
    await changedByAnother(tester, doc, renamed);
    await press(tester, 'open-theirs', () => title(tester) == 'Example');
    // The screen shows the other program's document, from the root.
    await until(tester, () => shown(tester, 0) == 'Theirs');
    expect(line(tester, 'trouble'), isNull);
    expect(line(tester, 'message'), isNull);
    expect(File(doc.path).readAsStringSync(), isNot(contains('"mine"')));
    // And works on it.
    await press(tester, 'cell-1', () => key('editor').evaluate().isNotEmpty);
    tester.testTextInput.enterText('on theirs');
    await press(tester, 'save', () => shown(tester, 1) == 'on theirs');
    await tester.pump(const Duration(seconds: 1));
    await until(
      tester,
      () => File(doc.path).readAsStringSync().contains('"on theirs"'),
    );
    expect(File(doc.path).readAsStringSync(), contains('"Theirs"'));
  });

  realTest('open theirs when theirs cannot be read: nothing is given up', (
    tester,
    stage,
    doc,
    watch,
  ) async {
    await changedByAnother(tester, doc, (_) => 'half written');
    await press(tester, 'open-theirs', () => line(tester, 'message') != null);
    expect(
      line(tester, 'message'),
      startsWith(
        'The file as the other program left it cannot be opened, so this '
        'document is still open, and not saved. Error (syntax): ',
      ),
    );
    expect(key('ended'), findsNothing);
    expect(shown(tester, 0), 'mine');
    expect(key('open-theirs'), findsOneWidget);
    await press(tester, 'keep-mine', () => line(tester, 'trouble') == null);
    expect(File(doc.path).readAsStringSync(), contains('"text": "mine"'));
  });

  realTest('what the core refuses to enter stays in the editor', (
    tester,
    stage,
    doc,
    watch,
  ) async {
    // A new row, and the editor on a cell of it; then the row is taken away again
    // behind the screen, as a second screen on the document could do.
    await real(tester, () async {
      await doc.apply(Command.insertRows(cell: doc.root, at: 3, count: 1));
      await stage.refresh();
    });
    await tester.pump();
    await press(tester, 'cell-6', () => key('editor').evaluate().isNotEmpty);
    tester.testTextInput.enterText('typed into a cell that is gone');
    await real(tester, doc.undo);
    await press(tester, 'save', () => line(tester, 'message') != null);
    expect(line(tester, 'message'), startsWith('Error (notFound): '));
    expect(editor(tester), 'typed into a cell that is gone');
    expect(tester.widget<TextField>(key('editor')).focusNode!.hasFocus, isTrue);
    await press(tester, 'cancel', () => key('editor').evaluate().isEmpty);
  });

  realTest('the app out of sight: the text is in the file, and the cross '
      'takes it out again', (tester, stage, doc, watch) async {
    bool inFile(String text) =>
        File(doc.path).readAsStringSync().contains('"text": "$text"');
    await press(tester, 'cell-1', () => key('editor').evaluate().isNotEmpty);
    tester.testTextInput.enterText('not this');
    // Only no longer in front: saved, but not what is being typed.
    final saved = watch.next('save');
    await lifecycle(tester, [AppLifecycleState.inactive]);
    await real(tester, () => saved);
    expect(inFile('not this'), isFalse);
    // Out of sight: entered and saved.
    await lifecycle(tester, [AppLifecycleState.hidden]);
    await until(tester, () => inFile('not this'));
    await lifecycle(tester, back);
    expect(editor(tester), 'not this');
    await press(tester, 'cancel', () => shown(tester, 1) == 'what to buy');
    // The real core has nothing left to undo: the entering was taken back.
    await press(tester, 'undo', () => line(tester, 'message') != null);
    expect(line(tester, 'message'), 'Nothing to undo');
    await tester.pump(const Duration(seconds: 1));
    await until(tester, () => inFile('what to buy'));
    expect(inFile('not this'), isFalse);
  });

  realTest('one editing with the app out of sight in the middle is one undo', (
    tester,
    stage,
    doc,
    watch,
  ) async {
    await press(tester, 'cell-1', () => key('editor').evaluate().isNotEmpty);
    tester.testTextInput.enterText('half');
    await lifecycle(tester, out);
    await until(
      tester,
      () => File(doc.path).readAsStringSync().contains('"half"'),
    );
    await lifecycle(tester, back);
    tester.testTextInput.enterText('half and whole');
    await press(tester, 'save', () => key('editor').evaluate().isEmpty);
    expect(shown(tester, 1), 'half and whole');
    await press(tester, 'undo', () => shown(tester, 1) == 'what to buy');
    await press(tester, 'undo', () => line(tester, 'message') != null);
    expect(line(tester, 'message'), 'Nothing to undo');
  });

  realTest('a grid that could not be read again, and then can', (
    tester,
    stage,
    doc,
    watch,
  ) async {
    await press(tester, 'cell-0', () => key('editor').evaluate().isNotEmpty);
    tester.testTextInput.enterText('typed');
    // The edit is made, by the real core, and reading the grid after it fails once.
    failFirst(
      watch,
      'grid',
      const BridgeError(kind: ErrorKind.io, message: 'not now'),
    );
    await press(tester, 'save', () => line(tester, 'trouble') != null);
    expect(
      line(tester, 'trouble'),
      'What is shown may be old. Error (io): not now',
    );
    expect(key('editor'), findsNothing);
    expect(shown(tester, 0), 'Shopping');
    await press(tester, 'read-again', () => line(tester, 'trouble') == null);
    expect(shown(tester, 0), 'typed');
  });
}
