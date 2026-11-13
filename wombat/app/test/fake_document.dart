import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/widgets.dart' show Characters;
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;
import 'package:wombat_app/src/rust/api/document.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

/// One cell of a [FakeDocument].
class FakeCell {
  FakeCell(
    this.text, {
    this.result,
    this.failed = false,
    this.xs = 0,
    this.cells = const [],
  });

  String text;
  String? result;
  bool failed;
  int xs;
  List<String> cells;
}

/// What a document under test can be made to do at the start of each call.
mixin Hooks {
  /// Runs at the start of every call, with its name, which is the name of the call in
  /// Dart (`grid`, `apply`, `retype`, `endGroup`, `saveView` and so on; all but `id`,
  /// `root` and `path`). The call goes on when it ends, and fails with what it throws.
  FutureOr<void> Function(String call)? before;

  /// Every call, as `name` when it began and `name.` when it ended, well or badly.
  final List<String> log = [];

  final Map<String, Completer<void>> _awaited = {};

  /// Ends when the next call named `call` has ended, well or badly: for waiting for
  /// what the app does by itself, such as saving, and not for a time.
  Future<void> next(String call) =>
      (_awaited[call] ??= Completer<void>()).future;

  /// Called when a call that changes the document was failed with a panic, or any
  /// call with the error that the bridge gives after one.
  void broke() {}

  Future<T> hooked<T>(String name, FutureOr<T> Function() work) async {
    log.add(name);
    try {
      try {
        await before?.call(name);
      } on PanicException {
        // The calls that change the document, as the bridge has them.
        if (const {
          'apply',
          'undo',
          'redo',
          'retype',
          'endGroup',
          'cancelGroup',
        }.contains(name)) {
          broke();
        }
        rethrow;
      } on BridgeError catch (error) {
        // The bridge answers so only once a panic has left the document half
        // changed, whatever the call.
        if (error.kind == ErrorKind.internal) broke();
        rethrow;
      }
      return await work();
    } finally {
      log.add('$name.');
      _awaited.remove(name)?.complete();
    }
  }
}

/// Makes the first call named `call` wait until the returned completer is completed;
/// the calls after it go on at once.
Completer<void> holdFirst(Hooks doc, String call) {
  final gate = Completer<void>();
  var held = false;
  doc.before = (name) {
    if (held || name != call) return null;
    held = true;
    return gate.future;
  };
  return gate;
}

/// Makes the first call named `call` fail with `error`.
void failFirst(Hooks doc, String call, Object error) {
  var failed = false;
  doc.before = (name) {
    if (failed || name != call) return;
    failed = true;
    throw error;
  };
}

/// A document of the real core that can be held up and made to fail like the
/// stand-in: every call goes on to the handle it was made over.
class Watched with Hooks implements OpenDocument {
  Watched(this._real);

  final OpenDocument _real;

  @override
  Future<String> id() => _real.id();

  @override
  Future<String> root() => _real.root();

  @override
  Future<String> path() => _real.path();

  @override
  Future<bool> isSaved() => hooked('isSaved', _real.isSaved);

  @override
  Future<GridInfo> grid({required String cell}) =>
      hooked('grid', () => _real.grid(cell: cell));

  /// How many cells the stage reads gave, one after the other.
  final List<int> stageCells = [];

  @override
  Future<StageInfo> stage({required String cell, required StageAsk ask}) =>
      hooked('stage', () async {
        final info = await _real.stage(cell: cell, ask: ask);
        stageCells.add(info.cells.length);
        return info;
      });

  @override
  Future<CellLook> cell({required String cell}) =>
      hooked('cell', () => _real.cell(cell: cell));

  @override
  Future<Report> apply({required Command command}) =>
      hooked('apply', () => _real.apply(command: command));

  @override
  Future<void> beginGroup() => hooked('beginGroup', _real.beginGroup);

  @override
  Future<Report> endGroup() => hooked('endGroup', _real.endGroup);

  @override
  Future<Report> cancelGroup() => hooked('cancelGroup', _real.cancelGroup);

  @override
  Future<bool> inGroup() => hooked('inGroup', _real.inGroup);

  @override
  Future<Report> retype({required String cell, required String text}) =>
      hooked('retype', () => _real.retype(cell: cell, text: text));

