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
    final first = await measure(folder, across: 2, down: 2, runs: 3);
    expect(first.cells, 405);
    expect(first.built, isTrue);
    expect(first.build, isNotNull);
    expect(first.recalculates, isTrue);
    expect(first.fileBytes, greaterThan(405 * 80));
    expect(first.timings.map((t) => t.what), [
      'open',
      'read the root grid (4 cells)',
      'read an inner grid (100 cells)',
      'set the text of one cell',
      'undo that',
      'insert a row in the root grid',
      'undo that',
      'set the text of one cell, no formula',
      'undo that',
      'insert a row in the root grid, no formula',
      'undo that',
      'save',
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

    final second = await measure(folder, across: 2, down: 2, runs: 2);
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

  test('median and worst of a timing', () {
    Duration ms(int n) => Duration(milliseconds: n);
    final odd = Timing('x', [ms(5), ms(1), ms(9)]);
    expect((odd.median, odd.worst), (ms(5), ms(9)));
    final even = Timing('x', [ms(4), ms(1), ms(9), ms(2)]);
    expect((even.median, even.worst), (ms(4), ms(9)));
  });
}
