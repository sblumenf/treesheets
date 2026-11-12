import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';

import 'session.dart';
import 'src/rust/api/wire.dart';
import 'stage_read.dart';

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
/// **What a screen draws from is read here** ([read]): a [StageReader] holds the
/// stage as last read, is read again by the keeper after every change, whoever made
/// it, and says what differs.
///
/// **A cell's text is edited through an [Editing]** ([edit]): nothing is sent while
/// the user types, one editing is one step of the undo history, and dropping it
/// leaves the document and both histories exactly as they were. While an editing
/// holds something in the document, every other change is refused ([apply]).
///
/// **Where the screen is belongs to this device** (the view state): the level shown,
/// what each level remembers, what is folded. The keeper writes it to its own file
/// with every save of the document, and a failure to do so is never a failed save
/// ([viewError]).
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

  /// The stages that are read again after every change.
  final _readers = <StageReader>[];

  /// Whether something was done since the readers last read.
  bool _stale = false;

  /// The editing that is open, if one is.
  Editing? _editing;
  Object? _viewError;

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

  /// What the last try to write the view state ended with, if it failed: where the
  /// screen is may not be in its file. It says nothing about the document, is never
  /// part of [state], and listeners are not told of it. Null when the last try worked.
  Object? get viewError => _viewError;

  /// The editing that is open ([edit]), or null.
  Editing? get editing => _editing;

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
  /// it says the same: the core makes a new one for every failure, and two of them
  /// are equal when their kind and message are. ([ended] and [opens] are not here:
  /// they change only when [state] does.)
  Object get _view => (state, _saveError);

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
  /// Whatever it did to the document, well or badly, the stages are read again
  /// before it answers: who asked finds them as the document is then.
  Future<T> _piece<T>(Future<T> Function() work) => _turns.run(() async {
    _refuseAfterTheEnd();
    try {
      return await work();
    } finally {
      await _reread();
      _tell();
    }
  });

  /// Reads the stages again if something was done since they were read. It never
  /// throws: that a stage could not be read is the reader's to say, and the piece
  /// that made the change has its own answer to give.
  Future<void> _reread() async {
    if (!_stale || _ended != null) return;
    _stale = false;
    for (final reader in [..._readers]) {
      try {
        await reader._again(_doc);
      } catch (error) {
        if (isFatal(error)) return _end(faultSentence);
        reader._failed(error);
      }
    }
  }

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
    _editing?._gone();
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
    if (report == null) return report;
    final same =
        report.changed.isEmpty &&
        report.created.isEmpty &&
        report.removed.isEmpty &&
        report.grids.isEmpty &&
        report.tags.isEmpty;
    // What a formula cell shows is on the screen, though in no file.
    if (!same || report.recalculated.isNotEmpty) _stale = true;
    if (same) return report;
    _unsaved = true;
    _changes++;
    if (_saveTimer == null) _saveIn(_saveAfter);
    return report;
  }

  /// A cell and the cells of its grid ([Session.grid]).
  Future<GridInfo> grid(String cell) => _use((doc) => doc.grid(cell));

  /// One read of a stage ([Session.stage]) that nobody keeps up to date: for a look
  /// at something beside the screen's own stage, such as the larger preview of a
  /// cell's grid. The screen's own stage is a [StageReader] ([read]).
  Future<StageInfo> stage(String cell, StageAsk ask) =>
      _use((doc) => doc.stage(cell, ask));

  /// One cell whole, nothing cut ([Session.cell]).
  Future<CellLook> cell(String cell) => _use((doc) => doc.cell(cell));

  /// An editing holds a group, which every command from anywhere would join, and in
  /// which the core refuses undo and redo. So whoever asks, other than the editing
  /// itself, has not ended the editing first: a mistake in the app, said loudly.
  void _refuseBesideAnEditing(String what) {
    if (_editing?.held ?? false) {
      throw StateError(
        'The text of a cell is being edited and part of that is in the '
        'document, so nothing else can $what now. Enter or drop the editing '
        'first (Editing.enter, Editing.drop).',
      );
    }
  }

  /// Carries out a command ([Session.apply]), and saves it a while later.
  ///
  /// Refused with a [StateError], and nothing done, while an [Editing] holds
  /// something in the document ([Editing.held]); so are [undo] and [redo]. A screen
  /// enters or drops its editing before it does anything else.
  Future<Report> apply(Command command) => _use((doc) async {
    _refuseBesideAnEditing('change the document');
    return _done(await doc.apply(command));
  });

  /// Undoes the latest step ([Session.undo]): a command, or a whole editing. Saves
  /// that a while later. Refused while an editing holds something ([apply]).
  Future<Report?> undo() => _use((doc) async {
    _refuseBesideAnEditing('undo');
    return _done<Report?>(await doc.undo());
  });

  /// Redoes the step undone last ([Session.redo]), and saves that a while later.
  /// Refused while an editing holds something ([apply]).
  Future<Report?> redo() => _use((doc) async {
    _refuseBesideAnEditing('redo');
    return _done<Report?>(await doc.redo());
  });

  /// Starts to keep a stage for a screen: the level that this device showed last
  /// ([Session.viewStage]; the root at first), read with `ask`. The keeper reads it
  /// again after every change to the document, by whomever. [StageReader.close] ends
  /// that.
  ///
  /// With [phoneAsk] the stage's grid is read whole, however long: never wrong, and
  /// slow for a grid of a thousand cells that hold grids. A screen that moves a
  /// window as it scrolls ([StageReader.see]) asks [phoneWindowAsk].
  Future<StageReader> read({StageAsk ask = phoneAsk}) => _use((doc) async {
    final reader = StageReader._(this, ask);
    await reader._start(doc);
    _readers.add(reader);
    return reader;
  });

  /// Opens an editing of the text of `cell` and reads the cell whole for it
  /// ([Editing.start]): what the editor opens on is what the cell holds when the
  /// turn of this call comes, not what a screen showed when it was tapped. Nothing
  /// is changed, and no group is opened, until the editing is asked to put
  /// something into the document.
  ///
  /// One editing is open at a time. An editing that was open and holds nothing in
  /// the document is over from here on (nothing of it was ever sent). While one
  /// holds something this is refused with a [StateError]: enter or drop that first.
  /// A cell that does not exist is [ErrorKind.notFound].
  Future<Editing> edit(String cell) => _use((doc) async {
    _refuseBesideAnEditing('be edited');
    final editing = Editing._(this, await doc.cell(cell));
    _editing?._gone();
    return _editing = editing;
  });

  /// Folds (hides) or unfolds the grid of `cell` on this device
  /// ([Session.setFolded]) and answers whether that changed anything. The stages are
  /// read again: the cell's `folded` is what differs. It is no change to the
  /// document and cannot be undone.
  Future<bool> setFolded(String cell, {required bool folded}) =>
      _use((doc) async {
        final changed = await doc.setFolded(cell, folded: folded);
        if (changed) _stale = true;
        return changed;
      });

  /// Whether the view state is as its file has it ([Session.isViewSaved]). A
  /// position that a [StageReader] was told of ([StageReader.at]) and that was not
  /// handed on yet does not count.
  Future<bool> isViewSaved() => _use((doc) => doc.isViewSaved());

  /// Hands on where every stage is ([StageReader.at]), if that moved, and writes the
  /// view state to its file if it changed. Whatever goes wrong here but a fault is
  /// kept in [viewError] and is nobody else's concern: nothing is lost but where the
  /// screen was.
  Future<void> _keepView() async {
    try {
      for (final reader in [..._readers]) {
        await reader._remember(_doc);
      }
      await _doc.saveView();
      _viewError = null;
    } catch (error) {
      if (isFatal(error)) return _end(faultSentence);
      _viewError = error;
    }
  }

  /// After an editing was dropped, the core says whether anything is left to save: a
  /// cancelled editing leaves a saved document saved, unless its file was written
  /// meanwhile and holds what was taken back.
  Future<void> _afterDrop(Session doc) async {
    _stale = true;
    if (await doc.isSaved()) {
      _unsaved = false;
      // A save that is being tried again finds nothing to write, and ends well.
      if (_saveError == null) {
        _saveTimer?.cancel();
        _saveTimer = null;
      }
    } else {
      _unsaved = true;
      _changes++;
      if (_saveTimer == null) _saveIn(_saveAfter);
    }
  }

  /// One try to make the file hold the document, in its turn. How it ended is in
  /// [state] and [saveError].
  Future<void> _write(Future<void> Function(Session doc) how) => _piece(
    () async {
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
      // After the document, and whether or not that could be written: where the
      // screen is can neither hold up nor fail a save.
      if (_ended == null) await _keepView();
    },
  );

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
  ///
  /// What an open [Editing] holds in the document is written with the rest, and the
  /// editing stays as it is. The view state is written after the document, if it
  /// changed ([viewError]).
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
      for (final attachment in [..._attached]) one(attachment),
    ]);
    return failed;
  }

  /// The app is out of sight, and the system may end it without another word. What
  /// is in the document goes into its file first; then what is being typed on the
  /// screens goes into the document, and that into the file.
  ///
  /// A screen with an editor open holds its text in the document then
  /// ([Editing.hold]): the file has what was typed, and the editing stays open, to be
  /// entered or dropped when the app comes back. Where each stage is goes into the
  /// view state and its file with the first save.
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
  /// other. An [Editing] that is open is over ([Editing.open]), with what it held:
  /// its screen closes the editor without entering. Every [StageReader] starts again
  /// at the level the view state names for the document that was opened, and says
  /// so ([StageChange.everything]).
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
      // What was being edited was of the document that is gone, group and all.
      _editing?._gone();
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
      _viewError = null;
      // The stages start again where the view state of the file says.
      for (final reader in [..._readers]) {
        try {
          await reader._start(_doc);
        } catch (error) {
          if (isFatal(error)) return _end(faultSentence);
          reader._failed(error);
        }
      }
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
  ///
  /// An [Editing] that is still open then is neither entered nor dropped by the
  /// keeper: what it holds in the document is saved as it is, and the editing ends
  /// with the document. After a close that failed it is as open as before. Where
  /// each stage is goes into the view state, which is written with the document.
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
      // Before the document, since nothing can be asked of it after: but what goes
      // wrong with it does not keep the document from closing.
      await _keepView();
      _refuseAfterTheEnd();
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
      _editing?._gone();
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

