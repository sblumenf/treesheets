import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../cell_editor.dart';
import '../keeper.dart';
import '../session.dart';
import '../src/rust/api/wire.dart';
import 'stage.dart';

/// The smallest height of anything that can be tapped (UI_SPEC section 3).
const double _target = 48;

/// The simple grid screen: one grid, into a nested grid and back out, and the text of one
/// cell edited at a time, with undo and redo. A first step towards the screen that
/// `docs/UI_SPEC.md` specifies, with none of its previews, gestures or animations.
///
/// The document belongs to the stage's keeper, which saves it; the screen shows what
/// the keeper says and never saves by itself.
class GridScreen extends StatefulWidget {
  const GridScreen({super.key, required this.stage, this.menu = const []});

  final Stage stage;

  /// More entries for the menu of the title bar.
  final List<PopupMenuEntry<VoidCallback>> menu;

  @override
  State<GridScreen> createState() => _GridScreenState();
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

class _GridScreenState extends State<GridScreen> {
  /// The editor, through what the real screen will know of one.
  final CellEditor _editor = FlutterCellEditor();

  /// Everything the user does, one after the other in the order it was done (decision
  /// A158): each action finds the editor and the stage as the actions before it left
  /// them, not as they were when the finger came down. An error that an action did
  /// not expect is shown like the others.
  late final _actions = InTurn(
    onUnheard: (error, stack) {
      // With no screen left to show it on, it is an error that nobody caught.
      if (!mounted) Error.throwWithStackTrace(error, stack);
      _say(describeError(error));
    },
  );

  /// The cell whose text is being edited.
  String? _editing;

  /// The editing of that cell, as the keeper has it. It is over when its text was
  /// entered while more was typed, or its document was let go of; the next entering
  /// opens another.
  Editing? _edit;

  /// What the keeper asks through to enter what is being typed.
  late Attachment _attachment;

  /// What the last action had to say: an error, or that there was nothing to do. It has
  /// a line of its own, so that it never lies over something that can be tapped.
  String? _message;

  Stage get _stage => widget.stage;

  @override
  void initState() {
    super.initState();
    // Return: enter, and on to the cell below.
    _editor.onSubmit = () => _confirm(below: true);
    _arrive();
  }

  @override
  void didUpdateWidget(GridScreen old) {
    super.didUpdateWidget(old);
    if (identical(old.stage, _stage)) return;
    _leave(old.stage);
    _arrive();
  }

  @override
  void dispose() {
    _leave(_stage);
    _editor.dispose();
    super.dispose();
  }

  void _arrive() {
    _stage.addListener(_redraw);
    _attachment = _stage.keeper.attach(
      queue: _actions,
      finish: () => _enter(away: true),
    );
  }

  /// The screen stops showing `stage`. What is in the editor is entered, as by a tap
  /// elsewhere. That is a step of the stage like any other, so the keeper waits for it
  /// before it closes the document. There is no one left to show a refusal to.
  void _leave(Stage stage) {
    stage.removeListener(_redraw);
    _attachment.detach();
    final cell = _editing;
    if (cell != null) {
      stage
          .enterText(cell, _editor.text, editing: _edit)
          .then<void>((_) {}, onError: (Object _) {});
    }
    _editing = null;
    _edit = null;
  }

  void _redraw() => setState(() {});

  void _say(String? message) {
    if (mounted && message != null) setState(() => _message = message);
  }

  /// Takes one action of the user into the queue. What was on the message line was
  /// said about an action before this one, and goes.
  void _do(Future<void> Function() action) {
    if (_message != null) setState(() => _message = null);
    _actions.run(() async {
      // A screen that has gone does nothing more. (After a fault the stage refuses
      // every step, and the screen says so in place of the grid.)
      if (mounted) await action();
    });
  }

  /// Asks one thing of the stage and says whether it did what it was for. What goes
  /// wrong is shown in the message line, and the screen stays as it was.
  Future<bool> _ask(Future<Object?> Function() step, {String? nothing}) async {
    try {
      if (await step() != false) return true;
      _say(nothing);
    } catch (error) {
      _say(describeError(error));
    }
    return false;
  }