  @override
  Future<String> viewStage() => hooked('viewStage', _real.viewStage);

  @override
  Future<void> viewGo({required String cell, LevelInfo? left}) =>
      hooked('viewGo', () => _real.viewGo(cell: cell, left: left));

  @override
  Future<void> viewRemember({required LevelInfo level}) =>
      hooked('viewRemember', () => _real.viewRemember(level: level));

  @override
  Future<bool> setFolded({required String cell, required bool folded}) =>
      hooked('setFolded', () => _real.setFolded(cell: cell, folded: folded));

  @override
  Future<bool> isViewSaved() => hooked('isViewSaved', _real.isViewSaved);

  @override
  Future<void> saveView() => hooked('saveView', _real.saveView);

  @override
  Future<Report?> undo() => hooked('undo', _real.undo);

  @override
  Future<Report?> redo() => hooked('redo', _real.redo);

  @override
  Future<void> save() => hooked('save', _real.save);

  @override
  Future<void> overwrite() => hooked('overwrite', _real.overwrite);

  @override
  Future<void> close() => hooked('close', _real.close);

  @override
  void dispose() => _real.dispose();

  @override
  bool get isDisposed => _real.isDisposed;
}

/// A stand-in for the core, in Dart alone, for tests of what the app does around a
/// document: it can be made to wait, to fail and to count.
///
/// **What it knows**: `EnterText` and `SetText` (every other command is refused);
/// `retype`; groups, ended and cancelled, with both histories kept as the core keeps
/// them; undo and redo; reading a grid, the stage and one cell; the view state.
/// Text that is sent is made what a cell may hold, as the core does (CR LF and CR
/// become one line break, other control characters spaces).
///
/// **What it does not know**, and what is therefore tested on the real core: styles,
/// links, colours and tags (a cell here has none, so that `retype` keeps them cannot
/// be seen); formulas (a cell shows the result it was made with, whatever its text
/// becomes); commands that make or remove cells, so no stage here ever disappears.
/// `fake_document_test.dart` holds its stage read to the real core's, entry for
/// entry, on the document it starts with.
///
/// Its file is three switches, and it saves as the real document does (decisions A140,
/// A142, A143): a save writes when something was done since the last one or the file
/// is gone, and refuses when another program changed the file; letting go of it
/// unclosed saves what the real one would; after a panic nothing is written. The view
/// state's file is two more: [viewSaves] and [viewFails].
///
/// The document it starts with: the root "Root" has two columns and two rows, `a` "one",
/// `b` "=X" (a formula that failed, showing "#NAME?"), `c` "three" (with a grid of one
/// column: `e` "five", `f` "=1+1" showing "2") and `d` "four".
class FakeDocument with Hooks implements OpenDocument {
  final Map<String, FakeCell> cells = {
    'root': FakeCell('Root', xs: 2, cells: ['a', 'b', 'c', 'd']),
    'a': FakeCell('one'),
    'b': FakeCell('=X', result: '#NAME?', failed: true),
    'c': FakeCell('three', xs: 1, cells: ['e', 'f']),
    'd': FakeCell('four'),
    'e': FakeCell('five'),
    'f': FakeCell('=1+1', result: '2'),
  };

  /// How often the file was written by a save, a close or a let-go: writes, not calls.
  int saves = 0;

  /// How often the file was written without looking at it first.
  int overwrites = 0;

  /// Whether nothing was done since the file was read or written.
  bool saved = true;

  /// Another program changed the file.
  bool fileChanged = false;

  /// The file was removed.
  bool fileGone = false;

  /// What a command that worked answers with in place of its own report, if set:
  /// for reports that the two commands of the stand-in never give.
  Report Function(Report report)? reports;

  /// A panic left the document half changed.
  bool broken = false;
  bool closed = false;
  bool disposed = false;

  /// The undo history and the redo history: each step is the texts it set, as
  /// (cell, text before, text after).
  List<List<(String, String, String)>> _done = [];
  List<List<(String, String, String)>> _undone = [];

  /// How many groups are open, one inside the other.
  int _groups = 0;

  /// What the commands of the open group did so far.
  List<(String, String, String)> _inGroup = [];

  /// The histories as they were when the outermost group was opened.
  List<List<(String, String, String)>> _doneBefore = [];
  List<List<(String, String, String)>> _undoneBefore = [];

