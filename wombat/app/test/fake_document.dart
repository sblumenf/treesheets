import 'dart:async';

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
  /// Runs at the start of every call, with its name: `grid`, `apply`, `undo`, `redo`,
  /// `save`, `overwrite`, `close`, `isSaved`. The call goes on when it ends, and fails
  /// with what it throws.
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
        if (const {'apply', 'undo', 'redo'}.contains(name)) broke();
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

  @override
  Future<Report> apply({required Command command}) =>
      hooked('apply', () => _real.apply(command: command));

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
/// document: it can be made to wait, to fail and to count. It knows `EnterText` and
/// `SetText`, undo and redo of those, and reading a grid.
///
/// Its file is three switches, and it saves as the real document does (decisions A140,
/// A142, A143): a save writes when something was done since the last one or the file
/// is gone, and refuses when another program changed the file; letting go of it
/// unclosed saves what the real one would; after a panic nothing is written.
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

  /// A panic left the document half changed.
  bool broken = false;
  bool closed = false;
  bool disposed = false;
  final List<(String, String, String)> _done = [];
  final List<(String, String, String)> _undone = [];

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
    if (!saved || fileGone) saves++;
    saved = true;
    fileGone = false;
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
    final same = _info(id).text == text;
    cells[id]!.text = text;
    saved = saved && same;
    return Report(
      changed: same ? const [] : [id],
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
    // As in the core: a command that asks for nothing new leaves nothing to undo.
    if (_info(cell).text != text) {
      _done.add((cell, _info(cell).text, text));
      _undone.clear();
    }
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
  Future<void> save() => _call('save', _save);

  @override
  Future<void> overwrite() => _call('overwrite', () {
    overwrites++;
    saved = true;
    fileChanged = false;
    fileGone = false;
  });

  @override
  Future<void> close() => _call('close', () {
    // With nothing unsaved the file is not looked at.
    if (!saved) _save();
    closed = true;
  });

  @override
  void dispose() {
    disposed = true;
    if (!closed && !saved && !broken && !fileChanged) {
      saves++;
      saved = true;
    }
  }

  @override
  bool get isDisposed => disposed;
}
