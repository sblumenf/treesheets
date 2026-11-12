import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';

import 'session.dart';
import 'src/rust/api/wire.dart';

/// How a kept document stands with its file ([Keeper.state]).
enum KeeperState {
  /// The file holds the document as it is.
  saved,

  /// Something was done that is not in the file yet. It is saved by itself soon.
  unsaved,

  /// What was done is being written now.
  saving,

  /// The last try to save failed ([Keeper.saveError]); it is tried again by itself.
  failed,

  /// Another program changed the file, and nothing is written until the user has
  /// chosen between [Keeper.keepMine] and [Keeper.openTheirs].
  changedOnDisk,

  /// The document is no longer in use ([Keeper.ended]).
  ended,
}

/// Something that works on the document could not finish what it had begun, so what
/// was asked ([Keeper.close], [Keeper.openTheirs]) was not done and nothing changed.
class Unfinished implements Exception {
  Unfinished(this.causes);

  /// What each one that did not finish threw; a [TimeoutException] for one that took
  /// too long.
  final List<Object> causes;

  @override
  String toString() =>
      'Not everything that works on the document could finish what it had '
      'begun: ${causes.join('; ')}';
}

/// What [Keeper.attach] gives back, to take the attaching back with.
class Attachment {
  Attachment._(this._keeper, this._onChange, this._finish, this._queue);

  final Keeper _keeper;
  final VoidCallback? _onChange;
  final Future<void> Function()? _finish;
  final InTurn? _queue;

  /// Ends the attachment: nothing is told or asked through it any more. Doing so
  /// twice is doing so once.
  void detach() {
    if (!_keeper._attached.remove(this)) return;
    if (_onChange != null) _keeper.removeListener(_onChange);
  }
}

/// Keeps one open document in its file, for every screen that works on it: the one
/// place that saves (decision A145). The screens read and change the document through
/// it and show what it says; they do not save, and they do not hold the [Session].
///
/// **The document saves itself**: `saveAfter` after the first change that is not saved
/// yet, when [save] or [putAway] is called (whoever owns the keeper does so when the
/// app is no longer in front), and at [close]. [state] says how it stands. A save that
/// failed is in [saveError] until one works, and is tried again without being asked,
/// after a wait that doubles with every failure up to `retryAtMost`. One that failed
/// because another program changed the file is not tried again: the user chooses
/// between [keepMine] and [openTheirs].
///
/// **What works on the document attaches itself** ([attach]) for as long as it does,
/// to be told what changes here and to be asked to finish what it has begun.
///
/// **After a fault in the program** the document is let go of and [ended] says so,
/// and nothing more is saved. **After the end**, by a fault, [close] or [letGo], every
/// call fails the same way, with a [BridgeError] of kind [ErrorKind.closed]: also one
/// that was made before the end and whose turn came after it, and also those that
/// otherwise never throw.
///
/// Listeners, and what is attached, are told when [state], [saveError], [ended] or
/// [opens] changed, never twice of the same. They are told apart from the call that
/// made the change, so they may ask the keeper again at once.
class Keeper extends ChangeNotifier {
  /// Keeps `doc`, which is the keeper's from here on. What is attached gets
  /// `finishWithin` to finish what it has begun. `reopen` opens the file again and
  /// `canBeRead` finds out whether it can be, for [openTheirs]; tests put stand-ins
  /// there.
  Keeper(
    this._doc, {
    Duration saveAfter = const Duration(seconds: 1),
    this._retryAtMost = const Duration(minutes: 1),
    this._finishWithin = const Duration(seconds: 2),
    this._reopen = Session.open,
    this._canBeRead = readable,
  }) : _saveAfter = saveAfter,
       _wait = saveAfter {
    _told = _view;
  }

  Session _doc;
  final Duration _saveAfter;
  final Duration _retryAtMost;
  final Duration _finishWithin;
  final Future<Session> Function(String path) _reopen;
  final Future<void> Function(String path) _canBeRead;
  final _turns = InTurn();

  Timer? _saveTimer;

