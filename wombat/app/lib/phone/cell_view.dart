import 'package:flutter/material.dart';

import '../cell_editor.dart';
import '../session.dart';
import '../src/rust/api/wire.dart';
import '../stage_read.dart';
import 'budget.dart';
import 'variants.dart';
import 'words.dart';

/// A colour of the document as Flutter has it.
Color colorOf(Rgb color) => Color.fromARGB(255, color.r, color.g, color.b);

/// The direction a text is written in, by its first letter: right to left for the
/// Hebrew and Arabic scripts and their neighbours, left to right otherwise. Digits,
/// spaces and punctuation decide nothing.
TextDirection directionOf(String text) {
  for (final rune in text.runes) {
    if (rune < 0x41 || rune >= 0x2000 && rune < 0x2070) continue;
    final rightToLeft =
        rune >= 0x590 && rune < 0x900 ||
        rune >= 0xFB1D && rune < 0xFE00 ||
        rune >= 0xFE70 && rune < 0xFF00 ||
        rune >= 0x10800 && rune < 0x11000 ||
        rune >= 0x1E800 && rune < 0x1F000;
    return rightToLeft ? TextDirection.rtl : TextDirection.ltr;
  }
  return TextDirection.ltr;
}

/// What every cell of one drawing of the stage is drawn with: the read, the
/// settings, which cell is selected or edited, and where a tap goes.
class Scene {
  const Scene({
    required this.stage,
    required this.variants,
    required this.textScale,
    required this.fullCells,
    required this.selected,
    required this.editing,
    required this.editor,
    required this.held,
    required this.unfolded,
    required this.flash,
    required this.editedKey,
    required this.onText,
    required this.onChild,
    required this.onFold,
  });

  /// The read that is drawn.
  final StageRead stage;
  final Variants variants;

  /// The system's text size, 1 for the usual one.
  final double textScale;

  /// The most cells a grid may have to be drawn Full without being unfolded.
  final int fullCells;

  /// The ID of the cell that is selected, if one is.
  final String? selected;

  /// The ID of the cell whose text is in the editor, if one is: it is drawn from
  /// [editor], not from the read.
  final String? editing;
  final CellEditor editor;

  /// The edited cell as the document holds it meanwhile (after a style was put on
  /// it), if it does: its runs are drawn while the editor has the same text.
  final CellLook? held;

  /// The IDs of the cells whose grid the user unfolded in place.
  final Set<String> unfolded;

  /// The ID of the cell that flashes once: the one the screen came up out of.
  final String? flash;

  /// Goes to the edited cell, so that the screen can scroll it into view.
  final GlobalKey editedKey;

  /// A tap on a cell's text.
  final void Function(String id) onText;

  /// A tap on a cell's grid where that is one target (Preview, Shape, Badge).
  final void Function(String id) onChild;

  /// A tap on the bar that folds an unfolded grid again.
  final void Function(String id) onFold;
}

/// One cell: its text on top and, if it holds a grid, that grid below in the form
/// the depth budget gives it (UI_SPEC sections 3 and 4).
///
/// `name` is the cell's place as the keys of its parts have it: `x-y` for column x
/// and row y of the stage's grid, and `x-y/i-j` for a cell of the Full grid inside
/// that one. The text is `cell-<name>`; the grid as one target `child-<name>`, in
/// which `preview-<name>`, `shape-<name>` or `badge-<name>` says the form; a Full
/// grid is `full-<name>`.
class CellView extends StatelessWidget {
  const CellView({
    super.key,
    required this.scene,
    required this.look,
    required this.name,
    required this.width,
  });

  final Scene scene;
  final CellLook look;
  final String name;

  /// The width the cell is given, in logical pixels.
  final double width;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final summary = scene.stage.summary(look);
    final edited = scene.editing == look.id;
    final selected = scene.selected == look.id;
    final text = Padding(
      padding: const EdgeInsets.all(6),
      child: edited
          ? ListenableBuilder(
              listenable: scene.editor,
              builder: (context, _) {
                final typed = scene.editor.text;
                final held = scene.held;
                return CellText(
                  look: look,
                  typed: typed,
                  styled: held != null && held.text == typed ? held : null,
                );
              },
            )
          : CellText(look: look, maxLines: 8),
    );

