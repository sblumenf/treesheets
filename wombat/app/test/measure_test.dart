// The measuring of `lib/measure.dart` on a small document, without a device, so that its
// steps are tested on every push; the numbers mean nothing here.
import 'dart:async';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/measure.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'support.dart';

void main() {
  late Directory folder;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-measure-test');
  });

  tearDownAll(() => folder.delete(recursive: true));

  test('measures a small document, and again with the file it left', () async {
    const small = [(3, 2), (2, 1), (1, 1)];
    final first = await measure(
      folder,
      across: 2,
      down: 2,
      runs: 3,
      nested: small,
    );
    expect(first.cells, 405);
    expect(first.built, isTrue);
    expect(first.build, isNotNull);
    expect(first.recalculates, isTrue);
    expect(first.fileBytes, greaterThan(405 * 80));
    expect(first.timings.map((t) => t.what), [
      'open',
      'read the root grid (4 cells)',
      'read an inner grid (100 cells)',
      'stage of the root, whole (4 cells, 400 shape cells)',
      '  of which the Dart thread was held',
      '  and comparing it with the read before',
      'stage of the root, 40 rows (4 cells, 400 shape cells)',
      '  of which the Dart thread was held',
      '  and comparing it with the read before',
      'set the text of one cell',
      'undo that',
      'insert a row in the root grid',
      'undo that',
      'set the text of one cell, no formula',
      'undo that',
      'insert a row in the root grid, no formula',
      'undo that',
      'save',
      // The nested one: the root holds 3 by 2. Its first cell holds 3 by 2 again
      // (the first of a level is as large as the root), in which the first holds
      // 3 by 2 and the fourth 1 by 1; its fourth holds 2 by 1, whose first holds
      // 1 by 1. With the root: 1 + 6 + 6 + 6 + 1 + 2 + 1.
      'nested, 23 cells: stage of the root (14 cells, 16 shape cells)',
      '  of which the Dart thread was held',
      '  and comparing it with the read before',
      'nested: the same, two levels cell by cell (22 cells, 16 shape cells)',
      '  of which the Dart thread was held',
      '  and comparing it with the read before',
    ]);
    for (final timing in first.timings) {
      expect(timing.runs.length, 3);
      expect(timing.median, lessThanOrEqualTo(timing.worst));
      expect(timing.runs.every((d) => d >= Duration.zero), isTrue);
    }
    expect(first.rssBefore, greaterThan(0));
    expect(first.sampledPeak, greaterThanOrEqualTo(first.rssOpen));
    // The system counts the peak and the present size apart, so they are not compared.
    expect(first.maxRss, greaterThan(0));
    final report = first.report();
    expect(report, contains('document: 405 cells, ${first.fileBytes} bytes'));
    expect(report, contains('built in this run'));
    expect(report, contains('a formula that reads every cell: yes'));
    expect(report, contains('peak of the process (maxRss)'));

    // The file is as it was built: the edits were undone or went to a copy.
    final file = File('${folder.path}/large-2x2.wombat');
    final before = file.readAsBytesSync();
    final doc = await Session.open(file.path);
    final root = await doc.grid(doc.root);
    expect((root.cell.text, root.xs, root.ys), ('Large', 2, 2));
    expect(root.cells[1].text, 'Section 1');
    final last = await doc.grid(root.cells[3].id);
    expect(last.cells[98].text, 'Item 3.98 with some ordinary text in it');
    expect(
      (last.cells[99].text, last.cells[99].result),
      ('=TREESUM(Large)', '0'),
    );
    await doc.close();

    final second = await measure(
      folder,
      across: 2,
      down: 2,
      runs: 2,
      nested: small,
    );
    expect(second.built, isFalse);
    expect(second.build, isNull);
    expect(second.fileBytes, first.fileBytes);
    expect(second.report(), contains('the file was there from an earlier run'));
    expect(file.readAsBytesSync(), before);
    expect(File('${folder.path}/work.wombat').existsSync(), isFalse);
  });

  test('a document that cannot be opened leaves no clock running', () async {
    final broken = await folder.createTemp('broken');
    File('${broken.path}/large-2x2.wombat').writeAsStringSync('no document');
    // Every timer that the measuring starts, to see what became of it.
    final timers = <Timer>[];
    await runZoned(
      () => expectLater(
        measure(broken, across: 2, down: 2, runs: 1),
        throwsA(
          isA<BridgeError>().having((e) => e.kind, 'kind', ErrorKind.syntax),
        ),
      ),
      zoneSpecification: ZoneSpecification(
        createPeriodicTimer: (self, parent, zone, period, tick) {
          final timer = parent.createPeriodicTimer(zone, period, tick);
          timers.add(timer);
          return timer;
        },
      ),
    );
    expect(timers.length, 1);
    expect(timers.single.isActive, isFalse);
  });

  test('the nested document of the measurement is the core\'s', () {
    final table = nestedTable(sixLevels);
    // With the root, the 68,749 cells of the bridge's own test.
    expect(table.cells.length + 1, 68749);
    expect(table.xs, 12);
    // The first cell holds 12 by 20, as does its own first cell.
    expect(table.cells[0].grid, const GridSize(xs: 12, cells: 240));
    expect(table.cells[1].grid, const GridSize(xs: 12, cells: 240));
    expect(table.cells[1].text, 'Level 1 cell 0.0 text');
    // The innermost level has no grids.
    expect(
      table.cells
          .where((c) => c.text.startsWith('Level 5'))
          .every((c) => c.grid == null),
      isTrue,
    );
    expect(windowOf(40, y0: 3).rows, 40);
    expect(windowOf(40, y0: 3).y0, 3);
    expect(windowOf(7, fullDepth: 2).fullDepth, 2);
    expect(windowOf(4294967295), phoneAsk);
  });

  test('how long a thread is held during a call', () async {
    // Held for 30 ms in the middle of the call, as decoding an answer holds it; the
    // longest wait counts, not the last.
    final (answer, held) = await heldDuring(() async {
      await Future<void>.delayed(const Duration(milliseconds: 5));
      final watch = Stopwatch()..start();
      while (watch.elapsedMilliseconds < 30) {}
      await Future<void>.delayed(const Duration(milliseconds: 20));
      return 7;
    });
    expect(answer, 7);
    expect(held, greaterThanOrEqualTo(const Duration(milliseconds: 30)));
    // Held to the very end of the call, as it is when the answer has just been
    // decoded: counted too, though no turn of the timer comes after it.
    final (_, last) = await heldDuring(() async {
      await Future<void>.delayed(const Duration(milliseconds: 5));
      final watch = Stopwatch()..start();
      while (watch.elapsedMilliseconds < 30) {}
    });
    expect(last, greaterThanOrEqualTo(const Duration(milliseconds: 30)));
    // Not held at all: the call waits, and the thread is free meanwhile.
    final (_, free) = await heldDuring(
      () => Future<void>.delayed(const Duration(milliseconds: 60)),
    );
    expect(free, lessThan(const Duration(milliseconds: 30)));
    // A call that fails leaves no timer running, and fails; nor does one that ends
    // well. Every timer that is made is counted.
    var timers = 0;
    Future<void> counted(Future<void> Function() call) => runZoned(
      call,
      zoneSpecification: ZoneSpecification(
        createTimer: (self, parent, zone, duration, callback) {
          timers++;
          return parent.createTimer(zone, duration, callback);
        },
      ),
    );
    await counted(
      () => expectLater(
        heldDuring<void>(() async => throw StateError('no')),
        throwsStateError,
      ),
    );
    await counted(() => heldDuring(() async => 1));
    final made = timers;
    await Future<void>.delayed(const Duration(milliseconds: 20));
    expect(timers, made);
  });

  test('median and worst of a timing', () {
    Duration ms(int n) => Duration(milliseconds: n);
    final odd = Timing('x', [ms(5), ms(1), ms(9)]);
    expect((odd.median, odd.worst), (ms(5), ms(9)));
    final even = Timing('x', [ms(4), ms(1), ms(9), ms(2)]);
    expect((even.median, even.worst), (ms(4), ms(9)));
  });
}
