// The parts of the phone screen that are plain functions and small classes: the depth
// budget, the widths of columns, where a cell is in a read, which ranges are styled,
// the direction of a text, the pieces a styled text is drawn in, and the prototype log.
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/phone/budget.dart';
import 'package:wombat_app/phone/cell_view.dart';
import 'package:wombat_app/phone/log.dart';
import 'package:wombat_app/phone/place.dart';
import 'package:wombat_app/phone/variants.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';
import 'package:wombat_app/stage_read.dart';

import 'fake_document.dart';

const _white = Rgb(r: 255, g: 255, b: 255);
const _black = Rgb(r: 0, g: 0, b: 0);

GridSummary summary(int xs, int ys, {int? previewCols, int? shapeCols}) {
  final cols = previewCols ?? (xs < 3 ? xs : 3);
  final rows = ys < 4 ? ys : 4;
  final across = shapeCols ?? (xs < 12 ? xs : 12);
  final down = ys < 20 ? ys : 20;
  return GridSummary(
    owner: 'owner',
    xs: xs,
    ys: ys,
    borderColor: _black,
    previewCols: cols,
    previewRows: rows,
    preview: [
      for (var i = 0; i < cols * rows; i++)
        const CutText(text: 'text', cut: false),
    ],
    shapeCols: across,
    shapeRows: down,
    shape: Uint8List(across * down * 4),
  );
}

CellLook look(
  String text, {
  Style style = plain,
  List<RunInfo> runs = const [],
  List<LinkInfo> links = const [],
}) => CellLook(
  id: 'cell',
  text: text,
  textCut: false,
  kind: CellKind.data,
  resultCut: false,
  failed: false,
  cellColor: _white,
  textColor: _black,
  isTag: false,
  style: style,
  relSize: 0,
  textAlign: HorizAlign.auto,
  vertAlign: VertAlign.auto,
  runs: runs,
  links: links,
  hasNote: false,
  hasGrid: false,
  folded: false,
);

