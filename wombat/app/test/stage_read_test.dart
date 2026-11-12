// What differs between two reads of a stage (`StageRead.since`), on the example
// document in the real core: each kind of change names what it must and nothing
// else.
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';
import 'package:wombat_app/stage_read.dart';

import 'support.dart';

StageAsk asking({int rows = 4294967295, int y0 = 0, int fullDepth = 1}) =>
    StageAsk(
      x0: 0,
      y0: y0,
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
  late Session doc;
  var count = 0;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-stage-read');
  });

  tearDownAll(() => folder.delete(recursive: true));

  // The example: the root "Example" has two columns and three rows: "Shopping" (a
  // grid of one column: "Item", "Apples", "Bread", "Cheese"), "what to buy", "Costs"
  // (one column: "3", "2", "5"), "what it costs", "Total", "=SUM(Costs)".
  setUp(() async => doc = await openExample(folder, 'read-${count++}'));

  tearDown(() => doc.close());

  Future<StageRead> read(String cell, [StageAsk? ask]) async =>
      StageRead(cell, ask ?? phoneAsk, await doc.stage(cell, ask ?? phoneAsk));

  /// The ID of the cell that shows `text` in a read.
  String id(StageRead stage, String text) =>
      stage.cells.firstWhere((cell) => cell.text == text).id;

  /// What a change names, to compare in one go.
  List<Object> named(StageChange change) => [
    change.looks,
    change.summaries,
    change.grids,
    change.gone,
    change.trail,
    change.everything,
  ];

  const none = <String>{};

  test('the ways through one read', () async {
    final stage = await read(doc.root);
    expect((stage.cell, stage.ask), (doc.root, phoneAsk));
    expect(stage.trail.single.shown.text, 'Example');
    expect(stage.level, isNull);
    final grid = stage.grid!;
    expect((grid.xs, grid.ys, stage.whole), (2, 3, true));
    expect(stage.at(grid, 0, 0)!.text, 'Shopping');
    expect(stage.at(grid, 1, 2)!.text, '=SUM(Costs)');
    expect(stage.at(grid, 1, 2)!.result, '10');
    expect(stage.at(grid, 2, 0), isNull);
    expect(stage.at(grid, 0, 3), isNull);
    expect(stage.at(grid, -1, 0), isNull);
    expect(stage.at(grid, 0, -1), isNull);

    final shopping = stage.look(id(stage, 'Shopping'))!;
    final summary = stage.summary(shopping)!;
    expect((summary.owner, summary.xs, summary.ys), (shopping.id, 1, 4));
    expect(summary.preview.map((text) => text.text), [
      'Item',
      'Apples',
      'Bread',
      'Cheese',
    ]);
    // Four cells: given cell by cell too.
    final inside = stage.full(shopping)!;
    expect((inside.owner, inside.level), (shopping.id, 1));
    expect(stage.at(inside, 0, 1)!.text, 'Apples');
    expect(stage.look(stage.at(inside, 0, 1)!.id)!.text, 'Apples');

    final plain = stage.look(id(stage, 'Total'))!;
    expect((stage.summary(plain), stage.full(plain)), (null, null));
    expect(stage.look('no such cell'), isNull);

    // A window of the last row alone is not the whole grid, and its cells are
    // found by their place in the grid, not in the window.
    final window = await read(doc.root, asking(y0: 2, rows: 1));
    expect(window.whole, isFalse);
    expect(window.at(window.grid!, 0, 2)!.text, 'Total');
    expect(window.at(window.grid!, 0, 1), isNull);
    expect(window.look(shopping.id), isNull);

    // A cell without a grid is a stage with a trail and nothing else.
    final bare = await read(plain.id);
    expect([bare.grid, bare.whole, bare.cells], [null, true, isEmpty]);
    expect(bare.trail.map((step) => step.shown.text), ['Example', 'Total']);
  });

  test('read twice, nothing differs', () async {
    final before = await read(doc.root);
    final change = (await read(doc.root)).since(before);
    expect(change.isEmpty, isTrue);
    expect(named(change), [none, none, none, none, false, false]);
    expect(change.touches(before.cells.first.id), isFalse);
    expect(StageChange.none.isEmpty, isTrue);
    expect(StageChange.all.isEmpty, isFalse);
    expect(StageChange.all.touches('anything'), isTrue);
  });

  test('a text of a cell of the stage: that cell', () async {
    final before = await read(doc.root);
    final cell = id(before, 'what to buy');
    await doc.apply(Command.setText(cell: cell, text: 'changed'));
    final change = (await read(doc.root)).since(before);
    expect(named(change), [
      {cell},
      none,
      none,
      none,
      false,
      false,
    ]);
    expect(change.isEmpty, isFalse);
    expect(change.touches(cell), isTrue);
    expect(change.touches(id(before, 'Total')), isFalse);
  });

  test('a text inside a grid given cell by cell: the cell and its grid\'s '
      'reduced form', () async {
    final before = await read(doc.root);
    final apples = id(before, 'Apples');
    final shopping = id(before, 'Shopping');
    await doc.apply(Command.setText(cell: apples, text: 'Pears'));
    final change = (await read(doc.root)).since(before);
    expect(named(change), [
      {apples},
      {shopping},
      none,
      none,
      false,
      false,
    ]);
    expect(change.touches(shopping), isTrue);
  });

  test('a text inside a grid in reduced form: the owner alone', () async {
    final whole = await read(doc.root);
    final apples = id(whole, 'Apples');
    final shopping = id(whole, 'Shopping');
    final before = await read(doc.root, asking(fullDepth: 0));
    expect(before.look(apples), isNull);
    await doc.apply(Command.setText(cell: apples, text: 'Pears'));
    final change = (await read(doc.root, asking(fullDepth: 0))).since(before);
    expect(named(change), [
      none,
      {shopping},
      none,
      none,
      false,
      false,
    ]);
  });

  test('a number that a formula reads: the formula cell too', () async {
    final before = await read(doc.root);
    final three = id(before, '3');
    final sum = id(before, '=SUM(Costs)');
    final costs = id(before, 'Costs');
    await doc.apply(Command.setText(cell: three, text: '30'));
    final after = await read(doc.root);
    expect(after.look(sum)!.result, '37');
    expect(named(after.since(before)), [
      {three, sum},
      {costs},
      none,
      none,
      false,
      false,
    ]);
  });

  test('a cell that gains a grid moves the places of those after it, which '
      'are not named for that', () async {
    final before = await read(doc.root);
    final cell = id(before, 'what to buy');
    final costs = id(before, 'Costs');
    await doc.apply(Command.createGrid(cell: cell, xs: 1, ys: 1));
    final after = await read(doc.root);
    // The grid of "Costs" is the third now, in both lists.
    expect((
      before.look(costs)!.summary,
      before.look(costs)!.full,
    ), isNot((after.look(costs)!.summary, after.look(costs)!.full)));
    expect(before.full(before.look(costs)!)!.first, 10);
    expect(after.full(after.look(costs)!)!.first, 11);
    final made = after.at(after.full(after.look(cell)!)!, 0, 0)!.id;
    final change = after.since(before);
    expect(named(change), [
      {cell, made},
      {cell},
      {cell},
      none,
      false,
      false,
    ]);
    expect(change.touches(costs), isFalse);
    // And back.
    await doc.undo();
    final back = (await read(doc.root)).since(after);
    expect(named(back), [
      {cell},
      {cell},
      {cell},
      {made},
      false,
      false,
    ]);
  });

  test('rows added and removed: the stage\'s grid, and the cells', () async {
    final before = await read(doc.root);
    final made = (await doc.apply(
      Command.insertRows(cell: doc.root, at: 1, count: 1),
    )).created.toSet();
    expect(made.length, 2);
    final after = await read(doc.root);
    expect(named(after.since(before)), [
      made,
      none,
      {doc.root},
      none,
      false,
      false,
    ]);
    await doc.undo();
    expect(named((await read(doc.root)).since(after)), [
      none,
      none,
      {doc.root},
      made,
      false,
      false,
    ]);
  });

  test('a look of the grid itself: the owner', () async {
    final before = await read(doc.root);
    final shopping = id(before, 'Shopping');
    await doc.apply(
      Command.setColumnWidth(cell: shopping, column: 0, width: 30),
    );
    expect(named((await read(doc.root)).since(before)), [
      none,
      none,
      {shopping},
      none,
      false,
      false,
    ]);
  });

  test('the text of the stage cell or of one above it: the trail', () async {
    final root = await read(doc.root);
    final shopping = id(root, 'Shopping');
    final before = await read(shopping);
    await doc.apply(Command.setText(cell: shopping, text: 'Groceries'));
    expect(named((await read(shopping)).since(before)), [
      none,
      none,
      none,
      none,
      true,
      false,
    ]);
    final between = await read(shopping);
    await doc.apply(Command.setText(cell: doc.root, text: 'Top'));
    final change = (await read(shopping)).since(between);
    expect((change.trail, change.isEmpty), (true, false));
  });

  test('a fold: the cell that is folded', () async {
    final before = await read(doc.root);
    final shopping = id(before, 'Shopping');
    await doc.setFolded(shopping, folded: true);
    final after = await read(doc.root);
    expect(after.look(shopping)!.folded, isTrue);
    expect(named(after.since(before)), [
      {shopping},
      none,
      none,
      none,
      false,
      false,
    ]);
  });

  test('a window that moved: what came in and what went', () async {
    final whole = await read(doc.root);
    final first = [id(whole, 'Shopping'), id(whole, 'what to buy')];
    final inside = [
      for (final text in ['Item', 'Apples', 'Bread', 'Cheese']) id(whole, text),
    ];
    final second = [id(whole, 'Costs'), id(whole, 'what it costs')];
    final costs = [
      for (final text in ['3', '2', '5']) id(whole, text),
    ];
    final before = await read(doc.root, asking(rows: 1));
    final after = await read(doc.root, asking(y0: 1, rows: 1));
    expect(named(after.since(before)), [
      {...second, ...costs},
      none,
      {doc.root},
      {...first, ...inside},
      false,
      false,
    ]);
  });

  test('another stage: everything, and nothing is compared', () async {
    final root = await read(doc.root);
    final shopping = await read(id(root, 'Shopping'));
    final change = shopping.since(root);
    expect(named(change), [none, none, none, none, true, true]);
    expect(identical(change, StageChange.all), isTrue);
  });
}