/// A report in which nothing changed.
const _nothing = Report(
  changed: [],
  created: [],
  removed: [],
  grids: [],
  tags: [],
  recalculated: [],
);

/// One editing of the text of one cell ([Keeper.edit]): from the editor opening on
/// the cell to the tick or the cross.
///
/// **Nothing is sent while the user types.** The text is the editor's until one of
/// the four calls here hands it over; each is one turn of the keeper, so nothing of
/// anyone else comes between its steps.
///
/// **One editing is one step of the undo history, or none.** The first call that
/// puts something into the document opens a group there ([Session.beginGroup]), and
/// the editing [held]s something from then on. [enter] ends the group: everything
/// the editing did, styles included, is one step. [drop] cancels it: the document,
/// the undo history and the redo history are exactly as when the editing was
/// opened, and a document that was saved then is saved again, unless its file was
/// written meanwhile.
///
/// **While the editing holds something, the keeper refuses every other change**
/// ([Keeper.apply], [Keeper.undo], [Keeper.redo], [Keeper.edit]) with a
/// [StateError]. Saving goes on: what is held is written with the rest
/// ([Keeper.save], [Keeper.putAway]).
///
/// **The text in the cell can differ from the text that was sent** (CR LF becomes
/// one line break, other control characters become spaces): [hold] and [style]
/// answer with the cell as the document has it, and positions are counted in that.
///
/// **An editing ends** with [enter] or [drop], and with its document: when the keeper
/// closes it, lets go of it, opens the other program's file or meets a fault. After
/// the end [open] is false and every call throws: a [StateError], or once the keeper's
/// work has ended the keeper's own [BridgeError] of kind [ErrorKind.closed]. A screen that goes
/// away enters or drops its editing: one that is forgotten while it holds something
/// keeps every other change out until the document is closed.
class Editing {
  Editing._(this._keeper, this.start);

