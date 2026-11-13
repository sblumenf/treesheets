import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../cell_editor.dart';
import '../keeper.dart';
import '../session.dart';
import '../src/rust/api/wire.dart';
import '../stage_read.dart';
import 'budget.dart';
import 'cell_view.dart';
import 'editor_bar.dart';
import 'log.dart';
import 'place.dart';
import 'settings_sheet.dart';
import 'stage_view.dart';
import 'variants.dart';
import 'words.dart';

/// The phone screen of `docs/UI_SPEC.md`, its first slice (sections 3 to 8): the
/// stage's grid with what is nested in it drawn by the depth budget, the title and
/// the trail, dive and up, a tap to edit, the editor docked above the keyboard with
/// its toolbar, and rows and columns added; with the prototype's variant switches
/// and its log.
///
/// It keeps nothing of the document. What it draws is the keeper's stage
/// ([StageReader]), read again by the keeper after every change; a cell's text is
/// edited through the keeper's [Editing], and the cell that is edited is drawn from
/// the editor's text. Everything the user does is one piece of one queue, in the
/// order it was done, and finds the screen as the pieces before it left it.
///
/// The screen is of one keeper for as long as it lives: give it a key that changes
/// with the keeper. The keeper is its owner's to close.
///
/// Keys, besides those of [StageView], [CellView], [EditorBar] and
/// [SettingsSheet]: `title`, `stripe`, `state`, `undo`, `redo`, `menu`,
/// `menu-settings`; `up`, `trail-<level>`; `bar-add` (with `add-row-below`,
/// `add-column-right`), `bar-edit`, `bar-open`; `message`, `trouble` (with
/// `keep-mine`, `open-theirs`, `save-again`, `read-again`), `view-error`; `ended`,
/// `stage-error`, `reading`; `empty-root` and `first-cell`; `link-target`,
/// `link-ok`, `link-cancel`.
class PhoneScreen extends StatefulWidget {
  const PhoneScreen({
    super.key,
    required this.keeper,
    required this.variants,
    required this.log,
    this.editor = FlutterCellEditor.new,
    this.exportFolder,
    this.menu = const [],
  });

  /// What keeps the document, and reads and changes it for the screen.
  final Keeper keeper;

  /// The prototype's switches, which the screen's settings change.
  final ValueNotifier<Variants> variants;

  /// The prototype log, to which the screen adds what is done.
  final ProtoLog log;

  /// Makes the editor: the switch between kinds of editor.
  final CellEditorFactory editor;

  /// Where an exported log goes ([SettingsSheet.exportFolder]).
  final Future<Directory> Function()? exportFolder;

  /// More entries for the menu of the title bar.
  final List<PopupMenuEntry<VoidCallback>> menu;

  @override
  State<PhoneScreen> createState() => _PhoneScreenState();
}

/// How putting the editor's text into the document ended.
enum _Entered {
  /// The document has it, and the editor holds nothing else.
  yes,

  /// The core refused it; the message line says why.
  refused,

  /// The document has what was sent, and more was typed while the core worked.
  typedOn,
}

class _PhoneScreenState extends State<PhoneScreen> {
  late final CellEditor _editor = widget.editor();

  /// Everything the user does, one after the other (decision A158). An error that
  /// an action did not expect is shown like the others.
  late final _actions = InTurn(
    onUnheard: (error, stack) {
      // With no screen left to show it on, it is an error that nobody caught.
      if (!mounted) Error.throwWithStackTrace(error, stack);
      _say(describeError(error));
    },
  );
  late final Attachment _attachment;
  late int _opens = widget.keeper.opens;

  /// The stage as the keeper keeps it for this screen; null until first read.
  StageReader? _reader;

  /// Why there is no reader, if the first read failed.
  Object? _startError;

  /// The cell whose text is in the editor: the editor is shown while this is set.
  String? _editCell;

  /// The editing of that cell, as the keeper has it. It is over when its text was
  /// entered while more was typed, or its document was let go of; whatever needs
  /// one then opens another.
  Editing? _editing;

  /// The edited cell as the document holds it: when the editor opened on it, and
  /// after its text was held there or a style put on it. Of use only while the
  /// editor is open.
  CellLook? _held;

  /// Whether the row of styles is shown in the toolbar.
  bool _styles = false;
  String? _selected;
  final _unfolded = <String>{};

  /// The column and row at the top left of what is seen on this level, as last
  /// known, and the way to ask the grid for it.
  ({int x, int y}) _at = (x: 0, y: 0);
  final _place = StagePlace();

  /// Counts the levels arrived at: the grid is made anew for each.
  int _arrival = 0;

  /// Whether the last arrival was a way up, which is drawn shrinking.
  bool _cameUp = false;

