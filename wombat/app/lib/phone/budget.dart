import 'dart:typed_data';

import '../src/rust/api/wire.dart';
import 'variants.dart';

/// The smallest side of anything that can be tapped, in logical pixels (UI_SPEC
/// section 3: 48 dp on Android, which is more than the 44 pt of iOS).
const double target = 48;

/// The size of a cell's text before the system's text size is applied.
const double cellFont = 16;

/// The width of the bar after the last column, and the height of the one after the
/// last row.
const double plusBar = target;

/// The form a nested grid is drawn in (UI_SPEC section 3), the richest first.
enum DepthForm {
  /// Cell by cell, each cell a target of its own.
  full,

  /// The first cells' texts, smaller, and the size.
  preview,

  /// A miniature of rows, columns and colours, without text.
  shape,

  /// A grid icon and the number of cells.
  badge,
}

/// The form in which the grid behind `summary` is drawn in a space `width` wide: the
/// richest one that is readable and can be tapped there.
///
/// - `given` says that the stage read has the grid cell by cell, without which there
///   is nothing to draw a Full form from.
/// - `unfolded`: the user unfolded it in place; it is then Full whatever its size, if
///   it is given.
/// - `folded`: the grid is folded on this device, so only its Badge shows.
/// - `fullCells`: the most cells a grid may have to be Full by itself.
/// - `textScale`: the system's text size, 1 for the usual one. Larger text needs
///   wider columns, so a grid steps down sooner.
DepthForm formOf({
  required double width,
  required GridSummary summary,
  required bool given,
  required bool unfolded,
  required bool folded,
  required int fullCells,
  required double textScale,
  required Variants variants,
}) {
  if (folded) return DepthForm.badge;
  if (given && unfolded) return DepthForm.full;
  if (given &&
      summary.xs * summary.ys <= fullCells &&
      width >= summary.xs * fullColumn(textScale)) {
    return DepthForm.full;
  }
  // A Preview needs room for a few characters of each of its columns.
  final previewFont = cellFont * variants.previewScale;
  final previewWidth = summary.previewCols * 2.5 * previewFont * textScale;
  if (summary.preview.isNotEmpty &&
      previewFont >= variants.minText &&
      width >= target &&
      width >= previewWidth) {
    return DepthForm.preview;
  }
  if (summary.shapeCols > 0 &&
      width >= target &&
      width >= 3 * summary.shapeCols) {
    return DepthForm.shape;
  }
  return DepthForm.badge;
}

/// The least width of a column of a Full grid: a target, and room for three
/// characters' worth of text at the system's size.
double fullColumn(double textScale) {
  final text = 3 * cellFont * textScale;
  return text > target ? text : target;
}

/// The widths of the columns of the stage's grid, in logical pixels, for a space
/// `viewport` wide: `units` are the document's own widths (TreeSheets' unit, 80 the
/// usual one), which keep their proportions. A grid of few columns fills the width;
/// of many, a phone shows three to four, and the rest scrolls.
List<double> columnWidths(
  Uint32List units, {
  required double viewport,
  required double textScale,
}) {
  if (units.isEmpty) return const [];
  final room = viewport - plusBar;
  // Larger text gets wider columns, up to half as wide again.
  final least = 110 * textScale.clamp(1.0, 1.5);
  final share = room / units.length;
  final usual = share > least ? share : least;
  final most = room > least ? room : least;
  return [
    for (final unit in units) (usual * unit / 80).clamp(target * 1.5, most),
  ];
}
