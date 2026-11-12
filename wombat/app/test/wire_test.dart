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

  test('a table is a flat list, and one nested too deep is refused', () async {
    final doc = await OpenDocument.create(path: '${folder.path}/table.wombat');
    final root = await doc.root();
    Future<Report> import(List<EntryData> cells) => doc.apply(
      command: Command.import_(
        cell: root,
        table: TableData(xs: 1, cells: cells),
        replace: true,
      ),
    );
    const inside = EntryData(
      text: 'x',
      formula: false,
      grid: GridSize(xs: 1, cells: 1),
    );
    const last = EntryData(text: 'x', formula: false);

    // A hundred thousand grids, each inside the one before: an error, not a crash.
    try {
      await import([...List.filled(100000, inside), last]);
      fail('imported a table nested 100,000 deep');
    } on BridgeError catch (error) {
      expect(error.kind, ErrorKind.limit);
    }
    // A grid that says it has more cells than follow.
    try {
      await import([inside]);
      fail('imported a table with a grid that has no cells');
    } on BridgeError catch (error) {
      expect(error.kind, ErrorKind.badRequest);
    }
    expect((await doc.grid(cell: root)).cell.hasGrid, false);

    // a, with a grid of c and d inside, and b.
    await import(const [
      EntryData(text: 'a', formula: false, grid: GridSize(xs: 2, cells: 2)),
      EntryData(text: 'c', formula: false),
      EntryData(text: 'd', formula: false),
      EntryData(text: 'b', formula: false),
    ]);
    final outer = await doc.grid(cell: root);
    expect(outer.cells.map((cell) => cell.text), ['a', 'b']);
    expect((outer.xs, outer.ys), (1, 2));
    final inner = await doc.grid(cell: outer.cells[0].id);
    expect(inner.cells.map((cell) => cell.text), ['c', 'd']);
    expect((inner.xs, inner.ys), (2, 1));
  });

  // The JSON form refuses a field that is left out; between Dart and Rust there are
  // no names, and null is none as it always was.
  test('a field that is none crosses as none', () async {
    final doc = await twoRows('none');
    final cell = (await doc.grid(cell: await doc.root())).cells[0].id;
    await doc.apply(
      command: Command.setText(cell: cell, text: 'ab'),
    );
    Command color(Rgb? color) =>
        Command.setRangeColor(cell: cell, start: 0, len: 1, color: color);
    Command link(LinkTarget? target) =>
        Command.setLink(cell: cell, start: 0, len: 1, target: target);

    expect(
      (await doc.apply(command: color(const Rgb(r: 255, g: 0, b: 0)))).changed,
      [cell],
    );
    expect((await doc.apply(command: color(null))).changed, [cell]);
    // No colour is there to remove now: nothing changes.
    expect((await doc.apply(command: color(null))).changed, isEmpty);

    final other = (await doc.grid(cell: await doc.root())).cells[1].id;
    expect(
      (await doc.apply(command: link(LinkTarget.cell(cell: other)))).changed,
      [cell],
    );
    expect((await doc.apply(command: link(null))).changed, [cell]);
    expect((await doc.apply(command: link(null))).changed, isEmpty);
  });

  // A Dart string can hold half of a surrogate pair, which is no character. Dart's
  // UTF-8 encoder writes U+FFFD in its place, so that is what the core gets: nothing is
  // refused and nothing says so. `wire.rs` documents it; this pins it.
  test('half a surrogate pair arrives as U+FFFD', () async {
    final doc = await twoRows('surrogate');
    final cell = (await doc.grid(cell: await doc.root())).cells[0].id;
    final half = String.fromCharCode(0xD800);
    const replacement = '\u{FFFD}';

    await doc.apply(
      command: Command.setText(cell: cell, text: 'a${half}b'),
    );
    final text = (await doc.grid(cell: cell)).cell.text;
    expect(text, 'a${replacement}b');
    expect(text.runes.length, 3);
    // A whole pair is one character and crosses as it is.
    await doc.apply(
      command: Command.setText(cell: cell, text: 'a\u{1F600}b'),
    );
    expect((await doc.grid(cell: cell)).cell.text, 'a\u{1F600}b');

    // A note, the same; an ID with one in it is no ID.
    await doc.apply(
      command: Command.setProps(cells: [cell], props: [Prop.note(half)]),
    );
    await doc.save();
    final saved = await File(await doc.path()).readAsString();
    expect(saved, contains('"note": "$replacement"'));
    expect(
      await refusal(doc, Command.removeGrid(cell: '$cell$half')),
      ErrorKind.badRequest,
    );
    // And a path: the file is made under the name with U+FFFD in it.
    final named = await OpenDocument.create(
      path: '${folder.path}/x$half.wombat',
    );
    expect(await named.path(), '${folder.path}/x$replacement.wombat');
    expect(File('${folder.path}/x$replacement.wombat').existsSync(), true);
  });
}
