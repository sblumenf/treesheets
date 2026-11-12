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
  /// - **Cell text may hold line breaks (U+000A) and no other control character.** In
  ///   text that is sent, CR LF and a CR on its own become one U+000A, and every other
  ///   character below U+0020, a tab too, becomes a space. The text in the cell can
  ///   therefore be shorter than what was sent: read it back ([cell]) and count
  ///   positions in that, never in what was sent. Notes keep everything. Half a
  ///   surrogate pair on its own arrives as U+FFFD.
  /// - **Cells are named by their ID as text.** Text that is no ID is refused
  ///   ([ErrorKind.badRequest]), and an ID that no cell has is [ErrorKind.notFound].
  ///   Whole numbers out of range are refused ([ErrorKind.badRequest]), never cut.
  /// - A command that changed something is one step of the undo history and empties the
  ///   redo history. Inside a group ([beginGroup]) it is part of the group's step, and
  ///   the redo history waits for the end of the group.
  ///
  /// The comments of the constructors of [Command] and [Prop] say what each one does
  /// and refuses.
  Future<Report> apply(Command command) =>
      _call(() => _doc.apply(command: command));

  /// Undoes the latest step of the undo history, a command or a group of them, and
  /// returns what that changed, or null if there is nothing to undo. Refused while a
  /// group is open ([ErrorKind.refused]).
  Future<Report?> undo() => _call(_doc.undo);

  /// Redoes the step that was undone last and returns what that changed, or null if
  /// there is nothing to redo. Refused while a group is open ([ErrorKind.refused]).
  Future<Report?> redo() => _call(_doc.redo);

  /// What a screen that shows nested grids draws from, in one answer: the grid of
  /// `cell` (the stage) or a window of its rows and columns, the trail from the root
  /// down to `cell`, and every grid nested below as far as `ask` says, cell by cell or
  /// in reduced form. It only reads, and the same document and request always give the
  /// same answer, entry for entry. The work is in proportion to the answer, not to the
  /// document; nearly all of the time of a large answer is the shapes.
  ///
  /// A `cell` without a grid gives its trail and nothing else. A cell that does not
  /// exist is [ErrorKind.notFound]; a number in `ask` below 0 or above 4294967295 is
  /// [ErrorKind.badRequest]. [phoneAsk] is the request to start from.
  ///
  /// **How the answer hangs together** (no entry holds another; they name each other
  /// by their place in the four lists):
  ///
  /// - `trail`: the cells from the root to `cell`, the root first; never empty. Each
  ///   has its `id`, what it shows on one line (`shown`, cut to `previewCut`) and the
  ///   background `color` it is drawn in.
  /// - `grids`: the grids given cell by cell. `grids[0]` is the stage's own (its
  ///   window); empty if `cell` has no grid. Then the nested grids given whole, level by
  ///   level, in the order of their owners in `cells`.
  /// - `cells`: the cells of all `grids`, grid after grid, each row by row.
  /// - `summaries`: one for each cell in `cells` that has a grid, in the order of those
  ///   cells.
  /// - `level`: what this level remembers on this device ([viewGo]), or null.
  ///
  /// **A [GridPart]** (the generator left its fields without comments): `owner` the ID
  /// of the cell whose grid it is; `level` 0 for the stage's grid, 1 for the grids of
  /// its cells, and so on; `xs`, `ys` the columns and rows of the whole grid; `x0`,
  /// `y0`, `cols`, `rows` the part that is given (all of a nested grid; the window of
  /// the stage's, cut to the grid); `first` where its cells start in `cells`: the cell
  /// in column `x0 + i` of row `y0 + j` is `cells[first + j * cols + i]`; `colWidths`
  /// the width at which text wraps (TreeSheets' unit, 80 by default) of each column
  /// given; `borderColor`, `outerSpacing`; `vertical` whether the grid is below its
  /// owner's text and not beside it; `drawStyle` how the owner wants it drawn.
  ///
  /// **A [CellLook]**: `id`; `text` (the formula of a formula cell; it may hold line
  /// breaks) and `textCut`, whether the text was cut to `textCut` of the request (only
  /// ever between two user-perceived characters); `kind`; `result`, what a formula
  /// cell shows in place of its text, with `resultCut`, and `failed` when that is an
  /// error value; `cellColor` and `textColor`, the colours to draw (a tag's, if the
  /// cell is one); `isTag` whether its whole text is the name of a tag of the document
  /// (no run has a colour then); `style` the base style and `relSize` the text size
  /// against the default (positive is smaller); `textAlign`, `vertAlign`; `runs`, the
  /// ranges styled otherwise, and `links`, the ranges that are links, both sorted, not
  /// overlapping, inside `text` as given, and **counted in code points** ([units]
  /// makes them offsets of the string); `image` the file name of its image in the
  /// document's image folder; `hasNote`; `hasGrid`; `folded` whether its grid is
  /// folded on this device (the answer holds the grid all the same: hiding it is the
  /// screen's); `summary` the place of its entry in `summaries`, set exactly when it
  /// has a grid; `full` the place of its grid in `grids`, if that is given cell by
  /// cell.
  ///
  /// **A [GridSummary]**, from which a Preview, a Shape and a Badge are drawn: `owner`;
  /// `xs`, `ys` (their product is the count a Badge shows); `borderColor`;
  /// `previewCols` by `previewRows` texts in `preview`, row by row, what the grid's
  /// top-left cells show, each on one line; `shapeCols` by `shapeRows` cells in
  /// `shape`, row by row, four bytes each: red, green and blue of the cell's
  /// background, then how much it holds: 255 a grid, else the code points it shows up
  /// to 254, so 0 is a cell that shows nothing.
  ///
  /// **Comparing two answers.** `==` on a [TrailStep], [GridPart], [CellLook] or
  /// [GridSummary] compares everything in it; `==` on [StageInfo] itself does not (it
  /// asks whether its lists are the very same objects). The places (`first`,
  /// `summary`, `full`) move when a cell before them gains or loses a grid. The lists
  /// inside a [CellLook], [GridPart] or [GridSummary] are wrapped anew at every read
  /// of the field: take `look.runs` once, not inside a loop. `StageRead` and
  /// `StageReader` (`stage_reader.dart`) do the comparing for a screen.
  Future<StageInfo> stage(String cell, StageAsk ask) =>
      _call(() => _doc.stage(cell: cell, ask: ask));

  /// One cell with its whole text, runs and links, nothing cut: what an editor opens
  /// on. The fields are those of a cell of [stage]; `summary` and `full` are null. A
  /// cell that does not exist is [ErrorKind.notFound].
  Future<CellLook> cell(String cell) => _call(() => _doc.cell(cell: cell));

  /// Opens a group: the commands carried out until the matching [endGroup] are one
  /// step of the undo history, or none at all if the group is cancelled
  /// ([cancelGroup]). Opening one changes nothing in the document.
  ///
  /// - Each command inside is carried out at once and reports what it changed.
  /// - Groups may be nested; only the outermost counts.
  /// - While a group is open, [undo] and [redo] are refused, and what could be redone
  ///   before stays.
  /// - **A group is a state of the document, not of the caller**: every command from
  ///   anywhere joins it until it is ended or cancelled.
  /// - Saving does not wait for the end: [save], [overwrite], [close] and [discard]
  ///   write the document as it is, with what the group did so far, and leave the
  ///   group as it is (a close that succeeds gives it up with the document).
  Future<void> beginGroup() => _call(_doc.beginGroup);

  /// Closes a group. Closing the outermost returns what the group changed as a whole,
  /// between the document before its first command and the document now, with every
  /// formula cell that a command of the group reported and that still exists in
  /// `recalculated`. If anything changed, the group is one step of the undo history
  /// and the redo history is emptied; if nothing did, both are as they were. Closing
  /// an inner group, or with no group open, does nothing and returns an empty report.
  Future<Report> endGroup() => _call(_doc.endGroup);

  /// Abandons the open group, the outermost with every group inside it: the document,
  /// the undo history and the redo history are exactly as when it was opened, and no
  /// group is open. Returns what the taking back changed, from the document just
  /// before this call. Without an open group it does nothing and returns an empty
  /// report.
  ///
  /// The document is then as saved as it was when the group was opened, unless its
  /// file was written in between: then the file holds what was taken back, and the
  /// document is unsaved ([isSaved] says which). Not taken back: the variables a `Run`
  /// inside the group left, and what the view state forgot because a command of the
  /// group removed a cell.
  Future<Report> cancelGroup() => _call(_doc.cancelGroup);

  /// Whether a group is open.
  Future<bool> inGroup() => _call(_doc.inGroup);

  /// Makes the text of `cell` this text by replacing only the part that differs, so
  /// that the styles and links of the rest stay, where `setText` and `enterText` drop
  /// them all: for the text of an editor. It is one command with everything [apply]
  /// says: one step of the undo history, or part of the open group.
  ///
  /// - `text` is first made what a cell may hold (see [apply]), so **the text in the
  ///   cell afterwards can differ from `text`**: read it back ([cell]).
  /// - A text that the cell has already changes nothing: the report is empty.
  /// - **The kind of the cell is left alone**: a formula cell stays one, and text that
  ///   starts with `=` does not become a formula. Follow with `Command.enterText` of
  ///   the same text in the same group to settle that as typing does; it finds the
  ///   text there and keeps the styles.
  /// - The difference is one range: after two changes far apart it spans both, and
  ///   what was typed takes the style and link of the first character replaced.
  Future<Report> retype(String cell, String text) =>
      _call(() => _doc.retype(cell: cell, text: text));

  /// The level to show: the cell whose grid fills the screen. That is the level last
  /// gone to on this device ([viewGo]), also before the app was last closed; if that
  /// cell is gone or has lost its grid, the nearest cell above it that has one;
  /// otherwise the root. Always a cell of the document: where a screen starts, and
  /// where it goes when its stage disappears.
  ///
  /// This and the five calls below are about the **view state**: which level is
  /// shown, what each level remembers and which grids are folded. It is this device's,
  /// not the document's: changing it is no command, is not in the undo history and
  /// never makes the document unsaved. It follows every change by itself (what refers
  /// to a removed cell is forgotten, and does not come back with the cell), and is
  /// written to a file of its own by [saveView], by a [close] that succeeds and by
  /// [discard].
  Future<String> viewStage() => _call(_doc.viewStage);

  /// Makes `cell` the level that is shown, after remembering in `left`, if given, how
  /// the level that is left looked: its `cell`, the column `x` and row `y` of its grid
  /// at the top left of what was seen (counted from 0; a column and a row, not a
  /// distance), and the cell `selected` there, if one was. [stage] gives that back as
  /// `level` when the level is shown again.
  ///
  /// Refused, with nothing remembered and nowhere gone: a `cell` without a grid, except
  /// the root ([ErrorKind.refused]); a `left` whose cell has no grid, the root
  /// included ([ErrorKind.refused]); a cell that does not exist, `left.selected` too
  /// ([ErrorKind.notFound]); a position below 0 or above 4294967295
  /// ([ErrorKind.badRequest]).
  Future<void> viewGo(String cell, {LevelInfo? left}) =>
      _call(() => _doc.viewGo(cell: cell, left: left));

  /// Remembers how a level looks without going anywhere: for the level that is shown,
  /// before the app is put away. It replaces what was remembered of that level; the
  /// 256 levels remembered last are kept. Refused as `left` of [viewGo] is, which
  /// includes the root of a document that has no grid yet.
  Future<void> viewRemember(LevelInfo level) =>
      _call(() => _doc.viewRemember(level: level));

  /// Folds (hides) or unfolds the grid of `cell` on this device, and returns whether
  /// that changed anything. A cell without a grid is refused ([ErrorKind.refused]).
  /// The fold goes when the cell or its grid is removed, and does not come back.
  Future<bool> setFolded(String cell, {required bool folded}) =>
      _call(() => _doc.setFolded(cell: cell, folded: folded));

  /// Whether the view state is as its file has it: nothing of it changed since it was
  /// read or written, by a call or by a change to the document that removed something
  /// it referred to. True also when there is no file and nothing to put in one.
  Future<bool> isViewSaved() => _call(_doc.isViewSaved);

  /// Writes the view state to its file if it changed since it was read or written.
  /// It has nothing to do with the document's file: it is written also when the
  /// document cannot be saved, and an error here ([ErrorKind.io], or
  /// [ErrorKind.invalid] when `.wombat-view` beside the document is a link or a file)
  /// says nothing about the document. Nothing is lost by it but where the screen was:
  /// never show it as a failed save.
  Future<void> saveView() => _call(_doc.saveView);

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
  /// A group that is open is neither ended nor cancelled: the document is saved as it
  /// is and the group given up with it; after a close that failed it is still open.
  /// The view state is written too if it changed, after the document; that it could
  /// not be is not reported ([saveView] first, to know).
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
/// surrogate pair, where no cursor stands, counts as after the pair. A line break is
/// one code point and one unit, in the document always: the core keeps a line end as
/// U+000A alone, so count in the text the document gave back, not in one that may
/// still hold CR LF.
///
/// `units` is from 0 to `text.length`; anything else is refused with a [RangeError]
/// (the one `substring` throws), as the core refuses a position outside the text and
/// never moves it. That includes -1, which a `TextSelection` gives when there is no
/// selection: that is no position, so ask `selection.isValid` first.
int codePoints(String text, int units) => text.substring(0, units).runes.length;