  /// The cell that flashes once: the one the screen came up out of.
  String? _flash;

  /// What the last action had to say: an error, or that there was nothing to do.
  String? _message;
  final _editedKey = GlobalKey();

  Keeper get _keeper => widget.keeper;
  Variants get _variants => widget.variants.value;

  @override
  void initState() {
    super.initState();
    // Return: save, and on to the cell below.
    _editor.onSubmit = () => _move(down: true);
    _attachment = _keeper.attach(
      onChange: _kept,
      queue: _actions,
      finish: _finish,
    );
    widget.variants.addListener(_redraw);
    _actions.post(_start);
  }

  @override
  void didUpdateWidget(PhoneScreen old) {
    super.didUpdateWidget(old);
    assert(
      identical(old.keeper, widget.keeper),
      'A PhoneScreen is of one keeper: key it by the keeper.',
    );
    if (!identical(old.variants, widget.variants)) {
      old.variants.removeListener(_redraw);
      widget.variants.addListener(_redraw);
    }
  }

  /// The screen goes. What is in the editor is entered, as by a tap elsewhere:
  /// one call, on the editing that is open already, so that it is asked of the
  /// keeper before anyone can ask it to close. There is no one left to show a
  /// refusal to.
  @override
  void dispose() {
    widget.variants.removeListener(_redraw);
    _attachment.detach();
    final editing = _editing;
    if (_editCell != null && editing != null) {
      // (An editing that ended with its document refuses, which is nothing new.)
      editing.enter(_editor.text).then<void>((_) {}, onError: (Object _) {});
    }
    _reader?.removeListener(_changed);
    _reader?.close();
    _editor.dispose();
    super.dispose();
  }

  void _redraw() {
    if (mounted) setState(() {});
  }

  /// The first read, at the level this device showed last.
  Future<void> _start() async {
    try {
      final reader = await _keeper.read();
      if (!mounted) return reader.close();
      reader.addListener(_changed);
      setState(() {
        _reader = reader;
        _arrived(reader.stage);
      });
    } catch (error) {
      if (mounted) setState(() => _startError = error);
    }
  }

  /// Another level is shown: the screen is where that level was left.
  void _arrived(StageRead stage) {
    final level = stage.level;
    _arrival++;
    _at = (x: level?.x ?? 0, y: level?.y ?? 0);
    _selected = level?.selected;
    _unfolded.clear();
    _flash = null;
  }

  /// The stage was read again and something differs. (Called apart from the call
  /// that made the change.)
  void _changed(StageChange change) {
    if (!mounted) return;
    setState(() {
      final stage = _reader!.stage;
      if (change.everything) {
        _cameUp = false;
        _arrived(stage);
      } else if (_selected != null && stage.look(_selected!) == null) {
        _selected = null;
      }
    });
  }

  /// The keeper has something new to say. If it has opened the file again, the
  /// editing ended with the document that is gone: the editor closes without
  /// entering. (The reader starts again by itself.)
  void _kept() {
    if (!mounted) return;
    setState(() {
      if (_keeper.opens != _opens) {
        _opens = _keeper.opens;
        _editCell = null;
        _editing = null;
      }
    });
  }

  void _say(String? message) {
    if (mounted && message != null) setState(() => _message = message);
  }

  void _log(String what, [Map<String, Object?> where = const {}]) {
    final stage = _reader?.stage;
    widget.log.add(what, _variants, {
      if (stage != null) 'depth': stage.trail.length - 1,
      ...where,
    });
  }

  /// Where a cell is, for the log: no text and no ID.
  Map<String, Object?> _where(String id) {
    final stage = _reader?.stage;
    final place = stage == null ? null : placeOf(stage, id);
    return place == null
        ? const {}
        : {'x': place.x, 'y': place.y, 'nested': place.part.level};
  }

  /// Takes one action of the user into the queue. What was on the message line
  /// was said about an action before this one, and goes.
  void _do(Future<void> Function() action) {
    if (_message != null) setState(() => _message = null);
    _actions.post(() async {
      // A screen that has gone does nothing. (Without a stage nothing is on the
      // screen that asks for an action.)
      if (mounted) await action();
    });
  }

  /// The editing to put the editor's text into the document with: the one that is
  /// open, or a new one of the same cell.
  Future<Editing> _open(String cell) async {
    final editing = _editing;
    if (editing != null && editing.open) return editing;
    return _editing = await _keeper.edit(cell);
  }

