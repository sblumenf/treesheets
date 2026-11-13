import 'dart:io';
import 'dart:typed_data';

import 'example.dart';
import 'session.dart';
import 'src/rust/api/wire.dart';

/// What the three buttons of the bare app do, apart from the screen: each step returns
/// the text to show. Kept apart so that a test can go through the same steps. [close]
/// ends it.
class Demo {
  Session? _doc;
  int _edits = 0;

  /// Whether a document is open.
  bool get isOpen => _doc != null;

  /// Opens the example: `example` is the file that comes with the app, and what is
  /// opened is a copy of it in `folder`, with IDs of its own ([openExampleCopy]).
  Future<String> open(Directory folder, Uint8List example) async {
    // The copy from the press before is done with: saved, and its file left there.
    await close();
    final stamp = DateTime.now().microsecondsSinceEpoch;
    final doc = await openExampleCopy(
      example,
      folder: folder,
      to: 'example-$stamp.wombat',
    );
    _doc = doc;
    _edits = 0;
    return 'Opened ${doc.path}\n'
        'document ${doc.id}\n\n${await _show(doc)}';
  }

  /// Saves and closes the document that is open, if one is. If it cannot be saved,
  /// the error is thrown and the document is let go of all the same: this is a check
  /// on copies that nobody keeps, and a document neither closed nor let go of would
  /// hold its file until the app ends.
  Future<void> close() async {
    final doc = _doc;
    if (doc == null) return;
    try {
      await doc.close();
    } catch (_) {
      await doc.discard();
      rethrow;
    } finally {
      _doc = null;
    }
  }

  /// Sets the text of one cell (the first cost) and shows what the core reports.
  Future<String> change() async {
    final doc = _doc!;
    final root = await doc.grid(doc.root);
    final costs = await doc.grid(root.cells[2].id);
    _edits += 1;
    final text = '${30 + _edits}';
    final report = await doc.apply(
      Command.setText(cell: costs.cells[0].id, text: text),
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
  Future<String> _show(Session doc) async {
    final out = StringBuffer();
    final root = await doc.grid(doc.root);
    out.writeln('${_cell(root.cell)}  (${root.xs} x ${root.ys})');
    for (final cell in root.cells) {
      out.writeln('  ${_cell(cell)}');
      if (!cell.hasGrid) continue;
      final inside = await doc.grid(cell.id);
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
