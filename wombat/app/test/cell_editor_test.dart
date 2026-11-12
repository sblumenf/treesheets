// The editor of a cell as the screen and its toolbar see it (`CellEditor`), in its one
// kind so far, made of Flutter's text field: what a cell may hold, a selection that
// is always a place in the text, Return that is no line break, and the line break
// that the toolbar asks for.
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/cell_editor.dart';

void main() {
  const field = Key('field');

  /// A test with an editor on the screen and nothing else. It is given through the
  /// factory, as a screen is given its editor.
  void editorTest(
    String name,
    Future<void> Function(WidgetTester tester, CellEditor editor) body,
  ) {
    testWidgets(name, (tester) async {
      const CellEditorFactory make = FlutterCellEditor.new;
      final editor = make();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Builder(
              builder: (context) => editor.build(context, key: field),
            ),
          ),
        ),
      );
      await body(tester, editor);
      await tester.pumpWidget(const SizedBox());
      editor.dispose();
    });
  }

  /// What the keyboard sends: the whole text and where the cursor is.
  Future<void> typed(WidgetTester tester, TextEditingValue value) async {
    tester.testTextInput.updateEditingValue(value);
    await tester.pump();
  }

  TextEditingValue at(String text, int start, [int? end]) => TextEditingValue(
    text: text,
    selection: TextSelection(baseOffset: start, extentOffset: end ?? start),
  );

  test('a text as a cell may hold it', () {
    expect(asCellText('plain'), 'plain');
    expect(asCellText('a\r\nb\rc\nd'), 'a\nb\nc\nd');
    expect(asCellText('a\r\r\nb'), 'a\n\nb');
    expect(asCellText('a\tb\u0000c\u001fd\u000be'), 'a b c d e');
    expect(asCellText('  \u007f'), '  \u007f');
    expect(asCellText(''), '');
  });

  editorTest('opens on a text, with the cursor at its end or where it is put', (
    tester,
    editor,
  ) async {
    expect((editor.text, editor.selection), ('', (start: 0, end: 0)));
    editor.set('what to buy');
    expect(editor.text, 'what to buy');
    expect(editor.selection, (start: 11, end: 11));
    editor.set('what to buy', cursor: 4);
    expect(editor.selection, (start: 4, end: 4));
    // A place outside the text is the nearest one in it.
    editor.set('abc', cursor: 99);
    expect(editor.selection, (start: 3, end: 3));
    editor.set('abc', cursor: -1);
    expect(editor.selection, (start: 0, end: 0));
    // What is put in is made what a cell may hold.
    editor.set('one\r\ntwo\tthree');
    expect(editor.text, 'one\ntwo three');
    expect(editor.selection, (start: 13, end: 13));
    await tester.pump();
    expect(find.text('one\ntwo three'), findsOneWidget);
  });

  editorTest('what is typed or pasted is made what a cell may hold as it '
      'arrives, and the cursor stays in its place', (tester, editor) async {
    editor.focus();
    await tester.pump();
    // Pasted with the line ends of another system, the cursor after the "b".
    await typed(tester, at('a\r\nb\tc\rd', 4));
    expect(editor.text, 'a\nb c\nd');
    expect(editor.selection, (start: 3, end: 3));
    await typed(tester, at('a\r\nb', 4));
    expect((editor.text, editor.selection), ('a\nb', (start: 3, end: 3)));
  });

  editorTest('Return is no line break: it is what the screen says, and the '
      'field keeps the keyboard and its text', (tester, editor) async {
    var submitted = 0;
    editor.focus();
    await tester.pump();
    await typed(tester, at('typed', 5));
    // With nothing said, Return does nothing at all.
    await tester.testTextInput.receiveAction(TextInputAction.next);
    await tester.pump();
    expect((editor.text, editor.hasFocus), ('typed', true));

    editor.onSubmit = () => submitted++;
    await tester.testTextInput.receiveAction(TextInputAction.next);
    await tester.pump();
    expect(submitted, 1);
    expect((editor.text, editor.hasFocus), ('typed', true));
    // A keyboard that sends "new line" all the same gets nothing for it: Flutter
    // takes it that a field of several lines has its line break already. No break,
    // then, and no action either.
    await tester.testTextInput.receiveAction(TextInputAction.newline);
    await tester.pump();
    expect(submitted, 1);
    expect((editor.text, editor.hasFocus), ('typed', true));

    // What the field tells the keyboard: one key that acts, not one that breaks
    // the line; and yet it shows several lines.
    final shown = tester.widget<TextField>(find.byKey(field));
    expect(shown.textInputAction, TextInputAction.next);
    expect(shown.keyboardType, TextInputType.text);
    expect((shown.minLines, shown.maxLines), (1, FlutterCellEditor.lines));
    expect(
      tester.testTextInput.setClientArgs!['inputAction'],
      'TextInputAction.next',
    );
    expect(
      (tester.testTextInput.setClientArgs!['inputType'] as Map)['name'],
      'TextInputType.text',
    );
  });

  editorTest('the line break of the toolbar goes where the cursor is, and in '
      'the place of what is selected', (tester, editor) async {
    var told = 0;
    editor.addListener(() => told++);
    editor.set('onetwo', cursor: 3);
    final before = told;
    editor.insert('\n');
    expect(editor.text, 'one\ntwo');
    expect(editor.selection, (start: 4, end: 4));
    expect(told, greaterThan(before));
    await tester.pump();
    final one = tester.getSize(find.byKey(field)).height;

    // In the place of a selection, whichever way it was made.
    editor.focus();
    await tester.pump();
    await typed(tester, at('one\ntwo', 7, 4));
    expect(editor.selection, (start: 4, end: 7));
    editor.insert('2\r\n3');
    expect(editor.text, 'one\n2\n3');
    expect(editor.selection, (start: 7, end: 7));
    // The field grows with its lines, up to as many as it shows.
    await tester.pump();
    final two = tester.getSize(find.byKey(field)).height;
    expect(two, greaterThan(one));
    editor.set('1\n2\n3\n4\n5');
    await tester.pump();
    final five = tester.getSize(find.byKey(field)).height;
    editor.set('1\n2\n3\n4\n5\n6\n7');
    await tester.pump();
    expect(tester.getSize(find.byKey(field)).height, five);
    expect(five, greaterThan(two));

    // A listener that was taken away is not told.
    void other() => fail('told after it was taken away');
    editor.addListener(other);
    editor.removeListener(other);
    editor.insert('x');
  });

  editorTest('the selection is always a place in the text, and never inside '
      'a character', (tester, editor) async {
    editor.focus();
    await tester.pump();
    // No selection at all, as a field has that never had the keyboard: the end.
    await typed(
      tester,
      const TextEditingValue(
        text: 'abc',
        selection: TextSelection.collapsed(offset: -1),
      ),
    );
    expect(editor.selection, (start: 3, end: 3));
    editor.insert('d');
    expect(editor.text, 'abcd');

    // a, an emoji of two units, e with an accent of its own, a flag of four units.
    const text = 'a\u{1F600}e\u{301}\u{1F1E9}\u{1F1EA}';
    expect(text.length, 9);
    await typed(tester, at(text, 1, 3));
    expect(editor.selection, (start: 1, end: 3));
    // Between the letter and its accent: the accent is held too.
    await typed(tester, at(text, 1, 4));
    expect(editor.selection, (start: 1, end: 5));
    await typed(tester, at(text, 4));
    expect(editor.selection, (start: 3, end: 5));
    // A cursor inside the flag holds the flag.
    await typed(tester, at(text, 7));
    expect(editor.selection, (start: 5, end: 9));
    // What is put there takes the place of the whole of it.
    editor.insert('!');
    expect(editor.text, 'a\u{1F600}e\u{301}!');
    expect(editor.selection, (start: 6, end: 6));
  });

  editorTest('text that an input method has under way is said, and made '
      'final when asked', (tester, editor) async {
    editor.focus();
    await tester.pump();
    expect(editor.composing, isFalse);
    await typed(
      tester,
      const TextEditingValue(
        text: 'half a wor',
        selection: TextSelection.collapsed(offset: 10),
        composing: TextRange(start: 7, end: 10),
      ),
    );
    expect(editor.composing, isTrue);
    editor.finishComposing();
    expect(editor.composing, isFalse);
    expect(editor.text, 'half a wor');
    expect(editor.selection, (start: 10, end: 10));
    // An empty range under way is nothing under way.
    await typed(
      tester,
      const TextEditingValue(
        text: 'half',
        selection: TextSelection.collapsed(offset: 4),
        composing: TextRange.collapsed(4),
      ),
    );
    expect(editor.composing, isFalse);
  });

  editorTest('the keyboard is given and taken, and listeners hear of it', (
    tester,
    editor,
  ) async {
    var told = 0;
    editor.addListener(() => told++);
    expect(editor.hasFocus, isFalse);
    editor.focus();
    await tester.pump();
    expect(editor.hasFocus, isTrue);
    expect(told, greaterThan(0));
    expect(tester.testTextInput.isVisible, isTrue);
    final before = told;
    editor.unfocus();
    await tester.pump();
    expect(editor.hasFocus, isFalse);
    expect(told, greaterThan(before));
  });
}
