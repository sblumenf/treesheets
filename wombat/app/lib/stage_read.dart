import 'package:flutter/foundation.dart';

import 'src/rust/api/wire.dart';

/// One answer of a stage read ([StageInfo]) with the ways through it that a screen
/// needs, and the comparison with the answer before it.
///
/// It holds what was read and nothing else: after a change to the document a new one
/// is read, and [since] says what of it a screen has to draw again.
class StageRead {
  /// The answer `info` to the request `ask` for the stage `cell`.
  StageRead(this.cell, this.ask, this.info)
    : cells = info.cells,
      summaries = info.summaries,
      grids = info.grids;

  /// The ID of the cell whose grid is the stage.
  final String cell;

  /// What was asked.
  final StageAsk ask;

  /// The answer as the bridge gave it.
  final StageInfo info;

  /// The cells of all [grids], grid after grid and each row by row.
  final List<CellLook> cells;

  /// One for each of [cells] that has a grid, in the order of those cells.
  final List<GridSummary> summaries;

  /// The grids given cell by cell: the stage's own first, if the stage has one.
  final List<GridPart> grids;

  /// Where each cell is in [cells], by its ID.
  late final Map<String, int> _places = {
    for (var i = 0; i < cells.length; i++) cells[i].id: i,
  };

  /// The cells from the root to the stage, the root first and the stage last.
  List<TrailStep> get trail => info.trail;

  /// What this level remembers on this device: the column and row at the top left of
  /// what was last seen, and the cell selected there. Null for a level that was never
  /// left, and for one whose cell was removed in between.
  LevelInfo? get level => info.level;

  /// The stage's own grid, or the window of it that was asked for; null if the stage
  /// cell has no grid.
  GridPart? get grid => grids.isEmpty ? null : grids.first;

  /// Whether the stage's grid is given whole, and not a window of it.
  bool get whole {
    final grid = this.grid;
    return grid == null || grid.cols == grid.xs && grid.rows == grid.ys;
  }

  /// The cell with this ID, if the answer has it: a cell of the stage's grid (inside
  /// the window) or of a nested grid that is given cell by cell.
  CellLook? look(String id) {
    final at = _places[id];
    return at == null ? null : cells[at];
  }

  /// The reduced form of the grid of `look`, from which a Preview, a Shape and a Badge
  /// are drawn; null for a cell without a grid.
  GridSummary? summary(CellLook look) {
    final at = look.summary;
    return at == null ? null : summaries[at];
  }

  /// The grid of `look` cell by cell, if the answer gives it so (a Full child).
  GridPart? full(CellLook look) {
    final at = look.full;
    return at == null ? null : grids[at];
  }

  /// The cell of `part` in column `column` of row `row`, both counted in the whole
  /// grid from 0; null outside the part that is given.
  CellLook? at(GridPart part, int column, int row) {
    final i = column - part.x0;
    final j = row - part.y0;
    if (i < 0 || j < 0 || i >= part.cols || j >= part.rows) return null;
    return cells[part.first + j * part.cols + i];
  }

  /// What is to be drawn again when this answer takes the place of `before`.
  ///
  /// Cells are told apart by their ID, wherever they are in the answers: a cell that
  /// moved in the lists because another gained or lost a grid, or because the window
  /// moved, is the same cell and is named only if it looks different. The comparison
  /// reads every cell of both answers once; where nothing changed that is all it does.
  StageChange since(StageRead before) {
    if (before.cell != cell) return StageChange.all;
    final looks = <String>{};
    final inside = <String>{};
    final laidOut = <String>{};
    for (final now in cells) {
      final place = before._places[now.id];
      if (place == null) {
        looks.add(now.id);
        continue;
      }
      final was = before.cells[place];
      if (!_sameLook(was, now)) looks.add(now.id);
      if (before.summary(was) != summary(now)) inside.add(now.id);
      if (!_samePart(before.full(was), full(now))) laidOut.add(now.id);
    }
    if (!_samePart(before.grid, grid)) laidOut.add(cell);
    return StageChange._(
      everything: false,
      trail: !listEquals(before.trail, trail),
      looks: looks,
      summaries: inside,
      grids: laidOut,
      gone: {
        for (final was in before.cells)
          if (!_places.containsKey(was.id)) was.id,
      },
    );
  }

  /// Whether two reads of a cell draw the same. Its places in the lists of summaries
  /// and grids are not what it looks like: they move when a cell before it gains or
  /// loses a grid.
  static bool _sameLook(CellLook was, CellLook now) =>
      was == now || was.copyWith(summary: now.summary, full: now.full) == now;

  /// The same for a grid given cell by cell, whose place in the list of cells moves.
  static bool _samePart(GridPart? was, GridPart? now) =>
      was == now ||
      was != null && now != null && was.copyWith(first: now.first) == now;
}

/// What differs between two reads of a stage ([StageRead.since]): what a screen draws
/// again. Everything that is not named here is exactly as it was.
class StageChange {
  const StageChange._({
    required this.everything,
    required this.trail,
    required this.looks,
    required this.summaries,
    required this.grids,
    required this.gone,
  });

  /// Nothing differs.
  static const none = StageChange._(
    everything: false,
    trail: false,
    looks: {},
    summaries: {},
    grids: {},
    gone: {},
  );

  /// The stage is another cell (a dive, a way up, a document opened again): nothing
  /// of the two answers was compared, and everything is drawn anew. The sets are empty.
  static const all = StageChange._(
    everything: true,
    trail: true,
    looks: {},
    summaries: {},
    grids: {},
    gone: {},
  );

  /// Whether the stage is another cell than before, so that nothing compares.
  final bool everything;

  /// Whether the trail differs: a cell on the way from the root shows another text or
  /// colour. (Always true with [everything].)
  final bool trail;

  /// The IDs of the cells that look different themselves (text, result, colours,
  /// style, runs, links, image, note, whether there is a grid, whether it is folded),
  /// and of the cells that the answer before did not have.
  final Set<String> looks;

  /// The IDs of the cells whose grid's reduced form differs (its size, its preview
  /// texts, its shape), appeared or went: what a Preview, Shape or Badge is drawn from.
  final Set<String> summaries;

  /// The IDs of the cells whose grid is laid out differently where it is given cell
  /// by cell: another size, window, column widths, border or spacing, or given cell by
  /// cell now and not before, or the other way round. The stage cell itself is here
  /// when its own grid is. The cells inside such a grid are named in [looks] each for
  /// itself.
  final Set<String> grids;

  /// The IDs of the cells that the answer before had and this one has not: removed,
  /// outside the window now, or in a nested grid no longer given cell by cell.
  final Set<String> gone;

  /// Whether nothing is to be drawn again.
  bool get isEmpty =>
      !everything &&
      !trail &&
      looks.isEmpty &&
      summaries.isEmpty &&
      grids.isEmpty &&
      gone.isEmpty;

  /// Whether the cell with this ID is to be drawn again, for any of the reasons.
  bool touches(String id) =>
      everything ||
      looks.contains(id) ||
      summaries.contains(id) ||
      grids.contains(id);
}
