// The logic of the simple grid screen without a device and without widgets, against the
// real core, and against a stand-in where the core must be slow or fail.
import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/grid/stage.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'fake_document.dart';
import 'support.dart';

void main() {
  late Directory folder;
  var count = 0;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-stage');
  });

  tearDownAll(() => folder.delete(recursive: true));

  // Every stage is closed at the end of its test, which also stops its clock.
  final stages = <Stage>[];
  tearDown(() async {
    for (final stage in stages) {
      await stage.close().catchError((Object _) {});
    }
    stages.clear();
  });

  const never = Duration(hours: 1);

  /// A stage on a copy of the example. It saves by itself only when told after how long.
  Future<(Stage, Session)> example({Duration saveAfter = never}) async {
    final doc = await openExample(folder, 'stage-${count++}');
    final stage = await Stage.open(doc, saveAfter: saveAfter);
    stages.add(stage);
    return (stage, doc);
  }

  /// A stage on the stand-in for the core.
  Future<(Stage, FakeDocument)> standIn({
    Future<Session> Function(String path)? reopen,
  }) async {
    final fake = FakeDocument();
    final stage = await Stage.open(
      await Session.over(fake),
      saveAfter: never,
      reopen: reopen ?? Session.open,
    );
    stages.add(stage);
    return (stage, fake);
  }

  Matcher refusedAs(ErrorKind kind) =>
      throwsA(isA<BridgeError>().having((e) => e.kind, 'kind', kind));

  ErrorKind? kindOf(Object? error) => error is BridgeError ? error.kind : null;

  const broken = BridgeError(kind: ErrorKind.io, message: 'the disk is gone');

  List<String> texts(Stage stage) =>
      stage.grid.cells.map((c) => c.result ?? c.text).toList();

  List<String> trail(Stage stage) => stage.trail.map((l) => l.text).toList();

  test('starts at the root', () async {
    final (stage, doc) = await example();
    expect(stage.atRoot, isTrue);
    expect(trail(stage), ['Example']);
    expect(stage.trail.single.id, doc.root);
    expect((stage.grid.xs, stage.grid.ys), (2, 3));
    expect(texts(stage), [
      'Shopping',
      'what to buy',
      'Costs',
      'what it costs',
      'Total',
      '10',
    ]);
    expect(stage.touched, isEmpty);
    expect(await stage.up(), isFalse);
  });

  test('goes into a grid and back out', () async {
    final (stage, _) = await example();
    var changes = 0;
    stage.addListener(() => changes++);
    final costs = stage.grid.cells[2];

    expect(await stage.dive(costs.id), isTrue);
    expect(changes, 1);
    expect(stage.atRoot, isFalse);
    expect(trail(stage), ['Example', 'Costs']);
    expect(stage.grid.cell.id, costs.id);
    expect(texts(stage), ['3', '2', '5']);

    // A cell without a grid, and a cell that is not on the stage.
    expect(await stage.dive(stage.grid.cells[0].id), isFalse);
    expect(await stage.dive(costs.id), isFalse);
    expect(trail(stage), ['Example', 'Costs']);
    expect(changes, 1);

    expect(await stage.up(), isTrue);
    expect(trail(stage), ['Example']);
    expect(texts(stage)[2], 'Costs');
    expect(changes, 2);
  });

  test('jumps to a level of the trail', () async {
    final (stage, doc) = await example();
    // A third level: a grid inside the first cost.
    await stage.dive(stage.grid.cells[2].id);
    final first = stage.grid.cells[0].id;
    await doc.apply(Command.createGrid(cell: first, xs: 1, ys: 1));
    await stage.jumpTo(0);
    await stage.dive(stage.grid.cells[2].id);
    expect(await stage.dive(first), isTrue);
    expect(trail(stage), ['Example', 'Costs', '3']);

    expect(await stage.jumpTo(2), isFalse);
    expect(await stage.jumpTo(3), isFalse);
    expect(await stage.jumpTo(-1), isFalse);
    expect(await stage.jumpTo(0), isTrue);
    expect(trail(stage), ['Example']);
  });

  test('typing changes the cell and what reads it, and undoes', () async {
    final (stage, _) = await example();
    final total = stage.grid.cells[5].id;
    await stage.dive(stage.grid.cells[2].id);
    final first = stage.grid.cells[0].id;

    final report = await stage.enterText(first, '30');
    expect(report.changed, [first]);
    expect(report.recalculated, [total]);
    expect(texts(stage), ['30', '2', '5']);
    expect(stage.touched, {first, total});

    await stage.up();
    expect(stage.touched, isEmpty);
    expect(texts(stage)[5], '37');

    expect(await stage.undo(), isTrue);
    expect(texts(stage)[5], '10');
    expect(stage.touched, {first, total});
    expect(await stage.undo(), isFalse);
    expect(await stage.redo(), isTrue);
    expect(texts(stage)[5], '37');
    expect(await stage.redo(), isFalse);
  });

  test(
    'typing a formula makes a formula cell, and typing text unmakes it',
    () async {
      final (stage, _) = await example();
      final cell = stage.grid.cells[1];
      expect(cell.result, isNull);

      await stage.enterText(cell.id, '=SUM(Costs)*2');
      var now = stage.grid.cells[1];
      expect(now.kind, CellKind.formula);
      expect(
        (now.text, now.result, now.failed),
        ('=SUM(Costs)*2', '20', false),
      );

      await stage.enterText(cell.id, '=nosuchfunction(1)');
      now = stage.grid.cells[1];
      expect(now.failed, isTrue);

      await stage.enterText(cell.id, 'plain');
      now = stage.grid.cells[1];
      expect(now.kind, CellKind.data);
      expect((now.text, now.result), ('plain', null));
    },
  );

  test('the title of the stage follows its text', () async {
    final (stage, _) = await example();
    final root = stage.grid.cell.id;
    await stage.enterText(root, 'Renamed');
    expect(trail(stage), ['Renamed']);
    await stage.undo();
    expect(trail(stage), ['Example']);
  });

  test(
    'a stage that an undo takes away gives way to the level above',
    () async {
      final (stage, doc) = await example();
      // A grid inside "what to buy", made and then undone while it is on the stage.
      final cell = stage.grid.cells[1].id;
      await doc.apply(Command.createGrid(cell: cell, xs: 1, ys: 2));
      await stage.jumpTo(0);
      await stage.dive(stage.grid.cells[2].id);
      await stage.up();
      expect(await stage.dive(cell), isTrue);
      final inner = stage.grid.cells[0].id;
      await doc.apply(Command.createGrid(cell: inner, xs: 1, ys: 1));
      await stage.up();
      await stage.dive(cell);
      expect(await stage.dive(inner), isTrue);
      expect(stage.trail.length, 3);

      // Undo the inner grid: its cell is still there, without a grid.
      expect(await stage.undo(), isTrue);
      expect(stage.trail.length, 3);
      expect(stage.grid.xs, 0);
      // Undo the outer grid: the cell on the stage is gone, and so is its parent's grid.
      expect(await stage.undo(), isTrue);
      expect(stage.trail.length, 2);
      expect(stage.grid.cell.id, cell);
      expect(stage.grid.xs, 0);
    },
  );

  test('what the core refuses is an error and changes nothing', () async {
    final (stage, _) = await example();
    final before = texts(stage);
    await expectLater(
      stage.enterText('00000000-0000-4000-8000-000000000000', 'x'),
      throwsA(
        isA<BridgeError>().having((e) => e.kind, 'kind', ErrorKind.notFound),
      ),
    );
    expect(texts(stage), before);
    expect(await stage.undo(), isFalse);
  });

  test('what a redo makes is among the cells touched', () async {
    final (stage, doc) = await example();
    await doc.apply(Command.insertRows(cell: doc.root, at: 3, count: 1));
    expect(await stage.undo(), isTrue);
    expect(stage.grid.ys, 3);
    expect(await stage.redo(), isTrue);
    expect(stage.grid.ys, 4);
    final made = stage.grid.cells.sublist(6).map((c) => c.id).toSet();
    expect(made.length, 2);
    expect(stage.touched.containsAll(made), isTrue);
  });

  test('going into a grid forgets what was touched', () async {
    final (stage, _) = await example();
    await stage.enterText(stage.grid.cells[1].id, 'x');
    expect(stage.touched, isNotEmpty);
    expect(await stage.dive(stage.grid.cells[2].id), isTrue);
    expect(stage.touched, isEmpty);
  });

  test('typing the text a cell has changes nothing', () async {
    final (stage, doc) = await example();
    final report = await stage.enterText(stage.grid.cells[0].id, 'Shopping');
    expect(report.changed, isEmpty);
    expect(await doc.isSaved(), isTrue);
    expect(await stage.undo(), isFalse);
  });

  group('one step at a time', () {
    test('two dives asked for at once go in once', () async {
      final (stage, _) = await example();
      final costs = stage.grid.cells[2].id;
      final first = stage.dive(costs);
      final second = stage.dive(costs);
      expect((await first, await second), (true, false));
      expect(trail(stage), ['Example', 'Costs']);
    });

    test('two steps up asked for at once go up twice', () async {
      final (stage, doc) = await example();
      final costs = stage.grid.cells[2].id;
      await stage.dive(costs);
      final cost = stage.grid.cells[0].id;
      await doc.apply(Command.createGrid(cell: cost, xs: 1, ys: 1));
      await stage.refresh();
      await stage.dive(cost);
      expect(trail(stage), ['Example', 'Costs', '3']);
      final first = stage.up();
      final second = stage.up();
      final third = stage.up();
      expect((await first, await second, await third), (true, true, false));
      expect(trail(stage), ['Example']);
    });

    test('a cell is read after what was asked before it', () async {
      final (stage, _) = await example();
      final cell = stage.grid.cells[1].id;
      await stage.enterText(cell, 'typed');
      final undo = stage.undo();
      final asked = stage.cell(cell);
      expect((await asked)!.text, 'what to buy');
      await undo;
      expect(await stage.cell('00000000-0000-4000-8000-000000000000'), isNull);
    });
  });

  group('saving by itself', () {
    test('an edit is in the file soon after, without being asked', () async {
      final (stage, doc) = await example(
        saveAfter: const Duration(milliseconds: 20),
      );
      await stage.enterText(stage.grid.cells[1].id, 'saved by itself');
      expect(await doc.isSaved(), isFalse);
      for (var i = 0; i < 200 && !await doc.isSaved(); i++) {
        await Future<void>.delayed(const Duration(milliseconds: 10));
      }
      expect(await doc.isSaved(), isTrue);
      expect(File(doc.path).readAsStringSync(), contains('"saved by itself"'));
      expect(stage.saveError, isNull);
    });

    test('undo and redo are saved too', () async {
      final (stage, doc) = await example(
        saveAfter: const Duration(milliseconds: 20),
      );
      await stage.enterText(stage.grid.cells[1].id, 'gone again');
      await stage.save();
      await stage.undo();
      for (var i = 0; i < 200 && !await doc.isSaved(); i++) {
        await Future<void>.delayed(const Duration(milliseconds: 10));
      }
      expect(File(doc.path).readAsStringSync(), isNot(contains('gone again')));
    });

    test('saving when asked, and closing, write what was typed', () async {
      final (stage, doc) = await example();
      await stage.enterText(stage.grid.cells[1].id, 'asked for');
      expect(await doc.isSaved(), isFalse);
      await stage.save();
      expect(await doc.isSaved(), isTrue);
      expect(File(doc.path).readAsStringSync(), contains('"asked for"'));

      await stage.enterText(stage.grid.cells[1].id, 'at the end');
      // A file is open once.
      await expectLater(
        Session.open(doc.path),
        refusedAs(ErrorKind.alreadyOpen),
      );
      await stage.close();
      final again = await Session.open(doc.path);
      expect((await again.grid(again.root)).cells[1].text, 'at the end');
      await again.close();
    });

    test('a file that another program changed: keep mine', () async {
      final (stage, doc) = await example();
      var told = 0;
      stage.addListener(() => told++);
      await stage.enterText(stage.grid.cells[1].id, 'mine');
      final theirs = '${File(doc.path).readAsStringSync()}\n';
      File(doc.path).writeAsStringSync(theirs);

      told = 0;
      await stage.save();
      expect(kindOf(stage.saveError), ErrorKind.changedOnDisk);
      expect(told, 1);
      expect(File(doc.path).readAsStringSync(), theirs);

      await stage.keepMine();
      expect(stage.saveError, isNull);
      expect(told, 2);
      expect(File(doc.path).readAsStringSync(), contains('"mine"'));
      expect(await doc.isSaved(), isTrue);
    });

    test('a file that another program changed: open theirs', () async {
      final (stage, doc) = await example();
      final theirs = '${File(doc.path).readAsStringSync()}\n';
      await stage.dive(stage.grid.cells[2].id);
      await stage.enterText(stage.grid.cells[0].id, 'mine');
      File(doc.path).writeAsStringSync(theirs);
      await stage.save();
      expect(kindOf(stage.saveError), ErrorKind.changedOnDisk);

      await stage.openTheirs();
      expect(stage.saveError, isNull);
      expect(File(doc.path).readAsStringSync(), theirs);
      expect(trail(stage), ['Example']);
      expect(stage.touched, isEmpty);
      await stage.dive(stage.grid.cells[2].id);
      expect(texts(stage), ['3', '2', '5']);
      // The stage is on the document that was opened: it edits and saves that one.
      await stage.enterText(stage.grid.cells[0].id, 'theirs, edited');
      await stage.save();
      expect(stage.saveError, isNull);
      expect(File(doc.path).readAsStringSync(), contains('"theirs, edited"'));
      await expectLater(doc.isSaved(), refusedAs(ErrorKind.closed));
    });

    test(
      'a save that fails is told, and the next one that works too',
      () async {
        final (stage, fake) = await standIn();
        var told = 0;
        stage.addListener(() => told++);
        failFirst(fake, 'save', broken);
        await stage.save();
        expect(stage.saveError, broken);
        expect((told, fake.saves), (1, 0));
        await stage.save();
        expect(stage.saveError, isNull);
        expect((told, fake.saves), (2, 1));
        // Nothing new to tell.
        await stage.save();
        expect(told, 2);
      },
    );

    test('a file that cannot be opened again ends the work', () async {
      final (stage, fake) = await standIn(reopen: (_) => throw broken);
      await stage.openTheirs();
      expect(fake.disposed, isTrue);
      expect(stage.ended, contains('the disk is gone'));
      await expectLater(stage.undo(), refusedAs(ErrorKind.closed));
    });
  });

  group('when the core fails', () {
    test('a root that cannot be read is an error, not a crash', () async {
      final fake = FakeDocument();
      fake.cells.remove('root');
      await expectLater(
        Session.over(fake).then(Stage.open),
        refusedAs(ErrorKind.notFound),
      );
    });

    test(
      'an edit that was made stays made when the stage cannot be read',
      () async {
        final (stage, fake) = await standIn();
        await stage.dive('c');
        failFirst(fake, 'grid', broken);
        final report = await stage.enterText('e', 'typed');
        expect(report.changed, ['e']);
        expect(fake.cells['e']!.text, 'typed');
        expect(stage.readError, broken);
        // Only a cell that is gone gives way to the level above; this one is there.
        expect(trail(stage), ['Root', 'three']);
        expect(stage.touched, {'e'});

        await stage.refresh();
        expect(stage.readError, isNull);
        expect(texts(stage), ['typed', '2']);
      },
    );

    test('going into a grid that cannot be read changes nothing', () async {
      final (stage, fake) = await standIn();
      failFirst(fake, 'grid', broken);
      await expectLater(stage.dive('c'), throwsA(broken));
      expect(trail(stage), ['Root']);
      expect(stage.grid.cell.id, 'root');
      expect(await stage.dive('c'), isTrue);
      expect(trail(stage), ['Root', 'three']);
    });

    test('going up to a level that cannot be read changes nothing', () async {
      final (stage, fake) = await standIn();
      await stage.dive('c');
      failFirst(fake, 'grid', broken);
      await expectLater(stage.up(), throwsA(broken));
      expect(trail(stage), ['Root', 'three']);
      expect(stage.grid.cell.id, 'c');
    });

    test(
      'after a fault the document is let go of and not used again',
      () async {
        final (stage, fake) = await standIn();
        var told = 0;
        stage.addListener(() => told++);
        failFirst(fake, 'apply', PanicException('at /home/builder/core.rs'));
        await expectLater(
          stage.enterText('a', 'x'),
          throwsA(isA<PanicException>()),
        );
        expect(stage.ended, faultSentence);
        expect(told, 1);
        expect(fake.disposed, isTrue);
        final calls = fake.log.length;
        await expectLater(stage.undo(), refusedAs(ErrorKind.closed));
        await expectLater(stage.dive('c'), refusedAs(ErrorKind.closed));
        await stage.save();
        await stage.keepMine();
        await stage.openTheirs();
        await stage.close();
        expect(fake.log.length, calls);
        expect((fake.saves, fake.overwrites, fake.closed), (0, 0, false));
      },
    );

    test('the refusal of every call after a fault ends the work too', () async {
      final (stage, fake) = await standIn();
      failFirst(
        fake,
        'save',
        const BridgeError(kind: ErrorKind.internal, message: 'half changed'),
      );
      await stage.save();
      expect(stage.ended, faultSentence);
      expect(fake.disposed, isTrue);
    });
  });
}
