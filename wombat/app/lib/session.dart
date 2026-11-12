import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge.dart'
    show PanicException;

import 'src/rust/api/document.dart';
import 'src/rust/api/wire.dart';

/// Runs pieces of work one at a time, in the order they were asked for, whether or not
/// the caller waits for one before asking for the next.
///
/// **An error is never lost.** It goes to whoever has taken the answer of its piece by
/// the time the piece fails (`await`, `then`, `catchError` and the like). If no one
/// has, it goes once to `onUnheard`, which the owner of the queue gives, or without
/// one to the zone as an error that nobody caught. Someone who takes the answer only
/// after that still finds the error in it. An asynchronous function that returns the
/// answer has taken it: the error is then in what that function answers, and is its
/// caller's to hear, as any error of that function is. Work that is done for no one
/// goes through [post], which gives no answer to pass on.
///
/// **A piece must not wait for its own queue.** A piece that waits for another piece
/// of the same queue would wait for ever, since that one comes after it; and so would
/// two pieces of two queues that each wait for the other's queue. [run] called from
/// inside a piece fails at once with a [StateError], and runs nothing, when the piece
/// that runs now in this queue is the caller itself or (through any number of queues)
/// has asked the caller's queue for something that is not answered yet. Whether the
/// caller waits for the answer cannot be seen, so it is taken to.
///
/// What is "inside a piece": the work itself and everything it starts that is not set
/// apart, whether the piece waits for it or not (Dart does not tell `await f()` from
/// `unawaited(f())`). Set apart are the callbacks of timers the piece makes, which run
/// when it may long have ended, and what is run through [apart]. So a wait for ever
/// that goes through a timer's callback is not seen.
class InTurn {
  InTurn({this.onUnheard});

  /// Where an error goes that no caller had taken when its piece failed.
  final void Function(Object error, StackTrace stack)? onUnheard;
  Future<void> _last = Future.value();

  /// The piece that runs now.
  _Piece? _now;

  /// Under this key a zone has the pieces, of any queues, that what runs in it is
  /// inside of.
  static final _inside = Object();

  /// Set while a timer's callback is on its way into the zone it was made in.
  static bool _timer = false;

  static List<_Piece> get _around =>
      Zone.current[_inside] as List<_Piece>? ?? const [];

  /// Runs `work` as something that is inside no piece, although a piece starts it:
  /// for what the piece does not wait for, such as telling listeners, which may then
  /// ask the queues that the piece is in.
  static R apart<R>(R Function() work) =>
      runZoned(work, zoneValues: {_inside: const <_Piece>[]});

  /// A timer's callback runs apart from the piece that made the timer.
  static final _pieceZone = ZoneSpecification(
    createTimer: (self, parent, zone, duration, callback) =>
        parent.createTimer(zone, duration, () {
          _timer = true;
          try {
            callback();
          } finally {
            _timer = false;
          }
        }),
    createPeriodicTimer: (self, parent, zone, period, callback) =>
        parent.createPeriodicTimer(zone, period, (timer) {
          _timer = true;
          try {
            callback(timer);
          } finally {
            _timer = false;
          }
        }),
    run: <R>(self, parent, zone, R Function() f) =>
        parent.run(_timer ? _apartFrom(zone) : zone, f),
    runUnary: <R, T>(self, parent, zone, R Function(T) f, T arg) =>
        parent.runUnary(_timer ? _apartFrom(zone) : zone, f, arg),
  );

  static Zone _apartFrom(Zone zone) {
    _timer = false;
    return zone.fork(zoneValues: {_inside: const <_Piece>[]});
  }

  /// Whether the piece that runs now waits for something of `queue`, as far as can be
  /// seen: it is asking this very queue, or what it has asked and not got yet leads
  /// there.
  bool _waitsFor(InTurn queue) =>
      identical(this, queue) ||
      (_now?.asked.any((other) => other._waitsFor(queue)) ?? false);