  /// Whether the file held the document when the outermost group was opened and was
  /// not written since.
  bool _savedBeforeGroup = false;

  /// The level that is shown, what each level remembers and what is folded.
  String zoom = 'root';
  final Map<String, LevelInfo> levels = {};
  final Set<String> folded = {};

  /// Whether the view state is as its file has it, and how often that file was
  /// written.
  bool viewSaved = true;
  int viewSaves = 0;

  /// The view state cannot be written.
  bool viewFails = false;

  @override
  void broke() => broken = true;

  Future<T> _call<T>(String name, T Function() work) => hooked(name, () {
    if (broken) {
      // A new error for every failure, as the core makes them.
      throw BridgeError(
        kind: ErrorKind.internal,
        message: 'a call before this one did not end',
      );
    }
    if (closed) {
      throw BridgeError(
        kind: ErrorKind.closed,
        message: 'the document was closed',
      );
    }
    return work();
  });

  /// What `save` does, and `close` before it gives the document up.
  void _save() {
    if (fileChanged) {
      throw BridgeError(
        kind: ErrorKind.changedOnDisk,
        message: 'the file was changed by another program',
      );
    }
    if (!saved || fileGone) {
      saves++;
      _savedBeforeGroup = false;
    }
    saved = true;
    fileGone = false;
  }

  /// What `saveView` does, and what letting go of the document does without telling.
  void _saveView() {
    if (viewSaved) return;
    if (viewFails) {
      throw BridgeError(
        kind: ErrorKind.io,
        message: 'the view state cannot be written',
      );
    }
    viewSaves++;
    viewSaved = true;
  }

  FakeCell _cell(String id) {
    final cell = cells[id];
    if (cell == null) {
      throw BridgeError(kind: ErrorKind.notFound, message: 'no cell $id');
    }
    return cell;
  }

  CellInfo _info(String id) {
    final cell = _cell(id);
    return CellInfo(
      id: id,
      text: cell.text,
      kind: cell.result == null ? CellKind.data : CellKind.formula,
      result: cell.result,
      failed: cell.failed,
      hasGrid: cell.xs > 0,
    );
  }

  static const _white = Rgb(r: 255, g: 255, b: 255);
  static const _black = Rgb(r: 0, g: 0, b: 0);
  static const _border = Rgb(r: 160, g: 160, b: 160);
  static const _plain = Style(
    bold: false,
    italic: false,
    fixed: false,
    underline: false,
    strike: false,
  );

  /// The longest beginning of `text` that has at most `most` code points and does
  /// not cut a user-perceived character.
  static CutText _cut(String text, int most) {
    final kept = StringBuffer();
    var count = 0;
    for (final character in Characters(text)) {
      count += character.runes.length;
      if (count > most) return CutText(text: '$kept', cut: true);
      kept.write(character);
    }
    return CutText(text: text, cut: false);
  }

  /// The same on one line, as a preview and the trail have it.
  static CutText _line(String text, int most) {
    final cut = _cut(text, most);
    return CutText(
      text: cut.text.replaceAll(RegExp(r'[\u0000-\u001f]'), ' '),
      cut: cut.cut,
    );
  }

  /// What a cell may hold of a text that is sent.
  static String _clean(String text) => text
      .replaceAll('\r\n', '\n')
      .replaceAll('\r', '\n')
      .replaceAll(RegExp(r'[\u0000-\u0009\u000b-\u001f]'), ' ');

  String _shown(FakeCell cell) => cell.result ?? cell.text;

  CellLook _look(String id, StageAsk ask, {int? summary, int? full}) {
    final cell = _cell(id);
    final text = _cut(cell.text, ask.textCut);
    final result = cell.result == null ? null : _cut(cell.result!, ask.textCut);
    return CellLook(
      id: id,
      text: text.text,
      textCut: text.cut,
      kind: cell.result == null ? CellKind.data : CellKind.formula,
      result: result?.text,
      resultCut: result?.cut ?? false,
      failed: cell.failed,
      cellColor: _white,
      textColor: _black,
      isTag: false,
      style: _plain,
      relSize: 0,
      textAlign: HorizAlign.auto,
      vertAlign: VertAlign.auto,
      runs: const [],
      links: const [],
      hasNote: false,
      hasGrid: cell.xs > 0,
      folded: folded.contains(id),
      summary: summary,
      full: full,
    );
  }

