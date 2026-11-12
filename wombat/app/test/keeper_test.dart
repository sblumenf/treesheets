// The keeper of a document: saving by itself, trying again, how it stands with its
// file, what is attached to it, the app put away, another program's file, closing. On
// a stand-in for the core where time or failures must be made, with the clock of the
// test, and on the real core for what reads and writes a file.
import 'dart:async';
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

  int calls(Hooks fake, String call) =>
      fake.log.where((name) => name == call).length;

  /// Every state the keeper tells its listeners of, in order.
  List<KeeperState> states(Keeper keeper) {
    final seen = <KeeperState>[];
    keeper.addListener(() => seen.add(keeper.state));
    return seen;
  }

  const saved = KeeperState.saved;
  const unsaved = KeeperState.unsaved;
  const saving = KeeperState.saving;
  const failed = KeeperState.failed;
  const changedOnDisk = KeeperState.changedOnDisk;
  const ended = KeeperState.ended;

  /// Another program changes the file, and a save finds that out.
  Future<void> changedByAnother(Keeper keeper, FakeDocument fake) async {
    fake.fileChanged = true;
    await keeper.save();
    expect(keeper.state, changedOnDisk);
  }

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
      await endKeeping(keeper);
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
      final seen = states(keeper);
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
      // One thing is wrong, and it was said once.
      expect(seen, [unsaved, saving, failed]);
      // The fifth, 8 seconds on, works; and then there is no more trying.
      await tester.pump(const Duration(seconds: 8));
      expect((calls(fake, 'save'), fake.saves, fake.saved), (5, 1, true));
      expect(keeper.saveError, isNull);
      expect(seen, [unsaved, saving, failed, saved]);
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

    // The fourth review's (Q2): the stand-in threw one constant error, and the keeper
    // compared errors by identity, so on the real core every try was told.
    keeperTest('a failure that repeats is told once, a new one again', (
      tester,
      keeper,
      fake,
    ) async {
      var message = 'the disk is gone';
      fake.before = (call) {
        // As the core does: a new error for every failure.
        if (call == 'save') {
          throw BridgeError(kind: ErrorKind.io, message: message);
        }
      };
      final seen = states(keeper);
      await keeper.apply(edit);
      await tester.pump(const Duration(minutes: 10));
      expect(calls(fake, 'save'), greaterThan(10));
      expect(seen, [unsaved, saving, failed]);
      message = 'the disk is full';
      await tester.pump(const Duration(minutes: 1));
      expect(seen, [unsaved, saving, failed, failed]);
      fake.before = null;
    });

    // No test showed that a save waits for its turn.
    keeperTest('a save comes after the calls made before it', (
      tester,
      keeper,
      fake,
    ) async {
      final gate = holdFirst(fake, 'apply');
      final change = keeper.apply(edit);
      final save = keeper.save();
      await tester.pump();
      expect(calls(fake, 'save'), 0);
      gate.complete();
      await change;
      await save;
      expect((fake.saves, fake.saved), (1, true));
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

  group('how it stands with its file', () {
    keeperTest('saved, unsaved, saving and saved again', (
      tester,
      keeper,
      fake,
    ) async {
      expect(keeper.state, saved);
      final seen = states(keeper);
      await keeper.apply(edit);
      expect(keeper.state, unsaved);
      await keeper.apply(const Command.enterText(cell: 'a', text: 'more'));
      // While the file is written, with the core held up in the middle of it.
      final gate = holdFirst(fake, 'save');
      final save = keeper.save();
      await tester.pump();
      expect(keeper.state, saving);
      gate.complete();
      await save;
      expect(keeper.state, saved);
      expect(seen, [unsaved, saving, saved]);
    });

    keeperTest('what changed nothing leaves it saved, and starts no clock', (
      tester,
      keeper,
      fake,
    ) async {
      final seen = states(keeper);
      // The text the cell has; nothing to undo; reading.
      await keeper.apply(const Command.enterText(cell: 'a', text: 'one'));
      await keeper.undo();
      await keeper.redo();
      await keeper.grid('a');
      expect(keeper.state, saved);
      expect(seen, isEmpty);
      await tester.pump(const Duration(minutes: 5));
      expect(calls(fake, 'save'), 0);
      // Nor is a save with nothing to write told as one.
      await keeper.save();
      expect(seen, isEmpty);
    });

    keeperTest('failed, changed by another program, and ended', (
      tester,
      keeper,
      fake,
    ) async {
      final seen = states(keeper);
      await keeper.apply(edit);
      failFirst(fake, 'save', broken);
      await keeper.save();
      expect(keeper.state, failed);
      // What is wrong outranks what is under way and what is new.
      await keeper.undo();
      expect(keeper.state, failed);
      fake.fileChanged = true;
      await keeper.save();
      expect(keeper.state, changedOnDisk);
      await keeper.redo();
      await tester.pump(const Duration(seconds: 2));
      await keeper.keepMine();
      expect(keeper.state, saved);
      await keeper.letGo();
      expect(keeper.state, ended);
      expect(seen, [unsaved, saving, failed, changedOnDisk, saved, ended]);
    });

    // The fourth review's (P5, P5b): listeners were told inside the keeper's own
    // piece of work, so one that asked the keeper was refused.
    keeperTest(
      'a listener may ask the keeper when it is told',
      (tester, keeper, fake) async {
        final asked = <Object?>[];
        keeper.addListener(() {
          keeper
              .grid('a')
              .then<void>(
                (grid) => asked.add(grid.cell.text),
                onError: (Object error) => asked.add(error),
              );
        });
        await keeper.apply(edit);
        await changedByAnother(keeper, fake);
        await keeper.openTheirs();
        await tester.pump();
        // Told of: unsaved, saving, changed by another program, and theirs opened.
        expect(asked, ['typed', 'typed', 'typed', 'theirs']);
      },
      reopen: (_) => Session.over(FakeDocument()..cells['a']!.text = 'theirs'),
    );
  });

  group('what is attached', () {
    keeperTest('is told and asked until it is detached', (
      tester,
      keeper,
      fake,
    ) async {
      var told = 0;
      var asked = 0;
      final attachment = keeper.attach(
        onChange: () => told++,
        finish: () async => asked++,
      );
      await keeper.apply(edit);
      expect(await keeper.putAway(), isEmpty);
      expect((told, asked), (3, 1));
      attachment.detach();
      // ignore: invalid_use_of_protected_member
      expect(keeper.hasListeners, isFalse);
      await keeper.undo();
      await keeper.putAway();
      expect((told, asked), (3, 1));
      // Twice is once: the listener of another is not taken with it.
      final other = keeper.attach(onChange: () => told++);
      attachment.detach();
      await keeper.redo();
      expect(told, 4);
      other.detach();
    });

    keeperTest('finishes in the turn of its queue', (
      tester,
      keeper,
      fake,
    ) async {
      final actions = InTurn();
      final order = <String>[];
      keeper.attach(queue: actions, finish: () async => order.add('finish'));
      final gate = Completer<void>();
      actions.post(() async {
        await gate.future;
        order.add('asked before');
      });
      final away = keeper.putAway();
      await tester.pump();
      expect(order, isEmpty);
      gate.complete();
      await away;
      expect(order, ['asked before', 'finish']);
    });

    keeperTest('a queue alone is waited for', (tester, keeper, fake) async {
      final steps = InTurn();
      keeper.attach(queue: steps);
      final gate = Completer<void>();
      steps.post(() async {
        await gate.future;
        await keeper.apply(edit);
      });
      final close = keeper.close();
      await tester.pump();
      expect(fake.closed, isFalse);
      gate.complete();
      await close;
      expect(
        (fake.closed, fake.cells['a']!.text, fake.saves),
        (true, 'typed', 1),
      );
    });

    // The fourth review's (P7): a "close" action in the screen's own queue of
    // actions closed the document without the editor's text, and then threw.
    for (final (name, end) in <(String, Future<void> Function(Keeper))>[
      ('close', (keeper) => keeper.close()),
      ('put away', (keeper) => keeper.putAway()),
    ]) {
      keeperTest('$name asked from the queue of what is attached', (
        tester,
        keeper,
        fake,
      ) async {
        final actions = InTurn();
        keeper.attach(queue: actions, finish: () => keeper.apply(edit));
        await actions.run(() => end(keeper));
        expect((fake.cells['a']!.text, fake.saved), ('typed', true));
        expect(fake.closed, name == 'close');
      });
    }

    keeperTest(
      'something that asks its own queue for itself: refused, nothing closed',
      (tester, keeper, fake) async {
        final actions = InTurn();
        // Not said to the keeper, so the keeper cannot make it part of the piece.
        keeper.attach(finish: () => actions.run(() => keeper.apply(edit)));
        await keeper.apply(const Command.enterText(cell: 'd', text: 'done'));
        Object? error;
        await actions.run(() async {
          try {
            await keeper.close();
          } catch (e) {
            error = e;
          }
        });
        expect(
          error,
          isA<Unfinished>().having(
            (e) => e.causes.single,
            'cause',
            isStateError,
          ),
        );
        expect('$error', contains('its own queue'));
        // Open and in use, and what was in the document is in the file.
        expect((fake.closed, keeper.ended, fake.saves), (false, null, 1));
        expect((await keeper.grid('a')).cell.text, 'one');
      },
    );

    // The fourth review's (P9).
    keeperTest('something that puts the app away while it finishes', (
      tester,
      keeper,
      fake,
    ) async {
      var asked = 0;
      keeper.attach(
        finish: () async {
          asked++;
          await keeper.apply(edit);
          await keeper.putAway();
        },
      );
      expect(await keeper.putAway(), isEmpty);
      expect((asked, fake.saved), (1, true));
    });

    keeperTest('something that detaches itself while it finishes', (
      tester,
      keeper,
      fake,
    ) async {
      late Attachment first;
      var others = 0;
      first = keeper.attach(finish: () async => first.detach());
      keeper.attach(finish: () async => others++);
      keeper.attach(finish: () async => others++);
      expect(await keeper.putAway(), isEmpty);
      expect(others, 2);
    });
  });

  group('the app is put away', () {
    keeperTest('saved, then what is unfinished is finished, then saved again', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(const Command.enterText(cell: 'd', text: 'done'));
      final typed = keeper.attach(
        finish: () async {
          // What was in the document is in the file before anyone is asked.
          expect((fake.saves, fake.saved), (1, true));
          await keeper.apply(edit);
        },
      );
      expect(await keeper.putAway(), isEmpty);
      expect(fake.cells['a']!.text, 'typed');
      expect((fake.saves, fake.saved), (2, true));
      expect(calls(fake, 'save'), 2);

      // With nothing new from them the file is not looked at a second time.
      typed.detach();
      keeper.attach(finish: () async {});
      await keeper.putAway();
      expect((calls(fake, 'apply'), calls(fake, 'save')), (2, 3));
    });

    // The fourth review's (P6): the save waited for the finishers, for ever.
    keeperTest('what never finishes does not keep the document from the file', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      final never = keeper.attach(finish: () => Completer<void>().future);
      keeper.attach(finish: () async => throw StateError('a mistake'));
      List<Object>? unfinished;
      unawaited(keeper.putAway().then((failed) => unfinished = failed));
      await tester.pump();
      expect((fake.saves, fake.saved), (1, true));
      // It is given two seconds, and then it is said.
      await tester.pump(const Duration(milliseconds: 1999));
      expect(unfinished, isNull);
      await tester.pump(const Duration(milliseconds: 1));
      expect(unfinished, [isStateError, isA<TimeoutException>()]);
      never.detach();
    });

    keeperTest('a save that fails does not keep it from finishing', (
      tester,
      keeper,
      fake,
    ) async {
      keeper.attach(finish: () => keeper.apply(edit));
      await keeper.apply(const Command.enterText(cell: 'd', text: 'done'));
      failFirst(fake, 'save', broken);
      expect(await keeper.putAway(), isEmpty);
      expect(
        (fake.cells['a']!.text, fake.saved, calls(fake, 'save')),
        ('typed', true, 2),
      );
    });
  });

  group('closing', () {
    keeperTest('saves, gives the document up, and says so', (
      tester,
      keeper,
      fake,
    ) async {
      final seen = states(keeper);
      await keeper.apply(edit);
      await keeper.close();
      expect((fake.saves, fake.closed, fake.disposed), (1, true, true));
      expect(keeper.ended, 'The document was closed.');
      expect(seen, [unsaved, ended]);
      await tester.pump(const Duration(seconds: 5));
      expect(fake.log.last, 'close.');
    });

    // The class comment said "every call then throws", and six of them did not.
    for (final how in ['a close', 'a fault', 'letting go']) {
      keeperTest('after $how every call fails the same way', (
        tester,
        keeper,
        fake,
      ) async {
        var finished = 0;
        keeper.attach(finish: () async => finished++);
        await keeper.apply(edit);
        // Asked before the end, and its turn comes after it.
        final gate = holdFirst(fake, how == 'a fault' ? 'undo' : 'apply');
        final Future<void> end;
        if (how == 'a fault') {
          end = keeper.undo().then((_) {}, onError: (Object _) {});
          await tester.pump();
          gate.completeError(PanicException('at /home/builder/core.rs'));
        } else {
          final last = keeper.apply(edit);
          end = how == 'a close' ? keeper.close() : keeper.letGo();
          gate.complete();
          await last;
        }
        final late = keeper.save();
        await end;
        await expectLater(late, refusedAs(ErrorKind.closed));
        expect(keeper.state, ended);
        final seen = states(keeper);
        final before = (fake.log.length, finished);
        for (final call in <Future<Object?> Function()>[
          () => keeper.grid('a'),
          () => keeper.apply(edit),
          keeper.undo,
          keeper.redo,
          keeper.save,
          keeper.keepMine,
          keeper.putAway,
          keeper.openTheirs,
          keeper.close,
          keeper.letGo,
        ]) {
          await expectLater(
            call(),
            throwsA(
              isA<BridgeError>()
                  .having((e) => e.kind, 'kind', ErrorKind.closed)
                  .having(
                    (e) => e.message,
                    'message',
                    'the document is no longer in use',
                  ),
            ),
          );
        }
        // Nothing reached the document, no one was asked to finish or told, and no
        // clock runs. Ending it with no one to ask is still nothing to throw for.
        await endKeeping(keeper);
        await tester.pump(const Duration(minutes: 5));
        expect((fake.log.length, finished), before);
        expect(seen, isEmpty);
      });
    }

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
        final seen = states(keeper);
        await keeper.apply(edit);
        failFirst(fake, 'close', broken);
        await expectLater(keeper.close(), throwsA(broken));
        expect((fake.disposed, fake.closed, fake.saves), (false, false, 0));
        expect((keeper.ended, keeper.saveError), (null, broken));
        expect(seen, [unsaved, failed]);
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
      // The clock is not left stopped: a change after it is tried a second later.
      await tester.pump(const Duration(minutes: 5));
      expect(calls(fake, 'save'), 0);
      await keeper.undo();
      await tester.pump(const Duration(seconds: 1));
      expect(calls(fake, 'save'), 1);
      // With no one to ask: let go, which never writes over the other's work.
      final seen = states(keeper);
      await keeper.letGo();
      expect((fake.disposed, fake.closed), (true, false));
      expect((fake.saves, fake.overwrites), (0, 0));
      expect(keeper.ended, 'The document was closed.');
      expect(seen, [ended]);
    });

    // The fourth review's (P6b, G4): a close waited for ever for what never
    // finished, and after something that could not finish it closed and then threw.
    keeperTest('what cannot finish keeps the document open, and is said', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      final never = keeper.attach(finish: () => Completer<void>().future);
      Object? error;
      unawaited(keeper.close().catchError((Object e) => error = e));
      await tester.pump(const Duration(milliseconds: 1999));
      expect((error, fake.closed), (null, false));
      await tester.pump(const Duration(milliseconds: 1));
      expect(
        error,
        isA<Unfinished>().having(
          (e) => e.causes.single,
          'cause',
          isA<TimeoutException>(),
        ),
      );
      // Not closed, in use, and what is in the document is in the file.
      expect((fake.closed, keeper.ended), (false, null));
      expect((fake.saves, fake.saved), (1, true));
      never.detach();
      final wrong = keeper.attach(finish: () async => throw StateError('no'));
      await keeper.undo();
      await expectLater(keeper.close(), throwsA(isA<Unfinished>()));
      expect((fake.closed, fake.saves), (false, 2));
      wrong.detach();
      await keeper.close();
      expect(fake.closed, isTrue);
    });

    group('with no one left to ask', () {
      keeperTest('saved and closed', (tester, keeper, fake) async {
        await keeper.apply(edit);
        await endKeeping(keeper);
        expect((fake.saves, fake.closed, fake.disposed), (1, true, true));
        // Once more is nothing.
        await endKeeping(keeper);
      });

      keeperTest('let go of when it cannot be saved, and no error for that', (
        tester,
        keeper,
        fake,
      ) async {
        await keeper.apply(edit);
        fake.fileChanged = true;
        await endKeeping(keeper);
        expect((fake.closed, fake.disposed), (false, true));
        expect((fake.saves, fake.overwrites), (0, 0));
        expect(keeper.ended, 'The document was closed.');
        await tester.pump(const Duration(minutes: 5));
        expect(fake.log.last, 'close.');
      });

      keeperTest('let go of when something cannot finish', (
        tester,
        keeper,
        fake,
      ) async {
        await keeper.apply(edit);
        keeper.attach(finish: () async => throw StateError('no'));
        await endKeeping(keeper);
        expect((fake.closed, fake.disposed, fake.saves), (false, true, 1));
      });
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
        await keeper.apply(edit);
        final seen = states(keeper);
        await changedByAnother(keeper, fake);
        expect(keeper.opens, 1);
        // A change after it, whose save is still to come when the choice is made.
        await keeper.undo();
        await keeper.openTheirs();
        expect(keeper.opens, 2);
        expect(seen, [saving, changedOnDisk, saved]);
        expect(keeper.saveError, isNull);
        expect((fake.disposed, fake.saves, fake.overwrites), (true, 0, 0));
        expect((await keeper.grid('a')).cell.text, 'theirs');
        expect((keeper.root, keeper.path), ('root', '/fake.wombat'));
        // The clock of the document that is gone does not save the new one.
        await tester.pump(const Duration(minutes: 5));
        expect(theirs.log.contains('save'), isFalse);
        // Its own does: what is done to it is saved a second later.
        await keeper.apply(edit);
        await tester.pump(const Duration(seconds: 1));
        expect((theirs.saves, theirs.saved), (1, true));
      },
      reopen: (path) {
        expect(path, '/fake.wombat');
        return Session.over(theirs);
      },
    );

    keeperTest(
      'is opened after the calls made before the choice',
      (tester, keeper, fake) async {
        await changedByAnother(keeper, fake);
        final gate = holdFirst(fake, 'apply');
        final change = keeper.apply(edit);
        final open = keeper.openTheirs();
        await tester.pump();
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

    // The fourth review's (G7, Q1): what a screen had begun on this document went
    // on in the other program's.
    keeperTest(
      'is opened when what is attached has finished what it had begun',
      (tester, keeper, fake) async {
        final steps = InTurn();
        keeper.attach(queue: steps);
        await changedByAnother(keeper, fake);
        // One step of two calls, held up in the middle.
        final gate = Completer<void>();
        steps.post(() async {
          await keeper.undo();
          await gate.future;
          await keeper.apply(edit);
        });
        final open = keeper.openTheirs();
        await tester.pump(const Duration(seconds: 1));
        expect(fake.disposed, isFalse);
        gate.complete();
        await open;
        expect(fake.cells['a']!.text, 'typed');
        expect((await keeper.grid('a')).cell.text, 'theirs');
      },
      reopen: (_) => Session.over(FakeDocument()..cells['a']!.text = 'theirs'),
    );

    keeperTest('is not opened when something attached cannot finish', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      await changedByAnother(keeper, fake);
      final never = keeper.attach(finish: () => Completer<void>().future);
      final seen = states(keeper);
      Object? error;
      unawaited(keeper.openTheirs().catchError((Object e) => error = e));
      await tester.pump(const Duration(seconds: 2));
      expect(error, isA<Unfinished>());
      // Nothing changed: the choice is still to make.
      expect(
        (fake.disposed, keeper.opens, keeper.state),
        (false, 1, changedOnDisk),
      );
      expect(seen, isEmpty);
      never.detach();
    }, reopen: (_) => fail('opened the file'));

    // The fourth review's (G9, P13): letting go of a document whose file is as it
    // left it saves the document, so "open theirs" wrote mine and opened that.
    keeperTest(
      'is not asked for when no other program changed the file',
      (tester, keeper, fake) async {
        await keeper.apply(edit);
        await expectLater(keeper.openTheirs(), refusedAs(ErrorKind.refused));
        expect((fake.saves, fake.disposed, keeper.opens), (0, false, 1));
        expect(keeper.state, unsaved);
        // Nor after a save that failed for another reason, or with all saved.
        failFirst(fake, 'save', broken);
        await keeper.save();
        await expectLater(keeper.openTheirs(), refusedAs(ErrorKind.refused));
        await keeper.save();
        await expectLater(keeper.openTheirs(), refusedAs(ErrorKind.refused));
        expect((fake.disposed, keeper.opens), (false, 1));
      },
      canBeRead: (_) => fail('looked at the file'),
      reopen: (_) => fail('opened the file'),
    );

    keeperTest(
      'that cannot be read leaves everything as it is',
      (tester, keeper, fake) async {
        await keeper.apply(edit);
        await changedByAnother(keeper, fake);
        await keeper.undo();
        final seen = states(keeper);
        await expectLater(keeper.openTheirs(), throwsA(broken));
        expect((fake.disposed, keeper.ended, keeper.opens), (false, null, 1));
        expect(seen, isEmpty);
        // Its own clock included.
        await tester.pump(const Duration(seconds: 1));
        expect(calls(fake, 'save'), 2);
      },
      canBeRead: (path) {
        expect(path, '/fake.wombat');
        throw broken;
      },
      reopen: (_) => fail('opened a file that cannot be read'),
    );

    keeperTest('that is changed again while it is opened ends the work', (
      tester,
      keeper,
      fake,
    ) async {
      // It could be read when it was looked at, and cannot when it is opened.
      await changedByAnother(keeper, fake);
      final seen = states(keeper);
      await keeper.openTheirs();
      expect(fake.disposed, isTrue);
      expect(seen, [ended]);
      expect(keeper.ended, contains('the disk is gone'));
      // Not a dead end: what to do is said.
      expect(keeper.ended, contains('Close Wombat and open it again'));
      await expectLater(keeper.undo(), refusedAs(ErrorKind.closed));
    }, reopen: (_) => throw broken);

    keeperTest('a fault while it is opened is said as one', (
      tester,
      keeper,
      fake,
    ) async {
      await changedByAnother(keeper, fake);
      await keeper.openTheirs();
      expect(keeper.ended, faultSentence);
    }, reopen: (_) => throw PanicException('at /home/builder/core.rs'));
  });

  group('when the core fails', () {
    keeperTest('after a fault the document is let go of, and nothing saved', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(edit);
      final seen = states(keeper);
      failFirst(fake, 'apply', PanicException('at /home/builder/core.rs'));
      await expectLater(keeper.apply(edit), throwsA(isA<PanicException>()));
      expect((keeper.ended, keeper.state), (faultSentence, ended));
      expect(seen, [ended]);
      expect((fake.disposed, fake.saves, fake.closed), (true, 0, false));
    });

    for (final call in ['save', 'overwrite', 'close']) {
      keeperTest('the refusal of $call after a fault ends the work too', (
        tester,
        keeper,
        fake,
      ) async {
        await keeper.apply(edit);
        failFirst(
          fake,
          call,
          const BridgeError(kind: ErrorKind.internal, message: 'half changed'),
        );
        final end = switch (call) {
          'save' => keeper.save(),
          'overwrite' => keeper.keepMine(),
          _ => keeper.close().catchError((Object _) {}),
        };
        await end;
        expect((keeper.ended, keeper.saveError), (faultSentence, null));
        // Let go of, and nothing written on the way (the fourth review's P10: the
        // stand-in wrote there, which the bridge does not after a fault).
        expect((fake.disposed, fake.saves, fake.overwrites), (true, 0, 0));
      });
    }
  });

  // What the stand-in cannot show: the file itself.
  group('on the real core', () {
    var count = 0;
    final keepers = <Keeper>[];
    tearDown(() async {
      for (final keeper in keepers) {
        await endKeeping(keeper);
      }
      keepers.clear();
    });

    Future<(Keeper, Session, Watched)> kept({
      Duration saveAfter = const Duration(hours: 1),
    }) async {
      final (doc, watch) = await watchExample(folder, 'kept-${count++}');
      final keeper = Keeper(doc, saveAfter: saveAfter);
      keepers.add(keeper);
      return (keeper, doc, watch);
    }

    Future<String> second(Keeper keeper) async =>
        (await keeper.grid(keeper.root)).cells[1].id;

    Command typing(String cell, String text) =>
        Command.enterText(cell: cell, text: text);

    test('a change is in the file soon after, and so is its undo', () async {
      final (keeper, doc, watch) = await kept(
        saveAfter: const Duration(milliseconds: 20),
      );
      final seen = states(keeper);
      // Not a time to wait for: the save itself, whenever it comes.
      var written = watch.next('save');
      await keeper.apply(typing(await second(keeper), 'saved by itself'));
      await written;
      await keeper.grid(keeper.root);
      expect(calls(watch, 'save'), 1);
      expect((await doc.isSaved(), keeper.state), (true, saved));
      expect(File(doc.path).readAsStringSync(), contains('"saved by itself"'));
      expect(seen, [unsaved, saving, saved]);
      written = watch.next('save');
      await keeper.undo();
      await written;
      await keeper.grid(keeper.root);
      expect(
        File(doc.path).readAsStringSync(),
        isNot(contains('saved by itself')),
      );
      // What changes nothing in the core is not unsaved here either.
      await keeper.apply(typing(await second(keeper), 'what to buy'));
      expect((await doc.isSaved(), keeper.state), (true, saved));
    });

    test('closing writes what was done, and gives the file back', () async {
      final (keeper, doc, _) = await kept();
      await keeper.apply(typing(await second(keeper), 'at the end'));
      await expectLater(
        Session.open(doc.path),
        refusedAs(ErrorKind.alreadyOpen),
      );
      await keeper.close();
      final again = await Session.open(doc.path);
      expect((await again.grid(again.root)).cells[1].text, 'at the end');
      await again.close();
    });

    test('a file that another program changed: keep mine', () async {
      final (keeper, doc, _) = await kept();
      await keeper.apply(typing(await second(keeper), 'mine'));
      final theirs = '${File(doc.path).readAsStringSync()}\n';
      File(doc.path).writeAsStringSync(theirs);
      final seen = states(keeper);
      await keeper.save();
      expect(kindOf(keeper.saveError), ErrorKind.changedOnDisk);
      expect(File(doc.path).readAsStringSync(), theirs);
      // Looked at again, and nothing new to tell.
      await keeper.save();
      expect(seen, [saving, changedOnDisk]);
      await keeper.keepMine();
      expect((keeper.saveError, keeper.state), (null, saved));
      expect(File(doc.path).readAsStringSync(), contains('"mine"'));
      expect(await doc.isSaved(), isTrue);
    });

    test('a file that another program changed: open theirs', () async {
      final (keeper, doc, _) = await kept();
      final theirs = File(doc.path)
          .readAsStringSync()
          .replaceFirst('"Shopping"', '"Theirs"');
      await keeper.apply(typing(await second(keeper), 'mine'));
      File(doc.path).writeAsStringSync(theirs);
      await keeper.save();
      expect(keeper.state, changedOnDisk);
      await keeper.openTheirs();
      expect((keeper.state, keeper.opens), (saved, 2));
      expect(File(doc.path).readAsStringSync(), theirs);
      final cells = (await keeper.grid(keeper.root)).cells;
      expect((cells[0].text, cells[1].text), ('Theirs', 'what to buy'));
      // The keeper is on the document that was opened: it changes and saves that one.
      await keeper.apply(typing(cells[1].id, 'theirs, edited'));
      await keeper.save();
      expect(keeper.saveError, isNull);
      expect(File(doc.path).readAsStringSync(), contains('"theirs, edited"'));
      await expectLater(doc.isSaved(), refusedAs(ErrorKind.closed));
    });

    test('open theirs when theirs cannot be read: this one stays', () async {
      final (keeper, doc, _) = await kept();
      await keeper.apply(typing(await second(keeper), 'an hour of work'));
      const theirs = 'half written by the other program';
      File(doc.path).writeAsStringSync(theirs);
      await keeper.save();
      final seen = states(keeper);
      await expectLater(keeper.openTheirs(), refusedAs(ErrorKind.syntax));
      // The choice is still to make, and nothing was written or lost.
      expect((keeper.ended, keeper.state), (null, changedOnDisk));
      expect(seen, isEmpty);
      expect(File(doc.path).readAsStringSync(), theirs);
      expect(await doc.isSaved(), isFalse);
      await keeper.keepMine();
      expect(File(doc.path).readAsStringSync(), contains('"an hour of work"'));
    });

    test(
      'open theirs when the file is as it was: nothing is written',
      () async {
        final (keeper, doc, _) = await kept();
        final before = File(doc.path).readAsStringSync();
        await keeper.apply(typing(await second(keeper), 'mine'));
        await expectLater(keeper.openTheirs(), refusedAs(ErrorKind.refused));
        expect(File(doc.path).readAsStringSync(), before);
        expect((await doc.isSaved(), keeper.state), (false, unsaved));
      },
    );

    test('after a fault the file is as it was last saved, and free', () async {
      final (keeper, doc, watch) = await kept();
      final cell = await second(keeper);
      await keeper.apply(typing(cell, 'saved'));
      await keeper.save();
      await keeper.apply(typing(cell, 'not saved'));
      // As the bridge answers every call after a panic (decision A143).
      failFirst(
        watch,
        'undo',
        const BridgeError(kind: ErrorKind.internal, message: 'half changed'),
      );
      await expectLater(keeper.undo(), refusedAs(ErrorKind.internal));
      expect(keeper.ended, faultSentence);
      await expectLater(keeper.save(), refusedAs(ErrorKind.closed));
      final again = await Session.open(doc.path);
      expect((await again.grid(again.root)).cells[1].text, isNotEmpty);
      await again.close();
    });
  });

  group('whether a file can be read', () {
    // The looks of these tests are made in a folder of their own: other test files
    // run at the same time and look into the system's temporary folder.
    late Directory within;
    setUp(() async => within = await folder.createTemp('looks'));

    test('also while it is open, and nothing is left behind', () async {
      final doc = await openExample(folder, 'open');
      await doc.apply(Command.setText(cell: doc.root, text: 'not saved'));
      final bytes = File(doc.path).readAsBytesSync();
      await readable(doc.path, within: within);
      // The file was read, not written, and the document is as it was.
      expect(File(doc.path).readAsBytesSync(), bytes);
      expect(await doc.isSaved(), isFalse);
      expect(within.listSync(), isEmpty);
      // Without a folder given, it is the system's temporary folder.
      await readable(doc.path);
      await doc.close();
      // The copy was made where it was said, and not somewhere else all the same:
      // with no folder to make it in, that is the error, and it is not the core's.
      await expectLater(
        readable(doc.path, within: Directory('${folder.path}/none')),
        throwsA(isA<FileSystemException>()),
      );
    });

    test('a copy that cannot be closed is let go of', () async {
      final fake = FakeDocument();
      failFirst(fake, 'close', broken);
      await expectLater(
        readable(
          '/fake.wombat',
          within: within,
          openCopy: ({required from, required to}) {
            expect(from, '/fake.wombat');
            expect(to, startsWith('${within.path}/'));
            return Session.over(fake);
          },
        ),
        throwsA(broken),
      );
      expect(fake.disposed, isTrue);
      expect(within.listSync(), isEmpty);
    });

    test('says why not, and leaves nothing behind either', () async {
      final file = File('${folder.path}/half.wombat')
        ..writeAsStringSync('half written by the other program');
      await expectLater(
        readable(file.path, within: within),
        refusedAs(ErrorKind.syntax),
      );
      await expectLater(
        readable('${folder.path}/none.wombat', within: within),
        refusedAs(ErrorKind.io),
      );
      expect(within.listSync(), isEmpty);
    });
  });
}
