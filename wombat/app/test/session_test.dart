// The session: calls to one document in the order they were made, errors, closing.
// Against the real core, and against a stand-in where the order must be seen.
import 'dart:async';
import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'fake_document.dart';
import 'support.dart';

void main() {
  late Directory folder;
  var count = 0;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-session');
  });

  tearDownAll(() => folder.delete(recursive: true));

  Future<Session> create() =>
      Session.create('${folder.path}/doc-${count++}.wombat');

  Future<String> text(Session doc) async =>
      (await doc.grid(doc.root)).cell.text;

  Matcher refusedAs(ErrorKind kind) =>
      throwsA(isA<BridgeError>().having((e) => e.kind, 'kind', kind));

  Command insert(Session doc, int at, String text) =>
      Command.insertText(cell: doc.root, at: at, text: text);

  // The test of the review. On the bare handle the text was wrong in about a third of
  // the rounds: its calls are kept apart, not in order.
  test('forty insertions made without waiting arrive in order', () async {
    const letters = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMN';
    for (var round = 0; round < 20; round++) {
      final doc = await create();
      final calls = [
        for (var i = 0; i < letters.length; i++)
          doc.apply(insert(doc, i, letters[i])),
      ];
      final reports = await Future.wait(calls);
      expect(reports.every((r) => r.changed.single == doc.root), isTrue);
      expect(await text(doc), letters, reason: 'round $round');
      await doc.close();
    }
  });

  test('a call starts when the one before it has ended', () async {
    final fake = FakeDocument();
    final doc = await Session.over(fake);
    // The first call is held up; without a queue the second would overtake it.
    final gate = holdFirst(fake, 'apply');
    final first = doc.apply(const Command.setText(cell: 'a', text: '1'));
    final second = doc.undo();
    final third = doc.grid('a');
    await pumpEventQueue();
    expect(fake.log, ['apply']);
    gate.complete();
    expect((await first).changed, ['a']);
    expect((await second)!.changed, ['a']);
    expect((await third).cell.text, 'one');
    expect(fake.log, ['apply', 'apply.', 'undo', 'undo.', 'grid', 'grid.']);
  });

  test('an error reaches its caller and the calls after it go on', () async {
    final doc = await create();
    final first = doc.apply(insert(doc, 0, 'a'));
    // Outside the text: refused.
    final second = doc.apply(insert(doc, 99, 'x'));
    final third = doc.apply(insert(doc, 1, 'b'));
    final fourth = doc.grid('no ID');
    final fifth = doc.apply(insert(doc, 2, 'c'));
    await expectLater(second, refusedAs(ErrorKind.refused));
    await expectLater(fourth, refusedAs(ErrorKind.badRequest));
    expect((await first).changed, [doc.root]);
    expect((await third).changed, [doc.root]);
    expect((await fifth).changed, [doc.root]);
    expect(await text(doc), 'abc');
  });

  test(
    'a save between two edits writes the first and not the second',
    () async {
      for (var round = 0; round < 20; round++) {
        final doc = await create();
        final first = doc.apply(
          Command.setText(cell: doc.root, text: 'before'),
        );
        final save = doc.save();
        final between = doc.isSaved();
        final second = doc.apply(
          Command.setText(cell: doc.root, text: 'after'),
        );
        final last = doc.isSaved();
        await Future.wait([first, save, second]);
        expect((await between, await last), (true, false));
        final file = File(doc.path).readAsStringSync();
        expect(file, contains('"text": "before"'), reason: 'round $round');
        expect(file, isNot(contains('after')));
        await doc.discard();
      }
    },
  );

  test(
    'close saves what was asked before it and refuses what comes after',
    () async {
      final doc = await create();
      final edit = doc.apply(Command.setText(cell: doc.root, text: 'kept'));
      final close = doc.close();
      // Made while the close is still under way: it comes after it.
      final late = doc.apply(Command.setText(cell: doc.root, text: 'late'));
      await expectLater(late, refusedAs(ErrorKind.closed));
      await edit;
      await close;

      await expectLater(doc.grid(doc.root), refusedAs(ErrorKind.closed));
      await expectLater(doc.isSaved(), refusedAs(ErrorKind.closed));
      await expectLater(doc.undo(), refusedAs(ErrorKind.closed));
      await expectLater(doc.redo(), refusedAs(ErrorKind.closed));
      await expectLater(doc.save(), refusedAs(ErrorKind.closed));
      await expectLater(doc.overwrite(), refusedAs(ErrorKind.closed));
      await expectLater(doc.close(), refusedAs(ErrorKind.closed));
      await doc.discard();

      // The file is free and holds the edit; the same document as before.
      final again = await Session.open(doc.path);
      expect((again.id, again.root, again.path), (doc.id, doc.root, doc.path));
      expect(await text(again), 'kept');
      await again.close();
    },
  );

  test('closing lets go of the handle, and so does discarding', () async {
    final closed = FakeDocument();
    final one = await Session.over(closed);
    await one.close();
    expect((closed.closed, closed.disposed), (true, true));

    final discarded = FakeDocument();
    final two = await Session.over(discarded);
    await two.discard();
    expect((discarded.closed, discarded.disposed), (false, true));
    await expectLater(two.grid('a'), refusedAs(ErrorKind.closed));
    expect(discarded.log, isEmpty);
  });

  test('a close that cannot save leaves the document open', () async {
    final doc = await create();
    await doc.apply(Command.setText(cell: doc.root, text: 'mine'));
    // Another program saves something else under the name.
    final theirs = '${File(doc.path).readAsStringSync()}\n';
    File(doc.path).writeAsStringSync(theirs);

    await expectLater(doc.save(), refusedAs(ErrorKind.changedOnDisk));
    await expectLater(doc.close(), refusedAs(ErrorKind.changedOnDisk));
    expect(await text(doc), 'mine');
    expect(await doc.isSaved(), isFalse);
    await expectLater(Session.open(doc.path), refusedAs(ErrorKind.alreadyOpen));

    // The user keeps the other program's file: nothing is written over it.
    await doc.discard();
    expect(File(doc.path).readAsStringSync(), theirs);
    final other = await Session.open(doc.path);
    expect(await text(other), '');

    // Or keeps this document.
    await other.apply(Command.setText(cell: other.root, text: 'mine again'));
    File(other.path).writeAsStringSync('$theirs\n');
    await expectLater(other.close(), refusedAs(ErrorKind.changedOnDisk));
    await other.overwrite();
    await other.close();
    expect(File(doc.path).readAsStringSync(), contains('"mine again"'));
  });

  test('a position in a Dart string as the core counts it', () async {
    // a, an emoji (two units, one code point), e with an accent of its own (two units,
    // two code points, one character to the user), b.
    const sample = 'a\u{1F600}éb';
    expect(sample.length, 6);
    expect(codePoints(sample, 0), 0);
    expect(codePoints(sample, 1), 1);
    expect(codePoints(sample, 3), 2);
    expect(codePoints(sample, 4), 3);
    expect(codePoints(sample, 5), 4);
    expect(codePoints(sample, 6), 5);
    // Between the halves of the emoji: after it.
    expect(codePoints(sample, 2), 2);

    // The core agrees: inserting at the position after the emoji lands there, where
    // the position in units would land inside the accented letter and be refused.
    final doc = await create();
    await doc.apply(Command.setText(cell: doc.root, text: sample));
    final after = sample.indexOf('e');
    expect(after, 3);
    await expectLater(
      doc.apply(insert(doc, after, '|')),
      refusedAs(ErrorKind.refused),
    );
    await doc.apply(insert(doc, codePoints(sample, after), '|'));
    expect(await text(doc), 'a\u{1F600}|éb');
    await doc.close();
  });

  test('a position outside the text is refused, not moved', () {
    expect(codePoints('abc', 3), 3);
    // What a selection gives when there is none.
    expect(() => codePoints('abc', -1), throwsRangeError);
    expect(() => codePoints('abc', 4), throwsRangeError);
    expect(() => codePoints('', 1), throwsRangeError);
  });

  test('a position of the core as an offset of a Dart string', () {
    // a, an emoji (two units), e with an accent of its own (two code points), a line
    // break, a flag (two code points of two units each), b.
    const sample = 'a\u{1F600}e\u{301}\n\u{1F1E9}\u{1F1EA}b';
    expect(sample.runes.length, 8);
    expect(sample.length, 11);
    expect(
      [for (var n = 0; n <= 8; n++) units(sample, n)],
      [0, 1, 3, 4, 5, 6, 8, 10, 11],
    );
    // There and back, at every position.
    for (var n = 0; n <= 8; n++) {
      expect(codePoints(sample, units(sample, n)), n);
    }
    // Half a pair on its own is one code point of one unit, either half.
    const halves = 'x\u{D83D}y\u{DE00}\u{DE00}z';
    expect(halves.runes.length, 6);
    expect(
      [for (var n = 0; n <= 6; n++) units(halves, n)],
      [0, 1, 2, 3, 4, 5, 6],
    );
    // The halves the other way round are no pair.
    expect(units('\u{DE00}\u{D83D}', 1), 1);
    // A high half at the very end.
    expect(units('a\u{D83D}', 2), 2);
    expect(units('', 0), 0);
  });

  test('a position outside the text is refused there too', () {
    expect(() => units('abc', -1), throwsRangeError);
    expect(() => units('abc', 4), throwsRangeError);
    expect(() => units('', 1), throwsRangeError);
    // Three code points in four units.
    expect(units('a\u{1F600}b', 3), 4);
    expect(() => units('a\u{1F600}b', 4), throwsRangeError);
  });

  const plain = Style(
    bold: false,
    italic: false,
    fixed: false,
    underline: false,
    strike: false,
  );
  const bold = Style(
    bold: true,
    italic: false,
    fixed: false,
    underline: false,
    strike: false,
  );

  test('the spans of a text of several lines are where its runs say', () async {
    // The line end arrives as CR LF and is one code point in the document; the run
    // that the core gives for the bold word after it is placed by `units`.
    final doc = await create();
    await doc.apply(
      Command.setText(cell: doc.root, text: 'one \u{1F600}\r\ntwo three'),
    );
    final text = (await doc.cell(doc.root)).text;
    expect(text, 'one \u{1F600}\ntwo three');
    final at = codePoints(text, text.indexOf('two'));
    expect(at, 6);
    await doc.apply(
      Command.setRangeStyle(
        cell: doc.root,
        start: at,
        len: 3,
        set_: bold,
        clear: plain,
      ),
    );
    final look = await doc.cell(doc.root);
    final run = look.runs.single;
    expect((run.start, run.len, run.style.bold), (6, 3, true));
    expect(
      look.text.substring(
        units(look.text, run.start),
        units(look.text, run.start + run.len),
      ),
      'two',
    );
    await doc.close();
  });

  test('the request of a phone is the one the bridge names', () {
    // The comment of `StageAsk` in the bridge names the core's starting values, and a
    // test there holds the comment to the core. This holds the app to the comment.
    final wire = File('../crates/wombat-bridge/src/api/wire.rs')
        .readAsStringSync()
        .replaceAll(RegExp(r'\s*\n\s*///\s*'), ' ');
    final said = RegExp(
      r'starting values for a phone are: the whole stage grid \(`x0` (\d+), '
      r'`y0` (\d+), `cols` and `rows` (\d+)\), `full_depth` (\d+), '
      r'`full_cells` (\d+), previews of (\d+) by (\d+) cells cut at (\d+) code '
      r'points, shapes of (\d+) by (\d+) cells, `text_cut` (\d+)\.',
    ).firstMatch(wire);
    expect(said, isNotNull, reason: 'the comment of StageAsk has changed');
    int n(int group) => int.parse(said!.group(group)!);
    expect(
      phoneAsk,
      StageAsk(
        x0: n(1),
        y0: n(2),
        cols: n(3),
        rows: n(3),
        fullDepth: n(4),
        fullCells: n(5),
        previewCols: n(6),
        previewRows: n(7),
        previewCut: n(8),
        shapeCols: n(9),
        shapeRows: n(10),
        textCut: n(11),
      ),
    );
    expect(phoneAsk.rows, 4294967295);
  });

  test('the stage and one cell of the example', () async {
    final doc = await openExample(folder, 'stage');
    final stage = await doc.stage(doc.root, phoneAsk);
    expect(stage.trail.map((step) => step.id), [doc.root]);
    final grid = stage.grids.first;
    expect((grid.owner, grid.level, grid.first), (doc.root, 0, 0));
    expect((grid.cols, grid.rows), (grid.xs, grid.ys));
    final cells = stage.cells;
    expect(cells.length, greaterThanOrEqualTo(grid.xs * grid.ys));
    // Every cell with a grid has its summary, in the order of the cells.
    final owners = [
      for (final cell in cells)
        if (cell.hasGrid) cell.id,
    ];
    expect(stage.summaries.map((summary) => summary.owner), owners);
    for (final cell in cells) {
      expect(cell.summary == null, !cell.hasGrid);
      if (cell.summary case final at?) {
        expect(stage.summaries[at].owner, cell.id);
      }
      if (cell.full case final at?) expect(stage.grids[at].owner, cell.id);
    }
    // Read again, every entry is equal; the answer as a whole is not.
    final again = await doc.stage(doc.root, phoneAsk);
    expect(again.cells, cells);
    expect(again.summaries, stage.summaries);
    expect(again.grids, stage.grids);
    expect(again.trail, stage.trail);
    expect(again == stage, isFalse);

    final one = await doc.cell(cells.first.id);
    expect(one.id, cells.first.id);
    expect((one.summary, one.full), (null, null));
    await expectLater(
      doc.stage('no ID', phoneAsk),
      refusedAs(ErrorKind.badRequest),
    );
    await expectLater(
      doc.stage(
        doc.root,
        const StageAsk(
          x0: -1,
          y0: 0,
          cols: 1,
          rows: 1,
          fullDepth: 0,
          fullCells: 0,
          previewCols: 0,
          previewRows: 0,
          shapeCols: 0,
          shapeRows: 0,
          textCut: 0,
          previewCut: 0,
        ),
      ),
      refusedAs(ErrorKind.badRequest),
    );
    await doc.close();
  });

  test('a group is one step, and a cancelled one is none', () async {
    final doc = await create();
    await doc.apply(Command.setText(cell: doc.root, text: 'start'));
    await doc.save();
    expect(await doc.inGroup(), isFalse);

    await doc.beginGroup();
    expect(await doc.inGroup(), isTrue);
    expect((await doc.retype(doc.root, 'started')).changed, [doc.root]);
    expect((await doc.retype(doc.root, 'started')).changed, isEmpty);
    await expectLater(doc.undo(), refusedAs(ErrorKind.refused));
    expect(await doc.isSaved(), isFalse);
    expect((await doc.cancelGroup()).changed, [doc.root]);
    expect(await doc.inGroup(), isFalse);
    expect(await text(doc), 'start');
    // Nothing is left to save of a cancelled group.
    expect(await doc.isSaved(), isTrue);

    await doc.beginGroup();
    await doc.retype(doc.root, 'one\r\ntwo');
    await doc.apply(Command.enterText(cell: doc.root, text: 'one\r\ntwo'));
    expect((await doc.endGroup()).changed, [doc.root]);
    // The line end is one code point in the document.
    expect(await text(doc), 'one\ntwo');
    expect((await doc.endGroup()).changed, isEmpty);
    expect((await doc.cancelGroup()).changed, isEmpty);
    expect((await doc.undo())!.changed, [doc.root]);
    expect(await text(doc), 'start');
    await doc.close();
  });

  test('the view state: a level gone to, remembered, folded, saved', () async {
    final doc = await openExample(folder, 'view');
    expect(await doc.viewStage(), doc.root);
    expect(await doc.isViewSaved(), isTrue);
    final stage = await doc.stage(doc.root, phoneAsk);
    final inner = stage.cells.firstWhere((cell) => cell.hasGrid);
    final plain = stage.cells.firstWhere((cell) => !cell.hasGrid);

    await doc.viewGo(
      inner.id,
      left: LevelInfo(cell: doc.root, x: 1, y: 2, selected: plain.id),
    );
    expect(await doc.viewStage(), inner.id);
    expect(await doc.isViewSaved(), isFalse);
    // Neither is a change to the document.
    expect(await doc.isSaved(), isTrue);
    final level = (await doc.stage(doc.root, phoneAsk)).level!;
    expect(
      (level.cell, level.x, level.y, level.selected),
      (doc.root, 1, 2, plain.id),
    );
    await doc.viewRemember(LevelInfo(cell: inner.id, x: 0, y: 1));
    expect((await doc.stage(inner.id, phoneAsk)).level!.y, 1);
    await expectLater(doc.viewGo(plain.id), refusedAs(ErrorKind.refused));
    expect(await doc.viewStage(), inner.id);

    expect(await doc.setFolded(inner.id, folded: true), isTrue);
    expect(await doc.setFolded(inner.id, folded: true), isFalse);
    expect((await doc.cell(inner.id)).folded, isTrue);
    await expectLater(
      doc.setFolded(plain.id, folded: true),
      refusedAs(ErrorKind.refused),
    );

    await doc.saveView();
    expect(await doc.isViewSaved(), isTrue);
    await doc.close();
    // The next start finds the level, the position and the fold.
    final again = await Session.open(doc.path);
    expect(await again.viewStage(), inner.id);
    expect((await again.stage(inner.id, phoneAsk)).level!.y, 1);
    expect((await again.cell(inner.id)).folded, isTrue);
    await again.close();
  });

  test('each new call waits for its turn, and is refused after the end', () async {
    final (doc, watched) = await watchExample(folder, 'turns');
    final stage = await doc.stage(doc.root, phoneAsk);
    final inner = stage.cells.firstWhere((cell) => cell.hasGrid).id;
    watched.log.clear();
    final level = LevelInfo(cell: doc.root, x: 0, y: 0);
    // Every call is slow to start, so that one made after it would overtake it.
    watched.before = (_) =>
        Future<void>.delayed(const Duration(milliseconds: 2));
    final calls = <String, Future<Object?> Function()>{
      'beginGroup': doc.beginGroup,
      'inGroup': doc.inGroup,
      'retype': () => doc.retype(doc.root, 'typed'),
      'stage': () => doc.stage(doc.root, phoneAsk),
      'cell': () => doc.cell(doc.root),
      'endGroup': doc.endGroup,
      'cancelGroup': doc.cancelGroup,
      'viewGo': () => doc.viewGo(inner, left: level),
      'viewStage': doc.viewStage,
      'viewRemember': () => doc.viewRemember(level),
      'setFolded': () => doc.setFolded(inner, folded: true),
      'isViewSaved': doc.isViewSaved,
      'saveView': doc.saveView,
    };
    // None is waited for before the next is made.
    final answers = [for (final call in calls.values) call()];
    await Future.wait(answers);
    expect(watched.log, [
      for (final name in calls.keys) ...[name, '$name.'],
    ]);
    expect(await answers[1], isTrue);
    expect(await answers[8], inner);
    expect(await answers[11], isFalse);

    watched.before = null;
    await doc.close();
    for (final call in calls.values) {
      await expectLater(call(), refusedAs(ErrorKind.closed));
    }
    // Refused by the session: nothing reached the handle.
    expect(watched.log.where((name) => name == 'saveView').length, 1);
  });

  test('discarding comes after the calls made before it', () async {
    final fake = FakeDocument();
    final doc = await Session.over(fake);
    final gate = holdFirst(fake, 'apply');
    final edit = doc.apply(const Command.setText(cell: 'a', text: '1'));
    final gone = doc.discard();
    await pumpEventQueue();
    expect(fake.disposed, isFalse);
    gate.complete();
    expect((await edit).changed, ['a']);
    await gone;
    expect(fake.disposed, isTrue);
    // A second time is nothing, and so is one after a close.
    await doc.discard();
    final closed = await Session.over(FakeDocument());
    await closed.close();
    await closed.discard();
  });

  test('a handle that cannot say what it is is let go of', () async {
    final fake = _NoRoot();
    await expectLater(Session.over(fake), refusedAs(ErrorKind.io));
    expect(fake.disposed, isTrue);
  });

  group('one piece at a time', () {
    test('many pieces, and one that throws at once', () async {
      final turns = InTurn();
      var n = 0;
      late Future<int> last;
      for (var i = 0; i < 100000; i++) {
        last = turns.run(() async => ++n);
      }
      expect(await last, 100000);
      // Not an asynchronous function: it throws before it answers.
      final bad = turns.run<int>(() => throw StateError('at once'));
      final after = turns.run(() async => 7);
      await expectLater(bad, throwsStateError);
      expect(await after, 7);
    });

    test('an error that no one waits for goes to the owner, once', () async {
      final unheard = <Object>[];
      final turns = InTurn(onUnheard: (error, _) => unheard.add(error));
      final lost = StateError('lost');
      final answer = turns.run<void>(() async => throw lost);
      final after = turns.run(() async => 1);
      expect(await after, 1);
      expect(unheard, [lost]);
      // Whoever takes the answer afterwards still finds the error in it.
      await expectLater(answer, throwsA(lost));
      expect(unheard, [lost]);
    });

    test(
      'an error that its caller waits for goes to the caller alone',
      () async {
        final unheard = <Object>[];
        final turns = InTurn(onUnheard: (error, _) => unheard.add(error));
        await expectLater(
          turns.run<void>(() async => throw StateError('heard')),
          throwsStateError,
        );
        // Each way of taking the answer counts.
        final caught = turns
            .run<int>(() async => throw StateError('caught'))
            .catchError((Object _) => 0);
        final timed = turns
            .run<int>(() async => throw StateError('timed'))
            .timeout(const Duration(hours: 1))
            .catchError((Object _) => 0);
        final finished = turns
            .run<int>(() async => throw StateError('finished'))
            .whenComplete(() {})
            .catchError((Object _) => 0);
        final streamed = turns
            .run<int>(() async => throw StateError('streamed'))
            .asStream()
            .handleError((Object _) {})
            .drain<void>();
        expect((await caught, await timed, await finished), (0, 0, 0));
        await streamed;
        expect(unheard, isEmpty);
      },
    );

    test(
      'without an owner to tell, an unheard error is not swallowed',
      () async {
        final uncaught = <Object>[];
        await runZonedGuarded(() async {
          final turns = InTurn();
          unawaited(turns.run<void>(() async => throw StateError('lost')));
          expect(await turns.run(() async => 1), 1);
        }, (error, _) => uncaught.add(error));
        expect(uncaught.single, isStateError);
      },
    );

    test(
      'an owner that fails to take an error does not stop the queue',
      () async {
        final uncaught = <Object>[];
        await runZonedGuarded(() async {
          final turns = InTurn(onUnheard: (_, _) => throw StateError('owner'));
          unawaited(turns.run<void>(() async => throw ArgumentError('lost')));
          expect(await turns.run(() async => 1), 1);
        }, (error, _) => uncaught.add(error));
        expect(uncaught.single, isStateError);
      },
    );

    test('a piece that asks its own queue for more fails at once', () async {
      final turns = InTurn();
      final other = InTurn();
      var ran = false;
      final outer = turns.run(
        () => turns.run(() async {
          ran = true;
          return 1;
        }),
      );
      await expectLater(
        outer.timeout(const Duration(seconds: 5)),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            contains('its own queue'),
          ),
        ),
      );
      expect(ran, isFalse);
      // The queue is not stuck, another queue may be asked from inside a piece, and so
      // may this one by something that a piece left behind and that runs after it.
      expect(await turns.run(() => other.run(() async => 2)), 2);
      late Future<int> later;
      await turns.run(() async {
        Timer.run(() => later = turns.run(() async => 3));
      });
      await pumpEventQueue();
      expect(await later, 3);
      // The same after a piece that failed.
      await expectLater(
        turns.run<void>(() async {
          Timer.run(() => later = turns.run(() async => 4));
          throw StateError('failed');
        }),
        throwsStateError,
      );
      await pumpEventQueue();
      expect(await later, 4);
    });

    // The fourth review's (P1): the mark of a piece went with everything the piece
    // started, so the keeper's save clock was refused while the step that made the
    // change still ran.
    test('a timer that a piece left may ask while the piece still runs', () async {
      final turns = InTurn();
      final gate = Completer<void>();
      final asked = Completer<int>();
      final ticks = <Future<int>>[];
      final piece = turns.run(() async {
        Timer(Duration.zero, () => asked.complete(turns.run(() async => 5)));
        final tick = Timer.periodic(
          const Duration(milliseconds: 1),
          (_) => ticks.add(turns.run(() async => 6)),
        );
        await Future<void>.delayed(const Duration(milliseconds: 20));
        tick.cancel();
        // What the piece itself goes on to do after a wait is still the piece.
        await expectLater(turns.run(() async => 0), throwsStateError);
        await gate.future;
      });
      await Future<void>.delayed(const Duration(milliseconds: 40));
      gate.complete();
      await piece;
      expect(await asked.future, 5);
      expect(ticks, isNotEmpty);
      expect(await Future.wait(ticks), everyElement(6));
    });

    test('work that a piece sets apart may ask the queue', () async {
      final turns = InTurn();
      late Future<int> later;
      await turns.run(() async {
        InTurn.apart(() => later = turns.run(() async => 8));
        // The piece itself is refused as before.
        await expectLater(turns.run(() async => 0), throwsStateError);
      });
      expect(await later, 8);
    });

    // The fourth review's (P4): each of two queues waited for the other, for ever.
    test('two queues that would wait for each other: refused', () async {
      final a = InTurn();
      final b = InTurn();
      final third = InTurn();
      final gate = Completer<void>();
      // A piece of b, asked from outside, asks a after a while; meanwhile the piece
      // of a asks b, through a third queue for good measure.
      final fromB = b.run(() async {
        await gate.future;
        return a.run(() async => 1);
      });
      final fromA = a.run(() async {
        final asked = third.run(() => b.run(() async => 2));
        gate.complete();
        return asked;
      });
      await expectLater(
        fromB.timeout(const Duration(seconds: 5)),
        throwsA(
          isA<StateError>().having(
            (e) => e.message,
            'message',
            contains('wait for each other'),
          ),
        ),
      );
      expect(await fromA.timeout(const Duration(seconds: 5)), 2);
      // Asking the other queue is fine while it does not wait for this one, and
      // again once the piece that did has ended.
      expect(await b.run(() => a.run(() async => 3)), 3);
      expect(await a.run(() => third.run(() => b.run(() async => 4))), 4);
    });

    test(
      'a wait that is over is forgotten, and so is a piece that has ended',
      () async {
        final a = InTurn();
        final b = InTurn();
        final asked = Completer<void>();
        final gate = Completer<void>();
        final fromA = a.run(() async {
          await b.run(() async {});
          asked.complete();
          await gate.future;
        });
        await asked.future;
        // The piece of a still runs, and no longer waits for b: a piece of b may ask a.
        final fromB = b.run(() => a.run(() async => 9));
        await pumpEventQueue();
        gate.complete();
        await fromA;
        expect(await fromB.timeout(const Duration(seconds: 5)), 9);

        // What a piece started and did not wait for may ask the queue once the piece
        // has ended: it is inside nothing that runs.
        final ended = Completer<void>();
        late Future<int> later;
        final left = Completer<void>();
        await a.run(() async {
          unawaited(() async {
            await ended.future;
            later = a.run(() async => 3);
            left.complete();
          }());
        });
        ended.complete();
        await left.future;
        expect(await later, 3);
      },
    );

    test('a piece is joined by what asks from inside it', () async {
      final turns = InTurn();
      final order = <String>[];
      final gate = Completer<void>();
      final first = turns.run(() async {
        await gate.future;
        // Its turn is now: it does not wait for itself.
        expect(await turns.join(() async => 'joined'), 'joined');
        order.add('first');
      });
      // From outside it waits for its turn like any other.
      final second = turns.join(() async => order.add('second'));
      gate.complete();
      await first;
      await second;
      expect(order, ['first', 'second']);
      await expectLater(
        turns.run(() => turns.join<void>(() async => throw StateError('x'))),
        throwsStateError,
      );
    });

    test('work done for no one: its error always goes to the owner', () async {
      final unheard = <Object>[];
      final turns = InTurn(onUnheard: (error, _) => unheard.add(error));
      final order = <int>[];
      turns.post(() async => order.add(1));
      turns.post(() async => throw StateError('posted'));
      turns.post(() async => order.add(2));
      // Also the refusal, when a piece posts to its own queue.
      await turns.run(() async => turns.post(() async => order.add(3)));
      await turns.run(() async {});
      expect(order, [1, 2]);
      expect(unheard.map((e) => (e as StateError).message), [
        'posted',
        contains('its own queue'),
      ]);
    });

    // The fourth review's (P3). An asynchronous function that returns the answer has
    // taken it: the error is in what that function answers, and is its caller's.
    test('an answer passed on by an async function is heard by it', () async {
      final unheard = <Object>[];
      final uncaught = <Object>[];
      final turns = InTurn(onUnheard: (error, _) => unheard.add(error));
      Future<void> wrapped() async =>
          turns.run<void>(() async => throw StateError('wrapped'));
      await runZonedGuarded(() async {
        unawaited(wrapped());
        await turns.run(() async {});
        await pumpEventQueue();
      }, (error, _) => uncaught.add(error));
      // Told once, where Dart tells of a future nobody listens to.
      expect(unheard, isEmpty);
      expect(uncaught.single, isStateError);
    });
  });

  test('a fault in the program is told in a fixed sentence', () {
    final panic = PanicException('panicked at /home/builder/src/core.rs:1:1');
    const after = BridgeError(
      kind: ErrorKind.internal,
      message: 'a fault in the program',
    );
    const refused = BridgeError(kind: ErrorKind.refused, message: 'not so');
    expect(isFatal(panic), isTrue);
    expect(isFatal(after), isTrue);
    expect(isFatal(refused), isFalse);
    expect(isFatal(StateError('x')), isFalse);
    expect(describeError(panic), faultSentence);
    expect(describeError(after), faultSentence);
    expect(describeError(refused), 'Error (refused): not so');
    expect(faultSentence, isNot(contains('/')));
  });
}

/// A handle that fails when asked for its root.
class _NoRoot extends FakeDocument {
  @override
  Future<String> root() async =>
      throw const BridgeError(kind: ErrorKind.io, message: 'no root');
}