  final Keeper _keeper;

  /// The cell as the document had it when the editing was opened, with its whole
  /// text: what the editor opens on.
  final CellLook start;
  bool _open = true;
  bool _held = false;

  /// The ID of the cell that is edited.
  String get cell => start.id;

  /// Whether the editing goes on: it was neither entered nor dropped, and its
  /// document is still the keeper's.
  bool get open => _open;

  /// Whether something of this editing is in the document, in a group that is open:
  /// [hold] or [style] put it there and neither [enter] nor [drop] has ended it.
  bool get held => _held;

  /// The editing is over, by its own end or with its document.
  void _gone() {
    _open = false;
    _held = false;
    _keeper._editing = null;
  }

  void _refuseAfterItsEnd() {
    if (!_open) {
      throw StateError(
        'This editing is over: it was entered or dropped, another was opened '
        'in its place, or its document is no longer open.',
      );
    }
  }

  /// Puts `text` into the cell as part of the editing's group, opening the group if
  /// this is the first thing. Only what differs is replaced, so the styles and links
  /// of the rest stay ([Session.retype]); then the kind of the cell is settled as
  /// typing settles it (`Command.enterText`: text that starts with `=` is a formula).
  ///
  /// If the core refuses the text and this call opened the group, the group is
  /// cancelled: the document is exactly as before the call and the editing holds
  /// nothing. If the group was open before, it stays open, with what the core took
  /// before it refused.
  Future<void> _put(Session doc, String text) async {
    final opened = !_held;
    if (opened) {
      await doc.beginGroup();
      _held = true;
    }
    try {
      _keeper._done(await doc.retype(cell, text));
      _keeper._done(await doc.apply(Command.enterText(cell: cell, text: text)));
    } catch (error) {
      if (opened && !isFatal(error)) {
        await doc.cancelGroup();
        _held = false;
        await _keeper._afterDrop(doc);
      }
      rethrow;
    }
  }

