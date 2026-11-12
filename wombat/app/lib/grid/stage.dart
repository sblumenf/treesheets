import 'package:flutter/foundation.dart';

import '../session.dart';
import '../src/rust/api/wire.dart';

/// One step of the way from the root to the grid on the stage.
class Level {
  const Level(this.id, this.text);

  /// The ID of the cell.
  final String id;

  /// Its text when the level was last read.
  final String text;
}

/// What the simple grid screen shows and does, apart from the widgets: one grid is on the
/// stage, the trail says how one got there, and every change goes to the core as a
/// command. Nothing of the document is kept here but what was last read.
class Stage extends ChangeNotifier {
  Stage._(this._doc);

  final Session _doc;

  /// The IDs of the cells from the root to the cell on the stage.
  final List<String> _path = [];
  final List<String> _titles = [];
  late GridInfo _grid;
  Set<String> _touched = const {};

  /// Starts at the root of an open document.
  static Future<Stage> open(Session doc) async {
    final stage = Stage._(doc);
    stage._path.add(doc.root);
    stage._titles.add('');
    await stage._read();
    return stage;
  }

  /// The cell on the stage and the cells of its grid.
  GridInfo get grid => _grid;

  /// The way from the root to the stage, the root first.
  List<Level> get trail => [
    for (var i = 0; i < _path.length; i++) Level(_path[i], _titles[i]),
  ];

  /// Whether the stage is the root.
  bool get atRoot => _path.length == 1;

  /// The cells that the last command, undo or redo changed or recalculated.
  Set<String> get touched => _touched;

  /// Reads the stage again. A cell that is no longer there (an undo can take it away)
  /// gives way to the nearest cell above it that is.
  Future<void> _read() async {
    while (true) {
      try {
        _grid = await _doc.grid(_path.last);
        _titles[_titles.length - 1] = _grid.cell.text;
        return;
      } on BridgeError catch (error) {
        if (error.kind != ErrorKind.notFound || atRoot) rethrow;
        _path.removeLast();
        _titles.removeLast();
      }
    }
  }

  /// Puts the grid of a cell of the stage on the stage. Returns false, and changes
  /// nothing, if the cell is not on the stage or has no grid.
  Future<bool> dive(String cell) async {
    final found = _grid.cells.where((c) => c.id == cell && c.hasGrid);
    if (found.isEmpty) return false;
    _path.add(cell);
    _titles.add(found.first.text);
    _touched = const {};
    await _read();
    notifyListeners();
    return true;
  }

  /// Goes to a level of the trail, 0 being the root. Returns false at a level that is
  /// not above the stage.
  Future<bool> jumpTo(int level) async {
    if (level < 0 || level >= _path.length - 1) return false;
    _path.removeRange(level + 1, _path.length);
    _titles.removeRange(level + 1, _titles.length);
    _touched = const {};
    await _read();
    notifyListeners();
    return true;
  }

  /// Goes up one level. Returns false at the root.
  Future<bool> up() => jumpTo(_path.length - 2);

  Future<void> _changed(Report report) async {
    _touched = {...report.changed, ...report.created, ...report.recalculated};
    await _read();
    notifyListeners();
  }

  /// Typing into a cell has ended with this text: the core sets it, and makes the cell a
  /// formula cell if the text is a formula. Returns what changed.
  Future<Report> enterText(String cell, String text) async {
    final report = await _doc.apply(Command.enterText(cell: cell, text: text));
    await _changed(report);
    return report;
  }

  /// Undoes the last command. Returns false if there is nothing to undo.
  Future<bool> undo() async {
    final report = await _doc.undo();
    if (report == null) return false;
    await _changed(report);
    return true;
  }

  /// Redoes the command undone last. Returns false if there is nothing to redo.
  Future<bool> redo() async {
    final report = await _doc.redo();
    if (report == null) return false;
    await _changed(report);
    return true;
  }

  /// Saves the document to its file.
  Future<void> save() => _doc.save();
}
