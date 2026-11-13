// The phone screen, without a device, on a stand-in for the core that can be made to
// wait and to fail: what is shown, taps in any order, that nothing typed is lost, that
// the document is saved without being asked, and what the screen says when it is not.
// What needs the real core (rows and columns added, styles, the file) is in
// `phone_screen_real_test.dart`.
import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/phone/log.dart';
import 'package:wombat_app/phone/phone_screen.dart';

import 'package:wombat_app/phone/variants.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'fake_document.dart';
import 'phone_support.dart';

/// Gives the cell `c` of the stand-in a grid of 5 by 6 cells, `g0` to `g29`: more
/// than a read gives cell by cell, so that it is drawn as one target.
void bigChild(FakeDocument fake) {
  final ids = [for (var i = 0; i < 30; i++) 'g$i'];
  for (final id in ids) {
    fake.cells[id] = FakeCell(id);
  }
  fake.cells['c']!
    ..xs = 5
    ..cells = ids;
}

/// Makes the grid of 5 by 6 of [bigChild] the root's own; its cell `g7` (column
/// 2, row 1) holds a grid of one cell, `in`.
void wideRoot(FakeDocument fake) {
  bigChild(fake);
  fake.cells['root']!
    ..xs = 5
    ..cells = fake.cells['c']!.cells;
  fake.cells['c']!
    ..xs = 0
    ..cells = [];
  fake.cells['in'] = FakeCell('inside');
  fake.cells['g7']!
    ..xs = 1
    ..cells = ['in'];
}

/// Makes the root of the stand-in one column of `rows` cells, `r0` and so on, of
/// which the one in row `holder` has a grid of one cell `in`.
void tallRoot(FakeDocument fake, int rows, {int holder = 5}) {
  final ids = [for (var i = 0; i < rows; i++) 'r$i'];
  for (final id in ids) {
    fake.cells[id] = FakeCell('row $id');
  }
  fake.cells['in'] = FakeCell('inside');
  fake.cells['r$holder']!
    ..xs = 1
    ..cells = ['in'];
  fake.cells['root']!
    ..xs = 1
    ..cells = ids;
}

