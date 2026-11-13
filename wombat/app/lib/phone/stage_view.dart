import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

import '../src/rust/api/wire.dart';
import 'budget.dart';
import 'cell_view.dart';
import 'place.dart';
import 'words.dart';

/// The stage's grid, scrolling both ways (UI_SPEC section 4): every row a strip of
/// cells as wide as its columns and as tall as its tallest cell, the first row and
/// the first column kept in view if the settings say so, and after the last row and
/// the last column the bars that add one (section 8).
///
/// Where a [StageView] is: its owner makes one and asks it. It answers while the
/// view is there.
class StagePlace {
  /// The column and the row at the top left of what is seen, counted from 0 in
  /// the stage's grid, as last drawn; null without a view.
  ({int x, int y})? Function() read = () => null;
}

/// Only the rows near what is seen are built. The view is made anew for every level
/// (give it a key that says which arrival it is of): it starts where the level was
/// left (`scene.stage.level`), and says where it is whenever it has come to rest
/// ([onPlace]).
///
/// Keys: `stage-across` and `stage-down` are the two scrolling parts, `add-row` and
/// `add-column` the bars; the cells are named by [CellView].
class StageView extends StatefulWidget {
  const StageView({
    super.key,
    required this.scene,
    required this.place,
    required this.onPlace,
    required this.onAddRow,
    required this.onAddColumn,
  });

  final Scene scene;

  /// Through this the view says where it is whenever it is asked.
  final StagePlace place;

  /// Told the column and the row at the top left of what is seen, counted from 0
  /// in the stage's grid, when scrolling has come to rest and been drawn.
  final void Function(int x, int y) onPlace;
  final VoidCallback onAddRow;
  final VoidCallback onAddColumn;

  @override
  State<StageView> createState() => _StageViewState();
}

class _StageViewState extends State<StageView> {
  ScrollController? _across;
  ScrollController _down = ScrollController();

  /// Scrolls the first row in what it is given, while it is kept in view.
  final _top = ScrollController();

  /// The row that is at the top when the list is scrolled to its zero: the rows
  /// before it are laid out upwards from there, so that the list can start at any
  /// row without knowing how tall the rows before it are.
  late int _anchor;

  /// Counts the times the list started again at another anchor.
  int _starts = 0;
  final _center = UniqueKey();
  final _list = GlobalKey();
  final _first = GlobalKey();
  final _content = GlobalKey();
  final _rows = <int, GlobalKey>{};

  /// The left edge of every column, and the right edge of the last.
  List<double> _lefts = const [0];
  Size? _size;
  bool _pinRow = false;
  bool _pinColumn = false;

  Scene get _scene => widget.scene;

  @override
  void initState() {
    super.initState();
    _anchor = _scene.stage.level?.y ?? 0;
    widget.place.read = _place;
  }

  @override
  void didUpdateWidget(StageView old) {
    super.didUpdateWidget(old);
    final editing = _scene.editing;
    if (editing != null && editing != old.scene.editing) _revealSoon();
    if (!identical(old.place, widget.place)) widget.place.read = _place;
  }

  @override
  void dispose() {
    // Unless the view of the next level has taken its place already.
    if (widget.place.read == _place) widget.place.read = () => null;
    _across?.dispose();
    _down.dispose();
    _top.dispose();
    super.dispose();
  }

  /// How far the grid is scrolled sideways, for the cells that stay in view.
  double get _shift {
    final across = _across;
    if (across == null || !across.hasClients) return 0;
    return across.position.pixels;
  }

  int _columnAt(double offset) {
    var x = 0;
    while (x + 2 < _lefts.length && _lefts[x + 1] <= offset + 0.5) {
      x++;
    }
    return x;
  }

  /// The first row of the list of which something is seen.
  int _topRow() {
    final list = _list.currentContext?.findRenderObject();
    int? top;
    for (final MapEntry(key: row, value: key) in _rows.entries) {
      final box = key.currentContext?.findRenderObject();
      if (list == null || box is! RenderBox || !_laidOut(box)) continue;
      final bottom =
          box.localToGlobal(Offset.zero, ancestor: list).dy + box.size.height;

      if (bottom > 1 && (top == null || row < top)) top = row;
    }
    return top ?? _anchor;
  }

  /// Whether `box` is where the list last put it. A row that is out of sight can
  /// still be there, kept alive by a tap's ripple, and is then nowhere: its place
  /// is whatever it was when it was last seen.
  static bool _laidOut(RenderObject box) {
    if (!box.attached) return false;
    for (RenderObject? at = box; at != null; at = at.parent) {
      final data = at.parentData;
      if (data is SliverMultiBoxAdaptorParentData) return !data.keptAlive;
    }
    // Not in the list: the first row, kept in view.
    return true;
  }

