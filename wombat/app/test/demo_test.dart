// The steps of the bare app without a device: open the example, read the root grid, set
// the text of one cell, undo. It calls the real core through the bridge, which
// `loadBridge` builds first.
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/demo.dart';
import 'package:wombat_app/src/rust/api/document.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'support.dart';

void main() {
  late Directory folder;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-test');
  });

  tearDownAll(() => folder.delete(recursive: true));

  test('open, read, change and undo', () async {
    final demo = Demo();
    final example = await File('assets/example.wombat').readAsBytes();

    final opened = await demo.open(folder, example);
    expect(opened, contains('"Example"  (2 x 3)'));
    expect(opened, contains('  "Costs"\n    "3"\n    "2"\n    "5"'));
    expect(opened, contains('"=SUM(Costs)" = "10"'));

    final changed = await demo.change();
    expect(changed, contains('Set the first cost to 31'));
    expect(changed, contains('  changed: '));
    expect(changed, contains('  recalculated: '));
    expect(changed, contains('    "31"\n    "2"\n    "5"'));
    expect(changed, contains('"=SUM(Costs)" = "38"'));

    final undone = await demo.undo();
    expect(undone, startsWith('Undone\nchange report\n  changed: '));
    expect(undone, contains('    "3"\n    "2"\n    "5"'));
    expect(undone, contains('"=SUM(Costs)" = "10"'));

    expect(await demo.undo(), startsWith('Nothing to undo'));
  });

  test('the copy that is opened has IDs of its own', () async {
    final example = File('assets/example.wombat');
    final original = await OpenDocument.open(path: example.path);
    final copy = await OpenDocument.openCopy(
      from: example.path,
      to: '${folder.path}/copy.wombat',
    );
    expect(await copy.id(), isNot(await original.id()));
    expect(await copy.root(), isNot(await original.root()));
  });

  test('errors arrive as values with a kind', () async {
    final missing = '${folder.path}/none.wombat';
    try {
      await OpenDocument.open(path: missing);
      fail('opened a file that is not there');
    } on BridgeError catch (error) {
      expect(error.kind, ErrorKind.io);
      expect(error.path, missing);
      expect(describeError(error), startsWith('Error (io): '));
    }

    final doc = await OpenDocument.create(path: '${folder.path}/new.wombat');
    final root = await doc.root();
    try {
      await doc.apply(command: Command.removeGrid(cell: root));
      fail('removed the grid of the root');
    } on BridgeError catch (error) {
      expect(error.kind, ErrorKind.refused);
    }
    try {
      await doc.grid(cell: 'no ID');
      fail('read a cell that has no ID');
    } on BridgeError catch (error) {
      expect(error.kind, ErrorKind.badRequest);
    }
    // The document is as it was: a root without a grid.
    final view = await doc.grid(cell: root);
    expect((view.xs, view.ys, view.cells.length), (0, 0, 0));
  });
}
