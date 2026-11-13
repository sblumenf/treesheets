// The stand-in for the core (`fake_document.dart`) against the core itself: what the
// tests of the app's layers see through the stand-in is what the real document would
// show them, as far as the stand-in goes.
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'fake_document.dart';
import 'support.dart';

/// The document the stand-in starts with, made in the real core. Answers with the
/// stand-in's name for each of its cells' IDs.
Future<Map<String, String>> build(Session doc) async {
  await doc.apply(
    Command.import_(
      cell: doc.root,
      replace: false,
      table: const TableData(
        xs: 2,
        cells: [
          EntryData(text: 'one', formula: false),
          EntryData(text: '=X', formula: true),
          EntryData(
            text: 'three',
            formula: false,
            grid: GridSize(xs: 1, cells: 2),
          ),
          EntryData(text: 'five', formula: false),
          EntryData(text: '=1+1', formula: true),
          EntryData(text: 'four', formula: false),
        ],
      ),
    ),
  );
  await doc.apply(Command.setText(cell: doc.root, text: 'Root'));
  final root = (await doc.grid(doc.root)).cells;
  final inner = (await doc.grid(root[2].id)).cells;
  return {
    doc.root: 'root',
    root[0].id: 'a',
    root[1].id: 'b',
    root[2].id: 'c',
    root[3].id: 'd',
    inner[0].id: 'e',
    inner[1].id: 'f',
  };
}

String rgb(Rgb c) => '${c.r},${c.g},${c.b}';

String style(Style s) =>
    '${s.bold}${s.italic}${s.fixed}${s.underline}${s.strike}';

String cut(CutText t) => '"${t.text}"${t.cut ? '…' : ''}';

/// A cell with everything in it, and the names of the stand-in for the IDs.
String look(CellLook c, Map<String, String> names) =>
    '${names[c.id]} "${c.text}" ${c.textCut} ${c.kind} "${c.result}" '
    '${c.resultCut} ${c.failed} ${rgb(c.cellColor)} ${rgb(c.textColor)} '
    '${c.isTag} ${style(c.style)} ${c.relSize} ${c.textAlign} ${c.vertAlign} '
    '${c.runs.length} ${c.links.length} ${c.image} ${c.hasNote} ${c.hasGrid} '
    '${c.folded} ${c.summary} ${c.full}';

String level(LevelInfo? l, Map<String, String> names) => l == null
    ? 'no level'
    : '${names[l.cell]} ${l.x} ${l.y} ${names[l.selected]}';

/// Every field of an answer, line by line.
List<String> lines(StageInfo stage, Map<String, String> names) => [
  for (final t in stage.trail)
    'trail ${names[t.id]} ${cut(t.shown)} ${rgb(t.color)}',
  for (final g in stage.grids)
    'grid ${names[g.owner]} ${g.level} ${g.xs}x${g.ys} ${g.x0},${g.y0} '
        '${g.cols}x${g.rows} ${g.first} ${g.colWidths} ${rgb(g.borderColor)} '
        '${g.outerSpacing} ${g.vertical} ${g.drawStyle}',
  for (final c in stage.cells) 'cell ${look(c, names)}',
  for (final s in stage.summaries)
    'summary ${names[s.owner]} ${s.xs}x${s.ys} ${rgb(s.borderColor)} '
        '${s.previewCols}x${s.previewRows} ${s.preview.map(cut).toList()} '
        '${s.shapeCols}x${s.shapeRows} ${s.shape}',
  level(stage.level, names),
];

StageAsk ask({
  int x0 = 0,
  int y0 = 0,
  int cols = 4294967295,
  int rows = 4294967295,
  int fullDepth = 1,
  int fullCells = 24,
  int previewCols = 3,
  int previewRows = 4,
  int shapeCols = 12,
  int shapeRows = 20,
  int textCut = 2000,
  int previewCut = 40,
}) => StageAsk(
  x0: x0,
  y0: y0,
  cols: cols,
  rows: rows,
  fullDepth: fullDepth,
  fullCells: fullCells,
  previewCols: previewCols,
  previewRows: previewRows,
  shapeCols: shapeCols,
  shapeRows: shapeRows,
  textCut: textCut,
  previewCut: previewCut,
);

