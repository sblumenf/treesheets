import 'package:flutter/foundation.dart';

import '../keeper.dart';
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
/// **The stage does not save.** The document belongs to a [Keeper], which saves it for
/// every screen that works on it; the stage passes on what the keeper says
/// ([saveError], [ended]) and asks it ([save], [keepMine], [openTheirs]). Whoever made
/// the keeper closes it.
class Stage extends ChangeNotifier {
  Stage._(this.keeper) : _opens = keeper.opens;

  /// What keeps the document of this stage in its file.
  final Keeper keeper;
  final _turns = InTurn();

  /// The IDs of the cells from the root to the cell on the stage.
  final List<String> _path = [];
  final List<String> _titles = [];
  late GridInfo _grid;
  Set<String> _touched = const {};
  Object? _readError;

  /// Which document of the keeper the stage shows.
  int _opens;

  /// Starts at the root of the document that `keeper` keeps. If the root cannot be
  /// read, the error is thrown and there is no stage.
  static Future<Stage> open(Keeper keeper) async {
    final stage = Stage._(keeper);
    await stage._start();
    keeper.addListener(stage._kept);
    keeper.addFinisher(stage._settled);
    return stage;
  }

  /// Ends when every step asked of the stage before now is done.
  Future<void> _settled() => _turns.run(() async {});

  Future<void> _start() async {
    _path
      ..clear()
      ..add(keeper.root);
    _titles
      ..clear()
      ..add('');
    _touched = const {};
    await _read();
  }

  /// The keeper has something new to say. If it has opened the file again, what is on
  /// the stage is of a document that is gone: the stage starts again at the root.
  void _kept() {
    if (keeper.opens != _opens) {
      _opens = keeper.opens;
      _turns.run(() async {
        try {
          await _start();
        } catch (error) {
          // After a fault the keeper has ended the work, and that is what is shown.
          if (!isFatal(error)) _readError = error;
        }
        notifyListeners();
      });
    }
    notifyListeners();
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

  /// What the last try to save ended with, if it failed ([Keeper.saveError]).
  Object? get saveError => keeper.saveError;

  /// Why the stage could not be read again after a change, if it could not: what it
  /// shows is then older than the document. [refresh] tries again.
  Object? get readError => _readError;

  /// Why the document is no longer in use, as a sentence for the screen
  /// ([Keeper.ended]). Null while the work goes on. Once set, every step throws a
  /// [BridgeError] of kind [ErrorKind.closed].
  String? get ended => keeper.ended;

  /// Runs one step in its turn.
  Future<T> _step<T>(Future<T> Function() step) => _turns.run(() {
    if (keeper.ended != null) {
      throw const BridgeError(
        kind: ErrorKind.closed,
        message: 'the document is no longer in use',
      );
    }
    return step();
  });

  /// Reads the stage again. A cell that is no longer there (an undo can take it away)
  /// gives way to the nearest cell above it that is.
  Future<void> _read() async {
    while (true) {
      try {
        _grid = await keeper.grid(_path.last);
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
    final grid = await keeper.grid(cell);
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

  /// After changes that the core made. If the stage cannot be read again, the changes
  /// are no less made: that goes to [readError] and not to the caller.
  Future<void> _changed(List<Report> reports) async {
    final changed = {for (final report in reports) ...report.changed};
    _touched = {
      ...changed,
      for (final report in reports) ...report.created,
      for (final report in reports) ...report.recalculated,
    };
    try {
      await _read();
      // The levels above the stage are read only when they changed: reading one copies
      // the texts of its whole grid.
      for (var i = 0; i < _path.length - 1; i++) {
        if (changed.contains(_path[i])) {
          _titles[i] = (await keeper.grid(_path[i])).cell.text;
        }
      }
    } catch (error) {
      if (isFatal(error)) rethrow;
      _readError = error;
    }
    notifyListeners();
  }

  /// Typing into a cell has ended with this text: the core sets it, and makes the cell a
  /// formula cell if the text is a formula. Returns what changed; a text the cell
  /// already has changes nothing.
  ///
  /// With `insteadOfLast` the latest command is undone first, so that the two are one
  /// step of the undo history: for an editor whose text went into the document when the
  /// app was put away, and is now entered for good (decision A157). The caller knows
  /// that the latest command is that one. If the core refuses the text, the undone
  /// command is redone and the document is as before.
  Future<Report> enterText(
    String cell,
    String text, {
    bool insteadOfLast = false,
  }) => _step(() async {
    final undone = insteadOfLast ? await keeper.undo() : null;
    final Report report;
    try {
      report = await keeper.apply(Command.enterText(cell: cell, text: text));
    } catch (_) {
      if (undone != null) await keeper.redo();
      rethrow;
    }
    await _changed([?undone, report]);
    return report;
  });

  /// Undoes the last command. Returns false if there is nothing to undo.
  Future<bool> undo() => _step(() async {
    final report = await keeper.undo();
    if (report == null) return false;
    await _changed([report]);
    return true;
  });

  /// Redoes the command undone last. Returns false if there is nothing to redo.
  Future<bool> redo() => _step(() async {
    final report = await keeper.redo();
    if (report == null) return false;
    await _changed([report]);
    return true;
  });

  /// Saves the document now, after every step asked before ([Keeper.save]). It does not
  /// throw: see [saveError].
  Future<void> save() => _turns.run(keeper.save);

  /// Writes the document over what another program saved ([Keeper.keepMine]). It does
  /// not throw: see [saveError].
  Future<void> keepMine() => _turns.run(keeper.keepMine);

  /// Lets go of this document and opens the file as the other program left it, after
  /// every step asked before ([Keeper.openTheirs]); the stage then starts again at the
  /// root. If that file cannot be read, the error is thrown and nothing changed.
  Future<void> openTheirs() async {
    // Not inside a step: the keeper tells the stage while it is at it, and the stage
    // then asks for the step in which it starts again, which this waits for.
    await _settled();
    await keeper.openTheirs();
    await _settled();
  }

  /// The stage is no longer used: it stops listening to the keeper, which stays open.
  @override
  void dispose() {
    keeper.removeListener(_kept);
    keeper.removeFinisher(_settled);
    super.dispose();
  }
}