/// How many UTF-16 units the first `codePoints` code points of `text` are: turns a
/// position that the core counts in, such as the `start` of a [RunInfo] or a
/// [LinkInfo], into an offset of the Dart string, as a `TextSpan` or a
/// `TextSelection` needs it. The other way round from [codePoints]: for every
/// position `n` in a text, `codePoints(text, units(text, n)) == n`.
///
/// It counts and nothing more: a character made of several code points (a letter
/// with accents, a flag, a line break of the document, which is one) is as many as it
/// is made of. `codePoints` is from 0 to the number of code points of `text`;
/// anything else is refused with a [RangeError], never moved. Half a surrogate pair
/// on its own counts as one code point of one unit, as [codePoints] counts it.
int units(String text, int codePoints) {
  if (codePoints < 0) throw RangeError.range(codePoints, 0, null, 'codePoints');
  var at = 0;
  for (var left = codePoints; left > 0; left--) {
    // Past the end of the text this throws, a [RangeError] too.
    final unit = text.codeUnitAt(at);
    final pair =
        unit & 0xFC00 == 0xD800 &&
        at + 1 < text.length &&
        text.codeUnitAt(at + 1) & 0xFC00 == 0xDC00;
    at += pair ? 2 : 1;
  }
  return at;
}

/// What a phone's screen asks of the stage to start with: the core's own starting
/// values, of which the comment of [StageAsk] tells. The whole stage grid; one level
/// of nested grids cell by cell where a grid has at most 24 cells; previews of 3 by 4
/// texts cut at 40 code points; shapes of 12 by 20 cells; a cell's text cut at 2000
/// code points.
///
/// Measured from Dart on a desktop (profile build; `docs/milestones/B1.md` has the
/// table): a stage of 240 cells with grids on six levels below it is read in 11 to
/// 14 ms, of which the thread that draws is held 3 ms to decode the answer and 2 ms
/// to compare it with the one before. One more level cell by cell (`fullDepth` 2)
/// holds that thread 20 to 27 ms, more than a frame, so it is not the start. A
/// stage of 1,000 cells that all hold grids takes 54 to 78 ms, 10 to 14 ms of it on
/// the thread that draws: for a grid that large, [phoneWindowAsk].
const phoneAsk = StageAsk(
  x0: 0,
  y0: 0,
  cols: 4294967295,
  rows: 4294967295,
  fullDepth: 1,
  fullCells: 24,
  previewCols: 3,
  previewRows: 4,
  shapeCols: 12,
  shapeRows: 20,
  textCut: 2000,
  previewCut: 40,
);

/// [phoneAsk] for a stage grid of any length: 40 rows of it at a time. A grid of at
/// most 40 rows is given whole, as by [phoneAsk]; of a longer one this gives the
/// rows the screen is at, and the screen says where it is as it scrolls
/// (`StageReader.see` in `keeper.dart`). Measured as [phoneAsk] says: 40 rows of 10
/// cells that all hold grids are read in 22 to 28 ms, 4 to 6 ms of it on the thread
/// that draws, where the 100 rows of that grid whole take 54 to 78 ms and 10 to 14.
///
/// A screen that asks this must move the window, or the rows after the fortieth
/// are never shown: that is why it is not what a reader asks unless told.
const phoneWindowAsk = StageAsk(
  x0: 0,
  y0: 0,
  cols: 4294967295,
  rows: 40,
  fullDepth: 1,
  fullCells: 24,
  previewCols: 3,
  previewRows: 4,
  shapeCols: 12,
  shapeRows: 20,
  textCut: 2000,
  previewCut: 40,
);

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
