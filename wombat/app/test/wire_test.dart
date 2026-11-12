// What crosses the bridge from Dart is checked on the other side, not cut to fit: these
// call the real library, which must be built first, from `wombat/`:
// cargo build -p wombat_bridge
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/src/rust/api/document.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'support.dart';

void main() {
  late Directory folder;

  setUpAll(() async {
    await loadBridge();
    folder = await Directory.systemTemp.createTemp('wombat-wire');
  });

  tearDownAll(() => folder.delete(recursive: true));

  /// A new document whose root has a grid of one column and two rows.
  Future<OpenDocument> twoRows(String name) async {
    final doc = await OpenDocument.create(path: '${folder.path}/$name.wombat');
    final root = await doc.root();
    await doc.apply(command: Command.createGrid(cell: root, xs: 1, ys: 2));
    return doc;
  }

  Future<ErrorKind?> refusal(OpenDocument doc, Command command) async {
    try {
      await doc.apply(command: command);
      return null;
    } on BridgeError catch (error) {
      return error.kind;
    }
  }

  test('a number that does not fit is refused, not cut to fit', () async {
    final doc = await twoRows('numbers');
    final root = await doc.root();
    Future<List<String>> rows() async =>
        (await doc.grid(cell: root)).cells.map((cell) => cell.id).toList();
    final before = await rows();

    // 2^32 would be row 0 if only its low 32 bits crossed.
    const big = 4294967296;
    final commands = [
      Command.insertRows(cell: root, at: big, count: 1),
      Command.insertRows(cell: root, at: 0, count: big + 1),
      Command.insertRows(cell: root, at: -1, count: 1),
      Command.setProps(cells: [root], props: [const Prop.relSize(big + 3)]),
      Command.setProps(
        cells: [root],
        props: [const Prop.cellColor(Rgb(r: 256, g: 0, b: 0))],
      ),
    ];
    for (final command in commands) {
      expect(
        await refusal(doc, command),
        ErrorKind.badRequest,
        reason: '$command',
      );
    }
    expect(await rows(), before);

    // What fits still does what it says.
    await doc.apply(command: Command.insertRows(cell: root, at: 2, count: 1));
    expect((await rows()).sublist(0, 2), before);
    expect((await doc.grid(cell: root)).ys, 3);
  });
}