  GridSummary _summary(String id, StageAsk ask) {
    final cell = _cell(id);
    final ys = cell.cells.length ~/ cell.xs;
    int least(int asked, int there) => asked < there ? asked : there;
    final previewCols = least(ask.previewCols, cell.xs);
    final previewRows = least(ask.previewRows, ys);
    final shapeCols = least(ask.shapeCols, cell.xs);
    final shapeRows = least(ask.shapeRows, ys);
    return GridSummary(
      owner: id,
      xs: cell.xs,
      ys: ys,
      borderColor: _border,
      previewCols: previewCols,
      previewRows: previewRows,
      preview: [
        for (var y = 0; y < previewRows; y++)
          for (var x = 0; x < previewCols; x++)
            _line(_shown(_cell(cell.cells[y * cell.xs + x])), ask.previewCut),
      ],
      shapeCols: shapeCols,
      shapeRows: shapeRows,
      shape: Uint8List.fromList([
        for (var y = 0; y < shapeRows; y++)
          for (var x = 0; x < shapeCols; x++) ...[
            255,
            255,
            255,
            _cell(cell.cells[y * cell.xs + x]).xs > 0
                ? 255
                : least(
                    _shown(_cell(cell.cells[y * cell.xs + x])).runes.length,
                    254,
                  ),
          ],
      ]),
    );
  }

  /// The cells from the root to `id`, the root first.
  List<String> _path(String id) {
    _cell(id);
    final path = [id];
    while (path.first != 'root') {
      path.insert(
        0,
        cells.entries.firstWhere((e) => e.value.cells.contains(path.first)).key,
      );
    }
    return path;
  }

  static void _inRange(Iterable<int> numbers) {
    if (numbers.any((n) => n < 0 || n > 4294967295)) {
      throw BridgeError(
        kind: ErrorKind.badRequest,
        message: 'a number is out of range',
      );
    }
  }

  @override
  Future<String> id() async => 'fake';

  @override
  Future<String> root() async => 'root';

  @override
  Future<String> path() async => '/fake.wombat';

  @override
  Future<bool> isSaved() => _call('isSaved', () => saved);

  @override
  Future<GridInfo> grid({required String cell}) => _call('grid', () {
    final info = _info(cell);
    final inside = cells[cell]!;
    return GridInfo(
      cell: info,
      xs: inside.xs,
      ys: inside.xs == 0 ? 0 : inside.cells.length ~/ inside.xs,
      cells: inside.cells.map(_info).toList(),
    );
  });

  @override
  Future<StageInfo> stage({
    required String cell,
    required StageAsk ask,
  }) => _call('stage', () {
    _inRange([
      ask.x0,
      ask.y0,
      ask.cols,
      ask.rows,
      ask.fullDepth,
      ask.fullCells,
      ask.previewCols,
      ask.previewRows,
      ask.shapeCols,
      ask.shapeRows,
      ask.textCut,
      ask.previewCut,
    ]);
    final trail = [
      for (final id in _path(cell))
        TrailStep(
          id: id,
          shown: _line(_shown(_cell(id)), ask.previewCut),
          color: _white,
        ),
    ];
    // The grids given cell by cell, in the order they are found: each as its
    // owner, how far below the stage it is, and its window.
    final found = <(String, int, int, int, int, int)>[];
    final stage = _cell(cell);
    if (stage.xs > 0) {
      final ys = stage.cells.length ~/ stage.xs;
      final x0 = ask.x0 < stage.xs ? ask.x0 : stage.xs;
      final y0 = ask.y0 < ys ? ask.y0 : ys;
      final cols = ask.cols < stage.xs - x0 ? ask.cols : stage.xs - x0;
      final rows = ask.rows < ys - y0 ? ask.rows : ys - y0;
      found.add((cell, 0, x0, y0, cols, rows));
    }
    final grids = <GridPart>[];
    final looks = <CellLook>[];
    final summaries = <GridSummary>[];
    for (var g = 0; g < found.length; g++) {
      final (owner, level, x0, y0, cols, rows) = found[g];
      final inside = _cell(owner);
      grids.add(
        GridPart(
          owner: owner,
          level: level,
          xs: inside.xs,
          ys: inside.cells.length ~/ inside.xs,
          x0: x0,
          y0: y0,
          cols: cols,
          rows: rows,
          first: looks.length,
          colWidths: Uint32List.fromList(List.filled(cols, 80)),
          borderColor: _border,
          outerSpacing: 3,
          vertical: true,
          drawStyle: DrawStyle.grid,
        ),
      );
      for (var y = y0; y < y0 + rows; y++) {
        for (var x = x0; x < x0 + cols; x++) {
          final id = inside.cells[y * inside.xs + x];
          final child = _cell(id);
          int? summary;
          int? full;
          if (child.xs > 0) {
            summary = summaries.length;
            summaries.add(_summary(id, ask));
            if (level < ask.fullDepth && child.cells.length <= ask.fullCells) {
              full = found.length;
              found.add((
                id,
                level + 1,
                0,
                0,
                child.xs,
                child.cells.length ~/ child.xs,
              ));
            }
          }
          looks.add(_look(id, ask, summary: summary, full: full));
        }
      }
    }
    return StageInfo(
      trail: trail,
      grids: grids,
      cells: looks,
      summaries: summaries,
      level: levels[cell],
    );
  });