  /// Puts the editor's `text` into the document without ending the editing: for when
  /// the app is put away (the text is then in the file with the next save) and
  /// before something is done to the text where it is (a style, a link). Answers
  /// with the cell as the document has it now; its text is the one to count
  /// positions in, and can differ from `text`.
  ///
  /// The cross still takes it all back ([drop]). A text that the core refuses
  /// throws its [BridgeError], and the editing is as it was: open, the editor's
  /// text untouched, holding what it held.
  Future<CellLook> hold(String text) => _keeper._use((doc) async {
    _refuseAfterItsEnd();
    await _put(doc, text);
    return doc.cell(cell);
  });

  /// Holds the editor's `text` ([hold]) and then carries out a command on it as
  /// part of the same editing: bold, italic, a link, a colour on a range. `command`
  /// is given the cell as the document has it after the hold, and builds the
  /// command from that: count its positions in that text ([codePoints]). Answers
  /// with the cell after the command.
  ///
  /// If the core refuses the command, its error is thrown; the text stays held.
  Future<CellLook> style(
    String text,
    Command Function(CellLook held) command,
  ) => _keeper._use((doc) async {
    _refuseAfterItsEnd();
    await _put(doc, text);
    _keeper._done(await doc.apply(command(await doc.cell(cell))));
    return doc.cell(cell);
  });