void main() {
  late Directory folder;
  late Session real;
  late Session fake;
  late FakeDocument stand;

  /// The stand-in's names of the real document's cells, and the other way round.
  late Map<String, String> names;
  late Map<String, String> ids;
  const same = {
    'root': 'root',
    'a': 'a',
    'b': 'b',
    'c': 'c',
    'd': 'd',
    'e': 'e',
    'f': 'f',
  };
  var count = 0;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-fake');
  });

  tearDownAll(() => folder.delete(recursive: true));

  setUp(() async {
    final made = await Session.create('${folder.path}/doc-${count++}.wombat');
    names = await build(made);
    // Opened again, it has nothing to undo, like the stand-in.
    await made.close();
    real = await Session.open(made.path);
    ids = {for (final MapEntry(:key, :value) in names.entries) value: key};
    stand = FakeDocument();
    fake = await Session.over(stand);
  });

  tearDown(() => real.close());

  /// Both give the same for the same request of the same cell.
  Future<void> alike(String cell, StageAsk ask) async {
    expect(
      lines(await fake.stage(cell, ask), same),
      lines(await real.stage(ids[cell]!, ask), names),
      reason: 'the stage $cell',
    );
  }

  /// Both do the same and end in the same state, or refuse alike.
  Future<void> both(
    Future<Object?> Function(Session doc, String Function(String) id) step,
  ) async {
    Object? one;
    Object? other;
    ErrorKind? refusedOne;
    ErrorKind? refusedOther;
    try {
      one = await step(real, (name) => ids[name]!);
    } on BridgeError catch (error) {
      refusedOne = error.kind;
    }
    try {
      other = await step(fake, (name) => name);
    } on BridgeError catch (error) {
      refusedOther = error.kind;
    }
    expect(refusedOther, refusedOne);
    String told(Object? answer, Map<String, String> names) => switch (answer) {
      final Report r =>
        'changed ${r.changed.map((id) => names[id]).toList()..sort()} '
            '${r.created} ${r.removed} ${r.grids} ${r.tags}',
      final CellLook c => look(c, names),
      final String id => names[id] ?? id,
      _ => '$answer',
    };
    expect(told(other, same), told(one, names));
    expect(await fake.isSaved(), await real.isSaved());
    expect(await fake.inGroup(), await real.inGroup());
    expect(await fake.isViewSaved(), await real.isViewSaved());
    expect(await fake.viewStage(), names[await real.viewStage()]);
    for (final cell in same.keys) {
      expect(
        look(await fake.cell(cell), same),
        look(await real.cell(ids[cell]!), names),
      );
    }
    await alike('root', phoneAsk);
    await alike('c', phoneAsk);
  }

  test(
    'the stage of the document it starts with, asked in many ways',
    () async {
      for (final cell in same.keys) {
        for (final request in [
          phoneAsk,
          ask(fullDepth: 0),
          ask(fullCells: 1),
          ask(fullDepth: 5, fullCells: 2),
          ask(x0: 1, cols: 1),
          ask(y0: 1, rows: 5),
          ask(x0: 7, y0: 7),
          ask(cols: 0),
          ask(previewCols: 1, previewRows: 1, previewCut: 2),
          ask(previewCols: 0, shapeRows: 0),
          ask(shapeCols: 1, shapeRows: 1),
          ask(textCut: 3),
          ask(textCut: 0, previewCut: 0),
        ]) {
          await alike(cell, request);
        }
      }
    },
  );

  test('a request out of range and a cell that is not there', () async {
    for (final doc in [real, fake]) {
      await expectLater(
        doc.stage(doc.root, ask(rows: 4294967296)),
        throwsA(
          isA<BridgeError>().having(
            (e) => e.kind,
            'kind',
            ErrorKind.badRequest,
          ),
        ),
      );
      await expectLater(
        doc.stage(doc.root, ask(textCut: -1)),
        throwsA(isA<BridgeError>()),
      );
    }
    // The real core's IDs have a form, the stand-in's do not: an ID that could be one
    // and is no cell's is not found by either.
    const none = '00000000-0000-4000-8000-000000000000';
    for (final doc in [real, fake]) {
      for (final call in [
        () => doc.stage(none, phoneAsk),
        () => doc.cell(none),
        () => doc.retype(none, 'x'),
        () => doc.viewGo(none),
        () => doc.setFolded(none, folded: true),
        () => doc.viewRemember(LevelInfo(cell: none, x: 0, y: 0)),
        () => doc.viewRemember(
          LevelInfo(cell: doc.root, x: 0, y: 0, selected: none),
        ),
      ]) {
        await expectLater(
          call(),
          throwsA(
            isA<BridgeError>().having(
              (e) => e.kind,
              'kind',
              ErrorKind.notFound,
            ),
          ),
        );
      }
    }
  });

  test('texts, groups, undo and redo, step by step beside the core', () async {
    await both((doc, id) => doc.save());
    await both((doc, id) => doc.retype(id('a'), 'one\r\ntwo\tthree\rfour'));
    await both((doc, id) => doc.retype(id('a'), 'one\ntwo three\nfour'));
    await both((doc, id) => doc.undo());
    await both((doc, id) => doc.redo());
    await both((doc, id) => doc.redo());
    await both((doc, id) => doc.save());

    // A group that is cancelled leaves a saved document saved.
    await both((doc, id) => doc.beginGroup());
    await both(
      (doc, id) => doc.retype(id('e'), 'e\u{301}\u{1F1E9}\u{1F1EA} long'),
    );
    await both(
      (doc, id) => doc.apply(Command.enterText(cell: id('d'), text: '4')),
    );
    await both((doc, id) => doc.undo());
    await both((doc, id) => doc.redo());
    await both((doc, id) => doc.cancelGroup());
    await both((doc, id) => doc.cancelGroup());
    await both((doc, id) => doc.undo());
    await both((doc, id) => doc.redo());

    // One that was saved inside does not.
    await both((doc, id) => doc.beginGroup());
    await both((doc, id) => doc.retype(id('d'), 'held'));
    await both((doc, id) => doc.save());
    await both((doc, id) => doc.cancelGroup());
    await both((doc, id) => doc.save());

    // A group that ends is one step; what could be redone goes with it.
    await both((doc, id) => doc.undo());
    await both((doc, id) => doc.beginGroup());
    await both((doc, id) => doc.beginGroup());
    await both((doc, id) => doc.retype(id('d'), 'first'));
    await both((doc, id) => doc.endGroup());
    await both((doc, id) => doc.retype(id('e'), 'second'));
    await both((doc, id) => doc.retype(id('d'), 'four'));
    await both((doc, id) => doc.endGroup());
    await both((doc, id) => doc.endGroup());
    await both((doc, id) => doc.redo());
    await both((doc, id) => doc.undo());
    await both((doc, id) => doc.undo());
    await both((doc, id) => doc.redo());

    // A group in which nothing changed leaves the redo history.
    await both((doc, id) => doc.undo());
    await both((doc, id) => doc.beginGroup());
    await both((doc, id) => doc.retype(id('d'), 'four'));
    await both((doc, id) => doc.endGroup());
    await both((doc, id) => doc.redo());

    // A cancelled group inside which an inner one was opened.
    await both((doc, id) => doc.beginGroup());
    await both((doc, id) => doc.retype(id('a'), 'x'));
    await both((doc, id) => doc.beginGroup());
    await both((doc, id) => doc.retype(id('a'), 'one\ntwo three\nfour'));
    await both((doc, id) => doc.cancelGroup());
    await both((doc, id) => doc.endGroup());
  });

  test('the view state, step by step beside the core', () async {
    await both((doc, id) => doc.save());
    await both((doc, id) => doc.viewGo(id('c')));
    await both((doc, id) => doc.saveView());
    await both((doc, id) => doc.viewGo(id('c')));
    await both(
      (doc, id) => doc.viewGo(
        id('root'),
        left: LevelInfo(cell: id('c'), x: 5, y: 1, selected: id('f')),
      ),
    );
    await both((doc, id) => doc.viewGo(id('a')));
    await both(
      (doc, id) =>
          doc.viewGo(id('c'), left: LevelInfo(cell: id('a'), x: 0, y: 0)),
    );
    await both(
      (doc, id) => doc.viewRemember(LevelInfo(cell: id('root'), x: 1, y: 9)),
    );
    await both(
      (doc, id) => doc.viewRemember(LevelInfo(cell: id('e'), x: 0, y: 0)),
    );
    await both(
      (doc, id) => doc.viewRemember(LevelInfo(cell: id('root'), x: -1, y: 0)),
    );
    await both((doc, id) => doc.setFolded(id('c'), folded: true));
    await both((doc, id) => doc.saveView());
    await both((doc, id) => doc.setFolded(id('c'), folded: true));
    await both((doc, id) => doc.setFolded(id('a'), folded: true));
    await both((doc, id) => doc.setFolded(id('c'), folded: false));
    await both((doc, id) => doc.viewGo(id('c')));
    // Neither made the document unsaved, which `both` looked at each time.
  });
}