  /// Opens the editor on a cell, or moves it there. Like [_close] it does nothing on
  /// a screen that went while the action was at work.
  void _show(Editing editing) {
    if (!mounted) return;
    setState(() {
      _editing = editing.cell;
      _edit = editing;
      // The cursor goes to the end of the text when the field gets the focus.
      _editor.set(editing.start.text);
    });
    _editor.focus();
  }

  void _close() {
    if (!mounted) return;
    setState(() {
      _editing = null;
      _edit = null;
    });
  }

  /// Opens the editor on a cell of the stage as the cell is when the turn of this
  /// comes: an undo before the tap may have changed its text, or taken it away.
  /// Asked of the stage and not read from what it shows, so that it also comes
  /// after steps that did not come from this screen.
  /// Says whether it did.
  Future<bool> _open(String id) async {
    Editing? editing;
    await _ask(() async => editing = await _stage.edit(id));
    if (editing != null) _show(editing!);
    return editing != null;
  }

  /// Sends what is in the editor to the core. The editor stays as it is: if the core
  /// refuses, with what was typed, and if more was typed while the core worked, with
  /// that, which closing or moving the editor would throw away.
  ///
  /// `away` says that the app is being put away and the user has not said yes to the
  /// text: the editing holds it in the document and goes on, so that the cross can
  /// still take it back. The keeper asks for that, in the turn of the screen's
  /// actions, and saves when it is done; the editor stays as it is for when the app
  /// comes back.
  Future<_Entered> _enter({bool away = false}) async {
    final cell = _editing;
    if (cell == null) return _Entered.yes;
    final text = _editor.text;
    final entered = await _ask(() async {
      if (away) {
        _edit = await _stage.holdText(cell, text, editing: _edit);
      } else {
        await _stage.enterText(cell, text, editing: _edit);
      }
      return true;
    });
    if (!entered) return _Entered.refused;
    return _editor.text == text ? _Entered.yes : _Entered.typedOn;
  }

  /// A tap on a cell, on its text or on the way into its grid. While the editor is
  /// open, a tap outside it only closes the editor (UI_SPEC section 6), keeping what
  /// was typed; it does not open another cell or go into a grid. A cell that is no
  /// longer on the stage when the turn of its tap comes is not opened.
  void _tap(String id, {required bool dive}) => _do(() async {
    if (_editing != null) {
      if (await _enter() == _Entered.yes) _close();
    } else if (dive) {
      await _ask(() => _stage.dive(id));
    } else {
      await _open(id);
    }
  });

  /// The tick, and with `below` Return: enters the text and closes the editor, or
  /// moves it to the cell under the one edited, if there is one.
  void _confirm({bool below = false}) => _do(() async {
    final cell = _editing;
    if (await _enter() != _Entered.yes) return;
    // The stage was read just now, so the cell below is as the document has it.
    final grid = _stage.grid;
    final at = grid.cells.indexWhere((c) => c.id == cell);
    final next = at + grid.xs;
    final moved =
        below &&
        at >= 0 &&
        next < grid.cells.length &&
        await _open(grid.cells[next].id);
    if (!moved) _close();
  });

  /// The cross: "not this". The cell is as it was before the editor opened, also when
  /// the text went into the document meanwhile because the app was put away, and so
  /// is everything that could be undone or redone.
  void _cancel() => _do(() async {
    final editing = _edit;
    if (editing != null && !await _ask(() => _stage.drop(editing))) return;
    _close();
  });

  /// An action of the bars. What is being typed is kept first; if that cannot be done,
  /// the action is not, the editor stays, and the message line says why.
  void _act(
    Future<Object?> Function() action, {
    String? nothing,
  }) => _do(() async {
    final entered = await _enter();
    if (entered == _Entered.typedOn) {
      _say(
        'Not done: more was typed in the meantime. It is still in the editor.',
      );
    }
    // Nor is it done on a screen that went while the text was entered.
    if (entered != _Entered.yes || !mounted) return;
    _close();
    await _ask(action, nothing: nothing);
  });

