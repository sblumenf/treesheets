// The widgets of the simple grid screen, without a device, on a stand-in for the core
// that can be made to wait and to fail: what is shown, that nothing typed is lost, that
// the document is saved without being asked, and what the screen says when it is not.
import 'dart:async';

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/grid/grid_screen.dart';
import 'package:wombat_app/grid/stage.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/main.dart' show Kept;
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'fake_document.dart';

void main() {
  Finder key(String name) => find.byKey(Key(name));

  /// The text widget of a cell of the stage, by its place in the grid.
  Text cellText(WidgetTester tester, int index) => tester.widget<Text>(
    find.descendant(of: key('cell-$index'), matching: find.byType(Text)),
  );

  String shown(WidgetTester tester, int index) => cellText(tester, index).data!;

  String title(WidgetTester tester) => tester.widget<Text>(key('title')).data!;

  String editor(WidgetTester tester) =>
      tester.widget<TextField>(key('editor')).controller!.text;

  Future<void> press(WidgetTester tester, String name) async {
    await tester.tap(key(name));
    await tester.pump();
  }

  /// Types into the editor as a user can: into whatever has the keyboard, which is
  /// the editor only while it has the focus. (`tester.enterText` would give it the
  /// focus first, and so pass where a user's keys go nowhere.)
  void type(WidgetTester tester, String text) =>
      tester.testTextInput.enterText(text);

  Future<void> pressReturn(WidgetTester tester) async {
    await tester.testTextInput.receiveAction(TextInputAction.next);
    await tester.pump();
  }

  bool focused(WidgetTester tester) =>
      tester.widget<TextField>(key('editor')).focusNode!.hasFocus;

  String? message(WidgetTester tester) => key('message').evaluate().isEmpty
      ? null
      : tester.widget<Text>(key('message')).data;

  Future<void> lifecycle(WidgetTester tester, AppLifecycleState state) async {
    tester.binding.handleAppLifecycleStateChanged(state);
    await tester.pump();
  }

  // The system takes an app away step by step, and brings it back the same way.
  const away = [
    AppLifecycleState.inactive,
    AppLifecycleState.hidden,
    AppLifecycleState.paused,
    AppLifecycleState.detached,
  ];

  /// The app goes out of sight, as far as `to`, and comes back to the front.
  Future<void> awayAndBack(WidgetTester tester, AppLifecycleState to) async {
    final steps = away.sublist(0, away.indexOf(to) + 1);
    for (final step in steps) {
      await lifecycle(tester, step);
    }
    for (final step in steps.reversed.skip(1)) {
      await lifecycle(tester, step);
    }
    await lifecycle(tester, AppLifecycleState.resumed);
  }

  /// The system's back, as the edge swipe or the back button of Android sends it.
  /// Leaving the app is asked of the system by a message, which is counted.
  Future<List<String>> back(WidgetTester tester, {int times = 1}) async {
    final left = <String>[];
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'SystemNavigator.pop') left.add(call.method);
        return null;
      },
    );
    for (var i = 0; i < times; i++) {
      await tester.binding.handlePopRoute();
    }
    await tester.pump();
    return left;
  }

  /// A test with the screen on a stand-in document, kept as the app keeps its own.
  /// The document is closed at the end, which stops the clock of its saving.
  void screenTest(
    String name,
    Future<void> Function(WidgetTester tester, Stage stage, FakeDocument fake)
    body, {
    Future<Session> Function(String path)? reopen,
    Future<void> Function(String path)? canBeRead,
  }) {
    testWidgets(name, (tester) async {
      final fake = FakeDocument();
      final keeper = Keeper(
        await Session.over(fake),
        reopen: reopen ?? Session.open,
        canBeRead: canBeRead ?? (_) async {},
      );
      final stage = await Stage.open(keeper);
      await tester.pumpWidget(
        MaterialApp(
          home: Kept(
            keeper: keeper,
            child: GridScreen(stage: stage),
          ),
        ),
      );
      await body(tester, stage, fake);
      if (keeper.ended == null) await keeper.close();
    });
  }

  const broken = BridgeError(kind: ErrorKind.io, message: 'the disk is gone');

  screenTest('a formula cell shows its result, and a failed one as an error', (
    tester,
    stage,
    fake,
  ) async {
    final colors = Theme.of(tester.element(key('grid'))).colorScheme;
    // Plain text.
    expect(shown(tester, 0), 'one');
    expect(cellText(tester, 0).style!.fontStyle, isNull);
    expect(cellText(tester, 0).style!.color, isNull);
    // A formula that failed: its error value, in italics and the error colour.
    expect(shown(tester, 1), '#NAME?');
    expect(cellText(tester, 1).style!.fontStyle, FontStyle.italic);
    expect(cellText(tester, 1).style!.color, colors.error);
    // A formula that worked: its result, in italics.
    await press(tester, 'open-2');
    expect(shown(tester, 1), '2');
    expect(cellText(tester, 1).style!.fontStyle, FontStyle.italic);
    expect(cellText(tester, 1).style!.color, isNull);
    // The editor has the formula.
    await press(tester, 'cell-1');
    expect(editor(tester), '=1+1');
    await press(tester, 'cancel');
  });

  screenTest('the cells touched last and the cell being edited are tinted', (
    tester,
    stage,
    fake,
  ) async {
    final colors = Theme.of(tester.element(key('grid'))).colorScheme;
    Color tint(int index) => tester
        .widget<ColoredBox>(
          find
              .ancestor(
                of: key('cell-$index'),
                matching: find.byType(ColoredBox),
              )
              .first,
        )
        .color;
    expect(tint(0), Colors.transparent);
    await press(tester, 'cell-0');
    expect(tint(0), colors.primaryContainer);
    expect(tint(3), Colors.transparent);
    type(tester, 'typed');
    await press(tester, 'save');
    expect(tint(0), colors.tertiaryContainer);
    expect(tint(3), Colors.transparent);
    // Being edited shows over having been touched.
    await press(tester, 'cell-0');
    expect(tint(0), colors.primaryContainer);
    await press(tester, 'cancel');
  });

  screenTest('names where the document has none', (tester, stage, fake) async {
    // A stage without a text, a level of the trail without one, a cell without a grid.
    fake.cells['root']!.text = '';
    fake.cells['c']!.text = '';
    await stage.refresh();
    await tester.pump();
    expect(title(tester), 'Wombat');
    expect(find.text('(empty)'), findsOneWidget);
    await press(tester, 'open-2');
    expect(find.text('(empty)'), findsOneWidget);
    expect(find.text('› (empty)'), findsOneWidget);
    expect(key('no-grid'), findsNothing);
    fake.cells['c']!
      ..xs = 0
      ..cells = [];
    await stage.refresh();
    await tester.pump();
    expect(find.text('This cell has no grid.'), findsOneWidget);
    expect(key('grid'), findsNothing);
  });

  screenTest('the trail: the level one is at cannot be pressed', (
    tester,
    stage,
    fake,
  ) async {
    TextButton level(int i) => tester.widget<TextButton>(key('trail-$i'));
    expect(level(0).onPressed, isNull);
    expect(tester.widget<IconButton>(key('up')).onPressed, isNull);
    await press(tester, 'open-2');
    expect(level(0).onPressed, isNotNull);
    expect(level(1).onPressed, isNull);
    expect(tester.widget<IconButton>(key('up')).onPressed, isNotNull);
    expect(find.text('› three'), findsOneWidget);
  });

  screenTest('a message goes when it is tapped, and with the next action', (
    tester,
    stage,
    fake,
  ) async {
    await press(tester, 'redo');
    expect(message(tester), 'Nothing to redo');
    await tester.tap(key('message'));
    await tester.pump();
    expect(message(tester), isNull);
    await press(tester, 'undo');
    expect(message(tester), 'Nothing to undo');
    // Gone at the tap, not when the action's turn comes.
    final gate = holdFirst(fake, 'grid');
    await press(tester, 'open-2');
    expect(message(tester), isNull);
    gate.complete();
    await tester.pump();
  });

  screenTest('the editor opens with the cursor at the end of the text', (
    tester,
    stage,
    fake,
  ) async {
    await press(tester, 'cell-0');
    final controller = tester.widget<TextField>(key('editor')).controller!;
    expect(controller.selection, const TextSelection.collapsed(offset: 3));
    expect(focused(tester), isTrue);
    await press(tester, 'cancel');
  });

  group('one action after the other', () {
    // The review's three: the screen judged "is an editor open?" when the finger came
    // down, and the stage moved away from under the editor it then opened.
    screenTest('a cell tapped, then Open, behind an undo that is held up', (
      tester,
      stage,
      fake,
    ) async {
      await stage.enterText('a', 'x');
      await tester.pump();
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.tap(key('cell-0'));
      await tester.tap(key('open-2'));
      await tester.pump();
      expect(key('editor'), findsNothing);
      gate.complete();
      await tester.pump();
      await tester.pump();
      // In their order: the undo, the editor on what it left, and the tap outside
      // the editor, which only closes it (UI_SPEC section 6).
      expect(key('editor'), findsNothing);
      expect(title(tester), 'Root');
      expect(fake.log.where((call) => call == 'apply').length, 2);
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('a cell tapped, then up, behind an undo that is held up', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'open-2');
      await stage.enterText('e', 'x');
      await tester.pump();
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.tap(key('cell-0'));
      await tester.tap(key('up'));
      await tester.pump();
      gate.complete();
      await tester.pump();
      await tester.pump();
      // The editor opened on "five", and up closed it, kept its text and went up.
      expect(key('editor'), findsNothing);
      expect(title(tester), 'Root');
      expect(fake.cells['e']!.text, 'five');
    });

    screenTest(
      'a cell tapped, then the trail, behind an undo that is held up',
      (tester, stage, fake) async {
        await press(tester, 'open-2');
        await stage.enterText('e', 'x');
        await tester.pump();
        final gate = holdFirst(fake, 'undo');
        await tester.tap(key('undo'));
        await tester.tap(key('cell-0'));
        await tester.tap(key('trail-0'));
        await tester.pump();
        gate.complete();
        await tester.pump();
        await tester.pump();
        expect(key('editor'), findsNothing);
        expect(title(tester), 'Root');
      },
    );

    screenTest(
      'a cell tapped, then Open, behind the save that came by itself',
      (tester, stage, fake) async {
        await stage.enterText('a', 'x');
        await tester.pump();
        final gate = holdFirst(fake, 'save');
        await tester.pump(const Duration(seconds: 1));
        expect(fake.log.last, 'save');
        await tester.tap(key('cell-0'));
        await tester.tap(key('open-2'));
        await tester.pump();
        gate.complete();
        await tester.pump();
        await tester.pump();
        expect(key('editor'), findsNothing);
        expect(title(tester), 'Root');
      },
    );

    screenTest('up, then a cell, while the way up is held: no editor on it', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'open-2');
      final gate = holdFirst(fake, 'grid');
      await tester.tap(key('up'));
      // Where "five" is now, "one" will be: the tap meant a cell that is gone from
      // the stage when its turn comes, and opens nothing.
      await tester.tap(key('cell-0'));
      await tester.pump();
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect(title(tester), 'Root');
      expect(key('editor'), findsNothing);
      expect(message(tester), isNull);
    });

    // The review's two: actions that were dropped without a word.
    screenTest('two taps on undo with the editor open undo twice', (
      tester,
      stage,
      fake,
    ) async {
      await stage.enterText('d', '4');
      await tester.pump();
      await press(tester, 'cell-0');
      type(tester, 'typed');
      final gate = holdFirst(fake, 'apply');
      await tester.tap(key('undo'));
      await tester.tap(key('undo'));
      await tester.pump();
      gate.complete();
      await tester.pump();
      await tester.pump();
      // The first takes back what was typed, the second the edit before it.
      expect((fake.cells['a']!.text, fake.cells['d']!.text), ('one', 'four'));
      expect(message(tester), isNull);
      expect(key('editor'), findsNothing);
    });

    for (final (name, target, tap) in [
      ('undo', 'cell-0', 'undo'),
      ('redo', 'cell-0', 'redo'),
      ('up', 'cell-0', 'up'),
      ('the trail', 'cell-0', 'trail-0'),
    ]) {
      screenTest('$name, and more typed while the text is entered: said', (
        tester,
        stage,
        fake,
      ) async {
        await press(tester, 'open-2');
        await press(tester, target);
        type(tester, 'first');
        final gate = holdFirst(fake, 'apply');
        await press(tester, tap);
        type(tester, 'first and more');
        gate.complete();
        await tester.pump();
        // What was sent is in the document, the rest is in the editor, and the
        // action was not done, which is said.
        expect(fake.cells['e']!.text, 'first');
        expect(editor(tester), 'first and more');
        expect(title(tester), 'three');
        expect(
          message(tester),
          'Not done: more was typed in the meantime. It is still in the editor.',
        );
        expect(fake.log.contains('undo'), isFalse);
        // The same again does it.
        await press(tester, tap);
        expect(
          fake.cells['e']!.text,
          tap == 'undo' ? 'first' : 'first and more',
        );
        expect(key('editor'), findsNothing);
        expect(message(tester), tap == 'redo' ? 'Nothing to redo' : null);
      });
    }

    screenTest('an error that no action expected is shown like the others', (
      tester,
      stage,
      fake,
    ) async {
      // No ordinary error: a mistake in the program, from the middle of an action.
      failFirst(fake, 'grid', StateError('a mistake'));
      await press(tester, 'open-2');
      expect(message(tester), 'Error: Bad state: a mistake');
      expect(tester.takeException(), isNull);
      // The queue of actions goes on.
      await press(tester, 'open-2');
      expect(title(tester), 'three');
    });
  });

  testWidgets('an error from where none is expected is shown too', (
    tester,
  ) async {
    final fake = FakeDocument();
    final keeper = _Mistaken(await Session.over(fake));
    final stage = await Stage.open(keeper);
    await tester.pumpWidget(MaterialApp(home: GridScreen(stage: stage)));
    await stage.enterText('a', 'mine');
    fake.fileChanged = true;
    await tester.pump(const Duration(seconds: 1));
    // Keeping mine never throws, and so nothing catches what it throws here.
    await press(tester, 'keep-mine');
    expect(message(tester), 'Error: Bad state: a mistake');
    expect(tester.takeException(), isNull);
    // With the screen gone there is no line to show it on: it is not swallowed.
    // It goes where the tap came from, as an error that nobody caught.
    final gate = keeper.gate = Completer<void>();
    final uncaught = <Object>[];
    await runZonedGuarded(
      () => tester.tap(key('keep-mine')),
      (error, _) => uncaught.add(error),
    );
    await tester.pumpWidget(const SizedBox());
    gate.complete();
    await tester.pump();
    expect(uncaught.single, isStateError);
    await keeper.letGo();
  });

  screenTest('after a fault the system\'s back leaves the app, once', (
    tester,
    stage,
    fake,
  ) async {
    failFirst(fake, 'undo', PanicException('at /home/builder/core.rs'));
    await stage.enterText('a', 'typed');
    await press(tester, 'undo');
    expect(key('ended'), findsOneWidget);
    // The system does the leaving; the screen does not ask for it a second time.
    final left = await back(tester);
    await tester.pump();
    await tester.pump();
    expect(left, ['SystemNavigator.pop']);
  });

  testWidgets('after a fault, back from a screen that is not the first one', (
    tester,
  ) async {
    final fake = FakeDocument();
    final keeper = Keeper(await Session.over(fake));
    final stage = await Stage.open(keeper);
    final navigator = GlobalKey<NavigatorState>();
    await tester.pumpWidget(
      MaterialApp(navigatorKey: navigator, home: const Text('below')),
    );
    navigator.currentState!.push(
      MaterialPageRoute<void>(builder: (_) => GridScreen(stage: stage)),
    );
    await tester.pumpAndSettle();
    failFirst(fake, 'undo', PanicException('at /home/builder/core.rs'));
    await press(tester, 'undo');
    expect(key('ended'), findsOneWidget);
    // The system takes the screen away, and that is all: the screen does not go on
    // to leave the app as from its root.
    final left = await back(tester);
    await tester.pumpAndSettle();
    expect(find.byType(GridScreen), findsNothing);
    expect(find.text('below'), findsOneWidget);
    expect(left, isEmpty);
  });

  group('the system\'s back', () {
    screenTest('goes up one level, and leaves the app only from the root', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'open-2');
      expect(title(tester), 'three');
      expect(await back(tester), isEmpty);
      expect(title(tester), 'Root');
      expect(await back(tester), ['SystemNavigator.pop']);
      expect(title(tester), 'Root');
    });

    screenTest('twice, while the way up is held: up, then out', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'open-2');
      final gate = holdFirst(fake, 'grid');
      final left = await back(tester, times: 2);
      // Not out of the app from a level that is only about to be left.
      expect(left, isEmpty);
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect(title(tester), 'Root');
      expect(left, ['SystemNavigator.pop']);
    });

    screenTest('keeps what is being typed, as the arrow does', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'open-2');
      await press(tester, 'cell-0');
      type(tester, 'typed');
      expect(await back(tester), isEmpty);
      expect(fake.cells['e']!.text, 'typed');
      expect(key('editor'), findsNothing);
      expect(title(tester), 'Root');
      // Text that the core refuses: the editor stays, and the app is not left.
      await press(tester, 'cell-0');
      type(tester, 'too much');
      failFirst(fake, 'apply', broken);
      expect(await back(tester), isEmpty);
      expect(editor(tester), 'too much');
      expect(message(tester), 'Error (io): the disk is gone');
      await press(tester, 'cancel');
    });
  });

  group('taps that come close together', () {
    screenTest('two taps on the way into a grid go in once', (
      tester,
      stage,
      fake,
    ) async {
      // The core is slow to read the grid, so the second tap comes while the first
      // is still at work.
      final gate = holdFirst(fake, 'grid');
      await tester.tap(key('open-2'));
      await tester.tap(key('open-2'));
      await tester.pump();
      expect(title(tester), 'Root');
      gate.complete();
      await tester.pump();
      expect(title(tester), 'three');
      expect(stage.trail.length, 2);
      expect(key('message'), findsNothing);
    });

    screenTest('two taps on undo undo twice', (tester, stage, fake) async {
      await stage.enterText('a', '1');
      await stage.enterText('d', '4');
      await tester.pump();
      // The second tap comes while the first undo is still at work: it is not dropped.
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.tap(key('undo'));
      await tester.pump();
      expect((shown(tester, 0), shown(tester, 3)), ('1', '4'));
      gate.complete();
      await tester.pump();
      expect((shown(tester, 0), shown(tester, 3)), ('one', 'four'));
    });

    screenTest('a cell tapped while an undo runs opens on what the undo left', (
      tester,
      stage,
      fake,
    ) async {
      await stage.enterText('a', 'typed');
      await tester.pump();
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.tap(key('cell-0'));
      await tester.pump();
      // Nothing yet: the editor must not open on text that is about to change.
      expect(key('editor'), findsNothing);
      gate.complete();
      await tester.pump();
      expect(editor(tester), 'one');
      await press(tester, 'cancel');
    });

    screenTest('an action that ends after the screen is gone is not shown', (
      tester,
      stage,
      fake,
    ) async {
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.pumpWidget(const MaterialApp(home: SizedBox()));
      gate.complete();
      await tester.pump();
      // "Nothing to undo" had nowhere to go, and nothing was thrown for that.
      expect(tester.takeException(), isNull);
    });

    screenTest('actions still waiting when the screen goes are not done', (
      tester,
      stage,
      fake,
    ) async {
      await stage.enterText('a', '1');
      await stage.enterText('d', '4');
      await tester.pump();
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.tap(key('undo'));
      await tester.tap(key('open-2'));
      await tester.pumpWidget(const MaterialApp(home: SizedBox()));
      gate.complete();
      await tester.pump();
      await tester.pump();
      // The one that was at work ended; the ones behind it never began.
      expect((fake.cells['a']!.text, fake.cells['d']!.text), ('1', 'four'));
      expect(stage.atRoot, isTrue);
      expect(tester.takeException(), isNull);
    });

    for (final (name, act) in [
      ('the tick', (WidgetTester tester) => tester.tap(key('save'))),
      (
        'Return',
        (WidgetTester tester) =>
            tester.testTextInput.receiveAction(TextInputAction.next),
      ),
      ('undo', (WidgetTester tester) => tester.tap(key('undo'))),
    ]) {
      screenTest(
        'the screen goes while $name enters the text: no more is done',
        (tester, stage, fake) async {
          await press(tester, 'cell-0');
          type(tester, 'typed');
          final gate = holdFirst(fake, 'apply');
          await act(tester);
          await tester.pump();
          await tester.pumpWidget(const MaterialApp(home: SizedBox()));
          gate.complete();
          await tester.pump();
          await tester.pump();
          // The text is in the document; the editor that is gone is not closed or
          // moved, and what was typed is not undone behind the user's back.
          expect(fake.cells['a']!.text, 'typed');
          expect(fake.log.contains('undo'), isFalse);
          expect(tester.takeException(), isNull);
        },
      );
    }
  });

  group('what was typed is not thrown away', () {
    screenTest('a tap on the way into a grid closes the editor with its text', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'typed');
      await press(tester, 'open-2');
      expect(fake.cells['a']!.text, 'typed');
      expect(key('editor'), findsNothing);
      // UI_SPEC section 6: a tap outside the editor only closes it.
      expect(title(tester), 'Root');
      expect(shown(tester, 0), 'typed');
    });

    screenTest('a tap on another cell closes the editor with its text', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'typed');
      await press(tester, 'cell-3');
      expect(fake.cells['a']!.text, 'typed');
      expect(fake.cells['d']!.text, 'four');
      expect(key('editor'), findsNothing);
      // And the next tap opens that cell.
      await press(tester, 'cell-3');
      expect(editor(tester), 'four');
      await press(tester, 'cancel');
    });

    for (final target in ['cell-3', 'open-2']) {
      screenTest('a tap on $target with text that the core refuses: it stays', (
        tester,
        stage,
        fake,
      ) async {
        await press(tester, 'cell-0');
        type(tester, 'too much');
        failFirst(fake, 'apply', broken);
        await press(tester, target);
        expect(editor(tester), 'too much');
        expect(message(tester), 'Error (io): the disk is gone');
        expect(title(tester), 'Root');
        // More typed while the text is entered: that stays too, without a word.
        final gate = holdFirst(fake, 'apply');
        await press(tester, target);
        type(tester, 'too much, and more');
        gate.complete();
        await tester.pump();
        expect(fake.cells['a']!.text, 'too much');
        expect(editor(tester), 'too much, and more');
        expect(message(tester), isNull);
        await press(tester, 'cancel');
      });
    }

    screenTest('going up keeps the text and goes up', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'open-2');
      await press(tester, 'cell-0');
      type(tester, 'typed');
      await press(tester, 'up');
      expect(fake.cells['e']!.text, 'typed');
      expect(key('editor'), findsNothing);
      expect(title(tester), 'Root');
    });

    screenTest('the trail keeps the text and goes there', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'open-2');
      await press(tester, 'cell-0');
      type(tester, 'typed');
      await press(tester, 'trail-0');
      expect(fake.cells['e']!.text, 'typed');
      expect(key('editor'), findsNothing);
      expect(title(tester), 'Root');
    });

    screenTest('undo keeps the text first, and then undoes that', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'typed');
      await press(tester, 'undo');
      expect(key('editor'), findsNothing);
      expect(shown(tester, 0), 'one');
      await press(tester, 'redo');
      expect(shown(tester, 0), 'typed');
    });

    screenTest('text that the core refuses stays in the editor', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'open-2');
      await press(tester, 'cell-0');
      type(tester, 'too much');
      failFirst(
        fake,
        'apply',
        const BridgeError(kind: ErrorKind.limit, message: 'too long'),
      );
      await press(tester, 'up');
      expect(editor(tester), 'too much');
      expect(title(tester), 'three');
      expect(
        tester.widget<Text>(key('message')).data,
        'Error (limit): too long',
      );
      await press(tester, 'cancel');
    });

    screenTest('keys typed while Return is at work are not overwritten', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'first');
      final gate = holdFirst(fake, 'apply');
      await pressReturn(tester);
      // The core is still at work, and the user types on.
      type(tester, 'first and more');
      gate.complete();
      await tester.pump();
      expect(fake.cells['a']!.text, 'first');
      expect(editor(tester), 'first and more');
      // Return again takes the rest and moves on to the cell below.
      await pressReturn(tester);
      expect(fake.cells['a']!.text, 'first and more');
      expect(editor(tester), 'three');
      await press(tester, 'cancel');
    });

    // The review's: Return took the focus away, so that keys typed after it went
    // nowhere; the test above passed because it gave the focus back itself.
    screenTest('while Return is at work the editor keeps the keyboard', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'first');
      final gate = holdFirst(fake, 'apply');
      await pressReturn(tester);
      expect(focused(tester), isTrue);
      expect(tester.testTextInput.isVisible, isTrue);
      gate.complete();
      await tester.pump();
      expect(editor(tester), 'three');
      expect(focused(tester), isTrue);
      await press(tester, 'cancel');
    });

    screenTest(
      'Return on text that the core refuses leaves the editor in use',
      (tester, stage, fake) async {
        await press(tester, 'cell-0');
        type(tester, 'too much');
        failFirst(
          fake,
          'apply',
          const BridgeError(kind: ErrorKind.limit, message: 'too long'),
        );
        await pressReturn(tester);
        await tester.pump();
        expect(message(tester), 'Error (limit): too long');
        expect(editor(tester), 'too much');
        expect(focused(tester), isTrue);
        // And the user types on, and Return then takes it.
        type(tester, 'less');
        await pressReturn(tester);
        expect(fake.cells['a']!.text, 'less');
        expect(editor(tester), 'three');
        await press(tester, 'cancel');
      },
    );

    screenTest('Return in the last row enters the text and closes the editor', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-3');
      type(tester, 'last');
      await pressReturn(tester);
      expect(tester.takeException(), isNull);
      expect(fake.cells['d']!.text, 'last');
      expect(key('editor'), findsNothing);
      // The last row of one column, too.
      await press(tester, 'open-2');
      await press(tester, 'cell-1');
      await pressReturn(tester);
      expect(tester.takeException(), isNull);
      expect(key('editor'), findsNothing);
    });

    screenTest('Return on a cell that an undo took off the stage closes it', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      // The cell is gone from the stage behind the editor's back.
      fake.cells['root']!.cells = ['x', 'b', 'c', 'd'];
      fake.cells['x'] = FakeCell('in its place');
      await pressReturn(tester);
      expect(tester.takeException(), isNull);
      // Not on to "the cell below" of a cell that is not there.
      expect(key('editor'), findsNothing);
    });

    screenTest('keys typed while the tick is at work are not thrown away', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'first');
      final gate = holdFirst(fake, 'apply');
      await press(tester, 'save');
      type(tester, 'first and more');
      gate.complete();
      await tester.pump();
      expect(editor(tester), 'first and more');
      await press(tester, 'save');
      expect(key('editor'), findsNothing);
      expect(fake.cells['a']!.text, 'first and more');
    });

    screenTest(
      'Return saves and moves to the cell below; the cross does not save',
      (tester, stage, fake) async {
        await press(tester, 'cell-1');
        type(tester, 'typed');
        await pressReturn(tester);
        expect(fake.cells['b']!.text, 'typed');
        expect(editor(tester), 'four');
        type(tester, 'not this');
        await press(tester, 'cancel');
        expect(key('editor'), findsNothing);
        expect(fake.cells['d']!.text, 'four');
      },
    );
  });

  group('the document is saved without being asked', () {
    screenTest('a second after the first change, however many follow', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, '1');
      await press(tester, 'save');
      await tester.pump(const Duration(milliseconds: 600));
      await press(tester, 'cell-3');
      type(tester, '4');
      await press(tester, 'save');
      expect(fake.saves, 0);
      // A second after the first change, not after the last.
      await tester.pump(const Duration(milliseconds: 400));
      expect(fake.saves, 1);
      expect(fake.saved, isTrue);
      // Nothing more without a change.
      await tester.pump(const Duration(seconds: 5));
      expect(fake.saves, 1);
      await press(tester, 'undo');
      await tester.pump(const Duration(seconds: 1));
      expect(fake.saves, 2);
    });

    screenTest(
      'when the app is no longer in front, but not what is being typed',
      (tester, stage, fake) async {
        await stage.enterText('d', 'entered');
        await press(tester, 'cell-0');
        type(tester, 'half a sentence');
        // A window that loses the focus, a dialog of the system: not leaving.
        await lifecycle(tester, AppLifecycleState.inactive);
        expect(fake.saves, 1);
        expect(fake.log.contains('apply'), isTrue);
        expect(fake.cells['a']!.text, 'one');
        expect(fake.log.where((call) => call == 'apply').length, 1);
        await lifecycle(tester, AppLifecycleState.resumed);
        expect(editor(tester), 'half a sentence');
        expect(focused(tester), isTrue);
        // So the cross is still "not this", with nothing to take back.
        await press(tester, 'cancel');
        expect(fake.cells['a']!.text, 'one');
        expect(fake.log.contains('undo'), isFalse);
      },
    );

    for (var steps = 2; steps <= away.length; steps++) {
      final state = away[steps - 1];
      screenTest(
        'at once when the app is ${state.name}, with what is being typed',
        (tester, stage, fake) async {
          await press(tester, 'cell-0');
          type(tester, 'half a sentence');
          for (final step in away.take(steps)) {
            await lifecycle(tester, step);
          }
          expect(fake.cells['a']!.text, 'half a sentence');
          // One editing and written once, however many steps the way has: the
          // text is held again at each, which changes nothing, and the saves
          // after it find nothing to write.
          expect(fake.log.where((call) => call == 'beginGroup').length, 1);
          expect(fake.saves, 1);
          expect(fake.saved, isTrue);
          expect(
            fake.log,
            containsAllInOrder(['beginGroup', 'retype.', 'apply.', 'save']),
          );
          // Held, not entered: the editing goes on, for the tick or the cross.
          expect(fake.log.contains('endGroup'), isFalse);
          expect(stage.keeper.editing!.held, isTrue);
          // The editor is still there for when the app comes back.
          expect(editor(tester), 'half a sentence');

          final back = state == AppLifecycleState.detached
              ? <AppLifecycleState>[]
              : away.take(steps - 1).toList().reversed;
          for (final step in back) {
            await lifecycle(tester, step);
          }
          final calls = fake.log.where((call) => call == 'save').length;
          await lifecycle(tester, AppLifecycleState.resumed);
          expect(fake.log.where((call) => call == 'save').length, calls);
          await press(tester, 'cancel');
        },
      );
    }

    // The review's two: after the app had been away, the cross kept the text, and
    // one editing was two steps of the undo history.
    screenTest(
      'the cross after the app was out of sight still means "not this"',
      (tester, stage, fake) async {
        // Something that can be redone, and nothing that can be undone.
        await stage.enterText('d', 'undone');
        await stage.undo();
        await stage.keeper.save();
        await press(tester, 'cell-0');
        type(tester, 'not this');
        await lifecycle(tester, AppLifecycleState.inactive);
        await lifecycle(tester, AppLifecycleState.hidden);
        expect((fake.cells['a']!.text, fake.saves), ('not this', 2));
        expect(shown(tester, 0), 'not this');
        await lifecycle(tester, AppLifecycleState.inactive);
        await lifecycle(tester, AppLifecycleState.resumed);
        await press(tester, 'cancel');
        expect(key('editor'), findsNothing);
        expect(fake.cells['a']!.text, 'one');
        expect(shown(tester, 0), 'one');
        expect(stage.keeper.editing, isNull);
        // The file follows a second later.
        await tester.pump(const Duration(seconds: 1));
        expect((fake.saves, fake.saved), (3, true));
        // Nothing of it is left to undo, and Redo does not bring it back: what
        // could be redone before the editing still can.
        await press(tester, 'undo');
        expect(message(tester), 'Nothing to undo');
        await press(tester, 'redo');
        expect(
          (fake.cells['a']!.text, fake.cells['d']!.text),
          ('one', 'undone'),
        );
        await press(tester, 'redo');
        expect(message(tester), 'Nothing to redo');
      },
    );

    screenTest('one editing, the app out of sight in the middle: one undo', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'half');
      await awayAndBack(tester, AppLifecycleState.hidden);
      type(tester, 'half and more');
      // Away a second time, with more: still not said yes to.
      await awayAndBack(tester, AppLifecycleState.paused);
      expect(fake.cells['a']!.text, 'half and more');
      type(tester, 'half and whole');
      await press(tester, 'save');
      expect(fake.cells['a']!.text, 'half and whole');
      await press(tester, 'undo');
      expect(fake.cells['a']!.text, 'one');
      await press(tester, 'undo');
      expect(message(tester), 'Nothing to undo');
    });

    screenTest('the tick on the very text that went in when the app left', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'whole');
      await awayAndBack(tester, AppLifecycleState.hidden);
      final saves = fake.saves;
      await press(tester, 'save');
      // The document has it: it is said yes to, and that is the end of the
      // editing. Nothing changed, so nothing is written again.
      expect(fake.log.sublist(fake.log.length - 2), ['grid', 'grid.']);
      expect(fake.log, containsAllInOrder(['save.', 'endGroup.']));
      expect(key('editor'), findsNothing);
      expect(stage.keeper.editing, isNull);
      await tester.pump(const Duration(minutes: 1));
      expect(fake.saves, saves);
      // So a cross on the next editing of the cell takes nothing back.
      await press(tester, 'cell-0');
      await press(tester, 'cancel');
      expect(fake.cells['a']!.text, 'whole');
      // And the whole of it is one step.
      await press(tester, 'undo');
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('away with the text the cell has: nothing to take back', (
      tester,
      stage,
      fake,
    ) async {
      await stage.enterText('d', 'before');
      await press(tester, 'cell-0');
      await awayAndBack(tester, AppLifecycleState.hidden);
      await press(tester, 'cancel');
      // The cross did not undo the edit before this editing.
      expect(fake.cells['d']!.text, 'before');
      expect(fake.log.contains('undo'), isFalse);
    });

    screenTest(
      'text that the core refuses when the app leaves is kept and said',
      (tester, stage, fake) async {
        await stage.enterText('d', 'entered');
        await press(tester, 'cell-0');
        type(tester, 'too long');
        failFirst(
          fake,
          'apply',
          const BridgeError(kind: ErrorKind.limit, message: 'too long'),
        );
        await lifecycle(tester, AppLifecycleState.inactive);
        await lifecycle(tester, AppLifecycleState.hidden);
        expect(editor(tester), 'too long');
        expect(message(tester), 'Error (limit): too long');
        // What was in the document is saved all the same.
        expect((fake.saves, fake.saved), (1, true));
        // And the cross has nothing to take back.
        await press(tester, 'cancel');
        expect(fake.cells['d']!.text, 'entered');
      },
    );

    screenTest('what one editing left unconfirmed is not the next one\'s', (
      tester,
      stage,
      fake,
    ) async {
      await stage.enterText('d', 'before');
      await press(tester, 'cell-0');
      type(tester, 'half');
      await awayAndBack(tester, AppLifecycleState.hidden);
      // Return says yes to it and moves on; the cross on the next cell then has
      // nothing to take back.
      await pressReturn(tester);
      expect(editor(tester), 'three');
      await press(tester, 'cancel');
      expect(fake.cells['a']!.text, 'half');
      expect(fake.log.contains('cancelGroup'), isFalse);
      // And after a cross that did take something back, neither has the next.
      await press(tester, 'cell-1');
      type(tester, 'not this');
      await awayAndBack(tester, AppLifecycleState.hidden);
      expect(fake.cells['b']!.text, 'not this');
      await press(tester, 'cancel');
      await press(tester, 'cell-1');
      await press(tester, 'cancel');
      expect(fake.log.where((call) => call == 'cancelGroup').length, 1);
      expect(fake.log.contains('undo'), isFalse);
      expect(
        (fake.cells['a']!.text, fake.cells['b']!.text, fake.cells['d']!.text),
        ('half', '=X', 'before'),
      );
    });

    screenTest('what is held is nobody else\'s to take back or to add to', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'not this');
      await awayAndBack(tester, AppLifecycleState.hidden);
      // Behind the screen, as a second screen could: refused, and loudly.
      await expectLater(stage.undo(), throwsStateError);
      await expectLater(stage.enterText('d', 'theirs'), throwsStateError);
      expect(
        (fake.cells['a']!.text, fake.cells['d']!.text),
        ('not this', 'four'),
      );
      await press(tester, 'cancel');
      expect(key('editor'), findsNothing);
      expect(message(tester), isNull);
      expect(fake.cells['a']!.text, 'one');
      // And free again after the cross.
      await stage.enterText('d', 'theirs');
    });

    screenTest('text entered while more was typed, then the app away, then the '
        'cross: what was held of the rest is taken back', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'first');
      // The tick, and more typed while the core is at work on it.
      final gate = holdFirst(fake, 'retype');
      await press(tester, 'save');
      type(tester, 'first and more');
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect(
        (fake.cells['a']!.text, editor(tester)),
        ('first', 'first and more'),
      );
      // That editing is over; the rest is held by one of its own.
      expect(stage.keeper.editing, isNull);
      await awayAndBack(tester, AppLifecycleState.hidden);
      expect(fake.cells['a']!.text, 'first and more');
      expect(stage.keeper.editing!.held, isTrue);
      await press(tester, 'cancel');
      expect(key('editor'), findsNothing);
      // Back to what was said yes to, and nothing is left open.
      expect((fake.cells['a']!.text, stage.keeper.editing), ('first', null));
      await press(tester, 'undo');
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('the editor opens on the cell with one read of it', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'typed');
      await press(tester, 'save');
      // The editing that was opened for the editor is the one that is entered.
      expect(fake.log.where((call) => call == 'cell').length, 1);
      expect(fake.log.where((call) => call == 'beginGroup').length, 1);
    });

    screenTest('a cross that cannot take the text back leaves the editor', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'not this');
      await awayAndBack(tester, AppLifecycleState.hidden);
      failFirst(fake, 'cancelGroup', broken);
      await press(tester, 'cancel');
      expect(message(tester), 'Error (io): the disk is gone');
      expect(editor(tester), 'not this');
      expect(stage.keeper.editing!.held, isTrue);
      await press(tester, 'cancel');
      expect(key('editor'), findsNothing);
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('a screen that goes with its editor open enters the text', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'left in the editor');
      await tester.pumpWidget(const SizedBox());
      // As the app does when its screen goes: the close waits for the text.
      await stage.keeper.close();
      expect(fake.cells['a']!.text, 'left in the editor');
      expect(fake.log, containsAllInOrder(['apply.', 'close']));
      expect((fake.saves, fake.closed), (1, true));
    });

    screenTest('a screen that goes after the app was away enters one step', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'half');
      await awayAndBack(tester, AppLifecycleState.hidden);
      type(tester, 'half and whole');
      await tester.pumpWidget(const SizedBox());
      await tester.pump();
      expect(fake.cells['a']!.text, 'half and whole');
      await stage.undo();
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('a screen that has gone is no longer asked or told', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      type(tester, 'the same');
      await awayAndBack(tester, AppLifecycleState.hidden);
      await tester.pumpWidget(const SizedBox());
      // Its text was in the document already; going, it ends the editing, so that
      // no group stays open behind it.
      await tester.pump();
      expect(fake.log.sublist(fake.log.length - 4), [
        'endGroup',
        'endGroup.',
        'grid',
        'grid.',
      ]);
      expect(stage.keeper.editing, isNull);
      final calls = fake.log.length;
      // What watched the app's lifecycle for it went with it.
      await awayAndBack(tester, AppLifecycleState.paused);
      expect(fake.log.length, calls);
      // Neither a change of the stage nor the app being put away reaches it.
      await stage.dive('c');
      await stage.keeper.putAway();
      expect(tester.takeException(), isNull);
      // Held once and entered once, each of which settles what the cell is.
      expect(fake.log.where((call) => call == 'apply').length, 2);
      expect(fake.log.where((call) => call == 'retype').length, 2);
    });

    screenTest('there is no Save to forget', (tester, stage, fake) async {
      expect(find.text('Save'), findsNothing);
      // With nothing else to put in it, there is no menu.
      expect(key('menu'), findsNothing);
    });
  });

  group('a save that did not work is shown', () {
    screenTest('with the way to try again', (tester, stage, fake) async {
      await stage.enterText('a', 'typed');
      failFirst(fake, 'save', broken);
      await tester.pump(const Duration(seconds: 1));
      expect(
        tester.widget<Text>(key('trouble')).data,
        'Not saved. Error (io): the disk is gone',
      );
      expect(fake.saves, 0);
      // It stays while other things are done.
      await press(tester, 'undo');
      expect(key('trouble'), findsOneWidget);
      await press(tester, 'save-again');
      expect(key('trouble'), findsNothing);
      expect(fake.saves, 1);
    });

    screenTest('a file changed by another program: keep mine', (
      tester,
      stage,
      fake,
    ) async {
      await stage.enterText('a', 'mine');
      fake.fileChanged = true;
      await tester.pump(const Duration(seconds: 1));
      expect(
        tester.widget<Text>(key('trouble')).data,
        'Not saved: another program changed the file.',
      );
      expect(key('save-again'), findsNothing);
      await press(tester, 'keep-mine');
      expect((fake.overwrites, fake.saves), (1, 0));
      expect(key('trouble'), findsNothing);
      expect(shown(tester, 0), 'mine');
    });

    final theirs = FakeDocument()..cells['a']!.text = 'theirs';
    screenTest(
      'a file changed by another program: open theirs',
      (tester, stage, fake) async {
        await press(tester, 'open-2');
        await stage.enterText('e', 'mine');
        fake.fileChanged = true;
        await tester.pump(const Duration(seconds: 1));
        // Also what is in the editor goes: the choice is the other file.
        await press(tester, 'cell-0');
        await press(tester, 'open-theirs');
        expect(key('editor'), findsNothing);
        expect(key('trouble'), findsNothing);
        expect(fake.disposed, isTrue);
        expect((fake.overwrites, fake.saves), (0, 0));
        expect(title(tester), 'Root');
        expect(shown(tester, 0), 'theirs');
      },
      reopen: (path) {
        expect(path, '/fake.wombat');
        return Session.over(theirs);
      },
    );

    screenTest(
      'open theirs when theirs cannot be read: said, and nothing given up',
      (tester, stage, fake) async {
        await stage.enterText('d', 'mine');
        fake.fileChanged = true;
        await tester.pump(const Duration(seconds: 1));
        await press(tester, 'cell-0');
        type(tester, 'being typed');
        await press(tester, 'open-theirs');
        expect(
          message(tester),
          'The file as the other program left it cannot be opened, so this '
          'document is still open, and not saved. '
          'Error (io): the disk is gone',
        );
        // The choice is still there, and so are the document and the editor.
        expect(key('open-theirs'), findsOneWidget);
        expect(fake.disposed, isFalse);
        expect(editor(tester), 'being typed');
        await press(tester, 'keep-mine');
        expect(key('trouble'), findsNothing);
        expect(fake.overwrites, 1);
        await press(tester, 'cancel');
      },
      canBeRead: (_) => throw broken,
    );

    // Read by the fourth review: a look that could not be made at all was said to
    // be the other program's file being unreadable.
    screenTest(
      'open theirs when the file cannot even be looked at: said as that',
      (tester, stage, fake) async {
        await stage.enterText('d', 'mine');
        fake.fileChanged = true;
        await tester.pump(const Duration(seconds: 1));
        await press(tester, 'open-theirs');
        expect(
          message(tester),
          'The other program\'s file was not opened, so this document is still '
          'open, and not saved. '
          'Error: FileSystemException: no room, path = \'/tmp\'',
        );
        expect(fake.disposed, isFalse);
        await press(tester, 'keep-mine');
      },
      canBeRead: (_) => throw const FileSystemException('no room', '/tmp'),
    );

    screenTest(
      'open theirs when reading theirs runs into a fault: this one is whole',
      (tester, stage, fake) async {
        await stage.enterText('d', 'mine');
        fake.fileChanged = true;
        await tester.pump(const Duration(seconds: 1));
        await press(tester, 'open-theirs');
        // Not the sentence that says this document cannot be used any more.
        expect(message(tester), endsWith('Reading it ran into a fault.'));
        expect(find.textContaining('builder'), findsNothing);
        expect(key('ended'), findsNothing);
        await press(tester, 'keep-mine');
      },
      canBeRead: (_) => throw PanicException('at /home/builder/core.rs'),
    );

    screenTest(
      'a grid that could not be read again, with the way to try again',
      (tester, stage, fake) async {
        await press(tester, 'cell-0');
        type(tester, 'typed');
        failFirst(fake, 'grid', broken);
        await press(tester, 'save');
        // The edit was made, so the editor is closed; what is shown is old.
        expect(key('editor'), findsNothing);
        expect(fake.cells['a']!.text, 'typed');
        expect(shown(tester, 0), 'one');
        expect(
          tester.widget<Text>(key('trouble')).data,
          'What is shown may be old. Error (io): the disk is gone',
        );
        await press(tester, 'read-again');
        expect(key('trouble'), findsNothing);
        expect(shown(tester, 0), 'typed');
      },
    );
  });

  testWidgets('a screen given another stage moves to it, text entered', (
    tester,
  ) async {
    Future<(Keeper, Stage, FakeDocument)> open(String a) async {
      final fake = FakeDocument()..cells['a']!.text = a;
      final keeper = Keeper(await Session.over(fake));
      return (keeper, await Stage.open(keeper), fake);
    }

    final (keeper1, stage1, fake1) = await open('one');
    final (keeper2, stage2, fake2) = await open('the other');
    Widget on(Stage stage) => MaterialApp(home: GridScreen(stage: stage));
    await tester.pumpWidget(on(stage1));
    await press(tester, 'cell-0');
    type(tester, 'typed on the first');
    await tester.pumpWidget(on(stage2));
    await tester.pump();
    expect(fake1.cells['a']!.text, 'typed on the first');
    expect(key('editor'), findsNothing);
    expect(shown(tester, 0), 'the other');
    // It follows the new stage and not the old one.
    await stage2.enterText('a', 'changed');
    await tester.pump();
    expect(shown(tester, 0), 'changed');
    await stage1.dive('c');
    await tester.pump();
    expect(title(tester), 'Root');
    // The same stage given again changes nothing.
    await press(tester, 'cell-0');
    await tester.pumpWidget(on(stage2));
    expect(editor(tester), 'changed');
    // The keeper of the stage it left no longer asks it for what is being typed.
    type(tester, 'typed on the second');
    await keeper1.putAway();
    expect(fake2.cells['a']!.text, 'changed');
    await keeper2.putAway();
    expect(fake2.cells['a']!.text, 'typed on the second');
    await press(tester, 'cancel');
    expect(fake2.cells['a']!.text, 'changed');
    await keeper1.close();
    await keeper2.close();
  });

  screenTest('after a fault a fixed sentence, and the document is left alone', (
    tester,
    stage,
    fake,
  ) async {
    await press(tester, 'cell-0');
    type(tester, 'typed');
    failFirst(
      fake,
      'apply',
      PanicException('panicked at /home/builder/wombat/core.rs:12:3'),
    );
    await press(tester, 'save');
    expect(tester.widget<Text>(key('ended')).data, faultSentence);
    expect(find.textContaining('builder'), findsNothing);
    expect(find.textContaining('PanicException'), findsNothing);
    expect(fake.disposed, isTrue);
    // Nothing is left to press that would reach the document.
    expect(key('grid'), findsNothing);
    expect(key('editor'), findsNothing);
    expect(key('up'), findsNothing);
    expect(tester.widget<IconButton>(key('undo')).onPressed, isNull);
    expect(tester.widget<IconButton>(key('redo')).onPressed, isNull);
    final calls = fake.log.length;
    await awayAndBack(tester, AppLifecycleState.paused);
    await tester.pump(const Duration(seconds: 2));
    expect(fake.log.length, calls);
    // The system's back is the system's again, which leaves the app: there is
    // nothing to go up in.
    final left = await back(tester);
    await tester.pump();
    await tester.pump();
    expect(left, ['SystemNavigator.pop']);
    expect(fake.log.length, calls);
  });
}

/// A keeper with a mistake in it: keeping mine throws.
class _Mistaken extends Keeper {
  _Mistaken(super.doc);

  /// What the mistake waits for, if anything.
  Completer<void>? gate;

  @override
  Future<void> keepMine() async {
    await gate?.future;
    throw StateError('a mistake');
  }
}
