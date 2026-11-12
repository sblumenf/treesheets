// One editing of a cell's text through the keeper (`Editing`): nothing sent while
// typing, one step of the undo history or none, the cross exact, other writers kept
// out, and what the app being put away, closing, another program's file and a fault
// do to an editing that is open. On the stand-in where order and failures must be
// made, and on the real core for what only it has: styles, links, formulas, the
// histories and the file.
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
    folder = await Directory.systemTemp.createTemp('wombat-keeper-editing');
  });

  tearDownAll(() => folder.delete(recursive: true));

  const refused = BridgeError(kind: ErrorKind.refused, message: 'not this');

  Command typing(String cell, String text) =>
      Command.enterText(cell: cell, text: text);

  Matcher refusedAs(ErrorKind kind) =>
      throwsA(isA<BridgeError>().having((e) => e.kind, 'kind', kind));

  int calls(Hooks doc, String call) =>
      doc.log.where((name) => name == call).length;

  /// The calls that began, without those that ended.
  List<String> begun(Hooks doc) => [
    for (final name in doc.log)
      if (!name.endsWith('.')) name,
  ];

  const saved = KeeperState.saved;
  const unsaved = KeeperState.unsaved;

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
      await endKeeping(keeper);
    });
  }

  group('an editing', () {
    keeperTest('opens on the cell as it is, and sends nothing', (
      tester,
      keeper,
      fake,
    ) async {
      fake.cells['a']!.text = 'x' * 5000;
      final editing = await keeper.edit('a');
      expect(editing.cell, 'a');
      // Whole, where a stage read cuts.
      expect((editing.start.text.length, editing.start.textCut), (5000, false));
      expect((editing.open, editing.held), (true, false));
      expect(identical(keeper.editing, editing), isTrue);
      expect(fake.log, ['cell', 'cell.']);
      expect(await Session.over(fake).then((doc) => doc.inGroup()), isFalse);
      await expectLater(keeper.edit('nowhere'), refusedAs(ErrorKind.notFound));
      // The one that is open stays so when another could not be opened.
      expect((editing.open, identical(keeper.editing, editing)), (true, true));
    });

    keeperTest('the tick: one turn of the keeper, one step to undo', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      final editing = await keeper.edit('a');
      fake.log.clear();
      final gate = holdFirst(fake, 'retype');
      final entered = editing.enter('typed');
      // Asked while the editing is at work: it comes after all of it.
      final after = keeper.apply(typing('d', 'after'));
      await tester.pump();
      expect(begun(fake), ['beginGroup', 'retype']);
      gate.complete();
      expect((await entered).changed, ['a']);
      await after;
      expect(begun(fake), [
        'beginGroup',
        'retype',
        'apply',
        'endGroup',
        'stage',
        'apply',
        'stage',
      ]);
      expect(
        (editing.open, editing.held, keeper.editing),
        (false, false, null),
      );
      expect(reader.stage.look('a')!.text, 'typed');
      expect(keeper.state, unsaved);
      await tester.pump(const Duration(seconds: 1));
      expect((fake.saves, keeper.state), (1, saved));

      // The step after it first, then the editing as one.
      await keeper.undo();
      expect(fake.cells['a']!.text, 'typed');
      await keeper.undo();
      expect(fake.cells['a']!.text, 'one');
      expect(await keeper.undo(), isNull);
    });

    keeperTest('a text the cell has already leaves nothing to undo or save', (
      tester,
      keeper,
      fake,
    ) async {
      final editing = await keeper.edit('a');
      expect((await editing.enter('one')).changed, isEmpty);
      expect((keeper.state, editing.open), (saved, false));
      expect(await keeper.undo(), isNull);
      await tester.pump(const Duration(minutes: 5));
      expect(calls(fake, 'save'), 0);
    });

    keeperTest('the cross with nothing held sends nothing at all', (
      tester,
      keeper,
      fake,
    ) async {
      final editing = await keeper.edit('a');
      fake.log.clear();
      final report = await editing.drop();
      expect(report.changed, isEmpty);
      expect(fake.log, isEmpty);
      expect((editing.open, keeper.editing), (false, null));
    });

    keeperTest('held, then the cross: as if it had never been', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      await keeper.apply(typing('d', 'first'));
      await keeper.apply(typing('d', 'second'));
      await keeper.undo();
      await keeper.save();
      final saves = fake.saves;

      final editing = await keeper.edit('a');
      final held = await editing.hold('typed so far');
      expect(held.text, 'typed so far');
      expect((editing.open, editing.held), (true, true));
      expect(fake.cells['a']!.text, 'typed so far');
      expect(reader.stage.look('a')!.text, 'typed so far');
      expect(keeper.state, unsaved);
      // The same text again changes nothing, and opens no second group.
      await editing.hold('typed so far');
      expect(calls(fake, 'beginGroup'), 1);

      final report = await editing.drop();
      expect(report.changed, ['a']);
      expect(
        (editing.open, editing.held, keeper.editing),
        (false, false, null),
      );
      expect(fake.cells['a']!.text, 'one');
      expect(reader.stage.look('a')!.text, 'one');
      // Nothing is left to save, and nothing is saved by the clock.
      expect(keeper.state, saved);
      await tester.pump(const Duration(minutes: 5));
      expect(fake.saves, saves);
      // Both histories are as before the editing.
      expect((await keeper.redo())!.changed, ['d']);
      expect(fake.cells['d']!.text, 'second');
      await keeper.undo();
      await keeper.undo();
      expect(fake.cells['d']!.text, 'four');
      expect(await keeper.undo(), isNull);
    });

    keeperTest('held and saved, then the cross: the file is put right', (
      tester,
      keeper,
      fake,
    ) async {
      final editing = await keeper.edit('a');
      await editing.hold('typed so far');
      // The clock saves what is held, and the editing stays as it is.
      await tester.pump(const Duration(seconds: 1));
      expect((fake.saves, keeper.state), (1, saved));
      expect((editing.open, editing.held), (true, true));
      await editing.drop();
      // The file holds what was taken back.
      expect((keeper.state, fake.saved), (unsaved, false));
      await tester.pump(const Duration(seconds: 1));
      expect(
        (fake.saves, keeper.state, fake.cells['a']!.text),
        (2, saved, 'one'),
      );
    });

    keeperTest('dropped while a save is being tried again: that ends well', (
      tester,
      keeper,
      fake,
    ) async {
      final editing = await keeper.edit('a');
      await editing.hold('typed');
      failFirst(
        fake,
        'save',
        const BridgeError(kind: ErrorKind.io, message: ''),
      );
      await keeper.save();
      expect(keeper.state, KeeperState.failed);
      await editing.drop();
      // Nothing was written, so nothing is left to save; the try that is due finds
      // that out.
      expect(keeper.state, KeeperState.failed);
      await tester.pump(const Duration(seconds: 1));
      expect((keeper.state, keeper.saveError, fake.saves), (saved, null, 0));
    });

    keeperTest('the text can come back different from what was sent', (
      tester,
      keeper,
      fake,
    ) async {
      final editing = await keeper.edit('a');
      final held = await editing.hold('one\r\ntwo\tthree');
      expect(held.text, 'one\ntwo three');
      await editing.enter('one\r\ntwo\tthree');
      expect((await keeper.cell('a')).text, 'one\ntwo three');
    });

    keeperTest('a text that is refused: the editing is as it was', (
      tester,
      keeper,
      fake,
    ) async {
      final editing = await keeper.edit('a');
      // With nothing held before: no group is left open for it.
      failFirst(fake, 'retype', refused);
      await expectLater(editing.enter('typed'), throwsA(refused));
      expect((editing.open, editing.held), (true, false));
      expect(begun(fake).sublist(1), [
        'beginGroup',
        'retype',
        'cancelGroup',
        'isSaved',
      ]);
      expect((fake.cells['a']!.text, keeper.state), ('one', saved));
      await tester.pump(const Duration(minutes: 5));
      expect(calls(fake, 'save'), 0);
      // The text was taken and its kind could not be settled: taken back too.
      failFirst(fake, 'apply', refused);
      await expectLater(editing.hold('typed'), throwsA(refused));
      expect(
        (editing.held, fake.cells['a']!.text, keeper.state),
        (false, 'one', saved),
      );
      // Others are not kept out by an editing that holds nothing.
      await keeper.apply(typing('d', 'other'));
      await keeper.undo();

      // With something held before: that stays, and so does the group.
      await editing.hold('held');
      failFirst(fake, 'retype', refused);
      await expectLater(editing.enter('more'), throwsA(refused));
      expect((editing.open, editing.held), (true, true));
      expect(fake.cells['a']!.text, 'held');
      expect(calls(fake, 'cancelGroup'), 2);
      failFirst(fake, 'retype', refused);
      await expectLater(editing.hold('more'), throwsA(refused));
      expect((editing.held, fake.cells['a']!.text), (true, 'held'));
      // The next try works, and the whole editing is one step.
      await editing.enter('more');
      await keeper.undo();
      expect(fake.cells['a']!.text, 'one');
    });

    keeperTest('while it holds something, everyone else is refused loudly', (
      tester,
      keeper,
      fake,
    ) async {
      final editing = await keeper.edit('a');
      // Not before: nothing of it is in the document.
      await keeper.apply(typing('d', 'before'));
      await editing.hold('typed');
      fake.log.clear();
      await expectLater(keeper.apply(typing('d', 'x')), throwsStateError);
      await expectLater(keeper.undo(), throwsStateError);
      await expectLater(keeper.redo(), throwsStateError);
      await expectLater(keeper.edit('d'), throwsStateError);
      // Nothing of it reached the document, and the keeper works on.
      expect(fake.log, isEmpty);
      expect((keeper.ended, editing.open, editing.held), (null, true, true));
      expect(identical(keeper.editing, editing), isTrue);
      // Reading and saving are no change.
      await keeper.grid('root');
      await keeper.cell('a');
      await keeper.save();
      expect(fake.saves, 1);

      await editing.enter('typed');
      await keeper.apply(typing('d', 'after'));
      expect((await keeper.undo())!.changed, ['d']);
    });

    keeperTest('another editing takes the place of one that holds nothing', (
      tester,
      keeper,
      fake,
    ) async {
      final first = await keeper.edit('a');
      final second = await keeper.edit('d');
      expect((first.open, second.open), (false, true));
      expect(identical(keeper.editing, second), isTrue);
      fake.log.clear();
      await expectLater(first.enter('typed'), throwsStateError);
      await expectLater(first.hold('typed'), throwsStateError);
      await expectLater(first.drop(), throwsStateError);
      await expectLater(
        first.style('typed', (held) => typing('a', 'x')),
        throwsStateError,
      );
      expect(fake.log, isEmpty);
      // Nor does the one that is over end the one that is open.
      expect((second.open, identical(keeper.editing, second)), (true, true));
      await second.enter('typed');
      await expectLater(second.enter('again'), throwsStateError);
      await expectLater(second.drop(), throwsStateError);
      expect(fake.cells['d']!.text, 'typed');
    });
  });

  group('an editing that is open when', () {
    keeperTest('the app is put away: what is typed is in the file, and the '
        'editing goes on', (tester, keeper, fake) async {
      final editing = await keeper.edit('a');
      // A screen holds its editor's text when the keeper asks it to finish.
      keeper.attach(finish: () => editing.hold('half a sentence'));
      expect(await keeper.putAway(), isEmpty);
      expect((fake.saves, fake.saved, keeper.state), (1, true, saved));
      expect(fake.cells['a']!.text, 'half a sentence');
      expect((editing.open, editing.held), (true, true));
      // Put away again with nothing new: nothing is written again.
      await keeper.putAway();
      expect(fake.saves, 1);
      // Back in front, the cross still takes it all back.
      await editing.drop();
      expect((fake.cells['a']!.text, keeper.state), ('one', unsaved));
    });

    keeperTest('the document is closed: what it holds is saved, and it ends '
        'with the document', (tester, keeper, fake) async {
      final editing = await keeper.edit('a');
      await editing.hold('held');
      await keeper.close();
      expect(
        (fake.closed, fake.saves, fake.cells['a']!.text),
        (true, 1, 'held'),
      );
      expect(
        (editing.open, editing.held, keeper.editing),
        (false, false, null),
      );
      // The keeper did not end the group or cancel it.
      expect(calls(fake, 'endGroup') + calls(fake, 'cancelGroup'), 0);
      await expectLater(editing.drop(), refusedAs(ErrorKind.closed));
    });

    keeperTest('a close fails: it is as open as before', (
      tester,
      keeper,
      fake,
    ) async {
      final editing = await keeper.edit('a');
      await editing.hold('held');
      fake.fileChanged = true;
      await expectLater(keeper.close(), refusedAs(ErrorKind.changedOnDisk));
      expect((editing.open, editing.held), (true, true));
      await expectLater(keeper.undo(), throwsStateError);
      fake.fileChanged = false;
      await editing.enter('entered');
      expect(fake.cells['a']!.text, 'entered');
    });

    keeperTest('the document is let go of', (tester, keeper, fake) async {
      final editing = await keeper.edit('a');
      await editing.hold('held');
      await keeper.letGo();
      expect(
        (editing.open, editing.held, keeper.editing),
        (false, false, null),
      );
      // What the bridge saves of a document that is let go of has what was held.
      expect((fake.saves, fake.cells['a']!.text), (1, 'held'));
    });

    keeperTest(
      'the other program\'s file is opened: over, and nothing of it '
      'in the other document',
      (tester, keeper, fake) async {
        final editing = await keeper.edit('a');
        await editing.hold('mine');
        fake.fileChanged = true;
        await keeper.save();
        // Theirs cannot be opened: nothing changed, the editing is still there.
        await expectLater(keeper.openTheirs(), throwsA(refused));
        expect((editing.open, editing.held), (true, true));
        await keeper.openTheirs();
        expect(
          (editing.open, editing.held, keeper.editing),
          (false, false, null),
        );
        await expectLater(editing.enter('mine'), throwsStateError);
        // The other document is free for whoever comes.
        expect(await second.inGroup(), isFalse);
        await keeper.apply(typing('a', 'theirs, edited'));
        final next = await keeper.edit('a');
        await next.enter('and again');
        expect(second.cells['a']!.text, 'and again');
      },
      reopen: (_) async => Session.over(second = FakeDocument()),
      canBeRead: (_) async {
        if (tries++ == 0) throw refused;
      },
    );

    keeperTest('a fault ends the work', (tester, keeper, fake) async {
      final editing = await keeper.edit('a');
      await editing.hold('held');
      failFirst(fake, 'retype', PanicException('in the core'));
      await expectLater(editing.enter('more'), throwsA(isA<PanicException>()));
      expect(keeper.ended, faultSentence);
      expect(
        (editing.open, editing.held, keeper.editing),
        (false, false, null),
      );
      // No try to cancel a group of a document that may be half changed.
      expect(calls(fake, 'cancelGroup'), 0);
      expect(fake.saves, 0);
      await expectLater(editing.drop(), refusedAs(ErrorKind.closed));
    });
  });

  group('on the real core', () {
    var count = 0;
    final keepers = <Keeper>[];
    tearDown(() async {
      for (final keeper in keepers) {
        await endKeeping(keeper);
      }
      keepers.clear();
    });

    Future<(Keeper, Session, Watched)> kept() async {
      final (doc, watch) = await watchExample(folder, 'edited-${count++}');
      final keeper = Keeper(doc, saveAfter: const Duration(hours: 1));
      keepers.add(keeper);
      return (keeper, doc, watch);
    }

    /// The ID of the cell of the example that shows `text`.
    Future<String> cellOf(Keeper keeper, String text) async =>
        (await keeper.stage(
          keeper.root,
          phoneAsk,
        )).cells.firstWhere((c) => c.text == text).id;

    const none = Style(
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

    test('held, styled and dropped: document, both histories and the saved '
        'flag exactly as before', () async {
      final (keeper, doc, _) = await kept();
      final cell = await cellOf(keeper, 'what to buy');
      final other = await cellOf(keeper, 'Total');
      await keeper.apply(typing(other, 'first'));
      await keeper.apply(typing(other, 'second'));
      await keeper.undo();
      await keeper.save();
      final file = File(doc.path).readAsStringSync();
      final before = await keeper.stage(keeper.root, phoneAsk);

      final editing = await keeper.edit(cell);
      expect(editing.start.text, 'what to buy');
      await editing.hold('what to buy today');
      await editing.style(
        'what to buy today',
        (held) => Command.setRangeStyle(
          cell: cell,
          start: 0,
          len: 4,
          set_: bold,
          clear: none,
        ),
      );
      expect((await keeper.cell(cell)).runs.single.len, 4);
      expect((keeper.state, await doc.isSaved()), (unsaved, false));
      await editing.drop();

      expect(
        (keeper.state, await doc.isSaved(), await doc.inGroup()),
        (saved, true, false),
      );
      final after = await keeper.stage(keeper.root, phoneAsk);
      expect(after.cells, before.cells);
      expect(after.summaries, before.summaries);
      // What could be redone still can, and what could be undone too.
      expect((await keeper.redo())!.changed, [other]);
      expect((await keeper.cell(other)).text, 'second');
      await keeper.undo();
      await keeper.undo();
      expect((await keeper.cell(other)).text, 'Total');
      expect(await keeper.undo(), isNull);
      // And closing writes nothing that was not there.
      await keeper.redo();
      await keeper.close();
      expect(File(doc.path).readAsStringSync(), file);
    });

    test('entered: the styles and links of what did not change are kept, '
        'and it is one step with the style made on the way', () async {
      final (keeper, doc, _) = await kept();
      final cell = await cellOf(keeper, 'what to buy');
      await keeper.apply(
        Command.setRangeStyle(
          cell: cell,
          start: 0,
          len: 4,
          set_: bold,
          clear: none,
        ),
      );
      await keeper.apply(
        Command.setLink(
          cell: cell,
          start: 8,
          len: 3,
          target: const LinkTarget.external_(target: 'https://example.org'),
        ),
      );
      final styled = await keeper.cell(cell);
      expect((styled.runs.length, styled.links.length), (1, 1));

      // "what to buy" becomes "what", a line break, "we buy": the bold word before
      // what changed and the link after it stay.
      final editing = await keeper.edit(cell);
      expect(editing.start.runs, styled.runs);
      final held = await editing.style(
        'what\r\nwe buy',
        (held) => Command.setRangeStyle(
          cell: cell,
          // Counted in what the document gave back: the line end is one.
          start: codePoints(held.text, held.text.indexOf('we')),
          len: 2,
          set_: const Style(
            bold: false,
            italic: true,
            fixed: false,
            underline: false,
            strike: false,
          ),
          clear: none,
        ),
      );
      expect(held.text, 'what\nwe buy');
      final report = await editing.enter('what\r\nwe buy');
      expect(report.changed, [cell]);
      final look = await keeper.cell(cell);
      expect(look.text, 'what\nwe buy');
      expect(look.runs.map((run) => (run.start, run.len, run.style.bold)), [
        (0, 4, true),
        (5, 2, false),
      ]);
      expect(look.runs.last.style.italic, isTrue);
      expect(look.links.map((link) => (link.start, link.len)), [(8, 3)]);
      expect(await doc.inGroup(), isFalse);

      // One step back: the text, the line and the italics, and not the link.
      await keeper.undo();
      expect(await keeper.cell(cell), styled);
      await keeper.redo();
      expect(await keeper.cell(cell), look);
    });

    test('what is typed settles what the cell is, as typing does', () async {
      final (keeper, doc, _) = await kept();
      final cell = await cellOf(keeper, 'Total');
      final sum = await cellOf(keeper, '=SUM(Costs)');
      // Held, a formula is one already: the file of an app that is ended then
      // has a cell that works.
      final editing = await keeper.edit(cell);
      final held = await editing.hold('=1+2');
      expect((held.kind, held.result), (CellKind.formula, '3'));
      await editing.drop();
      expect((await keeper.cell(cell)).kind, CellKind.data);

      // A formula cell whose text is no formula any more is none.
      final other = await keeper.edit(sum);
      expect(
        (other.start.kind, other.start.text),
        (CellKind.formula, '=SUM(Costs)'),
      );
      await other.enter('just words');
      final look = await keeper.cell(sum);
      expect(
        (look.kind, look.result, look.text),
        (CellKind.data, null, 'just words'),
      );
      await keeper.undo();
      expect((await keeper.cell(sum)).result, '10');
      expect(await doc.inGroup(), isFalse);
    });

    test(
      'a kind that cannot be settled: the text is taken back with it',
      () async {
        final (keeper, doc, watch) = await kept();
        final cell = await cellOf(keeper, 'what to buy');
        await keeper.save();
        final editing = await keeper.edit(cell);
        // The core took the text and then refuses the second step.
        failFirst(watch, 'apply', refused);
        await expectLater(editing.enter('typed'), throwsA(refused));
        expect((editing.open, editing.held), (true, false));
        expect((await doc.inGroup(), await doc.isSaved()), (false, true));
        expect((await keeper.cell(cell)).text, 'what to buy');
        expect(keeper.state, saved);
        expect(await keeper.undo(), isNull);
      },
    );

    test(
      'put away with an editor open: in the file, and still one step',
      () async {
        final (keeper, doc, _) = await kept();
        final cell = await cellOf(keeper, 'what to buy');
        final editing = await keeper.edit(cell);
        keeper.attach(finish: () => editing.hold('half a sent'));
        expect(await keeper.putAway(), isEmpty);
        expect(File(doc.path).readAsStringSync(), contains('"half a sent"'));
        expect((keeper.state, await doc.inGroup()), (saved, true));

        // The app comes back, more is typed, and the tick ends it.
        await editing.enter('half a sentence');
        await keeper.save();
        expect(
          File(doc.path).readAsStringSync(),
          contains('"half a sentence"'),
        );
        await keeper.undo();
        expect((await keeper.cell(cell)).text, 'what to buy');
        expect(await keeper.undo(), isNull);
      },
    );

    test(
      'put away, and then the cross: the file gets the old text back',
      () async {
        final (keeper, doc, _) = await kept();
        final cell = await cellOf(keeper, 'what to buy');
        final editing = await keeper.edit(cell);
        await editing.hold('not this');
        await keeper.save();
        expect(File(doc.path).readAsStringSync(), contains('"not this"'));
        await editing.drop();
        expect((keeper.state, await doc.isSaved()), (unsaved, false));
        await keeper.save();
        expect(File(doc.path).readAsStringSync(), isNot(contains('not this')));
        expect(await keeper.undo(), isNull);
      },
    );

    test('closed with an editor open: the next start has the text', () async {
      final (keeper, doc, _) = await kept();
      final cell = await cellOf(keeper, 'Total');
      final editing = await keeper.edit(cell);
      await editing.hold('=2*3');
      await keeper.close();
      expect(editing.open, isFalse);
      final again = await Session.open(doc.path);
      final look = await again.cell(cell);
      expect(
        (look.text, look.kind, look.result),
        ('=2*3', CellKind.formula, '6'),
      );
      expect(await again.inGroup(), isFalse);
      await again.close();
    });
  });
}

late FakeDocument second;
var tries = 0;