  /// How long after a save that fails the next one is tried.
  Duration _wait;
  Object? _saveError;
  String? _ended;
  int _opens = 1;

  /// Whether something was done that no save has written, and how many times
  /// something was done at all.
  bool _unsaved = false;
  int _changes = 0;
  bool _writing = false;
  final _attached = <Attachment>[];

  /// What the listeners were told of last.
  late Object _told;

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

  /// How the document stands with its file. Leaving is safe when it is
  /// [KeeperState.saved]; what is wrong outranks what is under way, so a save that is
  /// tried again shows as [KeeperState.failed] until one works.
  KeeperState get state {
    final error = _saveError;
    if (_ended != null) return KeeperState.ended;
    if (error is BridgeError && error.kind == ErrorKind.changedOnDisk) {
      return KeeperState.changedOnDisk;
    }
    if (error != null) return KeeperState.failed;
    if (!_unsaved) return KeeperState.saved;
    return _writing ? KeeperState.saving : KeeperState.unsaved;
  }

  /// Everything listeners are told of. An error is the same as the one before when
  /// it says the same: the core makes a new one for every failure.
  Object get _view {
    final error = _saveError;
    final said = error is BridgeError
        ? '${error.kind.name}: ${error.message}'
        : error == null
        ? null
        : '${error.runtimeType}: $error';
    return (state, said, _ended, _opens);
  }

  /// Tells the listeners if there is something new, apart from the piece that runs.
  void _tell() {
    final view = _view;
    if (view == _told) return;
    _told = view;
    InTurn.apart(notifyListeners);
  }

  /// Every call after the end fails with this.
  void _refuseAfterTheEnd() {
    if (_ended != null) {
      throw const BridgeError(
        kind: ErrorKind.closed,
        message: 'the document is no longer in use',
      );
    }
  }

  /// One piece of the keeper's work, in its turn; refused once the work is ended.
  Future<T> _piece<T>(Future<T> Function() work) => _turns.run(() async {
    _refuseAfterTheEnd();
    try {
      return await work();
    } finally {
      _tell();
    }
  });

