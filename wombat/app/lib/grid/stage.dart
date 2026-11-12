import 'dart:async';

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
///
/// **One step at a time.** Every step asked of the stage is carried out when the steps
/// asked before it are done, whether or not the caller waited, and is judged by the
/// stage as it is then: of two taps on the same way into a grid the second finds that
/// the cell is no longer on the stage and does nothing.
///
/// **The document saves itself** (decision A145): a while after a change, when [save]
/// is called (the screen does so when the app is no longer in front), and at [close].
/// A save that failed is in [saveError] until one works.
class Stage extends ChangeNotifier {
  Stage._(this._doc, this._saveAfter, this._reopen);

  Session _doc;
  final Duration _saveAfter;
  final Future<Session> Function(String path) _reopen;
  final _turns = InTurn();

  /// The IDs of the cells from the root to the cell on the stage.
  final List<String> _path = [];
  final List<String> _titles = [];
  late GridInfo _grid;
  Set<String> _touched = const {};
  Timer? _saveTimer;
  Object? _saveError;
  Object? _readError;
  String? _ended;

  /// Starts at the root of an open document, which is the stage's from here on: the
  /// stage saves it and [close] closes it. A change is in the file `saveAfter` later at
  /// the latest, as long as the app runs. `reopen` opens the file again for
  /// [openTheirs]; tests put a stand-in there.
  static Future<Stage> open(
    Session doc, {
    Duration saveAfter = const Duration(seconds: 1),
    Future<Session> Function(String path) reopen = Session.open,
  }) async {
    final stage = Stage._(doc, saveAfter, reopen);
    try {
      await stage._start();
    } catch (_) {
      // No stage, so no one to close the document: its file must not stay taken.
      await doc.discard();
      rethrow;
    }
    return stage;
  }

  Future<void> _start() async {
    _path
      ..clear()
      ..add(_doc.root);
    _titles
      ..clear()
      ..add('');
    _touched = const {};
    await _read();
  }

  /// The cell on the stage and the cells of its grid.
  GridInfo get grid => _grid;

  /// The way from the root to the stage, the root first.
  List<Level> get trail => [
    for (var i = 0; i < _path.length; i++) Level(_path[i], _titles[i]),
  ];

  /// Whether the stage is the root.
  bool get atRoot => _path.length == 1;

  /// The cells that the last command, undo or redo changed, made or recalculated.
  Set<String> get touched => _touched;

  /// What the last try to save ended with, if it failed: the document is not in its
  /// file as it is now. A [BridgeError] of kind [ErrorKind.changedOnDisk] means that
  /// another program changed the file, and the user chooses between [keepMine] and
  /// [openTheirs]. Null when the last save worked.
  Object? get saveError => _saveError;

  /// Why the stage could not be read again after a change, if it could not: what it
  /// shows is then older than the document. [refresh] tries again.
  Object? get readError => _readError;

  /// Why the document is no longer in use, as a sentence for the screen: after a fault
  /// in the program ([faultSentence]), or when the file could not be opened again.
  /// Null while the work goes on. Once set, every step throws a [BridgeError] of kind
  /// [ErrorKind.closed] and nothing more is saved.
  String? get ended => _ended;

  /// Runs one step in its turn. A fault in the program ends the work on the document.
  Future<T> _step<T>(Future<T> Function() step) => _turns.run(() async {
    if (_ended != null) {
      throw const BridgeError(
        kind: ErrorKind.closed,
        message: 'the document is no longer in use',
      );
    }
    try {
      return await step();
    } catch (error) {
      if (isFatal(error)) await _end(faultSentence);
      rethrow;
    }
  });

  Future<void> _end(String why) async {
    _ended = why;
    _saveTimer?.cancel();
    // Nothing is saved from a document after a fault, and nothing over a file that
    // another program changed; the file is free to be opened again.
    await _doc.discard();
    notifyListeners();
  }

  /// Reads the stage again. A cell that is no longer there (an undo can take it away)
  /// gives way to the nearest cell above it that is.
  Future<void> _read() async {
    while (true) {
      try {
        _grid = await _doc.grid(_path.last);
        _titles[_titles.length - 1] = _grid.cell.text;
        _readError = null;
        return;
      } on BridgeError catch (error) {
        if (error.kind != ErrorKind.notFound || atRoot) rethrow;
        _path.removeLast();
        _titles.removeLast();
      }
    }
  }

  /// Puts the grid of a cell of the stage on the stage. Returns false, and changes
  /// nothing, if the cell is not on the stage or has no grid. If the grid cannot be
  /// read, the error is thrown and the stage stays where it was.
  Future<bool> dive(String cell) => _step(() async {
    if (!_grid.cells.any((c) => c.id == cell && c.hasGrid)) return false;
    final grid = await _doc.grid(cell);
    _path.add(cell);
    _titles.add(grid.cell.text);
    _grid = grid;
    _touched = const {};
    _readError = null;
    notifyListeners();
    return true;
  });