  /// The tick: the cell's text becomes `text`, and the editing ends as one step of
  /// the undo history, with everything it did before. Answers with what the editing
  /// changed as a whole ([Session.endGroup]); a text the cell had already, with
  /// nothing else done, changes nothing and leaves nothing to undo.
  ///
  /// If the core refuses the text, its [BridgeError] is thrown and the editing
  /// stays open with what it held before, so that the editor can stay open with
  /// what was typed.
  Future<Report> enter(String text) => _keeper._use((doc) async {
    _refuseAfterItsEnd();
    await _put(doc, text);
    // What it names, each command of the group has reported already.
    final report = await doc.endGroup();
    _gone();
    return report;
  });

  /// The cross: the editing ends and nothing of it stays. If it held something, the
  /// document, the undo history and the redo history are exactly as when it was
  /// opened, and the answer says what taking it back changed
  /// ([Session.cancelGroup]). If it held nothing, nothing was ever sent, and the
  /// answer is empty.
  Future<Report> drop() => _keeper._use((doc) async {
    _refuseAfterItsEnd();
    if (!_held) {
      _gone();
      return _nothing;
    }
    final report = await doc.cancelGroup();
    _gone();
    await _keeper._afterDrop(doc);
    return report;
  });
}

/// The stage as a screen draws it, kept up to date by the [Keeper] ([Keeper.read]).
///
/// **It is read again after every change to the document**, in the turn of the call
/// that made the change and before that call answers: whoever awaited
/// `keeper.apply(...)` finds [stage] as the document is then. The same holds for
/// undo, redo, the calls of an [Editing] and folding, and for changes that another
/// screen made. Each time, the new answer is compared with the one before, and the
/// listeners are told what differs ([StageChange]): only that is drawn again.
///
/// **It goes where the document's levels go.** [go] makes another cell the stage and
/// tells the view state, with where this level was ([at]). If the stage cell is
/// removed or loses its grid (an undo can do that), the stage becomes the nearest
/// level above it that is left, and the listeners are told that everything differs.
/// The same after the keeper opened the other program's file.
///
/// **Reading does not call for every scroll.** A screen tells the reader where it is
/// with [at], which costs nothing; the keeper hands that on when the level is left,
/// with every save, and when the app is put away or closed.
class StageReader {
  StageReader._(this._keeper, this._wanted);

  final Keeper _keeper;

  /// What the screen asked for, before a window was moved.
  StageAsk _wanted;
  late StageRead _stage;
  StageChange _change = StageChange.all;
  Object? _error;
  final _listeners = <void Function(StageChange change)>[];

  /// Where the screen said it is on this level, and whether the view state was told.
  ({int x, int y, String? selected})? _place;
  bool _moved = false;

  /// What was read last.
  StageRead get stage => _stage;

  /// What differed when [stage] was read, from the read before it: the same that the
  /// listeners were told.
  StageChange get change => _change;

  /// Why the stage could not be read again after a change, if it could not: [stage]
  /// is then older than the document. [refresh] tries again. Null after a read that
  /// worked.
  Object? get readError => _error;

  /// Calls `listener` after every read that found something different, with what
  /// differs, and when [readError] changes (with [StageChange.none] if nothing else
  /// did). It is called apart from the call that made the change, so it may ask the
  /// keeper at once.
  void addListener(void Function(StageChange change) listener) =>
      _listeners.add(listener);

  /// Ends what [addListener] began.
  void removeListener(void Function(StageChange change) listener) =>
      _listeners.remove(listener);

  /// The keeper stops reading this stage. The last read stays in [stage].
  void close() => _keeper._readers.remove(this);

  void _tell(StageChange change) {
    _change = change;
    InTurn.apart(() {
      for (final listener in [..._listeners]) {
        listener(change);
      }
    });
  }

  void _failed(Object error) {
    _error = error;
    _tell(StageChange.none);
  }