  /// The system's back (UI_SPEC section 10): up one level, and out of the app only
  /// from the root.
  void _back() => _act(() async {
    if (!_stage.atRoot) return _stage.up();
    await SystemNavigator.pop();
    return true;
  });

  /// The other program's file is chosen over this document, and what is being typed
  /// goes with the rest. If that file cannot be read, nothing is given up.
  void _openTheirs() => _do(() async {
    try {
      await _stage.openTheirs();
    } catch (error) {
      // Only the core's answer to reading the file says something about the file.
      final theirs =
          isFatal(error) ||
          error is BridgeError && error.kind != ErrorKind.refused;
      return _say(
        '${theirs ? 'The file as the other program left it cannot be opened' : 'The other program\'s file was not opened'}, '
        'so this document is still open, and not saved. '
        '${isFatal(error) ? 'Reading it ran into a fault.' : describeError(error)}',
      );
    }
    _close();
  });

  @override
  Widget build(BuildContext context) {
    final ended = _stage.ended;
    final grid = _stage.grid;
    final title = grid.cell.text.isEmpty ? 'Wombat' : grid.cell.text;
    return PopScope(
      // Never by itself: the back action decides when its turn comes. After a fault
      // nothing is left to go up in.
      canPop: ended != null,
      onPopInvokedWithResult: (popped, _) {
        if (!popped) _back();
      },
      child: Scaffold(
        appBar: AppBar(
          title: Text(title, key: const Key('title')),
          actions: [
            IconButton(
              key: const Key('undo'),
              tooltip: 'Undo',
              icon: const Icon(Icons.undo),
              onPressed: ended != null
                  ? null
                  : () => _act(_stage.undo, nothing: 'Nothing to undo'),
            ),
            IconButton(
              key: const Key('redo'),
              tooltip: 'Redo',
              icon: const Icon(Icons.redo),
              onPressed: ended != null
                  ? null
                  : () => _act(_stage.redo, nothing: 'Nothing to redo'),
            ),
            // No Save: the document saves itself (UI_SPEC has none either).
            if (widget.menu.isNotEmpty)
              PopupMenuButton<VoidCallback>(
                key: const Key('menu'),
                onSelected: (action) => action(),
                itemBuilder: (context) => widget.menu,
              ),
          ],
        ),
        body: SafeArea(
          child: ended != null
              ? Center(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Text(ended, key: const Key('ended')),
                  ),
                )
              : Column(
                  children: [
                    Expanded(child: _cells(context, grid)),
                    ?_trouble(context),
                    if (_message != null) _messageLine(context, _message!),
                    const Divider(height: 1),
                    _trail(context),
                    if (_editing != null) _editorBar(context),
                  ],
                ),
        ),
      ),
    );
  }

  /// What stays wrong until something is done about it, with the ways to do that: the
  /// document is not in its file, or the stage could not be read again. Unlike the
  /// message line it does not go away with the next action.
  Widget? _trouble(BuildContext context) {
    final saveError = _stage.saveError;
    final readError = _stage.readError;
    final String text;
    final List<(String, String, VoidCallback)> ways;
    if (saveError is BridgeError && saveError.kind == ErrorKind.changedOnDisk) {
      text = 'Not saved: another program changed the file.';
      ways = [
        ('keep-mine', 'Keep mine', () => _do(_stage.keepMine)),
        ('open-theirs', 'Open theirs', _openTheirs),
      ];
    } else if (saveError != null) {
      text = 'Not saved. ${describeError(saveError)}';
      ways = [('save-again', 'Try again', () => _do(_stage.save))];
    } else if (readError != null) {
      text = 'What is shown may be old. ${describeError(readError)}';
      ways = [
        ('read-again', 'Try again', () => _do(() => _ask(_stage.refresh))),
      ];
    } else {
      return null;
    }
    final colors = Theme.of(context).colorScheme;
    return Material(
      color: colors.errorContainer,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12),
        child: Row(
          children: [
            Expanded(
              child: Text(
                text,
                key: const Key('trouble'),
                style: TextStyle(color: colors.onErrorContainer),
              ),
            ),
            for (final (key, label, action) in ways)
              TextButton(
                key: Key(key),
                style: TextButton.styleFrom(
                  minimumSize: const Size(_target, _target),
                ),
                onPressed: action,
                child: Text(label),
              ),
          ],
        ),
      ),
    );
  }

  Widget _cells(BuildContext context, GridInfo grid) {
    if (grid.xs == 0) {
      return const Center(
        key: Key('no-grid'),
        child: Text('This cell has no grid.'),
      );
    }
    final border = BorderSide(color: Theme.of(context).dividerColor);
    return SingleChildScrollView(
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Table(
          key: const Key('grid'),
          border: TableBorder.all(color: border.color),
          defaultColumnWidth: const FixedColumnWidth(150),
          defaultVerticalAlignment: TableCellVerticalAlignment.top,
          children: [
            for (var y = 0; y < grid.ys; y++)
              TableRow(
                children: [
                  for (var x = 0; x < grid.xs; x++)
                    _cell(
                      context,
                      y * grid.xs + x,
                      grid.cells[y * grid.xs + x],
                    ),
                ],
              ),
          ],
        ),
      ),
    );
  }

  /// A cell: its text, which opens the editor, and under it the way into its grid.
  Widget _cell(BuildContext context, int index, CellInfo cell) {
    final colors = Theme.of(context).colorScheme;
    final result = cell.result;
    final Color? background = cell.id == _editing
        ? colors.primaryContainer
        : _stage.touched.contains(cell.id)
        ? colors.tertiaryContainer
        : null;
    return ColoredBox(
      color: background ?? Colors.transparent,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          InkWell(
            key: Key('cell-$index'),
            onTap: () => _tap(cell.id, dive: false),
            child: ConstrainedBox(
              constraints: const BoxConstraints(minHeight: _target),
              child: Padding(
                padding: const EdgeInsets.all(8),
                child: Text(
                  result ?? cell.text,
                  maxLines: 6,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: cell.failed ? colors.error : null,
                    fontStyle: result == null ? null : FontStyle.italic,
                  ),
                ),
              ),
            ),
          ),
          if (cell.hasGrid)
            InkWell(
              key: Key('open-$index'),
              onTap: () => _tap(cell.id, dive: true),
              child: Container(
                constraints: const BoxConstraints(minHeight: _target),
                margin: const EdgeInsets.fromLTRB(6, 0, 6, 6),
                decoration: BoxDecoration(
                  border: Border.all(color: colors.outline),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: const Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Icon(Icons.grid_on, size: 18),
                    SizedBox(width: 6),
                    Text('Open ›'),
                  ],
                ),
              ),
            ),
        ],
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
          width: double.infinity,
          constraints: const BoxConstraints(minHeight: _target),
          alignment: Alignment.centerLeft,
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

  /// The way from the root to here: back one level, or to any level by its name.
  Widget _trail(BuildContext context) {
    final trail = _stage.trail;
    return SizedBox(
      height: _target,
      child: Row(
        children: [
          IconButton(
            key: const Key('up'),
            tooltip: 'Up one level',
            icon: const Icon(Icons.chevron_left),
            onPressed: _stage.atRoot ? null : () => _act(_stage.up),
          ),
          Expanded(
            child: ListView(
              scrollDirection: Axis.horizontal,
              reverse: true,
              children: [
                for (var i = trail.length - 1; i >= 0; i--)
                  TextButton(
                    key: Key('trail-$i'),
                    onPressed: i == trail.length - 1
                        ? null
                        : () => _act(() => _stage.jumpTo(i)),
                    child: Text(
                      '${i == 0 ? '' : '› '}'
                      '${trail[i].text.isEmpty ? '(empty)' : trail[i].text}',
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _editorBar(BuildContext context) {
    return Material(
      elevation: 8,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(8, 4, 0, 4),
        child: Row(
          children: [
            Expanded(child: _editor.build(context, key: const Key('editor'))),
            IconButton(
              key: const Key('save'),
              tooltip: 'Save',
              icon: const Icon(Icons.check),
              onPressed: _confirm,
            ),
            IconButton(
              key: const Key('cancel'),
              tooltip: 'Cancel',
              icon: const Icon(Icons.close),
              onPressed: _cancel,
            ),
          ],
        ),
      ),
    );
  }
}
