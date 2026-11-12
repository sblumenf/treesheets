import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';

import 'session.dart';
import 'src/rust/api/wire.dart';

/// Keeps one open document in its file, for every screen that works on it: the one
/// place that saves (decision A145). The screens read and change the document through
/// it and show what it says; they do not save, and they do not hold the [Session].
///
/// **The document saves itself**: `saveAfter` after the first change that is not saved
/// yet, when [save] or [putAway] is called (whoever owns the keeper does so when the
/// app is no longer in front), and at [close]. A save that failed is in [saveError]
/// until one works, and is tried again without being asked, after a wait that doubles
/// with every failure up to `retryAtMost`. One that failed because another program
/// changed the file is not tried again: the user chooses between [keepMine] and
/// [openTheirs].
///
/// **After a fault in the program** the document is let go of and [ended] says so;
/// every call then throws a [BridgeError] of kind [ErrorKind.closed], and nothing more
/// is saved.
///
/// Listeners are told when [saveError], [ended] or [opens] changed.
class Keeper extends ChangeNotifier {
  /// Keeps `doc`, which is the keeper's from here on. `reopen` opens the file again and
  /// `canBeRead` finds out whether it can be, for [openTheirs]; tests put stand-ins
  /// there.
  Keeper(
    this._doc, {
    Duration saveAfter = const Duration(seconds: 1),
    this._retryAtMost = const Duration(minutes: 1),
    this._reopen = Session.open,
    this._canBeRead = readable,
  }) : _saveAfter = saveAfter,
       _wait = saveAfter;

  Session _doc;
  final Duration _saveAfter;
  final Duration _retryAtMost;
  final Future<Session> Function(String path) _reopen;
  final Future<void> Function(String path) _canBeRead;
  final _turns = InTurn();

  Timer? _saveTimer;

  /// How long after a save that fails the next one is tried.
  Duration _wait;
  Object? _saveError;
  String? _ended;
  int _opens = 1;
  final _finishers = <Future<void> Function()>[];
  final _owed = <Future<void>>{};

  /// The ID of the root cell of the document.
  String get root => _doc.root;

  /// The file the document is kept in.
  String get path => _doc.path;

  /// How many documents this keeper has had: it goes up when [openTheirs] opened the
  /// file again, which tells a screen to start again at the root.
  int get opens => _opens;

  /// What the last try to save ended with, if it failed: the document is not in its
  /// file as it is now. A [BridgeError] of kind [ErrorKind.changedOnDisk] means that
  /// another program changed the file, and the user chooses between [keepMine] and
  /// [openTheirs]. Null when the last save worked.
  Object? get saveError => _saveError;

  /// Why the document is no longer in use, as a sentence for the screen: it was closed,
  /// there was a fault in the program ([faultSentence]), or the file could not be
  /// opened again. Null while the work goes on.
  String? get ended => _ended;