    final mark = edited
        ? colors.primary
        : selected
        ? colors.tertiary
        : null;
    return DecoratedBox(
      // The whole cell is what is scrolled into view while it is edited.
      key: edited ? scene.editedKey : null,
      decoration: BoxDecoration(color: colorOf(look.cellColor)),
      child: DecoratedBox(
        position: DecorationPosition.foreground,
        decoration: BoxDecoration(
          border: mark == null
              ? Border(
                  right: BorderSide(color: colors.outlineVariant),
                  bottom: BorderSide(color: colors.outlineVariant),
                )
              : Border.all(color: mark, width: 2),
        ),
        child: _Flash(
          on: scene.flash == look.id,
          color: colors.tertiary,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: [
              InkWell(
                key: Key('cell-$name'),
                onTap: () => scene.onText(look.id),
                child: ConstrainedBox(
                  constraints: const BoxConstraints(minHeight: target),
                  child: Align(
                    alignment: AlignmentDirectional.topStart,
                    widthFactor: 1,
                    child: text,
                  ),
                ),
              ),
              if (summary != null) _child(context, summary),
            ],
          ),
        ),
      ),
    );
  }

  /// The grid of the cell, in its form.
  Widget _child(BuildContext context, GridSummary summary) {
    final colors = Theme.of(context).colorScheme;
    final inner = width - 12;
    final part = scene.stage.full(look);
    final unfolded = scene.unfolded.contains(look.id);
    final form = formOf(
      width: inner,
      summary: summary,
      given: part != null,
      unfolded: unfolded,
      folded: look.folded,
      fullCells: scene.fullCells,
      textScale: scene.textScale,
      variants: scene.variants,
    );
    final frame = BoxDecoration(
      border: Border.all(color: colorOf(summary.borderColor)),
      borderRadius: BorderRadius.circular(3),
    );
    if (form == DepthForm.full && part != null) {
      return Padding(
        padding: const EdgeInsets.fromLTRB(6, 0, 6, 6),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          mainAxisSize: MainAxisSize.min,
          children: [
            if (unfolded)
              InkWell(
                key: Key('fold-$name'),
                onTap: () => scene.onFold(look.id),
                child: Container(
                  constraints: const BoxConstraints(minHeight: target),
                  alignment: AlignmentDirectional.centerStart,
                  child: Row(
                    children: [
                      const Icon(Icons.expand_less, size: 18),
                      Flexible(
                        child: Text(
                          '${Words.fold}  ${summary.xs}×${summary.ys}',
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            DecoratedBox(
              key: Key('full-$name'),
              // Over the cells, which have backgrounds of their own.
              position: DecorationPosition.foreground,
              decoration: frame,
              child: _full(part, inner, unfolded),
            ),
          ],
        ),
      );
    }
    final font = TextStyle(
      fontSize: cellFont * scene.variants.previewScale,
      color: colorOf(look.textColor),
      height: 1.2,
    );
    return Semantics(
      button: true,
      label: Words.gridOf(summary.xs, summary.ys),
      child: InkWell(
        key: Key('child-$name'),
        onTap: () => scene.onChild(look.id),
        child: Container(
          constraints: const BoxConstraints(minHeight: target),
          margin: const EdgeInsets.fromLTRB(6, 0, 6, 6),
          padding: const EdgeInsets.all(3),
          decoration: frame,
          alignment: Alignment.center,
          child: switch (form) {
            DepthForm.preview => PreviewView(
              key: Key('preview-$name'),
              summary: summary,
              style: font,
              fade: colorOf(look.cellColor),
            ),
            DepthForm.shape => ShapeView(
              key: Key('shape-$name'),
              summary: summary,
              width: inner - 8,
            ),
            // Made smaller where the count does not fit beside the icon.
            _ => FittedBox(
              key: Key('badge-$name'),
              fit: BoxFit.scaleDown,
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.grid_on, size: 18, color: colors.onSurfaceVariant),
                  const SizedBox(width: 4),
                  Text('${summary.xs * summary.ys}'),
                ],
              ),
            ),
          },
        ),
      ),
    );
  }

  /// The grid cell by cell. An unfolded grid keeps columns that can be tapped and
  /// read, and scrolls sideways inside its cell when it has more of them than fit.
  Widget _full(GridPart part, double inner, bool unfolded) {
    final least = unfolded ? fullColumn(scene.textScale) : 0.0;
    final share = inner / part.cols;
    final column = share > least ? share : least;
    final rows = Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (var j = 0; j < part.rows; j++)
          IntrinsicHeight(
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                for (var i = 0; i < part.cols; i++)
                  SizedBox(
                    width: column,
                    child: CellView(
                      scene: scene,
                      look: scene.stage.cells[part.first + j * part.cols + i],
                      name: '$name/${part.x0 + i}-${part.y0 + j}',
                      width: column,
                    ),
                  ),
              ],
            ),
          ),
      ],
    );
    if (column * part.cols <= inner + 0.5) return rows;
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: SizedBox(width: column * part.cols, child: rows),
    );
  }
}