void main() {
  group('the depth budget', () {
    DepthForm form(
      double width,
      GridSummary summary, {
      bool given = true,
      bool unfolded = false,
      bool folded = false,
      double textScale = 1,
      Variants variants = const Variants(),
    }) => formOf(
      width: width,
      summary: summary,
      given: given,
      unfolded: unfolded,
      folded: folded,
      fullCells: 24,
      textScale: textScale,
      variants: variants,
    );

    test(
      'Full where every column is a target wide and the cells are given',
      () {
        expect(form(144, summary(2, 2)), DepthForm.full);
        expect(form(143.9, summary(2, 2)), DepthForm.preview);
        // Not given cell by cell: nothing to draw it from.
        expect(form(400, summary(2, 2), given: false), DepthForm.preview);
        // More cells than a grid may have to be Full by itself: 24 are, 25 not.
        expect(form(2000, summary(4, 6)), DepthForm.full);
        expect(form(2000, summary(5, 5)), DepthForm.preview);
      },
    );

    test('larger text needs wider columns, so a grid steps down sooner', () {
      expect(fullColumn(1), 72);
      expect(fullColumn(0.5), 48);
      expect(fullColumn(2), 144);
      expect(form(150, summary(2, 2)), DepthForm.full);
      expect(form(150, summary(2, 2), textScale: 2), DepthForm.preview);
      // And a Preview's text needs more room too.
      expect(form(111, summary(2, 2), textScale: 2), DepthForm.shape);
      expect(form(288, summary(2, 2), textScale: 2), DepthForm.full);
      expect(form(287, summary(2, 2), textScale: 2), DepthForm.preview);
    });

    test('Preview where its text is readable and has room', () {
      // Three columns of text at 70% of 16: 84 wide at least.
      expect(form(84, summary(5, 5)), DepthForm.preview);
      expect(form(83.9, summary(5, 5)), DepthForm.shape);
      // One column needs less, but never less than a target.
      expect(form(48, summary(1, 30)), DepthForm.preview);
      expect(form(47.9, summary(1, 30)), DepthForm.badge);
      // The smallest text allowed is a setting, and so is the scale.
      expect(
        form(200, summary(5, 5), variants: const Variants(minText: 11.2)),
        DepthForm.preview,
      );
      expect(
        form(200, summary(5, 5), variants: const Variants(minText: 12)),
        DepthForm.shape,
      );
      expect(
        form(200, summary(5, 5), variants: const Variants(previewScale: 0.6)),
        DepthForm.shape,
      );
      expect(
        form(
          200,
          summary(5, 5),
          variants: const Variants(previewScale: 0.6, minText: 9),
        ),
        DepthForm.preview,
      );
      // A summary without texts has no Preview.
      expect(form(200, summary(5, 5, previewCols: 0)), DepthForm.shape);
    });

    test(
      'Shape where there is a target\'s width, Badge where there is not',
      () {
        const small = Variants(minText: 16);
        expect(form(48, summary(5, 5), variants: small), DepthForm.shape);
        expect(form(47.9, summary(5, 5), variants: small), DepthForm.badge);
        // Each of its columns needs three pixels: twenty of them, sixty.
        expect(
          form(60, summary(30, 2, shapeCols: 20), variants: small),
          DepthForm.shape,
        );
        expect(
          form(59.9, summary(30, 2, shapeCols: 20), variants: small),
          DepthForm.badge,
        );
        expect(
          form(200, summary(5, 5, shapeCols: 0), variants: small),
          DepthForm.badge,
        );
      },
    );

    test('unfolded is Full whatever its size; folded is a Badge', () {
      expect(form(10, summary(12, 20), unfolded: true), DepthForm.full);
      // Without its cells there is nothing to unfold.
      expect(
        form(200, summary(12, 20), unfolded: true, given: false),
        DepthForm.preview,
      );
      expect(form(400, summary(2, 2), folded: true), DepthForm.badge);
      expect(
        form(400, summary(2, 2), folded: true, unfolded: true),
        DepthForm.badge,
      );
    });
  });

  group('the widths of the stage\'s columns', () {
    List<double> widths(List<int> units, double viewport, [double scale = 1]) =>
        columnWidths(
          Uint32List.fromList(units),
          viewport: viewport,
          textScale: scale,
        );

    test('few columns fill the width that the bar leaves', () {
      expect(widths([80], 400), [352]);
      expect(widths([80, 80], 400), [176, 176]);
      expect(widths([80, 80, 80], 378), [110, 110, 110]);
      expect(widths([], 400), isEmpty);
    });

    test('of many, a phone shows three to four', () {
      expect(widths(List.filled(9, 80), 400), List.filled(9, 110.0));
      // Larger text, wider columns, up to half as wide again.
      expect(widths(List.filled(9, 80), 400, 1.2), List.filled(9, 132.0));
      expect(widths(List.filled(9, 80), 400, 3), List.filled(9, 165.0));
      expect(widths(List.filled(9, 80), 400, 0.5), List.filled(9, 110.0));
    });

    test(
      'the document\'s own widths keep their proportions, within bounds',
      () {
        expect(widths([80, 160, 40], 378), [110, 220, 72]);
        // Never narrower than a target and a half, never wider than the screen.
        expect(widths([8, 800, 80], 378), [72, 330, 110]);
        expect(widths([80], 100), [110]);
      },
    );
  });

  group('where a cell is in a read', () {
    late StageRead stage;
    setUp(() async {
      final doc = await Session.over(FakeDocument());
      stage = StageRead('root', phoneAsk, await doc.stage('root', phoneAsk));
      await doc.discard();
    });

    test('in the stage\'s grid, or in a Full grid inside it', () {
      final d = placeOf(stage, 'd')!;
      expect((d.part.owner, d.part.level, d.x, d.y), ('root', 0, 1, 1));
      final c = placeOf(stage, 'c')!;
      expect((c.x, c.y), (0, 1));
      final f = placeOf(stage, 'f')!;
      expect((f.part.owner, f.part.level, f.x, f.y), ('c', 1, 0, 1));
      expect(placeOf(stage, 'root'), isNull);
      expect(placeOf(stage, 'nowhere'), isNull);
    });

    test('the grid of a cell as it is given', () {
      expect(partOf(stage, 'root'), stage.grid);
      expect(partOf(stage, 'c')!.owner, 'c');
      expect(partOf(stage, 'a'), isNull);
    });
  });

  group('which ranges are styled', () {
    const b = bold;
    RunInfo run(int start, int len, Style style) =>
        RunInfo(start: start, len: len, style: style);
    bool isBold(Style style) => style.bold;

    test('by the base style where no run says otherwise', () {
      expect(allStyled(look('abcdef'), 0, 6, isBold), isFalse);
      expect(allStyled(look('abcdef', style: b), 0, 6, isBold), isTrue);
      // A range without text is what the cell is.
      expect(allStyled(look('abcdef', style: b), 3, 3, isBold), isTrue);
      expect(allStyled(look('abcdef'), 3, 3, isBold), isFalse);
      expect(
        allStyled(look('abcdef', runs: [run(0, 6, b)]), 3, 3, isBold),
        isFalse,
      );
    });

    test('by the runs where they cover the range', () {
      final some = look('abcdef', runs: [run(1, 2, b), run(3, 2, b)]);
      expect(allStyled(some, 1, 5, isBold), isTrue);
      expect(allStyled(some, 2, 4, isBold), isTrue);
      expect(allStyled(some, 0, 5, isBold), isFalse);
      expect(allStyled(some, 1, 6, isBold), isFalse);
      // A gap between two runs is the base style's.
      final gap = look('abcdef', runs: [run(0, 2, b), run(4, 2, b)]);
      expect(allStyled(gap, 0, 6, isBold), isFalse);
      expect(allStyled(gap, 0, 2, isBold), isTrue);
      expect(allStyled(gap, 4, 6, isBold), isTrue);
      // A run that switches the bit off inside a bold cell.
      final off = look('abcdef', style: b, runs: [run(2, 2, plain)]);
      expect(allStyled(off, 0, 6, isBold), isFalse);
      expect(allStyled(off, 0, 2, isBold), isTrue);
      expect(allStyled(off, 4, 6, isBold), isTrue);
      expect(allStyled(off, 3, 5, isBold), isFalse);
      // Another bit is another question.
      expect(allStyled(some, 1, 5, (style) => style.italic), isFalse);
    });

    test('the selection of an editor as a range of code points', () {
      final held = look('a😀b');
      expect(rangeIn(held, (start: 0, end: 4)), (start: 0, end: 3));
      expect(rangeIn(held, (start: 3, end: 4)), (start: 2, end: 3));
      // Past the end of what the document kept: its end.
      expect(rangeIn(held, (start: 1, end: 99)), (start: 1, end: 3));
    });

    test('what is asked of a read for an unfolded grid', () {
      final ask = askFull(240);
      expect(ask.fullCells, 240);
      expect(
        (ask.x0, ask.y0, ask.cols, ask.rows, ask.fullDepth),
        (
          phoneAsk.x0,
          phoneAsk.y0,
          phoneAsk.cols,
          phoneAsk.rows,
          phoneAsk.fullDepth,
        ),
      );
      expect(
        (ask.previewCols, ask.previewRows, ask.shapeCols, ask.shapeRows),
        (
          phoneAsk.previewCols,
          phoneAsk.previewRows,
          phoneAsk.shapeCols,
          phoneAsk.shapeRows,
        ),
      );
      expect(
        (ask.textCut, ask.previewCut),
        (phoneAsk.textCut, phoneAsk.previewCut),
      );
      expect(askFull(phoneAsk.fullCells), phoneAsk);
    });
  });

  group('how a text is drawn', () {
    test('its direction is that of its first letter', () {
      expect(directionOf('abc'), TextDirection.ltr);
      expect(directionOf(''), TextDirection.ltr);
      expect(directionOf('123 (!)'), TextDirection.ltr);
      expect(directionOf('שלום'), TextDirection.rtl);
      expect(directionOf('مرحبا'), TextDirection.rtl);
      expect(directionOf('12. שלום abc'), TextDirection.rtl);
      expect(directionOf('abc שלום'), TextDirection.ltr);
      // The edges of the ranges: the first Hebrew block and the last Arabic
      // presentation form are in, their neighbours out.
      expect(directionOf('֐'), TextDirection.rtl);
      expect(directionOf('֏'), TextDirection.ltr);
      expect(directionOf('ࣿ'), TextDirection.rtl);
      expect(directionOf('ऀ'), TextDirection.ltr);
      expect(directionOf('יִ'), TextDirection.rtl);
      expect(directionOf('﬜'), TextDirection.ltr);
      expect(directionOf('﷿'), TextDirection.rtl);
      expect(directionOf('ﹰ'), TextDirection.rtl);
      expect(directionOf('﻿'), TextDirection.rtl);
      expect(directionOf('Ａ'), TextDirection.ltr);
      expect(directionOf('\u{10800}'), TextDirection.rtl);
      expect(directionOf('\u{1E900}'), TextDirection.rtl);
      expect(directionOf('\u{1F600} שלום'), TextDirection.ltr);
      // General punctuation decides nothing; a letter right after it does.
      expect(directionOf('—… שלום'), TextDirection.rtl);
      expect(directionOf('⁰ שלום'), TextDirection.ltr);
      expect(directionOf('@ שלום'), TextDirection.rtl);
      expect(directionOf('A שלום'), TextDirection.ltr);
    });

    List<(String, bool, bool, Color?)> pieces(CellLook look) => [
      for (final span in spansOf(look.text, look, const Color(0xFF0000FF)))
        (
          (span as TextSpan).text!,
          span.style?.fontWeight == FontWeight.bold,
          span.style?.decoration == TextDecoration.underline,
          span.style?.color,
        ),
    ];

    test('in one piece without runs or links', () {
      expect(pieces(look('plain')), [('plain', false, false, null)]);
    });

    test(
      'in pieces where runs and links begin and end, counted in code points',
      () {
        const red = Color(0xFFFF0000);
        const blue = Color(0xFF0000FF);
        final styled = look(
          'a😀bcdef',
          runs: [
            const RunInfo(start: 1, len: 2, style: bold),
            const RunInfo(
              start: 4,
              len: 1,
              style: plain,
              color: Rgb(r: 255, g: 0, b: 0),
            ),
          ],
          links: [
            const LinkInfo(
              start: 2,
              len: 3,
              target: LinkTarget.external_(target: 'https://example.org'),
            ),
          ],
        );
        expect(pieces(styled), [
          ('a', false, false, null),
          ('😀', true, false, null),
          ('b', true, true, blue),
          ('c', false, true, blue),
          ('d', false, true, blue),
          ('ef', false, false, null),
        ]);
        // A run's colour where no link is.
        final coloured = look(
          'abc',
          runs: [
            const RunInfo(
              start: 1,
              len: 1,
              style: plain,
              color: Rgb(r: 255, g: 0, b: 0),
            ),
          ],
        );
        expect(pieces(coloured)[1], ('b', false, false, red));
      },
    );

    test('a run says the whole style of its range', () {
      final off = look(
        'abcd',
        style: bold,
        runs: [const RunInfo(start: 1, len: 2, style: italic)],
      );
      final spans = spansOf(off.text, off, const Color(0xFF0000FF));
      final middle = (spans[1] as TextSpan).style!;
      expect(middle.fontWeight, FontWeight.normal);
      expect(middle.fontStyle, FontStyle.italic);
      expect((spans[0] as TextSpan).style!.fontWeight, isNull);
    });

    testWidgets('the ranges are of the text, not of what a formula shows', (
      tester,
    ) async {
      // A formula cell whose text has a bold range: its result is one piece.
      final formula = look(
        '=1+2+3',
        runs: [const RunInfo(start: 0, len: 2, style: bold)],
      ).copyWith(result: '6');
      await tester.pumpWidget(MaterialApp(home: CellText(look: formula)));
      final text = tester.widget<Text>(find.byType(Text));
      final spans = (text.textSpan! as TextSpan).children!;
      expect(spans.length, 1);
      expect((spans.single as TextSpan).text, '6');
      expect((spans.single as TextSpan).style, isNull);
    });

    testWidgets('a Shape is at least a target tall and never taller than 160', (
      tester,
    ) async {
      Future<Size> drawn(GridSummary of, double width) async {
        await tester.pumpWidget(
          Center(
            child: ShapeView(summary: of, width: width),
          ),
        );
        return tester.getSize(find.byType(CustomPaint).last);
      }

      // Twenty rows of twelve each would be 240.
      expect(await drawn(summary(12, 20), 300), const Size(300, 160));
      // One row of four would be 4.
      expect(await drawn(summary(12, 1), 96), const Size(96, 40));
      // In between, half as tall as its columns are wide.
      expect(await drawn(summary(5, 10), 100), const Size(100, 100));
    });

    test('ranges that are not in the text give the text plain', () {
      final wrong = look(
        'ab',
        runs: [const RunInfo(start: 1, len: 9, style: bold)],
      );
      expect(pieces(wrong), [('ab', false, false, null)]);
    });

    testWidgets('a cell\'s base style, its size and its alignment', (
      tester,
    ) async {
      Future<Text> drawn(
        CellLook look, {
        String? typed,
        CellLook? styled,
      }) async {
        await tester.pumpWidget(
          MaterialApp(
            home: CellText(look: look, typed: typed, styled: styled),
          ),
        );
        return tester.widget<Text>(find.byType(Text));
      }

      const all = Style(
        bold: true,
        italic: true,
        fixed: true,
        underline: true,
        strike: true,
      );
      var text = await drawn(look('styled', style: all));
      expect(text.style!.fontWeight, FontWeight.bold);
      expect(text.style!.fontStyle, FontStyle.italic);
      expect(text.style!.fontFamily, 'monospace');
      expect(
        text.style!.decoration,
        TextDecoration.combine([
          TextDecoration.underline,
          TextDecoration.lineThrough,
        ]),
      );
      expect(text.style!.fontSize, 16);
      expect(text.textAlign, TextAlign.start);
      text = await drawn(
        look('plain').copyWith(
          relSize: 3,
          textAlign: HorizAlign.center,
          textColor: const Rgb(r: 1, g: 2, b: 3),
        ),
      );
      expect(text.style!.fontWeight, isNull);
      expect(text.style!.decoration, isNull);
      expect(text.style!.fontFamily, isNull);
      expect(text.style!.fontSize, 13);
      expect(text.style!.color, const Color(0xFF010203));
      expect(text.textAlign, TextAlign.center);
      // The size against the usual one has its bounds.
      text = await drawn(look('tiny').copyWith(relSize: 20));
      expect(text.style!.fontSize, 8);
      text = await drawn(look('huge').copyWith(relSize: -40));
      expect(text.style!.fontSize, 40);
      text = await drawn(look('left').copyWith(textAlign: HorizAlign.left));
      expect(text.textAlign, TextAlign.left);
      text = await drawn(look('right').copyWith(textAlign: HorizAlign.right));
      expect(text.textAlign, TextAlign.right);
      text = await drawn(
        look('underlined').copyWith(
          style: const Style(
            bold: false,
            italic: false,
            fixed: false,
            underline: true,
            strike: false,
          ),
        ),
      );
      expect(text.style!.decoration, TextDecoration.underline);
      // A text that was cut says so.
      text = await drawn(look('cut he').copyWith(textCut: true));
      expect(text.textSpan!.toPlainText(), 'cut he…');
      text = await drawn(look('=1').copyWith(result: '12345', resultCut: true));
      expect(text.textSpan!.toPlainText(), '12345…');
      // What is typed is shown as typed: not cut, not as a result, and in the
      // styles of the held cell only while that has the very same text.
      final held = look(
        'typed',
        runs: [const RunInfo(start: 0, len: 2, style: bold)],
      );
      text = await drawn(
        look('=1').copyWith(result: '1', textCut: true, failed: true),
        typed: 'typed',
        styled: held,
      );
      expect(text.textSpan!.toPlainText(), 'typed');
      expect(text.style!.fontStyle, isNull);
      expect(text.style!.color, const Color(0xFF000000));
      expect(
        ((text.textSpan! as TextSpan).children!.first as TextSpan)
            .style!
            .fontWeight,
        FontWeight.bold,
      );
      text = await drawn(look('one'), typed: 'typed more');
      expect((text.textSpan! as TextSpan).children!.length, 1);
    });
  });

  group('the settings of the prototype', () {
    test(
      'start as the specification has them, and keep within their ranges',
      () {
        const v = Variants();
        expect(
          (v.tapTextSelects, v.tapChildUnfolds, v.pin),
          (false, false, true),
        );
        expect((v.minText, v.previewScale), (11, 0.7));
        expect(v.copyWith(minText: 99).minText, 16);
        expect(v.copyWith(minText: 1).minText, 8);
        expect(v.copyWith(previewScale: 9).previewScale, 1);
        expect(v.copyWith(previewScale: 0.1).previewScale, 0.5);
        expect(v.copyWith(), v);
        expect(v.copyWith().hashCode, v.hashCode);
        for (final other in [
          v.copyWith(tapTextSelects: true),
          v.copyWith(tapChildUnfolds: true),
          v.copyWith(pin: false),
          v.copyWith(minText: 12),
          v.copyWith(previewScale: 0.8),
        ]) {
          expect(other, isNot(v));
        }
        expect(
          v
              .copyWith(
                tapTextSelects: true,
                tapChildUnfolds: true,
                pin: false,
                minText: 12,
                previewScale: 0.8,
              )
              .toJson(),
          {
            'tapText': 'select',
            'tapChild': 'unfold',
            'pin': false,
            'minText': 12.0,
            'previewScale': 0.8,
          },
        );
      },
    );
  });

  group('the prototype log', () {
    late Directory folder;
    setUp(() => folder = Directory.systemTemp.createTempSync('wombat-log'));
    tearDown(() => folder.deleteSync(recursive: true));
    // The clock of the device, which is not on universal time.
    final local = DateTime(2026, 11, 12, 10, 30, 5);
    final at = local.toUtc();

    test('one line of JSON for each entry, added to its file as it is made', () async {
      final file = File('${folder.path}/log.jsonl');
      final log = ProtoLog(file: file, now: () => local);
      var told = 0;
      log.addListener(() => told++);
      log.add('dive', const Variants(), {'depth': 2});
      log.add('undo', const Variants(pin: false));
      await log.written();
      expect(told, 2);
      expect(log.lines.length, 2);
      expect(file.readAsStringSync(), log.text);
      expect(jsonDecode(log.lines.first), {
        't': at.toIso8601String(),
        'what': 'dive',
        'depth': 2,
        'variants': const Variants().toJson(),
      });
      expect((jsonDecode(log.lines.last) as Map)['variants']['pin'], isFalse);
      expect(log.text, '${log.lines[0]}\n${log.lines[1]}\n');
      // The next run finds them, and goes on after them.
      // A line left empty by something else is passed over.
      file.writeAsStringSync('\n', mode: FileMode.append);
      final next = ProtoLog(file: file, now: () => at);
      final loading = next.load();
      // Added while the file is read: after what was there, and not read twice.
      next.add('first of this run', const Variants());
      await loading;
      expect(next.lines.length, 3);
      expect(next.lines.take(2), log.lines);
      await next.written();
      expect(file.readAsLinesSync().where((line) => line.isNotEmpty).length, 3);
    });

    test('in memory alone without a file, or with one that cannot be written', () async {
      final none = ProtoLog();
      none.add('dive', const Variants());
      await none.load();
      await none.written();
      expect(none.lines.length, 1);
      // A folder where the file should be: nothing is thrown, the entry stays.
      final taken = ProtoLog(file: File(folder.path));
      await taken.load();
      taken.add('dive', const Variants());
      await taken.written();
      expect(taken.lines.length, 1);
      // And no file at all yet.
      final fresh = ProtoLog(file: File('${folder.path}/none/log.jsonl'));
      await fresh.load();
      expect(fresh.lines, isEmpty);
    });

    test('keeps the newest entries when there are too many', () async {
      final file = File('${folder.path}/log.jsonl');
      file.writeAsStringSync(
        [for (var i = 0; i < ProtoLog.most + 10; i++) '{"n":$i}\n'].join(),
      );
      final log = ProtoLog(file: file);
      await log.load();
      expect(log.lines.length, ProtoLog.most);
      expect(log.lines.first, '{"n":10}');
      log.add('one more', const Variants());
      expect(log.lines.length, ProtoLog.most);
      expect(log.lines.first, '{"n":11}');
    });

    test('is exported as a file of its own, named by the time', () async {
      final log = ProtoLog(now: () => at);
      log.add('dive', const Variants());
      final out = await log.export(Directory('${folder.path}/a/b'));
      final stamp = at.toIso8601String().replaceAll(RegExp('[^0-9T]'), '');
      expect(out.path, '${folder.path}/a/b/wombat-log-$stamp.jsonl');
      expect(out.readAsStringSync(), log.text);
      // Nothing of the log's own file is needed for that.
      expect(folder.listSync().length, 1);
    });
  });
}