  /// Sends what is in the editor to the core, which ends the editing. The editor
  /// stays as it is: if the core refuses, with what was typed, and if more was
  /// typed while the core worked, with that.
  Future<_Entered> _enter() async {
    final cell = _editCell;
    if (cell == null) return _Entered.yes;
    _editor.finishComposing();
    final text = _editor.text;
    try {
      await (await _open(cell)).enter(text);
    } catch (error) {
      _say(describeError(error));
      return _Entered.refused;
    }
    _editing = null;
    _log('enter', _where(cell));
    if (_editor.text == text) return _Entered.yes;
    // The editor stays, with what was typed meanwhile: it has an editing again at
    // once, so that a screen that goes can enter that text with one call.
    try {
      final again = await _keeper.edit(cell);
      _editing = again;
      _held = again.start;
    } catch (error) {
      _say(describeError(error));
    }
    return _Entered.typedOn;
  }

  /// What the keeper asks when the app is put away or the document closed: the
  /// editor's text goes into the document, held there by the editing, which goes
  /// on, so that the cross can still take it back. The keeper runs this in the
  /// turn of the screen's actions; it asks that queue for nothing.
  Future<void> _finish() async {
    final cell = _editCell;
    if (cell == null) return;
    _editor.finishComposing();
    try {
      final held = await (await _open(cell)).hold(_editor.text);
      if (mounted) setState(() => _held = held);
    } catch (error) {
      // Shown, not thrown: nothing is unfinished then.
      _say(describeError(error));
    }
  }

  /// Opens the editor on a cell, or moves it there: on the cell as it is when the
  /// turn of this comes, and only if the stage still has it. Says whether it did.
  Future<bool> _edit(String id) async {
    if (_reader!.stage.look(id) == null) return false;
    final Editing editing;
    try {
      editing = await _keeper.edit(id);
    } catch (error) {
      _say(describeError(error));
      return false;
    }
    if (!mounted) return false;
    setState(() {
      _editCell = id;
      _editing = editing;
      // The cell as the document has it: its styles are drawn for as long as
      // the editor has that very text.
      _held = editing.start;
      _selected = id;
      // The cursor goes to the end of the text.
      _editor.set(editing.start.text);
    });
    _editor.focus();
    _note();
    _log('edit', _where(id));
    return true;
  }

  void _close() {
    if (!mounted) return;
    setState(() {
      _editCell = null;
      _editing = null;
      _styles = false;
    });
  }

  /// With the editor open, a tap outside it only closes the editor, keeping what
  /// was typed (UI_SPEC section 6). Says whether the editor was open.
  Future<bool> _onlyCloses() async {
    if (_editCell == null) return false;
    if (await _enter() == _Entered.yes) _close();
    return true;
  }

  /// Tells the reader where the screen is, which costs nothing.
  void _note() {
    _at = _place.read() ?? _at;
    _reader?.at(x: _at.x, y: _at.y, selected: _selected);
  }

  /// A tap on a cell's text.
  void _tapText(String id) => _do(() async {
    if (await _onlyCloses()) return;
    if (_variants.tapTextSelects && _selected != id) {
      setState(() => _selected = id);
      _note();
      return _log('select', _where(id));
    }
    await _edit(id);
  });

  /// A tap on a cell's grid where it is one target: a dive, or with the other
  /// variant the grid unfolded in place. A cell that the stage no longer has when
  /// the turn of its tap comes (the second of two fast taps) does nothing.
  void _tapChild(String id) => _do(() async {
    if (await _onlyCloses()) return;
    final stage = _reader!.stage;
    final look = stage.look(id);
    if (look == null || !look.hasGrid) return;
    // Only a grid of a cell of the stage itself unfolds: the read gives one level
    // cell by cell.
    if (_variants.tapChildUnfolds && placeOf(stage, id)?.part.level == 0) {
      setState(() => _unfolded.add(id));
      _log('unfold', _where(id));
      return _askFor();
    }
    await _go(id, 'dive');
  });

  void _fold(String id) => _do(() async {
    if (await _onlyCloses()) return;
    setState(() => _unfolded.remove(id));
    _log('fold', _where(id));
    await _askFor();
  });

  /// Asks the reader for what the unfolded grids need: each of them cell by cell,
  /// however many cells it has.
  Future<void> _askFor() async {
    final reader = _reader!;
    final stage = reader.stage;
    var cells = phoneAsk.fullCells;
    for (final id in _unfolded) {
      final look = stage.look(id);
      final summary = look == null ? null : stage.summary(look);
      if (summary != null && summary.xs * summary.ys > cells) {
        cells = summary.xs * summary.ys;
      }
    }
    if (cells == stage.ask.fullCells) return;
    try {
      await reader.ask(askFull(cells));
    } catch (error) {
      _say(describeError(error));
    }
  }