/// A tint that shows once and fades, when `on` becomes true.
class _Flash extends StatelessWidget {
  const _Flash({required this.on, required this.color, required this.child});

  final bool on;
  final Color color;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (!on) return child;
    return TweenAnimationBuilder<double>(
      key: const Key('flash'),
      tween: Tween(begin: 0.45, end: 0),
      duration: const Duration(milliseconds: 600),
      builder: (context, strength, child) => DecoratedBox(
        position: DecorationPosition.foreground,
        decoration: BoxDecoration(color: color.withValues(alpha: strength)),
        child: child,
      ),
      child: child,
    );
  }
}

/// The text of a cell as the document styles it: the base style, the ranges styled
/// otherwise, the links; a formula cell shows its result.
class CellText extends StatelessWidget {
  const CellText({
    super.key,
    required this.look,
    this.typed,
    this.styled,
    this.maxLines,
  });

  /// The cell as read.
  final CellLook look;

  /// The text of the editor, for the cell that is being edited: it is shown in
  /// place of what was read.
  final String? typed;

  /// With [typed]: the cell as the document holds that very text, whose ranges are
  /// drawn. Without it typed text is drawn in the base style alone.
  final CellLook? styled;

  /// The most lines shown; the rest is cut with an ellipsis.
  final int? maxLines;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final typed = this.typed;
    final result = typed == null ? look.result : null;
    final text = typed ?? result ?? look.text;
    final cut =
        typed == null && (result != null ? look.resultCut : look.textCut);
    // The ranges are places in the cell's own text, not in what a formula shows.
    final from = typed != null ? styled : (result == null ? look : null);
    final base = _style(
      (from ?? look).style,
      TextStyle(
        fontSize: (cellFont - look.relSize).clamp(8, 40).toDouble(),
        color: look.failed && typed == null
            ? colors.error
            : colorOf(look.textColor),
        fontStyle: result == null ? null : FontStyle.italic,
      ),
    );
    return Text.rich(
      TextSpan(
        children: [
          ...(from == null
              ? [TextSpan(text: text)]
              : spansOf(text, from, colors.primary)),
          if (cut) const TextSpan(text: '…'),
        ],
      ),
      style: base,
      maxLines: maxLines,
      overflow: maxLines == null ? null : TextOverflow.ellipsis,
      textDirection: directionOf(text),
      textAlign: switch (look.textAlign) {
        HorizAlign.auto => TextAlign.start,
        HorizAlign.left => TextAlign.left,
        HorizAlign.center => TextAlign.center,
        HorizAlign.right => TextAlign.right,
      },
    );
  }
}

/// `onto` with the bits of `style` switched on.
TextStyle _style(Style style, TextStyle onto) => onto.copyWith(
  fontWeight: style.bold ? FontWeight.bold : null,
  fontStyle: style.italic ? FontStyle.italic : null,
  fontFamily: style.fixed ? 'monospace' : null,
  decoration: style.underline || style.strike
      ? TextDecoration.combine([
          if (style.underline) TextDecoration.underline,
          if (style.strike) TextDecoration.lineThrough,
        ])
      : null,
);

/// `text` in pieces, one for every stretch that the runs and links of `look` style
/// alike. The document counts their places in code points; a place that is not in
/// the text (the two do not belong together) gives the text plain, never an error.
List<InlineSpan> spansOf(String text, CellLook look, Color linkColor) {
  final runs = look.runs;
  final links = look.links;
  if (runs.isEmpty && links.isEmpty) return [TextSpan(text: text)];
  try {
    final edges = <int>{0, text.runes.length};
    for (final run in runs) {
      edges
        ..add(run.start)
        ..add(run.start + run.len);
    }
    for (final link in links) {
      edges
        ..add(link.start)
        ..add(link.start + link.len);
    }
    final sorted = edges.toList()..sort();
    final spans = <InlineSpan>[];
    for (var i = 0; i + 1 < sorted.length; i++) {
      final start = sorted[i];
      final end = sorted[i + 1];
      var style = const TextStyle();
      for (final run in runs) {
        if (run.start <= start && end <= run.start + run.len) {
          // A run says the whole style of its range, not what it adds.
          style = _style(
            run.style,
            TextStyle(
              fontWeight: FontWeight.normal,
              fontStyle: FontStyle.normal,
              decoration: TextDecoration.none,
              color: run.color == null ? null : colorOf(run.color!),
            ),
          );
        }
      }
      for (final link in links) {
        if (link.start <= start && end <= link.start + link.len) {
          style = style.copyWith(
            color: linkColor,
            decoration: TextDecoration.underline,
          );
        }
      }
      spans.add(
        TextSpan(
          text: text.substring(units(text, start), units(text, end)),
          style: style,
        ),
      );
    }
    return spans;
  } on RangeError {
    return [TextSpan(text: text)];
  }
}