  /// Runs `work` when everything asked for before has ended, well or badly, and
  /// answers with what it answers. The pieces after one that failed run all the same,
  /// in their order.
  Future<T> run<T>(Future<T> Function() work) {
    final asked = Zone.current;
    // The pieces the caller is inside of that still run: they wait for this one.
    final callers = [
      for (final piece in _around)
        if (identical(piece.queue._now, piece)) piece,
    ];
    final own = callers.any((piece) => identical(piece.queue, this));
    final refused = callers.any((piece) => _waitsFor(piece.queue));
    final Future<T> ran;
    if (refused) {
      ran = Future.error(
        StateError(
          own
              ? 'A piece of work asked its own queue for more work; it would '
                    'wait for that for ever.'
              : 'A piece of work asked a queue whose piece is waiting for the '
                    'queue of the asker; the two would wait for each other for '
                    'ever.',
        ),
        StackTrace.current,
      );
    } else {
      for (final piece in callers) {
        piece.asked.add(this);
      }
      ran = _last.then((_) {
        final piece = _Piece(this);
        _now = piece;
        return runZoned(
          work,
          zoneValues: {
            _inside: [..._around, piece],
          },
          zoneSpecification: _pieceZone,
        );
      });
    }
    final answer = _Answer<T>(ran);
    void unheard(Object error, StackTrace stack) {
      if (answer.taken) return;
      try {
        (onUnheard ?? asked.handleUncaughtError)(error, stack);
      } catch (error, stack) {
        asked.handleUncaughtError(error, stack);
      }
    }

    if (refused) {
      ran.then<void>((_) {}, onError: unheard);
      return answer;
    }
    void ended() {
      _now = null;
      for (final piece in callers) {
        piece.asked.remove(this);
      }
    }

    // The chain itself never fails, or one error would end every later piece.
    _last = ran.then<void>(
      (_) => ended(),
      onError: (Object error, StackTrace stack) {
        ended();
        unheard(error, stack);
      },
    );
    return answer;
  }

  /// Runs `work` in its turn, or at once when the caller is inside the piece that
  /// runs now: its turn is then, and it is part of that piece. For work that belongs
  /// in this queue and may be asked for by a piece of the queue itself.
  Future<T> join<T>(Future<T> Function() work) =>
      _around.any((piece) => identical(piece, _now))
      ? Future.sync(work)
      : run(work);

  /// Runs `work` in its turn for no one: there is no answer to take, so what it
  /// throws always goes to `onUnheard` (or the zone), and so does the refusal of
  /// [run]. For what the user does, which nothing waits for.
  void post(Future<void> Function() work) {
    run(work);
  }
}

/// A piece of work that runs or ran in a queue.
class _Piece {
  _Piece(this.queue);

  final InTurn queue;

  /// The queues it has asked for something that has not ended yet, once for each.
  final List<InTurn> asked = [];
}

/// The answer of a piece of work, which knows whether anyone has taken it.
class _Answer<T> implements Future<T> {
  _Answer(this._answer);

  final Future<T> _answer;
  bool taken = false;

  Future<T> get _taken {
    taken = true;
    return _answer;
  }

  @override
  Future<R> then<R>(
    FutureOr<R> Function(T value) onValue, {
    Function? onError,
  }) => _taken.then(onValue, onError: onError);

  @override
  Future<T> catchError(Function onError, {bool Function(Object error)? test}) =>
      _taken.catchError(onError, test: test);

  @override
  Future<T> whenComplete(FutureOr<void> Function() action) =>
      _taken.whenComplete(action);

  @override
  Stream<T> asStream() => _taken.asStream();

  @override
  Future<T> timeout(Duration limit, {FutureOr<T> Function()? onTimeout}) =>
      _taken.timeout(limit, onTimeout: onTimeout);
}

/// An open document: the one way the app, and its tests, reach a document of the core.
///
/// **Calls are carried out in the order they were made.** The handle underneath
/// ([OpenDocument]) keeps two calls apart but not in order, so two calls that are both
/// under way may be carried out either way round (characters typed in the wrong order,
/// a command refused because the one before it has not happened yet). This class sends
/// one call, waits for its answer and only then sends the next, so a caller need not
/// wait for one call before making another. Do not keep or use the handle itself.
///
/// **Errors.** Every call answers with a value or throws. A [BridgeError] says what the
/// core or the bridge refused, with a kind to switch on; after it the document is as it
/// was before the call. An error reaches the caller of the call it belongs to and no
/// one else: the calls made after it are carried out all the same.
///
/// **After a fault in the program** (a panic in the core) the call that hit it throws a
/// [PanicException], and every later call throws a [BridgeError] of kind
/// [ErrorKind.internal]. If the fault was in a call that changes the document, the
/// document may be half changed, and nothing more is saved from it: its file is as it
/// was last saved. After a fault in a call that only reads or saves, the document is
/// whole, and letting go of it saves it as letting go of any document does. [isFatal]
/// is true for both: [discard]
/// the session and open the file again. Show [describeError], never the exception's
/// own text, which can name files of the machine that built the app.
///
/// **Ending.** [close] saves and gives the document up. A session that is let go of
/// without it is saved by the garbage collector if that can be done without going over
/// another program's work, but that reports nothing, holds up the thread that draws for
/// the time of the save, and does not happen when the system ends the app.
///
/// **One document per file.** While this session is open, opening its file again in
/// this app is refused ([ErrorKind.alreadyOpen]).
class Session {
  Session._(this._doc, this.id, this.root, this.path);