  /// Makes `cell` the stage. Says whether it did.
  Future<bool> _go(String cell, String what) async {
    final reader = _reader!;
    final before = reader.stage;
    try {
      if (before.ask.fullCells != phoneAsk.fullCells) {
        _unfolded.clear();
        await reader.ask(phoneAsk);
      }
      await reader.go(cell);
    } catch (error) {
      _say(describeError(error));
      return false;
    }
    if (!mounted) return true;
    final depth = reader.stage.trail.length;
    final up = depth < before.trail.length;
    setState(() {
      _cameUp = up;
      // The cell of this level that the screen came out of.
      _flash = up ? before.trail[depth].id : null;
    });
    _log(what, {'from': before.trail.length - 1});
    return true;
  }

  /// An action of the bars. What is being typed is kept first; if that cannot be
  /// done, the action is not, the editor stays, and the message line says why.
  void _act(Future<void> Function() action) => _do(() async {
    final entered = await _enter();
    if (entered == _Entered.typedOn) _say(Words.typedOn);
    // Nor is it done on a screen that went while the text was entered.
    if (entered != _Entered.yes || !mounted) return;
    _close();
    await action();
  });

  Future<void> _upOne() async {
    final trail = _reader!.stage.trail;
    if (trail.length > 1) await _go(trail[trail.length - 2].id, 'up');
  }

  void _jump(int level) => _act(() async {
    final trail = _reader!.stage.trail;
    if (level >= 0 && level < trail.length - 1) {
      await _go(trail[level].id, 'jump');
    }
  });

  /// The system's back (UI_SPEC section 10): with the editor open it closes the
  /// editor and nothing else, as a tap outside does; otherwise up one level, and
  /// out of the app only from the root.
  void _back() => _do(() async {
    if (await _onlyCloses()) return;
    if (_reader!.stage.trail.length > 1) {
      _log('back');
      return _upOne();
    }
    await SystemNavigator.pop();
  });

  void _undo({required bool redo}) => _act(() async {
    try {
      final report = await (redo ? _keeper.redo() : _keeper.undo());
      if (report == null) {
        return _say(redo ? Words.nothingToRedo : Words.nothingToUndo);
      }
      _log(redo ? 'redo' : 'undo');
      await _askFor();
    } catch (error) {
      _say(describeError(error));
    }
  });

  /// Carries out a command, and says what the core refuses. Says whether it did.
  Future<bool> _apply(Command command) async {
    try {
      await _keeper.apply(command);
      return true;
    } catch (error) {
      _say(describeError(error));
      return false;
    }
  }

  /// Return and →: saves, and moves the editor to the cell below or on the right;
  /// at the edge a row or a column is added first, which is a step of the undo
  /// history after the text's (UI_SPEC section 7). `insert` adds one in any case
  /// (+Row, +Col).
  void _move({required bool down, bool insert = false}) => _do(() async {
    final cell = _editCell;
    if (cell == null) return;
    final entered = await _enter();
    // What was typed meanwhile stays in the editor, on the cell it was typed in.
    if (entered != _Entered.yes || !mounted) return;
    if (await _next(cell, down: down, insert: insert)) return;
    _close();
  });

  /// Moves the editor from `cell` one cell down or to the right, adding a row or
  /// column if there is none or `insert` says so. Says whether the editor is on
  /// the next cell.
  Future<bool> _next(
    String cell, {
    required bool down,
    required bool insert,
  }) async {
    var place = placeOf(_reader!.stage, cell);
    if (place == null) return false;
    final part = place.part;
    final x = place.x + (down ? 0 : 1);
    final y = place.y + (down ? 1 : 0);
    if (insert || x >= part.xs || y >= part.ys) {
      final added = await _apply(
        down
            ? Command.insertRows(cell: part.owner, at: y, count: 1)
            : Command.insertColumns(cell: part.owner, at: x, count: 1),
      );
      if (!added || !mounted) return false;
      _log(down ? 'add-row' : 'add-column', {'nested': part.level});
    }
    // A grid that is unfolded has grown: it is asked for cell by cell again.
    await _askFor();
    if (!mounted) return false;
    // As the stage is now: a grid that is not unfolded may have too many cells
    // now to be given cell by cell.
    final stage = _reader!.stage;
    final now = partOf(stage, part.owner);
    final next = now == null ? null : stage.at(now, x, y);
    return next != null && await _edit(next.id);
  }

  /// The bars after the last row and the last column, and "+ Add" without a cell
  /// selected: adds one to the stage's grid and opens the editor on its first
  /// cell (UI_SPEC section 8).
  void _append({required bool row}) => _act(() async {
    final stage = _reader!.stage;
    final grid = stage.grid;
    if (grid == null) return;
    final added = await _apply(
      row
          ? Command.insertRows(cell: stage.cell, at: grid.ys, count: 1)
          : Command.insertColumns(cell: stage.cell, at: grid.xs, count: 1),
    );
    if (!added || !mounted) return;
    _log(row ? 'add-row' : 'add-column', {'nested': 0});
    final now = _reader!.stage;
    final first = now.grid == null
        ? null
        : now.at(now.grid!, row ? 0 : grid.xs, row ? grid.ys : 0);
    if (first != null) await _edit(first.id);
  });