void main() {
  int calls(FakeDocument fake, String call) =>
      fake.log.where((name) => name == call).length;

  const broken = BridgeError(kind: ErrorKind.io, message: 'the disk is gone');
  const typedOn =
      'Not done: more was typed in the meantime. It is still in the editor.';

  /// A test with the screen on a stand-in document, kept as the app keeps its own,
  /// at the size of a phone. `before` changes the document before it is opened. The
  /// document is closed at the end, which stops the clock of its saving.
  void screenTest(
    String name,
    Future<void> Function(WidgetTester tester, Keeper keeper, FakeDocument fake)
    body, {
    void Function(FakeDocument fake)? before,
    Future<Session> Function(String path)? reopen,
    Future<void> Function(String path)? canBeRead,
    Variants variants = const Variants(),
    ProtoLog? log,
    Size size = phone,
    double textScale = 1,
  }) {
    testWidgets(name, (tester) async {
      screenSize(tester, size);
      addTearDown(
        () => tester.binding.handleAppLifecycleStateChanged(
          AppLifecycleState.resumed,
        ),
      );
      final fake = FakeDocument();
      before?.call(fake);
      final keeper = Keeper(
        await Session.over(fake),
        reopen: reopen ?? Session.open,
        canBeRead: canBeRead ?? (_) async {},
      );
      await tester.pumpWidget(
        phoneApp(
          keeper,
          variants: ValueNotifier(variants),
          log: log,
          textScale: textScale,
        ),
      );
      // The first read, and then the level drawn as it is arrived at.
      await tester.pump();
      await arrive(tester);
      await body(tester, keeper, fake);
      expect(tester.takeException(), isNull);
      if (keeper.ended == null) await endKeeping(keeper);
    });
  }

  /// Into the grid of the cell `c` of [bigChild], and until the level is drawn.
  Future<void> dive(WidgetTester tester) async {
    await press(tester, 'child-0-1');
    await arrive(tester);
  }

  /// What the keeper is asked from behind the screen, as a second screen could.
  Future<Report> enterBehind(Keeper keeper, String cell, String text) async =>
      (await keeper.edit(cell)).enter(text);

  group('what is shown', () {
    screenTest('the level\'s text, how the document stands, and the cells', (
      tester,
      keeper,
      fake,
    ) async {
      expect(line(tester, 'title'), 'Root');
      expect(line(tester, 'state'), 'Saved');
      expect(
        [
          for (final cell in ['0-0', '1-0', '0-1', '1-1']) shown(tester, cell),
        ],
        ['one', '#NAME?', 'three', 'four'],
      );
      // The colour of the level's cell is beside its text.
      expect(
        tester.widget<Container>(key('stripe')).color,
        const Color(0xFFFFFFFF),
      );
      // The small grid of "three" is there cell by cell.
      expect(key('full-0-1'), findsOneWidget);
      expect(shown(tester, '0-1/0-0'), 'five');
      expect(shown(tester, '0-1/0-1'), '2');
      expect(key('child-0-1'), findsNothing);
      // No Save: the document saves itself.
      expect(find.text('Save'), findsNothing);
    });

    screenTest(
      'a formula cell shows its result, and a failed one as an error',
      (tester, keeper, fake) async {
        final colors = Theme.of(tester.element(key('title'))).colorScheme;
        expect(cellText(tester, '0-0').style!.fontStyle, isNull);
        expect(cellText(tester, '0-0').style!.color, const Color(0xFF000000));
        // A formula that failed: its error value, in italics and the error colour.
        expect(cellText(tester, '1-0').style!.fontStyle, FontStyle.italic);
        expect(cellText(tester, '1-0').style!.color, colors.error);
        // One that worked: its result, in italics, in the cell's own colour.
        expect(cellText(tester, '0-1/0-1').style!.fontStyle, FontStyle.italic);
        expect(
          cellText(tester, '0-1/0-1').style!.color,
          const Color(0xFF000000),
        );
        // The editor has the formula, and so has the cell while it is edited.
        await press(tester, 'cell-0-1/0-1');
        expect(editor(tester), '=1+1');
        expect(shown(tester, '0-1/0-1'), '=1+1');
        expect(cellText(tester, '0-1/0-1').style!.fontStyle, isNull);
        await press(tester, 'cancel');
        expect(shown(tester, '0-1/0-1'), '2');
      },
    );

    screenTest(
      'names where the document has none',
      (tester, keeper, fake) async {
        expect(line(tester, 'title'), 'Wombat');
        expect(find.text('(empty)'), findsOneWidget);
        await dive(tester);
        expect(line(tester, 'title'), 'Wombat');
        expect(find.text('(empty)'), findsOneWidget);
        expect(find.text('› (empty)'), findsOneWidget);
      },
      before: (fake) {
        bigChild(fake);
        fake.cells['root']!.text = '';
        fake.cells['c']!.text = '';
      },
    );

    screenTest('a title that is longer than is read says so', (
      tester,
      keeper,
      fake,
    ) async {
      expect(line(tester, 'title'), '${'long ' * 8}…');
      expect(find.text('${'long ' * 8}…'), findsNWidgets(2));
    }, before: (fake) => fake.cells['root']!.text = 'long ' * 20);

    screenTest('the trail: the level one is at cannot be pressed', (
      tester,
      keeper,
      fake,
    ) async {
      TextButton level(int i) => tester.widget<TextButton>(key('trail-$i'));
      IconButton up() => tester.widget<IconButton>(key('up'));
      expect(level(0).onPressed, isNull);
      expect(up().onPressed, isNull);
      await dive(tester);
      expect(level(0).onPressed, isNotNull);
      expect(level(1).onPressed, isNull);
      expect(up().onPressed, isNotNull);
      expect(find.text('› three'), findsOneWidget);
    }, before: bigChild);

    screenTest(
      'a text written from right to left, and one of several lines',
      (tester, keeper, fake) async {
        expect(cellText(tester, '0-0').textDirection, TextDirection.rtl);
        expect(shown(tester, '0-0'), 'שלום עולם');
        expect(cellText(tester, '1-1').textDirection, TextDirection.ltr);
        // Digits and punctuation decide nothing: the first letter does.
        expect(cellText(tester, '1-0').textDirection, TextDirection.rtl);
        expect(shown(tester, '1-1'), 'first\nsecond\nthird');
        final lines = tester
            .renderObject<RenderBox>(
              find.descendant(
                of: key('cell-1-1'),
                matching: find.byType(RichText),
              ),
            )
            .size
            .height;
        final one = tester
            .renderObject<RenderBox>(
              find.descendant(
                of: key('cell-0-0'),
                matching: find.byType(RichText),
              ),
            )
            .size
            .height;
        expect(lines, one * 3);
        // The editor has the lines, and gives them back as they are.
        await press(tester, 'cell-1-1');
        expect(editor(tester), 'first\nsecond\nthird');
        await press(tester, 'save');
        expect(fake.cells['d']!.text, 'first\nsecond\nthird');
        expect(calls(fake, 'endGroup'), 1);
      },
      before: (fake) {
        fake.cells['a']!.text = 'שלום עולם';
        fake.cells['b']!
          ..text = '12, (مرحبا)'
          ..result = null
          ..failed = false;
        fake.cells['d']!.text = 'first\nsecond\nthird';
      },
    );

    final refused = ProtoLog();
    screenTest(
      'a root of one cell',
      (tester, keeper, fake) async {
        expect(shown(tester, '0-0'), 'one');
        expect(key('cell-1-0'), findsNothing);
        expect(key('add-row'), findsOneWidget);
        expect(key('add-column'), findsOneWidget);
        // The one cell fills the width that the bar leaves.
        expect(tester.getSize(key('cell-0-0')).width, phone.width - 48);
        await press(tester, 'cell-0-0');
        type(tester, 'alone');
        await pressReturn(tester);
        // No row below, and the stand-in cannot add one: said, the text kept.
        expect(fake.cells['a']!.text, 'alone');
        expect(key('editor'), findsNothing);
        expect(
          line(tester, 'message'),
          'Error (refused): the stand-in does not know this command',
        );
        // The same for the bar after the last row; and what was not added is
        // not in the log as added.
        await press(tester, 'add-row');
        expect(
          line(tester, 'message'),
          'Error (refused): the stand-in does not know this command',
        );
        expect(key('editor'), findsNothing);
        expect(refused.text, isNot(contains('add-')));
        expect(refused.lines.length, 2);
      },
      before: (fake) => fake.cells['root']!
        ..xs = 1
        ..cells = ['a'],
      log: refused,
    );

    screenTest(
      'a root without a grid',
      (tester, keeper, fake) async {
        expect(key('empty-root'), findsOneWidget);
        expect(find.text('This document is empty.'), findsOneWidget);
        expect(key('stage-across'), findsNothing);
        expect(line(tester, 'title'), 'Root');
        // Nothing to add a row to, nothing selected.
        expect(tester.widget<TextButton>(key('bar-edit')).onPressed, isNull);
        expect(
          tester.widget<PopupMenuButton<VoidCallback>>(key('bar-add')).enabled,
          isFalse,
        );
        await press(tester, 'undo');
        expect(line(tester, 'message'), 'Nothing to undo');
        expect(await back(tester), ['SystemNavigator.pop']);
      },
      before: (fake) => fake.cells['root']!
        ..xs = 0
        ..cells = [],
    );
  });

  group('the depth budget', () {
    screenTest('a grid of more cells than are given is a Preview: one target', (
      tester,
      keeper,
      fake,
    ) async {
      expect(key('preview-0-1'), findsOneWidget);
      expect(key('full-0-1'), findsNothing);
      // Its first texts, and its size.
      final preview = find.descendant(
        of: key('preview-0-1'),
        matching: find.byType(Text),
      );
      final texts = [
        for (final text in tester.widgetList<Text>(preview)) text.data,
      ];
      expect(texts.take(4), ['g0', 'g1', 'g2', 'g5']);
      expect(texts.last, '5×6');
      // Smaller than a cell's text, and no smaller than the smallest allowed.
      expect(tester.widget<Text>(preview.first).style!.fontSize, 16 * 0.7);
      // At least a target tall.
      expect(tester.getSize(key('child-0-1')).height, greaterThanOrEqualTo(48));
    }, before: bigChild);

    screenTest(
      'where a Preview\'s text would be too small, a Shape',
      (tester, keeper, fake) async {
        expect(key('shape-0-1'), findsOneWidget);
        expect(key('preview-0-1'), findsNothing);
        expect(
          tester.getSize(key('child-0-1')).height,
          greaterThanOrEqualTo(48),
        );
        await dive(tester);
        expect(line(tester, 'title'), 'three');
      },
      before: bigChild,
      variants: const Variants(minText: 12),
    );

    screenTest(
      'a grid that is folded on this device is a Badge with its count',
      (tester, keeper, fake) async {
        expect(key('badge-0-1'), findsOneWidget);
        expect(
          find.descendant(of: key('badge-0-1'), matching: find.text('30')),
          findsOneWidget,
        );
        await dive(tester);
        expect(line(tester, 'title'), 'three');
      },
      before: (fake) {
        bigChild(fake);
        fake.folded.add('c');
      },
    );

    screenTest(
      'larger text of the system: a grid steps down sooner, nothing overflows',
      (tester, keeper, fake) async {
        // Two columns of a Full grid need more room than the cell has now.
        expect(key('full-0-1'), findsNothing);
        expect(key('preview-0-1'), findsOneWidget);
        await press(tester, 'cell-0-0');
        type(tester, 'typed at twice the size');
        await press(tester, 'tool-styles');
        await press(tester, 'tool-open');
        expect(tester.takeException(), isNull);
        await press(tester, 'save');
        expect(fake.cells['a']!.text, 'typed at twice the size');
      },
      before: (fake) => fake.cells['c']!
        ..xs = 2
        ..cells = ['e', 'f'],
      textScale: 2,
    );

    for (final (scale, height) in [(1.0, 48.0), (1.3, 62.4), (2.0, 76.8)]) {
      screenTest(
        'the bars of buttons are taller for larger text, up to a point: $scale',
        (tester, keeper, fake) async {
          double bar(String button) => tester
              .getSize(
                find
                    .ancestor(of: key(button), matching: find.byType(SizedBox))
                    .first,
              )
              .height;
          expect(bar('up'), closeTo(height, 0.01));
          expect(bar('bar-edit'), closeTo(height, 0.01));
        },
        textScale: scale,
      );
    }

    screenTest(
      'the same grid at the usual size of text is Full',
      (tester, keeper, fake) async {
        expect(key('full-0-1'), findsOneWidget);
      },
      before: (fake) => fake.cells['c']!
        ..xs = 2
        ..cells = ['e', 'f'],
    );
  });

  group('what a form is drawn with', () {
    screenTest(
      'a grid two levels below the stage is not given cell by cell: a Preview',
      (tester, keeper, fake) async {
        // "three" is Full; its cell "five" holds a grid that would fit cell by
        // cell, if the read gave it so.
        expect(key('full-0-1'), findsOneWidget);
        expect(key('preview-0-1/0-0'), findsOneWidget);
        expect(key('full-0-1/0-0'), findsNothing);
        expect(key('badge-0-1/0-0'), findsNothing);
        // One target: into the grid, two levels down at once.
        await press(tester, 'child-0-1/0-0');
        await arrive(tester);
        expect(line(tester, 'title'), 'five');
        expect(key('trail-2'), findsOneWidget);
        expect(shown(tester, '0-0'), 'deep');
      },
      before: (fake) {
        fake.cells['h'] = FakeCell('deep');
        fake.cells['e']!
          ..xs = 1
          ..cells = ['h'];
      },
    );

    screenTest(
      'a Preview says where a text was cut; a Badge is a whole target',
      (tester, keeper, fake) async {
        final texts = [
          for (final text in tester.widgetList<Text>(
            find.descendant(
              of: key('preview-0-1'),
              matching: find.byType(Text),
            ),
          ))
            text.data,
        ];
        // Forty characters of it, and the sign that there is more.
        expect(texts.first, '${'long ' * 8}…');
        expect(texts[1], 'g1');
        await choose(tester, 'menu', 'menu-settings');
        await press(tester, 'min-text-more');
        await press(tester, 'settings-close');
        await tester.pumpAndSettle();
        expect(key('shape-0-1'), findsOneWidget);
        expect(
          tester.getSize(key('child-0-1')).height,
          greaterThanOrEqualTo(48),
        );
      },
      before: (fake) {
        bigChild(fake);
        fake.cells['g0']!.text = 'long ' * 20;
      },
    );

    screenTest(
      'a folded grid\'s Badge is a target tall, however little it shows',
      (tester, keeper, fake) async {
        expect(key('badge-0-1'), findsOneWidget);
        expect(
          tester.getSize(key('child-0-1')).height,
          greaterThanOrEqualTo(48),
        );
        expect(tester.getSize(key('badge-0-1')).height, lessThan(40));
      },
      before: (fake) {
        bigChild(fake);
        fake.folded.add('c');
      },
    );

    screenTest(
      'a long text is cut at eight lines, until it is edited',
      (tester, keeper, fake) async {
        expect(cellText(tester, '0-0').maxLines, 8);
        expect(cellText(tester, '0-0').overflow, TextOverflow.ellipsis);
        final cut = tester.getSize(key('cell-0-0')).height;
        await press(tester, 'cell-0-0');
        expect(cellText(tester, '0-0').maxLines, isNull);
        expect(tester.getSize(key('cell-0-0')).height, greaterThan(cut));
        await press(tester, 'cancel');
        expect(tester.getSize(key('cell-0-0')).height, cut);
      },
      before: (fake) =>
          fake.cells['a']!.text = [for (var i = 0; i < 20; i++) 'line $i']
              .join('\n'),
    );
  });

  group('sizes', () {
    for (final (name, size, scale) in [
      ('a small phone', const Size(320, 480), 1.0),
      ('a small phone with text twice the size', const Size(320, 480), 2.0),
      ('a phone on its side', const Size(800, 400), 1.0),
      ('a large window', const Size(1400, 900), 1.0),
      ('a large window with text twice the size', const Size(1400, 900), 2.0),
    ]) {
      screenTest(
        '$name: everything is there, and nothing overflows',
        (tester, keeper, fake) async {
          expect(shown(tester, '0-0'), 'one');
          await press(tester, 'cell-0-0');
          type(tester, 'typed\nand more');
          await press(tester, 'tool-styles');
          for (final tool in ['tool-break', 'tool-right']) {
            await tester.ensureVisible(key(tool));
          }
          await tester.pump();
          expect(tester.takeException(), isNull);
          await press(tester, 'tool-right');
          expect(fake.cells['a']!.text, 'typed\nand more');
          expect(editor(tester), '=X');
          await press(tester, 'cancel');
          // A line that stays and one that passes, with their buttons.
          fake.fileChanged = true;
          await keeper.save();
          await tester.pump();
          expect(key('keep-mine'), findsOneWidget);
          await press(tester, 'redo');
          expect(line(tester, 'message'), 'Nothing to redo');
          await choose(tester, 'menu', 'menu-settings');
          expect(key('variant-tap-text'), findsOneWidget);
          expect(tester.takeException(), isNull);
        },
        before: bigChild,
        size: size,
        textScale: scale,
      );
    }

    screenTest(
      'a cell edited in the first row, which is kept in view and scrolls in '
      'what it is given, is scrolled into view there: on a phone, also above '
      'the keyboard',
      (tester, keeper, fake) async {
        // The screen of the emulator of CI: 1080 by 2400 at 420 dpi.
        tester.view.devicePixelRatio = 2.625;
        tester.view.physicalSize = const Size(1080, 2400);
        tester.view.padding = const FakeViewPadding(top: 63);
        await tester.pumpAndSettle();
        void seen(int row) {
          final cell = tester.getRect(key('cell-0-0/0-$row'));
          final first = tester.getRect(key('stage-first-row'));
          expect(cell.top, greaterThanOrEqualTo(first.top - 0.5));
          expect(cell.bottom, lessThanOrEqualTo(first.bottom + 0.5));
          expect(first.bottom, lessThan(tester.getRect(key('editor')).top));
        }

        // Without a keyboard the first row is too tall already for its share.
        await press(tester, 'cell-0-0/0-1');
        await tester.pumpAndSettle();
        seen(1);
        // The keyboard comes, and takes a third of the screen.
        tester.view.viewInsets = const FakeViewPadding(bottom: 800);
        await tester.pumpAndSettle();
        seen(1);
        // Return goes down the grid, each cell seen while it is typed in.
        for (var row = 2; row < 5; row++) {
          type(tester, 'typed in row ${row - 1}');
          await pressReturn(tester);
          await tester.pumpAndSettle();
          expect(editor(tester), 'item s$row');
          seen(row);
        }
        // And back up by a tap on what is seen of the cell's own text.
        await press(tester, 'save');
        await tester.pumpAndSettle();
        await reveal(tester, 'cell-0-0');
        await press(tester, 'cell-0-0');
        await tester.pumpAndSettle();
        expect(editor(tester), 'one');
        final cell = tester.getRect(key('cell-0-0'));
        final first = tester.getRect(key('stage-first-row'));
        expect(cell.top, greaterThanOrEqualTo(first.top - 0.5));
        expect(cell.bottom, lessThanOrEqualTo(first.bottom + 0.5));
        // The keyboard goes: the cell is still seen.
        await press(tester, 'cancel');
        await reveal(tester, 'cell-0-0/0-4');
        await press(tester, 'cell-0-0/0-4');
        await tester.pumpAndSettle();
        expect(editor(tester), 'item s4');
        tester.view.viewInsets = FakeViewPadding.zero;
        await tester.pumpAndSettle();
        seen(4);
        expect(fake.cells['s1']!.text, 'typed in row 1');
        expect(fake.cells['s3']!.text, 'typed in row 3');
        await press(tester, 'cancel');
      },
      before: (fake) {
        final ids = [for (var i = 0; i < 5; i++) 's$i'];
        for (final id in ids) {
          fake.cells[id] = FakeCell('item $id');
        }
        fake.cells['a']!
          ..xs = 1
          ..cells = ids;
      },
    );

    screenTest(
      'turned while a cell is edited: the text stays, the cell is seen',
      (tester, keeper, fake) async {
        await tester.drag(key('stage-down-0'), const Offset(0, -2000));
        await tester.pumpAndSettle();
        await press(tester, 'cell-0-29');
        type(tester, 'half a sentence');
        await tester.pump();
        for (final size in [
          const Size(800, 400),
          phone,
          const Size(900, 360),
        ]) {
          tester.view.physicalSize = size;
          await tester.pumpAndSettle();
          expect(tester.takeException(), isNull);
          expect(editor(tester), 'half a sentence');
          expect(focused(tester), isTrue);
          // The edited cell is above the editor, inside what is seen of the grid.
          final cell = tester.getRect(key('cell-0-29'));
          final grid = tester.getRect(key('stage-across'));
          expect(cell.top, greaterThanOrEqualTo(grid.top - 0.5));
          expect(cell.bottom, lessThanOrEqualTo(grid.bottom + 0.5));
        }
        expect(fake.log.contains('retype'), isFalse);
        await press(tester, 'save');
        expect(fake.cells['r29']!.text, 'half a sentence');
      },
      before: (fake) => tallRoot(fake, 30),
    );
  });

  group('tapping', () {
    screenTest(
      'a cell\'s text: the editor opens on it, the cursor at the end',
      (tester, keeper, fake) async {
        expect(key('editor'), findsNothing);
        expect(key('bar-edit'), findsOneWidget);
        await press(tester, 'cell-0-0');
        expect(editor(tester), 'one');
        expect(
          editorField(tester).selection,
          const TextSelection.collapsed(offset: 3),
        );
        expect(focused(tester), isTrue);
        // The toolbar is there in place of the bottom bar.
        expect(key('bar-edit'), findsNothing);
        for (final tool in [
          'tool-styles',
          'tool-break',
          'tool-right',
          'tool-row',
          'tool-col',
          'tool-open',
          'save',
          'cancel',
        ]) {
          expect(key(tool), findsOneWidget, reason: tool);
        }
        // Nothing of the toolbar can take the keyboard from the field.
        for (final tool in ['tool-right', 'save', 'cancel']) {
          expect(
            tester
                .widget<ExcludeFocus>(
                  find
                      .ancestor(
                        of: key(tool),
                        matching: find.byType(ExcludeFocus),
                      )
                      .first,
                )
                .excluding,
            isTrue,
            reason: tool,
          );
        }
        // One read of the cell, and nothing sent.
        expect((calls(fake, 'cell'), calls(fake, 'beginGroup')), (1, 0));
        await press(tester, 'cancel');
        expect(key('editor'), findsNothing);
        expect(fake.log.contains('beginGroup'), isFalse);
      },
    );

    screenTest('a cell of a Full grid: the editor opens on that cell', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-1/0-0');
      expect(editor(tester), 'five');
      type(tester, '5');
      await press(tester, 'save');
      expect(fake.cells['e']!.text, '5');
      expect(shown(tester, '0-1/0-0'), '5');
      expect(line(tester, 'title'), 'Root');
    });

    screenTest(
      'a Preview: into the grid; the arrow, the trail and back lead out',
      (tester, keeper, fake) async {
        await dive(tester);
        expect(line(tester, 'title'), 'three');
        expect(shown(tester, '0-0'), 'g0');
        expect(shown(tester, '4-5'), 'g29');
        expect(fake.zoom, 'c');
        await press(tester, 'up');
        await arrive(tester);
        expect(line(tester, 'title'), 'Root');
        expect(fake.zoom, 'root');
        await dive(tester);
        await press(tester, 'trail-0');
        await arrive(tester);
        expect(line(tester, 'title'), 'Root');
        // The system's back goes up one level, and leaves the app only from the
        // root.
        await dive(tester);
        expect(await back(tester), isEmpty);
        expect(line(tester, 'title'), 'Root');
        expect(await back(tester), ['SystemNavigator.pop']);
        expect(line(tester, 'title'), 'Root');
      },
      before: bigChild,
    );

    screenTest('while the editor is open, a tap outside it only closes it', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'typed');
      // On a grid: no dive.
      await press(tester, 'child-0-1');
      expect(key('editor'), findsNothing);
      expect(fake.cells['a']!.text, 'typed');
      expect(line(tester, 'title'), 'Root');
      // On another cell: no second editor.
      await press(tester, 'cell-0-0');
      type(tester, 'again');
      await press(tester, 'cell-1-1');
      expect(key('editor'), findsNothing);
      expect(fake.cells['a']!.text, 'again');
      // On the cell itself.
      await press(tester, 'cell-0-0');
      type(tester, 'once more');
      await press(tester, 'cell-0-0');
      expect(key('editor'), findsNothing);
      expect(fake.cells['a']!.text, 'once more');
      // And the system's back: the editor, not the level.
      await dive(tester);
      await press(tester, 'cell-0-0');
      type(tester, 'inside');
      expect(await back(tester), isEmpty);
      expect(key('editor'), findsNothing);
      expect(fake.cells['g0']!.text, 'inside');
      expect(line(tester, 'title'), 'three');
      // Text that the core refuses: the editor stays, with what was typed.
      await press(tester, 'cell-0-0');
      type(tester, 'too much');
      failFirst(fake, 'retype', broken);
      expect(await back(tester), isEmpty);
      expect(editor(tester), 'too much');
      expect(line(tester, 'message'), 'Error (io): the disk is gone');
      await press(tester, 'cancel');
      expect(fake.cells['g0']!.text, 'inside');
    }, before: bigChild);

    screenTest(
      'the other variant of "tap text": select first, tap again to edit',
      (tester, keeper, fake) async {
        Color? mark(String cell) {
          final box = tester.widget<DecoratedBox>(
            find
                .ancestor(
                  of: key('cell-$cell'),
                  matching: find.byWidgetPredicate(
                    (w) =>
                        w is DecoratedBox &&
                        w.position == DecorationPosition.foreground,
                  ),
                )
                .first,
          );
          final border = (box.decoration as BoxDecoration).border! as Border;
          return border.top.width == 2 ? border.top.color : null;
        }

        final colors = Theme.of(tester.element(key('title'))).colorScheme;
        expect(mark('0-0'), isNull);
        await press(tester, 'cell-0-0');
        expect(key('editor'), findsNothing);
        expect(mark('0-0'), colors.tertiary);
        expect(fake.log.contains('cell'), isFalse);
        // Another cell: the selection moves, and still no editor.
        await press(tester, 'cell-1-1');
        expect(key('editor'), findsNothing);
        expect((mark('0-0'), mark('1-1')), (null, colors.tertiary));
        // The selected one again: the editor.
        await press(tester, 'cell-1-1');
        expect(editor(tester), 'four');
        expect(mark('1-1'), colors.primary);
        await press(tester, 'save');
        // "Edit" of the bottom bar edits the selected cell at once.
        await press(tester, 'cell-0-0');
        await press(tester, 'bar-edit');
        expect(editor(tester), 'one');
        await press(tester, 'cancel');
      },
      variants: const Variants(tapTextSelects: true),
    );

    screenTest('the bottom bar acts on the selected cell', (
      tester,
      keeper,
      fake,
    ) async {
      TextButton bar(String name) =>
          tester.widget<TextButton>(key('bar-$name'));
      expect((bar('edit').onPressed, bar('open').onPressed), (null, null));
      // A cell that was edited stays selected.
      await press(tester, 'cell-0-0');
      await press(tester, 'cancel');
      expect(bar('edit').onPressed, isNotNull);
      expect(bar('open').onPressed, isNull);
      await press(tester, 'bar-edit');
      expect(editor(tester), 'one');
      await press(tester, 'cancel');
      // One with a grid can be gone into, also where the grid is Full.
      await press(tester, 'cell-0-1');
      await press(tester, 'cancel');
      await press(tester, 'bar-open');
      await arrive(tester);
      expect(line(tester, 'title'), 'three');
      expect((shown(tester, '0-0'), shown(tester, '0-1')), ('five', '2'));
    });

    screenTest(
      'the other variant of "tap child grid": unfold in place',
      (tester, keeper, fake) async {
        expect(key('preview-0-1'), findsOneWidget);
        final below = tester.getTopLeft(key('add-row')).dy;
        await press(tester, 'child-0-1');
        // No dive: the grid is there cell by cell, and what is below moved down.
        expect(line(tester, 'title'), 'Root');
        expect(key('preview-0-1'), findsNothing);
        expect(key('full-0-1'), findsOneWidget);
        expect(shown(tester, '0-1/4-5'), 'g29');
        expect(tester.getTopLeft(key('add-row')).dy, greaterThan(below));
        // Every one of its cells can be tapped: at least a target wide and tall.
        final cell = tester.getSize(key('cell-0-1/0-0'));
        expect(cell.width, greaterThanOrEqualTo(48));
        expect(cell.height, greaterThanOrEqualTo(48));
        await press(tester, 'cell-0-1/0-0');
        expect(editor(tester), 'g0');
        type(tester, 'typed inside');
        await press(tester, 'save');
        expect(fake.cells['g0']!.text, 'typed inside');
        expect(key('full-0-1'), findsOneWidget);
        // With the editor open, a tap on its bar only closes the editor.
        await press(tester, 'cell-0-0');
        type(tester, 'kept');
        await press(tester, 'fold-0-1');
        expect(key('editor'), findsNothing);
        expect(fake.cells['a']!.text, 'kept');
        expect(key('full-0-1'), findsOneWidget);
        // Folded again by its bar; the read is asked for less again.
        final reads = calls(fake, 'stage');
        await press(tester, 'fold-0-1');
        expect(key('full-0-1'), findsNothing);
        expect(key('preview-0-1'), findsOneWidget);
        expect(calls(fake, 'stage'), reads + 1);
        expect(tester.getTopLeft(key('add-row')).dy, below);
        // Unfolded, then "Open ›": the dive is still there, and the way back
        // finds the grid folded.
        await press(tester, 'child-0-1');
        await press(tester, 'cell-0-1');
        final before = calls(fake, 'stage');
        await press(tester, 'tool-open');
        // Two reads: the stage asked for no more than usual again, and the
        // level gone to.
        expect(calls(fake, 'stage'), before + 2);
        await arrive(tester);
        expect(line(tester, 'title'), 'three');
        expect(shown(tester, '0-0'), 'typed inside');
        await press(tester, 'up');
        await arrive(tester);
        expect(key('preview-0-1'), findsOneWidget);
      },
      before: bigChild,
      variants: const Variants(tapChildUnfolds: true),
    );

    screenTest(
      'a grid inside a Full grid is gone into, also where grids unfold',
      (tester, keeper, fake) async {
        expect(key('preview-0-1/0-0'), findsOneWidget);
        final reads = calls(fake, 'stage');
        await press(tester, 'child-0-1/0-0');
        await arrive(tester);
        expect(line(tester, 'title'), 'five');
        expect(calls(fake, 'stage'), reads + 1);
      },
      before: (fake) {
        fake.cells['h'] = FakeCell('deep');
        fake.cells['e']!
          ..xs = 1
          ..cells = ['h'];
      },
      variants: const Variants(tapChildUnfolds: true),
    );

    screenTest(
      'what was unfolded is folded again when the level is left',
      (tester, keeper, fake) async {
        // A grid of three columns: given cell by cell, and too wide to be Full.
        expect(key('preview-0-1'), findsOneWidget);
        await press(tester, 'child-0-1');
        expect(key('full-0-1'), findsOneWidget);
        expect(key('fold-0-1'), findsOneWidget);
        // Nothing more was asked of the read for it.
        expect(calls(fake, 'stage'), 1);
        await press(tester, 'cell-0-1');
        await press(tester, 'tool-open');
        await arrive(tester);
        expect(line(tester, 'title'), 'three');
        await press(tester, 'up');
        await arrive(tester);
        expect(key('preview-0-1'), findsOneWidget);
        expect(key('full-0-1'), findsNothing);
      },
      before: (fake) {
        for (final id in ['g0', 'g1', 'g2', 'g3']) {
          fake.cells[id] = FakeCell(id);
        }
        fake.cells['c']!
          ..xs = 3
          ..cells = ['e', 'f', 'g0', 'g1', 'g2', 'g3'];
      },
      variants: const Variants(tapChildUnfolds: true),
    );

    screenTest(
      'the first row and the first column stay in view, unless switched off',
      (tester, keeper, fake) async {
        final corner = tester.getTopLeft(key('cell-0-0'));
        final first = tester.getTopLeft(key('cell-0-1'));
        await tester.drag(key('stage-across'), const Offset(-250, 0));
        await tester.pumpAndSettle();
        await tester.drag(key('stage-down-0'), const Offset(0, -120));
        await tester.pumpAndSettle();
        // The corner has not moved; the first row went sideways with the rest
        // and stayed at the top; the first column went up and stayed at the left.
        expect(tester.getTopLeft(key('cell-0-0')), corner);
        expect(tester.getTopLeft(key('cell-3-0')).dy, corner.dy);
        expect(tester.getTopLeft(key('cell-3-0')).dx, lessThan(250));
        expect(tester.getTopLeft(key('cell-0-4')).dx, first.dx);
        expect(tester.getTopLeft(key('cell-0-4')).dy, lessThan(first.dy + 150));
        // A tap where the first column lies over another finds the first column.
        await tester.tapAt(tester.getCenter(key('cell-0-4')));
        await tester.pump();
        expect(editor(tester), 'g20');
        await press(tester, 'cancel');
        // Switched off, everything scrolls.
        await choose(tester, 'menu', 'menu-settings');
        await tester.tap(key('variant-pin'));
        await tester.pumpAndSettle();
        await press(tester, 'settings-close');
        await tester.pumpAndSettle();
        expect(tester.getTopLeft(key('cell-0-4')).dx, lessThan(first.dx));
        expect(key('stage-first-row'), findsNothing);
      },
      before: (fake) {
        bigChild(fake);
        // The grid of 5 by 6 is the root's own.
        fake.cells['root']!
          ..xs = 5
          ..cells = fake.cells['c']!.cells;
        fake.cells['c']!
          ..xs = 0
          ..cells = [];
      },
    );
  });

  group('the edited cell is kept in sight', () {
    /// Whether the cell is inside what is seen of the grid, top to bottom.
    void inSight(WidgetTester tester, String cell) {
      final at = tester.getRect(key('cell-$cell'));
      final grid = tester.getRect(key('stage-across'));
      expect(at.top, greaterThanOrEqualTo(grid.top - 0.5), reason: 'top');
      expect(at.bottom, lessThanOrEqualTo(grid.bottom + 0.5), reason: 'bottom');
    }

    screenTest('a cell at the bottom, which the editor would cover', (
      tester,
      keeper,
      fake,
    ) async {
      // The last row that is seen whole.
      final grid = tester.getRect(key('stage-across'));
      var last = 1;
      while (tester.getRect(key('cell-0-${last + 1}')).bottom <= grid.bottom) {
        last++;
      }
      await tester.tap(key('cell-0-$last'));
      await tester.pump();
      await tester.pump();
      // The editor is taller than the bar it took the place of.
      expect(tester.getRect(key('stage-across')).bottom, lessThan(grid.bottom));
      inSight(tester, '0-$last');
      expect(editor(tester), 'row r$last');
      await press(tester, 'cancel');
    }, before: (fake) => tallRoot(fake, 40));

    screenTest('a cell half under the top edge', (tester, keeper, fake) async {
      // (A drag begins after the first 18 pixels of it.)
      await tester.drag(key('stage-down-0'), const Offset(0, -42));
      await tester.pumpAndSettle();
      final top = tester.getRect(key('stage-first-row')).bottom;
      expect(tester.getRect(key('cell-0-1')).top, lessThan(top - 20));
      // Its lower half is there to be tapped.
      await tester.tapAt(Offset(60, top + 12));
      await tester.pump();
      await tester.pump();
      expect(editor(tester), 'row r1');
      expect(tester.getRect(key('cell-0-1')).top, top);
      await press(tester, 'cancel');
    }, before: (fake) => tallRoot(fake, 40));

    screenTest(
      'a cell of a Full grid, whose own place in that grid says nothing',
      (tester, keeper, fake) async {
        // The cell inside row 11 is seen whole, and low enough for the editor
        // to cover it.
        final grid = tester.getRect(key('stage-across'));
        expect(
          tester.getRect(key('cell-0-11/0-0')).bottom,
          lessThan(grid.bottom),
        );
        await tester.tap(key('cell-0-11/0-0'));
        await tester.pump();
        await tester.pump();
        expect(editor(tester), 'inside');
        final now = tester.getRect(key('stage-across'));
        expect(now.bottom, lessThan(grid.bottom));
        expect(
          tester.getRect(key('cell-0-11/0-0')).bottom,
          lessThanOrEqualTo(now.bottom + 0.5),
        );
        await press(tester, 'cancel');
      },
      before: (fake) => tallRoot(fake, 40, holder: 11),
    );

    screenTest(
      'sideways: a cell past the right edge, and one under the first column',
      (tester, keeper, fake) async {
        const width = 400.0;
        // → from the third column to the fourth, which is half out of sight.
        await press(tester, 'cell-2-2');
        expect(tester.getRect(key('cell-3-2')).right, greaterThan(width));
        await press(tester, 'tool-right');
        await tester.pump();
        await tester.pump();
        expect(editor(tester), 'g13');
        expect(tester.getRect(key('cell-3-2')).right, width);
        await press(tester, 'cancel');
        // A cell of the first column, which stays in view: nothing moves.
        final before = tester.getTopLeft(key('cell-4-2')).dx;
        await tester.tapAt(tester.getCenter(key('cell-0-2')));
        await tester.pump();
        await tester.pump();
        expect(editor(tester), 'g10');
        expect(tester.getTopLeft(key('cell-4-2')).dx, before);
        await press(tester, 'cancel');
        // "Edit" on a selected cell that lies under the first column: it comes
        // out to the right of it.
        await press(tester, 'cell-1-2');
        await press(tester, 'cancel');
        await tester.drag(key('stage-across'), const Offset(-300, 0));
        await tester.pumpAndSettle();
        final first = tester.getRect(key('cell-0-2'));
        expect(first.left, 0);
        expect(
          tester.getRect(key('cell-1-2')).left,
          lessThan(first.right - 50),
        );
        await press(tester, 'bar-edit');
        await tester.pump();
        await tester.pump();
        expect(editor(tester), 'g11');
        expect(tester.getRect(key('cell-1-2')).left, first.right);
        await press(tester, 'cancel');
      },
      before: wideRoot,
    );
  });

  group('three levels', () {
    void deep(FakeDocument fake) {
      bigChild(fake);
      fake.cells['h'] = FakeCell('deepest');
      fake.cells['g0']!
        ..xs = 1
        ..cells = ['h'];
    }

    Future<void> down(WidgetTester tester) async {
      await press(tester, 'child-0-1');
      await arrive(tester);
      await press(tester, 'cell-0-0');
      await press(tester, 'tool-open');
      await arrive(tester);
    }

    /// Whether the cell with this name is the one that flashes.
    bool flashes(String cell) => find
        .descendant(of: key('flash'), matching: key('cell-$cell'))
        .evaluate()
        .isNotEmpty;

    screenTest('up goes one level, a name of the trail to its level', (
      tester,
      keeper,
      fake,
    ) async {
      await down(tester);
      expect(line(tester, 'title'), 'g0');
      expect(shown(tester, '0-0'), 'deepest');
      await press(tester, 'up');
      await tester.pump();
      expect(line(tester, 'title'), 'three');
      // The cell come up out of flashes, and no other.
      expect(flashes('0-0'), isTrue);
      expect(find.byKey(const Key('flash')), findsOneWidget);
      await arrive(tester);
      await press(tester, 'cell-0-0');
      await press(tester, 'tool-open');
      await arrive(tester);
      await press(tester, 'trail-1');
      await arrive(tester);
      expect(line(tester, 'title'), 'three');
      await press(tester, 'cell-0-0');
      await press(tester, 'tool-open');
      await arrive(tester);
      // Two levels at once: the cell of the level arrived at that leads to
      // where the screen was, not the cell it was in.
      await press(tester, 'trail-0');
      await tester.pump();
      expect(line(tester, 'title'), 'Root');
      expect(flashes('0-1'), isTrue);
      expect(find.byKey(const Key('flash')), findsOneWidget);
    }, before: deep);

    screenTest('two taps on the arrow one level down go up once', (
      tester,
      keeper,
      fake,
    ) async {
      await dive(tester);
      final went = calls(fake, 'viewGo');
      await tester.tap(key('up'));
      await tester.tap(key('up'), warnIfMissed: false);
      await tester.pump();
      await tester.pump();
      expect(line(tester, 'title'), 'Root');
      expect(calls(fake, 'viewGo'), went + 1);
      // The same for the name of the level in the trail.
      await arrive(tester);
      await dive(tester);
      await tester.tap(key('trail-0'));
      await tester.tap(key('trail-0'), warnIfMissed: false);
      await tester.pump();
      await tester.pump();
      expect(line(tester, 'title'), 'Root');
      expect(calls(fake, 'viewGo'), went + 3);
    }, before: bigChild);
  });

  group('how a level arrives', () {
    double opacity(WidgetTester tester) => tester
        .widget<Opacity>(
          find
              .ancestor(of: key('stage-across'), matching: find.byType(Opacity))
              .first,
        )
        .opacity;

    double scale(WidgetTester tester) => tester
        .widget<Transform>(
          find
              .ancestor(
                of: key('stage-across'),
                matching: find.byType(Transform),
              )
              .first,
        )
        .transform
        .storage[0];

    screenTest(
      'growing into place after a dive, shrinking into place after a way up',
      (tester, keeper, fake) async {
        await press(tester, 'child-0-1');
        expect(opacity(tester), closeTo(0.3, 0.01));
        expect(scale(tester), closeTo(0.88, 0.001));
        await tester.pump(const Duration(milliseconds: 150));
        expect(scale(tester), inExclusiveRange(0.88, 1));
        await tester.pump(const Duration(milliseconds: 150));
        expect((opacity(tester), scale(tester)), (1, 1));
        await press(tester, 'up');
        await tester.pump();
        expect(scale(tester), closeTo(1.12, 0.001));
        await tester.pump(const Duration(milliseconds: 300));
        expect((opacity(tester), scale(tester)), (1, 1));
      },
      before: bigChild,
    );

    testWidgets('at once where the system asks for no animations', (
      tester,
    ) async {
      screenSize(tester, phone);
      final fake = FakeDocument();
      bigChild(fake);
      final keeper = Keeper(await Session.over(fake));
      await tester.pumpWidget(phoneApp(keeper, disableAnimations: true));
      await tester.pump();
      await tester.pump();
      await press(tester, 'child-0-1');
      await tester.pump();
      expect(line(tester, 'title'), 'three');
      expect((opacity(tester), scale(tester)), (1, 1));
      await endKeeping(keeper);
    });
  });

  group('each level remembers where it was', () {
    screenTest(
      'the place and the selected cell, there again after a dive and back',
      (tester, keeper, fake) async {
        await tester.drag(key('stage-down-0'), const Offset(0, -170));
        await tester.pumpAndSettle();
        // Row 4 is at the top now, under the first row, which is kept in view.
        final top = tester.getTopLeft(key('cell-0-4')).dy;
        final first = tester.getBottomLeft(key('cell-0-0')).dy;
        expect(top, lessThan(first));
        expect(tester.getBottomLeft(key('cell-0-4')).dy, greaterThan(first));
        await tester.tap(key('cell-0-5'));
        await tester.pump();
        await press(tester, 'tool-open');
        await arrive(tester);
        expect(line(tester, 'title'), 'row r5');
        expect((fake.levels['root']!.x, fake.levels['root']!.y), (0, 4));
        expect(fake.levels['root']!.selected, 'r5');
        await press(tester, 'up');
        await arrive(tester);
        // Where it was left: row 4 at the top again, from its first line (the
        // level remembers the row, not how far into it), and the cell it came
        // out of selected.
        expect(tester.getTopLeft(key('cell-0-4')).dy, first);
        expect(key('cell-0-3'), findsNothing);
        expect(tester.widget<TextButton>(key('bar-open')).onPressed, isNotNull);
        // The rows above are there to scroll back to.
        await tester.drag(key('stage-down-0'), const Offset(0, 170));
        await tester.pumpAndSettle();
        expect(key('cell-0-1'), findsOneWidget);
      },
      before: (fake) => tallRoot(fake, 40),
    );

    screenTest('the cell the screen came up out of flashes once', (
      tester,
      keeper,
      fake,
    ) async {
      await dive(tester);
      await press(tester, 'up');
      await tester.pump();
      expect(
        find.descendant(
          of: find.ancestor(
            of: key('cell-0-1'),
            matching: find.byKey(const Key('flash')),
          ),
          matching: key('cell-0-1'),
        ),
        findsOneWidget,
      );
      expect(key('flash'), findsOneWidget);
      // The next thing done ends it.
      await arrive(tester);
      await press(tester, 'cell-0-0');
      await press(tester, 'cancel');
      await dive(tester);
      expect(key('flash'), findsNothing);
    }, before: bigChild);

    screenTest('the column too', (tester, keeper, fake) async {
      await tester.drag(key('stage-across'), const Offset(-150, 0));
      await tester.pumpAndSettle();
      // The second column is the first of which something is seen beside the
      // first, which stays in view. Into the grid of the third.
      await tester.tapAt(
        tester.getCenter(key('cell-2-1')) + const Offset(20, 0),
      );
      await tester.pump();
      await tester.pump();
      expect(editor(tester), 'g7');
      await press(tester, 'tool-open');
      await arrive(tester);
      expect(line(tester, 'title'), 'g7');
      expect((fake.levels['root']!.x, fake.levels['root']!.y), (1, 1));
      await press(tester, 'up');
      await arrive(tester);
      // The second column at the left edge again, under the first.
      final first = tester.getRect(key('cell-0-0'));
      expect(tester.getRect(key('cell-1-0')).left, first.left);
      expect(tester.getRect(key('cell-2-0')).left, first.right);
    }, before: wideRoot);

    screenTest(
      'a row that is out of sight, and still there for a tap\'s ripple, is not '
      'taken for the top one',
      (tester, keeper, fake) async {
        await tester.tap(key('cell-0-3'));
        await tester.pump();
        await tester.drag(key('stage-down-0'), const Offset(0, -600));
        // The frame in which the rows are where the drag left them, and the
        // place is read; the ripple of the tap has not run out.
        await tester.pump();
        await tester.pump();
        await keeper.save();
        final top = tester.getRect(key('stage-first-row')).bottom;
        var first = 1;
        while (!there('cell-0-$first') ||
            tester.getRect(key('cell-0-$first')).bottom <= top + 1) {
          first++;
        }
        expect(first, greaterThan(10));
        expect(fake.levels['root']!.y, first);
        expect(fake.levels['root']!.selected, 'r3');
      },
      before: (fake) => tallRoot(fake, 40),
      variants: const Variants(tapTextSelects: true),
    );

    for (final selects in [false, true]) {
      screenTest('the selected cell is told with the next save, '
          '${selects ? 'selected by a tap' : 'selected by being edited'}', (
        tester,
        keeper,
        fake,
      ) async {
        await enterBehind(keeper, 'a', 'changed');
        await tester.pump();
        await press(tester, 'cell-1-1');
        if (!selects) await press(tester, 'cancel');
        // No level was left: the save hands it on.
        await keeper.save();
        expect(fake.levels['root']!.selected, 'd');
        expect((fake.viewSaves, fake.viewSaved), (1, true));
      }, variants: Variants(tapTextSelects: selects));
    }

    screenTest('a screen that starts shows the level the device showed last', (
      tester,
      keeper,
      fake,
    ) async {
      expect(line(tester, 'title'), 'three');
      expect(shown(tester, '0-0'), 'five');
      expect(find.text('› three'), findsOneWidget);
    }, before: (fake) => fake.zoom = 'c');
  });

  group('editing', () {
    screenTest(
      'the edited cell is drawn from the editor, and nothing is sent',
      (tester, keeper, fake) async {
        await press(tester, 'cell-0-0');
        final calls = fake.log.length;
        type(tester, 'h');
        await tester.pump();
        expect(shown(tester, '0-0'), 'h');
        type(tester, 'half a sentence');
        await tester.pump();
        expect(shown(tester, '0-0'), 'half a sentence');
        type(tester, '');
        await tester.pump();
        expect(shown(tester, '0-0'), '');
        // Not one call to the core for all of that.
        expect(fake.log.length, calls);
        expect(fake.cells['a']!.text, 'one');
        expect(line(tester, 'state'), 'Saved');
        // The cross: the cell shows what the document has.
        await press(tester, 'cancel');
        expect(shown(tester, '0-0'), 'one');
        expect(fake.log.length, calls);
      },
    );

    screenTest(
      'the tick enters the text as one step; the cross enters nothing',
      (tester, keeper, fake) async {
        await press(tester, 'cell-0-0');
        type(tester, 'typed');
        await press(tester, 'save');
        expect(key('editor'), findsNothing);
        expect(
          (fake.cells['a']!.text, shown(tester, '0-0')),
          ('typed', 'typed'),
        );
        expect(
          fake.log,
          containsAllInOrder(['beginGroup', 'retype', 'apply', 'endGroup']),
        );
        expect(line(tester, 'state'), 'Not saved yet');
        await press(tester, 'cell-0-0');
        type(tester, 'not this');
        await press(tester, 'cancel');
        expect(
          (fake.cells['a']!.text, shown(tester, '0-0')),
          ('typed', 'typed'),
        );
        await press(tester, 'undo');
        expect(shown(tester, '0-0'), 'one');
        await press(tester, 'undo');
        expect(line(tester, 'message'), 'Nothing to undo');
        await press(tester, 'redo');
        expect(shown(tester, '0-0'), 'typed');
        await press(tester, 'redo');
        expect(line(tester, 'message'), 'Nothing to redo');
      },
    );

    screenTest('a message goes when it is tapped, and with the next action', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'redo');
      expect(line(tester, 'message'), 'Nothing to redo');
      await tester.tap(key('message'));
      await tester.pump();
      expect(line(tester, 'message'), isNull);
      await press(tester, 'undo');
      expect(line(tester, 'message'), 'Nothing to undo');
      // Gone at the tap, not when the action's turn comes.
      final gate = holdFirst(fake, 'cell');
      await press(tester, 'cell-0-0');
      expect(line(tester, 'message'), isNull);
      expect(key('editor'), findsNothing);
      gate.complete();
      await tester.pump();
      expect(editor(tester), 'one');
      await press(tester, 'cancel');
    });

    screenTest('Return saves and moves to the cell below; → to the right', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, '1');
      await pressReturn(tester);
      expect(fake.cells['a']!.text, '1');
      expect(editor(tester), 'three');
      // The field keeps the keyboard all the while.
      expect(focused(tester), isTrue);
      expect(shown(tester, '0-0'), '1');
      await press(tester, 'cancel');
      await press(tester, 'cell-0-0');
      type(tester, '2');
      await press(tester, 'tool-right');
      expect(fake.cells['a']!.text, '2');
      expect(editor(tester), '=X');
      expect(focused(tester), isTrue);
      await press(tester, 'cancel');
      // Inside a Full grid, the cell below is the one of that grid.
      await press(tester, 'cell-0-1/0-0');
      type(tester, '5');
      await pressReturn(tester);
      expect(fake.cells['e']!.text, '5');
      expect(editor(tester), '=1+1');
      await press(tester, 'cancel');
      // Each entering was a step of its own.
      await press(tester, 'undo');
      await press(tester, 'undo');
      await press(tester, 'undo');
      expect((fake.cells['a']!.text, fake.cells['e']!.text), ('one', 'five'));
    });

    screenTest(
      'Return on text that the core refuses leaves the editor in use',
      (tester, keeper, fake) async {
        await press(tester, 'cell-0-0');
        type(tester, 'too long');
        failFirst(
          fake,
          'retype',
          const BridgeError(kind: ErrorKind.limit, message: 'too long'),
        );
        await pressReturn(tester);
        expect(line(tester, 'message'), 'Error (limit): too long');
        expect(editor(tester), 'too long');
        expect(focused(tester), isTrue);
        expect(shown(tester, '0-0'), 'too long');
        expect(fake.cells['a']!.text, 'one');
        // Nothing is left open in the document, and the same again works.
        expect(keeper.editing!.held, isFalse);
        await press(tester, 'save');
        expect(fake.cells['a']!.text, 'too long');
        expect(line(tester, 'message'), isNull);
      },
    );

    screenTest('↵ breaks the line where the cursor is', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      editorField(tester).selection = const TextSelection.collapsed(offset: 1);
      await press(tester, 'tool-break');
      expect(editor(tester), 'o\nne');
      expect(
        editorField(tester).selection,
        const TextSelection.collapsed(offset: 2),
      );
      expect(focused(tester), isTrue);
      expect(shown(tester, '0-0'), 'o\nne');
      await press(tester, 'save');
      expect(fake.cells['a']!.text, 'o\nne');
    });

    screenTest('Aa shows the styles, and hides them again', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      expect(key('style-bold'), findsNothing);
      await press(tester, 'tool-styles');
      for (final style in ['bold', 'italic', 'link', 'tag']) {
        expect(key('style-$style'), findsOneWidget);
      }
      expect(focused(tester), isTrue);
      // A style holds the text first, in the editing's own group. (The stand-in
      // knows no styles: that they are put on is tested on the real core.)
      type(tester, 'typed');
      await press(tester, 'style-bold');
      expect(fake.cells['a']!.text, 'typed');
      expect(keeper.editing!.held, isTrue);
      expect(
        line(tester, 'message'),
        'Error (refused): the stand-in does not know this command',
      );
      expect(editor(tester), 'typed');
      await press(tester, 'tool-styles');
      expect(key('style-bold'), findsNothing);
      // The cross takes the held text back.
      await press(tester, 'cancel');
      expect(fake.cells['a']!.text, 'one');
      // The next editor starts without the row of styles.
      await press(tester, 'cell-0-0');
      await press(tester, 'tool-styles');
      await press(tester, 'save');
      await press(tester, 'cell-0-0');
      expect(key('style-bold'), findsNothing);
      await press(tester, 'cancel');
    });

    screenTest('Open › goes into the grid of the edited cell, text kept', (
      tester,
      keeper,
      fake,
    ) async {
      TextButton open() => tester.widget<TextButton>(key('tool-open'));
      await press(tester, 'cell-0-0');
      expect(open().onPressed, isNull);
      await press(tester, 'cancel');
      await press(tester, 'cell-0-1');
      expect(open().onPressed, isNotNull);
      type(tester, 'renamed');
      await press(tester, 'tool-open');
      await arrive(tester);
      expect(key('editor'), findsNothing);
      expect(fake.cells['c']!.text, 'renamed');
      expect(line(tester, 'title'), 'renamed');
      expect(shown(tester, '0-0'), 'five');
    });

    screenTest('the bars keep what is being typed, and then do what they do', (
      tester,
      keeper,
      fake,
    ) async {
      await enterBehind(keeper, 'd', '4');
      await tester.pump();
      await press(tester, 'cell-0-0');
      type(tester, 'typed');
      await press(tester, 'undo');
      // The text was entered, and that is what the undo took back.
      expect(key('editor'), findsNothing);
      expect(calls(fake, 'endGroup'), 2);
      expect((fake.cells['a']!.text, fake.cells['d']!.text), ('one', '4'));
      await press(tester, 'cell-0-0');
      type(tester, 'again');
      await press(tester, 'redo');
      expect(fake.cells['a']!.text, 'again');
      expect(line(tester, 'message'), 'Nothing to redo');
      // If the text cannot be entered, the action is not done.
      await press(tester, 'cell-0-0');
      type(tester, 'too much');
      failFirst(fake, 'retype', broken);
      await press(tester, 'undo');
      expect(editor(tester), 'too much');
      expect(line(tester, 'message'), 'Error (io): the disk is gone');
      expect((fake.cells['a']!.text, fake.cells['d']!.text), ('again', '4'));
      await press(tester, 'cancel');
    });
  });

  group('what was typed is not thrown away', () {
    for (final target in ['cell-1-1', 'child-0-1', 'cell-0-0']) {
      screenTest(
        'a tap on $target with text that the core refuses: the editor stays',
        (tester, keeper, fake) async {
          await press(tester, 'cell-0-0');
          type(tester, 'too much');
          failFirst(fake, 'retype', broken);
          await press(tester, target);
          expect(editor(tester), 'too much');
          expect(line(tester, 'message'), 'Error (io): the disk is gone');
          expect(line(tester, 'title'), 'Root');
          expect(fake.cells['a']!.text, 'one');
          // The same tap again closes it, with the text.
          await press(tester, target);
          expect(key('editor'), findsNothing);
          expect(fake.cells['a']!.text, 'too much');
        },
        before: bigChild,
      );
    }

    for (final leave in ['up', 'trail-0']) {
      screenTest('$leave keeps the text and goes there', (
        tester,
        keeper,
        fake,
      ) async {
        await dive(tester);
        await press(tester, 'cell-0-0');
        type(tester, 'typed');
        await press(tester, leave);
        await arrive(tester);
        expect(fake.cells['g0']!.text, 'typed');
        expect(key('editor'), findsNothing);
        expect(line(tester, 'title'), 'Root');
        // Text that the core refuses: the editor stays, and so does the level.
        await dive(tester);
        await press(tester, 'cell-0-0');
        type(tester, 'too much');
        failFirst(fake, 'retype', broken);
        await press(tester, leave);
        expect(editor(tester), 'too much');
        expect(line(tester, 'title'), 'three');
        expect(line(tester, 'message'), 'Error (io): the disk is gone');
        await press(tester, 'cancel');
      }, before: bigChild);
    }

    screenTest('while Return is at work the editor keeps the keyboard', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'typed');
      final gate = holdFirst(fake, 'retype');
      await pressReturn(tester);
      expect(focused(tester), isTrue);
      expect(editor(tester), 'typed');
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect(editor(tester), 'three');
      expect(focused(tester), isTrue);
      await press(tester, 'cancel');
    });
  });

  group('more is typed while the core is at work', () {
    screenTest('on the tick: the editor stays, with all of it', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'first');
      final gate = holdFirst(fake, 'retype');
      await press(tester, 'save');
      type(tester, 'first and more');
      await tester.pump();
      // The cell follows the editor while the core has the first part.
      expect(shown(tester, '0-0'), 'first and more');
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect(
        (fake.cells['a']!.text, editor(tester)),
        ('first', 'first and more'),
      );
      expect(shown(tester, '0-0'), 'first and more');
      expect(focused(tester), isTrue);
      expect(line(tester, 'message'), isNull);
      // What was sent is one editing, ended; the rest has one of its own, open
      // and holding nothing.
      expect(calls(fake, 'endGroup'), 1);
      expect((keeper.editing!.open, keeper.editing!.held), (true, false));
      await press(tester, 'save');
      expect(key('editor'), findsNothing);
      expect(fake.cells['a']!.text, 'first and more');
      // Two steps: what was said yes to each time.
      await press(tester, 'undo');
      expect(fake.cells['a']!.text, 'first');
      await press(tester, 'undo');
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('on Return: the editor stays on the cell it was typed in', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'first');
      final gate = holdFirst(fake, 'retype');
      await pressReturn(tester);
      type(tester, 'first and more');
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect(
        (fake.cells['a']!.text, editor(tester)),
        ('first', 'first and more'),
      );
      expect(shown(tester, '0-0'), 'first and more');
      expect(shown(tester, '0-1'), 'three');
      await pressReturn(tester);
      expect(fake.cells['a']!.text, 'first and more');
      expect(editor(tester), 'three');
      await press(tester, 'cancel');
    });

    for (final (name, tap) in [
      ('undo', 'undo'),
      ('redo', 'redo'),
      ('up', 'up'),
      ('the trail', 'trail-0'),
    ]) {
      screenTest('on $name: said, and not done', (tester, keeper, fake) async {
        await dive(tester);
        await press(tester, 'cell-0-0');
        type(tester, 'first');
        final gate = holdFirst(fake, 'retype');
        await press(tester, tap);
        type(tester, 'first and more');
        gate.complete();
        await tester.pump();
        await tester.pump();
        // What was sent is in the document, the rest is in the editor, and
        // the action was not done, which is said.
        expect(fake.cells['g0']!.text, 'first');
        expect(editor(tester), 'first and more');
        expect(line(tester, 'title'), 'three');
        expect(line(tester, 'message'), typedOn);
        expect(fake.log.contains('undo'), isFalse);
        // The same again does it.
        await press(tester, tap);
        await arrive(tester);
        expect(
          fake.cells['g0']!.text,
          tap == 'undo' ? 'first' : 'first and more',
        );
        expect(key('editor'), findsNothing);
        expect(
          line(tester, 'message'),
          tap == 'redo' ? 'Nothing to redo' : null,
        );
      }, before: bigChild);
    }

    screenTest('and then the app away, then the cross: back to what was said '
        'yes to', (tester, keeper, fake) async {
      await press(tester, 'cell-0-0');
      type(tester, 'first');
      final gate = holdFirst(fake, 'retype');
      await press(tester, 'save');
      type(tester, 'first and more');
      gate.complete();
      await tester.pump();
      await tester.pump();
      await awayAndBack(tester, AppLifecycleState.hidden);
      expect(fake.cells['a']!.text, 'first and more');
      expect(keeper.editing!.held, isTrue);
      await press(tester, 'cancel');
      expect(key('editor'), findsNothing);
      expect((fake.cells['a']!.text, keeper.editing), ('first', null));
      await press(tester, 'undo');
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('and then the screen goes: the rest is entered with one call', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'first');
      final gate = holdFirst(fake, 'retype');
      await press(tester, 'save');
      type(tester, 'first and more');
      gate.complete();
      await tester.pump();
      await tester.pump();
      await tester.pumpWidget(const SizedBox());
      // As the app does when its screen goes: the close comes behind the text.
      await keeper.close();
      expect(fake.cells['a']!.text, 'first and more');
      expect(fake.log, containsAllInOrder(['endGroup', 'endGroup', 'close']));
      expect((fake.saves, fake.closed), (1, true));
    });
  });

  group('one action after the other', () {
    // Each action finds the screen as the one before left it, not as it was when
    // the finger came down.
    screenTest('a cell tapped, then a grid, behind an undo that is held up', (
      tester,
      keeper,
      fake,
    ) async {
      await enterBehind(keeper, 'a', 'x');
      await tester.pump();
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.tap(key('cell-0-0'));
      await tester.tap(key('child-0-1'));
      await tester.pump();
      expect(key('editor'), findsNothing);
      gate.complete();
      await tester.pump();
      await tester.pump();
      // In their order: the undo, the editor on what it left, and the tap
      // outside the editor, which only closes it.
      expect(key('editor'), findsNothing);
      expect(line(tester, 'title'), 'Root');
      expect(fake.cells['a']!.text, 'one');
      expect(calls(fake, 'cell'), 2);
    }, before: bigChild);

    for (final leave in ['up', 'trail-0']) {
      screenTest('a cell tapped, then $leave, behind an undo that is held up', (
        tester,
        keeper,
        fake,
      ) async {
        await dive(tester);
        await enterBehind(keeper, 'g0', 'x');
        await tester.pump();
        final gate = holdFirst(fake, 'undo');
        await tester.tap(key('undo'));
        await tester.tap(key('cell-0-0'));
        await tester.tap(key(leave));
        await tester.pump();
        gate.complete();
        await tester.pump();
        await tester.pump();
        // The editor opened on "g0", and the way up closed it and went up.
        expect(key('editor'), findsNothing);
        expect(line(tester, 'title'), 'Root');
        expect(fake.cells['g0']!.text, 'g0');
      }, before: bigChild);
    }

    screenTest(
      'a cell tapped, then a grid, behind the save that came by itself',
      (tester, keeper, fake) async {
        await enterBehind(keeper, 'a', 'x');
        await tester.pump();
        final gate = holdFirst(fake, 'save');
        await tester.pump(const Duration(seconds: 1));
        expect(fake.log.last, 'save');
        expect(line(tester, 'state'), 'Saving…');
        await tester.tap(key('cell-0-0'));
        await tester.tap(key('child-0-1'));
        await tester.pump();
        gate.complete();
        await tester.pump();
        await tester.pump();
        expect(key('editor'), findsNothing);
        expect(line(tester, 'title'), 'Root');
        expect(line(tester, 'state'), 'Saved');
      },
      before: bigChild,
    );

    screenTest('up, then a cell, while the way up is held: no editor on it', (
      tester,
      keeper,
      fake,
    ) async {
      await dive(tester);
      final gate = holdFirst(fake, 'stage');
      await tester.tap(key('up'));
      // Where "g0" is now, "one" will be: the tap meant a cell that is gone
      // from the stage when its turn comes, and opens nothing.
      await tester.tap(key('cell-0-0'));
      await tester.pump();
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect(line(tester, 'title'), 'Root');
      expect(key('editor'), findsNothing);
      expect(line(tester, 'message'), isNull);
    }, before: bigChild);

    screenTest('two taps on undo with the editor open undo twice', (
      tester,
      keeper,
      fake,
    ) async {
      await enterBehind(keeper, 'd', '4');
      await tester.pump();
      await press(tester, 'cell-0-0');
      type(tester, 'typed');
      final gate = holdFirst(fake, 'retype');
      await tester.tap(key('undo'));
      await tester.tap(key('undo'));
      await tester.pump();
      gate.complete();
      await tester.pump();
      await tester.pump();
      // The first takes back what was typed, the second the edit before it.
      expect((fake.cells['a']!.text, fake.cells['d']!.text), ('one', 'four'));
      expect(line(tester, 'message'), isNull);
      expect(key('editor'), findsNothing);
    });

    testWidgets('an error that no action expected is shown like the others', (
      tester,
    ) async {
      screenSize(tester, phone);
      final fake = FakeDocument();
      final keeper = _Mistaken(await Session.over(fake));
      await tester.pumpWidget(phoneApp(keeper));
      await tester.pump();
      await enterBehind(keeper, 'a', 'mine');
      fake.fileChanged = true;
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
      // Keeping mine never throws, and so nothing catches what it throws here.
      await press(tester, 'keep-mine');
      expect(line(tester, 'message'), 'Error: Bad state: a mistake');
      expect(tester.takeException(), isNull);
      // The queue of actions goes on.
      await press(tester, 'undo');
      expect(fake.cells['a']!.text, 'one');
      // With the screen gone there is no line to show it on: it is not
      // swallowed. It goes where the tap came from, as an error that nobody
      // caught.
      final gate = keeper.gate = Completer<void>();
      final uncaught = <Object>[];
      await runZonedGuarded(
        () => tester.tap(key('keep-mine')),
        (error, _) => uncaught.add(error),
      );
      await tester.pumpWidget(const SizedBox());
      gate.complete();
      await tester.pump();
      expect(uncaught.single, isStateError);
      await keeper.letGo();
    });
  });

  group('taps that come close together', () {
    screenTest('two taps on a grid go in once', (tester, keeper, fake) async {
      // The core is slow to read the grid, so the second tap comes while the
      // first is still at work, and finds the cell gone from the stage.
      final gate = holdFirst(fake, 'stage');
      await tester.tap(key('child-0-1'));
      await tester.tap(key('child-0-1'));
      await tester.pump();
      expect(line(tester, 'title'), 'Root');
      gate.complete();
      await tester.pump();
      await arrive(tester);
      expect(line(tester, 'title'), 'three');
      expect(find.text('› three'), findsOneWidget);
      expect(key('trail-2'), findsNothing);
      expect(line(tester, 'message'), isNull);
      // Gone to once, not twice to the same place.
      expect(calls(fake, 'viewGo'), 1);
    }, before: bigChild);

    screenTest('two taps on undo undo twice', (tester, keeper, fake) async {
      await enterBehind(keeper, 'a', '1');
      await enterBehind(keeper, 'd', '4');
      await tester.pump();
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.tap(key('undo'));
      await tester.pump();
      expect((shown(tester, '0-0'), shown(tester, '1-1')), ('1', '4'));
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect((shown(tester, '0-0'), shown(tester, '1-1')), ('one', 'four'));
    });

    screenTest('a cell tapped while an undo runs opens on what the undo left', (
      tester,
      keeper,
      fake,
    ) async {
      await enterBehind(keeper, 'a', 'typed');
      await tester.pump();
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.tap(key('cell-0-0'));
      await tester.pump();
      // Nothing yet: the editor must not open on text that is about to change.
      expect(key('editor'), findsNothing);
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect(editor(tester), 'one');
      await press(tester, 'cancel');
    });

    screenTest('many taps on one cell: one editor, and nothing thrown', (
      tester,
      keeper,
      fake,
    ) async {
      // Open, close, open, close, open: each tap is judged when its turn comes.
      for (var i = 0; i < 5; i++) {
        await tester.tap(key('cell-0-0'), warnIfMissed: false);
      }
      await tester.pump();
      await tester.pump();
      expect(editor(tester), 'one');
      expect(key('editor'), findsOneWidget);
      expect(tester.takeException(), isNull);
      for (var i = 0; i < 4; i++) {
        await tester.tap(key('save'), warnIfMissed: false);
        await tester.tap(key('cell-1-1'), warnIfMissed: false);
        await tester.tap(key('cancel'), warnIfMissed: false);
      }
      await tester.pump();
      await tester.pump();
      expect(tester.takeException(), isNull);
      expect(fake.cells['a']!.text, 'one');
      expect(fake.cells['d']!.text, 'four');
      if (there('editor')) await press(tester, 'cancel');
    });

    screenTest(
      'every button in a row, while the core is slow: nothing thrown or lost',
      (tester, keeper, fake) async {
        await press(tester, 'cell-0-0');
        type(tester, 'kept');
        final gate = holdFirst(fake, 'retype');
        for (final name in [
          'tool-right',
          'tool-row',
          'tool-col',
          'tool-break',
          'save',
          'undo',
          'redo',
          'child-0-1',
          'up',
          'cancel',
          'add-row',
          'add-column',
        ]) {
          if (there(name)) {
            await tester.tap(key(name), warnIfMissed: false);
          }
        }
        await back(tester);
        gate.complete();
        for (var i = 0; i < 6; i++) {
          await tester.pump(const Duration(milliseconds: 100));
        }
        expect(tester.takeException(), isNull);
        // Whatever of it was done, the text that was typed was not thrown away
        // by anything but the user's own undo.
        expect(fake.log.contains('retype.'), isTrue);
        expect(keeper.editing?.held ?? false, isFalse);
        if (there('editor')) await press(tester, 'cancel');
      },
      before: bigChild,
    );

    screenTest('an action that ends after the screen is gone is not shown', (
      tester,
      keeper,
      fake,
    ) async {
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.pumpWidget(const MaterialApp(home: SizedBox()));
      gate.complete();
      await tester.pump();
      // "Nothing to undo" had nowhere to go, and nothing was thrown for that.
      expect(tester.takeException(), isNull);
    });

    screenTest('actions still waiting when the screen goes are not done', (
      tester,
      keeper,
      fake,
    ) async {
      await enterBehind(keeper, 'a', '1');
      await enterBehind(keeper, 'd', '4');
      await tester.pump();
      final gate = holdFirst(fake, 'undo');
      await tester.tap(key('undo'));
      await tester.tap(key('undo'));
      await tester.tap(key('child-0-1'));
      await tester.pumpWidget(const MaterialApp(home: SizedBox()));
      gate.complete();
      await tester.pump();
      await tester.pump();
      // The one that was at work ended; the ones behind it never began.
      expect((fake.cells['a']!.text, fake.cells['d']!.text), ('1', 'four'));
      expect(fake.zoom, 'root');
      expect(tester.takeException(), isNull);
    }, before: bigChild);

    for (final (name, act) in [
      ('the tick', (WidgetTester tester) => tester.tap(key('save'))),
      (
        'Return',
        (WidgetTester tester) =>
            tester.testTextInput.receiveAction(TextInputAction.next),
      ),
      ('→', (WidgetTester tester) => tester.tap(key('tool-right'))),
      ('undo', (WidgetTester tester) => tester.tap(key('undo'))),
    ]) {
      screenTest(
        'the screen goes while $name enters the text: no more is done',
        (tester, keeper, fake) async {
          await press(tester, 'cell-0-0');
          type(tester, 'typed');
          final gate = holdFirst(fake, 'retype');
          await act(tester);
          await tester.pump();
          await tester.pumpWidget(const MaterialApp(home: SizedBox()));
          gate.complete();
          await tester.pump();
          await tester.pump();
          // The text is in the document; the editor that is gone is not moved,
          // and what was typed is not undone behind the user's back.
          expect(fake.cells['a']!.text, 'typed');
          expect(fake.log.contains('undo'), isFalse);
          expect(calls(fake, 'cell'), 1);
          expect(keeper.editing, isNull);
          expect(tester.takeException(), isNull);
        },
      );
    }
  });

  group('the document is saved without being asked', () {
    screenTest('a second after the first change, however many follow', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, '1');
      await press(tester, 'save');
      expect(line(tester, 'state'), 'Not saved yet');
      await tester.pump(const Duration(milliseconds: 600));
      await press(tester, 'cell-1-1');
      type(tester, '4');
      await press(tester, 'save');
      expect(fake.saves, 0);
      // A second after the first change, not after the last.
      await tester.pump(const Duration(milliseconds: 400));
      expect(fake.saves, 1);
      expect(fake.saved, isTrue);
      await tester.pump();
      expect(line(tester, 'state'), 'Saved');
      // Nothing more without a change.
      await tester.pump(const Duration(seconds: 5));
      expect(fake.saves, 1);
      await press(tester, 'undo');
      await tester.pump(const Duration(seconds: 1));
      expect(fake.saves, 2);
    });

    screenTest(
      'when the app is no longer in front, but not what is being typed',
      (tester, keeper, fake) async {
        await enterBehind(keeper, 'd', 'entered');
        await press(tester, 'cell-0-0');
        type(tester, 'half a sentence');
        // A window that loses the focus, a dialog of the system: not leaving.
        await lifecycle(tester, AppLifecycleState.inactive);
        expect(fake.saves, 1);
        expect(fake.cells['a']!.text, 'one');
        expect(calls(fake, 'beginGroup'), 1);
        await lifecycle(tester, AppLifecycleState.resumed);
        expect(editor(tester), 'half a sentence');
        expect(focused(tester), isTrue);
        // So the cross is still "not this", with nothing to take back.
        await press(tester, 'cancel');
        expect(fake.cells['a']!.text, 'one');
        expect(fake.log.contains('cancelGroup'), isFalse);
      },
    );

    for (var steps = 2; steps <= away.length; steps++) {
      final state = away[steps - 1];
      screenTest(
        'at once when the app is ${state.name}, with what is being typed',
        (tester, keeper, fake) async {
          await press(tester, 'cell-0-0');
          type(tester, 'half a sentence');
          for (final step in away.take(steps)) {
            await lifecycle(tester, step);
          }
          expect(fake.cells['a']!.text, 'half a sentence');
          // One editing and written once, however many steps the way has: the
          // text is held again at each, which changes nothing, and the saves
          // after it find nothing to write.
          expect(calls(fake, 'beginGroup'), 1);
          expect(fake.saves, 1);
          expect(fake.saved, isTrue);
          expect(
            fake.log,
            containsAllInOrder(['beginGroup', 'retype.', 'apply.', 'save']),
          );
          // Held, not entered: the editing goes on, for the tick or the cross.
          expect(fake.log.contains('endGroup'), isFalse);
          expect(keeper.editing!.held, isTrue);
          // The editor is still there for when the app comes back.
          expect(editor(tester), 'half a sentence');

          final back = state == AppLifecycleState.detached
              ? <AppLifecycleState>[]
              : away.take(steps - 1).toList().reversed;
          for (final step in back) {
            await lifecycle(tester, step);
          }
          final saves = calls(fake, 'save');
          await lifecycle(tester, AppLifecycleState.resumed);
          expect(calls(fake, 'save'), saves);
          expect(shown(tester, '0-0'), 'half a sentence');
          await press(tester, 'cancel');
          expect(fake.cells['a']!.text, 'one');
        },
      );
    }

    screenTest(
      'the cross after the app was out of sight still means "not this"',
      (tester, keeper, fake) async {
        // Something that can be redone, and nothing that can be undone.
        await enterBehind(keeper, 'd', 'undone');
        await keeper.undo();
        await keeper.save();
        await press(tester, 'cell-0-0');
        type(tester, 'not this');
        await lifecycle(tester, AppLifecycleState.inactive);
        await lifecycle(tester, AppLifecycleState.hidden);
        expect((fake.cells['a']!.text, fake.saves), ('not this', 2));
        await lifecycle(tester, AppLifecycleState.inactive);
        await lifecycle(tester, AppLifecycleState.resumed);
        expect(shown(tester, '0-0'), 'not this');
        await press(tester, 'cancel');
        expect(key('editor'), findsNothing);
        expect(fake.cells['a']!.text, 'one');
        expect(shown(tester, '0-0'), 'one');
        expect(keeper.editing, isNull);
        // The file follows a second later.
        await tester.pump(const Duration(seconds: 1));
        expect((fake.saves, fake.saved), (3, true));
        // Nothing of it is left to undo, and Redo does not bring it back: what
        // could be redone before the editing still can.
        await press(tester, 'undo');
        expect(line(tester, 'message'), 'Nothing to undo');
        await press(tester, 'redo');
        expect(
          (fake.cells['a']!.text, fake.cells['d']!.text),
          ('one', 'undone'),
        );
        await press(tester, 'redo');
        expect(line(tester, 'message'), 'Nothing to redo');
      },
    );

    screenTest('one editing, the app out of sight in the middle: one undo', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'half');
      await awayAndBack(tester, AppLifecycleState.hidden);
      type(tester, 'half and more');
      // Away a second time, with more: still not said yes to.
      await awayAndBack(tester, AppLifecycleState.paused);
      expect(fake.cells['a']!.text, 'half and more');
      type(tester, 'half and whole');
      await press(tester, 'save');
      expect(fake.cells['a']!.text, 'half and whole');
      await press(tester, 'undo');
      expect(fake.cells['a']!.text, 'one');
      await press(tester, 'undo');
      expect(line(tester, 'message'), 'Nothing to undo');
    });

    screenTest('the tick on the very text that went in when the app left', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'whole');
      await awayAndBack(tester, AppLifecycleState.hidden);
      final saves = fake.saves;
      await press(tester, 'save');
      // The document has it: it is said yes to, and that is the end of the
      // editing. Nothing changed, so nothing is read or written again.
      expect(fake.log.last, 'endGroup.');
      expect(fake.log, containsAllInOrder(['save.', 'endGroup.']));
      expect(key('editor'), findsNothing);
      expect(keeper.editing, isNull);
      await tester.pump(const Duration(minutes: 1));
      expect(fake.saves, saves);
      // So a cross on the next editing of the cell takes nothing back.
      await press(tester, 'cell-0-0');
      await press(tester, 'cancel');
      expect(fake.cells['a']!.text, 'whole');
      // And the whole of it is one step.
      await press(tester, 'undo');
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('away with the text the cell has: nothing to take back', (
      tester,
      keeper,
      fake,
    ) async {
      await enterBehind(keeper, 'd', 'before');
      await press(tester, 'cell-0-0');
      await awayAndBack(tester, AppLifecycleState.hidden);
      await press(tester, 'cancel');
      // The cross did not undo the edit before this editing.
      expect(fake.cells['d']!.text, 'before');
      expect(fake.log.contains('undo'), isFalse);
    });

    screenTest(
      'text that the core refuses when the app leaves is kept and said',
      (tester, keeper, fake) async {
        await enterBehind(keeper, 'd', 'entered');
        await press(tester, 'cell-0-0');
        type(tester, 'too long');
        failFirst(
          fake,
          'retype',
          const BridgeError(kind: ErrorKind.limit, message: 'too long'),
        );
        await lifecycle(tester, AppLifecycleState.inactive);
        await lifecycle(tester, AppLifecycleState.hidden);
        expect(editor(tester), 'too long');
        expect(line(tester, 'message'), 'Error (limit): too long');
        // What was in the document is saved all the same.
        expect((fake.saves, fake.saved), (1, true));
        // And the cross has nothing to take back.
        await press(tester, 'cancel');
        expect(fake.cells['d']!.text, 'entered');
        expect(fake.cells['a']!.text, 'one');
      },
    );

    screenTest('what one editing left unconfirmed is not the next one\'s', (
      tester,
      keeper,
      fake,
    ) async {
      await enterBehind(keeper, 'd', 'before');
      await press(tester, 'cell-0-0');
      type(tester, 'half');
      await awayAndBack(tester, AppLifecycleState.hidden);
      // Return says yes to it and moves on; the cross on the next cell then has
      // nothing to take back.
      await pressReturn(tester);
      expect(editor(tester), 'three');
      await press(tester, 'cancel');
      expect(fake.cells['a']!.text, 'half');
      expect(fake.log.contains('cancelGroup'), isFalse);
      // And after a cross that did take something back, neither has the next.
      await press(tester, 'cell-1-0');
      type(tester, 'not this');
      await awayAndBack(tester, AppLifecycleState.hidden);
      expect(fake.cells['b']!.text, 'not this');
      await press(tester, 'cancel');
      await press(tester, 'cell-1-0');
      await press(tester, 'cancel');
      expect(calls(fake, 'cancelGroup'), 1);
      expect(fake.log.contains('undo'), isFalse);
      expect(
        (fake.cells['a']!.text, fake.cells['b']!.text, fake.cells['d']!.text),
        ('half', '=X', 'before'),
      );
    });

    screenTest('what is held is nobody else\'s to take back or to add to', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'not this');
      await awayAndBack(tester, AppLifecycleState.hidden);
      // Behind the screen, as a second screen could: refused, and loudly.
      await expectLater(keeper.undo(), throwsStateError);
      await expectLater(enterBehind(keeper, 'd', 'theirs'), throwsStateError);
      expect(
        (fake.cells['a']!.text, fake.cells['d']!.text),
        ('not this', 'four'),
      );
      await press(tester, 'cancel');
      expect(key('editor'), findsNothing);
      expect(line(tester, 'message'), isNull);
      expect(fake.cells['a']!.text, 'one');
      // And free again after the cross.
      await enterBehind(keeper, 'd', 'theirs');
    });

    screenTest('a cross that cannot take the text back leaves the editor', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'not this');
      await awayAndBack(tester, AppLifecycleState.hidden);
      failFirst(fake, 'cancelGroup', broken);
      await press(tester, 'cancel');
      expect(line(tester, 'message'), 'Error (io): the disk is gone');
      expect(editor(tester), 'not this');
      expect(keeper.editing!.held, isTrue);
      await press(tester, 'cancel');
      expect(key('editor'), findsNothing);
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('a screen that goes with its editor open enters the text', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'left in the editor');
      await tester.pumpWidget(const SizedBox());
      // As the app does when its screen goes: the close waits for the text.
      await keeper.close();
      expect(fake.cells['a']!.text, 'left in the editor');
      expect(fake.log, containsAllInOrder(['endGroup.', 'close']));
      expect((fake.saves, fake.closed), (1, true));
      // One read of the cell, when the editor opened on it: the editing that
      // was open is the one that was entered.
      expect((calls(fake, 'cell'), calls(fake, 'beginGroup')), (1, 1));
    });

    screenTest('a screen that goes after the app was away enters one step', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'half');
      await awayAndBack(tester, AppLifecycleState.hidden);
      type(tester, 'half and whole');
      await tester.pumpWidget(const SizedBox());
      await tester.pump();
      expect(fake.cells['a']!.text, 'half and whole');
      await keeper.undo();
      expect(fake.cells['a']!.text, 'one');
    });

    screenTest('a screen that has gone is no longer asked or told', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'the same');
      await awayAndBack(tester, AppLifecycleState.hidden);
      await tester.pumpWidget(const SizedBox());
      // Its text was in the document already; going, it ends the editing, so
      // that no group stays open behind it.
      await tester.pump();
      expect(fake.log.last, 'endGroup.');
      expect(keeper.editing, isNull);
      final before = fake.log.length;
      // What watched the app's lifecycle for it went with it.
      await awayAndBack(tester, AppLifecycleState.paused);
      expect(fake.log.length, before);
      // Neither a change of the document nor the app being put away reaches
      // it: its stage is no longer read, and it is not asked to finish.
      await keeper.undo();
      expect(calls(fake, 'stage'), calls(fake, 'stage.'));
      final stages = calls(fake, 'stage');
      await keeper.redo();
      expect(calls(fake, 'stage'), stages);
      await keeper.putAway();
      expect(tester.takeException(), isNull);
      // ignore: invalid_use_of_protected_member
      expect(keeper.hasListeners, isFalse);
      // Held once and entered once, each of which settles what the cell is.
      expect((calls(fake, 'apply'), calls(fake, 'retype')), (2, 2));
    });
  });

  group('what goes wrong is said, and nothing else happens', () {
    screenTest('a grid that cannot be read is not gone into', (
      tester,
      keeper,
      fake,
    ) async {
      failFirst(fake, 'stage', broken);
      await press(tester, 'child-0-1');
      expect(line(tester, 'message'), 'Error (io): the disk is gone');
      expect(line(tester, 'title'), 'Root');
      expect(fake.zoom, 'root');
      await dive(tester);
      expect(line(tester, 'title'), 'three');
      expect(line(tester, 'message'), isNull);
    }, before: bigChild);

    screenTest('an undo that fails is said', (tester, keeper, fake) async {
      await enterBehind(keeper, 'a', 'typed');
      await tester.pump();
      failFirst(fake, 'undo', broken);
      await press(tester, 'undo');
      expect(line(tester, 'message'), 'Error (io): the disk is gone');
      expect(shown(tester, '0-0'), 'typed');
      await press(tester, 'undo');
      expect(shown(tester, '0-0'), 'one');
    });

    screenTest('a cell that cannot be read is not opened', (
      tester,
      keeper,
      fake,
    ) async {
      failFirst(fake, 'cell', broken);
      await press(tester, 'cell-0-0');
      expect(line(tester, 'message'), 'Error (io): the disk is gone');
      expect(key('editor'), findsNothing);
      await press(tester, 'cell-0-0');
      expect(editor(tester), 'one');
      await press(tester, 'cancel');
    });

    screenTest('the screen goes while the editor is opening: nothing thrown', (
      tester,
      keeper,
      fake,
    ) async {
      final gate = holdFirst(fake, 'cell');
      await tester.tap(key('cell-0-0'));
      await tester.pump();
      await tester.pumpWidget(const MaterialApp(home: SizedBox()));
      gate.complete();
      await tester.pump();
      await tester.pump();
      expect(tester.takeException(), isNull);
      expect(fake.log.contains('beginGroup'), isFalse);
    });

    screenTest(
      'the cross on an editing that something else ended just closes',
      (tester, keeper, fake) async {
        await press(tester, 'cell-0-0');
        type(tester, 'not this');
        await enterBehind(keeper, 'd', 'theirs');
        await tester.pump();
        await press(tester, 'cancel');
        expect(key('editor'), findsNothing);
        expect(line(tester, 'message'), isNull);
        expect(
          (fake.cells['a']!.text, fake.cells['d']!.text),
          ('one', 'theirs'),
        );
      },
    );

    screenTest(
      'a style asked for behind the tick finds no editor, and says nothing',
      (tester, keeper, fake) async {
        await press(tester, 'cell-0-0');
        await press(tester, 'tool-styles');
        type(tester, 'typed');
        // (Each tap takes away what the one before it had to say, so each of the
        // two is looked at.)
        await tester.tap(key('save'));
        await tester.tap(key('tool-break'));
        await tester.tap(key('style-bold'));
        await tester.pump();
        await tester.pump();
        expect(key('editor'), findsNothing);
        expect(line(tester, 'message'), isNull);
        await press(tester, 'cell-0-0');
        await press(tester, 'tool-styles');
        await tester.tap(key('save'));
        await tester.tap(key('style-tag'));
        await tester.pump();
        await tester.pump();
        expect(key('editor'), findsNothing);
        expect(line(tester, 'message'), isNull);
        expect(fake.cells['a']!.text, 'typed');
        // One group for each of the two ticks, none for a style.
        expect(calls(fake, 'beginGroup'), 2);
      },
    );

    screenTest('an editing that something else ended gives way to a new one', (
      tester,
      keeper,
      fake,
    ) async {
      await press(tester, 'cell-0-0');
      type(tester, 'typed');
      // Behind the screen, as a second screen could: its editing takes the
      // place of this one, which held nothing.
      await enterBehind(keeper, 'd', 'theirs');
      await tester.pump();
      expect(shown(tester, '0-0'), 'typed');
      await press(tester, 'save');
      expect(key('editor'), findsNothing);
      expect(line(tester, 'message'), isNull);
      expect(
        (fake.cells['a']!.text, fake.cells['d']!.text),
        ('typed', 'theirs'),
      );
    });
  });

  group('the screen and its keeper', () {
    testWidgets(
      'a screen that goes before its first read leaves nothing behind',
      (tester) async {
        final fake = FakeDocument();
        final keeper = Keeper(await Session.over(fake));
        final gate = holdFirst(fake, 'viewStage');
        await tester.pumpWidget(phoneApp(keeper));
        await tester.pump();
        expect(key('reading'), findsOneWidget);
        await tester.pumpWidget(const SizedBox());
        gate.complete();
        await tester.pump();
        await tester.pump();
        expect(tester.takeException(), isNull);
        // The stage that was read for it is no longer kept up to date.
        final stages = fake.log.where((call) => call == 'stage').length;
        await (await keeper.edit('a')).enter('typed');
        expect(fake.log.where((call) => call == 'stage').length, stages);
        await keeper.close();
      },
    );

    screenTest(
      'what the keeper asks when the app is put away takes its turn among the '
      'screen\'s actions',
      (tester, keeper, fake) async {
        await press(tester, 'cell-0-0');
        type(tester, 'typed');
        // Return has entered the text and is opening the editor on the cell
        // below when the app is put away: the screen's answer to that comes
        // after the action, and is about the cell the editor is on then.
        final gate = holdFirst(fake, 'cell');
        fake.log.clear();
        await pressReturn(tester);
        expect(fake.log.last, 'cell');
        final away = keeper.putAway();
        await tester.pump();
        gate.complete();
        expect(await away, isEmpty);
        await tester.pump();
        expect(editor(tester), 'three');
        expect(line(tester, 'message'), isNull);
        expect(fake.cells['a']!.text, 'typed');
        // One group for the entering, one for holding the text of the cell the
        // editor moved to; none for the cell it left.
        expect(calls(fake, 'beginGroup'), 2);
        expect(keeper.editing!.cell, 'c');
        await press(tester, 'cancel');
        expect(fake.cells['c']!.text, 'three');
      },
    );

    screenTest(
      'the flash of the level come up to is not shown again for another document',
      (tester, keeper, fake) async {
        await dive(tester);
        await press(tester, 'up');
        await tester.pump();
        expect(key('flash'), findsOneWidget);
        await enterBehind(keeper, 'a', 'mine');
        fake.fileChanged = true;
        await tester.pump(const Duration(seconds: 1));
        await tester.pump();
        await press(tester, 'open-theirs');
        await tester.pump();
        expect(shown(tester, '0-0'), 'one');
        expect(key('flash'), findsNothing);
      },
      before: bigChild,
      reopen: (_) async {
        final theirs = FakeDocument();
        bigChild(theirs);
        return Session.over(theirs);
      },
    );
  });

  group('a save that did not work is shown', () {
    screenTest('with the way to try again', (tester, keeper, fake) async {
      await enterBehind(keeper, 'a', 'typed');
      failFirst(fake, 'save', broken);
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
      expect(
        line(tester, 'trouble'),
        'Not saved. Error (io): the disk is gone',
      );
      expect(line(tester, 'state'), 'Not saved');
      expect(fake.saves, 0);
      // It stays while other things are done.
      await press(tester, 'undo');
      expect(key('trouble'), findsOneWidget);
      await press(tester, 'save-again');
      expect(key('trouble'), findsNothing);
      expect(line(tester, 'state'), 'Saved');
      expect(fake.saves, 1);
    });

    screenTest('a file changed by another program: keep mine', (
      tester,
      keeper,
      fake,
    ) async {
      await enterBehind(keeper, 'a', 'mine');
      fake.fileChanged = true;
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
      expect(
        line(tester, 'trouble'),
        'Not saved: another program changed the file.',
      );
      expect(line(tester, 'state'), 'File changed');
      expect(key('save-again'), findsNothing);
      await press(tester, 'keep-mine');
      expect((fake.overwrites, fake.saves), (1, 0));
      expect(key('trouble'), findsNothing);
      expect(shown(tester, '0-0'), 'mine');
    });

    final theirs = FakeDocument()..cells['a']!.text = 'theirs';
    screenTest(
      'a file changed by another program: open theirs',
      (tester, keeper, fake) async {
        await dive(tester);
        await enterBehind(keeper, 'g0', 'mine');
        fake.fileChanged = true;
        await tester.pump(const Duration(seconds: 1));
        await tester.pump();
        // Also what is in the editor goes: the choice is the other file.
        await press(tester, 'cell-0-0');
        type(tester, 'being typed');
        await press(tester, 'open-theirs');
        await arrive(tester);
        expect(key('editor'), findsNothing);
        expect(key('trouble'), findsNothing);
        expect(fake.disposed, isTrue);
        expect((fake.overwrites, fake.saves), (0, 0));
        // The screen is where the other document's view state says: its root.
        expect(line(tester, 'title'), 'Root');
        expect(shown(tester, '0-0'), 'theirs');
        expect(theirs.log.contains('retype'), isFalse);
        // And works on that document from here on.
        await press(tester, 'cell-0-0');
        type(tester, 'on theirs');
        await press(tester, 'save');
        expect(theirs.cells['a']!.text, 'on theirs');
        // What the keeper has to say after that (the save that comes by
        // itself) is no reason to close an editor: the document is the same.
        await press(tester, 'cell-1-1');
        type(tester, 'typed while it saves');
        await tester.pump(const Duration(seconds: 1));
        await tester.pump();
        expect(theirs.saves, 1);
        expect(editor(tester), 'typed while it saves');
        await press(tester, 'cancel');
      },
      before: bigChild,
      reopen: (path) {
        expect(path, '/fake.wombat');
        return Session.over(theirs);
      },
    );

    screenTest(
      'open theirs when theirs cannot be read: said, and nothing given up',
      (tester, keeper, fake) async {
        await enterBehind(keeper, 'd', 'mine');
        fake.fileChanged = true;
        await tester.pump(const Duration(seconds: 1));
        await tester.pump();
        await press(tester, 'cell-0-0');
        type(tester, 'being typed');
        await press(tester, 'open-theirs');
        expect(
          line(tester, 'message'),
          'The file as the other program left it cannot be opened, so this '
          'document is still open, and not saved. '
          'Error (io): the disk is gone',
        );
        // The choice is still there, and so are the document and the editor.
        expect(key('open-theirs'), findsOneWidget);
        expect(fake.disposed, isFalse);
        expect(editor(tester), 'being typed');
        await press(tester, 'keep-mine');
        expect(key('trouble'), findsNothing);
        expect(fake.overwrites, 1);
        await press(tester, 'cancel');
        expect(fake.cells['a']!.text, 'one');
      },
      canBeRead: (_) => throw broken,
    );

    screenTest(
      'open theirs when the file cannot even be looked at: said as that',
      (tester, keeper, fake) async {
        await enterBehind(keeper, 'd', 'mine');
        fake.fileChanged = true;
        await tester.pump(const Duration(seconds: 1));
        await tester.pump();
        await press(tester, 'open-theirs');
        expect(
          line(tester, 'message'),
          'The other program\'s file was not opened, so this document is still '
          'open, and not saved. '
          'Error: FileSystemException: no room, path = \'/tmp\'',
        );
        expect(fake.disposed, isFalse);
        await press(tester, 'keep-mine');
      },
      canBeRead: (_) => throw const FileSystemException('no room', '/tmp'),
    );

    screenTest(
      'open theirs when reading theirs runs into a fault: this one is whole',
      (tester, keeper, fake) async {
        await enterBehind(keeper, 'd', 'mine');
        fake.fileChanged = true;
        await tester.pump(const Duration(seconds: 1));
        await tester.pump();
        await press(tester, 'open-theirs');
        // Not the sentence that says this document cannot be used any more.
        expect(
          line(tester, 'message'),
          'The file as the other program left it cannot be opened, so this '
          'document is still open, and not saved. '
          'Reading it ran into a fault.',
        );
        expect(find.textContaining('builder'), findsNothing);
        expect(key('ended'), findsNothing);
        await press(tester, 'keep-mine');
      },
      canBeRead: (_) => throw PanicException('at /home/builder/core.rs'),
    );

    screenTest(
      'a stage that could not be read again, with the way to try again',
      (tester, keeper, fake) async {
        await press(tester, 'cell-0-0');
        type(tester, 'typed');
        // The read after the entering, not the one of the cell before it.
        var stages = 0;
        fake.before = (call) {
          if (call == 'stage' && stages++ == 0) throw broken;
        };
        await press(tester, 'save');
        // The edit was made, so the editor is closed; what is shown is old.
        expect(key('editor'), findsNothing);
        expect(fake.cells['a']!.text, 'typed');
        expect(shown(tester, '0-0'), 'one');
        expect(
          line(tester, 'trouble'),
          'What is shown may be old. Error (io): the disk is gone',
        );
        await press(tester, 'read-again');
        expect(key('trouble'), findsNothing);
        expect(shown(tester, '0-0'), 'typed');
      },
    );

    screenTest('where the screen was could not be written: said quietly', (
      tester,
      keeper,
      fake,
    ) async {
      // Something of the view state to write: the level that is shown.
      await press(tester, 'cell-0-1');
      await press(tester, 'tool-open');
      await arrive(tester);
      fake.viewFails = true;
      await enterBehind(keeper, 'a', 'typed');
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
      // The document is saved; that is not what the line is about.
      expect((fake.saves, line(tester, 'state')), (1, 'Saved'));
      expect(line(tester, 'trouble'), isNull);
      expect(
        line(tester, 'view-error'),
        startsWith('Where the screen was could not be written down. '),
      );
      fake.viewFails = false;
      await enterBehind(keeper, 'a', 'again');
      await tester.pump(const Duration(seconds: 1));
      await tester.pump();
      expect(line(tester, 'view-error'), isNull);
    });
  });

  group('when the document is no longer in use', () {
    screenTest(
      'after a fault a fixed sentence, and the document is left alone',
      (tester, keeper, fake) async {
        await press(tester, 'cell-0-0');
        type(tester, 'typed');
        failFirst(
          fake,
          'retype',
          PanicException('panicked at /home/builder/wombat/core.rs:12:3'),
        );
        await press(tester, 'save');
        expect(line(tester, 'ended'), faultSentence);
        expect(line(tester, 'state'), 'Closed');
        expect(line(tester, 'title'), 'Wombat');
        expect(find.textContaining('builder'), findsNothing);
        expect(find.textContaining('PanicException'), findsNothing);
        expect(fake.disposed, isTrue);
        // Nothing is left to press that would reach the document.
        for (final gone in [
          'stage-across',
          'editor',
          'up',
          'bar-edit',
          'save',
        ]) {
          expect(key(gone), findsNothing, reason: gone);
        }
        expect(tester.widget<IconButton>(key('undo')).onPressed, isNull);
        expect(tester.widget<IconButton>(key('redo')).onPressed, isNull);
        final before = fake.log.length;
        await awayAndBack(tester, AppLifecycleState.paused);
        await tester.pump(const Duration(seconds: 2));
        expect(fake.log.length, before);
        // The system's back is the system's again, which leaves the app: there
        // is nothing to go up in.
        expect(await back(tester), ['SystemNavigator.pop']);
        expect(fake.log.length, before);
      },
    );

    testWidgets('after a fault, back from a screen that is not the first one', (
      tester,
    ) async {
      final fake = FakeDocument();
      final keeper = Keeper(await Session.over(fake));
      final navigator = GlobalKey<NavigatorState>();
      await tester.pumpWidget(
        MaterialApp(navigatorKey: navigator, home: const Text('below')),
      );
      navigator.currentState!.push(
        MaterialPageRoute<void>(
          builder: (_) => PhoneScreen(
            keeper: keeper,
            variants: ValueNotifier(const Variants()),
            log: ProtoLog(),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await (await keeper.edit('a')).enter('typed');
      await tester.pump();
      failFirst(fake, 'undo', PanicException('at /home/builder/core.rs'));
      await press(tester, 'undo');
      expect(key('ended'), findsOneWidget);
      // The system takes the screen away, and that is all: the screen does not
      // go on to leave the app as from its root.
      final left = await back(tester);
      await tester.pumpAndSettle();
      expect(find.byType(PhoneScreen), findsNothing);
      expect(find.text('below'), findsOneWidget);
      expect(left, isEmpty);
    });

    testWidgets(
      'a root that cannot be read is said, and nothing is left behind',
      (tester) async {
        final fake = FakeDocument();
        fake.cells.remove('root');
        final keeper = Keeper(await Session.over(fake));
        await tester.pumpWidget(phoneApp(keeper));
        await tester.pump();
        expect(
          line(tester, 'stage-error'),
          'The document could not be read. '
          'Error (notFound): no cell root',
        );
        expect(key('stage-across'), findsNothing);
        expect(tester.widget<IconButton>(key('undo')).onPressed, isNull);
        // With nothing to go up in, the system's back is the system's.
        expect(await back(tester), ['SystemNavigator.pop']);
        expect(tester.takeException(), isNull);
        expect(line(tester, 'message'), isNull);
        // The keeper is its owner's to close as before, and the screen leaves
        // nothing with it when it goes.
        await tester.pumpWidget(const SizedBox());
        // ignore: invalid_use_of_protected_member
        expect(keeper.hasListeners, isFalse);
        await keeper.close();
        expect(fake.closed, isTrue);
      },
    );

    testWidgets('a screen on a keeper whose work has ended says why', (
      tester,
    ) async {
      final fake = FakeDocument();
      final keeper = Keeper(await Session.over(fake));
      await keeper.close();
      await tester.pumpWidget(phoneApp(keeper));
      await tester.pump();
      expect(line(tester, 'ended'), 'The document was closed.');
      expect(tester.takeException(), isNull);
    });
  });

  group('the prototype log and its settings', () {
    final clock = DateTime.utc(2026, 11, 12, 10, 30);
    final log = ProtoLog(now: () => clock);
    screenTest(
      'every dive, way up, jump, edit and undo, with the variant settings',
      (tester, keeper, fake) async {
        await dive(tester);
        await press(tester, 'cell-1-0');
        type(tester, 'a secret text');
        await press(tester, 'save');
        await press(tester, 'cell-1-0');
        await press(tester, 'cancel');
        await press(tester, 'undo');
        await press(tester, 'redo');
        await press(tester, 'up');
        await arrive(tester);
        await dive(tester);
        await press(tester, 'trail-0');
        await arrive(tester);
        await dive(tester);
        await back(tester);
        final entries = [
          for (final line in log.lines)
            jsonDecode(line) as Map<String, Object?>,
        ];
        expect(
          [for (final entry in entries) entry['what']],
          [
            'dive',
            'edit',
            'enter',
            'edit',
            'cancel',
            'undo',
            'redo',
            'up',
            'dive',
            'jump',
            'dive',
            'back',
            'up',
          ],
        );
        // When, and under which settings.
        for (final entry in entries) {
          expect(entry['t'], '2026-11-12T10:30:00.000Z');
          expect(entry['variants'], {
            'tapText': 'edit',
            'tapChild': 'dive',
            'pin': true,
            'minText': 11.0,
            'previewScale': 0.7,
          });
        }
        // How deep, and where: the dive from the root, the edit of column 1,
        // row 0 one level down.
        expect((entries[0]['depth'], entries[0]['from']), (1, 0));
        expect(
          (entries[1]['depth'], entries[1]['x'], entries[1]['y']),
          (1, 1, 0),
        );
        expect((entries[7]['depth'], entries[7]['from']), (0, 1));
        // Nothing of what is in the document.
        expect(log.text.contains('secret'), isFalse);
        expect(log.text.contains('g1'), isFalse);
        expect(log.text, endsWith('}\n'));
      },
      before: bigChild,
      log: log,
    );

    testWidgets('the switches change what the screen does, and are logged', (
      tester,
    ) async {
      screenSize(tester, phone);
      final fake = FakeDocument();
      bigChild(fake);
      final keeper = Keeper(await Session.over(fake));
      final variants = ValueNotifier(const Variants());
      final log = ProtoLog();
      final folder = await tester.runAsync(
        () => Directory.systemTemp.createTemp('wombat-log'),
      );
      await tester.pumpWidget(
        phoneApp(
          keeper,
          variants: variants,
          log: log,
          exportFolder: () async => Directory('${folder!.path}/out'),
        ),
      );
      await tester.pump();
      await arrive(tester);
      await choose(tester, 'menu', 'menu-settings');
      // As the specification has them to start with.
      bool on(String name) =>
          tester.widget<SwitchListTile>(key('variant-$name')).value;
      expect(
        (on('tap-text'), on('tap-child'), on('pin')),
        (false, false, true),
      );
      expect(line(tester, 'min-text'), 'Smallest text: 11 pt');
      expect(line(tester, 'preview-scale'), 'Preview text: 70% of full size');
      expect(line(tester, 'log-count'), 'Prototype log: 0 entries');

      await tester.tap(key('variant-tap-text'));
      await tester.tap(key('variant-tap-child'));
      await tester.tap(key('variant-pin'));
      await tester.pump();
      expect(
        variants.value,
        const Variants(tapTextSelects: true, tapChildUnfolds: true, pin: false),
      );
      expect((on('tap-text'), on('tap-child'), on('pin')), (true, true, false));
      await tester.tap(key('min-text-more'));
      await tester.tap(key('preview-scale-less'));
      await tester.tap(key('preview-scale-less'));
      await tester.pump();
      expect((variants.value.minText, variants.value.previewScale), (12, 0.5));
      expect(line(tester, 'min-text'), 'Smallest text: 12 pt');
      expect(line(tester, 'preview-scale'), 'Preview text: 50% of full size');
      // At the end of its range a threshold goes no further.
      expect(
        tester.widget<IconButton>(key('preview-scale-less')).onPressed,
        isNull,
      );
      for (var i = 0; i < 5; i++) {
        await tester.tap(key('preview-scale-more'));
        await tester.pump();
      }
      // In steps of a tenth that add up to exactly one.
      expect(variants.value.previewScale, 1.0);
      expect(line(tester, 'preview-scale'), 'Preview text: 100% of full size');
      expect(
        tester.widget<IconButton>(key('preview-scale-more')).onPressed,
        isNull,
      );
      for (var i = 0; i < 5; i++) {
        await tester.tap(key('preview-scale-less'));
        await tester.pump();
      }
      for (var i = 0; i < 4; i++) {
        await tester.tap(key('min-text-less'));
        await tester.pump();
      }
      expect(variants.value.minText, 8);
      expect(tester.widget<IconButton>(key('min-text-less')).onPressed, isNull);
      for (var i = 0; i < 8; i++) {
        await tester.tap(key('min-text-more'));
        await tester.pump();
      }
      expect(variants.value.minText, 16);
      expect(tester.widget<IconButton>(key('min-text-more')).onPressed, isNull);
      for (var i = 0; i < 4; i++) {
        await tester.tap(key('min-text-less'));
        await tester.pump();
      }
      await tester.tap(key('min-text-less'));
      await tester.tap(key('preview-scale-more'));
      await tester.tap(key('preview-scale-more'));
      await tester.pump();
      expect((variants.value.minText, variants.value.previewScale), (11, 0.7));
      // Every change is in the log, with the settings as they then were.
      expect(line(tester, 'log-count'), 'Prototype log: 35 entries');
      final first = jsonDecode(log.lines.first) as Map<String, Object?>;
      expect(first['what'], 'variant');
      expect((first['variants']! as Map)['tapText'], 'select');

      // The log goes out as a file, or onto the clipboard.
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String;
          }
          return null;
        },
      );
      await tester.ensureVisible(key('log-copy'));
      await tester.tap(key('log-copy'));
      await tester.pump();
      await tester.pump();
      expect(copied, log.text);
      expect(
        tester.widget<SelectableText>(key('log-said')).data,
        'The log is on the clipboard.',
      );
      await tester.ensureVisible(key('log-export'));
      await tester.runAsync(() async {
        await tester.tap(key('log-export'));
        await Future<void>.delayed(const Duration(milliseconds: 200));
      });
      await tester.pump();
      final files = Directory('${folder!.path}/out').listSync();
      expect(files.single.path, endsWith('.jsonl'));
      expect(File(files.single.path).readAsStringSync(), log.text);
      expect(
        tester.widget<SelectableText>(key('log-said')).data,
        'The log is in ${files.single.path}',
      );
      // The sheet goes; the screen does what the switches say.
      await press(tester, 'settings-close');
      await tester.pumpAndSettle();
      expect(key('variant-pin'), findsNothing);
      await press(tester, 'cell-0-0');
      expect(key('editor'), findsNothing);
      await press(tester, 'child-0-1');
      expect(key('full-0-1'), findsOneWidget);
      expect(line(tester, 'title'), 'Root');
      await tester.runAsync(() => folder.delete(recursive: true));
      await endKeeping(keeper);
    });

    testWidgets('without a place for files the log can still be copied', (
      tester,
    ) async {
      screenSize(tester, phone);
      final keeper = Keeper(await Session.over(FakeDocument()));
      await tester.pumpWidget(phoneApp(keeper));
      await tester.pump();
      await choose(tester, 'menu', 'menu-settings');
      expect(key('log-export'), findsNothing);
      expect(key('log-copy'), findsOneWidget);
      await endKeeping(keeper);
    });

    testWidgets('more entries of the menu are the owner\'s to give', (
      tester,
    ) async {
      screenSize(tester, phone);
      final keeper = Keeper(await Session.over(FakeDocument()));
      var chosen = 0;
      await tester.pumpWidget(
        phoneApp(
          keeper,
          menu: [
            PopupMenuItem(
              key: const Key('more'),
              value: () => chosen++,
              child: const Text('More of it'),
            ),
          ],
        ),
      );
      await tester.pump();
      await choose(tester, 'menu', 'more');
      expect(chosen, 1);
      await endKeeping(keeper);
    });
  });
}

/// A keeper with a mistake in it: keeping mine throws what nothing expects.
class _Mistaken extends Keeper {
  _Mistaken(super.doc);

  /// What the mistake waits for, if anything.
  Completer<void>? gate;

  @override
  Future<void> keepMine() async {
    await gate?.future;
    throw StateError('a mistake');
  }
}
