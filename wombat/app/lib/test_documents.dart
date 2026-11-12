import 'dart:io';

import 'measure.dart';
import 'phone/place.dart';
import 'phone/words.dart';
import 'session.dart';
import 'src/rust/api/wire.dart';

/// The documents the prototype is tried on (UI_SPEC section 17): the example that
/// comes with the app, and four that the app makes itself the first time each is
/// asked for. Each is a file in the app's folder and stays as it was left.
enum TestDocument {
  /// The example that comes with the app: the app's own document, opened by
  /// `openDocument` in `main.dart`.
  example('example.wombat', Words.example),

  /// A small document made by hand that shows each form of the depth budget at
  /// the width of a phone, and text of several lines, right to left, long, styled
  /// and coloured.
  forms('depth-forms.wombat', Words.forms),

  /// Six levels deep, with grids of up to 12 by 20 (68,749 cells).
  nested('six-levels.wombat', Words.sixLevels),

  /// 10 by 100 cells, each with a grid of 10 by 10 (101,001 cells).
  large('large.wombat', Words.large),

  /// A new document: a root cell without a grid.
  empty('empty.wombat', Words.empty);

  const TestDocument(this.file, this.label);

  /// The name of its file in the app's folder.
  final String file;

  /// What the menu calls it.
  final String label;
}

/// Opens the test document `which` in `folder`, making it first if its file is not
/// there. Not for [TestDocument.example], which is a copy of an asset.
Future<Session> openTestDocument(TestDocument which, Directory folder) async {
  await folder.create(recursive: true);
  final file = File('${folder.path}/${which.file}');
  if (FileSystemEntity.typeSync(file.path, followLinks: false) ==
      FileSystemEntityType.notFound) {
    switch (which) {
      case TestDocument.example:
        throw ArgumentError.value(which, 'which', 'is not made here');
      case TestDocument.forms:
        await _writeForms(file);
      case TestDocument.nested:
        await writeDocument(file, nestedTable(sixLevels), 'Six levels');
      case TestDocument.large:
        await writeDocument(file, largeTable(10, 100), 'Large');
      case TestDocument.empty:
        await (await Session.create(file.path)).close();
    }
  }
  return Session.open(file.path);
}

/// A cell of a document written down by hand: its text and the cells of its grid.
class _Cell {
  const _Cell(this.text, [this.xs = 0, this.cells = const []]);

  final String text;
  final int xs;
  final List<_Cell> cells;

  /// This cell and everything in it as entries of a table to import: each cell is
  /// followed by the cells of its grid.
  void into(List<EntryData> entries) {
    entries.add(
      EntryData(
        text: text,
        formula: text.startsWith('='),
        grid: cells.isEmpty ? null : GridSize(xs: xs, cells: cells.length),
      ),
    );
    for (final cell in cells) {
      cell.into(entries);
    }
  }
}

/// A grid of 12 by 20 cells with texts of every length, some empty and some with
/// a grid: something for a Shape to show.
_Cell _mixed(String text) => _Cell(text, 12, [
  for (var n = 0; n < 240; n++)
    n % 5 == 3
        ? const _Cell('')
        : n % 31 == 7
        ? _Cell('grid $n', 2, const [_Cell('a'), _Cell('b')])
        : _Cell('x' * (1 + n % 7 * 4)),
]);

/// The cells of the document "Depth forms": two columns, a name and what it names.
List<_Cell> _forms() => [
  const _Cell('Full'),
  const _Cell('2 by 2', 2, [
    _Cell('a1'),
    _Cell('b1'),
    _Cell('a2'),
    _Cell('b2'),
  ]),
  const _Cell('Preview'),
  _Cell('4 by 5', 4, [
    for (var n = 0; n < 20; n++) _Cell('P ${n % 4}.${n ~/ 4}'),
  ]),
  const _Cell('Shape, inside a Full grid'),
  _Cell('2 by 1', 2, [_mixed('12 by 20'), const _Cell('beside it')]),
  const _Cell('Badge, inside a Full grid'),
  _Cell('3 by 1', 3, [
    _mixed('big'),
    const _Cell('small', 2, [_Cell('1'), _Cell('2'), _Cell('3'), _Cell('4')]),
    const _Cell('none'),
  ]),
  const _Cell('Line breaks'),
  const _Cell('first line\nsecond line\nthird line'),
  const _Cell('Right to left'),
  const _Cell('שלום עולם, זה טקסט מימין לשמאל'),
  const _Cell('Arabic'),
  const _Cell('مرحبا بالعالم'),
  const _Cell('Long text'),
  const _Cell(
    'A cell with a long text, long enough to wrap over several lines at the '
    'width of a phone, so that the row grows and the cells beside it grow '
    'with it. Past eight lines the text is cut with an ellipsis until the '
    'cell is edited.',
  ),
  const _Cell('Formula'),
  const _Cell('=1+2'),
  const _Cell('Styles'),
  const _Cell('bold italic plain'),
  const _Cell('Colour'),
  const _Cell('a coloured cell'),
];

/// Makes the document "Depth forms" and saves it at `file`: the cells as one
/// import, and then what a table cannot say, the styles and colours.
Future<void> _writeForms(File file) async {
  final part = File('${file.path}.part');
  if (part.existsSync()) part.deleteSync();
  final doc = await Session.create(part.path);
  final root = doc.root;
  final entries = <EntryData>[];
  for (final cell in _forms()) {
    cell.into(entries);
  }
  await doc.apply(
    Command.import_(
      cell: root,
      table: TableData(xs: 2, cells: entries),
      replace: false,
    ),
  );
  await doc.apply(Command.setText(cell: root, text: 'Depth forms'));
  final cells = (await doc.grid(root)).cells;
  // The rows as the list above has them: the name, then what it names.
  String named(int row) => cells[row * 2 + 1].id;
  await doc.apply(
    Command.setRangeStyle(
      cell: named(9),
      start: 0,
      len: 4,
      set_: bold,
      clear: plain,
    ),
  );
  await doc.apply(
    Command.setRangeStyle(
      cell: named(9),
      start: 5,
      len: 6,
      set_: italic,
      clear: plain,
    ),
  );
  await doc.apply(
    Command.setProps(
      cells: [named(10)],
      props: const [Prop.cellColor(Rgb(r: 255, g: 236, b: 179))],
    ),
  );
  // Colours for the Shape: the first row and the first column of the large grid.
  final wide = (await doc.grid(named(2))).cells.first.id;
  final inside = (await doc.grid(wide)).cells;
  await doc.apply(
    Command.setProps(
      cells: [for (var x = 0; x < 12; x++) inside[x].id],
      props: const [Prop.cellColor(Rgb(r: 187, g: 222, b: 251))],
    ),
  );
  await doc.apply(
    Command.setProps(
      cells: [for (var y = 1; y < 20; y++) inside[y * 12].id],
      props: const [Prop.cellColor(Rgb(r: 200, g: 230, b: 201))],
    ),
  );
  await doc.close();
  part.renameSync(file.path);
}