  /// "+ Add" of the bottom bar: a row below or a column to the right of the
  /// selected cell, and the editor on the new cell beside it.
  void _addBeside({required bool row}) {
    final selected = _selected;
    if (selected == null) return _append(row: row);
    _act(() async {
      if (!await _next(selected, down: row, insert: true)) _close();
    });
  }

  /// A document whose root has no grid: its first cell is made.
  void _firstCell() => _act(() async {
    final stage = _reader!.stage;
    if (stage.grid != null) return;
    if (!await _apply(Command.createGrid(cell: stage.cell, xs: 1, ys: 1))) {
      return;
    }
    if (!mounted) return;
    _log('first-cell');
    final now = _reader!.stage;
    final first = now.grid == null ? null : now.at(now.grid!, 0, 0);
    if (first != null) await _edit(first.id);
  });

  /// "Open ›": into the grid of a cell, the edited one or the selected one.
  void _openGrid(String? id) => _act(() async {
    final look = id == null ? null : _reader!.stage.look(id);
    if (look != null && look.hasGrid) await _go(look.id, 'dive');
  });

  /// The cross: "not this". The cell is as it was before the editor opened, also
  /// when the text went into the document meanwhile because the app was put away,
  /// and so is everything that could be undone or redone.
  void _cancel() => _do(() async {
    final cell = _editCell;
    final editing = _editing;
    if (editing != null && editing.open) {
      try {
        await editing.drop();
      } catch (error) {
        return _say(describeError(error));
      }
    }
    if (cell != null) _log('cancel', _where(cell));
    _close();
  });

  /// The ↵ key: a line break where the cursor is.
  void _break() => _do(() async {
    if (_editCell != null) _editor.insert('\n');
  });

  /// A key of the row of styles: the editor's text is held in the document and
  /// `command` carried out on it, as part of the same editing. `command` is given
  /// the cell as the document holds it and the range of its text that is
  /// selected, in code points; null says there is nothing to do.
  void _style(
    String what,
    Command? Function(CellLook held, int start, int end) command, {
    ({int start, int end})? selected,
  }) => _do(() async {
    final cell = _editCell;
    if (cell == null) return;
    _editor.finishComposing();
    final text = _editor.text;
    final selection = selected ?? _editor.selection;
    Command? made;
    final CellLook look;
    try {
      final editing = await _open(cell);
      // Held first, so that the command is built from the document's own text.
      final held = await editing.hold(text);
      final range = rangeIn(held, selection);
      made = command(held, range.start, range.end);
      look = made == null ? held : await editing.style(text, (_) => made!);
    } catch (error) {
      return _say(describeError(error));
    }
    if (!mounted) return;
    setState(() => _held = look);
    if (made != null) _log('style', {'style': what});
    // The core may keep another text than was sent; nothing typed meanwhile is
    // put aside for that.
    if (_editor.text == text && look.text != text) _editor.set(look.text);
  });

  void _bit(String what, Style on, bool Function(Style) bit) =>
      _style(what, (held, start, end) {
        // Without a selection, the whole cell.
        if (start == end) {
          final has = allStyled(held, 0, held.text.runes.length, bit);
          return Command.setStyle(
            cells: [held.id],
            set_: has ? plain : on,
            clear: has ? on : plain,
          );
        }
        final has = allStyled(held, start, end, bit);
        return Command.setRangeStyle(
          cell: held.id,
          start: start,
          len: end - start,
          set_: has ? plain : on,
          clear: has ? on : plain,
        );
      });

  /// TreeSheets' tag (UI_SPEC section 7, "Decided since"): the whole text of the
  /// cell becomes a tag of the document, or stops being one.
  void _tag() => _style(
    'tag',
    (held, _, _) => held.isTag
        ? Command.untagCells(cells: [held.id])
        : Command.tagCells(cells: [held.id]),
  );

  /// Asks where the link goes, and puts it on the selected text, or on all of it
  /// without a selection. The question is asked outside the queue of actions: the
  /// user may take any time over it, and the app may be put away meanwhile.
  Future<void> _link() async {
    if (_editCell == null) return;
    final selection = _editor.selection;
    final target = await showDialog<String>(
      context: context,
      builder: (context) => const _LinkDialog(),
    );
    if (!mounted || target == null) return;
    _editor.focus();
    _style('link', selected: selection, (held, start, end) {
      final whole = start == end;
      final from = whole ? 0 : start;
      final to = whole ? held.text.runes.length : end;
      if (from == to) {
        _say(Words.linkNoText);
        return null;
      }
      return Command.setLink(
        cell: held.id,
        start: from,
        len: to - from,
        target: target.isEmpty ? null : LinkTarget.external_(target: target),
      );
    });
  }

