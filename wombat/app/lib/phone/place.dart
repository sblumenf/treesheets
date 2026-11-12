import '../session.dart';
import '../src/rust/api/wire.dart';
import '../stage_read.dart';

/// Where a cell is in a stage read: the grid that is given cell by cell and holds
/// it, and its column and row there, counted in the whole grid from 0.
typedef Place = ({GridPart part, int x, int y});

/// Finds the cell with this ID among the grids that `stage` gives cell by cell: the
/// stage's own and the Full ones nested in it. Null for a cell that the answer does
/// not have.
Place? placeOf(StageRead stage, String id) {
  for (final part in stage.grids) {
    final count = part.cols * part.rows;
    for (var i = 0; i < count; i++) {
      if (stage.cells[part.first + i].id == id) {
        return (
          part: part,
          x: part.x0 + i % part.cols,
          y: part.y0 + i ~/ part.cols,
        );
      }
    }
  }
  return null;
}

/// The grid of `owner` as `stage` gives it cell by cell, if it does.
GridPart? partOf(StageRead stage, String owner) {
  for (final part in stage.grids) {
    if (part.owner == owner) return part;
  }
  return null;
}

/// No bit of a style.
const plain = Style(
  bold: false,
  italic: false,
  fixed: false,
  underline: false,
  strike: false,
);

/// Bold and nothing else.
const bold = Style(
  bold: true,
  italic: false,
  fixed: false,
  underline: false,
  strike: false,
);

/// Italic and nothing else.
const italic = Style(
  bold: false,
  italic: true,
  fixed: false,
  underline: false,
  strike: false,
);

/// [phoneAsk], with grids of up to `cells` cells given cell by cell: what a grid
/// that was unfolded in place needs.
StageAsk askFull(int cells) => StageAsk(
  x0: phoneAsk.x0,
  y0: phoneAsk.y0,
  cols: phoneAsk.cols,
  rows: phoneAsk.rows,
  fullDepth: phoneAsk.fullDepth,
  fullCells: cells,
  previewCols: phoneAsk.previewCols,
  previewRows: phoneAsk.previewRows,
  shapeCols: phoneAsk.shapeCols,
  shapeRows: phoneAsk.shapeRows,
  textCut: phoneAsk.textCut,
  previewCut: phoneAsk.previewCut,
);

/// Whether every code point of `look`'s text from `start` up to `end` has the bit
/// that `bit` reads: what decides whether Bold switches it off or on. A range
/// without text is styled when the cell's base style is.
bool allStyled(CellLook look, int start, int end, bool Function(Style) bit) {
  if (start >= end) return bit(look.style);
  var at = start;
  // The runs are sorted and do not overlap; between them the base style holds.
  for (final run in look.runs) {
    final runEnd = run.start + run.len;
    if (runEnd <= at) continue;
    if (run.start >= end) break;
    if (run.start > at && !bit(look.style)) return false;
    if (!bit(run.style)) return false;
    at = runEnd;
  }
  return at >= end || bit(look.style);
}

/// The range of `held`'s text, in code points, that the selection of an editor
/// covers: `selection` is in UTF-16 units of the editor's text, which is the held
/// text unless the core kept another; a position past its end is its end.
({int start, int end}) rangeIn(
  CellLook held,
  ({int start, int end}) selection,
) {
  int at(int unit) => codePoints(held.text, unit.clamp(0, held.text.length));
  return (start: at(selection.start), end: at(selection.end));
}