  /// One call to the document, in its turn. A fault in the program ends the work.
  /// Once the work is ended the session is closed or let go of, and refuses every
  /// call itself ([ErrorKind.closed]).
  Future<T> _use<T>(Future<T> Function(Session doc) call) =>
      _turns.run(() async {
        try {
          return await call(_doc);
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

  /// The first change since the last save starts the clock; later ones do not put it
  /// back, so that steady typing cannot keep the file old for ever.
  void _changed() => _saveTimer ??= Timer(_saveAfter, save);

  /// A cell and the cells of its grid ([Session.grid]).
  Future<GridInfo> grid(String cell) => _use((doc) => doc.grid(cell));

  /// Carries out a command ([Session.apply]), and saves it a while later.
  Future<Report> apply(Command command) => _use((doc) async {
    final report = await doc.apply(command);
    _changed();
    return report;
  });

  /// Undoes the latest command ([Session.undo]), and saves that a while later.
  Future<Report?> undo() => _use((doc) async {
    final report = await doc.undo();
    if (report != null) _changed();
    return report;
  });

  /// Redoes the command undone last ([Session.redo]), and saves that a while later.
  Future<Report?> redo() => _use((doc) async {
    final report = await doc.redo();
    if (report != null) _changed();
    return report;
  });

  /// One try to make the file hold the document. It never throws: how it ended is in
  /// [saveError], and listeners are told when that changed.
  Future<void> _write(Future<void> Function(Session doc) how) =>
      _turns.run(() async {
        if (_ended != null) return;
        _saveTimer?.cancel();
        _saveTimer = null;
        final before = _saveError;
        try {
          await how(_doc);
          _saved();
        } catch (error) {
          if (isFatal(error)) return _end(faultSentence);
          _failed(error);
        }
        if (!identical(before, _saveError)) notifyListeners();
      });

  void _saved() {
    _saveError = null;
    _wait = _saveAfter;
  }

  /// A save failed: it is tried again by itself, later each time, unless only the user
  /// can say what is to be done.
  void _failed(Object error) {
    _saveError = error;
    if (error is BridgeError && error.kind == ErrorKind.changedOnDisk) return;
    _saveTimer = Timer(_wait, save);
    final next = _wait * 2;
    _wait = next < _retryAtMost ? next : _retryAtMost;
  }

  /// Saves the document to its file now, after every call made before. Nothing is
  /// written when nothing was done since the last save, and never over what another
  /// program put there. It does not throw: see [saveError].
  Future<void> save() => _write((doc) => doc.save());

  /// Writes the document over what another program saved, after [saveError] said
  /// [ErrorKind.changedOnDisk] and the user chose this document. It does not throw:
  /// see [saveError].
  Future<void> keepMine() => _write((doc) => doc.overwrite());

  /// Says how something that works on the document finishes what it has begun and is
  /// not in the document yet, for [putAway] and [close] to wait for: a stage the steps
  /// asked of it, a screen what is being typed on it. `finish` ends when that is done,
  /// and does not throw.
  void addFinisher(Future<void> Function() finish) => _finishers.add(finish);

  /// Takes back [addFinisher].
  void removeFinisher(Future<void> Function() finish) =>
      _finishers.remove(finish);

  /// Something that is on its way into the document and must be there before the
  /// document is saved by [putAway] or closed: what a screen that is going away had in
  /// its editor. `work` does not throw.
  void owe(Future<void> work) {
    _owed.add(work);
    work.then((_) => _owed.remove(work));
  }

  Future<void> _finished() => Future.wait([
    ..._owed,
    for (final finish in [..._finishers]) finish(),
  ]);

  /// The app is out of sight, and the system may end it without another word: what is
  /// being typed on the screens goes into the document, and the document into its
  /// file, now.
  Future<void> putAway() async {
    try {
      await _finished();
    } finally {
      await save();
    }
  }

  /// Lets go of this document and opens the file as the other program left it, after
  /// [saveError] said [ErrorKind.changedOnDisk] and the user chose the file. What was
  /// done here since the last save is lost; that is the choice.
  ///
  /// Whether the file can be read is found out first, while this document is still
  /// open. If it cannot, the error is thrown and nothing changed: the document is
  /// open and unsaved as before, and the choice is still to make. If the file is
  /// changed once more between that look and the opening, and cannot be read then,
  /// the work is [ended].
  Future<void> openTheirs() => _turns.run(() async {
    if (_ended != null) return;
    final path = _doc.path;
    await _canBeRead(path);
    _saveTimer?.cancel();
    _saveTimer = null;
    await _doc.discard();
    try {
      _doc = await _reopen(path);
    } catch (error) {
      return _end(
        isFatal(error)
            ? faultSentence
            : 'The file was changed again while it was opened, and cannot be '
                  'read now, so no document is open. ${describeError(error)} '
                  'Close Wombat and open it again: it will offer to put that '
                  'file aside.',
      );
    }
    _opens++;
    _saved();
    notifyListeners();
  });

  /// Ends the work: what is being typed goes into the document, what is unsaved is
  /// saved and the document is closed, after every call made before.
  ///
  /// If it cannot be saved, the error is thrown and is in [saveError], and the
  /// document stays open and in use, so that a screen can say so and the user can
  /// choose; an owner with no screen left to ask calls [letGo].
  Future<void> close() async {
    try {
      await _finished();
    } finally {
      await _turns.run(() async {
        if (_ended != null) return;
        _saveTimer?.cancel();
        _saveTimer = null;
        try {
          await _doc.close();
        } catch (error) {
          if (isFatal(error)) {
            await _end(faultSentence);
          } else {
            _failed(error);
            notifyListeners();
          }
          rethrow;
        }
        _saveError = null;
        _ended = 'The document was closed.';
        notifyListeners();
      });
    }
  }

  /// Lets go of the document without closing it, for when [close] failed and there is
  /// no one to ask: its file may be opened again, and nothing is written over what
  /// another program put there. The bridge still saves what is unsaved if the file is
  /// as the document last read or wrote it, or gone (decision A142), on the calling
  /// thread.
  Future<void> letGo() => _turns.run(() async {
    if (_ended != null) return;
    await _end('The document was closed.');
  });
}

/// Finds out whether the document in the file at `path` can be read, also while a
/// session has that file open: the core reads it into a copy in a folder of its own,
/// which is removed again. Throws what opening the file would throw.
Future<void> readable(String path) async {
  final folder = await Directory.systemTemp.createTemp('wombat-look');
  try {
    final copy = await Session.openCopy(
      from: path,
      to: '${folder.path}/copy.wombat',
    );
    // Nothing was done to it, so this writes nothing.
    await copy.close();
  } finally {
    try {
      await folder.delete(recursive: true);
    } on FileSystemException {
      // What is left is in the system's temporary folder; the answer stands.
    }
  }
}
