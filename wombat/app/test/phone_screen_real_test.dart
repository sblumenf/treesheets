// The phone screen, without a device, on the real core: the flows of UI_SPEC section
// 17 that the first slice covers (1, 2, 3, 7, 9 and 10), rows and columns added,
// styles, a document that starts empty, and what the stand-in could differ from the
// core in (a file that another program changed, the app put away, the view state in
// its file). The core answers in real time, which a widget test does not let pass by
// itself: `until` does, and waits for the thing.
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/measure.dart';
import 'package:wombat_app/phone/place.dart' show bold, italic;
import 'package:wombat_app/phone/variants.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/document.dart';
import 'package:wombat_app/src/rust/api/wire.dart';
import 'package:wombat_app/test_documents.dart';

import 'fake_document.dart';
import 'phone_support.dart';
import 'support.dart';

void main() {
  late Directory folder;
  var count = 0;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-phone');
  });

  tearDownAll(() => folder.delete(recursive: true));

  /// Lets real time pass until `done` says so, twenty seconds at most.
  Future<void> until(WidgetTester tester, bool Function() done) async {
    final watch = Stopwatch()..start();
    while (!done() && watch.elapsed < const Duration(seconds: 20)) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 2)),
      );
      await tester.pump();
    }
    if (!done()) {
      // ignore: avoid_print
      print(
        'DBG title=${line(tester, 'title')} message=${line(tester, 'message')} '
        'trouble=${line(tester, 'trouble')} editor=${there('editor') ? editor(tester) : null} '
        'err=${line(tester, 'stage-error')} reading=${there('reading')} '
        'cells=${find.byWidgetPredicate((w) => w.key is ValueKey<String> && (w.key! as ValueKey<String>).value.startsWith('c')).evaluate().map((e) => (e.widget.key! as ValueKey<String>).value).take(40).toList()}',
      );
    }
    expect(done(), isTrue, reason: 'waited twenty seconds');
  }

  /// Does something that asks the core, and waits for it as `until` waits. Not
  /// `tester.runAsync`: what was made on the clock of the test (the keeper, as in
  /// the app) does not move while that waits.
  Future<T> real<T>(WidgetTester tester, Future<T> Function() work) async {
    late T value;
    Object? error;
    var done = false;
    work().then(
      (answer) {
        value = answer;
        done = true;
      },
      onError: (Object failure) {
        error = failure;
        done = true;
      },
    );
    await until(tester, () => done);
    if (error != null) throw error!;
    return value;
  }

  /// A tap, and then until `done`.
  Future<void> tap(
    WidgetTester tester,
    String name,
    bool Function() done,
  ) async {
    await press(tester, name);
    await until(tester, done);
  }

  /// A tap that opens the editor, on the text it is to open with.
  Future<void> edit(WidgetTester tester, String cell, String text) async {
    await reveal(tester, 'cell-$cell');
    await tap(
      tester,
      'cell-$cell',
      () => there('editor') && editor(tester) == text,
    );
  }

  Future<void> closed(WidgetTester tester, String name) =>
      tap(tester, name, () => !there('editor'));

  /// A way into a grid or out of one, until the title says so and the level is
  /// drawn.
  Future<void> go(WidgetTester tester, String name, String title) async {
    await tap(tester, name, () => line(tester, 'title') == title);
    await arrive(tester);
  }

  Future<(Session, Watched)> watch(String path, {bool create = false}) async {
    final watched = Watched(
      create
          ? await OpenDocument.create(path: path)
          : await OpenDocument.open(path: path),
    );
    return (await Session.over(watched), watched);
  }

  /// A document nested four levels deep, small enough to make for every test: the
  /// root has `across` columns and 4 rows; its first cell holds a grid of that
  /// size again, and every third cell after it one of 2 by 2. With two columns
  /// the grids fit cell by cell at the width of a phone; with three they do not.
  Future<(Session, Watched)> nestedOf(
    int across,
    Directory folder,
    String name,
  ) async {
    final file = File('${folder.path}/$name.wombat');
    await writeDocument(
      file,
      nestedTable([(across, 4), const (2, 2), const (2, 2), const (1, 2)]),
      'Nested',
    );
    return watch(file.path);
  }

  Future<(Session, Watched)> nested(Directory folder, String name) =>
      nestedOf(2, folder, name);

  Future<(Session, Watched)> nestedWide(Directory folder, String name) =>
      nestedOf(3, folder, name);

  Future<(Session, Watched)> empty(Directory folder, String name) =>
      watch('${folder.path}/$name.wombat', create: true);

  /// A test with the screen on a document of the real core, kept as the app keeps
  /// its own, at the size of a phone; the core is under a watcher that can make
  /// one call fail. The document is a copy of the example unless `open` makes
  /// another.
  void realTest(
    String name,
    Future<void> Function(
      WidgetTester tester,
      Keeper keeper,
      Session doc,
      Watched watch,
    )
    body, {
    Future<(Session, Watched)> Function(Directory folder, String name) open =
        watchExample,
    Variants variants = const Variants(),
  }) {
    testWidgets(name, (tester) async {
      screenSize(tester, phone);
      addTearDown(
        () => tester.binding.handleAppLifecycleStateChanged(
          AppLifecycleState.resumed,
        ),
      );
      final (doc, watch) = await real(
        tester,
        () => open(folder, 'phone-${count++}'),
      );
      final keeper = Keeper(doc);
      await tester.pumpWidget(
        phoneApp(keeper, variants: ValueNotifier(variants)),
      );
      await until(tester, () => there('stage-across') || there('empty-root'));
      await arrive(tester);
      await body(tester, keeper, doc, watch);
      final thrown = tester.takeException();
      // ignore: avoid_print
      if (thrown != null) print('DBG thrown: $thrown');
      expect(thrown, isNull);
      await real(tester, () => endKeeping(keeper));
    });
  }

  /// The texts of the grid of `cell` as the document has them, row by row.
  Future<List<String>> texts(
    WidgetTester tester,
    Keeper keeper,
    String cell,
  ) async => [
    for (final cell in (await real(tester, () => keeper.grid(cell))).cells)
      cell.text,
  ];

  Future<String> idAt(
    WidgetTester tester,
    Keeper keeper,
    String owner,
    int index,
  ) async => (await real(tester, () => keeper.grid(owner))).cells[index].id;

  const lifeOut = [AppLifecycleState.inactive, AppLifecycleState.hidden];
  const lifeBack = [AppLifecycleState.inactive, AppLifecycleState.resumed];
  Future<void> life(WidgetTester tester, List<AppLifecycleState> states) async {
    for (final state in states) {
      await lifecycle(tester, state);
    }
  }

  group('the flows of the specification', () {
    late String sixLevels;
    setUpAll(() async {
      // The generated document of the specification, made once: six levels deep,
      // 12 by 20 at the root. These two flows only look at it.
      final doc = await openTestDocument(TestDocument.nested, folder);
      sixLevels = doc.path;
      await doc.close();
    });

    realTest('1: find a cell three levels deep without search', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      expect(line(tester, 'title'), 'Six levels');
      expect(shown(tester, '3-0'), 'Level 0 cell 3.0 text');
      // Its grid of 4 by 5 does not fit cell by cell: a Preview, one target.
      expect(key('preview-3-0'), findsOneWidget);
      await go(tester, 'child-3-0', 'Level 0 cell 3.0 text');
      expect(shown(tester, '3-0'), 'Level 1 cell 3.0 text');
      await go(tester, 'child-3-0', 'Level 1 cell 3.0 text');
      expect(shown(tester, '0-1'), 'Level 2 cell 0.1 text');
      await go(tester, 'child-0-1', 'Level 2 cell 0.1 text');
      // Three levels down.
      expect(shown(tester, '1-1'), 'Level 3 cell 1.1 text');
      await edit(tester, '1-1', 'Level 3 cell 1.1 text');
      await closed(tester, 'cancel');
      // The trail says where that is, and leads back.
      expect(
        [
          for (var i = 0; i < 4; i++)
            tester
                .widget<Text>(
                  find.descendant(
                    of: key('trail-$i'),
                    matching: find.byType(Text),
                  ),
                )
                .data,
        ],
        [
          'Six levels',
          '› Level 0 cell 3.0 text',
          '› Level 1 cell 3.0 text',
          '› Level 2 cell 0.1 text',
        ],
      );
      await go(tester, 'trail-0', 'Six levels');
    }, open: (_, _) => watch(sixLevels));

    realTest('9: in a grid of more than nine columns, reach row 12, column 7', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      expect(key('cell-6-11'), findsNothing);
      // Sideways and down, by hand.
      await tester.drag(key('stage-across'), const Offset(-520, 0));
      await tester.pumpAndSettle();
      for (var i = 0; i < 20 && !there('cell-6-11'); i++) {
        await tester.drag(key('stage-down-0'), const Offset(0, -300));
        await tester.pumpAndSettle();
      }
      // Column 7 of row 12, counted from 1, and the first row and column
      // still in view to say where that is.
      expect(shown(tester, '6-11'), 'Level 0 cell 6.11 text');
      expect(shown(tester, '0-0'), 'Level 0 cell 0.0 text');
      expect(shown(tester, '6-0'), 'Level 0 cell 6.0 text');
      expect(shown(tester, '0-11'), 'Level 0 cell 0.11 text');
      await edit(tester, '6-11', 'Level 0 cell 6.11 text');
      await closed(tester, 'cancel');
    }, open: (_, _) => watch(sixLevels));

    realTest('2: edit a cell two levels deep, then return to the top', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      // The grids are drawn cell by cell here: into one by its cell and "Open ›".
      expect(key('full-0-0'), findsOneWidget);
      await edit(tester, '0-0', 'Level 0 cell 0.0 text');
      await go(tester, 'tool-open', 'Level 0 cell 0.0 text');
      await edit(tester, '0-0', 'Level 1 cell 0.0 text');
      await go(tester, 'tool-open', 'Level 1 cell 0.0 text');
      await edit(tester, '1-0', 'Level 2 cell 1.0 text');
      type(tester, 'edited two levels down');
      await closed(tester, 'save');
      expect(shown(tester, '1-0'), 'edited two levels down');
      await go(tester, 'trail-0', 'Nested');
      expect(shown(tester, '0-0'), 'Level 0 cell 0.0 text');
      // The document has it, two levels below the root.
      final one = await idAt(tester, keeper, keeper.root, 0);
      final two = await idAt(tester, keeper, one, 0);
      expect((await texts(tester, keeper, two))[1], 'edited two levels down');
    }, open: nested);

    realTest(
      '3 and 7: add a row to a nested grid and fill two cells; undo three times',
      (tester, keeper, doc, watch) async {
        // The grid of 2 by 2 in the second row, second column, drawn cell by cell.
        final owner = await idAt(tester, keeper, keeper.root, 3);
        expect(await texts(tester, keeper, owner), [
          'Level 1 cell 0.0 text',
          'Level 1 cell 1.0 text',
          'Level 1 cell 0.1 text',
          'Level 1 cell 1.1 text',
        ]);
        // Return in its last row adds a row, and the editor is on the new cell.
        await edit(tester, '1-1/0-1', 'Level 1 cell 0.1 text');
        await pressReturn(tester);
        await until(
          tester,
          () => there('cell-1-1/0-2') && editor(tester) == '',
        );
        type(tester, 'first');
        // → moves to the cell on the right, in the new row.
        await tap(
          tester,
          'tool-right',
          () => shown(tester, '1-1/0-2') == 'first' && editor(tester) == '',
        );
        type(tester, 'second');
        // (The field is empty from the press on: wait for the editor itself.)
        await until(tester, () => shown(tester, '1-1/1-2') == 'second');
        await closed(tester, 'save');
        expect(await texts(tester, keeper, owner), [
          'Level 1 cell 0.0 text',
          'Level 1 cell 1.0 text',
          'Level 1 cell 0.1 text',
          'Level 1 cell 1.1 text',
          'first',
          'second',
        ]);
        expect(line(tester, 'title'), 'Nested');

        // Flow 7: the last three changes, one after the other: the second text,
        // the first, and the row.
        await tap(tester, 'undo', () => shown(tester, '1-1/1-2') == '');
        await tap(tester, 'undo', () => shown(tester, '1-1/0-2') == '');
        await tap(tester, 'undo', () => !there('cell-1-1/0-2'));
        expect((await texts(tester, keeper, owner)).length, 4);
        // And nothing more: the text Return entered had not changed.
        await tap(tester, 'undo', () => line(tester, 'message') != null);
        expect(line(tester, 'message'), 'Nothing to undo');
        // Redo brings all three back.
        for (var i = 0; i < 3; i++) {
          await press(tester, 'redo');
        }
        await until(tester, () => there('cell-1-1/1-2'));
        await until(tester, () => shown(tester, '1-1/1-2') == 'second');
      },
      open: nested,
    );

    realTest(
      '10: fix the text of three cells in a small nested grid, then continue '
      'at the top',
      (tester, keeper, doc, watch) async {
        // "Shopping" holds a grid of four cells, drawn cell by cell.
        await edit(tester, '0-0/0-1', 'Apples');
        type(tester, 'Pears');
        await pressReturn(tester);
        await until(tester, () => editor(tester) == 'Bread');
        type(tester, 'Rolls');
        await pressReturn(tester);
        await until(tester, () => editor(tester) == 'Cheese');
        type(tester, 'Butter');
        await closed(tester, 'save');
        expect(
          [for (var y = 0; y < 4; y++) shown(tester, '0-0/0-$y')],
          ['Item', 'Pears', 'Rolls', 'Butter'],
        );
        // And on at the top, without having left it.
        expect(line(tester, 'title'), 'Example');
        await edit(tester, '1-0', 'what to buy');
        type(tester, 'what was bought');
        await closed(tester, 'save');
        expect(shown(tester, '1-0'), 'what was bought');
        final shopping = await idAt(tester, keeper, keeper.root, 0);
        expect(await texts(tester, keeper, shopping), [
          'Item',
          'Pears',
          'Rolls',
          'Butter',
        ]);
        // Four steps of the undo history, one for each cell.
        for (final (cell, text) in [
          ('1-0', 'what to buy'),
          ('0-0/0-3', 'Cheese'),
          ('0-0/0-2', 'Bread'),
          ('0-0/0-1', 'Apples'),
        ]) {
          await tap(tester, 'undo', () => shown(tester, cell) == text);
        }
      },
    );
  });

  group('rows and columns', () {
    realTest('Return in the last row adds a row: two steps to undo', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '0-2', 'Total');
      type(tester, 'Sum');
      await pressReturn(tester);
      await until(tester, () => there('cell-0-3') && editor(tester) == '');
      expect(shown(tester, '0-2'), 'Sum');
      expect(focused(tester), isTrue);
      type(tester, 'below');
      await closed(tester, 'save');
      expect(await texts(tester, keeper, keeper.root), [
        'Shopping',
        'what to buy',
        'Costs',
        'what it costs',
        'Sum',
        '=SUM(Costs)',
        'below',
        '',
      ]);
      // The text of the new cell, the row, and the text before it.
      await tap(tester, 'undo', () => shown(tester, '0-3') == '');
      // The new cell is the selected one, which "Edit" would open.
      expect(tester.widget<TextButton>(key('bar-edit')).onPressed, isNotNull);
      await tap(tester, 'undo', () => !there('cell-0-3'));
      // It went with its row, and so did the selection.
      expect(tester.widget<TextButton>(key('bar-edit')).onPressed, isNull);
      expect(shown(tester, '0-2'), 'Sum');
      await tap(tester, 'undo', () => shown(tester, '0-2') == 'Total');
    });

    // Decision A193, on the real core: what is typed after Return is the next
    // cell's, also while that cell's row is still being made.
    realTest('typed while Return in the last row adds the row: in the new row', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '0-2', 'Total');
      type(tester, 'Sum');
      // Held where the new cell is read: the text is in, and the row is there.
      final gate = holdFirst(watch, 'cell');
      await pressReturn(tester);
      await until(tester, () => watch.log.last == 'cell');
      expect(shown(tester, '0-2'), 'Sum');
      expect(there('cell-0-3'), isTrue);
      expect((editor(tester), focused(tester)), ('', true));
      type(tester, 'of it all');
      await tester.pump();
      expect(shown(tester, '0-2'), 'Sum');
      gate.complete();
      await until(tester, () => shown(tester, '0-3') == 'of it all');
      expect(editor(tester), 'of it all');
      expect(line(tester, 'message'), isNull);
      await closed(tester, 'save');
      expect((await texts(tester, keeper, keeper.root)).sublist(4), [
        'Sum',
        '=SUM(Costs)',
        'of it all',
        '',
      ]);
    });

    realTest(
      'two Returns in the last row, typed without waiting for the core',
      (tester, keeper, doc, watch) async {
        await edit(tester, '0-2', 'Total');
        type(tester, 'Sum');
        await pressReturn(tester);
        type(tester, 'x');
        await pressReturn(tester);
        type(tester, 'y');
        await until(
          tester,
          () => there('cell-0-4') && shown(tester, '0-4') == 'y',
        );
        expect((shown(tester, '0-2'), shown(tester, '0-3')), ('Sum', 'x'));
        expect(editor(tester), 'y');
        await closed(tester, 'save');
        expect((await texts(tester, keeper, keeper.root)).sublist(4), [
          'Sum',
          '=SUM(Costs)',
          'x',
          '',
          'y',
          '',
        ]);
      },
    );

    realTest('typed right after +Row and +Col: in the new cells', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '0-0', 'Shopping');
      await press(tester, 'tool-row');
      type(tester, 'new row');
      await press(tester, 'tool-col');
      type(tester, 'new column');
      await until(
        tester,
        () => there('cell-2-1') && shown(tester, '1-1') == 'new column',
      );
      await closed(tester, 'save');
      expect((await texts(tester, keeper, keeper.root)).sublist(0, 6), [
        'Shopping',
        '',
        'what to buy',
        'new row',
        'new column',
        '',
      ]);
    });

    realTest(
      'the screen goes while Return in the last row enters the text: the '
      'text is kept, and no row is added behind it',
      (tester, keeper, doc, watch) async {
        await edit(tester, '0-2', 'Total');
        type(tester, 'Sum');
        final gate = holdFirst(watch, 'retype');
        await pressReturn(tester);
        await tester.pumpWidget(const MaterialApp(home: SizedBox()));
        gate.complete();
        await real(tester, keeper.save);
        final all = await texts(tester, keeper, keeper.root);
        expect((all.length, all[4]), (6, 'Sum'));
        expect(keeper.editing, isNull);
      },
    );

    realTest('→ in the last column adds a column', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '1-1', 'what it costs');
      await tap(
        tester,
        'tool-right',
        () => there('cell-2-1') && editor(tester) == '',
      );
      type(tester, 'beside');
      await closed(tester, 'save');
      expect((await texts(tester, keeper, keeper.root)).length, 9);
      expect(shown(tester, '2-1'), 'beside');
      expect((shown(tester, '2-0'), shown(tester, '2-2')), ('', ''));
      // The text that → entered had not changed: the column and the new text.
      await tap(tester, 'undo', () => shown(tester, '2-1') == '');
      await tap(tester, 'undo', () => !there('cell-2-1'));
      await tap(tester, 'undo', () => line(tester, 'message') != null);
    });

    realTest('+Row and +Col insert beside the edited cell', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '0-0', 'Shopping');
      type(tester, 'Shop');
      await tap(
        tester,
        'tool-row',
        () => editor(tester) == '' && shown(tester, '0-2') == 'Costs',
      );
      // Below the first row, not at the end; the text was kept.
      expect(shown(tester, '0-0'), 'Shop');
      type(tester, 'new row');
      await tap(
        tester,
        'tool-col',
        () => there('cell-2-0') && shown(tester, '2-0') == 'what to buy',
      );
      type(tester, 'new column');
      await closed(tester, 'save');
      expect(await texts(tester, keeper, keeper.root), [
        'Shop',
        '',
        'what to buy',
        'new row',
        'new column',
        '',
        'Costs',
        '',
        'what it costs',
        'Total',
        '',
        '=SUM(Costs)',
      ]);
    });

    realTest(
      'a first column that takes most of the width is not kept in view',
      (tester, keeper, doc, watch) async {
        // Five times as wide as usual: as wide as the screen has room.
        await real(
          tester,
          () => keeper.apply(
            Command.setColumnWidth(cell: keeper.root, column: 0, width: 400),
          ),
        );
        await until(tester, () => tester.getSize(key('cell-0-0')).width > 300);
        expect(tester.getSize(key('cell-0-0')).width, phone.width - 48);
        await tester.drag(key('stage-across'), const Offset(-200, 0));
        await tester.pumpAndSettle();
        // It went sideways with the rest.
        expect(tester.getTopLeft(key('cell-0-0')).dx, lessThan(-100));
        expect(tester.getTopLeft(key('cell-0-1')).dx, lessThan(-100));
      },
    );

    realTest('the bars after the last row and the last column', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await tap(tester, 'add-row', () => there('editor'));
      expect(editor(tester), '');
      type(tester, 'a new row');
      await tester.pump();
      // The editor is on the first cell of the new row.
      expect(shown(tester, '0-3'), 'a new row');
      await closed(tester, 'save');
      await tap(tester, 'add-column', () => there('editor'));
      type(tester, 'a new column');
      await tester.pump();
      expect(shown(tester, '2-0'), 'a new column');
      // With the editor open the bar keeps the text and adds one more.
      await tap(
        tester,
        'add-column',
        () => there('cell-3-0') && there('editor') && editor(tester) == '',
      );
      expect(shown(tester, '2-0'), 'a new column');
      await closed(tester, 'cancel');
      final all = await texts(tester, keeper, keeper.root);
      expect(all.length, 16);
      expect((all[2], all[12]), ('a new column', 'a new row'));
    });

    realTest('"+ Add" adds beside the selected cell, or at the end', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      // Nothing selected: at the end.
      await choose(tester, 'bar-add', 'add-row-below');
      await until(tester, () => there('editor') && there('cell-0-3'));
      type(tester, 'last');
      await closed(tester, 'save');
      // "what to buy" selected: below it, and to the right of it.
      await edit(tester, '1-0', 'what to buy');
      await closed(tester, 'cancel');
      await choose(tester, 'bar-add', 'add-row-below');
      await until(tester, () => there('editor') && there('cell-0-4'));
      type(tester, 'below it');
      await tester.pump();
      expect(shown(tester, '1-1'), 'below it');
      await closed(tester, 'save');
      await edit(tester, '1-0', 'what to buy');
      await closed(tester, 'cancel');
      await choose(tester, 'bar-add', 'add-column-right');
      await until(tester, () => there('editor') && there('cell-2-0'));
      type(tester, 'beside it');
      await tester.pump();
      expect(shown(tester, '2-0'), 'beside it');
      await closed(tester, 'save');
      expect((await texts(tester, keeper, keeper.root)).length, 15);
    });

    realTest(
      'a row added to a grid that is unfolded: it stays cell by cell',
      (tester, keeper, doc, watch) async {
        // The first cell's grid of 3 by 4, made 3 by 8: 24 cells, as many as a
        // read gives cell by cell. Unfolded, since three columns do not fit.
        final owner = await idAt(tester, keeper, keeper.root, 0);
        await real(
          tester,
          () => keeper.apply(Command.insertRows(cell: owner, at: 4, count: 4)),
        );
        await tester.pump();
        await press(tester, 'child-0-0');
        await until(tester, () => there('full-0-0'));
        await edit(tester, '0-0/1-0', 'Level 1 cell 1.0 text');
        type(tester, 'kept');
        await press(tester, 'tool-row');
        // 27 cells now, and still cell by cell, with the editor on the new row.
        await until(
          tester,
          () => there('cell-0-0/0-8') && shown(tester, '0-0/1-1') == '',
        );
        expect((there('editor'), editor(tester)), (true, ''));
        expect(shown(tester, '0-0/1-0'), 'kept');
        await closed(tester, 'save');
        expect((await texts(tester, keeper, owner)).length, 27);
        // An undo that makes it smaller, and a redo that makes it larger again.
        await tap(tester, 'undo', () => !there('cell-0-0/0-8'));
        await tap(tester, 'redo', () => there('cell-0-0/0-8'));
        expect(key('full-0-0'), findsOneWidget);
      },
      open: nestedWide,
      variants: const Variants(tapChildUnfolds: true),
    );

    realTest(
      'a grid that is unfolded and then grows by a redo stays cell by cell',
      (tester, keeper, doc, watch) async {
        // 24 cells, with a ninth row made and undone behind the screen.
        final owner = await idAt(tester, keeper, keeper.root, 0);
        await real(tester, () async {
          await keeper.apply(Command.insertRows(cell: owner, at: 4, count: 4));
          await keeper.apply(Command.insertRows(cell: owner, at: 8, count: 1));
          await keeper.undo();
        });
        await tester.pump();
        await press(tester, 'child-0-0');
        await until(tester, () => there('full-0-0'));
        expect(key('cell-0-0/0-8'), findsNothing);
        // The redo makes it larger than the read was asked to give.
        await tap(tester, 'redo', () => there('cell-0-0/0-8'));
        expect(key('full-0-0'), findsOneWidget);
      },
      open: nestedWide,
      variants: const Variants(tapChildUnfolds: true),
    );

    realTest(
      'a row added to a Full grid that then has too many cells for that: the '
      'editor closes, nothing is lost',
      (tester, keeper, doc, watch) async {
        // The grid of 2 by 2 in the second row, second column, made 2 by 12: 24 cells.
        final owner = await idAt(tester, keeper, keeper.root, 3);
        await real(
          tester,
          () => keeper.apply(Command.insertRows(cell: owner, at: 2, count: 10)),
        );
        await until(tester, () => there('cell-1-1/0-11'));
        await edit(tester, '1-1/1-0', 'Level 1 cell 1.0 text');
        type(tester, 'kept');
        await press(tester, 'tool-row');
        // 26 cells: a Preview now, which has no cell for the editor to be on.
        await until(tester, () => there('preview-1-1'));
        expect(key('editor'), findsNothing);
        expect(key('full-1-1'), findsNothing);
        final all = await texts(tester, keeper, owner);
        expect((all.length, all[1], all[3]), (26, 'kept', ''));
      },
      open: nested,
    );

    realTest('a document that starts empty: the first cell, and on from there', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      expect(key('empty-root'), findsOneWidget);
      expect(line(tester, 'title'), 'Wombat');
      // Two taps in a row make one cell, and nothing is said about the second.
      await tester.tap(key('first-cell'));
      await tap(tester, 'first-cell', () => there('editor'));
      expect(line(tester, 'message'), isNull);
      expect(key('cell-1-0'), findsNothing);
      expect(key('cell-0-1'), findsNothing);
      expect(key('empty-root'), findsNothing);
      expect(editor(tester), '');
      type(tester, 'the first');
      await pressReturn(tester);
      await until(tester, () => there('cell-0-1') && editor(tester) == '');
      type(tester, 'the second');
      await closed(tester, 'save');
      expect(await texts(tester, keeper, keeper.root), [
        'the first',
        'the second',
      ]);
      // A root of one cell again, and then of none: the grid goes with its
      // last undo, and the screen says the document is empty.
      await tap(tester, 'undo', () => shown(tester, '0-1') == '');
      await tap(tester, 'undo', () => !there('cell-0-1'));
      expect(shown(tester, '0-0'), 'the first');
      expect(key('add-row'), findsOneWidget);
      await tap(tester, 'undo', () => shown(tester, '0-0') == '');
      await tap(tester, 'undo', () => there('empty-root'));
      await tap(tester, 'redo', () => there('cell-0-0'));
    }, open: empty);
  });

  group('styles', () {
    Future<CellLook> cell(WidgetTester tester, Keeper keeper, int index) async {
      final id = await idAt(tester, keeper, keeper.root, index);
      return real(tester, () => keeper.cell(id));
    }

    realTest('bold and italic on what is selected, and off again', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '1-0', 'what to buy');
      await press(tester, 'tool-styles');
      editorField(tester).selection = const TextSelection(
        baseOffset: 5,
        extentOffset: 7,
      );
      await press(tester, 'style-bold');
      await until(tester, () => keeper.editing?.held ?? false);
      // The cell above the keyboard shows it while the editor is open.
      await until(
        tester,
        () =>
            (cellText(tester, '1-0').textSpan! as TextSpan).children!.length ==
            3,
      );
      final spans = (cellText(tester, '1-0').textSpan! as TextSpan).children!;
      expect(
        [for (final span in spans) (span as TextSpan).text],
        ['what ', 'to', ' buy'],
      );
      expect((spans[1] as TextSpan).style!.fontWeight, FontWeight.bold);
      // More typed after it: the cell goes on showing the editor's text.
      type(tester, 'what to buy now');
      await tester.pump();
      expect(shown(tester, '1-0'), 'what to buy now');
      // In one piece: the ranges the document holds are of the text it holds.
      expect(
        (cellText(tester, '1-0').textSpan! as TextSpan).children!.length,
        1,
      );
      editorField(tester).selection = const TextSelection(
        baseOffset: 0,
        extentOffset: 4,
      );
      await press(tester, 'style-italic');
      await until(
        tester,
        () =>
            (cellText(tester, '1-0').textSpan! as TextSpan).children!.length ==
            4,
      );
      await closed(tester, 'save');
      var look = await cell(tester, keeper, 1);
      expect(look.text, 'what to buy now');
      expect(
        [for (final run in look.runs) (run.start, run.len, run.style)],
        [(0, 4, italic), (5, 2, bold)],
      );
      // Drawn so with the editor closed.
      expect(
        (cellText(tester, '1-0').textSpan! as TextSpan).children!.length,
        4,
      );
      // The whole editing, styles and all, is one step to undo.
      await tap(tester, 'undo', () => shown(tester, '1-0') == 'what to buy');
      expect((await cell(tester, keeper, 1)).runs, isEmpty);
      await tap(
        tester,
        'redo',
        () => shown(tester, '1-0') == 'what to buy now',
      );

      // The editor opens on a cell that has styles: they are drawn for as long
      // as the text is the document's, and come back when it is held there.
      await edit(tester, '1-0', 'what to buy now');
      expect(
        (cellText(tester, '1-0').textSpan! as TextSpan).children!.length,
        4,
      );
      type(tester, 'what to buy now!');
      await tester.pump();
      expect(
        (cellText(tester, '1-0').textSpan! as TextSpan).children!.length,
        1,
      );
      await life(tester, lifeOut);
      await until(tester, () => keeper.editing?.held ?? false);
      await life(tester, lifeBack);
      await until(
        tester,
        () =>
            (cellText(tester, '1-0').textSpan! as TextSpan).children!.length ==
            4,
      );
      await closed(tester, 'cancel');
      expect(shown(tester, '1-0'), 'what to buy now');

      // Bold on a range that is bold switches it off.
      await edit(tester, '1-0', 'what to buy now');
      await press(tester, 'tool-styles');
      editorField(tester).selection = const TextSelection(
        baseOffset: 5,
        extentOffset: 7,
      );
      await press(tester, 'style-bold');
      await until(tester, () => keeper.editing?.held ?? false);
      await closed(tester, 'save');
      look = await cell(tester, keeper, 1);
      expect(
        [for (final run in look.runs) (run.start, run.len, run.style)],
        [(0, 4, italic)],
      );
    });

    realTest('without a selection, the whole cell; the cross takes it back', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '1-0', 'what to buy');
      await press(tester, 'tool-styles');
      await press(tester, 'style-bold');
      await until(
        tester,
        () => cellText(tester, '1-0').style!.fontWeight == FontWeight.bold,
      );
      final id = await idAt(tester, keeper, keeper.root, 1);
      // In the document already, held by the editing.
      expect((await real(tester, () => keeper.cell(id))).style.bold, isTrue);
      await closed(tester, 'cancel');
      expect((await cell(tester, keeper, 1)).style.bold, isFalse);
      expect(cellText(tester, '1-0').style!.fontWeight, isNull);
      await tap(tester, 'undo', () => line(tester, 'message') != null);
      expect(line(tester, 'message'), 'Nothing to undo');
      // Said yes to, it stays; and once more switches it off.
      await edit(tester, '1-0', 'what to buy');
      await press(tester, 'tool-styles');
      await press(tester, 'style-bold');
      await until(tester, () => keeper.editing?.held ?? false);
      await closed(tester, 'save');
      expect((await cell(tester, keeper, 1)).style.bold, isTrue);
      await edit(tester, '1-0', 'what to buy');
      await press(tester, 'tool-styles');
      await press(tester, 'style-bold');
      await until(tester, () => keeper.editing?.held ?? false);
      await closed(tester, 'save');
      expect((await cell(tester, keeper, 1)).style.bold, isFalse);
    });

    realTest('a link on what is selected, on all of it, and removed', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      Future<void> link(String target, {bool ok = true}) async {
        await press(tester, 'style-link');
        await tester.pumpAndSettle();
        await tester.enterText(key('link-target'), target);
        await tester.tap(key(ok ? 'link-ok' : 'link-cancel'));
        await tester.pumpAndSettle();
      }

      await edit(tester, '1-0', 'what to buy');
      await press(tester, 'tool-styles');
      editorField(tester).selection = const TextSelection(
        baseOffset: 8,
        extentOffset: 11,
      );
      await link('  https://example.org/buy ');
      await until(tester, () => keeper.editing?.held ?? false);
      // Back in the editor, which has the keyboard again.
      expect(focused(tester), isTrue);
      await closed(tester, 'save');
      var look = await cell(tester, keeper, 1);
      expect(
        [for (final link in look.links) (link.start, link.len, link.target)],
        [(8, 3, const LinkTarget.external_(target: 'https://example.org/buy'))],
      );
      // Drawn as one.
      final spans = (cellText(tester, '1-0').textSpan! as TextSpan).children!;
      expect((spans.last as TextSpan).text, 'buy');
      expect(
        (spans.last as TextSpan).style!.decoration,
        TextDecoration.underline,
      );
      // Put aside: nothing is done.
      await edit(tester, '1-0', 'what to buy');
      await press(tester, 'tool-styles');
      await link('https://example.org/other', ok: false);
      // The tick after it: the link that was there is there, and no other.
      await closed(tester, 'save');
      look = await cell(tester, keeper, 1);
      expect([for (final link in look.links) (link.start, link.len)], [(8, 3)]);
      // Without a selection, the whole text; with nothing typed, removed.
      await edit(tester, '1-0', 'what to buy');
      await press(tester, 'tool-styles');
      await link('');
      await until(tester, () => keeper.editing?.held ?? false);
      await closed(tester, 'save');
      look = await cell(tester, keeper, 1);
      expect(look.links, isEmpty);
      await edit(tester, '1-0', 'what to buy');
      await press(tester, 'tool-styles');
      await link('notes.txt');
      await until(tester, () => keeper.editing?.held ?? false);
      await closed(tester, 'save');
      look = await cell(tester, keeper, 1);
      expect(
        [for (final link in look.links) (link.start, link.len)],
        [(0, 11)],
      );
      // An empty cell has nothing to make a link of.
      await edit(tester, '1-0', 'what to buy');
      type(tester, '');
      await press(tester, 'tool-styles');
      await link('notes.txt');
      await until(tester, () => line(tester, 'message') != null);
      expect(line(tester, 'message'), 'There is no text to make a link of.');
      await closed(tester, 'cancel');
    });

    realTest('a tag: the whole text of the cell, on and off', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '1-0', 'what to buy');
      type(tester, 'todo');
      await press(tester, 'tool-styles');
      await press(tester, 'style-tag');
      await until(tester, () => keeper.editing?.held ?? false);
      await closed(tester, 'save');
      var look = await cell(tester, keeper, 1);
      expect((look.text, look.isTag), ('todo', true));
      // Off again; the text stays.
      await edit(tester, '1-0', 'todo');
      await press(tester, 'tool-styles');
      await press(tester, 'style-tag');
      await until(tester, () => keeper.editing?.held ?? false);
      await closed(tester, 'save');
      look = await cell(tester, keeper, 1);
      expect((look.text, look.isTag), ('todo', false));
      // A text of several lines is no tag: the core says so, the editor stays.
      await edit(tester, '1-0', 'todo');
      type(tester, 'to\ndo');
      await press(tester, 'tool-styles');
      await tap(tester, 'style-tag', () => line(tester, 'message') != null);
      expect(line(tester, 'message'), startsWith('Error (refused): '));
      expect(editor(tester), 'to\ndo');
      await closed(tester, 'cancel');
      expect((await cell(tester, keeper, 1)).text, 'todo');
    });
  });

  group('text', () {
    realTest('of several lines, and written from right to left', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '1-0', 'what to buy');
      editorField(tester).selection = const TextSelection.collapsed(offset: 4);
      await press(tester, 'tool-break');
      expect(editor(tester), 'what\n to buy');
      await closed(tester, 'save');
      expect((await texts(tester, keeper, keeper.root))[1], 'what\n to buy');
      expect(shown(tester, '1-0'), 'what\n to buy');
      // Pasted with the line ends of another system: one line break each.
      await edit(tester, '1-0', 'what\n to buy');
      type(tester, 'one\r\ntwo\rthree');
      await closed(tester, 'save');
      expect((await texts(tester, keeper, keeper.root))[1], 'one\ntwo\nthree');
      await edit(tester, '1-1', 'what it costs');
      type(tester, 'שלום עולם');
      await tester.pump();
      expect(cellText(tester, '1-1').textDirection, TextDirection.rtl);
      await closed(tester, 'save');
      expect((await texts(tester, keeper, keeper.root))[3], 'שלום עולם');
      expect(cellText(tester, '1-1').textDirection, TextDirection.rtl);
    });

    realTest('a formula typed: its result, and the cells that read it', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      expect(shown(tester, '1-2'), '10');
      await edit(tester, '0-1/0-0', '3');
      type(tester, '30');
      await closed(tester, 'save');
      await until(tester, () => shown(tester, '1-2') == '37');
      await edit(tester, '1-2', '=SUM(Costs)');
      type(tester, '=SUM(Costs)*2');
      await closed(tester, 'save');
      await until(tester, () => shown(tester, '1-2') == '74');
    });

    realTest('what the core refuses to enter stays in the editor', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      // A new row, and the editor on a cell of it; then the row is taken away
      // again behind the screen, as a second screen on the document could do.
      await real(
        tester,
        () => keeper.apply(
          Command.insertRows(cell: keeper.root, at: 3, count: 1),
        ),
      );
      await until(tester, () => there('cell-0-3'));
      await edit(tester, '0-3', '');
      type(tester, 'typed into a cell that is gone');
      await real(tester, keeper.undo);
      await tap(tester, 'save', () => line(tester, 'message') != null);
      expect(line(tester, 'message'), startsWith('Error (notFound): '));
      expect(editor(tester), 'typed into a cell that is gone');
      expect(focused(tester), isTrue);
      await closed(tester, 'cancel');
    });
  });

  group('the file', () {
    /// Types "mine" into the first cell, lets another program put `theirs` in
    /// the file, and waits for the save that comes by itself to find that out.
    Future<void> changedByAnother(
      WidgetTester tester,
      Session doc,
      String Function(String file) theirs,
    ) async {
      await edit(
        tester,
        '0-0',
        line(tester, 'title') == 'Costs' ? '3' : 'Shopping',
      );
      type(tester, 'mine');
      await closed(tester, 'save');
      final file = File(doc.path);
      file.writeAsStringSync(theirs(file.readAsStringSync()));
      await tester.pump(const Duration(seconds: 1));
      await until(tester, () => line(tester, 'trouble') != null);
      expect(
        line(tester, 'trouble'),
        'Not saved: another program changed the file.',
      );
    }

    String renamed(String file) => file.replaceFirst('"Shopping"', '"Theirs"');

    realTest('a file that another program changed: keep mine', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await changedByAnother(tester, doc, renamed);
      expect(File(doc.path).readAsStringSync(), contains('"Theirs"'));
      await tap(tester, 'keep-mine', () => line(tester, 'trouble') == null);
      expect(File(doc.path).readAsStringSync(), contains('"text": "mine"'));
      expect(File(doc.path).readAsStringSync(), isNot(contains('"Theirs"')));
      expect(shown(tester, '0-0'), 'mine');
      expect(line(tester, 'state'), 'Saved');
    });

    realTest('a file that another program changed: open theirs', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '0-1', 'Costs');
      await go(tester, 'tool-open', 'Costs');
      await changedByAnother(tester, doc, renamed);
      await tap(tester, 'open-theirs', () => line(tester, 'trouble') == null);
      // The screen shows the other program's document, at the level this
      // device showed last, which that document has too.
      await until(tester, () => shown(tester, '0-0') == '3');
      expect(line(tester, 'title'), 'Costs');
      await go(tester, 'up', 'Example');
      expect(shown(tester, '0-0'), 'Theirs');
      expect(line(tester, 'trouble'), isNull);
      expect(line(tester, 'message'), isNull);
      expect(File(doc.path).readAsStringSync(), isNot(contains('"mine"')));
      // And works on it.
      await edit(tester, '1-0', 'what to buy');
      type(tester, 'on theirs');
      await closed(tester, 'save');
      await tester.pump(const Duration(seconds: 1));
      await until(
        tester,
        () => File(doc.path).readAsStringSync().contains('"on theirs"'),
      );
      expect(File(doc.path).readAsStringSync(), contains('"Theirs"'));
    });

    realTest('open theirs when theirs cannot be read: nothing is given up', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await changedByAnother(tester, doc, (_) => 'half written');
      await tap(tester, 'open-theirs', () => line(tester, 'message') != null);
      expect(
        line(tester, 'message'),
        startsWith(
          'The file as the other program left it cannot be opened, so this '
          'document is still open, and not saved. Error (syntax): ',
        ),
      );
      expect(key('ended'), findsNothing);
      expect(shown(tester, '0-0'), 'mine');
      expect(key('open-theirs'), findsOneWidget);
      await tap(tester, 'keep-mine', () => line(tester, 'trouble') == null);
      expect(File(doc.path).readAsStringSync(), contains('"text": "mine"'));
    });

    realTest('the app out of sight: the text is in the file, and the cross '
        'takes it out again', (tester, keeper, doc, watch) async {
      bool inFile(String text) =>
          File(doc.path).readAsStringSync().contains('"text": "$text"');
      await edit(tester, '1-0', 'what to buy');
      type(tester, 'not this');
      // Only no longer in front: saved, but not what is being typed.
      final saved = watch.next('save');
      await life(tester, [AppLifecycleState.inactive]);
      await real(tester, () => saved);
      expect(inFile('not this'), isFalse);
      // Out of sight: held in the document, and saved.
      await life(tester, [AppLifecycleState.hidden]);
      await until(tester, () => inFile('not this'));
      await life(tester, lifeBack);
      expect(editor(tester), 'not this');
      await tap(tester, 'cancel', () => shown(tester, '1-0') == 'what to buy');
      // The real core has nothing left to undo: the entering was taken back.
      await tap(tester, 'undo', () => line(tester, 'message') != null);
      expect(line(tester, 'message'), 'Nothing to undo');
      await tester.pump(const Duration(seconds: 1));
      await until(tester, () => inFile('what to buy'));
      expect(inFile('not this'), isFalse);
    });

    realTest(
      'one editing with the app out of sight in the middle is one undo',
      (tester, keeper, doc, watch) async {
        await edit(tester, '1-0', 'what to buy');
        type(tester, 'half');
        await life(tester, lifeOut);
        await until(
          tester,
          () => File(doc.path).readAsStringSync().contains('"half"'),
        );
        await life(tester, lifeBack);
        type(tester, 'half and whole');
        await closed(tester, 'save');
        expect(shown(tester, '1-0'), 'half and whole');
        await tap(tester, 'undo', () => shown(tester, '1-0') == 'what to buy');
        await tap(tester, 'undo', () => line(tester, 'message') != null);
        expect(line(tester, 'message'), 'Nothing to undo');
      },
    );

    realTest('a stage that could not be read again, and then can', (
      tester,
      keeper,
      doc,
      watch,
    ) async {
      await edit(tester, '0-0', 'Shopping');
      type(tester, 'typed');
      // The edit is made, by the real core, and reading the stage after it
      // fails once.
      failFirst(
        watch,
        'stage',
        const BridgeError(kind: ErrorKind.io, message: 'not now'),
      );
      await tap(tester, 'save', () => line(tester, 'trouble') != null);
      expect(
        line(tester, 'trouble'),
        'What is shown may be old. Error (io): not now',
      );
      expect(key('editor'), findsNothing);
      expect(shown(tester, '0-0'), 'Shopping');
      await tap(tester, 'read-again', () => line(tester, 'trouble') == null);
      expect(shown(tester, '0-0'), 'typed');
    });

    testWidgets('where the screen was is there again at the next start', (
      tester,
    ) async {
      screenSize(tester, phone);
      final file = File('${folder.path}/again.wombat');
      await real(
        tester,
        () => writeDocument(
          file,
          nestedTable(const [(3, 30), (2, 2), (1, 2)]),
          'Tall',
        ),
      );
      Future<Keeper> start() async {
        final keeper = Keeper(
          await real(tester, () => Session.open(file.path)),
        );
        await tester.pumpWidget(phoneApp(keeper));
        await until(tester, () => there('stage-across'));
        await arrive(tester);
        return keeper;
      }

      var keeper = await start();
      expect(line(tester, 'title'), 'Tall');
      await go(tester, 'child-0-0', 'Level 0 cell 0.0 text');
      await tester.drag(key('stage-down-0'), const Offset(0, -700));
      await tester.pumpAndSettle();
      final top = [
        for (var y = 0; y < 30; y++)
          if (there('cell-0-$y')) y,
      ];
      // Left there, with nothing of the document changed: the app is put away,
      // as a phone does before it ends an app, and goes.
      await real(tester, keeper.putAway);
      await tester.pumpWidget(const SizedBox());
      await real(tester, () => endKeeping(keeper));

      keeper = await start();
      // ignore: avoid_print
      expect(line(tester, 'title'), 'Level 0 cell 0.0 text');
      expect(key('trail-1'), findsOneWidget);
      // The rows that were seen are seen: the first of them at the top, under
      // the first row, which is kept in view.
      expect(top.length, greaterThan(3));
      expect(
        tester.getTopLeft(key('cell-0-${top[1]}')).dy,
        tester.getBottomLeft(key('stage-first-row')).dy,
      );
      await tester.pumpWidget(const SizedBox());
      await real(tester, () => endKeeping(keeper));
    });
  });
}
