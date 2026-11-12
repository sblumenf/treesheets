// The keeper of a document: saving by itself, trying again, the app put away, another
// program's file, closing. On a stand-in for the core where time or failures must be
// made, with the clock of the test, and on the real core for what reads a file.
import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'fake_document.dart';
import 'support.dart';

void main() {
  late Directory folder;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-keeper');
  });

  tearDownAll(() => folder.delete(recursive: true));

  const broken = BridgeError(kind: ErrorKind.io, message: 'the disk is gone');
  const edit = Command.enterText(cell: 'a', text: 'typed');

  Matcher refusedAs(ErrorKind kind) =>
      throwsA(isA<BridgeError>().having((e) => e.kind, 'kind', kind));

  ErrorKind? kindOf(Object? error) => error is BridgeError ? error.kind : null;

  int calls(FakeDocument fake, String call) =>
      fake.log.where((name) => name == call).length;

  /// A test with a keeper on the stand-in, on the clock of the test.
  void keeperTest(
    String name,
    Future<void> Function(WidgetTester tester, Keeper keeper, FakeDocument fake)
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
      await body(tester, keeper, fake);
      // Also stops the clock; a test that left the document unsavable lets go.
      await keeper.close().catchError((Object _) => keeper.letGo());
    });
  }

  group('saving by itself', () {
    keeperTest('a second after the first change, not after the last', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      await tester.pump(const Duration(milliseconds: 600));
      await keeper.undo();
      await tester.pump(const Duration(milliseconds: 399));
      expect(fake.saves, 0);
      await tester.pump(const Duration(milliseconds: 1));
      expect((fake.saves, fake.saved), (1, true));
      await keeper.redo();
      await tester.pump(const Duration(seconds: 1));
      expect(fake.saves, 2);
      // Nothing more without a change, and none for an undo or a redo that found
      // nothing to do.
      final before = calls(fake, 'save');
      await keeper.redo();
      await tester.pump(const Duration(minutes: 5));
      expect(calls(fake, 'save'), before);
      await keeper.undo();
      await keeper.undo();
      await tester.pump(const Duration(seconds: 1));
      expect(calls(fake, 'save'), before + 1);
      await keeper.undo();
      await tester.pump(const Duration(minutes: 5));
      expect(calls(fake, 'save'), before + 1);
    });

    // The review's: after one failure the clock was not started again.
    keeperTest('a save that failed is tried again, later each time', (
      tester,
      keeper,
      fake,
    ) async {
      var fails = 0;
      fake.before = (call) {
        if (call == 'save' && fails > 0) {
          fails--;
          throw broken;
        }
      };
      var told = 0;
      keeper.addListener(() => told++);
      await keeper.apply(edit);
      fails = 4;
      // One second after the change, then 1, 2 and 4 seconds after each failure.
      for (final (wait, tries) in [(1, 1), (1, 2), (2, 3), (4, 4)]) {
        await tester.pump(
          Duration(seconds: wait) - const Duration(milliseconds: 1),
        );
        expect(calls(fake, 'save'), tries - 1, reason: 'before try $tries');
        await tester.pump(const Duration(milliseconds: 1));
        expect(calls(fake, 'save'), tries, reason: 'try $tries');
        expect(keeper.saveError, broken);
        expect(fake.saved, isFalse);
      }
      expect(told, 1);
      // The fifth, 8 seconds on, works; and then there is no more trying.
      await tester.pump(const Duration(seconds: 8));
      expect((calls(fake, 'save'), fake.saves, fake.saved), (5, 1, true));
      expect((keeper.saveError, told), (null, 2));
      await tester.pump(const Duration(hours: 1));
      expect(calls(fake, 'save'), 5);

      // After a save that worked the waiting starts short again.
      await keeper.apply(const Command.enterText(cell: 'a', text: 'more'));
      fails = 1;
      await tester.pump(const Duration(seconds: 1));
      expect(keeper.saveError, broken);
      await tester.pump(const Duration(seconds: 1));
      expect((keeper.saveError, fake.saves), (null, 2));
    });

    keeperTest('the wait between tries has an upper bound, and no tight loop', (
      tester,
      keeper,
      fake,
    ) async {
      fake.before = (call) => call == 'save' ? throw broken : null;
      await keeper.apply(edit);
      // 1 second, then 1, 2, 4, 8, 16, 32 and from there every 60: in the first
      // hour that is 7 tries in 64 seconds and 58 more.
      await tester.pump(const Duration(seconds: 64));
      expect(calls(fake, 'save'), 7);
      await tester.pump(const Duration(seconds: 59));
      expect(calls(fake, 'save'), 7);
      await tester.pump(const Duration(seconds: 1));
      expect(calls(fake, 'save'), 8);
      await tester.pump(const Duration(seconds: 3600 - 124));
      expect(calls(fake, 'save'), 65);
      // A change in between does not make it sooner or more often.
      await keeper.apply(const Command.enterText(cell: 'a', text: 'more'));
      await tester.pump(const Duration(seconds: 3));
      expect(calls(fake, 'save'), 65);
      await tester.pump(const Duration(seconds: 1));
      expect(calls(fake, 'save'), 66);
      fake.before = null;
    });

    keeperTest('trying again stops when the document is closed or let go of', (
      tester,
      keeper,
      fake,
    ) async {
      fake.before = (call) =>
          call == 'save' || call == 'close' ? throw broken : null;
      await keeper.apply(edit);
      await tester.pump(const Duration(seconds: 1));
      expect(keeper.saveError, broken);
      await keeper.letGo();
      final before = fake.log.length;
      await tester.pump(const Duration(hours: 1));
      expect(fake.log.length, before);
    });

    keeperTest('trying again stops after a fault', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      failFirst(fake, 'save', broken);
      await tester.pump(const Duration(seconds: 1));
      failFirst(fake, 'undo', PanicException('at /home/builder/core.rs'));
      await expectLater(keeper.undo(), throwsA(isA<PanicException>()));
      expect(keeper.ended, faultSentence);
      final before = fake.log.length;
      await tester.pump(const Duration(hours: 1));
      expect(fake.log.length, before);
      expect(fake.saves, 0);
    });

    keeperTest('a file that another program changed is not tried again', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      fake.fileChanged = true;
      await tester.pump(const Duration(seconds: 1));
      expect(kindOf(keeper.saveError), ErrorKind.changedOnDisk);
      await tester.pump(const Duration(hours: 1));
      // It needs the user. But a change after it is still tried once.
      expect(calls(fake, 'save'), 1);
      await keeper.undo();
      await tester.pump(const Duration(seconds: 1));
      expect(calls(fake, 'save'), 2);
      await keeper.keepMine();
      expect((keeper.saveError, fake.overwrites, fake.saved), (null, 1, true));
    });

    keeperTest('a save asked for takes the place of the one that was due', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      await keeper.save();
      expect(fake.saves, 1);
      await tester.pump(const Duration(seconds: 5));
      expect(calls(fake, 'save'), 1);
      // So does writing over the other program's file.
      await keeper.undo();
      await keeper.keepMine();
      await tester.pump(const Duration(seconds: 5));
      expect(calls(fake, 'save'), 1);
    });
  });

  group('the app is put away', () {
    keeperTest('what is unfinished is finished first, and then it is saved', (
      tester,
      keeper,
      fake,
    ) async {
      Future<void> typed() async {
        await keeper.apply(edit);
      }

      keeper.addFinisher(typed);
      await keeper.putAway();
      expect(fake.cells['a']!.text, 'typed');
      expect(fake.log, containsAllInOrder(['apply.', 'save']));
      expect((fake.saves, fake.saved), (1, true));

      // And no more once it is taken back.
      keeper.removeFinisher(typed);
      await keeper.putAway();
      expect(calls(fake, 'apply'), 1);
    });

    keeperTest('what cannot be finished does not keep the rest from the file', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      keeper.addFinisher(() async => throw StateError('a mistake'));
      await expectLater(keeper.putAway(), throwsStateError);
      expect((fake.saves, fake.saved), (1, true));
      // Nor from being closed.
      await keeper.undo();
      await expectLater(keeper.close(), throwsStateError);
      expect((fake.saves, fake.closed), (2, true));
    });
  });

  group('closing', () {
    keeperTest('saves, gives the document up, and says so', (
      tester,
      keeper,
      fake,
    ) async {
      var told = 0;
      keeper.addListener(() => told++);
      await keeper.apply(edit);
      await keeper.close();
      expect((fake.saves, fake.closed, fake.disposed), (1, true, true));
      expect((keeper.ended, told), ('The document was closed.', 1));
      final before = fake.log.length;
      await expectLater(keeper.grid('a'), refusedAs(ErrorKind.closed));
      await expectLater(keeper.apply(edit), refusedAs(ErrorKind.closed));
      await expectLater(keeper.undo(), refusedAs(ErrorKind.closed));
      await expectLater(keeper.redo(), refusedAs(ErrorKind.closed));
      await keeper.save();
      await keeper.close();
      await tester.pump(const Duration(seconds: 5));
      expect((fake.log.length, told), (before, 1));
    });

    keeperTest('a close that works takes back what a save before it said', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      failFirst(fake, 'save', broken);
      await keeper.save();
      expect(keeper.saveError, broken);
      await keeper.close();
      expect((keeper.saveError, fake.closed, fake.saves), (null, true, 1));
    });

    keeperTest('comes after the calls made before it', (
      tester,
      keeper,
      fake,
    ) async {
      final gate = holdFirst(fake, 'apply');
      final change = keeper.apply(edit);
      final close = keeper.close();
      await tester.pump();
      expect(fake.closed, isFalse);
      gate.complete();
      expect((await change).changed, ['a']);
      await close;
      expect((fake.saves, fake.closed), (1, true));
    });

    // The review's: a close that failed let go of the document, which made the
    // bridge try the save once more on the thread that draws, and told no screen.
    keeperTest(
      'a close that cannot save leaves the document open, and says so',
      (tester, keeper, fake) async {
        var told = 0;
        keeper.addListener(() => told++);
        await keeper.apply(edit);
        failFirst(fake, 'close', broken);
        await expectLater(keeper.close(), throwsA(broken));
        expect((fake.disposed, fake.closed, fake.saves), (false, false, 0));
        expect((keeper.ended, keeper.saveError, told), (null, broken, 1));
        // In use as before, and saved by itself a second later, as after any save
        // that failed.
        expect((await keeper.grid('a')).cell.text, 'typed');
        await tester.pump(const Duration(seconds: 1));
        expect((fake.saves, keeper.saveError), (1, null));
        await keeper.close();
        expect(fake.closed, isTrue);
      },
    );

    keeperTest('a close over a file that another program changed', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      fake.fileChanged = true;
      await expectLater(keeper.close(), refusedAs(ErrorKind.changedOnDisk));
      expect(kindOf(keeper.saveError), ErrorKind.changedOnDisk);
      expect((keeper.ended, fake.disposed), (null, false));
      // With no one to ask: let go, which never writes over the other's work.
      var told = 0;
      keeper.addListener(() => told++);
      await keeper.letGo();
      expect((fake.disposed, fake.closed), (true, false));
      expect((fake.saves, fake.overwrites), (0, 0));
      expect((keeper.ended, told), ('The document was closed.', 1));
      await keeper.letGo();
      expect(told, 1);
    });

    keeperTest('a fault while closing ends the work', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      failFirst(
        fake,
        'close',
        const BridgeError(kind: ErrorKind.internal, message: 'half changed'),
      );
      await expectLater(keeper.close(), refusedAs(ErrorKind.internal));
      expect((keeper.ended, fake.disposed), (faultSentence, true));
      expect(keeper.saveError, isNull);
    });

    keeperTest('letting go comes after the calls made before it', (
      tester,
      keeper,
      fake,
    ) async {
      final gate = holdFirst(fake, 'apply');
      final change = keeper.apply(edit);
      final gone = keeper.letGo();
      await tester.pump();
      expect(fake.disposed, isFalse);
      gate.complete();
      await change;
      await gone;
      // Unclosed and unsaved, with the file as it was: the bridge saves it.
      expect((fake.disposed, fake.saves), (true, 1));
    });
  });

  group('the other program\'s file', () {
    final theirs = FakeDocument()..cells['a']!.text = 'theirs';
    keeperTest(
      'is opened in place of this document',
      (tester, keeper, fake) async {
        var told = 0;
        keeper.addListener(() => told++);
        await keeper.apply(edit);
        fake.fileChanged = true;
        await keeper.save();
        expect((keeper.opens, told), (1, 1));
        // A change after it, whose save is still to come when the choice is made.
        await keeper.undo();
        await keeper.openTheirs();
        expect((keeper.opens, told), (2, 2));
        expect(keeper.saveError, isNull);
        expect((fake.disposed, fake.saves, fake.overwrites), (true, 0, 0));
        expect((await keeper.grid('a')).cell.text, 'theirs');
        expect((keeper.root, keeper.path), ('root', '/fake.wombat'));
        // The clock of the document that is gone does not save the new one.
        await tester.pump(const Duration(minutes: 5));
        expect(theirs.log.contains('save'), isFalse);
      },
      reopen: (path) {
        expect(path, '/fake.wombat');
        return Session.over(theirs);
      },
    );

    keeperTest(
      'is opened after the calls made before the choice',
      (tester, keeper, fake) async {
        final gate = holdFirst(fake, 'apply');
        final change = keeper.apply(edit);
        final open = keeper.openTheirs();
        // And a call made after the choice is for the document that is opened.
        final after = keeper.grid('a');
        await tester.pump();
        expect(fake.disposed, isFalse);
        gate.complete();
        expect((await change).changed, ['a']);
        await open;
        expect(fake.disposed, isTrue);
        expect((await after).cell.text, 'theirs');
      },
      reopen: (_) => Session.over(FakeDocument()..cells['a']!.text = 'theirs'),
    );

    keeperTest(
      'that cannot be read leaves everything as it is',
      (tester, keeper, fake) async {
        var told = 0;
        keeper.addListener(() => told++);
        await keeper.apply(edit);
        await expectLater(keeper.openTheirs(), throwsA(broken));
        expect((fake.disposed, keeper.ended, keeper.opens), (false, null, 1));
        expect(told, 0);
        // Its own clock included.
        await tester.pump(const Duration(seconds: 1));
        expect(fake.saves, 1);
      },
      canBeRead: (path) {
        expect(path, '/fake.wombat');
        throw broken;
      },
      reopen: (_) => fail('opened a file that cannot be read'),
    );

    keeperTest('a fault while it is opened is said as one', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.openTheirs();
      expect(keeper.ended, faultSentence);
    }, reopen: (_) => throw PanicException('at /home/builder/core.rs'));
  });

  group('whether a file can be read', () {
    Set<String> looks() => {
      for (final entry in Directory.systemTemp.listSync())
        if (entry.path.contains('wombat-look')) entry.path,
    };

    test('also while it is open, and nothing is left behind', () async {
      final before = looks();
      final doc = await openExample(folder, 'open');
      await doc.apply(Command.setText(cell: doc.root, text: 'not saved'));
      final bytes = File(doc.path).readAsBytesSync();
      await readable(doc.path);
      // The file was read, not written, and the document is as it was.
      expect(File(doc.path).readAsBytesSync(), bytes);
      expect(await doc.isSaved(), isFalse);
      expect(looks(), before);
      await doc.close();
    });

    test('says why not, and leaves nothing behind either', () async {
      final before = looks();
      final file = File('${folder.path}/half.wombat')
        ..writeAsStringSync('half written by the other program');
      await expectLater(readable(file.path), refusedAs(ErrorKind.syntax));
      await expectLater(
        readable('${folder.path}/none.wombat'),
        refusedAs(ErrorKind.io),
      );
      expect(looks(), before);
    });
  });
}