  /// A session on a handle that is already open, for tests that put a stand-in for the
  /// core under it.
  @visibleForTesting
  static Future<Session> over(OpenDocument doc) async {
    try {
      return Session._(doc, await doc.id(), await doc.root(), await doc.path());
    } catch (_) {
      // No session, so no one to let go of the handle: its file must not stay taken.
      doc.dispose();
      rethrow;
    }
  }

  /// Opens the document in the file at `path`.
  static Future<Session> open(String path) async =>
      over(await OpenDocument.open(path: path));

  /// Makes a new document with an empty root cell and saves it at `path`, where no
  /// file may be yet ([ErrorKind.alreadyExists]).
  static Future<Session> create(String path) async =>
      over(await OpenDocument.create(path: path));

  /// Makes a copy of the document in the file `from` at `to`, where no file may be yet,
  /// and opens the copy. The copy has new IDs for itself and every cell: this is how a
  /// file that many people have, such as the example that comes with the app, becomes
  /// a document of one's own (decision A28). Never copy a document's file by hand.
  /// `from` may be open; what is copied is its file, not what was done since it was
  /// saved. If the copy fails, nothing of it stays.
  static Future<Session> openCopy({
    required String from,
    required String to,
  }) async => over(await OpenDocument.openCopy(from: from, to: to));

  final OpenDocument _doc;
  final _turns = InTurn();
  bool _gone = false;

  /// The permanent ID of the document.
  final String id;

  /// The ID of the root cell.
  final String root;

  /// The file the document is saved to, as a path from the root: the path it was opened
  /// with, made absolute at that moment (decision A149).
  final String path;

  /// What every call goes through: its turn, and the refusal once the document is gone.
  Future<T> _call<T>(Future<T> Function() call) => _turns.run(() {
    if (_gone) {
      throw const BridgeError(
        kind: ErrorKind.closed,
        message: 'the document was closed',
      );
    }
    return call();
  });

  /// Whether nothing was done to the document since it was read from or written to its
  /// file. A flag, not a comparison: any change clears it, undo and redo included (also
  /// when the two together lead back to what was saved), and a command that changed
  /// nothing leaves it. It does not look at the file: that another program removed or
  /// changed it shows at [save].
  Future<bool> isSaved() => _call(_doc.isSaved);

  /// The cell `cell` and the cells of its grid, row by row, each with its text, what it
  /// shows if it is a formula cell, and whether it has a grid. A cell without a grid
  /// gives 0 by 0 cells. Every call copies all of those texts: read a grid again when a
  /// [Report] names cells of it, not on a timer.
  Future<GridInfo> grid(String cell) => _call(() => _doc.grid(cell: cell));

  /// Carries out a command and returns what it changed. What holds for every command:
  ///
  /// - **On an error nothing changed**: the document and its undo history are exactly
  ///   as before, and the document is as saved or unsaved as it was.
  /// - **A command that asks for nothing new succeeds and changes nothing** (setting a
  ///   colour that a cell already has, inserting empty text): the report is empty,
  ///   nothing is added to the undo history, and the document stays saved if it was.
  ///   ([Command.run] can change what formula cells show without changing the
  ///   document; those cells are then in `recalculated` of a report that is otherwise
  ///   empty.)
  /// - **Positions and lengths in text are counted in Unicode code points**, not in the
  ///   UTF-16 units of a Dart string ([codePoints] turns one into the other), and must
  ///   not cut a user-perceived character (a letter with its accents, a flag, an emoji
  ///   made of several). One that does, or is outside the text, is refused
  ///   ([ErrorKind.refused]); it is never moved.
  /// - **Control characters in cell text become spaces** (everything below U+0020: line
  ///   breaks and tabs too). Notes keep them. Half a surrogate pair on its own arrives
  ///   as U+FFFD.
  /// - **Cells are named by their ID as text.** Text that is no ID is refused
  ///   ([ErrorKind.badRequest]), and an ID that no cell has is [ErrorKind.notFound].
  ///   Whole numbers out of range are refused ([ErrorKind.badRequest]), never cut.
  /// - A command that changed something is one step of the undo history and empties the
  ///   redo history.
  ///
  /// The comments of the constructors of [Command] and [Prop] say what each one does
  /// and refuses.
  Future<Report> apply(Command command) =>
      _call(() => _doc.apply(command: command));

