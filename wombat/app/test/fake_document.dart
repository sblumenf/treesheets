import 'dart:async';

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

/// A stand-in for the core, in Dart alone, for tests of what the app does around a
/// document: it can be made to wait, to fail and to count. It knows `EnterText` and
/// `SetText`, undo and redo of those, and reading a grid.
///
/// The document it starts with: the root "Root" has two columns and two rows, `a` "one",
/// `b` "=X" (a formula that failed, showing "#NAME?"), `c` "three" (with a grid of one
/// column: `e` "five", `f` "=1+1" showing "2") and `d` "four".
class FakeDocument implements OpenDocument {
  final Map<String, FakeCell> cells = {
    'root': FakeCell('Root', xs: 2, cells: ['a', 'b', 'c', 'd']),
    'a': FakeCell('one'),
    'b': FakeCell('=X', result: '#NAME?', failed: true),
    'c': FakeCell('three', xs: 1, cells: ['e', 'f']),
    'd': FakeCell('four'),
    'e': FakeCell('five'),
    'f': FakeCell('=1+1', result: '2'),
  };

  /// Runs at the start of every call, with its name: `grid`, `apply`, `undo`, `redo`,
  /// `save`, `overwrite`, `close`, `isSaved`. The call goes on when it ends, and fails
  /// with what it throws.
  FutureOr<void> Function(String call)? before;

  /// Every call, as `name` when it began and `name.` when it ended, well or badly.
  final List<String> log = [];

  int saves = 0;
  int overwrites = 0;
  bool saved = true;
  bool closed = false;
  bool disposed = false;
  final List<(String, String, String)> _done = [];
  final List<(String, String, String)> _undone = [];

  Future<T> _call<T>(String name, T Function() work) async {
    log.add(name);
    try {
      await before?.call(name);
      return work();
    } finally {
      log.add('$name.');
    }
  }

  CellInfo _info(String id) {
    final cell = cells[id];
    if (cell == null) {
      throw BridgeError(kind: ErrorKind.notFound, message: 'no cell $id');
    }
    return CellInfo(
      id: id,
      text: cell.text,
      kind: cell.result == null ? CellKind.data : CellKind.formula,
      result: cell.result,
      failed: cell.failed,
      hasGrid: cell.xs > 0,
    );
  }

  Report _set(String id, String text) {
    _info(id);
    cells[id]!.text = text;
    saved = false;
    return Report(
      changed: [id],
      created: const [],
      removed: const [],
      grids: const [],
      tags: const [],
      recalculated: const [],
    );
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
  Future<Report> apply({required Command command}) => _call('apply', () {
    final (cell, text) = switch (command) {
      Command_EnterText(:final cell, :final text) => (cell, text),
      Command_SetText(:final cell, :final text) => (cell, text),
      _ => throw const BridgeError(
        kind: ErrorKind.refused,
        message: 'the stand-in does not know this command',
      ),
    };
    _done.add((cell, _info(cell).text, text));
    _undone.clear();
    return _set(cell, text);
  });

  @override
  Future<Report?> undo() => _call('undo', () {
    if (_done.isEmpty) return null;
    final step = _done.removeLast();
    _undone.add(step);
    return _set(step.$1, step.$2);
  });

  @override
  Future<Report?> redo() => _call('redo', () {
    if (_undone.isEmpty) return null;
    final step = _undone.removeLast();
    _done.add(step);
    return _set(step.$1, step.$3);
  });

  @override
  Future<void> save() => _call('save', () {
    saves++;
    saved = true;
  });

  @override
  Future<void> overwrite() => _call('overwrite', () {
    overwrites++;
    saved = true;
  });

  @override
  Future<void> close() => _call('close', () {
    saves += saved ? 0 : 1;
    saved = true;
    closed = true;
  });

  @override
  void dispose() => disposed = true;

  @override
  bool get isDisposed => disposed;
}
