// The widgets of the simple grid screen, without a device, on a stand-in for the core
// that can be made to wait and to fail: what is shown, that nothing typed is lost, that
// the document is saved without being asked, and what the screen says when it is not.
import 'package:flutter/material.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/grid/grid_screen.dart';
import 'package:wombat_app/grid/stage.dart';
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

  /// A test with the screen on a stand-in document. The stage is closed at the end,
  /// which stops the clock of its saving.
  void screenTest(
    String name,
    Future<void> Function(WidgetTester tester, Stage stage, FakeDocument fake)
    body, {
    Future<Session> Function(String path)? reopen,
  }) {
    testWidgets(name, (tester) async {
      final fake = FakeDocument();
      final stage = await Stage.open(
        await Session.over(fake),
        reopen: reopen ?? Session.open,
      );
      await tester.pumpWidget(MaterialApp(home: GridScreen(stage: stage)));
      await body(tester, stage, fake);
      await stage.close();
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
  });

  group('what was typed is not thrown away', () {
    screenTest('a tap on the way into a grid closes the editor with its text', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      await tester.enterText(key('editor'), 'typed');
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
      await tester.enterText(key('editor'), 'typed');
      await press(tester, 'cell-3');
      expect(fake.cells['a']!.text, 'typed');
      expect(fake.cells['d']!.text, 'four');
      expect(key('editor'), findsNothing);
      // And the next tap opens that cell.
      await press(tester, 'cell-3');
      expect(editor(tester), 'four');
      await press(tester, 'cancel');
    });

    screenTest('going up keeps the text and goes up', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'open-2');
      await press(tester, 'cell-0');
      await tester.enterText(key('editor'), 'typed');
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
      await tester.enterText(key('editor'), 'typed');
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
      await tester.enterText(key('editor'), 'typed');
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
      await tester.enterText(key('editor'), 'too much');
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
      await tester.enterText(key('editor'), 'first');
      final gate = holdFirst(fake, 'apply');
      await tester.testTextInput.receiveAction(TextInputAction.next);
      await tester.pump();
      // The core is still at work, and the user types on.
      await tester.enterText(key('editor'), 'first and more');
      gate.complete();
      await tester.pump();
      expect(fake.cells['a']!.text, 'first');
      expect(editor(tester), 'first and more');
      // Return again takes the rest and moves on to the cell below.
      await tester.testTextInput.receiveAction(TextInputAction.next);
      await tester.pump();
      expect(fake.cells['a']!.text, 'first and more');
      expect(editor(tester), 'three');
      await press(tester, 'cancel');
    });

    screenTest('keys typed while the tick is at work are not thrown away', (
      tester,
      stage,
      fake,
    ) async {
      await press(tester, 'cell-0');
      await tester.enterText(key('editor'), 'first');
      final gate = holdFirst(fake, 'apply');
      await press(tester, 'save');
      await tester.enterText(key('editor'), 'first and more');
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
        await tester.enterText(key('editor'), 'typed');
        await tester.testTextInput.receiveAction(TextInputAction.next);
        await tester.pump();
        expect(fake.cells['b']!.text, 'typed');
        expect(editor(tester), 'four');
        await tester.enterText(key('editor'), 'not this');
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
      await tester.enterText(key('editor'), '1');
      await press(tester, 'save');
      await tester.pump(const Duration(milliseconds: 600));
      await press(tester, 'cell-3');
      await tester.enterText(key('editor'), '4');
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

    // The system takes an app away step by step, and brings it back the same way.
    const away = [
      AppLifecycleState.inactive,
      AppLifecycleState.hidden,
      AppLifecycleState.paused,
      AppLifecycleState.detached,
    ];
    for (var steps = 1; steps <= away.length; steps++) {
      final state = away[steps - 1];
      screenTest(
        'at once when the app is ${state.name}, with what is being typed',
        (tester, stage, fake) async {
          await press(tester, 'cell-0');
          await tester.enterText(key('editor'), 'half a sentence');
          for (final step in away.take(steps)) {
            tester.binding.handleAppLifecycleStateChanged(step);
            await tester.pump();
          }
          expect(fake.cells['a']!.text, 'half a sentence');
          // Once for every step of the way: this one too.
          expect(fake.saves, steps);
          expect(fake.log.sublist(fake.log.length - 2), ['save', 'save.']);
          // The editor is still there for when the app comes back.
          expect(editor(tester), 'half a sentence');

          final back = state == AppLifecycleState.detached
              ? <AppLifecycleState>[]
              : away.take(steps - 1).toList().reversed;
          for (final step in back) {
            tester.binding.handleAppLifecycleStateChanged(step);
            await tester.pump();
          }
          final saves = fake.saves;
          tester.binding.handleAppLifecycleStateChanged(
            AppLifecycleState.resumed,
          );
          await tester.pump();
          expect(fake.saves, saves);
          await press(tester, 'cancel');
        },
      );
    }

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
      failFirst(
        fake,
        'save',
        const BridgeError(kind: ErrorKind.changedOnDisk, message: 'changed'),
      );
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
        failFirst(
          fake,
          'save',
          const BridgeError(kind: ErrorKind.changedOnDisk, message: 'changed'),
        );
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
      'a grid that could not be read again, with the way to try again',
      (tester, stage, fake) async {
        await press(tester, 'cell-0');
        await tester.enterText(key('editor'), 'typed');
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

  screenTest('after a fault a fixed sentence, and the document is left alone', (
    tester,
    stage,
    fake,
  ) async {
    await press(tester, 'cell-0');
    await tester.enterText(key('editor'), 'typed');
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
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pump(const Duration(seconds: 2));
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    expect(fake.log.length, calls);
  });
}