  /// Reads `cell` as the stage. A cell that is gone, or has lost its grid, gives way
  /// to the level the view state names, which is the nearest above it that is left.
  Future<StageRead> _look(Session doc, String cell) async {
    StageRead? read;
    try {
      read = await _window(doc, cell);
      if (read.grid != null) return read;
    } on BridgeError catch (error) {
      if (error.kind != ErrorKind.notFound) rethrow;
    }
    final shown = await doc.viewStage();
    // Only the root is shown without a grid.
    if (read != null && shown == cell) return read;
    return _window(doc, shown);
  }

  /// One read as the screen asked. A level that remembers a place outside the
  /// window that was asked for is read with the window there.
  Future<StageRead> _window(Session doc, String cell) async {
    final read = StageRead(cell, _wanted, await doc.stage(cell, _wanted));
    final grid = read.grid;
    final level = read.level;
    if (grid == null || level == null) return read;
    final inside =
        level.x >= grid.x0 &&
        level.x < grid.x0 + grid.cols &&
        level.y >= grid.y0 &&
        level.y < grid.y0 + grid.rows;
    if (inside) return read;
    final ask = _windowAt(_wanted, x0: level.x, y0: level.y);
    return StageRead(cell, ask, await doc.stage(cell, ask));
  }

  /// At the level the view state names, with nothing to compare with.
  Future<void> _start(Session doc) async =>
      _show(await _look(doc, await doc.viewStage()));

  /// Another stage than before.
  void _show(StageRead read) {
    _stage = read;
    _error = null;
    _place = null;
    _tell(StageChange.all);
  }

  /// The same stage as before, read again or otherwise.
  void _update(StageRead read) {
    final failed = _error != null;
    final change = read.since(_stage);
    _stage = read;
    _error = null;
    if (failed || !change.isEmpty) _tell(change);
  }

  /// After a change to the document: the same stage read again, with the window
  /// where it was.
  Future<void> _again(Session doc) async {
    final before = _stage;
    StageRead? read;
    try {
      final info = await doc.stage(before.cell, before.ask);
      if (info.grids.isNotEmpty || before.grid == null) {
        read = StageRead(before.cell, before.ask, info);
      }
    } on BridgeError catch (error) {
      if (error.kind != ErrorKind.notFound) rethrow;
    }
    // The stage is gone, or its grid is: up to the nearest level that is left.
    if (read == null) return _show(await _look(doc, before.cell));
    _update(read);
  }

  /// Hands the place on this level to the view state, if the screen moved since.
  /// That the level cannot remember anything (the root of a document without a
  /// grid), or is gone, is no error; a selected cell that is gone is left out.
  Future<void> _remember(Session doc) async {
    if (!_moved) return;
    _moved = false;
    for (final level in _levels()) {
      try {
        return await doc.viewRemember(level);
      } on BridgeError catch (error) {
        if (error.kind == ErrorKind.refused) return;
        if (error.kind != ErrorKind.notFound) rethrow;
      }
    }
  }

  /// What this level is to remember: as the screen said, and without the selected
  /// cell for when that is gone. Nothing if the screen never said.
  List<LevelInfo> _levels() {
    final place = _place;
    if (place == null || _stage.grid == null) return const [];
    return [
      if (place.selected != null)
        LevelInfo(
          cell: _stage.cell,
          x: place.x,
          y: place.y,
          selected: place.selected,
        ),
      LevelInfo(cell: _stage.cell, x: place.x, y: place.y),
    ];
  }

  /// Tells where the screen is on this level: the column `x` and the row `y` of the
  /// stage's grid at the top left of what is seen, counted from 0, and the cell that
  /// is selected, if one is. It costs nothing and calls nothing: say it as often as
  /// the screen scrolls. The keeper hands it to the view state when the level is
  /// left ([go]), with every save, and when the app is put away or closed; a later
  /// read of this level gives it back in `stage.level`.
  void at({required int x, required int y, String? selected}) {
    final place = (x: x, y: y, selected: selected);
    if (place == _place) return;
    _place = place;
    _moved = true;
  }

