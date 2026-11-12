// The keeper's part in what a screen draws: the stage kept up to date for every
// screen (`StageReader`), and where the screen is on this device (the view state).
// On the stand-in for the core where order, failures and time must be made, and on
// the real core for what only it does (cells that go away, files).
import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';
import 'package:wombat_app/stage_read.dart';

import 'fake_document.dart';
import 'support.dart';

StageAsk rowsOf(int rows, {int fullDepth = 1}) => StageAsk(
  x0: 0,
  y0: 0,
  cols: 4294967295,
  rows: rows,
  fullDepth: fullDepth,
  fullCells: 24,
  previewCols: 3,
  previewRows: 4,
  shapeCols: 12,
  shapeRows: 20,
  textCut: 2000,
  previewCut: 40,
);

void main() {
  late Directory folder;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-keeper-stage');
  });

  tearDownAll(() => folder.delete(recursive: true));

  const broken = BridgeError(kind: ErrorKind.io, message: 'the disk is gone');
  const gone = BridgeError(kind: ErrorKind.notFound, message: 'no such cell');

  Command typing(String cell, String text) =>
      Command.enterText(cell: cell, text: text);

  Matcher refusedAs(ErrorKind kind) =>
      throwsA(isA<BridgeError>().having((e) => e.kind, 'kind', kind));

  int calls(Hooks doc, String call) =>
      doc.log.where((name) => name == call).length;

  /// Every change a reader tells its listeners of, in order.
  List<StageChange> told(StageReader reader) {
    final seen = <StageChange>[];
    reader.addListener(seen.add);
    return seen;
  }

  /// A test with a keeper on the stand-in, on the clock of the test.
  void keeperTest(
    String name,
    Future<void> Function(WidgetTester tester, Keeper keeper, FakeDocument fake)
    body, {
    Future<Session> Function(String path)? reopen,
  }) {
    testWidgets(name, (tester) async {
      final fake = FakeDocument();
      final keeper = Keeper(
        await Session.over(fake),
        reopen: reopen ?? Session.open,
        canBeRead: (_) async {},
      );
      await body(tester, keeper, fake);
      await endKeeping(keeper);
    });
  }

  group('the stage, read again after every change', () {
    keeperTest('starts at the root, and has the change when the call answers', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      expect(reader.stage.cell, 'root');
      expect(reader.stage.ask, phoneAsk);
      expect(reader.change.everything, isTrue);
      expect(reader.readError, isNull);
      expect(calls(fake, 'stage'), 1);
      final seen = told(reader);

      final report = await keeper.apply(typing('a', 'typed'));
      expect(report.changed, ['a']);
      // Read already, and told once, with exactly what differs.
      expect(reader.stage.look('a')!.text, 'typed');
      expect(calls(fake, 'stage'), 2);
      expect(seen.length, 1);
      expect(seen.single.looks, {'a'});
      expect(seen.single.summaries, isEmpty);
      expect(identical(reader.change, seen.single), isTrue);

      // The read is part of the same turn: nothing comes between the two.
      expect(fake.log.sublist(fake.log.length - 4), [
        'apply',
        'apply.',
        'stage',
        'stage.',
      ]);
    });

    keeperTest('a command that changes nothing reads nothing', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      final seen = told(reader);
      await keeper.apply(typing('a', 'one'));
      expect(await keeper.undo(), isNull);
      expect(await keeper.redo(), isNull);
      await keeper.grid('root');
      await keeper.save();
      expect(calls(fake, 'stage'), 1);
      expect(seen, isEmpty);
    });

    keeperTest('undo and redo are read, and so is what only formulas show', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      final seen = told(reader);
      await keeper.apply(typing('e', 'inside'));
      expect(seen.last.looks, {'e'});
      expect(seen.last.summaries, {'c'});
      await keeper.undo();
      expect(reader.stage.look('e')!.text, 'five');
      await keeper.redo();
      expect(reader.stage.look('e')!.text, 'inside');
      expect(seen.length, 3);

      // A report that names nothing but a cell that shows something else (`Run`):
      // no change to the document, and yet the screen is old.
      fake.reports = (report) => const Report(
        changed: [],
        created: [],
        removed: [],
        grids: [],
        tags: [],
        recalculated: ['f'],
      );
      fake.cells['f']!.result = '3';
      final before = calls(fake, 'stage');
      await keeper.apply(typing('a', 'one'));
      expect(calls(fake, 'stage'), before + 1);
      expect(reader.stage.look('f')!.result, '3');
      expect(seen.last.looks, {'f'});
      expect(keeper.state, KeeperState.unsaved);
      await keeper.save();
      expect(fake.saves, 1);
    });

    keeperTest('every reader is read, until it is closed', (
      tester,
      keeper,
      fake,
    ) async {
      final one = await keeper.read();
      final two = await keeper.read(ask: rowsOf(1));
      expect(two.stage.whole, isFalse);
      final seenOne = told(one);
      final seenTwo = told(two);
      await keeper.apply(typing('d', 'typed'));
      expect(one.stage.look('d')!.text, 'typed');
      // Outside the window of the second: read, and nothing differs.
      expect(two.stage.look('d'), isNull);
      expect((seenOne.length, seenTwo.length), (1, 0));
      await keeper.apply(typing('a', 'typed'));
      expect((seenOne.length, seenTwo.length), (2, 1));

      one.close();
      final before = calls(fake, 'stage');
      await keeper.apply(typing('a', 'again'));
      expect(calls(fake, 'stage'), before + 1);
      expect(one.stage.look('a')!.text, 'typed');
      expect(two.stage.look('a')!.text, 'again');
      expect(seenOne.length, 2);
      // A listener that was taken away is not told.
      two.removeListener(seenTwo.add);
      await keeper.apply(typing('a', 'once more'));
      expect(seenTwo.length, 2);
    });

    keeperTest('a listener is told apart from the call, and may ask at once', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      final answers = <Future<GridInfo>>[];
      // Asked from inside the keeper's own piece this would be refused.
      reader.addListener((_) => answers.add(keeper.grid('root')));
      await keeper.apply(typing('a', 'typed'));
      expect((await answers.single).cells.first.text, 'typed');
    });

    keeperTest('a stage that cannot be read again: said, and the change '
        'stands', (tester, keeper, fake) async {
      final reader = await keeper.read();
      final seen = told(reader);
      failFirst(fake, 'stage', broken);
      final report = await keeper.apply(typing('a', 'typed'));
      expect(report.changed, ['a']);
      expect(reader.readError, broken);
      expect(reader.stage.look('a')!.text, 'one');
      expect(seen.single.isEmpty, isTrue);
      expect((keeper.ended, keeper.state), (null, KeeperState.unsaved));

      await reader.refresh();
      expect(reader.readError, isNull);
      expect(reader.stage.look('a')!.text, 'typed');
      expect(seen.last.looks, {'a'});
      // A read that works after one that failed is told even if nothing differs.
      failFirst(fake, 'stage', broken);
      await reader.refresh().catchError((Object _) {});
      expect(reader.readError, isNull);
    });

    keeperTest('a refresh that fails throws, and the stage stays', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      fake.before = (call) => call == 'stage' ? throw broken : null;
      await expectLater(reader.refresh(), throwsA(broken));
      await keeper.apply(typing('a', 'typed'));
      expect(reader.readError, broken);
      fake.before = null;
      final seen = told(reader);
      await reader.refresh();
      expect(seen.single.looks, {'a'});
      // Read well, with nothing new: the error going is news all the same.
      fake.before = (call) => call == 'stage' ? throw broken : null;
      await keeper.apply(typing('a', 'more'));
      fake.before = null;
      await keeper.apply(typing('a', 'typed'));
      expect(reader.readError, isNull);
      expect(seen.length, 3);
      expect(seen.last.isEmpty, isTrue);
    });

    keeperTest('a fault while reading again ends the work', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      failFirst(fake, 'stage', PanicException('in the read'));
      // The command itself worked, and says so.
      expect((await keeper.apply(typing('a', 'typed'))).changed, ['a']);
      expect(keeper.ended, faultSentence);
      await expectLater(reader.refresh(), refusedAs(ErrorKind.closed));
      await expectLater(reader.go('c'), refusedAs(ErrorKind.closed));
      await expectLater(keeper.read(), refusedAs(ErrorKind.closed));
    });

    keeperTest('one read that nobody keeps, and one cell whole', (
      tester,
      keeper,
      fake,
    ) async {
      final stage = await keeper.stage('c', phoneAsk);
      expect(stage.cells.map((cell) => cell.id), ['e', 'f']);
      expect((await keeper.cell('f')).result, '2');
      await keeper.apply(typing('e', 'typed'));
      expect(calls(fake, 'stage'), 1);
      await expectLater(keeper.cell('x'), refusedAs(ErrorKind.notFound));
    });
  });

  group('from level to level', () {
    keeperTest('a dive and the way back, with where the screen was', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      final seen = told(reader);
      reader.at(x: 1, y: 1, selected: 'd');
      await reader.go('c');
      expect(reader.stage.cell, 'c');
      expect(reader.stage.trail.map((step) => step.id), ['root', 'c']);
      expect(reader.stage.level, isNull);
      expect(seen.single.everything, isTrue);
      expect(fake.zoom, 'c');
      expect(
        fake.levels['root'],
        const LevelInfo(cell: 'root', x: 1, y: 1, selected: 'd'),
      );
      // One turn of the keeper: told, then read.
      expect(fake.log.sublist(fake.log.length - 4), [
        'viewGo',
        'viewGo.',
        'stage',
        'stage.',
      ]);

      // Nothing was said of this level: it remembers nothing new.
      await reader.go('root');
      expect(fake.levels.containsKey('c'), isFalse);
      expect(
        reader.stage.level,
        const LevelInfo(cell: 'root', x: 1, y: 1, selected: 'd'),
      );
      // Nor is the place of the level before taken for this one's.
      await reader.go('c');
      expect(fake.levels['root']!.selected, 'd');
      reader.at(x: 0, y: 1);
      await reader.go('root');
      expect(fake.levels['c'], const LevelInfo(cell: 'c', x: 0, y: 1));
      // The document was not touched by any of it.
      expect((keeper.state, fake.saved), (KeeperState.saved, true));
    });

    keeperTest('a cell without a grid is refused, and the stage stays', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      final seen = told(reader);
      reader.at(x: 1, y: 0);
      await expectLater(reader.go('a'), refusedAs(ErrorKind.refused));
      await expectLater(reader.go('nowhere'), refusedAs(ErrorKind.notFound));
      expect([reader.stage.cell, seen, fake.zoom], ['root', isEmpty, 'root']);
      expect(fake.levels, isEmpty);
      // What was said of this level is still to hand on.
      await reader.go('c');
      expect(fake.levels['root']!.x, 1);
    });

    keeperTest('a selected cell that is gone is left out', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      reader.at(x: 0, y: 1, selected: 'd');
      failFirst(fake, 'viewGo', gone);
      await reader.go('c');
      expect(reader.stage.cell, 'c');
      expect(calls(fake, 'viewGo'), 2);
      expect(fake.levels['root'], const LevelInfo(cell: 'root', x: 0, y: 1));

      // Without a selected cell there is nothing to leave out: the error stands.
      reader.at(x: 0, y: 0);
      failFirst(fake, 'viewGo', gone);
      await expectLater(reader.go('root'), refusedAs(ErrorKind.notFound));
      expect(calls(fake, 'viewGo'), 3);
      // Any other refusal is not tried again either.
      reader.at(x: 0, y: 0, selected: 'e');
      failFirst(fake, 'viewGo', broken);
      await expectLater(reader.go('root'), throwsA(broken));
      expect(calls(fake, 'viewGo'), 4);
      expect(reader.stage.cell, 'c');
    });

    keeperTest('a window is moved, and starts where the level was', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read(ask: rowsOf(1));
      expect(reader.stage.cells.map((cell) => cell.id), ['a', 'b']);
      final seen = told(reader);
      await reader.window(y0: 1);
      expect(reader.stage.grid!.y0, 1);
      expect(reader.stage.look('d')!.text, 'four');
      expect(seen.single.looks, {'c', 'd', 'e', 'f'});
      expect(seen.single.gone, {'a', 'b'});
      // The window stays where it was put when the stage is read again.
      await keeper.apply(typing('d', 'typed'));
      expect(reader.stage.grid!.y0, 1);
      expect(seen.last.looks, {'d'});
      await reader.window(y0: 1);
      expect(seen.length, 2);
      await reader.window(x0: 1);
      expect((reader.stage.grid!.x0, reader.stage.grid!.y0), (1, 1));
      expect(reader.stage.cells.first.id, 'd');

      // Left at the second row: found there again, though the window that is
      // asked for starts at the first.
      reader.at(x: 0, y: 1);
      await reader.go('c');
      expect(reader.stage.grid!.y0, 0);
      final before = calls(fake, 'stage');
      await reader.go('root');
      expect(calls(fake, 'stage'), before + 2);
      expect((reader.stage.grid!.x0, reader.stage.grid!.y0), (0, 1));
      expect(reader.stage.ask.rows, 1);
      // A level whose place is inside the window is read once.
      reader.at(x: 0, y: 0);
      await reader.go('c');
      await reader.go('root');
      expect(calls(fake, 'stage'), before + 4);
      expect(reader.stage.grid!.y0, 0);
    });

    keeperTest('the rows a screen shows are read when they are not there', (
      tester,
      keeper,
      fake,
    ) async {
      // A grid of one column and seven rows, read three rows at a time.
      fake.cells['c']!.cells = ['e', 'f', 'g', 'h', 'i', 'j', 'k'];
      for (final id in ['g', 'h', 'i', 'j', 'k']) {
        fake.cells[id] = FakeCell(id);
      }
      final reader = await keeper.read(ask: rowsOf(3));
      await reader.go('c');
      List<String> rows() => [for (final cell in reader.stage.cells) cell.id];
      expect(rows(), ['e', 'f', 'g']);
      final seen = told(reader);
      final before = fake.log.length;
      // Rows that are there: nothing is asked of anyone.
      await reader.see(0, 2);
      await reader.see(1, 1);
      await reader.see(-5, 0);
      expect([fake.log.length, seen], [before, isEmpty]);
      // One that is not: the window is put around it.
      await reader.see(3, 3);
      expect(rows(), ['g', 'h', 'i']);
      expect(
        [seen.single.looks, seen.single.gone],
        [
          {'h', 'i'},
          {'e', 'f'},
        ],
      );
      await reader.see(1, 2);
      expect(rows(), ['f', 'g', 'h']);
      await reader.see(4, 4);
      expect(rows(), ['h', 'i', 'j']);
      // Not past the end of the grid, nor before its start.
      await reader.see(6, 99);
      expect(rows(), ['i', 'j', 'k']);
      await reader.see(99, 99);
      expect(rows(), ['i', 'j', 'k']);
      await reader.see(0, 0);
      expect(rows(), ['e', 'f', 'g']);
      // More rows than a window holds: from the first of them.
      await reader.see(3, 6);
      expect(rows(), ['h', 'i', 'j']);
      // Asked twice at once, the second finds the window moved and reads nothing.
      final calls = fake.log.length;
      await Future.wait([reader.see(6, 6), reader.see(6, 6)]);
      expect(fake.log.length, calls + 2);

      // A stage that is given whole has every row, and one without a grid none.
      final whole = await keeper.read();
      final asked = fake.log.length;
      await whole.see(0, 99);
      await reader.go('root');
      expect(reader.stage.whole, isTrue);
      await reader.see(1, 1);
      expect(fake.log.length, asked + 4);
    });

    keeperTest('another request from here on', (tester, keeper, fake) async {
      final reader = await keeper.read();
      final seen = told(reader);
      expect(reader.stage.look('e'), isNotNull);
      await reader.ask(rowsOf(4294967295, fullDepth: 0));
      expect(reader.stage.look('e'), isNull);
      expect(seen.single.gone, {'e', 'f'});
      expect(seen.single.grids, {'c'});
      // It is the request of later reads and levels too.
      await keeper.apply(typing('a', 'typed'));
      expect(reader.stage.ask.fullDepth, 0);
      await reader.go('c');
      expect(reader.stage.ask.fullDepth, 0);
    });

    test(
      'the level, its place and a fold are there at the next start',
      () async {
        final doc = await openExample(folder, 'levels');
        final keeper = Keeper(doc, saveAfter: const Duration(hours: 1));
        final reader = await keeper.read();
        final root = reader.stage;
        final shopping = root.cells.firstWhere((c) => c.text == 'Shopping').id;
        final costs = root.cells.firstWhere((c) => c.text == 'Costs').id;
        final total = root.cells.firstWhere((c) => c.text == 'Total').id;
        reader.at(x: 1, y: 2, selected: total);
        await reader.go(shopping);
        reader.at(x: 0, y: 3);
        expect(await keeper.setFolded(costs, folded: true), isTrue);
        expect(await keeper.isViewSaved(), isFalse);
        // Closing hands on where the screen is and writes it with the rest.
        await keeper.close();

        final again = Keeper(await Session.open(doc.path));
        final back = await again.read();
        expect(back.stage.cell, shopping);
        expect(back.stage.level, LevelInfo(cell: shopping, x: 0, y: 3));
        expect(await again.isViewSaved(), isTrue);
        await back.go(doc.root);
        expect(
          back.stage.level,
          LevelInfo(cell: doc.root, x: 1, y: 2, selected: total),
        );
        expect(back.stage.look(costs)!.folded, isTrue);
        await again.close();
      },
    );

    test(
      'a stage that an undo takes away gives way to the level above',
      () async {
        final doc = await openExample(folder, 'gone');
        final keeper = Keeper(doc, saveAfter: const Duration(hours: 1));
        final reader = await keeper.read();
        final shopping = reader.stage.cells
            .firstWhere((c) => c.text == 'Shopping')
            .id;
        final apples = reader.stage.cells
            .firstWhere((c) => c.text == 'Apples')
            .id;
        await reader.go(shopping);
        // A grid in "Apples", and in its one cell another.
        final made = (await keeper.apply(
          Command.createGrid(cell: apples, xs: 1, ys: 1),
        )).created.single;
        await keeper.apply(Command.createGrid(cell: made, xs: 1, ys: 1));
        await reader.go(apples);
        await reader.go(made);
        expect(reader.stage.trail.length, 4);
        final seen = told(reader);

        // The innermost grid goes: its cell is still there, and no stage.
        await keeper.undo();
        expect(reader.stage.cell, apples);
        expect(reader.stage.cells.single.id, made);
        expect(seen.single.everything, isTrue);
        // The cell itself goes with the grid it is in.
        await reader.go(made).catchError((Object _) {});
        await keeper.redo();
        await reader.go(made);
        await keeper.undo();
        await keeper.undo();
        expect(reader.stage.cell, shopping);
        expect(reader.readError, isNull);
        expect(seen.last.everything, isTrue);
        // And the level that is shown is the one the view state names.
        expect(await doc.viewStage(), shopping);
        await keeper.close();
      },
    );

    test(
      'the root of a new document has no grid, and nothing to remember',
      () async {
        final doc = await Session.create('${folder.path}/new.wombat');
        final keeper = Keeper(doc, saveAfter: const Duration(hours: 1));
        final reader = await keeper.read();
        expect((reader.stage.cell, reader.stage.grid), (doc.root, null));
        reader.at(x: 0, y: 0);
        await keeper.putAway();
        expect((keeper.viewError, keeper.state), (null, KeeperState.saved));
        // A grid comes, and goes again with an undo: the root stays the stage.
        final seen = told(reader);
        await keeper.apply(Command.createGrid(cell: doc.root, xs: 2, ys: 2));
        expect(reader.stage.grid!.xs, 2);
        expect(seen.single.grids, {doc.root});
        await keeper.undo();
        expect((reader.stage.cell, reader.stage.grid), (doc.root, null));
        expect(reader.readError, isNull);
        await keeper.close();
      },
    );
  });

  group('folding', () {
    keeperTest('is read, changes nothing in the document, and is not twice', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      final seen = told(reader);
      expect(await keeper.setFolded('c', folded: true), isTrue);
      expect(reader.stage.look('c')!.folded, isTrue);
      expect(seen.single.looks, {'c'});
      expect(await keeper.setFolded('c', folded: true), isFalse);
      expect(seen.length, 1);
      expect(calls(fake, 'stage'), 2);
      expect((keeper.state, fake.saved), (KeeperState.saved, true));
      await expectLater(
        keeper.setFolded('a', folded: true),
        refusedAs(ErrorKind.refused),
      );
      expect(await keeper.setFolded('c', folded: false), isTrue);
      expect(reader.stage.look('c')!.folded, isFalse);
    });
  });

  group('where the screen is, in its own file', () {
    keeperTest('is written after the document, with every save', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      await reader.go('c');
      expect(await keeper.isViewSaved(), isFalse);
      // A level gone to is no reason to save: nothing happens by the clock.
      await tester.pump(const Duration(minutes: 5));
      expect((calls(fake, 'save'), fake.viewSaves), (0, 0));

      await keeper.apply(typing('e', 'typed'));
      fake.log.clear();
      await tester.pump(const Duration(seconds: 1));
      expect(fake.log, ['save', 'save.', 'saveView', 'saveView.']);
      expect((fake.saves, fake.viewSaves, fake.viewSaved), (1, 1, true));
      expect(await keeper.isViewSaved(), isTrue);
    });

    keeperTest('the place is handed on when it moved, not otherwise', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      reader.at(x: 1, y: 0, selected: 'b');
      await keeper.save();
      expect(
        fake.levels['root'],
        const LevelInfo(cell: 'root', x: 1, y: 0, selected: 'b'),
      );
      expect((calls(fake, 'viewRemember'), fake.viewSaves), (1, 1));
      // The same place said again is no move.
      reader.at(x: 1, y: 0, selected: 'b');
      await keeper.save();
      expect((calls(fake, 'viewRemember'), fake.viewSaves), (1, 1));
      reader.at(x: 1, y: 1, selected: 'b');
      await keeper.save();
      expect((calls(fake, 'viewRemember'), fake.viewSaves), (2, 2));
      expect(fake.levels['root']!.y, 1);
      // A reader that was closed has nothing to say.
      reader.at(x: 0, y: 0);
      reader.close();
      await keeper.save();
      expect(calls(fake, 'viewRemember'), 2);
    });

    keeperTest('a selected cell that is gone is left out there too', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      reader.at(x: 1, y: 1, selected: 'd');
      failFirst(fake, 'viewRemember', gone);
      await keeper.save();
      expect(fake.levels['root'], const LevelInfo(cell: 'root', x: 1, y: 1));
      expect((calls(fake, 'viewRemember'), keeper.viewError), (2, null));
      // A level that is gone altogether, or cannot remember: no error, and the
      // rest is written.
      reader.at(x: 0, y: 0);
      fake.before = (call) => call == 'viewRemember' ? throw gone : null;
      await keeper.save();
      expect((calls(fake, 'viewRemember'), keeper.viewError), (3, null));
      reader.at(x: 0, y: 1, selected: 'd');
      fake.before = (call) => call == 'viewRemember'
          ? throw const BridgeError(kind: ErrorKind.refused, message: 'no grid')
          : null;
      await keeper.save();
      expect((calls(fake, 'viewRemember'), keeper.viewError), (4, null));
      fake.before = null;
    });

    keeperTest('that it cannot be written is never a failed save', (
      tester,
      keeper,
      fake,
    ) async {
      final states = <KeeperState>[];
      keeper.addListener(() => states.add(keeper.state));
      final reader = await keeper.read();
      await reader.go('c');
      fake.viewFails = true;
      await keeper.apply(typing('e', 'typed'));
      await keeper.save();
      expect((fake.saves, fake.saved), (1, true));
      expect((keeper.state, keeper.saveError), (KeeperState.saved, null));
      expect(kindOfError(keeper.viewError), ErrorKind.io);
      expect(states, [
        KeeperState.unsaved,
        KeeperState.saving,
        KeeperState.saved,
      ]);
      // Nothing tries again by the clock for it.
      final before = calls(fake, 'saveView');
      await tester.pump(const Duration(minutes: 5));
      expect(calls(fake, 'saveView'), before);
      // The next save does, and what worked is no error any more.
      fake.viewFails = false;
      await keeper.save();
      expect((keeper.viewError, fake.viewSaves), (null, 1));
      expect(states.length, 3);
    });

    keeperTest('it is written also when the document cannot be', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      reader.at(x: 1, y: 1);
      await keeper.apply(typing('a', 'typed'));
      failFirst(fake, 'save', broken);
      await keeper.save();
      expect((keeper.state, fake.saved), (KeeperState.failed, false));
      expect((fake.viewSaves, keeper.viewError), (1, null));
      expect(fake.levels['root']!.x, 1);
    });

    keeperTest('a fault in it ends the work like any other', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      await reader.go('c');
      failFirst(fake, 'saveView', PanicException('in the view state'));
      await keeper.save();
      expect(keeper.ended, faultSentence);
    });

    keeperTest('the app put away: the place and the level are in the file', (
      tester,
      keeper,
      fake,
    ) async {
      final reader = await keeper.read();
      await reader.go('c');
      reader.at(x: 0, y: 1, selected: 'f');
      expect(await keeper.putAway(), isEmpty);
      expect(
        fake.levels['c'],
        const LevelInfo(cell: 'c', x: 0, y: 1, selected: 'f'),
      );
      expect((fake.viewSaves, fake.viewSaved, fake.saves), (1, true, 0));
    });

    keeperTest('closing: the same, and a view state that cannot be written '
        'does not keep the document open', (tester, keeper, fake) async {
      final reader = await keeper.read();
      reader.at(x: 1, y: 1);
      fake.viewFails = true;
      await keeper.apply(typing('a', 'typed'));
      await keeper.close();
      expect((fake.closed, fake.saves, fake.viewSaves), (true, 1, 0));
      expect(fake.levels['root']!.y, 1);
      expect(keeper.state, KeeperState.ended);
    });

    keeperTest(
      'the other program\'s file: every stage starts again',
      (tester, keeper, fake) async {
        final reader = await keeper.read();
        await reader.go('c');
        reader.at(x: 0, y: 1);
        final seen = told(reader);
        await keeper.apply(typing('e', 'mine'));
        fake.fileChanged = true;
        await keeper.save();
        expect(seen.length, 1);
        await keeper.openTheirs();
        // Theirs was last shown at the root, and has its own texts.
        expect(reader.stage.cell, 'root');
        expect(reader.stage.look('e')!.text, 'theirs');
        expect(seen.last.everything, isTrue);
        expect(seen.length, 2);
        // What was said of the level of the other document is not handed on.
        await keeper.save();
        expect(theirs.levels, isEmpty);
        // The reader goes on with the document that was opened.
        await keeper.apply(typing('a', 'edited'));
        expect(reader.stage.look('a')!.text, 'edited');
        expect(seen.last.looks, {'a'});
      },
      reopen: (_) async =>
          Session.over(theirs = FakeDocument()..cells['e']!.text = 'theirs'),
    );

    test('on the real core, beside a document that cannot be saved', () async {
      final (doc, watch) = await watchExample(folder, 'beside');
      final keeper = Keeper(doc, saveAfter: const Duration(hours: 1));
      final reader = await keeper.read();
      final shopping = reader.stage.cells
          .firstWhere((c) => c.text == 'Shopping')
          .id;
      await keeper.apply(typing(shopping, 'mine'));
      await reader.go(shopping);
      reader.at(x: 0, y: 2);
      File(doc.path)
          .writeAsStringSync('${File(doc.path).readAsStringSync()}\n');
      await keeper.save();
      expect(keeper.state, KeeperState.changedOnDisk);
      expect((keeper.viewError, await keeper.isViewSaved()), (null, true));
      final view = Directory('${folder.path}/.wombat-view').listSync();
      expect(
        view.whereType<File>().any(
          (file) => file.readAsStringSync().contains(shopping),
        ),
        isTrue,
      );
      // A view state that cannot be written is no failed save either: the folder
      // for it is a file here.
      await keeper.keepMine();
      expect(keeper.state, KeeperState.saved);
      final other = await Directory('${folder.path}/other').create();
      final second = Keeper(
        await Session.create('${other.path}/new.wombat'),
        saveAfter: const Duration(hours: 1),
      );
      File('${other.path}/.wombat-view').writeAsStringSync('in the way');
      await second.apply(Command.createGrid(cell: second.root, xs: 1, ys: 1));
      expect(await second.setFolded(second.root, folded: true), isTrue);
      await second.save();
      expect((second.state, second.saveError), (KeeperState.saved, null));
      expect(kindOfError(second.viewError), ErrorKind.invalid);
      expect(await second.isViewSaved(), isFalse);
      await second.close();
      await keeper.close();
      expect(calls(watch, 'saveView'), greaterThan(1));
    });
  });
}

late FakeDocument theirs;

ErrorKind? kindOfError(Object? error) =>
    error is BridgeError ? error.kind : null;