  /// The other program's file is chosen over this document, and what is being
  /// typed goes with the rest. If that file cannot be read, nothing is given up.
  void _openTheirs() => _do(() async {
    try {
      await _keeper.openTheirs();
    } catch (error) {
      // Only the core's answer to reading the file says something about the file.
      _say(
        Words.theirsNotOpened(
          theirs:
              isFatal(error) ||
              error is BridgeError && error.kind != ErrorKind.refused,
          fault: isFatal(error),
          why: describeError(error),
        ),
      );
    }
  });

  void _settings() {
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      builder: (context) => SettingsSheet(
        variants: widget.variants,
        log: widget.log,
        exportFolder: widget.exportFolder,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final ended = _keeper.ended;
    final reader = _reader;
    final startError = _startError;
    final Widget body;
    if (ended != null) {
      body = _alone(Text(ended, key: const Key('ended')));
    } else if (reader != null) {
      body = _stage(context, reader);
    } else if (startError != null) {
      body = _alone(
        Text(
          Words.notRead(describeError(startError)),
          key: const Key('stage-error'),
        ),
      );
    } else {
      body = _alone(const CircularProgressIndicator(key: Key('reading')));
    }
    return PopScope(
      // Never by itself: the back action decides when its turn comes. After the
      // end, and while there is no stage to show, nothing is left to go up in.
      canPop: ended != null || reader == null,
      onPopInvokedWithResult: (popped, _) {
        if (!popped) _back();
      },
      child: Scaffold(
        body: SafeArea(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _titleBar(context, ended == null ? reader?.stage : null),
              Expanded(child: body),
            ],
          ),
        ),
      ),
    );
  }

  Widget _alone(Widget child) => Center(
    child: Padding(padding: const EdgeInsets.all(24), child: child),
  );

  /// The title bar: the colour and text of the level's cell, undo, redo and the
  /// menu; under it, in a word, how the document stands with its file.
  Widget _titleBar(BuildContext context, StageRead? stage) {
    final theme = Theme.of(context);
    final step = stage?.trail.last;
    final text = step?.shown.text ?? '';
    return Material(
      elevation: 2,
      color: theme.colorScheme.surfaceContainer,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          Row(
            children: [
              Container(
                key: const Key('stripe'),
                width: 8,
                height: target,
                color: step == null ? Colors.transparent : colorOf(step.color),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  text.isEmpty
                      ? Words.app
                      : '$text${step!.shown.cut ? '…' : ''}',
                  key: const Key('title'),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.titleMedium,
                  textDirection: directionOf(text),
                ),
              ),
              IconButton(
                key: const Key('undo'),
                tooltip: Words.undo,
                icon: const Icon(Icons.undo),
                onPressed: stage == null ? null : () => _undo(redo: false),
              ),
              IconButton(
                key: const Key('redo'),
                tooltip: Words.redo,
                icon: const Icon(Icons.redo),
                onPressed: stage == null ? null : () => _undo(redo: true),
              ),
              // No Save: the document saves itself.
              PopupMenuButton<VoidCallback>(
                key: const Key('menu'),
                tooltip: Words.menu,
                onSelected: (action) => action(),
                itemBuilder: (context) => [
                  PopupMenuItem(
                    key: const Key('menu-settings'),
                    value: _settings,
                    child: const Text(Words.settings),
                  ),
                  ...widget.menu,
                ],
              ),
            ],
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 2),
            child: Text(
              Words.state(_keeper.state),
              key: const Key('state'),
              textAlign: TextAlign.end,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.labelSmall,
            ),
          ),
        ],
      ),
    );
  }

  Widget _stage(BuildContext context, StageReader reader) {
    final stage = reader.stage;
    final viewError = _keeper.viewError;
    final message = _message;
    final editing = _editCell;
    final edited = editing == null ? null : stage.look(editing);
    final scene = Scene(
      stage: stage,
      variants: _variants,
      textScale: MediaQuery.textScalerOf(context).scale(cellFont) / cellFont,
      fullCells: phoneAsk.fullCells,
      selected: _selected,
      editing: editing,
      editor: _editor,
      held: _held,
      unfolded: _unfolded,
      flash: _flash,
      editedKey: _editedKey,
      onText: _tapText,
      onChild: _tapChild,
      onFold: _fold,
    );
    final bars = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        ?_trouble(context, reader),
        if (viewError != null)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 2),
            child: Text(
              Words.viewNotKept(describeError(viewError)),
              key: const Key('view-error'),
              style: Theme.of(context).textTheme.labelSmall,
            ),
          ),
        if (message != null) _messageLine(context, message),
        const Divider(height: 1),
        _trail(context, stage),
        if (editing != null)
          EditorBar(
            editor: _editor,
            stylesShown: _styles,
            onStyles: () => setState(() => _styles = !_styles),
            onBold: () => _bit('bold', bold, (style) => style.bold),
            onItalic: () => _bit('italic', italic, (style) => style.italic),
            onLink: _link,
            onTag: _tag,
            onBreak: _break,
            onRight: () => _move(down: false),
            onRow: () => _move(down: true, insert: true),
            onColumn: () => _move(down: false, insert: true),
            onOpen: edited != null && edited.hasGrid
                ? () => _openGrid(editing)
                : null,
            onTick: () => _do(() async {
              if (await _enter() == _Entered.yes) _close();
            }),
            onCross: _cancel,
          )
        else
          _bottomBar(context, stage),
      ],
    );
    return LayoutBuilder(
      builder: (context, box) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(
            child: stage.grid == null
                ? _alone(
                    Column(
                      key: const Key('empty-root'),
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Text(Words.emptyDocument),
                        const SizedBox(height: 16),
                        FilledButton(
                          key: const Key('first-cell'),
                          onPressed: _firstCell,
                          child: const Text(Words.firstCell),
                        ),
                      ],
                    ),
                  )
                : _Arrive(
                    key: ValueKey(_arrival),
                    up: _cameUp,
                    child: StageView(
                      scene: scene,
                      place: _place,
                      onPlace: (_, _) => _note(),
                      onAddRow: () => _append(row: true),
                      onAddColumn: () => _append(row: false),
                    ),
                  ),
          ),
          // On a small screen with large text the lines and bars can be taller
          // than the screen: they scroll then, the editor staying in sight, and
          // the grid keeps a quarter of the height.
          ConstrainedBox(
            constraints: BoxConstraints(maxHeight: box.maxHeight * 0.75),
            child: SingleChildScrollView(reverse: true, child: bars),
          ),
        ],
      ),
    );
  }

  /// What stays wrong until something is done about it, with the ways to do that:
  /// the document is not in its file, or the stage could not be read again.
  /// Unlike the message line it does not go away with the next action.
  Widget? _trouble(BuildContext context, StageReader reader) {
    final saveError = _keeper.saveError;
    final readError = reader.readError;
    final String text;
    final List<(String, String, VoidCallback)> ways;
    if (saveError is BridgeError && saveError.kind == ErrorKind.changedOnDisk) {
      text = Words.changedOnDisk;
      ways = [
        ('keep-mine', Words.keepMine, () => _do(_keeper.keepMine)),
        ('open-theirs', Words.openTheirs, _openTheirs),
      ];
    } else if (saveError != null) {
      text = Words.notSaved(describeError(saveError));
      ways = [('save-again', Words.tryAgain, () => _do(_keeper.save))];
    } else if (readError != null) {
      text = Words.old(describeError(readError));
      ways = [
        (
          'read-again',
          Words.tryAgain,
          () => _do(() async {
            try {
              await reader.refresh();
            } catch (error) {
              _say(describeError(error));
            }
          }),
        ),
      ];
    } else {
      return null;
    }
    final colors = Theme.of(context).colorScheme;
    return Material(
      color: colors.errorContainer,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12),
        child: Wrap(
          alignment: WrapAlignment.end,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            Text(
              text,
              key: const Key('trouble'),
              style: TextStyle(color: colors.onErrorContainer),
            ),
            for (final (key, label, action) in ways)
              TextButton(
                key: Key(key),
                style: TextButton.styleFrom(
                  minimumSize: const Size(target, target),
                ),
                onPressed: action,
                child: Text(label),
              ),
          ],
        ),
      ),
    );
  }

  Widget _messageLine(BuildContext context, String message) {
    final colors = Theme.of(context).colorScheme;
    return Material(
      color: colors.inverseSurface,
      child: InkWell(
        onTap: () => setState(() => _message = null),
        child: Container(
          constraints: const BoxConstraints(minHeight: target),
          alignment: AlignmentDirectional.centerStart,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: Text(
            message,
            key: const Key('message'),
            style: TextStyle(color: colors.onInverseSurface),
          ),
        ),
      ),
    );
  }

  /// The height of a bar of buttons, which grows with the system's text size.
  double _barHeight(BuildContext context) =>
      target *
      (MediaQuery.textScalerOf(context).scale(cellFont) / cellFont).clamp(
        1.0,
        1.6,
      );

  /// The way from the root to here: back one level, or to any level by its name.
  Widget _trail(BuildContext context, StageRead stage) {
    final trail = stage.trail;
    return SizedBox(
      height: _barHeight(context),
      child: Row(
        children: [
          IconButton(
            key: const Key('up'),
            tooltip: Words.up,
            icon: const Icon(Icons.chevron_left),
            onPressed: trail.length < 2 ? null : () => _act(_upOne),
          ),
          Expanded(
            // The end of the way is in sight; the rest scrolls. All of it is
            // built: a level out of sight can still be asked for by its key.
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              reverse: true,
              child: Row(
                children: [
                  for (var i = 0; i < trail.length; i++)
                    TextButton(
                      key: Key('trail-$i'),
                      onPressed: i == trail.length - 1 ? null : () => _jump(i),
                      child: Text(
                        '${i == 0 ? '' : '› '}'
                        '${trail[i].shown.text.isEmpty ? Words.noText : trail[i].shown.text}'
                        '${trail[i].shown.cut ? '…' : ''}',
                      ),
                    ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// The bottom bar (UI_SPEC section 4), which acts on the selected cell.
  Widget _bottomBar(BuildContext context, StageRead stage) {
    final selected = _selected == null ? null : stage.look(_selected!);
    final least = TextButton.styleFrom(minimumSize: const Size(target, target));
    return Material(
      elevation: 8,
      child: SizedBox(
        height: _barHeight(context),
        child: ListView(
          scrollDirection: Axis.horizontal,
          children: [
            PopupMenuButton<VoidCallback>(
              key: const Key('bar-add'),
              tooltip: Words.add,
              enabled: stage.grid != null,
              onSelected: (action) => action(),
              itemBuilder: (context) => [
                PopupMenuItem(
                  key: const Key('add-row-below'),
                  value: () => _addBeside(row: true),
                  child: const Text(Words.addRowBelow),
                ),
                PopupMenuItem(
                  key: const Key('add-column-right'),
                  value: () => _addBeside(row: false),
                  child: const Text(Words.addColumnRight),
                ),
              ],
              child: Container(
                constraints: const BoxConstraints(minWidth: target),
                padding: const EdgeInsets.symmetric(horizontal: 12),
                alignment: Alignment.center,
                child: Text(
                  Words.add,
                  style: Theme.of(context).textTheme.labelLarge,
                ),
              ),
            ),
            TextButton(
              key: const Key('bar-edit'),
              style: least,
              onPressed: selected == null
                  ? null
                  : () => _do(() async {
                      if (!await _onlyCloses()) await _edit(selected.id);
                    }),
              child: const Text(Words.edit),
            ),
            TextButton(
              key: const Key('bar-open'),
              style: least,
              onPressed: selected != null && selected.hasGrid
                  ? () => _openGrid(selected.id)
                  : null,
              child: const Text(Words.open),
            ),
          ],
        ),
      ),
    );
  }
}

/// Draws a level as it is arrived at: growing into place after a dive, shrinking
/// into place after a way up (UI_SPEC section 5), in 300 ms, or at once where the
/// system asks for no animations.
class _Arrive extends StatelessWidget {
  const _Arrive({super.key, required this.up, required this.child});

  final bool up;
  final Widget child;

  @override
  Widget build(BuildContext context) => TweenAnimationBuilder<double>(
    tween: Tween(begin: 0, end: 1),
    duration: MediaQuery.disableAnimationsOf(context)
        ? Duration.zero
        : const Duration(milliseconds: 300),
    curve: Curves.easeOut,
    builder: (context, t, child) => Opacity(
      opacity: 0.3 + 0.7 * t,
      child: Transform.scale(
        scale: up ? 1.12 - 0.12 * t : 0.88 + 0.12 * t,
        child: child,
      ),
    ),
    child: child,
  );
}

/// Asks for the target of a link. Answers with what was typed, which is empty to
/// remove the link, or with null when it was put aside.
class _LinkDialog extends StatefulWidget {
  const _LinkDialog();

  @override
  State<_LinkDialog> createState() => _LinkDialogState();
}

class _LinkDialogState extends State<_LinkDialog> {
  final _target = TextEditingController();

  @override
  void dispose() {
    _target.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: const Text(Words.linkTitle),
    content: TextField(
      key: const Key('link-target'),
      controller: _target,
      autofocus: true,
      keyboardType: TextInputType.url,
      decoration: const InputDecoration(hintText: Words.linkHint),
      onSubmitted: (text) => Navigator.of(context).pop(text.trim()),
    ),
    actions: [
      TextButton(
        key: const Key('link-cancel'),
        onPressed: () => Navigator.of(context).pop(),
        child: const Text(Words.cancel),
      ),
      TextButton(
        key: const Key('link-ok'),
        onPressed: () => Navigator.of(context).pop(_target.text.trim()),
        child: const Text(Words.ok),
      ),
    ],
  );
}