  /// A request that cuts nothing.
  static const _whole = StageAsk(
    x0: 0,
    y0: 0,
    cols: 0,
    rows: 0,
    fullDepth: 0,
    fullCells: 0,
    previewCols: 0,
    previewRows: 0,
    shapeCols: 0,
    shapeRows: 0,
    textCut: 4294967295,
    previewCut: 0,
  );

  @override
  Future<CellLook> cell({required String cell}) =>
      _call('cell', () => _look(cell, _whole));

  /// Sets the text of a cell as a command does, and says what changed.
  Report _set(String id, String text) {
    final before = _cell(id).text;
    final same = before == text;
    if (!same) {
      cells[id]!.text = text;
      saved = false;
      if (_groups > 0) {
        _inGroup.add((id, before, text));
      } else {
        // As in the core: a command that asks for nothing new leaves nothing to
        // undo, and one that changes something empties the redo history.
        _done.add([(id, before, text)]);
        _undone.clear();
      }
    }
    return _report(same ? const [] : [id]);
  }

  static Report _report(Iterable<String> changed) => Report(
    changed: changed.toSet().toList()..sort(),
    created: const [],
    removed: const [],
    grids: const [],
    tags: const [],
    recalculated: const [],
  );

  @override
  Future<void> beginGroup() => _call('beginGroup', () {
    if (_groups++ > 0) return;
    _inGroup = [];
    _doneBefore = [..._done];
    _undoneBefore = [..._undone];
    _savedBeforeGroup = saved;
  });

  @override
  Future<Report> endGroup() => _call('endGroup', () {
    if (_groups == 0 || --_groups > 0) return _report(const []);
    final steps = _inGroup;
    _inGroup = [];
    if (steps.isNotEmpty) {
      _done.add(steps);
      _undone.clear();
    }
    // The group as a whole. As in the core, a cell that was changed and changed back
    // is named too.
    return _report(steps.map((step) => step.$1));
  });

  @override
  Future<Report> cancelGroup() => _call('cancelGroup', () {
    if (_groups == 0) return _report(const []);
    _groups = 0;
    // What the taking back changes, from the document just before it.
    final now = {for (final (id, _, _) in _inGroup) id: cells[id]!.text};
    for (final (id, before, _) in _inGroup.reversed) {
      cells[id]!.text = before;
    }
    final changed = [
      for (final MapEntry(key: id, value: text) in now.entries)
        if (cells[id]!.text != text) id,
    ];
    _inGroup = [];
    _done = _doneBefore;
    _undone = _undoneBefore;
    if (changed.isNotEmpty) saved = false;
    if (_savedBeforeGroup) saved = true;
    return _report(changed);
  });

  @override
  Future<bool> inGroup() => _call('inGroup', () => _groups > 0);

  @override
  Future<Report> retype({required String cell, required String text}) =>
      _call('retype', () => _set(cell, _clean(text)));

  @override
  Future<String> viewStage() => _call('viewStage', () {
    // The level last gone to, or the nearest above it that has a grid.
    final path = _path(cells.containsKey(zoom) ? zoom : 'root');
    return path.lastWhere((id) => cells[id]!.xs > 0, orElse: () => 'root');
  });