  /// Makes `cell` the stage: a dive into a cell of the stage, a step up, a jump along
  /// the trail. The level that is left remembers where the screen said it was ([at]),
  /// the view state is told which level is shown, and the stage is read. `stage.level`
  /// then says where the screen was when it last left the new level.
  ///
  /// A cell without a grid, other than the root, is refused ([ErrorKind.refused]),
  /// and one that does not exist is [ErrorKind.notFound]: the stage stays as it was,
  /// and so does the view state. So it does when the cell cannot be read.
  Future<void> go(String cell) => _keeper._use((doc) async {
    // Read first: what cannot be shown is not gone to.
    final read = await _window(doc, cell);
    for (final left in [..._levels(), null]) {
      try {
        await doc.viewGo(cell, left: left);
        break;
      } on BridgeError catch (error) {
        // The selected cell is gone: the level is left without it.
        final again = left?.selected != null;
        if (error.kind != ErrorKind.notFound || !again) rethrow;
      }
    }
    _show(read);
  });

  /// Moves the window of the stage's grid that is read, when the request has one
  /// (fewer `cols` or `rows` than the grid has): it then starts at column `x0` and
  /// row `y0`. The cells that come into the window are named as cells that look
  /// different, those that leave it as gone.
  Future<void> window({int? x0, int? y0}) =>
      _keeper._use((doc) => _moveTo(doc, x0: x0, y0: y0));

  /// Makes sure that the rows from `first` to `last` of the stage's grid, counted
  /// from 0, are among those that were read: what a screen calls with the rows it
  /// shows, as often as it scrolls. Where the stage is given whole, or has those
  /// rows already, it does nothing and asks no one; otherwise the window is moved
  /// so that the rows are in its middle, and the listeners are told what came and
  /// went.
  Future<void> see(int first, int last) {
    if (_sees(first, last) == null) return Future.value();
    return _keeper._use((doc) async {
      // As the stage is now: what was asked before may have moved it.
      final y0 = _sees(first, last);
      if (y0 != null) await _moveTo(doc, y0: y0);
    });
  }

  /// Where the window has to start for the rows `first` to `last` to be in its
  /// middle, or null if they are among the rows that were read.
  int? _sees(int first, int last) {
    final grid = _stage.grid;
    if (grid == null) return null;
    final top = first.clamp(0, grid.ys - 1);
    final bottom = last.clamp(top, grid.ys - 1);
    if (top >= grid.y0 && bottom < grid.y0 + grid.rows) return null;
    final rows = _stage.ask.rows;
    final most = grid.ys > rows ? grid.ys - rows : 0;
    // With more rows to see than a window holds, from the first of them.
    final spare = rows - (bottom - top + 1);
    return (top - (spare > 0 ? spare ~/ 2 : 0)).clamp(0, most);
  }

  Future<void> _moveTo(Session doc, {int? x0, int? y0}) async {
    final ask = _windowAt(_stage.ask, x0: x0, y0: y0);
    _update(StageRead(_stage.cell, ask, await doc.stage(_stage.cell, ask)));
  }

  /// Asks something else of the stage from here on (deeper, larger previews, another
  /// size of window), and reads it so.
  Future<void> ask(StageAsk ask) => _keeper._use((doc) async {
    _wanted = ask;
    _update(await _window(doc, _stage.cell));
  });

  /// Reads the stage again now: after [readError].
  Future<void> refresh() => _keeper._use(_again);
}

/// `ask` with its window starting elsewhere.
StageAsk _windowAt(StageAsk ask, {int? x0, int? y0}) => StageAsk(
  x0: x0 ?? ask.x0,
  y0: y0 ?? ask.y0,
  cols: ask.cols,
  rows: ask.rows,
  fullDepth: ask.fullDepth,
  fullCells: ask.fullCells,
  previewCols: ask.previewCols,
  previewRows: ask.previewRows,
  shapeCols: ask.shapeCols,
  shapeRows: ask.shapeRows,
  textCut: ask.textCut,
  previewCut: ask.previewCut,
);

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
      // What could not be closed is let go of; what has ended refuses both.
    }
  }
}
