import '../keeper.dart';

/// Every text that the phone screen and the start screen show, in one place, so
/// that they can be translated later. (What the core says when it refuses
/// something is English and comes through `describeError`.)
abstract final class Words {
  /// The title of a level whose cell has no text, and the name of the app.
  static const app = 'Wombat';

  /// A level of the trail whose cell has no text.
  static const noText = '(empty)';

  static const undo = 'Undo';
  static const redo = 'Redo';
  static const nothingToUndo = 'Nothing to undo';
  static const nothingToRedo = 'Nothing to redo';
  static const menu = 'More';
  static const up = 'Up one level';

  /// The bottom bar.
  static const add = '+ Add';
  static const addRowBelow = 'Row below';
  static const addColumnRight = 'Column to the right';
  static const edit = 'Edit';
  static const open = 'Open ›';

  /// The "+" bars after the last row and the last column.
  static const addRow = 'Add a row';
  static const addColumn = 'Add a column';

  /// A document whose root has no grid yet.
  static const emptyDocument = 'This document is empty.';
  static const firstCell = 'Add the first cell';

  /// The keyboard toolbar.
  static const styles = 'Aa';
  static const stylesTip = 'Bold, italic, link, tag';
  static const lineBreak = 'Line break';
  static const right = 'Save, and on to the cell on the right';
  static const rowBelow = '+Row';
  static const columnRight = '+Col';
  static const tick = 'Close the editor';
  static const cross = 'Not this: leave the cell as it was';
  static const bold = 'Bold';
  static const italic = 'Italic';
  static const link = 'Link';
  static const tag = 'Tag';

  /// The dialog that asks where a link goes.
  static const linkTitle = 'Where does the link go?';
  static const linkHint = 'An address or a file; nothing removes the link';
  static const linkNoText = 'There is no text to make a link of.';
  static const ok = 'OK';
  static const cancel = 'Cancel';

  /// An unfolded grid is folded again by this.
  static const fold = 'Fold';

  static const typedOn =
      'Not done: more was typed in the meantime. It is still in the editor.';

  /// How the document stands with its file, in a word or two.
  static String state(KeeperState state) => switch (state) {
    KeeperState.saved => 'Saved',
    KeeperState.unsaved => 'Not saved yet',
    KeeperState.saving => 'Saving…',
    KeeperState.failed => 'Not saved',
    KeeperState.changedOnDisk => 'File changed',
    KeeperState.ended => 'Closed',
  };

  static const changedOnDisk = 'Not saved: another program changed the file.';
  static const keepMine = 'Keep mine';
  static const openTheirs = 'Open theirs';
  static String notSaved(String why) => 'Not saved. $why';
  static const tryAgain = 'Try again';
  static String old(String why) => 'What is shown may be old. $why';
  static String viewNotKept(String why) =>
      'Where the screen was could not be written down. $why';
  static String notRead(String why) => 'The document could not be read. $why';

  /// After "Open theirs" did not open the other program's file. `theirs` says
  /// that the file itself could not be read; `fault` that reading it ran into a
  /// fault of the program.
  static String theirsNotOpened({
    required bool theirs,
    required bool fault,
    required String why,
  }) =>
      '${theirs ? 'The file as the other program left it cannot be opened' : 'The other program\'s file was not opened'}, '
      'so this document is still open, and not saved. '
      '${fault ? 'Reading it ran into a fault.' : why}';

  /// The sheet of the prototype's switches.
  static const settings = 'Prototype settings';
  static const close = 'Close';
  static const tapTextSelects = 'Tap text: select first, tap again to edit';
  static const tapTextSelectsOff = 'Off: a tap on a cell\'s text edits it';
  static const tapChildUnfolds = 'Tap child grid: unfold in place';
  static const tapChildUnfoldsOff = 'Off: a tap on a child grid dives into it';
  static const pin = 'Keep the first row and column in view';
  static String minText(double points) =>
      'Smallest text: ${points.toStringAsFixed(0)} pt';
  static String previewScale(double scale) =>
      'Preview text: ${(scale * 100).toStringAsFixed(0)}% of full size';
  static const less = 'Less';
  static const more = 'More';
  static String logCount(int entries) => 'Prototype log: $entries entries';
  static const exportLog = 'Export the log as a file';
  static const copyLog = 'Copy the log';
  static String logExported(String path) => 'The log is in $path';
  static String logNotExported(String why) =>
      'The log could not be written. $why';
  static const logCopied = 'The log is on the clipboard.';

  /// The start screen.
  static String notOpened(String why) =>
      'The document could not be opened. $why';
  static String itsFile(String path) => 'Its file is $path';
  static const putAside = 'Put that file aside and start with the example';
  static const openExample = 'Open the example instead';
  static const bridgeCheck = 'Bridge check';
  static const documents = 'Test documents';
  static const example = 'Example';
  static const forms = 'Depth forms';
  static const sixLevels = 'Six levels (12×20)';
  static const large = 'Large (100,000 cells)';
  static const empty = 'Empty';

  /// What a screen reader says of a child grid.
  static String gridOf(int columns, int rows) =>
      'contains a $columns by $rows grid';
}