  Future<bool> _jump(int level) async {
    if (level < 0 || level >= _path.length - 1) return false;
    final path = [..._path];
    final titles = [..._titles];
    _path.removeRange(level + 1, _path.length);
    _titles.removeRange(level + 1, _titles.length);
    try {
      await _read();
    } catch (_) {
      // The stage stays where it was, trail and all.
      _path
        ..clear()
        ..addAll(path);
      _titles
        ..clear()
        ..addAll(titles);
      rethrow;
    }
    _touched = const {};
    notifyListeners();
    return true;
  }

  /// Goes to a level of the trail, 0 being the root. Returns false at a level that is
  /// not above the stage. If the level cannot be read, the error is thrown and the
  /// stage stays where it was.
  Future<bool> jumpTo(int level) => _step(() => _jump(level));

  /// Goes up one level. Returns false at the root.
  Future<bool> up() => _step(() => _jump(_path.length - 2));

  /// The cell of the stage's grid with this ID as it is when every step asked before
  /// is done, or null if it is not there then. For opening an editor on what the cell
  /// holds now, not on what the screen showed when it was tapped.
  Future<CellInfo?> cell(String id) => _step(() async {
    for (final cell in _grid.cells) {
      if (cell.id == id) return cell;
    }
    return null;
  });

  /// Reads the stage again, after [readError].
  Future<void> refresh() => _step(() async {
    await _read();
    notifyListeners();
  });

  /// After a change that the core made. If the stage cannot be read again, the change
  /// is no less made: that goes to [readError] and not to the caller.
  Future<void> _changed(Report report) async {
    _touched = {...report.changed, ...report.created, ...report.recalculated};
    // The first change since the last save starts the clock; later ones do not put it
    // back, so that steady typing cannot keep the file old for ever.
    _saveTimer ??= Timer(_saveAfter, save);
    try {
      await _read();
    } catch (error) {
      if (isFatal(error)) rethrow;
      _readError = error;
    }
    notifyListeners();
  }

  /// Typing into a cell has ended with this text: the core sets it, and makes the cell a
  /// formula cell if the text is a formula. Returns what changed; a text the cell
  /// already has changes nothing.
  Future<Report> enterText(String cell, String text) => _step(() async {
    final report = await _doc.apply(Command.enterText(cell: cell, text: text));
    await _changed(report);
    return report;
  });

  /// Undoes the last command. Returns false if there is nothing to undo.
  Future<bool> undo() => _step(() async {
    final report = await _doc.undo();
    if (report == null) return false;
    await _changed(report);
    return true;
  });

  /// Redoes the command undone last. Returns false if there is nothing to redo.
  Future<bool> redo() => _step(() async {
    final report = await _doc.redo();
    if (report == null) return false;
    await _changed(report);
    return true;
  });

  /// One try to make the file hold the document. It never throws: how it ended is in
  /// [saveError], and listeners are told when that changed.
  Future<void> _write(Future<void> Function() how) => _turns.run(() async {
    if (_ended != null) return;
    _saveTimer?.cancel();
    _saveTimer = null;
    final before = _saveError;
    try {
      await how();
      _saveError = null;
    } catch (error) {
      if (isFatal(error)) return _end(faultSentence);
      _saveError = error;
    }
    if (!identical(before, _saveError)) notifyListeners();
  });

  /// Saves the document to its file now, after every step asked before. Nothing is
  /// written when nothing was done since the last save, and never over what another
  /// program put there. It does not throw: see [saveError].
  Future<void> save() => _write(() => _doc.save());

  /// Writes the document over what another program saved, after [saveError] said
  /// [ErrorKind.changedOnDisk] and the user chose this document. It does not throw:
  /// see [saveError].
  Future<void> keepMine() => _write(() => _doc.overwrite());

  /// Lets go of this document and opens the file as the other program left it, after
  /// [saveError] said [ErrorKind.changedOnDisk] and the user chose the file. The stage
  /// starts again at the root. What was done here since the last save is lost; that is
  /// the choice. If the file cannot be opened, the work is [ended].
  Future<void> openTheirs() => _turns.run(() async {
    if (_ended != null) return;
    _saveTimer?.cancel();
    _saveTimer = null;
    await _doc.discard();
    try {
      _doc = await _reopen(_doc.path);
      await _start();
    } catch (error) {
      return _end(
        isFatal(error)
            ? faultSentence
            : 'The file could not be opened again. ${describeError(error)}',
      );
    }
    _saveError = null;
    notifyListeners();
  });

  /// Ends the work: saves what is unsaved and closes the document, after every step
  /// asked before. If it cannot be saved the error is thrown, and the document is let
  /// go of as the bridge lets go of one that was not closed (decision A142): nothing is
  /// written over a file that another program changed. The stage is not used after.
  Future<void> close() => _turns.run(() async {
    _saveTimer?.cancel();
    _saveTimer = null;
    if (_ended != null) return;
    _ended = 'The document was closed.';
    try {
      await _doc.close();
    } catch (_) {
      await _doc.discard();
      rethrow;
    }
  });
}
