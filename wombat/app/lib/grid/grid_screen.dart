import 'package:flutter/material.dart';

import '../demo.dart';
import '../src/rust/api/wire.dart';
import 'stage.dart';

/// The smallest height of anything that can be tapped (UI_SPEC section 3).
const double _target = 48;

/// The simple grid screen: one grid, into a nested grid and back out, and the text of one
/// cell edited at a time, with undo and redo. A first step towards the screen that
/// `docs/UI_SPEC.md` specifies, with none of its previews, gestures or animations.
class GridScreen extends StatefulWidget {
  const GridScreen({super.key, required this.stage, this.menu = const []});

  final Stage stage;

  /// More entries for the menu of the title bar.
  final List<PopupMenuEntry<VoidCallback>> menu;

  @override
  State<GridScreen> createState() => _GridScreenState();
}

class _GridScreenState extends State<GridScreen> {
  final _editor = TextEditingController();
  final _focus = FocusNode();

  /// The cell whose text is being edited.
  String? _editing;
  bool _busy = false;

  /// What the last action had to say: an error, or that there was nothing to do. It has
  /// a line of its own, so that it never lies over something that can be tapped.
  String? _message;

  Stage get _stage => widget.stage;

  @override
  void initState() {
    super.initState();
    _stage.addListener(_redraw);
  }

  @override
  void dispose() {
    _stage.removeListener(_redraw);
    _editor.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _redraw() => setState(() {});

  /// Runs one action; a second one that starts while it runs is dropped. What goes
  /// wrong is shown in the message line, and the screen stays as it was.
  Future<void> _run(
    Future<Object?> Function() action, {
    String? nothing,
  }) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _message = null;
    });
    String? message;
    try {
      if (await action() == false) message = nothing;
    } catch (error) {
      message = describeError(error);
    }
    if (!mounted) return;
    setState(() {
      _busy = false;
      _message = message;
    });
  }

  void _edit(CellInfo cell) {
    if (_busy) return;
    setState(() {
      _editing = cell.id;
      _editor.text = cell.text;
      _editor.selection = TextSelection.collapsed(offset: cell.text.length);
    });
    _focus.requestFocus();
  }

  void _close() => setState(() => _editing = null);

  /// Sends what was typed to the core and closes the editor, or with `below` moves it to
  /// the cell under the one edited, if there is one. If the core refuses, the editor
  /// stays open with what was typed.
  Future<void> _save({bool below = false}) async {
    final cell = _editing;
    if (cell == null) return;
    var saved = false;
    await _run(() async {
      await _stage.enterText(cell, _editor.text);
      saved = true;
      return true;
    });
    if (!saved || !mounted) return;
    final grid = _stage.grid;
    final at = grid.cells.indexWhere((c) => c.id == cell);
    final next = at + grid.xs;
    if (below && at >= 0 && grid.xs > 0 && next < grid.cells.length) {
      _edit(grid.cells[next]);
    } else {
      _close();
    }
  }

  Future<void> _dive(CellInfo cell) async {
    _close();
    await _run(() => _stage.dive(cell.id));
  }

  @override
  Widget build(BuildContext context) {
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
            onPressed: () => _run(_stage.undo, nothing: 'Nothing to undo'),
          ),
          IconButton(
            key: const Key('redo'),
            tooltip: 'Redo',
            icon: const Icon(Icons.redo),
            onPressed: () => _run(_stage.redo, nothing: 'Nothing to redo'),
          ),
          PopupMenuButton<VoidCallback>(
            key: const Key('menu'),
            onSelected: (action) => action(),
            itemBuilder: (context) => [
              PopupMenuItem(
                value: () => _run(() async {
                  await _stage.save();
                  return true;
                }),
                child: const Text('Save'),
              ),
              ...widget.menu,
            ],
          ),
        ],
      ),
      body: SafeArea(
        child: Column(
          children: [
            Expanded(child: _cells(context, grid)),
            if (_message != null) _messageLine(context, _message!),
            const Divider(height: 1),
            _trail(context),
            if (_editing != null) _editorBar(context),
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
            onTap: () => _edit(cell),
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
              onTap: () => _dive(cell),
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
            onPressed: _stage.atRoot
                ? null
                : () {
                    _close();
                    _run(_stage.up);
                  },
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
                        : () {
                            _close();
                            _run(() => _stage.jumpTo(i));
                          },
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
                onSubmitted: (_) => _save(below: true),
              ),
            ),
            IconButton(
              key: const Key('save'),
              tooltip: 'Save',
              icon: const Icon(Icons.check),
              onPressed: _save,
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
