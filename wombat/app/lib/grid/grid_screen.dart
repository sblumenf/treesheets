import 'package:flutter/material.dart';

import '../session.dart';
import '../src/rust/api/wire.dart';
import 'stage.dart';

/// The smallest height of anything that can be tapped (UI_SPEC section 3).
const double _target = 48;

/// The simple grid screen: one grid, into a nested grid and back out, and the text of one
/// cell edited at a time, with undo and redo; the document saves itself. A first step towards the screen that
/// `docs/UI_SPEC.md` specifies, with none of its previews, gestures or animations.
class GridScreen extends StatefulWidget {
  const GridScreen({super.key, required this.stage, this.menu = const []});

  final Stage stage;

  /// More entries for the menu of the title bar.
  final List<PopupMenuEntry<VoidCallback>> menu;

  @override
  State<GridScreen> createState() => _GridScreenState();
}

class _GridScreenState extends State<GridScreen> with WidgetsBindingObserver {
  final _editor = TextEditingController();
  final _focus = FocusNode();

  /// The cell whose text is being edited.
  String? _editing;

  /// What the last action had to say: an error, or that there was nothing to do. It has
  /// a line of its own, so that it never lies over something that can be tapped.
  String? _message;

  Stage get _stage => widget.stage;

  @override
  void initState() {
    super.initState();
    _stage.addListener(_redraw);
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _stage.removeListener(_redraw);
    _editor.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _redraw() => setState(() {});

  /// The app is no longer in front, and the system may end it without another word:
  /// what is being typed goes into the document and the document into its file, now.
  /// The editor stays as it is for when the app comes back.
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed || _stage.ended != null) return;
    _commit(close: false);
    _stage.save();
  }

  /// Runs one action and says whether it did what it was for. Nothing is dropped while
  /// another runs: the stage takes its steps in the order they were asked for. What
  /// goes wrong is shown in the message line, and the screen stays as it was.
  Future<bool> _run(
    Future<Object?> Function() action, {
    String? nothing,
  }) async {
    if (_message != null) setState(() => _message = null);
    String? message;
    var done = false;
    try {
      done = await action() != false;
      if (!done) message = nothing;
    } catch (error) {
      message = describeError(error);
    }
    if (mounted && message != null) setState(() => _message = message);
    return done;
  }

  /// Opens the editor on a cell of the stage, as it is when everything asked for before
  /// the tap is done: an undo still under way must not leave the editor with the text
  /// from before it.
  Future<void> _edit(String id) => _run(() async {
    final cell = await _stage.cell(id);
    if (cell != null && mounted && _editing == null) _show(cell);
    return true;
  });

  void _show(CellInfo cell) {
    setState(() {
      _editing = cell.id;
      _editor.text = cell.text;
      _editor.selection = TextSelection.collapsed(offset: cell.text.length);
    });
    _focus.requestFocus();
  }

  void _close() => setState(() => _editing = null);

  /// Sends what was typed to the core, and then closes the editor, or with `below`
  /// moves it to the cell under the one edited, if there is one. Returns whether the
  /// text is in the document and the editor where it was asked to be; with no editor
  /// open that is so at once.
  ///
  /// If the core refuses, the editor stays open with what was typed. If more was typed
  /// while the core worked, the editor stays as it is, with that: closing or moving it
  /// would throw those keys away.
  Future<bool> _commit({bool close = true, bool below = false}) async {
    final cell = _editing;
    if (cell == null) return true;
    final text = _editor.text;
    if (!await _run(() => _stage.enterText(cell, text))) return false;
    if (!mounted || _editing != cell || _editor.text != text) return false;
    if (!close) return true;
    final grid = _stage.grid;
    final at = grid.cells.indexWhere((c) => c.id == cell);
    final next = at + grid.xs;
    if (below && at >= 0 && grid.xs > 0 && next < grid.cells.length) {
      // The stage was read just now, so the cell below is as the document has it.
      _show(grid.cells[next]);
    } else {
      _close();
    }
    return true;
  }

  /// A tap on a cell, on its text or on the way into its grid. While the editor is
  /// open, a tap outside it only closes the editor (UI_SPEC section 6), keeping what
  /// was typed; it does not open another cell or go into a grid.
  Future<void> _tap(CellInfo cell, {required bool dive}) async {
    if (_editing != null) {
      await _commit();
    } else if (dive) {
      await _run(() => _stage.dive(cell.id));
    } else {
      await _edit(cell.id);
    }
  }

  /// An action of the bars. What is being typed is kept first; if that cannot be done,
  /// the action is not, and the editor stays.
  Future<void> _act(Future<bool> Function() action, {String? nothing}) async {
    if (await _commit()) await _run(action, nothing: nothing);
  }

  @override
  Widget build(BuildContext context) {
    final ended = _stage.ended;
    final grid = _stage.grid;
    final title = grid.cell.text.isEmpty ? 'Wombat' : grid.cell.text;
    return Scaffold(
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
        ('keep-mine', 'Keep mine', _stage.keepMine),
        // The choice is the other file, so what is being typed goes with the rest.
        (
          'open-theirs',
          'Open theirs',
          () {
            _close();
            _stage.openTheirs();
          },
        ),
      ];
    } else if (saveError != null) {
      text = 'Not saved. ${describeError(saveError)}';
      ways = [('save-again', 'Try again', _stage.save)];
    } else if (readError != null) {
      text = 'What is shown may be old. ${describeError(readError)}';
      ways = [('read-again', 'Try again', () => _run(_stage.refresh))];
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
            onTap: () => _tap(cell, dive: false),
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
              onTap: () => _tap(cell, dive: true),
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
            Expanded(
              child: TextField(
                key: const Key('editor'),
                controller: _editor,
                focusNode: _focus,
                textInputAction: TextInputAction.next,
                decoration: const InputDecoration(
                  isDense: true,
                  border: OutlineInputBorder(),
                ),
                // Return: save, and on to the cell below.
                onSubmitted: (_) => _commit(below: true),
              ),
            ),
            IconButton(
              key: const Key('save'),
              tooltip: 'Save',
              icon: const Icon(Icons.check),
              onPressed: _commit,
            ),
            IconButton(
              key: const Key('cancel'),
              tooltip: 'Cancel',
              icon: const Icon(Icons.close),
              onPressed: _close,
            ),
          ],
        ),
      ),
    );
  }
}