  /// What a level may remember, with the place cut to the grid.
  LevelInfo _level(LevelInfo level) {
    _inRange([level.x, level.y]);
    final cell = _cell(level.cell);
    final selected = level.selected;
    if (selected != null) _cell(selected);
    if (cell.xs == 0) {
      throw BridgeError(
        kind: ErrorKind.refused,
        message: 'not possible: the cell has no grid',
      );
    }
    final ys = cell.cells.length ~/ cell.xs;
    return LevelInfo(
      cell: level.cell,
      x: level.x < cell.xs ? level.x : cell.xs - 1,
      y: level.y < ys ? level.y : ys - 1,
      selected: selected,
    );
  }

  @override
  Future<void> viewGo({required String cell, LevelInfo? left}) =>
      _call('viewGo', () {
        if (_cell(cell).xs == 0 && cell != 'root') {
          throw BridgeError(
            kind: ErrorKind.refused,
            message: 'not possible: the cell has no grid',
          );
        }
        if (left != null) {
          levels[left.cell] = _level(left);
          viewSaved = false;
        }
        if (zoom != cell) viewSaved = false;
        zoom = cell;
      });

  @override
  Future<void> viewRemember({required LevelInfo level}) =>
      _call('viewRemember', () {
        levels[level.cell] = _level(level);
        viewSaved = false;
      });

  @override
  Future<bool> setFolded({required String cell, required bool folded}) => _call(
    'setFolded',
    () {
      if (_cell(cell).xs == 0) {
        throw BridgeError(
          kind: ErrorKind.refused,
          message: 'not possible: the cell has no grid',
        );
      }
      final changed = folded ? this.folded.add(cell) : this.folded.remove(cell);
      if (changed) viewSaved = false;
      return changed;
    },
  );

  @override
  Future<bool> isViewSaved() => _call('isViewSaved', () => viewSaved);

  @override
  Future<void> saveView() => _call('saveView', _saveView);

  @override
  Future<Report> apply({required Command command}) => _call('apply', () {
    final (cell, text) = switch (command) {
      Command_EnterText(:final cell, :final text) => (cell, text),
      Command_SetText(:final cell, :final text) => (cell, text),
      _ => throw const BridgeError(
        kind: ErrorKind.refused,
        message: 'the stand-in does not know this command',
      ),
    };
    final report = _set(cell, _clean(text));
    return reports?.call(report) ?? report;
  });

  void _refuseInGroup(String what) {
    if (_groups > 0) {
      throw BridgeError(
        kind: ErrorKind.refused,
        message: 'not possible: $what while a group of commands is open',
      );
    }
  }

  @override
  Future<Report?> undo() => _call('undo', () {
    _refuseInGroup('undo');
    if (_done.isEmpty) return null;
    final step = _done.removeLast();
    _undone.add(step);
    for (final (id, before, _) in step.reversed) {
      cells[id]!.text = before;
    }
    saved = false;
    return _report(step.map((s) => s.$1));
  });

  @override
  Future<Report?> redo() => _call('redo', () {
    _refuseInGroup('redo');
    if (_undone.isEmpty) return null;
    final step = _undone.removeLast();
    _done.add(step);
    for (final (id, _, after) in step) {
      cells[id]!.text = after;
    }
    saved = false;
    return _report(step.map((s) => s.$1));
  });

  @override
  Future<void> save() => _call('save', _save);

  @override
  Future<void> overwrite() => _call('overwrite', () {
    overwrites++;
    saved = true;
    _savedBeforeGroup = false;
    fileChanged = false;
    fileGone = false;
  });

  @override
  Future<void> close() => _call('close', () {
    // With nothing unsaved the file is not looked at.
    if (!saved) _save();
    closed = true;
    // The view state after the document, and whether it could be written is not
    // said.
    try {
      _saveView();
    } on BridgeError {
      // Nothing is lost but where the screen was.
    }
  });

  @override
  void dispose() {
    disposed = true;
    if (closed || broken) return;
    if (!saved && !fileChanged) {
      saves++;
      saved = true;
    }
    // Where the screen was is this device's to say, also beside a file that another
    // program changed.
    if (!viewSaved && !viewFails) {
      viewSaves++;
      viewSaved = true;
    }
  }

  @override
  bool get isDisposed => disposed;
}