/// A grid as a Preview: what its first cells show, smaller, and its size; faded at
/// the bottom when the grid has more than is shown.
class PreviewView extends StatelessWidget {
  const PreviewView({
    super.key,
    required this.summary,
    required this.style,
    required this.fade,
  });

  final GridSummary summary;
  final TextStyle style;

  /// The colour the texts fade into: the cell's own.
  final Color fade;

  @override
  Widget build(BuildContext context) {
    final preview = summary.preview;
    final cols = summary.previewCols;
    final rows = cols == 0 ? 0 : preview.length ~/ cols;
    final cutOff = summary.xs > cols || summary.ys > rows;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        DecoratedBox(
          position: DecorationPosition.foreground,
          decoration: BoxDecoration(
            gradient: cutOff
                ? LinearGradient(
                    begin: Alignment.topCenter,
                    end: Alignment.bottomCenter,
                    colors: [
                      fade.withValues(alpha: 0),
                      fade.withValues(alpha: 0.8),
                    ],
                    stops: const [0.6, 1],
                  )
                : null,
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: [
              for (var j = 0; j < rows; j++)
                Row(
                  children: [
                    for (var i = 0; i < cols; i++)
                      Expanded(
                        child: Padding(
                          padding: const EdgeInsetsDirectional.only(end: 3),
                          child: Text(
                            '${preview[j * cols + i].text}'
                            '${preview[j * cols + i].cut ? '…' : ''}',
                            style: style,
                            maxLines: 1,
                            softWrap: false,
                            overflow: TextOverflow.fade,
                            textDirection: directionOf(
                              preview[j * cols + i].text,
                            ),
                          ),
                        ),
                      ),
                  ],
                ),
            ],
          ),
        ),
        Text(
          '${summary.xs}×${summary.ys}',
          style: style.copyWith(fontWeight: FontWeight.bold),
          textAlign: TextAlign.end,
          maxLines: 1,
          softWrap: false,
          overflow: TextOverflow.fade,
        ),
      ],
    );
  }
}

/// A grid as a Shape: its rows, columns and colours in miniature, and in each cell
/// a bar as long as the cell has text, or a frame where it holds a grid.
class ShapeView extends StatelessWidget {
  const ShapeView({super.key, required this.summary, required this.width});

  final GridSummary summary;

  /// The width to draw in.
  final double width;

  @override
  Widget build(BuildContext context) {
    final cols = summary.shapeCols;
    final rows = summary.shapeRows;
    final row = (width / cols * 0.5).clamp(4.0, 12.0);
    final height = (rows * row).clamp(target - 8, 160.0);
    return CustomPaint(
      size: Size(width, height),
      painter: _ShapePainter(summary),
    );
  }
}

class _ShapePainter extends CustomPainter {
  _ShapePainter(this.summary);

  final GridSummary summary;

  @override
  void paint(Canvas canvas, Size size) {
    final cols = summary.shapeCols;
    final rows = summary.shapeRows;
    final shape = summary.shape;
    if (cols == 0 || rows == 0 || shape.length < cols * rows * 4) return;
    final w = size.width / cols;
    final h = size.height / rows;
    final fill = Paint();
    final line = Paint()
      ..style = PaintingStyle.stroke
      ..color = const Color(0x66000000);
    final bar = Paint()..color = const Color(0x59000000);
    final edge = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 0.5
      ..color = const Color(0x33000000);
    for (var j = 0; j < rows; j++) {
      for (var i = 0; i < cols; i++) {
        final at = (j * cols + i) * 4;
        final box = Rect.fromLTWH(i * w, j * h, w, h);
        fill.color = Color.fromARGB(
          255,
          shape[at],
          shape[at + 1],
          shape[at + 2],
        );
        canvas.drawRect(box, fill);
        canvas.drawRect(box, edge);
        final holds = shape[at + 3];
        if (holds == 255) {
          canvas.drawRect(box.deflate(h * 0.25), line);
        } else if (holds > 0) {
          final length = (holds / 24).clamp(0.2, 0.85) * w;
          canvas.drawRect(
            Rect.fromLTWH(i * w + w * 0.08, j * h + h * 0.3, length, h * 0.4),
            bar,
          );
        }
      }
    }
  }

  @override
  bool shouldRepaint(_ShapePainter old) => !identical(old.summary, summary);
}
