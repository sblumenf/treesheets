import 'dart:io';
import 'dart:typed_data';

import 'src/rust/api/document.dart';
import 'src/rust/api/wire.dart';

/// What the three buttons of the bare app do, apart from the screen: each step returns
/// the text to show. Kept apart so that a test can go through the same steps.
class Demo {
  OpenDocument? _doc;
  int _edits = 0;

  /// Whether a document is open.
  bool get isOpen => _doc != null;

  /// Opens the example: `example` is the file that comes with the app. Many installs
  /// have the same file, so the core makes a copy of it with IDs of its own in `folder`
  /// (decision A28), and that copy is what is opened.
  Future<String> open(Directory folder, Uint8List example) async {
    final original = File('${folder.path}/example-original.wombat');
    await original.writeAsBytes(example, flush: true);
    final stamp = DateTime.now().microsecondsSinceEpoch;
    final doc = await OpenDocument.openCopy(
      from: original.path,
      to: '${folder.path}/example-$stamp.wombat',
    );
    _doc = doc;
    _edits = 0;
    return 'Opened ${await doc.path()}\n'
        'document ${await doc.id()}\n\n${await _show(doc)}';
  }

  /// Sets the text of one cell (the first cost) and shows what the core reports.
  Future<String> change() async {
    final doc = _doc!;
    final root = await doc.grid(cell: await doc.root());
    final costs = await doc.grid(cell: root.cells[2].id);
    _edits += 1;
    final text = '${30 + _edits}';
    final report = await doc.apply(
      command: Command.setText(cell: costs.cells[0].id, text: text),
    );
    return 'Set the first cost to $text\n'
        '${describe(report)}\n\n${await _show(doc)}';
  }

  /// Undoes the last change.
  Future<String> undo() async {
    final doc = _doc!;
    final report = await doc.undo();
    if (report == null) return 'Nothing to undo\n\n${await _show(doc)}';
    return 'Undone\n${describe(report)}\n\n${await _show(doc)}';
  }

  /// A change report as text: only the lists that have something in them.
  static String describe(Report report) {
    final lines = <String>[];
    void list(String name, List<String> ids) {
      if (ids.isNotEmpty) lines.add('  $name: ${ids.join(', ')}');
    }

    list('changed', report.changed);
    list('created', report.created);
    list('removed', report.removed);
    list('grids', report.grids);
    list('tags', report.tags);
    list('recalculated', report.recalculated);
    return lines.isEmpty
        ? 'nothing changed'
        : 'change report\n${lines.join('\n')}';
  }

  /// The root grid and the grids one level inside it, as indented text.
  Future<String> _show(OpenDocument doc) async {
    final out = StringBuffer();
    final root = await doc.grid(cell: await doc.root());
    out.writeln('${_cell(root.cell)}  (${root.xs} x ${root.ys})');
    for (final cell in root.cells) {
      out.writeln('  ${_cell(cell)}');
      if (!cell.hasGrid) continue;
      final inside = await doc.grid(cell: cell.id);
      for (final inner in inside.cells) {
        out.writeln('    ${_cell(inner)}');
      }
    }
    return out.toString();
  }

  static String _cell(CellInfo cell) {
    final result = cell.result;
    final shown = result == null ? '' : ' = "$result"';
    return '"${cell.text}"$shown';
  }
}

/// An error from the core as text for the screen.
String describeError(Object error) => error is BridgeError
    ? 'Error (${error.kind.name}): ${error.message}'
    : 'Error: $error';
