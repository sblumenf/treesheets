// The session: calls to one document in the order they were made, errors, closing.
// Against the real core, and against a stand-in where the order must be seen.
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
