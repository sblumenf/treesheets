import 'package:flutter/material.dart';

import '../cell_editor.dart';
import 'budget.dart';
import 'words.dart';

/// The editor docked above the keyboard, with its toolbar (UI_SPEC section 7): the
/// field with the cross and the tick beside it, and under it Aa (which shows the
/// row of styles), the line break, →, +Row, +Col and Open ›. Return is the
/// keyboard's own key.
///
/// The toolbar is the screen's and not the editor's, so that it is the same over
/// any kind of editor (decision A180). It never takes the keyboard from the field.
///
/// Keys: `editor` the field, `save` the tick, `cancel` the cross, `tool-styles`,
/// `tool-break`, `tool-right`, `tool-row`, `tool-col`, `tool-open`, and in the row
/// of styles `style-bold`, `style-italic`, `style-link`, `style-tag`.
class EditorBar extends StatelessWidget {
  const EditorBar({
    super.key,
    required this.editor,
    required this.stylesShown,
    required this.onStyles,
    required this.onBold,
    required this.onItalic,
    required this.onLink,
    required this.onTag,
    required this.onBreak,
    required this.onRight,
    required this.onRow,
    required this.onColumn,
    required this.onOpen,
    required this.onTick,
    required this.onCross,
  });

  final CellEditor editor;

  /// Whether the row of styles is shown.
  final bool stylesShown;
  final VoidCallback onStyles;
  final VoidCallback onBold;
  final VoidCallback onItalic;
  final VoidCallback onLink;
  final VoidCallback onTag;
  final VoidCallback onBreak;
  final VoidCallback onRight;
  final VoidCallback onRow;
  final VoidCallback onColumn;

  /// Null when the edited cell has no grid to go into.
  final VoidCallback? onOpen;
  final VoidCallback onTick;
  final VoidCallback onCross;

  static final _least = TextButton.styleFrom(
    minimumSize: const Size(target, target),
  );

  Widget _word(String key, String label, VoidCallback? action, [String? tip]) =>
      Tooltip(
        message: tip ?? label,
        child: TextButton(
          key: Key(key),
          style: _least,
          onPressed: action,
          child: Text(label),
        ),
      );

  Widget _icon(String key, IconData icon, String tip, VoidCallback action) =>
      IconButton(
        key: Key(key),
        tooltip: tip,
        constraints: const BoxConstraints(minWidth: target, minHeight: target),
        icon: Icon(icon),
        onPressed: action,
      );

  Widget _strip(List<Widget> children) => SingleChildScrollView(
    scrollDirection: Axis.horizontal,
    child: Row(children: children),
  );

  @override
  Widget build(BuildContext context) => Material(
    elevation: 8,
    // A tap on the toolbar is not a tap outside the field: on a desktop that
    // would take the keyboard from it.
    child: TextFieldTapRegion(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(8, 4, 0, 0),
            child: Row(
              children: [
                Expanded(
                  child: editor.build(context, key: const Key('editor')),
                ),
                ExcludeFocus(
                  child: _icon('cancel', Icons.close, Words.cross, onCross),
                ),
                ExcludeFocus(
                  child: _icon('save', Icons.check, Words.tick, onTick),
                ),
              ],
            ),
          ),
          if (stylesShown)
            ExcludeFocus(
              child: _strip([
                _word('style-bold', Words.bold, onBold),
                _word('style-italic', Words.italic, onItalic),
                _word('style-link', Words.link, onLink),
                _word('style-tag', Words.tag, onTag),
              ]),
            ),
          ExcludeFocus(
            child: _strip([
              _word('tool-styles', Words.styles, onStyles, Words.stylesTip),
              _icon(
                'tool-break',
                Icons.keyboard_return,
                Words.lineBreak,
                onBreak,
              ),
              _icon('tool-right', Icons.arrow_forward, Words.right, onRight),
              _word('tool-row', Words.rowBelow, onRow),
              _word('tool-col', Words.columnRight, onColumn),
              _word('tool-open', Words.open, onOpen),
            ]),
          ),
        ],
      ),
    ),
  );
}