  ({int x, int y})? _place() =>
      mounted ? (x: _columnAt(_shift), y: _topRow()) : null;

  void _tellPlace() {
    final place = _place();
    if (place != null) widget.onPlace(place.x, place.y);
  }

  void _revealSoon() =>
      WidgetsBinding.instance.addPostFrameCallback((_) => _reveal(again: true));

  /// The cell of the stage's grid that holds the edited cell, or is it.
  ({int x, int y})? _editedAt() {
    final stage = _scene.stage;
    var id = _scene.editing;
    for (var i = 0; id != null && i < 64; i++) {
      final place = placeOf(stage, id);
      if (place == null) return null;
      if (place.part.level == 0) return (x: place.x, y: place.y);
      id = place.part.owner;
    }
    return null;
  }

  /// Scrolls so that the edited cell is seen (UI_SPEC section 7: it stays visible
  /// above the keyboard). If its row is not built, the list starts again there.
  void _reveal({required bool again}) {
    if (!mounted) return;
    final at = _editedAt();
    final across = _across;
    if (at == null || across == null) return;
    final box = _scene.editedKey.currentContext?.findRenderObject();
    if (box is! RenderBox || !_laidOut(box)) {
      if (!again || _pinRow && at.y == 0) return;
      setState(() {
        final old = _down;
        WidgetsBinding.instance.addPostFrameCallback((_) => old.dispose());
        _down = ScrollController();
        _anchor = at.y;
        _starts++;
      });
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _reveal(again: false);
        _tellPlace();
      });
      return;
    }
    // The first row, while it is kept in view, scrolls by itself in its share of
    // the height, which is small above a keyboard.
    final pinned = _pinRow && at.y == 0;
    final down = pinned ? _top : _down;
    final list = (pinned ? _first : _list).currentContext?.findRenderObject();
    if (down.hasClients && list is RenderBox) {
      // By hand, and not by the scroll position's own means: those look for the
      // nearest thing that scrolls, which for a cell of an unfolded grid is not
      // this list.
      final position = down.position;
      final top = box.localToGlobal(Offset.zero, ancestor: list).dy;
      final over = top + box.size.height - list.size.height;
      // The bottom first; then the top, which wins when the cell is too tall.
      var by = over > 0 ? over : 0.0;
      if (top - by < 0) by = top;
      if (by != 0) {
        position.jumpTo(
          (position.pixels + by).clamp(
            position.minScrollExtent,
            position.maxScrollExtent,
          ),
        );
      }
    }
    final content = _content.currentContext?.findRenderObject();
    final size = _size;
    if (!(_pinColumn && at.x == 0) &&
        across.hasClients &&
        content is RenderBox &&
        size != null) {
      final left = box.localToGlobal(Offset.zero, ancestor: content).dx;
      final right = left + box.size.width;
      final position = across.position;
      final start = position.pixels + (_pinColumn ? _lefts[1] : 0);
      double? to;
      if (left < start) {
        to = left - (_pinColumn ? _lefts[1] : 0);
      } else if (right > position.pixels + size.width) {
        to = right - size.width;
      }
      if (to != null) {
        position.jumpTo(
          to.clamp(position.minScrollExtent, position.maxScrollExtent),
        );
      }
    }
    _tellPlace();
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, box) {
      final colors = Theme.of(context).colorScheme;
      final grid = _scene.stage.grid;
      if (grid == null || grid.cols == 0) return const SizedBox();
      final widths = columnWidths(
        grid.colWidths,
        viewport: box.maxWidth,
        textScale: _scene.textScale,
      );
      final lefts = <double>[0];
      for (final width in widths) {
        lefts.add(lefts.last + width);
      }
      _lefts = lefts;
      final total = lefts.last;
      // (Not where it would leave less than two cells' height for the rest.)
      _pinRow =
          _scene.variants.pin && grid.ys > 1 && box.maxHeight >= 3 * target;
      _pinColumn = _scene.variants.pin && widths[0] <= box.maxWidth * 0.4;
      // The first time, where the level was left.
      _across ??= ScrollController(
        initialScrollOffset:
            lefts[(_scene.stage.level?.x ?? 0).clamp(0, widths.length - 1)],
      );
      final size = Size(box.maxWidth, box.maxHeight);
      // The keyboard came or went, or the screen was turned.
      if (_size != null && _size != size && _scene.editing != null) {
        _revealSoon();
      }
      _size = size;
      final first = _pinRow ? 1 : 0;
      final anchor = _anchor.clamp(first, grid.ys - 1);
      _rows.removeWhere((row, _) => row >= grid.ys || row < first);
      final line = BorderSide(color: colors.outline);
      return NotificationListener<ScrollEndNotification>(
        onNotification: (_) {
          // Scrolling comes to rest before the rows are laid out where it
          // ended, and sometimes in the middle of a layout, where nothing may
          // be measured: the place is read when the frame is done that the
          // scrolling itself has asked for.
          WidgetsBinding.instance.addPostFrameCallback((_) => _tellPlace());
          return false;
        },
        child: SingleChildScrollView(
          key: const Key('stage-across'),
          controller: _across,
          scrollDirection: Axis.horizontal,
          child: SizedBox(
            key: _content,
            width: total + plusBar,
            height: box.maxHeight,
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                SizedBox(
                  width: total,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (_pinRow)
                        // A first row that is very tall scrolls in what it is
                        // given, so that the rows below always have room.
                        ConstrainedBox(
                          constraints: BoxConstraints(
                            maxHeight: box.maxHeight * 0.4,
                          ),
                          child: DecoratedBox(
                            position: DecorationPosition.foreground,
                            decoration: BoxDecoration(
                              border: Border(bottom: line),
                            ),
                            child: KeyedSubtree(
                              key: _first,
                              child: SingleChildScrollView(
                                key: const Key('stage-first-row'),
                                controller: _top,
                                child: _row(grid, widths, 0),
                              ),
                            ),
                          ),
                        ),
                      Expanded(
                        child: KeyedSubtree(
                          key: _list,
                          child: CustomScrollView(
                            key: ValueKey('stage-down-$_starts'),
                            controller: _down,
                            center: _center,
                            slivers: [
                              SliverList(
                                delegate: SliverChildBuilderDelegate(
                                  (context, i) =>
                                      _row(grid, widths, anchor - 1 - i),
                                  childCount: anchor - first,
                                ),
                              ),
                              SliverList(
                                key: _center,
                                delegate: SliverChildBuilderDelegate(
                                  (context, i) => anchor + i < grid.ys
                                      ? _row(grid, widths, anchor + i)
                                      : _bar(
                                          'add-row',
                                          Words.addRow,
                                          widget.onAddRow,
                                          Border(top: line),
                                        ),
                                  childCount: grid.ys - anchor + 1,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                SizedBox(
                  width: plusBar,
                  child: _bar(
                    'add-column',
                    Words.addColumn,
                    widget.onAddColumn,
                    Border(left: line),
                  ),
                ),
              ],
            ),
          ),
        ),
      );
    },
  );

  /// A bar that adds a row or a column.
  Widget _bar(String key, String label, VoidCallback onTap, Border border) =>
      Semantics(
        button: true,
        label: label,
        child: InkWell(
          key: Key(key),
          onTap: onTap,
          child: Container(
            constraints: const BoxConstraints(
              minHeight: plusBar,
              minWidth: plusBar,
            ),
            decoration: BoxDecoration(border: border),
            alignment: Alignment.center,
            child: const Icon(Icons.add),
          ),
        ),
      );

  /// Row `y` of the stage's grid.
  Widget _row(GridPart grid, List<double> widths, int y) {
    final cells = <Widget>[
      for (var x = 0; x < widths.length; x++)
        SizedBox(
          width: widths[x],
          child: CellView(
            scene: _scene,
            look:
                _scene.stage.cells[grid.first + (y - grid.y0) * grid.cols + x],
            name: '${grid.x0 + x}-$y',
            width: widths[x],
          ),
        ),
    ];
    final pinned = _pinRow && y == 0;
    final key = pinned ? null : _rows.putIfAbsent(y, GlobalKey.new);
    if (!_pinColumn) {
      return IntrinsicHeight(
        key: key,
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: cells,
        ),
      );
    }
    // The first cell stays in view: it is moved by as much as the grid is
    // scrolled, and it is the last child, so that it is drawn over the others and
    // a tap finds it first. The row is laid out from the right for that.
    cells[0] = AnimatedBuilder(
      animation: _across!,
      builder: (context, child) =>
          Transform.translate(offset: Offset(_shift, 0), child: child),
      child: Material(elevation: 2, child: cells[0]),
    );
    return IntrinsicHeight(
      key: key,
      child: Row(
        textDirection: TextDirection.rtl,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: cells.reversed.toList(),
      ),
    );
  }
}