  /// One call to the document. A fault in the program ends the work.
  Future<T> _use<T>(Future<T> Function(Session doc) call) => _piece(() async {
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
  }

  /// Saves when the clock says so. The work may have ended before its turn comes.
  void _saveIn(Duration wait) => _saveTimer = Timer(wait, () {
    save().catchError(
      (Object _) {},
      test: (error) => error is BridgeError && error.kind == ErrorKind.closed,
    );
  });

  /// Something was done to the document, if the report says so. What formula cells
  /// show is worked out and in no file (decision A151). The first change since the
  /// last save starts the clock; later ones do not put it back, so that steady typing
  /// cannot keep the file old for ever.
  T _done<T extends Report?>(T report) {
    if (report == null ||
        report.changed.isEmpty &&
            report.created.isEmpty &&
            report.removed.isEmpty &&
            report.grids.isEmpty &&
            report.tags.isEmpty) {
      return report;
    }
    _unsaved = true;
    _changes++;
    if (_saveTimer == null) _saveIn(_saveAfter);
    return report;
  }

  /// A cell and the cells of its grid ([Session.grid]).
  Future<GridInfo> grid(String cell) => _use((doc) => doc.grid(cell));

  /// Carries out a command ([Session.apply]), and saves it a while later.
  Future<Report> apply(Command command) =>
      _use((doc) async => _done(await doc.apply(command)));

  /// Undoes the latest command ([Session.undo]), and saves that a while later.
  Future<Report?> undo() =>
      _use((doc) async => _done<Report?>(await doc.undo()));

  /// Redoes the command undone last ([Session.redo]), and saves that a while later.
  Future<Report?> redo() =>
      _use((doc) async => _done<Report?>(await doc.redo()));

  /// One try to make the file hold the document, in its turn. How it ended is in
  /// [state] and [saveError].
  Future<void> _write(Future<void> Function(Session doc) how) =>
      _piece(() async {
        _saveTimer?.cancel();
        _saveTimer = null;
        _writing = true;
        _tell();
        try {
          await how(_doc);
          _saved();
        } catch (error) {
          if (isFatal(error)) {
            await _end(faultSentence);
          } else {
            _failed(error);
          }
        } finally {
          _writing = false;
        }
      });

  void _saved() {
    _saveError = null;
    _unsaved = false;
    _wait = _saveAfter;
  }

  /// A save failed: it is tried again by itself, later each time, unless only the user
  /// can say what is to be done.
  void _failed(Object error) {
    _saveError = error;
    if (error is BridgeError && error.kind == ErrorKind.changedOnDisk) return;
    _saveIn(_wait);
    final next = _wait * 2;
    _wait = next < _retryAtMost ? next : _retryAtMost;
  }

  /// Saves the document to its file now, after every call made before. Nothing is
  /// written when nothing was done since the last save, and never over what another
  /// program put there. A save that fails does not throw: see [state] and
  /// [saveError].
  Future<void> save() => _write((doc) => doc.save());

  /// Writes the document over what another program saved, after [state] said
  /// [KeeperState.changedOnDisk] and the user chose this document. A write that
  /// fails does not throw: see [state] and [saveError].
  Future<void> keepMine() => _write((doc) => doc.overwrite());

  /// Attaches something that works on the document, until [Attachment.detach]:
  ///
  /// - `onChange` is called when the keeper has something new to say (as a listener
  ///   is).
  /// - `finish` is asked by [putAway], [close] and [openTheirs] to put into the
  ///   document what has been begun and is not there yet (what is being typed), and
  ///   ends when that is done. It gets `finishWithin`; what it throws, or that it
  ///   took too long, is said to whoever asked.
  /// - `queue` is the queue that orders the work of what is attached. `finish` is
  ///   run in its turn there, so it comes after everything asked before; and when
  ///   the keeper is asked from inside a piece of that queue, as part of that piece,
  ///   which is why the keeper runs it and `finish` does not ask the queue itself.
  ///   With a queue and no `finish`, the keeper waits for the queue's turn alone.
  Attachment attach({
    VoidCallback? onChange,
    Future<void> Function()? finish,
    InTurn? queue,
  }) {
    final attachment = Attachment._(this, onChange, finish, queue);
    _attached.add(attachment);
    if (onChange != null) addListener(onChange);
    return attachment;
  }

  /// Lets everything attached finish what it has begun, all at once and each within
  /// the time there is, and answers with what kept any of them from it. Asked by
  /// something that is itself finishing, it does nothing: that is this finishing.
  Future<List<Object>> _finish() async {
    if (Zone.current[this] == true) return const [];
    final failed = <Object>[];
    Future<void> one(Attachment attachment) {
      final finish = attachment._finish ?? () async {};
      final queue = attachment._queue;
      return runZoned(
            () => queue == null ? Future.sync(finish) : queue.join(finish),
            zoneValues: {this: true},
          )
          .timeout(
            _finishWithin,
            onTimeout: () => throw TimeoutException(
              'Something that works on the document did not finish what it '
              'had begun',
              _finishWithin,
            ),
          )
          .then<void>((_) {}, onError: (Object error) => failed.add(error));
    }

    await Future.wait([
      for (final attachment in [..._attached])
        if (attachment._finish != null || attachment._queue != null)
          one(attachment),
    ]);
    return failed;
  }

  /// The app is out of sight, and the system may end it without another word. What
  /// is in the document goes into its file first; then what is being typed on the
  /// screens goes into the document, and that into the file.
  ///
  /// Answers with what kept anything attached from finishing, which is nothing when
  /// all is well. It does not throw for that, nor for a save that fails ([state]).
  Future<List<Object>> putAway() async {
    await save();
    final before = _changes;
    final failed = await _finish();
    if (_changes != before && _ended == null) await save();
    return failed;
  }

  /// Lets go of this document and opens the file as the other program left it, after
  /// [state] said [KeeperState.changedOnDisk] and the user chose the file. What was
  /// done here since the last save is lost; that is the choice. What is attached
  /// finishes first, so that nothing it has begun on this document goes on in the
  /// other.
  ///
  /// Whether the file can be read is found out first, while this document is still
  /// open. If it cannot, if something attached cannot finish ([Unfinished]), or if
  /// there is no such choice to make ([ErrorKind.refused]), the error is thrown and
  /// nothing changed: the document is open and unsaved as before. If the file is
  /// changed once more between that look and the opening, and cannot be read then,
  /// the work is [ended].
  Future<void> openTheirs() async {
    _refuseAfterTheEnd();
    final failed = await _finish();
    if (failed.isNotEmpty) throw Unfinished(failed);
    await _piece(() async {
      // Letting go of a document whose file is as it left it saves the document
      // (decision A142), which is the opposite of what is asked for here.
      if (state != KeeperState.changedOnDisk) {
        throw const BridgeError(
          kind: ErrorKind.refused,
          message:
              'the file has not been changed by another program, so there '
              'is nothing else to open',
        );
      }
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
    });
  }

  /// Ends the work: what is being typed goes into the document, what is unsaved is
  /// saved and the document is closed, after every call made before.
  ///
  /// **It throws exactly when the document was not closed**, which then stays open
  /// and in use, so that a screen can say so and the user can choose; an owner with
  /// no screen left to ask calls [letGo]. That is when something attached could not
  /// finish ([Unfinished]; what is in the document is saved all the same), and when
  /// the document cannot be saved (the error is also in [saveError]).
  Future<void> close() async {
    _refuseAfterTheEnd();
    final failed = await _finish();
    if (failed.isNotEmpty) {
      await save();
      throw Unfinished(failed);
    }
    await _piece(() async {
      _saveTimer?.cancel();
      _saveTimer = null;
      try {
        await _doc.close();
      } catch (error) {
        if (isFatal(error)) {
          await _end(faultSentence);
        } else {
          _failed(error);
        }
        rethrow;
      }
      _saved();
      _ended = 'The document was closed.';
    });
  }

  /// Lets go of the document without closing it, for when [close] failed and there is
  /// no one to ask: its file may be opened again, and nothing is written over what
  /// another program put there. The bridge still saves what is unsaved if the file is
  /// as the document last read or wrote it, or gone (decision A142), on the calling
  /// thread.
  Future<void> letGo() => _piece(() => _end('The document was closed.'));
}

/// Finds out whether the document in the file at `path` can be read, also while a
/// session has that file open: the core reads it into a copy in a folder of its own,
/// which is removed again. Throws what opening the file would throw, a [BridgeError];
/// and a [FileSystemException] when there is no folder to make the copy in, which
/// says nothing about the file. The folder is made in `within`, or without one in the
/// system's temporary folder. `openCopy` is the core's; tests put a stand-in there.
Future<void> readable(
  String path, {
  Directory? within,
  Future<Session> Function({required String from, required String to})
      openCopy =
      Session.openCopy,
}) async {
  final folder = await (within ?? Directory.systemTemp).createTemp(
    'wombat-look',
  );
  try {
    final copy = await openCopy(from: path, to: '${folder.path}/copy.wombat');
    try {
      // Nothing was done to it, so this writes nothing.
      await copy.close();
    } finally {
      // Whatever the close said, the copy's handle and name are given back now and
      // not when the collector comes. (It does nothing after a close that worked.)
      await copy.discard();
    }
  } finally {
    try {
      await folder.delete(recursive: true);
    } on FileSystemException {
      // What is left is in the system's temporary folder; the answer stands.
    }
  }
}

/// Ends the keeping of a document when there is no one left to ask: saves and closes
/// it, and if that cannot be done lets go of it, which never writes over what another
/// program put in the file. It does nothing to a keeper that has ended, and never
/// throws.
Future<void> endKeeping(Keeper keeper) async {
  for (final end in [keeper.close, keeper.letGo]) {
    try {
      return await end();
    } catch (_) {
      if (keeper.ended != null) return;
    }
  }
}
