import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// The text of one cell while it is edited, as a part that can be exchanged: the
/// screen, its toolbar and the keeper talk to this, and to nothing of the text field
/// underneath (UI_SPEC section 2 keeps a native editor possible on iOS).
///
/// **Positions are UTF-16 units of [text]**, as Flutter and iOS both count; the core
/// counts in code points (`codePoints` and `units` in `session.dart` turn one into
/// the other).
///
/// **[text] is always what a cell may hold**, as far as that can be known here: a
/// line end is one U+000A whatever was typed or pasted (CR LF, CR), and every other
/// control character is a space. So what the document gives back after the text was
/// entered is the same text, and positions in one are positions in the other. (The
/// core still has the last word: read back what it kept.)
///
/// **Return does not break the line.** The keyboard's Return or "next" key calls
/// [onSubmit], which a screen makes "save, and on to the cell below"; a line break is
/// asked for by its own key of the toolbar, which calls `insert('\n')`.
///
/// It tells its listeners when the text, the selection or the focus changed.
abstract interface class CellEditor implements Listenable {
  /// The field itself, for the screen to place. `key` goes to the widget that holds
  /// the text.
  Widget build(BuildContext context, {Key? key});

  /// What is in the field now.
  String get text;

  /// What is selected: from `start` up to `end`, in UTF-16 units of [text], with
  /// `start <= end`, and equal for a cursor. Always a place in the text (with no
  /// selection at all, the end of it), and never inside a user-perceived character
  /// (a letter with its accents, a flag, an emoji made of several): a selection
  /// that would cut one is widened to hold it, since the core refuses a range that
  /// cuts one.
  ({int start, int end}) get selection;

  /// Whether an input method has text under way that is not final (the underlined
  /// text of a keyboard that suggests words, or of one that composes characters).
  bool get composing;

  /// Whether the field has the keyboard.
  bool get hasFocus;

  /// Puts `text` into the field, made what a cell may hold, with the cursor at
  /// `cursor` or at the end: to open on a cell, and to put back what the document
  /// kept.
  void set(String text, {int? cursor});

  /// Puts `text` in the place of the [selection] and the cursor after it: the line
  /// break of the toolbar is `insert('\n')`.
  void insert(String text);

  /// Makes what an input method has under way final, before the text is read to be
  /// entered.
  void finishComposing();

  /// Gives the field the keyboard.
  void focus();

  /// Takes the keyboard from the field.
  void unfocus();

  /// What the keyboard's own Return or "next" key does. The field keeps the
  /// keyboard and its text whatever this does.
  set onSubmit(VoidCallback? action);

  /// Lets go of what the editor holds. Nothing may be asked of it afterwards.
  void dispose();
}

/// Makes the editor of a screen: the switch between kinds of editor is which of
/// these a screen is given.
typedef CellEditorFactory = CellEditor Function();

/// CR LF and a CR on its own become one line break, as the core makes them.
final _lineEnds = RegExp(r'\r\n?');

/// Every other control character becomes a space, as the core makes it.
final _controls = RegExp(r'[\u0000-\u0009\u000b-\u001f]');

/// `text` as a cell may hold it.
String asCellText(String text) =>
    text.replaceAll(_lineEnds, '\n').replaceAll(_controls, ' ');

/// The [CellEditor] made of Flutter's own text field.
class FlutterCellEditor implements CellEditor {
  final _text = TextEditingController();
  final _focus = FocusNode();
  late final Listenable _both = Listenable.merge([_text, _focus]);

  @override
  VoidCallback? onSubmit;

  /// How many lines the field shows before it scrolls.
  static const lines = 5;

  /// What is typed and pasted is made what a cell may hold as it arrives, with the
  /// cursor kept in its place.
  static final _formatters = <TextInputFormatter>[
    FilteringTextInputFormatter(
      _lineEnds,
      allow: false,
      replacementString: '\n',
    ),
    FilteringTextInputFormatter(
      _controls,
      allow: false,
      replacementString: ' ',
    ),
  ];

  @override
  Widget build(BuildContext context, {Key? key}) => TextField(
    key: key,
    controller: _text,
    focusNode: _focus,
    minLines: 1,
    maxLines: lines,
    // Several lines are shown, and yet Return is an action and not a line break:
    // the keyboard for several lines insists on Return being one.
    keyboardType: TextInputType.text,
    textInputAction: TextInputAction.next,
    inputFormatters: _formatters,
    decoration: const InputDecoration(
      isDense: true,
      border: OutlineInputBorder(),
    ),
    // Saying what Return does here keeps Flutter from doing what it does otherwise
    // for "next", moving the focus away: the editor is in use until the screen
    // says otherwise.
    onEditingComplete: finishComposing,
    onSubmitted: (_) => onSubmit?.call(),
  );

  @override
  String get text => _text.text;

  @override
  ({int start, int end}) get selection {
    final text = _text.text;
    final selected = _text.selection;
    if (!selected.isValid) return (start: text.length, end: text.length);
    final start = selected.start.clamp(0, text.length);
    final end = selected.end.clamp(start, text.length);
    // Widened to whole user-perceived characters.
    final range = CharacterRange.at(text, start, end);
    final from = range.stringBeforeLength;
    return (start: from, end: from + range.current.length);
  }

  @override
  bool get composing => !_text.value.composing.isCollapsed;

  @override
  bool get hasFocus => _focus.hasFocus;

  @override
  void set(String text, {int? cursor}) {
    final clean = asCellText(text);
    _text.value = TextEditingValue(
      text: clean,
      selection: TextSelection.collapsed(
        offset: (cursor ?? clean.length).clamp(0, clean.length),
      ),
    );
  }

  @override
  void insert(String text) {
    final clean = asCellText(text);
    final at = selection;
    _text.value = TextEditingValue(
      text: _text.text.replaceRange(at.start, at.end, clean),
      selection: TextSelection.collapsed(offset: at.start + clean.length),
    );
  }

  @override
  void finishComposing() => _text.clearComposing();

  @override
  void focus() => _focus.requestFocus();

  @override
  void unfocus() => _focus.unfocus();

  @override
  void addListener(VoidCallback listener) => _both.addListener(listener);

  @override
  void removeListener(VoidCallback listener) => _both.removeListener(listener);

  @override
  void dispose() {
    _text.dispose();
    _focus.dispose();
  }
}