  /// Undoes the latest command and returns what that changed, or null if there is
  /// nothing to undo.
  Future<Report?> undo() => _call(_doc.undo);

  /// Redoes the command that was undone last and returns what that changed, or null if
  /// there is nothing to redo.
  Future<Report?> redo() => _call(_doc.redo);

  /// Makes the file hold the document as it is after every call made before this one,
  /// without ever going over what another program put there. It looks at the file
  /// first:
  ///
  /// - the file is as this document read or last wrote it: it is written if anything
  ///   was done since, and left alone otherwise;
  /// - the file is gone: it is written, whether anything was done or not;
  /// - the file was changed or replaced by someone else: nothing is written and the
  ///   error is [ErrorKind.changedOnDisk], whether anything was done or not. The user
  ///   chooses: [overwrite] keeps this document, [discard] and opening the file again
  ///   keeps the other;
  /// - the file cannot be looked at: [ErrorKind.io].
  ///
  /// On an error nothing was written, and the document is as unsaved as it was.
  Future<void> save() => _call(_doc.save);

  /// Writes the document to its file whatever is there, replacing what another program
  /// saved since: for when [save] answered [ErrorKind.changedOnDisk] and the user chose
  /// to keep this document.
  Future<void> overwrite() => _call(_doc.overwrite);

  /// Ends the work on the document, after every call made before this one: saves what
  /// is unsaved, as [save] does and with the same refusals, and gives the document up,
  /// so that its file may be opened again and its memory is freed.
  ///
  /// If the save fails, the error is thrown and the document stays open exactly as it
  /// was: try again, [overwrite] and close, keep working, or [discard]. With nothing
  /// unsaved, nothing is written and the file is not looked at, so closing a document
  /// that was only read never fails.
  ///
  /// After a close that succeeded every call, a second close included, throws a
  /// [BridgeError] of kind [ErrorKind.closed]; [id], [root] and [path] stay readable.
  Future<void> close() => _call(() async {
    await _doc.close();
    _gone = true;
    // Nothing is left in the handle to save, so letting go of it here costs the
    // thread that draws nothing.
    _doc.dispose();
  });

  /// Lets go of the document without asking it to save, after every call made before
  /// this one: for after a [close] or [save] that was refused with
  /// [ErrorKind.changedOnDisk] when the user chose the other program's file, and for
  /// after a fault ([isFatal]). Its file may then be opened again, and every later call
  /// throws [ErrorKind.closed].
  ///
  /// This is not a way to throw edits away: where the file is as the document last
  /// read or wrote it, or gone, the bridge still saves what is unsaved (decision A142),
  /// on the calling thread. It writes nothing over a file that someone else changed,
  /// and nothing after a fault. Does nothing on a session that was closed.
  Future<void> discard() => _turns.run(() async {
    if (_gone) return;
    _gone = true;
    _doc.dispose();
  });
}

/// How many code points the first `units` UTF-16 units of `text` are: turns a position
/// in a Dart string, such as `TextEditingController.selection` gives, into the position
/// that the core counts in ([Session.apply]). A position between the two halves of a
/// surrogate pair, where no cursor stands, counts as after the pair.
///
/// `units` is from 0 to `text.length`; anything else is refused with a [RangeError]
/// (the one `substring` throws), as the core refuses a position outside the text and
/// never moves it. That includes -1, which a `TextSelection` gives when there is no
/// selection: that is no position, so ask `selection.isValid` first.
int codePoints(String text, int units) => text.substring(0, units).runes.length;

/// Whether an error says that the document cannot be used any more: a fault in the
/// program, as it happened or as every call after it reports it.
bool isFatal(Object error) =>
    error is PanicException ||
    error is BridgeError && error.kind == ErrorKind.internal;

/// What the screen says after a fault in the program, instead of the text of the
/// exception, which can hold a backtrace with the paths of the machine that built the
/// app.
const String faultSentence =
    'Something went wrong inside Wombat, and this document cannot be used '
    'any more. Its file holds what was last saved, or more. Close Wombat and '
    'open it again.';

/// An error as text for the screen.
String describeError(Object error) => isFatal(error)
    ? faultSentence
    : error is BridgeError
    ? 'Error (${error.kind.name}): ${error.message}'
    : 'Error: $error';
