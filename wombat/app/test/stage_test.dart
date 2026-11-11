// The logic of the simple grid screen without a device and without widgets, against the
// real core. Build the library first, from `wombat/`: cargo build -p wombat_bridge
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/grid/stage.dart';
import 'package:wombat_app/src/rust/api/document.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'support.dart';

void main() {
  late Directory folder;
  var count = 0;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-stage');
  });

  tearDownAll(() => folder.delete(recursive: true));

  Future<(Stage, OpenDocument)> example() async {
    final doc = await openExample(folder, 'stage-${count++}');
    return (await Stage.open(doc), doc);
  }

  List<String> texts(Stage stage) =>
      stage.grid.cells.map((c) => c.result ?? c.text).toList();

  List<String> trail(Stage stage) => stage.trail.map((l) => l.text).toList();

  test('starts at the root', () async {
    final (stage, doc) = await example();
    expect(stage.atRoot, isTrue);
    expect(trail(stage), ['Example']);
    expect(stage.trail.single.id, await doc.root());
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
    await doc.apply(command: Command.createGrid(cell: first, xs: 1, ys: 1));
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
      await doc.apply(command: Command.createGrid(cell: cell, xs: 1, ys: 2));
      await stage.jumpTo(0);
      await stage.dive(stage.grid.cells[2].id);
      await stage.up();
      expect(await stage.dive(cell), isTrue);
      final inner = stage.grid.cells[0].id;
      await doc.apply(command: Command.createGrid(cell: inner, xs: 1, ys: 1));
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

  test('saving writes what was typed', () async {
    final (stage, doc) = await example();
    await stage.enterText(stage.grid.cells[1].id, 'saved text');
    expect(await doc.isSaved(), isFalse);
    await stage.save();
    expect(await doc.isSaved(), isTrue);
    final again = await OpenDocument.open(path: await doc.path());
    final root = await again.grid(cell: await again.root());
    expect(root.cells[1].text, 'saved text');
  });
}
