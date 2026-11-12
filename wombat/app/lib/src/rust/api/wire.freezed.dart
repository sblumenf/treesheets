// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint, type=warning, deprecated_member_use, deprecated_member_use_from_same_package
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'wire.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$Command {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'Command()';
}


}

/// @nodoc
class $CommandCopyWith<$Res>  {
$CommandCopyWith(Command _, $Res Function(Command) __);
}


/// Adds pattern-matching-related methods to [Command].
extension CommandPatterns on Command {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( Command_InsertText value)?  insertText,TResult Function( Command_DeleteText value)?  deleteText,TResult Function( Command_ReplaceText value)?  replaceText,TResult Function( Command_SetText value)?  setText,TResult Function( Command_SetRangeStyle value)?  setRangeStyle,TResult Function( Command_SetRangeColor value)?  setRangeColor,TResult Function( Command_SetLink value)?  setLink,TResult Function( Command_SetProps value)?  setProps,TResult Function( Command_SetStyle value)?  setStyle,TResult Function( Command_SetColumnWidth value)?  setColumnWidth,TResult Function( Command_CreateGrid value)?  createGrid,TResult Function( Command_RemoveGrid value)?  removeGrid,TResult Function( Command_InsertRows value)?  insertRows,TResult Function( Command_InsertColumns value)?  insertColumns,TResult Function( Command_DeleteRows value)?  deleteRows,TResult Function( Command_DeleteColumns value)?  deleteColumns,TResult Function( Command_ClearCells value)?  clearCells,TResult Function( Command_DeleteCells value)?  deleteCells,TResult Function( Command_MoveRows value)?  moveRows,TResult Function( Command_MoveColumns value)?  moveColumns,TResult Function( Command_SwapCells value)?  swapCells,TResult Function( Command_MoveCell value)?  moveCell,TResult Function( Command_CopyCell value)?  copyCell,TResult Function( Command_SetTag value)?  setTag,TResult Function( Command_RemoveTag value)?  removeTag,TResult Function( Command_SortRows value)?  sortRows,TResult Function( Command_Transpose value)?  transpose,TResult Function( Command_HierarchySwap value)?  hierarchySwap,TResult Function( Command_Hierarchify value)?  hierarchify,TResult Function( Command_Flatten value)?  flatten,TResult Function( Command_Wrap value)?  wrap,TResult Function( Command_Collapse value)?  collapse,TResult Function( Command_SplitCell value)?  splitCell,TResult Function( Command_TagCells value)?  tagCells,TResult Function( Command_UntagCells value)?  untagCells,TResult Function( Command_AdjustRelSize value)?  adjustRelSize,TResult Function( Command_AdjustColumnWidths value)?  adjustColumnWidths,TResult Function( Command_Run value)?  run,TResult Function( Command_EnterText value)?  enterText,TResult Function( Command_ToggleCheck value)?  toggleCheck,TResult Function( Command_PasteCells value)?  pasteCells,TResult Function( Command_AdoptCells value)?  adoptCells,TResult Function( Command_ReleaseCell value)?  releaseCell,TResult Function( Command_Import value)?  import_,TResult Function( Command_RetargetLinks value)?  retargetLinks,required TResult orElse(),}){
final _that = this;
switch (_that) {
case Command_InsertText() when insertText != null:
return insertText(_that);case Command_DeleteText() when deleteText != null:
return deleteText(_that);case Command_ReplaceText() when replaceText != null:
return replaceText(_that);case Command_SetText() when setText != null:
return setText(_that);case Command_SetRangeStyle() when setRangeStyle != null:
return setRangeStyle(_that);case Command_SetRangeColor() when setRangeColor != null:
return setRangeColor(_that);case Command_SetLink() when setLink != null:
return setLink(_that);case Command_SetProps() when setProps != null:
return setProps(_that);case Command_SetStyle() when setStyle != null:
return setStyle(_that);case Command_SetColumnWidth() when setColumnWidth != null:
return setColumnWidth(_that);case Command_CreateGrid() when createGrid != null:
return createGrid(_that);case Command_RemoveGrid() when removeGrid != null:
return removeGrid(_that);case Command_InsertRows() when insertRows != null:
return insertRows(_that);case Command_InsertColumns() when insertColumns != null:
return insertColumns(_that);case Command_DeleteRows() when deleteRows != null:
return deleteRows(_that);case Command_DeleteColumns() when deleteColumns != null:
return deleteColumns(_that);case Command_ClearCells() when clearCells != null:
return clearCells(_that);case Command_DeleteCells() when deleteCells != null:
return deleteCells(_that);case Command_MoveRows() when moveRows != null:
return moveRows(_that);case Command_MoveColumns() when moveColumns != null:
return moveColumns(_that);case Command_SwapCells() when swapCells != null:
return swapCells(_that);case Command_MoveCell() when moveCell != null:
return moveCell(_that);case Command_CopyCell() when copyCell != null:
return copyCell(_that);case Command_SetTag() when setTag != null:
return setTag(_that);case Command_RemoveTag() when removeTag != null:
return removeTag(_that);case Command_SortRows() when sortRows != null:
return sortRows(_that);case Command_Transpose() when transpose != null:
return transpose(_that);case Command_HierarchySwap() when hierarchySwap != null:
return hierarchySwap(_that);case Command_Hierarchify() when hierarchify != null:
return hierarchify(_that);case Command_Flatten() when flatten != null:
return flatten(_that);case Command_Wrap() when wrap != null:
return wrap(_that);case Command_Collapse() when collapse != null:
return collapse(_that);case Command_SplitCell() when splitCell != null:
return splitCell(_that);case Command_TagCells() when tagCells != null:
return tagCells(_that);case Command_UntagCells() when untagCells != null:
return untagCells(_that);case Command_AdjustRelSize() when adjustRelSize != null:
return adjustRelSize(_that);case Command_AdjustColumnWidths() when adjustColumnWidths != null:
return adjustColumnWidths(_that);case Command_Run() when run != null:
return run(_that);case Command_EnterText() when enterText != null:
return enterText(_that);case Command_ToggleCheck() when toggleCheck != null:
return toggleCheck(_that);case Command_PasteCells() when pasteCells != null:
return pasteCells(_that);case Command_AdoptCells() when adoptCells != null:
return adoptCells(_that);case Command_ReleaseCell() when releaseCell != null:
return releaseCell(_that);case Command_Import() when import_ != null:
return import_(_that);case Command_RetargetLinks() when retargetLinks != null:
return retargetLinks(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( Command_InsertText value)  insertText,required TResult Function( Command_DeleteText value)  deleteText,required TResult Function( Command_ReplaceText value)  replaceText,required TResult Function( Command_SetText value)  setText,required TResult Function( Command_SetRangeStyle value)  setRangeStyle,required TResult Function( Command_SetRangeColor value)  setRangeColor,required TResult Function( Command_SetLink value)  setLink,required TResult Function( Command_SetProps value)  setProps,required TResult Function( Command_SetStyle value)  setStyle,required TResult Function( Command_SetColumnWidth value)  setColumnWidth,required TResult Function( Command_CreateGrid value)  createGrid,required TResult Function( Command_RemoveGrid value)  removeGrid,required TResult Function( Command_InsertRows value)  insertRows,required TResult Function( Command_InsertColumns value)  insertColumns,required TResult Function( Command_DeleteRows value)  deleteRows,required TResult Function( Command_DeleteColumns value)  deleteColumns,required TResult Function( Command_ClearCells value)  clearCells,required TResult Function( Command_DeleteCells value)  deleteCells,required TResult Function( Command_MoveRows value)  moveRows,required TResult Function( Command_MoveColumns value)  moveColumns,required TResult Function( Command_SwapCells value)  swapCells,required TResult Function( Command_MoveCell value)  moveCell,required TResult Function( Command_CopyCell value)  copyCell,required TResult Function( Command_SetTag value)  setTag,required TResult Function( Command_RemoveTag value)  removeTag,required TResult Function( Command_SortRows value)  sortRows,required TResult Function( Command_Transpose value)  transpose,required TResult Function( Command_HierarchySwap value)  hierarchySwap,required TResult Function( Command_Hierarchify value)  hierarchify,required TResult Function( Command_Flatten value)  flatten,required TResult Function( Command_Wrap value)  wrap,required TResult Function( Command_Collapse value)  collapse,required TResult Function( Command_SplitCell value)  splitCell,required TResult Function( Command_TagCells value)  tagCells,required TResult Function( Command_UntagCells value)  untagCells,required TResult Function( Command_AdjustRelSize value)  adjustRelSize,required TResult Function( Command_AdjustColumnWidths value)  adjustColumnWidths,required TResult Function( Command_Run value)  run,required TResult Function( Command_EnterText value)  enterText,required TResult Function( Command_ToggleCheck value)  toggleCheck,required TResult Function( Command_PasteCells value)  pasteCells,required TResult Function( Command_AdoptCells value)  adoptCells,required TResult Function( Command_ReleaseCell value)  releaseCell,required TResult Function( Command_Import value)  import_,required TResult Function( Command_RetargetLinks value)  retargetLinks,}){
final _that = this;
switch (_that) {
case Command_InsertText():
return insertText(_that);case Command_DeleteText():
return deleteText(_that);case Command_ReplaceText():
return replaceText(_that);case Command_SetText():
return setText(_that);case Command_SetRangeStyle():
return setRangeStyle(_that);case Command_SetRangeColor():
return setRangeColor(_that);case Command_SetLink():
return setLink(_that);case Command_SetProps():
return setProps(_that);case Command_SetStyle():
return setStyle(_that);case Command_SetColumnWidth():
return setColumnWidth(_that);case Command_CreateGrid():
return createGrid(_that);case Command_RemoveGrid():
return removeGrid(_that);case Command_InsertRows():
return insertRows(_that);case Command_InsertColumns():
return insertColumns(_that);case Command_DeleteRows():
return deleteRows(_that);case Command_DeleteColumns():
return deleteColumns(_that);case Command_ClearCells():
return clearCells(_that);case Command_DeleteCells():
return deleteCells(_that);case Command_MoveRows():
return moveRows(_that);case Command_MoveColumns():
return moveColumns(_that);case Command_SwapCells():
return swapCells(_that);case Command_MoveCell():
return moveCell(_that);case Command_CopyCell():
return copyCell(_that);case Command_SetTag():
return setTag(_that);case Command_RemoveTag():
return removeTag(_that);case Command_SortRows():
return sortRows(_that);case Command_Transpose():
return transpose(_that);case Command_HierarchySwap():
return hierarchySwap(_that);case Command_Hierarchify():
return hierarchify(_that);case Command_Flatten():
return flatten(_that);case Command_Wrap():
return wrap(_that);case Command_Collapse():
return collapse(_that);case Command_SplitCell():
return splitCell(_that);case Command_TagCells():
return tagCells(_that);case Command_UntagCells():
return untagCells(_that);case Command_AdjustRelSize():
return adjustRelSize(_that);case Command_AdjustColumnWidths():
return adjustColumnWidths(_that);case Command_Run():
return run(_that);case Command_EnterText():
return enterText(_that);case Command_ToggleCheck():
return toggleCheck(_that);case Command_PasteCells():
return pasteCells(_that);case Command_AdoptCells():
return adoptCells(_that);case Command_ReleaseCell():
return releaseCell(_that);case Command_Import():
return import_(_that);case Command_RetargetLinks():
return retargetLinks(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( Command_InsertText value)?  insertText,TResult? Function( Command_DeleteText value)?  deleteText,TResult? Function( Command_ReplaceText value)?  replaceText,TResult? Function( Command_SetText value)?  setText,TResult? Function( Command_SetRangeStyle value)?  setRangeStyle,TResult? Function( Command_SetRangeColor value)?  setRangeColor,TResult? Function( Command_SetLink value)?  setLink,TResult? Function( Command_SetProps value)?  setProps,TResult? Function( Command_SetStyle value)?  setStyle,TResult? Function( Command_SetColumnWidth value)?  setColumnWidth,TResult? Function( Command_CreateGrid value)?  createGrid,TResult? Function( Command_RemoveGrid value)?  removeGrid,TResult? Function( Command_InsertRows value)?  insertRows,TResult? Function( Command_InsertColumns value)?  insertColumns,TResult? Function( Command_DeleteRows value)?  deleteRows,TResult? Function( Command_DeleteColumns value)?  deleteColumns,TResult? Function( Command_ClearCells value)?  clearCells,TResult? Function( Command_DeleteCells value)?  deleteCells,TResult? Function( Command_MoveRows value)?  moveRows,TResult? Function( Command_MoveColumns value)?  moveColumns,TResult? Function( Command_SwapCells value)?  swapCells,TResult? Function( Command_MoveCell value)?  moveCell,TResult? Function( Command_CopyCell value)?  copyCell,TResult? Function( Command_SetTag value)?  setTag,TResult? Function( Command_RemoveTag value)?  removeTag,TResult? Function( Command_SortRows value)?  sortRows,TResult? Function( Command_Transpose value)?  transpose,TResult? Function( Command_HierarchySwap value)?  hierarchySwap,TResult? Function( Command_Hierarchify value)?  hierarchify,TResult? Function( Command_Flatten value)?  flatten,TResult? Function( Command_Wrap value)?  wrap,TResult? Function( Command_Collapse value)?  collapse,TResult? Function( Command_SplitCell value)?  splitCell,TResult? Function( Command_TagCells value)?  tagCells,TResult? Function( Command_UntagCells value)?  untagCells,TResult? Function( Command_AdjustRelSize value)?  adjustRelSize,TResult? Function( Command_AdjustColumnWidths value)?  adjustColumnWidths,TResult? Function( Command_Run value)?  run,TResult? Function( Command_EnterText value)?  enterText,TResult? Function( Command_ToggleCheck value)?  toggleCheck,TResult? Function( Command_PasteCells value)?  pasteCells,TResult? Function( Command_AdoptCells value)?  adoptCells,TResult? Function( Command_ReleaseCell value)?  releaseCell,TResult? Function( Command_Import value)?  import_,TResult? Function( Command_RetargetLinks value)?  retargetLinks,}){
final _that = this;
switch (_that) {
case Command_InsertText() when insertText != null:
return insertText(_that);case Command_DeleteText() when deleteText != null:
return deleteText(_that);case Command_ReplaceText() when replaceText != null:
return replaceText(_that);case Command_SetText() when setText != null:
return setText(_that);case Command_SetRangeStyle() when setRangeStyle != null:
return setRangeStyle(_that);case Command_SetRangeColor() when setRangeColor != null:
return setRangeColor(_that);case Command_SetLink() when setLink != null:
return setLink(_that);case Command_SetProps() when setProps != null:
return setProps(_that);case Command_SetStyle() when setStyle != null:
return setStyle(_that);case Command_SetColumnWidth() when setColumnWidth != null:
return setColumnWidth(_that);case Command_CreateGrid() when createGrid != null:
return createGrid(_that);case Command_RemoveGrid() when removeGrid != null:
return removeGrid(_that);case Command_InsertRows() when insertRows != null:
return insertRows(_that);case Command_InsertColumns() when insertColumns != null:
return insertColumns(_that);case Command_DeleteRows() when deleteRows != null:
return deleteRows(_that);case Command_DeleteColumns() when deleteColumns != null:
return deleteColumns(_that);case Command_ClearCells() when clearCells != null:
return clearCells(_that);case Command_DeleteCells() when deleteCells != null:
return deleteCells(_that);case Command_MoveRows() when moveRows != null:
return moveRows(_that);case Command_MoveColumns() when moveColumns != null:
return moveColumns(_that);case Command_SwapCells() when swapCells != null:
return swapCells(_that);case Command_MoveCell() when moveCell != null:
return moveCell(_that);case Command_CopyCell() when copyCell != null:
return copyCell(_that);case Command_SetTag() when setTag != null:
return setTag(_that);case Command_RemoveTag() when removeTag != null:
return removeTag(_that);case Command_SortRows() when sortRows != null:
return sortRows(_that);case Command_Transpose() when transpose != null:
return transpose(_that);case Command_HierarchySwap() when hierarchySwap != null:
return hierarchySwap(_that);case Command_Hierarchify() when hierarchify != null:
return hierarchify(_that);case Command_Flatten() when flatten != null:
return flatten(_that);case Command_Wrap() when wrap != null:
return wrap(_that);case Command_Collapse() when collapse != null:
return collapse(_that);case Command_SplitCell() when splitCell != null:
return splitCell(_that);case Command_TagCells() when tagCells != null:
return tagCells(_that);case Command_UntagCells() when untagCells != null:
return untagCells(_that);case Command_AdjustRelSize() when adjustRelSize != null:
return adjustRelSize(_that);case Command_AdjustColumnWidths() when adjustColumnWidths != null:
return adjustColumnWidths(_that);case Command_Run() when run != null:
return run(_that);case Command_EnterText() when enterText != null:
return enterText(_that);case Command_ToggleCheck() when toggleCheck != null:
return toggleCheck(_that);case Command_PasteCells() when pasteCells != null:
return pasteCells(_that);case Command_AdoptCells() when adoptCells != null:
return adoptCells(_that);case Command_ReleaseCell() when releaseCell != null:
return releaseCell(_that);case Command_Import() when import_ != null:
return import_(_that);case Command_RetargetLinks() when retargetLinks != null:
return retargetLinks(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String cell,  int at,  String text)?  insertText,TResult Function( String cell,  int start,  int len)?  deleteText,TResult Function( String cell,  int start,  int len,  String text)?  replaceText,TResult Function( String cell,  String text)?  setText,TResult Function( String cell,  int start,  int len,  Style set_,  Style clear)?  setRangeStyle,TResult Function( String cell,  int start,  int len,  Rgb? color)?  setRangeColor,TResult Function( String cell,  int start,  int len,  LinkTarget? target)?  setLink,TResult Function( List<String> cells,  List<Prop> props)?  setProps,TResult Function( List<String> cells,  Style set_,  Style clear)?  setStyle,TResult Function( String cell,  int column,  int width)?  setColumnWidth,TResult Function( String cell,  int xs,  int ys)?  createGrid,TResult Function( String cell)?  removeGrid,TResult Function( String cell,  int at,  int count)?  insertRows,TResult Function( String cell,  int at,  int count)?  insertColumns,TResult Function( String cell,  int at,  int count)?  deleteRows,TResult Function( String cell,  int at,  int count)?  deleteColumns,TResult Function( List<String> cells)?  clearCells,TResult Function( String cell,  int x,  int y,  int xs,  int ys)?  deleteCells,TResult Function( String cell,  int from,  int count,  int to)?  moveRows,TResult Function( String cell,  int from,  int count,  int to)?  moveColumns,TResult Function( String a,  String b)?  swapCells,TResult Function( String from,  String to)?  moveCell,TResult Function( String from,  String to)?  copyCell,TResult Function( String name,  Rgb cellColor,  Rgb textColor)?  setTag,TResult Function( String name)?  removeTag,TResult Function( String cell,  int column,  int row,  int count,  bool descending)?  sortRows,TResult Function( List<String> cells)?  transpose,TResult Function( String cell)?  hierarchySwap,TResult Function( String cell)?  hierarchify,TResult Function( String cell)?  flatten,TResult Function( String cell,  int x,  int y,  int xs,  int ys)?  wrap,TResult Function( String cell,  int x,  int y,  int xs,  int ys)?  collapse,TResult Function( String cell,  int at)?  splitCell,TResult Function( List<String> cells)?  tagCells,TResult Function( List<String> cells)?  untagCells,TResult Function( List<String> cells,  int delta,  bool recursive)?  adjustRelSize,TResult Function( String cell,  int column,  int count,  int delta,  bool recursive)?  adjustColumnWidths,TResult Function()?  run,TResult Function( String cell,  String text)?  enterText,TResult Function( List<String> cells)?  toggleCheck,TResult Function( String to,  ClipData clip)?  pasteCells,TResult Function( String to,  ClipData clip)?  adoptCells,TResult Function( String cell)?  releaseCell,TResult Function( String cell,  TableData table,  bool replace)?  import_,TResult Function( List<String> cells,  String? from,  String? to)?  retargetLinks,required TResult orElse(),}) {final _that = this;
switch (_that) {
case Command_InsertText() when insertText != null:
return insertText(_that.cell,_that.at,_that.text);case Command_DeleteText() when deleteText != null:
return deleteText(_that.cell,_that.start,_that.len);case Command_ReplaceText() when replaceText != null:
return replaceText(_that.cell,_that.start,_that.len,_that.text);case Command_SetText() when setText != null:
return setText(_that.cell,_that.text);case Command_SetRangeStyle() when setRangeStyle != null:
return setRangeStyle(_that.cell,_that.start,_that.len,_that.set_,_that.clear);case Command_SetRangeColor() when setRangeColor != null:
return setRangeColor(_that.cell,_that.start,_that.len,_that.color);case Command_SetLink() when setLink != null:
return setLink(_that.cell,_that.start,_that.len,_that.target);case Command_SetProps() when setProps != null:
return setProps(_that.cells,_that.props);case Command_SetStyle() when setStyle != null:
return setStyle(_that.cells,_that.set_,_that.clear);case Command_SetColumnWidth() when setColumnWidth != null:
return setColumnWidth(_that.cell,_that.column,_that.width);case Command_CreateGrid() when createGrid != null:
return createGrid(_that.cell,_that.xs,_that.ys);case Command_RemoveGrid() when removeGrid != null:
return removeGrid(_that.cell);case Command_InsertRows() when insertRows != null:
return insertRows(_that.cell,_that.at,_that.count);case Command_InsertColumns() when insertColumns != null:
return insertColumns(_that.cell,_that.at,_that.count);case Command_DeleteRows() when deleteRows != null:
return deleteRows(_that.cell,_that.at,_that.count);case Command_DeleteColumns() when deleteColumns != null:
return deleteColumns(_that.cell,_that.at,_that.count);case Command_ClearCells() when clearCells != null:
return clearCells(_that.cells);case Command_DeleteCells() when deleteCells != null:
return deleteCells(_that.cell,_that.x,_that.y,_that.xs,_that.ys);case Command_MoveRows() when moveRows != null:
return moveRows(_that.cell,_that.from,_that.count,_that.to);case Command_MoveColumns() when moveColumns != null:
return moveColumns(_that.cell,_that.from,_that.count,_that.to);case Command_SwapCells() when swapCells != null:
return swapCells(_that.a,_that.b);case Command_MoveCell() when moveCell != null:
return moveCell(_that.from,_that.to);case Command_CopyCell() when copyCell != null:
return copyCell(_that.from,_that.to);case Command_SetTag() when setTag != null:
return setTag(_that.name,_that.cellColor,_that.textColor);case Command_RemoveTag() when removeTag != null:
return removeTag(_that.name);case Command_SortRows() when sortRows != null:
return sortRows(_that.cell,_that.column,_that.row,_that.count,_that.descending);case Command_Transpose() when transpose != null:
return transpose(_that.cells);case Command_HierarchySwap() when hierarchySwap != null:
return hierarchySwap(_that.cell);case Command_Hierarchify() when hierarchify != null:
return hierarchify(_that.cell);case Command_Flatten() when flatten != null:
return flatten(_that.cell);case Command_Wrap() when wrap != null:
return wrap(_that.cell,_that.x,_that.y,_that.xs,_that.ys);case Command_Collapse() when collapse != null:
return collapse(_that.cell,_that.x,_that.y,_that.xs,_that.ys);case Command_SplitCell() when splitCell != null:
return splitCell(_that.cell,_that.at);case Command_TagCells() when tagCells != null:
return tagCells(_that.cells);case Command_UntagCells() when untagCells != null:
return untagCells(_that.cells);case Command_AdjustRelSize() when adjustRelSize != null:
return adjustRelSize(_that.cells,_that.delta,_that.recursive);case Command_AdjustColumnWidths() when adjustColumnWidths != null:
return adjustColumnWidths(_that.cell,_that.column,_that.count,_that.delta,_that.recursive);case Command_Run() when run != null:
return run();case Command_EnterText() when enterText != null:
return enterText(_that.cell,_that.text);case Command_ToggleCheck() when toggleCheck != null:
return toggleCheck(_that.cells);case Command_PasteCells() when pasteCells != null:
return pasteCells(_that.to,_that.clip);case Command_AdoptCells() when adoptCells != null:
return adoptCells(_that.to,_that.clip);case Command_ReleaseCell() when releaseCell != null:
return releaseCell(_that.cell);case Command_Import() when import_ != null:
return import_(_that.cell,_that.table,_that.replace);case Command_RetargetLinks() when retargetLinks != null:
return retargetLinks(_that.cells,_that.from,_that.to);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String cell,  int at,  String text)  insertText,required TResult Function( String cell,  int start,  int len)  deleteText,required TResult Function( String cell,  int start,  int len,  String text)  replaceText,required TResult Function( String cell,  String text)  setText,required TResult Function( String cell,  int start,  int len,  Style set_,  Style clear)  setRangeStyle,required TResult Function( String cell,  int start,  int len,  Rgb? color)  setRangeColor,required TResult Function( String cell,  int start,  int len,  LinkTarget? target)  setLink,required TResult Function( List<String> cells,  List<Prop> props)  setProps,required TResult Function( List<String> cells,  Style set_,  Style clear)  setStyle,required TResult Function( String cell,  int column,  int width)  setColumnWidth,required TResult Function( String cell,  int xs,  int ys)  createGrid,required TResult Function( String cell)  removeGrid,required TResult Function( String cell,  int at,  int count)  insertRows,required TResult Function( String cell,  int at,  int count)  insertColumns,required TResult Function( String cell,  int at,  int count)  deleteRows,required TResult Function( String cell,  int at,  int count)  deleteColumns,required TResult Function( List<String> cells)  clearCells,required TResult Function( String cell,  int x,  int y,  int xs,  int ys)  deleteCells,required TResult Function( String cell,  int from,  int count,  int to)  moveRows,required TResult Function( String cell,  int from,  int count,  int to)  moveColumns,required TResult Function( String a,  String b)  swapCells,required TResult Function( String from,  String to)  moveCell,required TResult Function( String from,  String to)  copyCell,required TResult Function( String name,  Rgb cellColor,  Rgb textColor)  setTag,required TResult Function( String name)  removeTag,required TResult Function( String cell,  int column,  int row,  int count,  bool descending)  sortRows,required TResult Function( List<String> cells)  transpose,required TResult Function( String cell)  hierarchySwap,required TResult Function( String cell)  hierarchify,required TResult Function( String cell)  flatten,required TResult Function( String cell,  int x,  int y,  int xs,  int ys)  wrap,required TResult Function( String cell,  int x,  int y,  int xs,  int ys)  collapse,required TResult Function( String cell,  int at)  splitCell,required TResult Function( List<String> cells)  tagCells,required TResult Function( List<String> cells)  untagCells,required TResult Function( List<String> cells,  int delta,  bool recursive)  adjustRelSize,required TResult Function( String cell,  int column,  int count,  int delta,  bool recursive)  adjustColumnWidths,required TResult Function()  run,required TResult Function( String cell,  String text)  enterText,required TResult Function( List<String> cells)  toggleCheck,required TResult Function( String to,  ClipData clip)  pasteCells,required TResult Function( String to,  ClipData clip)  adoptCells,required TResult Function( String cell)  releaseCell,required TResult Function( String cell,  TableData table,  bool replace)  import_,required TResult Function( List<String> cells,  String? from,  String? to)  retargetLinks,}) {final _that = this;
switch (_that) {
case Command_InsertText():
return insertText(_that.cell,_that.at,_that.text);case Command_DeleteText():
return deleteText(_that.cell,_that.start,_that.len);case Command_ReplaceText():
return replaceText(_that.cell,_that.start,_that.len,_that.text);case Command_SetText():
return setText(_that.cell,_that.text);case Command_SetRangeStyle():
return setRangeStyle(_that.cell,_that.start,_that.len,_that.set_,_that.clear);case Command_SetRangeColor():
return setRangeColor(_that.cell,_that.start,_that.len,_that.color);case Command_SetLink():
return setLink(_that.cell,_that.start,_that.len,_that.target);case Command_SetProps():
return setProps(_that.cells,_that.props);case Command_SetStyle():
return setStyle(_that.cells,_that.set_,_that.clear);case Command_SetColumnWidth():
return setColumnWidth(_that.cell,_that.column,_that.width);case Command_CreateGrid():
return createGrid(_that.cell,_that.xs,_that.ys);case Command_RemoveGrid():
return removeGrid(_that.cell);case Command_InsertRows():
return insertRows(_that.cell,_that.at,_that.count);case Command_InsertColumns():
return insertColumns(_that.cell,_that.at,_that.count);case Command_DeleteRows():
return deleteRows(_that.cell,_that.at,_that.count);case Command_DeleteColumns():
return deleteColumns(_that.cell,_that.at,_that.count);case Command_ClearCells():
return clearCells(_that.cells);case Command_DeleteCells():
return deleteCells(_that.cell,_that.x,_that.y,_that.xs,_that.ys);case Command_MoveRows():
return moveRows(_that.cell,_that.from,_that.count,_that.to);case Command_MoveColumns():
return moveColumns(_that.cell,_that.from,_that.count,_that.to);case Command_SwapCells():
return swapCells(_that.a,_that.b);case Command_MoveCell():
return moveCell(_that.from,_that.to);case Command_CopyCell():
return copyCell(_that.from,_that.to);case Command_SetTag():
return setTag(_that.name,_that.cellColor,_that.textColor);case Command_RemoveTag():
return removeTag(_that.name);case Command_SortRows():
return sortRows(_that.cell,_that.column,_that.row,_that.count,_that.descending);case Command_Transpose():
return transpose(_that.cells);case Command_HierarchySwap():
return hierarchySwap(_that.cell);case Command_Hierarchify():
return hierarchify(_that.cell);case Command_Flatten():
return flatten(_that.cell);case Command_Wrap():
return wrap(_that.cell,_that.x,_that.y,_that.xs,_that.ys);case Command_Collapse():
return collapse(_that.cell,_that.x,_that.y,_that.xs,_that.ys);case Command_SplitCell():
return splitCell(_that.cell,_that.at);case Command_TagCells():
return tagCells(_that.cells);case Command_UntagCells():
return untagCells(_that.cells);case Command_AdjustRelSize():
return adjustRelSize(_that.cells,_that.delta,_that.recursive);case Command_AdjustColumnWidths():
return adjustColumnWidths(_that.cell,_that.column,_that.count,_that.delta,_that.recursive);case Command_Run():
return run();case Command_EnterText():
return enterText(_that.cell,_that.text);case Command_ToggleCheck():
return toggleCheck(_that.cells);case Command_PasteCells():
return pasteCells(_that.to,_that.clip);case Command_AdoptCells():
return adoptCells(_that.to,_that.clip);case Command_ReleaseCell():
return releaseCell(_that.cell);case Command_Import():
return import_(_that.cell,_that.table,_that.replace);case Command_RetargetLinks():
return retargetLinks(_that.cells,_that.from,_that.to);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String cell,  int at,  String text)?  insertText,TResult? Function( String cell,  int start,  int len)?  deleteText,TResult? Function( String cell,  int start,  int len,  String text)?  replaceText,TResult? Function( String cell,  String text)?  setText,TResult? Function( String cell,  int start,  int len,  Style set_,  Style clear)?  setRangeStyle,TResult? Function( String cell,  int start,  int len,  Rgb? color)?  setRangeColor,TResult? Function( String cell,  int start,  int len,  LinkTarget? target)?  setLink,TResult? Function( List<String> cells,  List<Prop> props)?  setProps,TResult? Function( List<String> cells,  Style set_,  Style clear)?  setStyle,TResult? Function( String cell,  int column,  int width)?  setColumnWidth,TResult? Function( String cell,  int xs,  int ys)?  createGrid,TResult? Function( String cell)?  removeGrid,TResult? Function( String cell,  int at,  int count)?  insertRows,TResult? Function( String cell,  int at,  int count)?  insertColumns,TResult? Function( String cell,  int at,  int count)?  deleteRows,TResult? Function( String cell,  int at,  int count)?  deleteColumns,TResult? Function( List<String> cells)?  clearCells,TResult? Function( String cell,  int x,  int y,  int xs,  int ys)?  deleteCells,TResult? Function( String cell,  int from,  int count,  int to)?  moveRows,TResult? Function( String cell,  int from,  int count,  int to)?  moveColumns,TResult? Function( String a,  String b)?  swapCells,TResult? Function( String from,  String to)?  moveCell,TResult? Function( String from,  String to)?  copyCell,TResult? Function( String name,  Rgb cellColor,  Rgb textColor)?  setTag,TResult? Function( String name)?  removeTag,TResult? Function( String cell,  int column,  int row,  int count,  bool descending)?  sortRows,TResult? Function( List<String> cells)?  transpose,TResult? Function( String cell)?  hierarchySwap,TResult? Function( String cell)?  hierarchify,TResult? Function( String cell)?  flatten,TResult? Function( String cell,  int x,  int y,  int xs,  int ys)?  wrap,TResult? Function( String cell,  int x,  int y,  int xs,  int ys)?  collapse,TResult? Function( String cell,  int at)?  splitCell,TResult? Function( List<String> cells)?  tagCells,TResult? Function( List<String> cells)?  untagCells,TResult? Function( List<String> cells,  int delta,  bool recursive)?  adjustRelSize,TResult? Function( String cell,  int column,  int count,  int delta,  bool recursive)?  adjustColumnWidths,TResult? Function()?  run,TResult? Function( String cell,  String text)?  enterText,TResult? Function( List<String> cells)?  toggleCheck,TResult? Function( String to,  ClipData clip)?  pasteCells,TResult? Function( String to,  ClipData clip)?  adoptCells,TResult? Function( String cell)?  releaseCell,TResult? Function( String cell,  TableData table,  bool replace)?  import_,TResult? Function( List<String> cells,  String? from,  String? to)?  retargetLinks,}) {final _that = this;
switch (_that) {
case Command_InsertText() when insertText != null:
return insertText(_that.cell,_that.at,_that.text);case Command_DeleteText() when deleteText != null:
return deleteText(_that.cell,_that.start,_that.len);case Command_ReplaceText() when replaceText != null:
return replaceText(_that.cell,_that.start,_that.len,_that.text);case Command_SetText() when setText != null:
return setText(_that.cell,_that.text);case Command_SetRangeStyle() when setRangeStyle != null:
return setRangeStyle(_that.cell,_that.start,_that.len,_that.set_,_that.clear);case Command_SetRangeColor() when setRangeColor != null:
return setRangeColor(_that.cell,_that.start,_that.len,_that.color);case Command_SetLink() when setLink != null:
return setLink(_that.cell,_that.start,_that.len,_that.target);case Command_SetProps() when setProps != null:
return setProps(_that.cells,_that.props);case Command_SetStyle() when setStyle != null:
return setStyle(_that.cells,_that.set_,_that.clear);case Command_SetColumnWidth() when setColumnWidth != null:
return setColumnWidth(_that.cell,_that.column,_that.width);case Command_CreateGrid() when createGrid != null:
return createGrid(_that.cell,_that.xs,_that.ys);case Command_RemoveGrid() when removeGrid != null:
return removeGrid(_that.cell);case Command_InsertRows() when insertRows != null:
return insertRows(_that.cell,_that.at,_that.count);case Command_InsertColumns() when insertColumns != null:
return insertColumns(_that.cell,_that.at,_that.count);case Command_DeleteRows() when deleteRows != null:
return deleteRows(_that.cell,_that.at,_that.count);case Command_DeleteColumns() when deleteColumns != null:
return deleteColumns(_that.cell,_that.at,_that.count);case Command_ClearCells() when clearCells != null:
return clearCells(_that.cells);case Command_DeleteCells() when deleteCells != null:
return deleteCells(_that.cell,_that.x,_that.y,_that.xs,_that.ys);case Command_MoveRows() when moveRows != null:
return moveRows(_that.cell,_that.from,_that.count,_that.to);case Command_MoveColumns() when moveColumns != null:
return moveColumns(_that.cell,_that.from,_that.count,_that.to);case Command_SwapCells() when swapCells != null:
return swapCells(_that.a,_that.b);case Command_MoveCell() when moveCell != null:
return moveCell(_that.from,_that.to);case Command_CopyCell() when copyCell != null:
return copyCell(_that.from,_that.to);case Command_SetTag() when setTag != null:
return setTag(_that.name,_that.cellColor,_that.textColor);case Command_RemoveTag() when removeTag != null:
return removeTag(_that.name);case Command_SortRows() when sortRows != null:
return sortRows(_that.cell,_that.column,_that.row,_that.count,_that.descending);case Command_Transpose() when transpose != null:
return transpose(_that.cells);case Command_HierarchySwap() when hierarchySwap != null:
return hierarchySwap(_that.cell);case Command_Hierarchify() when hierarchify != null:
return hierarchify(_that.cell);case Command_Flatten() when flatten != null:
return flatten(_that.cell);case Command_Wrap() when wrap != null:
return wrap(_that.cell,_that.x,_that.y,_that.xs,_that.ys);case Command_Collapse() when collapse != null:
return collapse(_that.cell,_that.x,_that.y,_that.xs,_that.ys);case Command_SplitCell() when splitCell != null:
return splitCell(_that.cell,_that.at);case Command_TagCells() when tagCells != null:
return tagCells(_that.cells);case Command_UntagCells() when untagCells != null:
return untagCells(_that.cells);case Command_AdjustRelSize() when adjustRelSize != null:
return adjustRelSize(_that.cells,_that.delta,_that.recursive);case Command_AdjustColumnWidths() when adjustColumnWidths != null:
return adjustColumnWidths(_that.cell,_that.column,_that.count,_that.delta,_that.recursive);case Command_Run() when run != null:
return run();case Command_EnterText() when enterText != null:
return enterText(_that.cell,_that.text);case Command_ToggleCheck() when toggleCheck != null:
return toggleCheck(_that.cells);case Command_PasteCells() when pasteCells != null:
return pasteCells(_that.to,_that.clip);case Command_AdoptCells() when adoptCells != null:
return adoptCells(_that.to,_that.clip);case Command_ReleaseCell() when releaseCell != null:
return releaseCell(_that.cell);case Command_Import() when import_ != null:
return import_(_that.cell,_that.table,_that.replace);case Command_RetargetLinks() when retargetLinks != null:
return retargetLinks(_that.cells,_that.from,_that.to);case _:
  return null;

}
}

}

/// @nodoc


class Command_InsertText extends Command {
  const Command_InsertText({required this.cell, required this.at, required this.text}): super._();
  

/// The cell.
 final  String cell;
/// Where to insert: a position in code points, not inside a user-perceived
/// character.
 final  int at;
/// What to insert; control characters become spaces. Half of a
/// surrogate pair on its own, which a Dart string can hold, arrives as U+FFFD.
 final  String text;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_InsertTextCopyWith<Command_InsertText> get copyWith => _$Command_InsertTextCopyWithImpl<Command_InsertText>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_InsertText&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.at, at) || other.at == at)&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,at,text);
}

@override
String toString() {
    return 'Command.insertText(cell: $cell, at: $at, text: $text)';
}


}

/// @nodoc
abstract mixin class $Command_InsertTextCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_InsertTextCopyWith(Command_InsertText value, $Res Function(Command_InsertText) _then) = _$Command_InsertTextCopyWithImpl;
@useResult
$Res call({
 String cell, int at, String text
});




}
/// @nodoc
class _$Command_InsertTextCopyWithImpl<$Res>
    implements $Command_InsertTextCopyWith<$Res> {
  _$Command_InsertTextCopyWithImpl(this._self, this._then);

  final Command_InsertText _self;
  final $Res Function(Command_InsertText) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? at = null,Object? text = null,}) {
  return _then(Command_InsertText(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_DeleteText extends Command {
  const Command_DeleteText({required this.cell, required this.start, required this.len}): super._();
  

/// The cell.
 final  String cell;
/// The first code point to delete. Positions count code points, and a range
/// must not cut a user-perceived character.
 final  int start;
/// How many code points to delete.
 final  int len;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_DeleteTextCopyWith<Command_DeleteText> get copyWith => _$Command_DeleteTextCopyWithImpl<Command_DeleteText>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_DeleteText&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.start, start) || other.start == start)&&(identical(other.len, len) || other.len == len));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,start,len);
}

@override
String toString() {
    return 'Command.deleteText(cell: $cell, start: $start, len: $len)';
}


}

/// @nodoc
abstract mixin class $Command_DeleteTextCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_DeleteTextCopyWith(Command_DeleteText value, $Res Function(Command_DeleteText) _then) = _$Command_DeleteTextCopyWithImpl;
@useResult
$Res call({
 String cell, int start, int len
});




}
/// @nodoc
class _$Command_DeleteTextCopyWithImpl<$Res>
    implements $Command_DeleteTextCopyWith<$Res> {
  _$Command_DeleteTextCopyWithImpl(this._self, this._then);

  final Command_DeleteText _self;
  final $Res Function(Command_DeleteText) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? start = null,Object? len = null,}) {
  return _then(Command_DeleteText(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,start: null == start ? _self.start : start // ignore: cast_nullable_to_non_nullable
as int,len: null == len ? _self.len : len // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_ReplaceText extends Command {
  const Command_ReplaceText({required this.cell, required this.start, required this.len, required this.text}): super._();
  

/// The cell.
 final  String cell;
/// The first code point to replace. Positions count code points, and a range
/// must not cut a user-perceived character.
 final  int start;
/// How many code points to replace.
 final  int len;
/// The new text; control characters become spaces. Half of a
/// surrogate pair on its own, which a Dart string can hold, arrives as U+FFFD.
 final  String text;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_ReplaceTextCopyWith<Command_ReplaceText> get copyWith => _$Command_ReplaceTextCopyWithImpl<Command_ReplaceText>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_ReplaceText&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.start, start) || other.start == start)&&(identical(other.len, len) || other.len == len)&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,start,len,text);
}

@override
String toString() {
    return 'Command.replaceText(cell: $cell, start: $start, len: $len, text: $text)';
}


}

/// @nodoc
abstract mixin class $Command_ReplaceTextCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_ReplaceTextCopyWith(Command_ReplaceText value, $Res Function(Command_ReplaceText) _then) = _$Command_ReplaceTextCopyWithImpl;
@useResult
$Res call({
 String cell, int start, int len, String text
});




}
/// @nodoc
class _$Command_ReplaceTextCopyWithImpl<$Res>
    implements $Command_ReplaceTextCopyWith<$Res> {
  _$Command_ReplaceTextCopyWithImpl(this._self, this._then);

  final Command_ReplaceText _self;
  final $Res Function(Command_ReplaceText) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? start = null,Object? len = null,Object? text = null,}) {
  return _then(Command_ReplaceText(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,start: null == start ? _self.start : start // ignore: cast_nullable_to_non_nullable
as int,len: null == len ? _self.len : len // ignore: cast_nullable_to_non_nullable
as int,text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_SetText extends Command {
  const Command_SetText({required this.cell, required this.text}): super._();
  

/// The cell.
 final  String cell;
/// The new text; control characters become spaces. Half of a
/// surrogate pair on its own, which a Dart string can hold, arrives as U+FFFD.
 final  String text;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SetTextCopyWith<Command_SetText> get copyWith => _$Command_SetTextCopyWithImpl<Command_SetText>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SetText&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,text);
}

@override
String toString() {
    return 'Command.setText(cell: $cell, text: $text)';
}


}

/// @nodoc
abstract mixin class $Command_SetTextCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SetTextCopyWith(Command_SetText value, $Res Function(Command_SetText) _then) = _$Command_SetTextCopyWithImpl;
@useResult
$Res call({
 String cell, String text
});




}
/// @nodoc
class _$Command_SetTextCopyWithImpl<$Res>
    implements $Command_SetTextCopyWith<$Res> {
  _$Command_SetTextCopyWithImpl(this._self, this._then);

  final Command_SetText _self;
  final $Res Function(Command_SetText) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? text = null,}) {
  return _then(Command_SetText(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_SetRangeStyle extends Command {
  const Command_SetRangeStyle({required this.cell, required this.start, required this.len, required this.set_, required this.clear}): super._();
  

/// The cell.
 final  String cell;
/// The first code point. Positions count code points, and a range
/// must not cut a user-perceived character.
 final  int start;
/// How many code points.
 final  int len;
/// The bits to switch on.
 final  Style set_;
/// The bits to switch off.
 final  Style clear;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SetRangeStyleCopyWith<Command_SetRangeStyle> get copyWith => _$Command_SetRangeStyleCopyWithImpl<Command_SetRangeStyle>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SetRangeStyle&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.start, start) || other.start == start)&&(identical(other.len, len) || other.len == len)&&(identical(other.set_, set_) || other.set_ == set_)&&(identical(other.clear, clear) || other.clear == clear));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,start,len,set_,clear);
}

@override
String toString() {
    return 'Command.setRangeStyle(cell: $cell, start: $start, len: $len, set_: $set_, clear: $clear)';
}


}

/// @nodoc
abstract mixin class $Command_SetRangeStyleCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SetRangeStyleCopyWith(Command_SetRangeStyle value, $Res Function(Command_SetRangeStyle) _then) = _$Command_SetRangeStyleCopyWithImpl;
@useResult
$Res call({
 String cell, int start, int len, Style set_, Style clear
});




}
/// @nodoc
class _$Command_SetRangeStyleCopyWithImpl<$Res>
    implements $Command_SetRangeStyleCopyWith<$Res> {
  _$Command_SetRangeStyleCopyWithImpl(this._self, this._then);

  final Command_SetRangeStyle _self;
  final $Res Function(Command_SetRangeStyle) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? start = null,Object? len = null,Object? set_ = null,Object? clear = null,}) {
  return _then(Command_SetRangeStyle(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,start: null == start ? _self.start : start // ignore: cast_nullable_to_non_nullable
as int,len: null == len ? _self.len : len // ignore: cast_nullable_to_non_nullable
as int,set_: null == set_ ? _self.set_ : set_ // ignore: cast_nullable_to_non_nullable
as Style,clear: null == clear ? _self.clear : clear // ignore: cast_nullable_to_non_nullable
as Style,
  ));
}


}

/// @nodoc


class Command_SetRangeColor extends Command {
  const Command_SetRangeColor({required this.cell, required this.start, required this.len, this.color}): super._();
  

/// The cell.
 final  String cell;
/// The first code point. Positions count code points, and a range
/// must not cut a user-perceived character.
 final  int start;
/// How many code points.
 final  int len;
/// The colour.
 final  Rgb? color;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SetRangeColorCopyWith<Command_SetRangeColor> get copyWith => _$Command_SetRangeColorCopyWithImpl<Command_SetRangeColor>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SetRangeColor&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.start, start) || other.start == start)&&(identical(other.len, len) || other.len == len)&&(identical(other.color, color) || other.color == color));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,start,len,color);
}

@override
String toString() {
    return 'Command.setRangeColor(cell: $cell, start: $start, len: $len, color: $color)';
}


}

/// @nodoc
abstract mixin class $Command_SetRangeColorCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SetRangeColorCopyWith(Command_SetRangeColor value, $Res Function(Command_SetRangeColor) _then) = _$Command_SetRangeColorCopyWithImpl;
@useResult
$Res call({
 String cell, int start, int len, Rgb? color
});




}
/// @nodoc
class _$Command_SetRangeColorCopyWithImpl<$Res>
    implements $Command_SetRangeColorCopyWith<$Res> {
  _$Command_SetRangeColorCopyWithImpl(this._self, this._then);

  final Command_SetRangeColor _self;
  final $Res Function(Command_SetRangeColor) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? start = null,Object? len = null,Object? color = freezed,}) {
  return _then(Command_SetRangeColor(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,start: null == start ? _self.start : start // ignore: cast_nullable_to_non_nullable
as int,len: null == len ? _self.len : len // ignore: cast_nullable_to_non_nullable
as int,color: freezed == color ? _self.color : color // ignore: cast_nullable_to_non_nullable
as Rgb?,
  ));
}


}

/// @nodoc


class Command_SetLink extends Command {
  const Command_SetLink({required this.cell, required this.start, required this.len, this.target}): super._();
  

/// The cell.
 final  String cell;
/// The first code point. Positions count code points, and a range
/// must not cut a user-perceived character.
 final  int start;
/// How many code points.
 final  int len;
/// Where the link goes: an external target or a cell. For a cell of this
/// document, the document's own ID is dropped from the target.
 final  LinkTarget? target;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SetLinkCopyWith<Command_SetLink> get copyWith => _$Command_SetLinkCopyWithImpl<Command_SetLink>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SetLink&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.start, start) || other.start == start)&&(identical(other.len, len) || other.len == len)&&(identical(other.target, target) || other.target == target));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,start,len,target);
}

@override
String toString() {
    return 'Command.setLink(cell: $cell, start: $start, len: $len, target: $target)';
}


}

/// @nodoc
abstract mixin class $Command_SetLinkCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SetLinkCopyWith(Command_SetLink value, $Res Function(Command_SetLink) _then) = _$Command_SetLinkCopyWithImpl;
@useResult
$Res call({
 String cell, int start, int len, LinkTarget? target
});


$LinkTargetCopyWith<$Res>? get target;

}
/// @nodoc
class _$Command_SetLinkCopyWithImpl<$Res>
    implements $Command_SetLinkCopyWith<$Res> {
  _$Command_SetLinkCopyWithImpl(this._self, this._then);

  final Command_SetLink _self;
  final $Res Function(Command_SetLink) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? start = null,Object? len = null,Object? target = freezed,}) {
  return _then(Command_SetLink(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,start: null == start ? _self.start : start // ignore: cast_nullable_to_non_nullable
as int,len: null == len ? _self.len : len // ignore: cast_nullable_to_non_nullable
as int,target: freezed == target ? _self.target : target // ignore: cast_nullable_to_non_nullable
as LinkTarget?,
  ));
}

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$LinkTargetCopyWith<$Res>? get target {
    if (_self.target == null) {
    return null;
  }

  return $LinkTargetCopyWith<$Res>(_self.target!, (value) {
    return _then(_self.copyWith(target: value));
  });
}
}

/// @nodoc


class Command_SetProps extends Command {
  const Command_SetProps({required  List<String> cells, required  List<Prop> props}): _cells = cells,_props = props,super._();
  

/// The cells.
 final  List<String> _cells;
/// The cells.
 List<String> get cells {
  if (_cells is EqualUnmodifiableListView) return _cells;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_cells);
}

/// The properties with their new values.
 final  List<Prop> _props;
/// The properties with their new values.
 List<Prop> get props {
  if (_props is EqualUnmodifiableListView) return _props;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_props);
}


/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SetPropsCopyWith<Command_SetProps> get copyWith => _$Command_SetPropsCopyWithImpl<Command_SetProps>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SetProps&&const DeepCollectionEquality().equals(other.cells, _cells)&&const DeepCollectionEquality().equals(other.props, _props));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_cells),const DeepCollectionEquality().hash(_props));
}

@override
String toString() {
    return 'Command.setProps(cells: $cells, props: $props)';
}


}

/// @nodoc
abstract mixin class $Command_SetPropsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SetPropsCopyWith(Command_SetProps value, $Res Function(Command_SetProps) _then) = _$Command_SetPropsCopyWithImpl;
@useResult
$Res call({
 List<String> cells, List<Prop> props
});




}
/// @nodoc
class _$Command_SetPropsCopyWithImpl<$Res>
    implements $Command_SetPropsCopyWith<$Res> {
  _$Command_SetPropsCopyWithImpl(this._self, this._then);

  final Command_SetProps _self;
  final $Res Function(Command_SetProps) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cells = null,Object? props = null,}) {
  return _then(Command_SetProps(
cells: null == cells ? _self._cells : cells // ignore: cast_nullable_to_non_nullable
as List<String>,props: null == props ? _self._props : props // ignore: cast_nullable_to_non_nullable
as List<Prop>,
  ));
}


}

/// @nodoc


class Command_SetStyle extends Command {
  const Command_SetStyle({required  List<String> cells, required this.set_, required this.clear}): _cells = cells,super._();
  

/// The cells.
 final  List<String> _cells;
/// The cells.
 List<String> get cells {
  if (_cells is EqualUnmodifiableListView) return _cells;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_cells);
}

/// The bits to switch on.
 final  Style set_;
/// The bits to switch off.
 final  Style clear;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SetStyleCopyWith<Command_SetStyle> get copyWith => _$Command_SetStyleCopyWithImpl<Command_SetStyle>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SetStyle&&const DeepCollectionEquality().equals(other.cells, _cells)&&(identical(other.set_, set_) || other.set_ == set_)&&(identical(other.clear, clear) || other.clear == clear));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_cells),set_,clear);
}

@override
String toString() {
    return 'Command.setStyle(cells: $cells, set_: $set_, clear: $clear)';
}


}

/// @nodoc
abstract mixin class $Command_SetStyleCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SetStyleCopyWith(Command_SetStyle value, $Res Function(Command_SetStyle) _then) = _$Command_SetStyleCopyWithImpl;
@useResult
$Res call({
 List<String> cells, Style set_, Style clear
});




}
/// @nodoc
class _$Command_SetStyleCopyWithImpl<$Res>
    implements $Command_SetStyleCopyWith<$Res> {
  _$Command_SetStyleCopyWithImpl(this._self, this._then);

  final Command_SetStyle _self;
  final $Res Function(Command_SetStyle) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cells = null,Object? set_ = null,Object? clear = null,}) {
  return _then(Command_SetStyle(
cells: null == cells ? _self._cells : cells // ignore: cast_nullable_to_non_nullable
as List<String>,set_: null == set_ ? _self.set_ : set_ // ignore: cast_nullable_to_non_nullable
as Style,clear: null == clear ? _self.clear : clear // ignore: cast_nullable_to_non_nullable
as Style,
  ));
}


}

/// @nodoc


class Command_SetColumnWidth extends Command {
  const Command_SetColumnWidth({required this.cell, required this.column, required this.width}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The column.
 final  int column;
/// The width.
 final  int width;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SetColumnWidthCopyWith<Command_SetColumnWidth> get copyWith => _$Command_SetColumnWidthCopyWithImpl<Command_SetColumnWidth>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SetColumnWidth&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.column, column) || other.column == column)&&(identical(other.width, width) || other.width == width));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,column,width);
}

@override
String toString() {
    return 'Command.setColumnWidth(cell: $cell, column: $column, width: $width)';
}


}

/// @nodoc
abstract mixin class $Command_SetColumnWidthCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SetColumnWidthCopyWith(Command_SetColumnWidth value, $Res Function(Command_SetColumnWidth) _then) = _$Command_SetColumnWidthCopyWithImpl;
@useResult
$Res call({
 String cell, int column, int width
});




}
/// @nodoc
class _$Command_SetColumnWidthCopyWithImpl<$Res>
    implements $Command_SetColumnWidthCopyWith<$Res> {
  _$Command_SetColumnWidthCopyWithImpl(this._self, this._then);

  final Command_SetColumnWidth _self;
  final $Res Function(Command_SetColumnWidth) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? column = null,Object? width = null,}) {
  return _then(Command_SetColumnWidth(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,column: null == column ? _self.column : column // ignore: cast_nullable_to_non_nullable
as int,width: null == width ? _self.width : width // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_CreateGrid extends Command {
  const Command_CreateGrid({required this.cell, required this.xs, required this.ys}): super._();
  

/// The cell.
 final  String cell;
/// The number of columns.
 final  int xs;
/// The number of rows.
 final  int ys;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_CreateGridCopyWith<Command_CreateGrid> get copyWith => _$Command_CreateGridCopyWithImpl<Command_CreateGrid>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_CreateGrid&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.xs, xs) || other.xs == xs)&&(identical(other.ys, ys) || other.ys == ys));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,xs,ys);
}

@override
String toString() {
    return 'Command.createGrid(cell: $cell, xs: $xs, ys: $ys)';
}


}

/// @nodoc
abstract mixin class $Command_CreateGridCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_CreateGridCopyWith(Command_CreateGrid value, $Res Function(Command_CreateGrid) _then) = _$Command_CreateGridCopyWithImpl;
@useResult
$Res call({
 String cell, int xs, int ys
});




}
/// @nodoc
class _$Command_CreateGridCopyWithImpl<$Res>
    implements $Command_CreateGridCopyWith<$Res> {
  _$Command_CreateGridCopyWithImpl(this._self, this._then);

  final Command_CreateGrid _self;
  final $Res Function(Command_CreateGrid) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? xs = null,Object? ys = null,}) {
  return _then(Command_CreateGrid(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,xs: null == xs ? _self.xs : xs // ignore: cast_nullable_to_non_nullable
as int,ys: null == ys ? _self.ys : ys // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_RemoveGrid extends Command {
  const Command_RemoveGrid({required this.cell}): super._();
  

/// The cell.
 final  String cell;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_RemoveGridCopyWith<Command_RemoveGrid> get copyWith => _$Command_RemoveGridCopyWithImpl<Command_RemoveGrid>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_RemoveGrid&&(identical(other.cell, cell) || other.cell == cell));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell);
}

@override
String toString() {
    return 'Command.removeGrid(cell: $cell)';
}


}

/// @nodoc
abstract mixin class $Command_RemoveGridCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_RemoveGridCopyWith(Command_RemoveGrid value, $Res Function(Command_RemoveGrid) _then) = _$Command_RemoveGridCopyWithImpl;
@useResult
$Res call({
 String cell
});




}
/// @nodoc
class _$Command_RemoveGridCopyWithImpl<$Res>
    implements $Command_RemoveGridCopyWith<$Res> {
  _$Command_RemoveGridCopyWithImpl(this._self, this._then);

  final Command_RemoveGrid _self;
  final $Res Function(Command_RemoveGrid) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,}) {
  return _then(Command_RemoveGrid(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_InsertRows extends Command {
  const Command_InsertRows({required this.cell, required this.at, required this.count}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The row before which to insert; the number of rows appends.
 final  int at;
/// How many rows.
 final  int count;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_InsertRowsCopyWith<Command_InsertRows> get copyWith => _$Command_InsertRowsCopyWithImpl<Command_InsertRows>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_InsertRows&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.at, at) || other.at == at)&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,at,count);
}

@override
String toString() {
    return 'Command.insertRows(cell: $cell, at: $at, count: $count)';
}


}

/// @nodoc
abstract mixin class $Command_InsertRowsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_InsertRowsCopyWith(Command_InsertRows value, $Res Function(Command_InsertRows) _then) = _$Command_InsertRowsCopyWithImpl;
@useResult
$Res call({
 String cell, int at, int count
});




}
/// @nodoc
class _$Command_InsertRowsCopyWithImpl<$Res>
    implements $Command_InsertRowsCopyWith<$Res> {
  _$Command_InsertRowsCopyWithImpl(this._self, this._then);

  final Command_InsertRows _self;
  final $Res Function(Command_InsertRows) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? at = null,Object? count = null,}) {
  return _then(Command_InsertRows(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_InsertColumns extends Command {
  const Command_InsertColumns({required this.cell, required this.at, required this.count}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The column before which to insert; the number of columns appends.
 final  int at;
/// How many columns.
 final  int count;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_InsertColumnsCopyWith<Command_InsertColumns> get copyWith => _$Command_InsertColumnsCopyWithImpl<Command_InsertColumns>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_InsertColumns&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.at, at) || other.at == at)&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,at,count);
}

@override
String toString() {
    return 'Command.insertColumns(cell: $cell, at: $at, count: $count)';
}


}

/// @nodoc
abstract mixin class $Command_InsertColumnsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_InsertColumnsCopyWith(Command_InsertColumns value, $Res Function(Command_InsertColumns) _then) = _$Command_InsertColumnsCopyWithImpl;
@useResult
$Res call({
 String cell, int at, int count
});




}
/// @nodoc
class _$Command_InsertColumnsCopyWithImpl<$Res>
    implements $Command_InsertColumnsCopyWith<$Res> {
  _$Command_InsertColumnsCopyWithImpl(this._self, this._then);

  final Command_InsertColumns _self;
  final $Res Function(Command_InsertColumns) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? at = null,Object? count = null,}) {
  return _then(Command_InsertColumns(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_DeleteRows extends Command {
  const Command_DeleteRows({required this.cell, required this.at, required this.count}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The first row.
 final  int at;
/// How many rows.
 final  int count;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_DeleteRowsCopyWith<Command_DeleteRows> get copyWith => _$Command_DeleteRowsCopyWithImpl<Command_DeleteRows>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_DeleteRows&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.at, at) || other.at == at)&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,at,count);
}

@override
String toString() {
    return 'Command.deleteRows(cell: $cell, at: $at, count: $count)';
}


}

/// @nodoc
abstract mixin class $Command_DeleteRowsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_DeleteRowsCopyWith(Command_DeleteRows value, $Res Function(Command_DeleteRows) _then) = _$Command_DeleteRowsCopyWithImpl;
@useResult
$Res call({
 String cell, int at, int count
});




}
/// @nodoc
class _$Command_DeleteRowsCopyWithImpl<$Res>
    implements $Command_DeleteRowsCopyWith<$Res> {
  _$Command_DeleteRowsCopyWithImpl(this._self, this._then);

  final Command_DeleteRows _self;
  final $Res Function(Command_DeleteRows) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? at = null,Object? count = null,}) {
  return _then(Command_DeleteRows(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_DeleteColumns extends Command {
  const Command_DeleteColumns({required this.cell, required this.at, required this.count}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The first column.
 final  int at;
/// How many columns.
 final  int count;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_DeleteColumnsCopyWith<Command_DeleteColumns> get copyWith => _$Command_DeleteColumnsCopyWithImpl<Command_DeleteColumns>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_DeleteColumns&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.at, at) || other.at == at)&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,at,count);
}

@override
String toString() {
    return 'Command.deleteColumns(cell: $cell, at: $at, count: $count)';
}


}

/// @nodoc
abstract mixin class $Command_DeleteColumnsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_DeleteColumnsCopyWith(Command_DeleteColumns value, $Res Function(Command_DeleteColumns) _then) = _$Command_DeleteColumnsCopyWithImpl;
@useResult
$Res call({
 String cell, int at, int count
});




}
/// @nodoc
class _$Command_DeleteColumnsCopyWithImpl<$Res>
    implements $Command_DeleteColumnsCopyWith<$Res> {
  _$Command_DeleteColumnsCopyWithImpl(this._self, this._then);

  final Command_DeleteColumns _self;
  final $Res Function(Command_DeleteColumns) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? at = null,Object? count = null,}) {
  return _then(Command_DeleteColumns(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_ClearCells extends Command {
  const Command_ClearCells({required  List<String> cells}): _cells = cells,super._();
  

/// The cells.
 final  List<String> _cells;
/// The cells.
 List<String> get cells {
  if (_cells is EqualUnmodifiableListView) return _cells;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_cells);
}


/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_ClearCellsCopyWith<Command_ClearCells> get copyWith => _$Command_ClearCellsCopyWithImpl<Command_ClearCells>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_ClearCells&&const DeepCollectionEquality().equals(other.cells, _cells));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_cells));
}

@override
String toString() {
    return 'Command.clearCells(cells: $cells)';
}


}

/// @nodoc
abstract mixin class $Command_ClearCellsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_ClearCellsCopyWith(Command_ClearCells value, $Res Function(Command_ClearCells) _then) = _$Command_ClearCellsCopyWithImpl;
@useResult
$Res call({
 List<String> cells
});




}
/// @nodoc
class _$Command_ClearCellsCopyWithImpl<$Res>
    implements $Command_ClearCellsCopyWith<$Res> {
  _$Command_ClearCellsCopyWithImpl(this._self, this._then);

  final Command_ClearCells _self;
  final $Res Function(Command_ClearCells) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cells = null,}) {
  return _then(Command_ClearCells(
cells: null == cells ? _self._cells : cells // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class Command_DeleteCells extends Command {
  const Command_DeleteCells({required this.cell, required this.x, required this.y, required this.xs, required this.ys}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The first selected column.
 final  int x;
/// The first selected row.
 final  int y;
/// The number of selected columns, at least 1.
 final  int xs;
/// The number of selected rows, at least 1.
 final  int ys;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_DeleteCellsCopyWith<Command_DeleteCells> get copyWith => _$Command_DeleteCellsCopyWithImpl<Command_DeleteCells>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_DeleteCells&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.x, x) || other.x == x)&&(identical(other.y, y) || other.y == y)&&(identical(other.xs, xs) || other.xs == xs)&&(identical(other.ys, ys) || other.ys == ys));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,x,y,xs,ys);
}

@override
String toString() {
    return 'Command.deleteCells(cell: $cell, x: $x, y: $y, xs: $xs, ys: $ys)';
}


}

/// @nodoc
abstract mixin class $Command_DeleteCellsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_DeleteCellsCopyWith(Command_DeleteCells value, $Res Function(Command_DeleteCells) _then) = _$Command_DeleteCellsCopyWithImpl;
@useResult
$Res call({
 String cell, int x, int y, int xs, int ys
});




}
/// @nodoc
class _$Command_DeleteCellsCopyWithImpl<$Res>
    implements $Command_DeleteCellsCopyWith<$Res> {
  _$Command_DeleteCellsCopyWithImpl(this._self, this._then);

  final Command_DeleteCells _self;
  final $Res Function(Command_DeleteCells) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? x = null,Object? y = null,Object? xs = null,Object? ys = null,}) {
  return _then(Command_DeleteCells(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,x: null == x ? _self.x : x // ignore: cast_nullable_to_non_nullable
as int,y: null == y ? _self.y : y // ignore: cast_nullable_to_non_nullable
as int,xs: null == xs ? _self.xs : xs // ignore: cast_nullable_to_non_nullable
as int,ys: null == ys ? _self.ys : ys // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_MoveRows extends Command {
  const Command_MoveRows({required this.cell, required this.from, required this.count, required this.to}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The first row to move.
 final  int from;
/// How many rows.
 final  int count;
/// The index the first moved row has afterwards.
 final  int to;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_MoveRowsCopyWith<Command_MoveRows> get copyWith => _$Command_MoveRowsCopyWithImpl<Command_MoveRows>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_MoveRows&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.from, from) || other.from == from)&&(identical(other.count, count) || other.count == count)&&(identical(other.to, to) || other.to == to));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,from,count,to);
}

@override
String toString() {
    return 'Command.moveRows(cell: $cell, from: $from, count: $count, to: $to)';
}


}

/// @nodoc
abstract mixin class $Command_MoveRowsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_MoveRowsCopyWith(Command_MoveRows value, $Res Function(Command_MoveRows) _then) = _$Command_MoveRowsCopyWithImpl;
@useResult
$Res call({
 String cell, int from, int count, int to
});




}
/// @nodoc
class _$Command_MoveRowsCopyWithImpl<$Res>
    implements $Command_MoveRowsCopyWith<$Res> {
  _$Command_MoveRowsCopyWithImpl(this._self, this._then);

  final Command_MoveRows _self;
  final $Res Function(Command_MoveRows) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? from = null,Object? count = null,Object? to = null,}) {
  return _then(Command_MoveRows(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_MoveColumns extends Command {
  const Command_MoveColumns({required this.cell, required this.from, required this.count, required this.to}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The first column to move.
 final  int from;
/// How many columns.
 final  int count;
/// The index the first moved column has afterwards.
 final  int to;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_MoveColumnsCopyWith<Command_MoveColumns> get copyWith => _$Command_MoveColumnsCopyWithImpl<Command_MoveColumns>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_MoveColumns&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.from, from) || other.from == from)&&(identical(other.count, count) || other.count == count)&&(identical(other.to, to) || other.to == to));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,from,count,to);
}

@override
String toString() {
    return 'Command.moveColumns(cell: $cell, from: $from, count: $count, to: $to)';
}


}

/// @nodoc
abstract mixin class $Command_MoveColumnsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_MoveColumnsCopyWith(Command_MoveColumns value, $Res Function(Command_MoveColumns) _then) = _$Command_MoveColumnsCopyWithImpl;
@useResult
$Res call({
 String cell, int from, int count, int to
});




}
/// @nodoc
class _$Command_MoveColumnsCopyWithImpl<$Res>
    implements $Command_MoveColumnsCopyWith<$Res> {
  _$Command_MoveColumnsCopyWithImpl(this._self, this._then);

  final Command_MoveColumns _self;
  final $Res Function(Command_MoveColumns) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? from = null,Object? count = null,Object? to = null,}) {
  return _then(Command_MoveColumns(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_SwapCells extends Command {
  const Command_SwapCells({required this.a, required this.b}): super._();
  

/// One cell.
 final  String a;
/// The other.
 final  String b;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SwapCellsCopyWith<Command_SwapCells> get copyWith => _$Command_SwapCellsCopyWithImpl<Command_SwapCells>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SwapCells&&(identical(other.a, a) || other.a == a)&&(identical(other.b, b) || other.b == b));
}


@override
int get hashCode {
    return Object.hash(runtimeType,a,b);
}

@override
String toString() {
    return 'Command.swapCells(a: $a, b: $b)';
}


}

/// @nodoc
abstract mixin class $Command_SwapCellsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SwapCellsCopyWith(Command_SwapCells value, $Res Function(Command_SwapCells) _then) = _$Command_SwapCellsCopyWithImpl;
@useResult
$Res call({
 String a, String b
});




}
/// @nodoc
class _$Command_SwapCellsCopyWithImpl<$Res>
    implements $Command_SwapCellsCopyWith<$Res> {
  _$Command_SwapCellsCopyWithImpl(this._self, this._then);

  final Command_SwapCells _self;
  final $Res Function(Command_SwapCells) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? a = null,Object? b = null,}) {
  return _then(Command_SwapCells(
a: null == a ? _self.a : a // ignore: cast_nullable_to_non_nullable
as String,b: null == b ? _self.b : b // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_MoveCell extends Command {
  const Command_MoveCell({required this.from, required this.to}): super._();
  

/// The cell to move.
 final  String from;
/// An empty cell (no text, image or grid) whose place it takes.
 final  String to;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_MoveCellCopyWith<Command_MoveCell> get copyWith => _$Command_MoveCellCopyWithImpl<Command_MoveCell>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_MoveCell&&(identical(other.from, from) || other.from == from)&&(identical(other.to, to) || other.to == to));
}


@override
int get hashCode {
    return Object.hash(runtimeType,from,to);
}

@override
String toString() {
    return 'Command.moveCell(from: $from, to: $to)';
}


}

/// @nodoc
abstract mixin class $Command_MoveCellCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_MoveCellCopyWith(Command_MoveCell value, $Res Function(Command_MoveCell) _then) = _$Command_MoveCellCopyWithImpl;
@useResult
$Res call({
 String from, String to
});




}
/// @nodoc
class _$Command_MoveCellCopyWithImpl<$Res>
    implements $Command_MoveCellCopyWith<$Res> {
  _$Command_MoveCellCopyWithImpl(this._self, this._then);

  final Command_MoveCell _self;
  final $Res Function(Command_MoveCell) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? from = null,Object? to = null,}) {
  return _then(Command_MoveCell(
from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as String,to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_CopyCell extends Command {
  const Command_CopyCell({required this.from, required this.to}): super._();
  

/// The cell to copy.
 final  String from;
/// The cell that becomes the copy.
 final  String to;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_CopyCellCopyWith<Command_CopyCell> get copyWith => _$Command_CopyCellCopyWithImpl<Command_CopyCell>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_CopyCell&&(identical(other.from, from) || other.from == from)&&(identical(other.to, to) || other.to == to));
}


@override
int get hashCode {
    return Object.hash(runtimeType,from,to);
}

@override
String toString() {
    return 'Command.copyCell(from: $from, to: $to)';
}


}

/// @nodoc
abstract mixin class $Command_CopyCellCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_CopyCellCopyWith(Command_CopyCell value, $Res Function(Command_CopyCell) _then) = _$Command_CopyCellCopyWithImpl;
@useResult
$Res call({
 String from, String to
});




}
/// @nodoc
class _$Command_CopyCellCopyWithImpl<$Res>
    implements $Command_CopyCellCopyWith<$Res> {
  _$Command_CopyCellCopyWithImpl(this._self, this._then);

  final Command_CopyCell _self;
  final $Res Function(Command_CopyCell) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? from = null,Object? to = null,}) {
  return _then(Command_CopyCell(
from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as String,to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_SetTag extends Command {
  const Command_SetTag({required this.name, required this.cellColor, required this.textColor}): super._();
  

/// The name; not empty, no control characters.
 final  String name;
/// The background colour of cells with the tag.
 final  Rgb cellColor;
/// The text colour of cells with the tag.
 final  Rgb textColor;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SetTagCopyWith<Command_SetTag> get copyWith => _$Command_SetTagCopyWithImpl<Command_SetTag>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SetTag&&(identical(other.name, name) || other.name == name)&&(identical(other.cellColor, cellColor) || other.cellColor == cellColor)&&(identical(other.textColor, textColor) || other.textColor == textColor));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name,cellColor,textColor);
}

@override
String toString() {
    return 'Command.setTag(name: $name, cellColor: $cellColor, textColor: $textColor)';
}


}

/// @nodoc
abstract mixin class $Command_SetTagCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SetTagCopyWith(Command_SetTag value, $Res Function(Command_SetTag) _then) = _$Command_SetTagCopyWithImpl;
@useResult
$Res call({
 String name, Rgb cellColor, Rgb textColor
});




}
/// @nodoc
class _$Command_SetTagCopyWithImpl<$Res>
    implements $Command_SetTagCopyWith<$Res> {
  _$Command_SetTagCopyWithImpl(this._self, this._then);

  final Command_SetTag _self;
  final $Res Function(Command_SetTag) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,Object? cellColor = null,Object? textColor = null,}) {
  return _then(Command_SetTag(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,cellColor: null == cellColor ? _self.cellColor : cellColor // ignore: cast_nullable_to_non_nullable
as Rgb,textColor: null == textColor ? _self.textColor : textColor // ignore: cast_nullable_to_non_nullable
as Rgb,
  ));
}


}

/// @nodoc


class Command_RemoveTag extends Command {
  const Command_RemoveTag({required this.name}): super._();
  

/// The name.
 final  String name;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_RemoveTagCopyWith<Command_RemoveTag> get copyWith => _$Command_RemoveTagCopyWithImpl<Command_RemoveTag>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_RemoveTag&&(identical(other.name, name) || other.name == name));
}


@override
int get hashCode {
    return Object.hash(runtimeType,name);
}

@override
String toString() {
    return 'Command.removeTag(name: $name)';
}


}

/// @nodoc
abstract mixin class $Command_RemoveTagCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_RemoveTagCopyWith(Command_RemoveTag value, $Res Function(Command_RemoveTag) _then) = _$Command_RemoveTagCopyWithImpl;
@useResult
$Res call({
 String name
});




}
/// @nodoc
class _$Command_RemoveTagCopyWithImpl<$Res>
    implements $Command_RemoveTagCopyWith<$Res> {
  _$Command_RemoveTagCopyWithImpl(this._self, this._then);

  final Command_RemoveTag _self;
  final $Res Function(Command_RemoveTag) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,}) {
  return _then(Command_RemoveTag(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_SortRows extends Command {
  const Command_SortRows({required this.cell, required this.column, required this.row, required this.count, required this.descending}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The column to sort by first.
 final  int column;
/// The first row that takes part.
 final  int row;
/// How many rows take part.
 final  int count;
/// From high to low instead of from low to high.
 final  bool descending;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SortRowsCopyWith<Command_SortRows> get copyWith => _$Command_SortRowsCopyWithImpl<Command_SortRows>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SortRows&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.column, column) || other.column == column)&&(identical(other.row, row) || other.row == row)&&(identical(other.count, count) || other.count == count)&&(identical(other.descending, descending) || other.descending == descending));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,column,row,count,descending);
}

@override
String toString() {
    return 'Command.sortRows(cell: $cell, column: $column, row: $row, count: $count, descending: $descending)';
}


}

/// @nodoc
abstract mixin class $Command_SortRowsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SortRowsCopyWith(Command_SortRows value, $Res Function(Command_SortRows) _then) = _$Command_SortRowsCopyWithImpl;
@useResult
$Res call({
 String cell, int column, int row, int count, bool descending
});




}
/// @nodoc
class _$Command_SortRowsCopyWithImpl<$Res>
    implements $Command_SortRowsCopyWith<$Res> {
  _$Command_SortRowsCopyWithImpl(this._self, this._then);

  final Command_SortRows _self;
  final $Res Function(Command_SortRows) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? column = null,Object? row = null,Object? count = null,Object? descending = null,}) {
  return _then(Command_SortRows(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,column: null == column ? _self.column : column // ignore: cast_nullable_to_non_nullable
as int,row: null == row ? _self.row : row // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,descending: null == descending ? _self.descending : descending // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class Command_Transpose extends Command {
  const Command_Transpose({required  List<String> cells}): _cells = cells,super._();
  

/// The cells whose grids are transposed; those without a grid are passed over.
 final  List<String> _cells;
/// The cells whose grids are transposed; those without a grid are passed over.
 List<String> get cells {
  if (_cells is EqualUnmodifiableListView) return _cells;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_cells);
}


/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_TransposeCopyWith<Command_Transpose> get copyWith => _$Command_TransposeCopyWithImpl<Command_Transpose>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_Transpose&&const DeepCollectionEquality().equals(other.cells, _cells));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_cells));
}

@override
String toString() {
    return 'Command.transpose(cells: $cells)';
}


}

/// @nodoc
abstract mixin class $Command_TransposeCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_TransposeCopyWith(Command_Transpose value, $Res Function(Command_Transpose) _then) = _$Command_TransposeCopyWithImpl;
@useResult
$Res call({
 List<String> cells
});




}
/// @nodoc
class _$Command_TransposeCopyWithImpl<$Res>
    implements $Command_TransposeCopyWith<$Res> {
  _$Command_TransposeCopyWithImpl(this._self, this._then);

  final Command_Transpose _self;
  final $Res Function(Command_Transpose) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cells = null,}) {
  return _then(Command_Transpose(
cells: null == cells ? _self._cells : cells // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class Command_HierarchySwap extends Command {
  const Command_HierarchySwap({required this.cell}): super._();
  

/// The cell whose text is swapped upwards.
 final  String cell;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_HierarchySwapCopyWith<Command_HierarchySwap> get copyWith => _$Command_HierarchySwapCopyWithImpl<Command_HierarchySwap>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_HierarchySwap&&(identical(other.cell, cell) || other.cell == cell));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell);
}

@override
String toString() {
    return 'Command.hierarchySwap(cell: $cell)';
}


}

/// @nodoc
abstract mixin class $Command_HierarchySwapCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_HierarchySwapCopyWith(Command_HierarchySwap value, $Res Function(Command_HierarchySwap) _then) = _$Command_HierarchySwapCopyWithImpl;
@useResult
$Res call({
 String cell
});




}
/// @nodoc
class _$Command_HierarchySwapCopyWithImpl<$Res>
    implements $Command_HierarchySwapCopyWith<$Res> {
  _$Command_HierarchySwapCopyWithImpl(this._self, this._then);

  final Command_HierarchySwap _self;
  final $Res Function(Command_HierarchySwap) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,}) {
  return _then(Command_HierarchySwap(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_Hierarchify extends Command {
  const Command_Hierarchify({required this.cell}): super._();
  

/// The cell that has the table.
 final  String cell;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_HierarchifyCopyWith<Command_Hierarchify> get copyWith => _$Command_HierarchifyCopyWithImpl<Command_Hierarchify>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_Hierarchify&&(identical(other.cell, cell) || other.cell == cell));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell);
}

@override
String toString() {
    return 'Command.hierarchify(cell: $cell)';
}


}

/// @nodoc
abstract mixin class $Command_HierarchifyCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_HierarchifyCopyWith(Command_Hierarchify value, $Res Function(Command_Hierarchify) _then) = _$Command_HierarchifyCopyWithImpl;
@useResult
$Res call({
 String cell
});




}
/// @nodoc
class _$Command_HierarchifyCopyWithImpl<$Res>
    implements $Command_HierarchifyCopyWith<$Res> {
  _$Command_HierarchifyCopyWithImpl(this._self, this._then);

  final Command_Hierarchify _self;
  final $Res Function(Command_Hierarchify) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,}) {
  return _then(Command_Hierarchify(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_Flatten extends Command {
  const Command_Flatten({required this.cell}): super._();
  

/// The cell whose grid is flattened.
 final  String cell;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_FlattenCopyWith<Command_Flatten> get copyWith => _$Command_FlattenCopyWithImpl<Command_Flatten>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_Flatten&&(identical(other.cell, cell) || other.cell == cell));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell);
}

@override
String toString() {
    return 'Command.flatten(cell: $cell)';
}


}

/// @nodoc
abstract mixin class $Command_FlattenCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_FlattenCopyWith(Command_Flatten value, $Res Function(Command_Flatten) _then) = _$Command_FlattenCopyWithImpl;
@useResult
$Res call({
 String cell
});




}
/// @nodoc
class _$Command_FlattenCopyWithImpl<$Res>
    implements $Command_FlattenCopyWith<$Res> {
  _$Command_FlattenCopyWithImpl(this._self, this._then);

  final Command_Flatten _self;
  final $Res Function(Command_Flatten) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,}) {
  return _then(Command_Flatten(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_Wrap extends Command {
  const Command_Wrap({required this.cell, required this.x, required this.y, required this.xs, required this.ys}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The first column of the block.
 final  int x;
/// The first row of the block.
 final  int y;
/// The number of columns, at least 1.
 final  int xs;
/// The number of rows, at least 1.
 final  int ys;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_WrapCopyWith<Command_Wrap> get copyWith => _$Command_WrapCopyWithImpl<Command_Wrap>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_Wrap&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.x, x) || other.x == x)&&(identical(other.y, y) || other.y == y)&&(identical(other.xs, xs) || other.xs == xs)&&(identical(other.ys, ys) || other.ys == ys));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,x,y,xs,ys);
}

@override
String toString() {
    return 'Command.wrap(cell: $cell, x: $x, y: $y, xs: $xs, ys: $ys)';
}


}

/// @nodoc
abstract mixin class $Command_WrapCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_WrapCopyWith(Command_Wrap value, $Res Function(Command_Wrap) _then) = _$Command_WrapCopyWithImpl;
@useResult
$Res call({
 String cell, int x, int y, int xs, int ys
});




}
/// @nodoc
class _$Command_WrapCopyWithImpl<$Res>
    implements $Command_WrapCopyWith<$Res> {
  _$Command_WrapCopyWithImpl(this._self, this._then);

  final Command_Wrap _self;
  final $Res Function(Command_Wrap) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? x = null,Object? y = null,Object? xs = null,Object? ys = null,}) {
  return _then(Command_Wrap(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,x: null == x ? _self.x : x // ignore: cast_nullable_to_non_nullable
as int,y: null == y ? _self.y : y // ignore: cast_nullable_to_non_nullable
as int,xs: null == xs ? _self.xs : xs // ignore: cast_nullable_to_non_nullable
as int,ys: null == ys ? _self.ys : ys // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_Collapse extends Command {
  const Command_Collapse({required this.cell, required this.x, required this.y, required this.xs, required this.ys}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The first column of the block.
 final  int x;
/// The first row of the block.
 final  int y;
/// The number of columns.
 final  int xs;
/// The number of rows.
 final  int ys;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_CollapseCopyWith<Command_Collapse> get copyWith => _$Command_CollapseCopyWithImpl<Command_Collapse>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_Collapse&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.x, x) || other.x == x)&&(identical(other.y, y) || other.y == y)&&(identical(other.xs, xs) || other.xs == xs)&&(identical(other.ys, ys) || other.ys == ys));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,x,y,xs,ys);
}

@override
String toString() {
    return 'Command.collapse(cell: $cell, x: $x, y: $y, xs: $xs, ys: $ys)';
}


}

/// @nodoc
abstract mixin class $Command_CollapseCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_CollapseCopyWith(Command_Collapse value, $Res Function(Command_Collapse) _then) = _$Command_CollapseCopyWithImpl;
@useResult
$Res call({
 String cell, int x, int y, int xs, int ys
});




}
/// @nodoc
class _$Command_CollapseCopyWithImpl<$Res>
    implements $Command_CollapseCopyWith<$Res> {
  _$Command_CollapseCopyWithImpl(this._self, this._then);

  final Command_Collapse _self;
  final $Res Function(Command_Collapse) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? x = null,Object? y = null,Object? xs = null,Object? ys = null,}) {
  return _then(Command_Collapse(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,x: null == x ? _self.x : x // ignore: cast_nullable_to_non_nullable
as int,y: null == y ? _self.y : y // ignore: cast_nullable_to_non_nullable
as int,xs: null == xs ? _self.xs : xs // ignore: cast_nullable_to_non_nullable
as int,ys: null == ys ? _self.ys : ys // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_SplitCell extends Command {
  const Command_SplitCell({required this.cell, required this.at}): super._();
  

/// The cell.
 final  String cell;
/// Where to cut the text: a position in code points, not inside a user-perceived
/// character.
 final  int at;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_SplitCellCopyWith<Command_SplitCell> get copyWith => _$Command_SplitCellCopyWithImpl<Command_SplitCell>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_SplitCell&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.at, at) || other.at == at));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,at);
}

@override
String toString() {
    return 'Command.splitCell(cell: $cell, at: $at)';
}


}

/// @nodoc
abstract mixin class $Command_SplitCellCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_SplitCellCopyWith(Command_SplitCell value, $Res Function(Command_SplitCell) _then) = _$Command_SplitCellCopyWithImpl;
@useResult
$Res call({
 String cell, int at
});




}
/// @nodoc
class _$Command_SplitCellCopyWithImpl<$Res>
    implements $Command_SplitCellCopyWith<$Res> {
  _$Command_SplitCellCopyWithImpl(this._self, this._then);

  final Command_SplitCell _self;
  final $Res Function(Command_SplitCell) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? at = null,}) {
  return _then(Command_SplitCell(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,at: null == at ? _self.at : at // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Command_TagCells extends Command {
  const Command_TagCells({required  List<String> cells}): _cells = cells,super._();
  

/// The cells.
 final  List<String> _cells;
/// The cells.
 List<String> get cells {
  if (_cells is EqualUnmodifiableListView) return _cells;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_cells);
}


/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_TagCellsCopyWith<Command_TagCells> get copyWith => _$Command_TagCellsCopyWithImpl<Command_TagCells>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_TagCells&&const DeepCollectionEquality().equals(other.cells, _cells));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_cells));
}

@override
String toString() {
    return 'Command.tagCells(cells: $cells)';
}


}

/// @nodoc
abstract mixin class $Command_TagCellsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_TagCellsCopyWith(Command_TagCells value, $Res Function(Command_TagCells) _then) = _$Command_TagCellsCopyWithImpl;
@useResult
$Res call({
 List<String> cells
});




}
/// @nodoc
class _$Command_TagCellsCopyWithImpl<$Res>
    implements $Command_TagCellsCopyWith<$Res> {
  _$Command_TagCellsCopyWithImpl(this._self, this._then);

  final Command_TagCells _self;
  final $Res Function(Command_TagCells) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cells = null,}) {
  return _then(Command_TagCells(
cells: null == cells ? _self._cells : cells // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class Command_UntagCells extends Command {
  const Command_UntagCells({required  List<String> cells}): _cells = cells,super._();
  

/// The cells.
 final  List<String> _cells;
/// The cells.
 List<String> get cells {
  if (_cells is EqualUnmodifiableListView) return _cells;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_cells);
}


/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_UntagCellsCopyWith<Command_UntagCells> get copyWith => _$Command_UntagCellsCopyWithImpl<Command_UntagCells>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_UntagCells&&const DeepCollectionEquality().equals(other.cells, _cells));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_cells));
}

@override
String toString() {
    return 'Command.untagCells(cells: $cells)';
}


}

/// @nodoc
abstract mixin class $Command_UntagCellsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_UntagCellsCopyWith(Command_UntagCells value, $Res Function(Command_UntagCells) _then) = _$Command_UntagCellsCopyWithImpl;
@useResult
$Res call({
 List<String> cells
});




}
/// @nodoc
class _$Command_UntagCellsCopyWithImpl<$Res>
    implements $Command_UntagCellsCopyWith<$Res> {
  _$Command_UntagCellsCopyWithImpl(this._self, this._then);

  final Command_UntagCells _self;
  final $Res Function(Command_UntagCells) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cells = null,}) {
  return _then(Command_UntagCells(
cells: null == cells ? _self._cells : cells // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class Command_AdjustRelSize extends Command {
  const Command_AdjustRelSize({required  List<String> cells, required this.delta, required this.recursive}): _cells = cells,super._();
  

/// The cells.
 final  List<String> _cells;
/// The cells.
 List<String> get cells {
  if (_cells is EqualUnmodifiableListView) return _cells;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_cells);
}

/// What to add; TreeSheets adds 1 or -1.
 final  int delta;
/// Also the cells inside the listed ones.
 final  bool recursive;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_AdjustRelSizeCopyWith<Command_AdjustRelSize> get copyWith => _$Command_AdjustRelSizeCopyWithImpl<Command_AdjustRelSize>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_AdjustRelSize&&const DeepCollectionEquality().equals(other.cells, _cells)&&(identical(other.delta, delta) || other.delta == delta)&&(identical(other.recursive, recursive) || other.recursive == recursive));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_cells),delta,recursive);
}

@override
String toString() {
    return 'Command.adjustRelSize(cells: $cells, delta: $delta, recursive: $recursive)';
}


}

/// @nodoc
abstract mixin class $Command_AdjustRelSizeCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_AdjustRelSizeCopyWith(Command_AdjustRelSize value, $Res Function(Command_AdjustRelSize) _then) = _$Command_AdjustRelSizeCopyWithImpl;
@useResult
$Res call({
 List<String> cells, int delta, bool recursive
});




}
/// @nodoc
class _$Command_AdjustRelSizeCopyWithImpl<$Res>
    implements $Command_AdjustRelSizeCopyWith<$Res> {
  _$Command_AdjustRelSizeCopyWithImpl(this._self, this._then);

  final Command_AdjustRelSize _self;
  final $Res Function(Command_AdjustRelSize) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cells = null,Object? delta = null,Object? recursive = null,}) {
  return _then(Command_AdjustRelSize(
cells: null == cells ? _self._cells : cells // ignore: cast_nullable_to_non_nullable
as List<String>,delta: null == delta ? _self.delta : delta // ignore: cast_nullable_to_non_nullable
as int,recursive: null == recursive ? _self.recursive : recursive // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class Command_AdjustColumnWidths extends Command {
  const Command_AdjustColumnWidths({required this.cell, required this.column, required this.count, required this.delta, required this.recursive}): super._();
  

/// The cell that has the grid.
 final  String cell;
/// The first column.
 final  int column;
/// How many columns.
 final  int count;
/// What to add; TreeSheets adds 5 or -5.
 final  int delta;
/// Also every column of every grid inside the cells of those columns.
 final  bool recursive;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_AdjustColumnWidthsCopyWith<Command_AdjustColumnWidths> get copyWith => _$Command_AdjustColumnWidthsCopyWithImpl<Command_AdjustColumnWidths>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_AdjustColumnWidths&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.column, column) || other.column == column)&&(identical(other.count, count) || other.count == count)&&(identical(other.delta, delta) || other.delta == delta)&&(identical(other.recursive, recursive) || other.recursive == recursive));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,column,count,delta,recursive);
}

@override
String toString() {
    return 'Command.adjustColumnWidths(cell: $cell, column: $column, count: $count, delta: $delta, recursive: $recursive)';
}


}

/// @nodoc
abstract mixin class $Command_AdjustColumnWidthsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_AdjustColumnWidthsCopyWith(Command_AdjustColumnWidths value, $Res Function(Command_AdjustColumnWidths) _then) = _$Command_AdjustColumnWidthsCopyWithImpl;
@useResult
$Res call({
 String cell, int column, int count, int delta, bool recursive
});




}
/// @nodoc
class _$Command_AdjustColumnWidthsCopyWithImpl<$Res>
    implements $Command_AdjustColumnWidthsCopyWith<$Res> {
  _$Command_AdjustColumnWidthsCopyWithImpl(this._self, this._then);

  final Command_AdjustColumnWidths _self;
  final $Res Function(Command_AdjustColumnWidths) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? column = null,Object? count = null,Object? delta = null,Object? recursive = null,}) {
  return _then(Command_AdjustColumnWidths(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,column: null == column ? _self.column : column // ignore: cast_nullable_to_non_nullable
as int,count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,delta: null == delta ? _self.delta : delta // ignore: cast_nullable_to_non_nullable
as int,recursive: null == recursive ? _self.recursive : recursive // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class Command_Run extends Command {
  const Command_Run(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_Run);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'Command.run()';
}


}




/// @nodoc


class Command_EnterText extends Command {
  const Command_EnterText({required this.cell, required this.text}): super._();
  

/// The cell.
 final  String cell;
/// The text typed; control characters become spaces, and half of a surrogate
/// pair on its own arrives as U+FFFD.
 final  String text;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_EnterTextCopyWith<Command_EnterText> get copyWith => _$Command_EnterTextCopyWithImpl<Command_EnterText>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_EnterText&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,text);
}

@override
String toString() {
    return 'Command.enterText(cell: $cell, text: $text)';
}


}

/// @nodoc
abstract mixin class $Command_EnterTextCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_EnterTextCopyWith(Command_EnterText value, $Res Function(Command_EnterText) _then) = _$Command_EnterTextCopyWithImpl;
@useResult
$Res call({
 String cell, String text
});




}
/// @nodoc
class _$Command_EnterTextCopyWithImpl<$Res>
    implements $Command_EnterTextCopyWith<$Res> {
  _$Command_EnterTextCopyWithImpl(this._self, this._then);

  final Command_EnterText _self;
  final $Res Function(Command_EnterText) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? text = null,}) {
  return _then(Command_EnterText(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_ToggleCheck extends Command {
  const Command_ToggleCheck({required  List<String> cells}): _cells = cells,super._();
  

/// The cells.
 final  List<String> _cells;
/// The cells.
 List<String> get cells {
  if (_cells is EqualUnmodifiableListView) return _cells;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_cells);
}


/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_ToggleCheckCopyWith<Command_ToggleCheck> get copyWith => _$Command_ToggleCheckCopyWithImpl<Command_ToggleCheck>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_ToggleCheck&&const DeepCollectionEquality().equals(other.cells, _cells));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_cells));
}

@override
String toString() {
    return 'Command.toggleCheck(cells: $cells)';
}


}

/// @nodoc
abstract mixin class $Command_ToggleCheckCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_ToggleCheckCopyWith(Command_ToggleCheck value, $Res Function(Command_ToggleCheck) _then) = _$Command_ToggleCheckCopyWithImpl;
@useResult
$Res call({
 List<String> cells
});




}
/// @nodoc
class _$Command_ToggleCheckCopyWithImpl<$Res>
    implements $Command_ToggleCheckCopyWith<$Res> {
  _$Command_ToggleCheckCopyWithImpl(this._self, this._then);

  final Command_ToggleCheck _self;
  final $Res Function(Command_ToggleCheck) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cells = null,}) {
  return _then(Command_ToggleCheck(
cells: null == cells ? _self._cells : cells // ignore: cast_nullable_to_non_nullable
as List<String>,
  ));
}


}

/// @nodoc


class Command_PasteCells extends Command {
  const Command_PasteCells({required this.to, required this.clip}): super._();
  

/// The cell that becomes the copy.
 final  String to;
/// What to paste.
 final  ClipData clip;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_PasteCellsCopyWith<Command_PasteCells> get copyWith => _$Command_PasteCellsCopyWithImpl<Command_PasteCells>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_PasteCells&&(identical(other.to, to) || other.to == to)&&(identical(other.clip, clip) || other.clip == clip));
}


@override
int get hashCode {
    return Object.hash(runtimeType,to,clip);
}

@override
String toString() {
    return 'Command.pasteCells(to: $to, clip: $clip)';
}


}

/// @nodoc
abstract mixin class $Command_PasteCellsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_PasteCellsCopyWith(Command_PasteCells value, $Res Function(Command_PasteCells) _then) = _$Command_PasteCellsCopyWithImpl;
@useResult
$Res call({
 String to, ClipData clip
});




}
/// @nodoc
class _$Command_PasteCellsCopyWithImpl<$Res>
    implements $Command_PasteCellsCopyWith<$Res> {
  _$Command_PasteCellsCopyWithImpl(this._self, this._then);

  final Command_PasteCells _self;
  final $Res Function(Command_PasteCells) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? to = null,Object? clip = null,}) {
  return _then(Command_PasteCells(
to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as String,clip: null == clip ? _self.clip : clip // ignore: cast_nullable_to_non_nullable
as ClipData,
  ));
}


}

/// @nodoc


class Command_AdoptCells extends Command {
  const Command_AdoptCells({required this.to, required this.clip}): super._();
  

/// An empty cell (no text, image or grid) whose place the top cell takes.
 final  String to;
/// The cells that move in.
 final  ClipData clip;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_AdoptCellsCopyWith<Command_AdoptCells> get copyWith => _$Command_AdoptCellsCopyWithImpl<Command_AdoptCells>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_AdoptCells&&(identical(other.to, to) || other.to == to)&&(identical(other.clip, clip) || other.clip == clip));
}


@override
int get hashCode {
    return Object.hash(runtimeType,to,clip);
}

@override
String toString() {
    return 'Command.adoptCells(to: $to, clip: $clip)';
}


}

/// @nodoc
abstract mixin class $Command_AdoptCellsCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_AdoptCellsCopyWith(Command_AdoptCells value, $Res Function(Command_AdoptCells) _then) = _$Command_AdoptCellsCopyWithImpl;
@useResult
$Res call({
 String to, ClipData clip
});




}
/// @nodoc
class _$Command_AdoptCellsCopyWithImpl<$Res>
    implements $Command_AdoptCellsCopyWith<$Res> {
  _$Command_AdoptCellsCopyWithImpl(this._self, this._then);

  final Command_AdoptCells _self;
  final $Res Function(Command_AdoptCells) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? to = null,Object? clip = null,}) {
  return _then(Command_AdoptCells(
to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as String,clip: null == clip ? _self.clip : clip // ignore: cast_nullable_to_non_nullable
as ClipData,
  ));
}


}

/// @nodoc


class Command_ReleaseCell extends Command {
  const Command_ReleaseCell({required this.cell}): super._();
  

/// The cell that leaves.
 final  String cell;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_ReleaseCellCopyWith<Command_ReleaseCell> get copyWith => _$Command_ReleaseCellCopyWithImpl<Command_ReleaseCell>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_ReleaseCell&&(identical(other.cell, cell) || other.cell == cell));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell);
}

@override
String toString() {
    return 'Command.releaseCell(cell: $cell)';
}


}

/// @nodoc
abstract mixin class $Command_ReleaseCellCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_ReleaseCellCopyWith(Command_ReleaseCell value, $Res Function(Command_ReleaseCell) _then) = _$Command_ReleaseCellCopyWithImpl;
@useResult
$Res call({
 String cell
});




}
/// @nodoc
class _$Command_ReleaseCellCopyWithImpl<$Res>
    implements $Command_ReleaseCellCopyWith<$Res> {
  _$Command_ReleaseCellCopyWithImpl(this._self, this._then);

  final Command_ReleaseCell _self;
  final $Res Function(Command_ReleaseCell) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,}) {
  return _then(Command_ReleaseCell(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Command_Import extends Command {
  const Command_Import({required this.cell, required this.table, required this.replace}): super._();
  

/// The cell that gets the grid.
 final  String cell;
/// What to import.
 final  TableData table;
/// Whether a grid with content may be replaced.
 final  bool replace;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_ImportCopyWith<Command_Import> get copyWith => _$Command_ImportCopyWithImpl<Command_Import>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_Import&&(identical(other.cell, cell) || other.cell == cell)&&(identical(other.table, table) || other.table == table)&&(identical(other.replace, replace) || other.replace == replace));
}


@override
int get hashCode {
    return Object.hash(runtimeType,cell,table,replace);
}

@override
String toString() {
    return 'Command.import_(cell: $cell, table: $table, replace: $replace)';
}


}

/// @nodoc
abstract mixin class $Command_ImportCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_ImportCopyWith(Command_Import value, $Res Function(Command_Import) _then) = _$Command_ImportCopyWithImpl;
@useResult
$Res call({
 String cell, TableData table, bool replace
});




}
/// @nodoc
class _$Command_ImportCopyWithImpl<$Res>
    implements $Command_ImportCopyWith<$Res> {
  _$Command_ImportCopyWithImpl(this._self, this._then);

  final Command_Import _self;
  final $Res Function(Command_Import) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cell = null,Object? table = null,Object? replace = null,}) {
  return _then(Command_Import(
cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,table: null == table ? _self.table : table // ignore: cast_nullable_to_non_nullable
as TableData,replace: null == replace ? _self.replace : replace // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class Command_RetargetLinks extends Command {
  const Command_RetargetLinks({required  List<String> cells, this.from, this.to}): _cells = cells,super._();
  

/// The cells that the links point at.
 final  List<String> _cells;
/// The cells that the links point at.
 List<String> get cells {
  if (_cells is EqualUnmodifiableListView) return _cells;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_cells);
}

/// The document that the links to correct name.
 final  String? from;
/// The document that they are to name.
 final  String? to;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Command_RetargetLinksCopyWith<Command_RetargetLinks> get copyWith => _$Command_RetargetLinksCopyWithImpl<Command_RetargetLinks>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Command_RetargetLinks&&const DeepCollectionEquality().equals(other.cells, _cells)&&(identical(other.from, from) || other.from == from)&&(identical(other.to, to) || other.to == to));
}


@override
int get hashCode {
    return Object.hash(runtimeType,const DeepCollectionEquality().hash(_cells),from,to);
}

@override
String toString() {
    return 'Command.retargetLinks(cells: $cells, from: $from, to: $to)';
}


}

/// @nodoc
abstract mixin class $Command_RetargetLinksCopyWith<$Res> implements $CommandCopyWith<$Res> {
  factory $Command_RetargetLinksCopyWith(Command_RetargetLinks value, $Res Function(Command_RetargetLinks) _then) = _$Command_RetargetLinksCopyWithImpl;
@useResult
$Res call({
 List<String> cells, String? from, String? to
});




}
/// @nodoc
class _$Command_RetargetLinksCopyWithImpl<$Res>
    implements $Command_RetargetLinksCopyWith<$Res> {
  _$Command_RetargetLinksCopyWithImpl(this._self, this._then);

  final Command_RetargetLinks _self;
  final $Res Function(Command_RetargetLinks) _then;

/// Create a copy of Command
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cells = null,Object? from = freezed,Object? to = freezed,}) {
  return _then(Command_RetargetLinks(
cells: null == cells ? _self._cells : cells // ignore: cast_nullable_to_non_nullable
as List<String>,from: freezed == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as String?,to: freezed == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc
mixin _$LinkTarget {





@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkTarget);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkTarget()';
}


}

/// @nodoc
class $LinkTargetCopyWith<$Res>  {
$LinkTargetCopyWith(LinkTarget _, $Res Function(LinkTarget) __);
}


/// Adds pattern-matching-related methods to [LinkTarget].
extension LinkTargetPatterns on LinkTarget {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( LinkTarget_External value)?  external_,TResult Function( LinkTarget_Cell value)?  cell,TResult Function( LinkTarget_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case LinkTarget_External() when external_ != null:
return external_(_that);case LinkTarget_Cell() when cell != null:
return cell(_that);case LinkTarget_Unknown() when unknown != null:
return unknown(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( LinkTarget_External value)  external_,required TResult Function( LinkTarget_Cell value)  cell,required TResult Function( LinkTarget_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case LinkTarget_External():
return external_(_that);case LinkTarget_Cell():
return cell(_that);case LinkTarget_Unknown():
return unknown(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( LinkTarget_External value)?  external_,TResult? Function( LinkTarget_Cell value)?  cell,TResult? Function( LinkTarget_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case LinkTarget_External() when external_ != null:
return external_(_that);case LinkTarget_Cell() when cell != null:
return cell(_that);case LinkTarget_Unknown() when unknown != null:
return unknown(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String target)?  external_,TResult Function( String? doc,  String cell)?  cell,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case LinkTarget_External() when external_ != null:
return external_(_that.target);case LinkTarget_Cell() when cell != null:
return cell(_that.doc,_that.cell);case LinkTarget_Unknown() when unknown != null:
return unknown();case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String target)  external_,required TResult Function( String? doc,  String cell)  cell,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case LinkTarget_External():
return external_(_that.target);case LinkTarget_Cell():
return cell(_that.doc,_that.cell);case LinkTarget_Unknown():
return unknown();}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String target)?  external_,TResult? Function( String? doc,  String cell)?  cell,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case LinkTarget_External() when external_ != null:
return external_(_that.target);case LinkTarget_Cell() when cell != null:
return cell(_that.doc,_that.cell);case LinkTarget_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class LinkTarget_External extends LinkTarget {
  const LinkTarget_External({required this.target}): super._();
  

/// The URL or path; not empty.
 final  String target;

/// Create a copy of LinkTarget
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LinkTarget_ExternalCopyWith<LinkTarget_External> get copyWith => _$LinkTarget_ExternalCopyWithImpl<LinkTarget_External>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkTarget_External&&(identical(other.target, target) || other.target == target));
}


@override
int get hashCode {
    return Object.hash(runtimeType,target);
}

@override
String toString() {
    return 'LinkTarget.external_(target: $target)';
}


}

/// @nodoc
abstract mixin class $LinkTarget_ExternalCopyWith<$Res> implements $LinkTargetCopyWith<$Res> {
  factory $LinkTarget_ExternalCopyWith(LinkTarget_External value, $Res Function(LinkTarget_External) _then) = _$LinkTarget_ExternalCopyWithImpl;
@useResult
$Res call({
 String target
});




}
/// @nodoc
class _$LinkTarget_ExternalCopyWithImpl<$Res>
    implements $LinkTarget_ExternalCopyWith<$Res> {
  _$LinkTarget_ExternalCopyWithImpl(this._self, this._then);

  final LinkTarget_External _self;
  final $Res Function(LinkTarget_External) _then;

/// Create a copy of LinkTarget
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? target = null,}) {
  return _then(LinkTarget_External(
target: null == target ? _self.target : target // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LinkTarget_Cell extends LinkTarget {
  const LinkTarget_Cell({this.doc, required this.cell}): super._();
  

/// The ID of the document the cell is in, or `None` for the document that holds
/// the link. The ID of that document itself is read as `None`, so that a link to
/// one cell has one form.
 final  String? doc;
/// The ID of the cell.
 final  String cell;

/// Create a copy of LinkTarget
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$LinkTarget_CellCopyWith<LinkTarget_Cell> get copyWith => _$LinkTarget_CellCopyWithImpl<LinkTarget_Cell>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkTarget_Cell&&(identical(other.doc, doc) || other.doc == doc)&&(identical(other.cell, cell) || other.cell == cell));
}


@override
int get hashCode {
    return Object.hash(runtimeType,doc,cell);
}

@override
String toString() {
    return 'LinkTarget.cell(doc: $doc, cell: $cell)';
}


}

/// @nodoc
abstract mixin class $LinkTarget_CellCopyWith<$Res> implements $LinkTargetCopyWith<$Res> {
  factory $LinkTarget_CellCopyWith(LinkTarget_Cell value, $Res Function(LinkTarget_Cell) _then) = _$LinkTarget_CellCopyWithImpl;
@useResult
$Res call({
 String? doc, String cell
});




}
/// @nodoc
class _$LinkTarget_CellCopyWithImpl<$Res>
    implements $LinkTarget_CellCopyWith<$Res> {
  _$LinkTarget_CellCopyWithImpl(this._self, this._then);

  final LinkTarget_Cell _self;
  final $Res Function(LinkTarget_Cell) _then;

/// Create a copy of LinkTarget
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? doc = freezed,Object? cell = null,}) {
  return _then(LinkTarget_Cell(
doc: freezed == doc ? _self.doc : doc // ignore: cast_nullable_to_non_nullable
as String?,cell: null == cell ? _self.cell : cell // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class LinkTarget_Unknown extends LinkTarget {
  const LinkTarget_Unknown(): super._();
  






@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is LinkTarget_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
    return 'LinkTarget.unknown()';
}


}




/// @nodoc
mixin _$Prop {

 Object? get field0;



@override
bool operator ==(Object other) {
  final _this = this as Prop;
  return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop&&const DeepCollectionEquality().equals(other.field0, _this.field0));
}


@override
int get hashCode {
  final _this = this as Prop;
  return Object.hash(runtimeType,const DeepCollectionEquality().hash(_this.field0));
}

@override
String toString() {
  final _this = this as Prop;
  return 'Prop(field0: ${_this.field0})';
}


}

/// @nodoc
class $PropCopyWith<$Res>  {
$PropCopyWith(Prop _, $Res Function(Prop) __);
}


/// Adds pattern-matching-related methods to [Prop].
extension PropPatterns on Prop {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( Prop_CellColor value)?  cellColor,TResult Function( Prop_TextColor value)?  textColor,TResult Function( Prop_RelSize value)?  relSize,TResult Function( Prop_TextAlign value)?  textAlign,TResult Function( Prop_VertAlign value)?  vertAlign,TResult Function( Prop_Kind value)?  kind,TResult Function( Prop_Note value)?  note,TResult Function( Prop_DrawStyle value)?  drawStyle,TResult Function( Prop_Image value)?  image,TResult Function( Prop_ImageScale value)?  imageScale,TResult Function( Prop_LastEdit value)?  lastEdit,TResult Function( Prop_BorderColor value)?  borderColor,TResult Function( Prop_OuterSpacing value)?  outerSpacing,TResult Function( Prop_Vertical value)?  vertical,required TResult orElse(),}){
final _that = this;
switch (_that) {
case Prop_CellColor() when cellColor != null:
return cellColor(_that);case Prop_TextColor() when textColor != null:
return textColor(_that);case Prop_RelSize() when relSize != null:
return relSize(_that);case Prop_TextAlign() when textAlign != null:
return textAlign(_that);case Prop_VertAlign() when vertAlign != null:
return vertAlign(_that);case Prop_Kind() when kind != null:
return kind(_that);case Prop_Note() when note != null:
return note(_that);case Prop_DrawStyle() when drawStyle != null:
return drawStyle(_that);case Prop_Image() when image != null:
return image(_that);case Prop_ImageScale() when imageScale != null:
return imageScale(_that);case Prop_LastEdit() when lastEdit != null:
return lastEdit(_that);case Prop_BorderColor() when borderColor != null:
return borderColor(_that);case Prop_OuterSpacing() when outerSpacing != null:
return outerSpacing(_that);case Prop_Vertical() when vertical != null:
return vertical(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( Prop_CellColor value)  cellColor,required TResult Function( Prop_TextColor value)  textColor,required TResult Function( Prop_RelSize value)  relSize,required TResult Function( Prop_TextAlign value)  textAlign,required TResult Function( Prop_VertAlign value)  vertAlign,required TResult Function( Prop_Kind value)  kind,required TResult Function( Prop_Note value)  note,required TResult Function( Prop_DrawStyle value)  drawStyle,required TResult Function( Prop_Image value)  image,required TResult Function( Prop_ImageScale value)  imageScale,required TResult Function( Prop_LastEdit value)  lastEdit,required TResult Function( Prop_BorderColor value)  borderColor,required TResult Function( Prop_OuterSpacing value)  outerSpacing,required TResult Function( Prop_Vertical value)  vertical,}){
final _that = this;
switch (_that) {
case Prop_CellColor():
return cellColor(_that);case Prop_TextColor():
return textColor(_that);case Prop_RelSize():
return relSize(_that);case Prop_TextAlign():
return textAlign(_that);case Prop_VertAlign():
return vertAlign(_that);case Prop_Kind():
return kind(_that);case Prop_Note():
return note(_that);case Prop_DrawStyle():
return drawStyle(_that);case Prop_Image():
return image(_that);case Prop_ImageScale():
return imageScale(_that);case Prop_LastEdit():
return lastEdit(_that);case Prop_BorderColor():
return borderColor(_that);case Prop_OuterSpacing():
return outerSpacing(_that);case Prop_Vertical():
return vertical(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( Prop_CellColor value)?  cellColor,TResult? Function( Prop_TextColor value)?  textColor,TResult? Function( Prop_RelSize value)?  relSize,TResult? Function( Prop_TextAlign value)?  textAlign,TResult? Function( Prop_VertAlign value)?  vertAlign,TResult? Function( Prop_Kind value)?  kind,TResult? Function( Prop_Note value)?  note,TResult? Function( Prop_DrawStyle value)?  drawStyle,TResult? Function( Prop_Image value)?  image,TResult? Function( Prop_ImageScale value)?  imageScale,TResult? Function( Prop_LastEdit value)?  lastEdit,TResult? Function( Prop_BorderColor value)?  borderColor,TResult? Function( Prop_OuterSpacing value)?  outerSpacing,TResult? Function( Prop_Vertical value)?  vertical,}){
final _that = this;
switch (_that) {
case Prop_CellColor() when cellColor != null:
return cellColor(_that);case Prop_TextColor() when textColor != null:
return textColor(_that);case Prop_RelSize() when relSize != null:
return relSize(_that);case Prop_TextAlign() when textAlign != null:
return textAlign(_that);case Prop_VertAlign() when vertAlign != null:
return vertAlign(_that);case Prop_Kind() when kind != null:
return kind(_that);case Prop_Note() when note != null:
return note(_that);case Prop_DrawStyle() when drawStyle != null:
return drawStyle(_that);case Prop_Image() when image != null:
return image(_that);case Prop_ImageScale() when imageScale != null:
return imageScale(_that);case Prop_LastEdit() when lastEdit != null:
return lastEdit(_that);case Prop_BorderColor() when borderColor != null:
return borderColor(_that);case Prop_OuterSpacing() when outerSpacing != null:
return outerSpacing(_that);case Prop_Vertical() when vertical != null:
return vertical(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( Rgb field0)?  cellColor,TResult Function( Rgb field0)?  textColor,TResult Function( int field0)?  relSize,TResult Function( HorizAlign field0)?  textAlign,TResult Function( VertAlign field0)?  vertAlign,TResult Function( CellKind field0)?  kind,TResult Function( String field0)?  note,TResult Function( DrawStyle field0)?  drawStyle,TResult Function( ImageRef? field0)?  image,TResult Function( double field0)?  imageScale,TResult Function( int? field0)?  lastEdit,TResult Function( Rgb field0)?  borderColor,TResult Function( int field0)?  outerSpacing,TResult Function( bool field0)?  vertical,required TResult orElse(),}) {final _that = this;
switch (_that) {
case Prop_CellColor() when cellColor != null:
return cellColor(_that.field0);case Prop_TextColor() when textColor != null:
return textColor(_that.field0);case Prop_RelSize() when relSize != null:
return relSize(_that.field0);case Prop_TextAlign() when textAlign != null:
return textAlign(_that.field0);case Prop_VertAlign() when vertAlign != null:
return vertAlign(_that.field0);case Prop_Kind() when kind != null:
return kind(_that.field0);case Prop_Note() when note != null:
return note(_that.field0);case Prop_DrawStyle() when drawStyle != null:
return drawStyle(_that.field0);case Prop_Image() when image != null:
return image(_that.field0);case Prop_ImageScale() when imageScale != null:
return imageScale(_that.field0);case Prop_LastEdit() when lastEdit != null:
return lastEdit(_that.field0);case Prop_BorderColor() when borderColor != null:
return borderColor(_that.field0);case Prop_OuterSpacing() when outerSpacing != null:
return outerSpacing(_that.field0);case Prop_Vertical() when vertical != null:
return vertical(_that.field0);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( Rgb field0)  cellColor,required TResult Function( Rgb field0)  textColor,required TResult Function( int field0)  relSize,required TResult Function( HorizAlign field0)  textAlign,required TResult Function( VertAlign field0)  vertAlign,required TResult Function( CellKind field0)  kind,required TResult Function( String field0)  note,required TResult Function( DrawStyle field0)  drawStyle,required TResult Function( ImageRef? field0)  image,required TResult Function( double field0)  imageScale,required TResult Function( int? field0)  lastEdit,required TResult Function( Rgb field0)  borderColor,required TResult Function( int field0)  outerSpacing,required TResult Function( bool field0)  vertical,}) {final _that = this;
switch (_that) {
case Prop_CellColor():
return cellColor(_that.field0);case Prop_TextColor():
return textColor(_that.field0);case Prop_RelSize():
return relSize(_that.field0);case Prop_TextAlign():
return textAlign(_that.field0);case Prop_VertAlign():
return vertAlign(_that.field0);case Prop_Kind():
return kind(_that.field0);case Prop_Note():
return note(_that.field0);case Prop_DrawStyle():
return drawStyle(_that.field0);case Prop_Image():
return image(_that.field0);case Prop_ImageScale():
return imageScale(_that.field0);case Prop_LastEdit():
return lastEdit(_that.field0);case Prop_BorderColor():
return borderColor(_that.field0);case Prop_OuterSpacing():
return outerSpacing(_that.field0);case Prop_Vertical():
return vertical(_that.field0);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( Rgb field0)?  cellColor,TResult? Function( Rgb field0)?  textColor,TResult? Function( int field0)?  relSize,TResult? Function( HorizAlign field0)?  textAlign,TResult? Function( VertAlign field0)?  vertAlign,TResult? Function( CellKind field0)?  kind,TResult? Function( String field0)?  note,TResult? Function( DrawStyle field0)?  drawStyle,TResult? Function( ImageRef? field0)?  image,TResult? Function( double field0)?  imageScale,TResult? Function( int? field0)?  lastEdit,TResult? Function( Rgb field0)?  borderColor,TResult? Function( int field0)?  outerSpacing,TResult? Function( bool field0)?  vertical,}) {final _that = this;
switch (_that) {
case Prop_CellColor() when cellColor != null:
return cellColor(_that.field0);case Prop_TextColor() when textColor != null:
return textColor(_that.field0);case Prop_RelSize() when relSize != null:
return relSize(_that.field0);case Prop_TextAlign() when textAlign != null:
return textAlign(_that.field0);case Prop_VertAlign() when vertAlign != null:
return vertAlign(_that.field0);case Prop_Kind() when kind != null:
return kind(_that.field0);case Prop_Note() when note != null:
return note(_that.field0);case Prop_DrawStyle() when drawStyle != null:
return drawStyle(_that.field0);case Prop_Image() when image != null:
return image(_that.field0);case Prop_ImageScale() when imageScale != null:
return imageScale(_that.field0);case Prop_LastEdit() when lastEdit != null:
return lastEdit(_that.field0);case Prop_BorderColor() when borderColor != null:
return borderColor(_that.field0);case Prop_OuterSpacing() when outerSpacing != null:
return outerSpacing(_that.field0);case Prop_Vertical() when vertical != null:
return vertical(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class Prop_CellColor extends Prop {
  const Prop_CellColor(this.field0): super._();
  

@override final  Rgb field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_CellColorCopyWith<Prop_CellColor> get copyWith => _$Prop_CellColorCopyWithImpl<Prop_CellColor>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_CellColor&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.cellColor(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_CellColorCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_CellColorCopyWith(Prop_CellColor value, $Res Function(Prop_CellColor) _then) = _$Prop_CellColorCopyWithImpl;
@useResult
$Res call({
 Rgb field0
});




}
/// @nodoc
class _$Prop_CellColorCopyWithImpl<$Res>
    implements $Prop_CellColorCopyWith<$Res> {
  _$Prop_CellColorCopyWithImpl(this._self, this._then);

  final Prop_CellColor _self;
  final $Res Function(Prop_CellColor) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_CellColor(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Rgb,
  ));
}


}

/// @nodoc


class Prop_TextColor extends Prop {
  const Prop_TextColor(this.field0): super._();
  

@override final  Rgb field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_TextColorCopyWith<Prop_TextColor> get copyWith => _$Prop_TextColorCopyWithImpl<Prop_TextColor>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_TextColor&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.textColor(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_TextColorCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_TextColorCopyWith(Prop_TextColor value, $Res Function(Prop_TextColor) _then) = _$Prop_TextColorCopyWithImpl;
@useResult
$Res call({
 Rgb field0
});




}
/// @nodoc
class _$Prop_TextColorCopyWithImpl<$Res>
    implements $Prop_TextColorCopyWith<$Res> {
  _$Prop_TextColorCopyWithImpl(this._self, this._then);

  final Prop_TextColor _self;
  final $Res Function(Prop_TextColor) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_TextColor(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Rgb,
  ));
}


}

/// @nodoc


class Prop_RelSize extends Prop {
  const Prop_RelSize(this.field0): super._();
  

@override final  int field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_RelSizeCopyWith<Prop_RelSize> get copyWith => _$Prop_RelSizeCopyWithImpl<Prop_RelSize>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_RelSize&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.relSize(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_RelSizeCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_RelSizeCopyWith(Prop_RelSize value, $Res Function(Prop_RelSize) _then) = _$Prop_RelSizeCopyWithImpl;
@useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$Prop_RelSizeCopyWithImpl<$Res>
    implements $Prop_RelSizeCopyWith<$Res> {
  _$Prop_RelSizeCopyWithImpl(this._self, this._then);

  final Prop_RelSize _self;
  final $Res Function(Prop_RelSize) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_RelSize(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Prop_TextAlign extends Prop {
  const Prop_TextAlign(this.field0): super._();
  

@override final  HorizAlign field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_TextAlignCopyWith<Prop_TextAlign> get copyWith => _$Prop_TextAlignCopyWithImpl<Prop_TextAlign>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_TextAlign&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.textAlign(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_TextAlignCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_TextAlignCopyWith(Prop_TextAlign value, $Res Function(Prop_TextAlign) _then) = _$Prop_TextAlignCopyWithImpl;
@useResult
$Res call({
 HorizAlign field0
});




}
/// @nodoc
class _$Prop_TextAlignCopyWithImpl<$Res>
    implements $Prop_TextAlignCopyWith<$Res> {
  _$Prop_TextAlignCopyWithImpl(this._self, this._then);

  final Prop_TextAlign _self;
  final $Res Function(Prop_TextAlign) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_TextAlign(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as HorizAlign,
  ));
}


}

/// @nodoc


class Prop_VertAlign extends Prop {
  const Prop_VertAlign(this.field0): super._();
  

@override final  VertAlign field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_VertAlignCopyWith<Prop_VertAlign> get copyWith => _$Prop_VertAlignCopyWithImpl<Prop_VertAlign>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_VertAlign&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.vertAlign(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_VertAlignCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_VertAlignCopyWith(Prop_VertAlign value, $Res Function(Prop_VertAlign) _then) = _$Prop_VertAlignCopyWithImpl;
@useResult
$Res call({
 VertAlign field0
});




}
/// @nodoc
class _$Prop_VertAlignCopyWithImpl<$Res>
    implements $Prop_VertAlignCopyWith<$Res> {
  _$Prop_VertAlignCopyWithImpl(this._self, this._then);

  final Prop_VertAlign _self;
  final $Res Function(Prop_VertAlign) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_VertAlign(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as VertAlign,
  ));
}


}

/// @nodoc


class Prop_Kind extends Prop {
  const Prop_Kind(this.field0): super._();
  

@override final  CellKind field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_KindCopyWith<Prop_Kind> get copyWith => _$Prop_KindCopyWithImpl<Prop_Kind>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_Kind&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.kind(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_KindCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_KindCopyWith(Prop_Kind value, $Res Function(Prop_Kind) _then) = _$Prop_KindCopyWithImpl;
@useResult
$Res call({
 CellKind field0
});




}
/// @nodoc
class _$Prop_KindCopyWithImpl<$Res>
    implements $Prop_KindCopyWith<$Res> {
  _$Prop_KindCopyWithImpl(this._self, this._then);

  final Prop_Kind _self;
  final $Res Function(Prop_Kind) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_Kind(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as CellKind,
  ));
}


}

/// @nodoc


class Prop_Note extends Prop {
  const Prop_Note(this.field0): super._();
  

@override final  String field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_NoteCopyWith<Prop_Note> get copyWith => _$Prop_NoteCopyWithImpl<Prop_Note>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_Note&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.note(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_NoteCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_NoteCopyWith(Prop_Note value, $Res Function(Prop_Note) _then) = _$Prop_NoteCopyWithImpl;
@useResult
$Res call({
 String field0
});




}
/// @nodoc
class _$Prop_NoteCopyWithImpl<$Res>
    implements $Prop_NoteCopyWith<$Res> {
  _$Prop_NoteCopyWithImpl(this._self, this._then);

  final Prop_Note _self;
  final $Res Function(Prop_Note) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_Note(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class Prop_DrawStyle extends Prop {
  const Prop_DrawStyle(this.field0): super._();
  

@override final  DrawStyle field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_DrawStyleCopyWith<Prop_DrawStyle> get copyWith => _$Prop_DrawStyleCopyWithImpl<Prop_DrawStyle>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_DrawStyle&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.drawStyle(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_DrawStyleCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_DrawStyleCopyWith(Prop_DrawStyle value, $Res Function(Prop_DrawStyle) _then) = _$Prop_DrawStyleCopyWithImpl;
@useResult
$Res call({
 DrawStyle field0
});




}
/// @nodoc
class _$Prop_DrawStyleCopyWithImpl<$Res>
    implements $Prop_DrawStyleCopyWith<$Res> {
  _$Prop_DrawStyleCopyWithImpl(this._self, this._then);

  final Prop_DrawStyle _self;
  final $Res Function(Prop_DrawStyle) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_DrawStyle(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as DrawStyle,
  ));
}


}

/// @nodoc


class Prop_Image extends Prop {
  const Prop_Image([this.field0]): super._();
  

@override final  ImageRef? field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_ImageCopyWith<Prop_Image> get copyWith => _$Prop_ImageCopyWithImpl<Prop_Image>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_Image&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.image(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_ImageCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_ImageCopyWith(Prop_Image value, $Res Function(Prop_Image) _then) = _$Prop_ImageCopyWithImpl;
@useResult
$Res call({
 ImageRef? field0
});




}
/// @nodoc
class _$Prop_ImageCopyWithImpl<$Res>
    implements $Prop_ImageCopyWith<$Res> {
  _$Prop_ImageCopyWithImpl(this._self, this._then);

  final Prop_Image _self;
  final $Res Function(Prop_Image) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = freezed,}) {
  return _then(Prop_Image(
freezed == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as ImageRef?,
  ));
}


}

/// @nodoc


class Prop_ImageScale extends Prop {
  const Prop_ImageScale(this.field0): super._();
  

@override final  double field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_ImageScaleCopyWith<Prop_ImageScale> get copyWith => _$Prop_ImageScaleCopyWithImpl<Prop_ImageScale>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_ImageScale&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.imageScale(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_ImageScaleCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_ImageScaleCopyWith(Prop_ImageScale value, $Res Function(Prop_ImageScale) _then) = _$Prop_ImageScaleCopyWithImpl;
@useResult
$Res call({
 double field0
});




}
/// @nodoc
class _$Prop_ImageScaleCopyWithImpl<$Res>
    implements $Prop_ImageScaleCopyWith<$Res> {
  _$Prop_ImageScaleCopyWithImpl(this._self, this._then);

  final Prop_ImageScale _self;
  final $Res Function(Prop_ImageScale) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_ImageScale(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as double,
  ));
}


}

/// @nodoc


class Prop_LastEdit extends Prop {
  const Prop_LastEdit([this.field0]): super._();
  

@override final  int? field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_LastEditCopyWith<Prop_LastEdit> get copyWith => _$Prop_LastEditCopyWithImpl<Prop_LastEdit>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_LastEdit&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.lastEdit(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_LastEditCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_LastEditCopyWith(Prop_LastEdit value, $Res Function(Prop_LastEdit) _then) = _$Prop_LastEditCopyWithImpl;
@useResult
$Res call({
 int? field0
});




}
/// @nodoc
class _$Prop_LastEditCopyWithImpl<$Res>
    implements $Prop_LastEditCopyWith<$Res> {
  _$Prop_LastEditCopyWithImpl(this._self, this._then);

  final Prop_LastEdit _self;
  final $Res Function(Prop_LastEdit) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = freezed,}) {
  return _then(Prop_LastEdit(
freezed == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc


class Prop_BorderColor extends Prop {
  const Prop_BorderColor(this.field0): super._();
  

@override final  Rgb field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_BorderColorCopyWith<Prop_BorderColor> get copyWith => _$Prop_BorderColorCopyWithImpl<Prop_BorderColor>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_BorderColor&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.borderColor(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_BorderColorCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_BorderColorCopyWith(Prop_BorderColor value, $Res Function(Prop_BorderColor) _then) = _$Prop_BorderColorCopyWithImpl;
@useResult
$Res call({
 Rgb field0
});




}
/// @nodoc
class _$Prop_BorderColorCopyWithImpl<$Res>
    implements $Prop_BorderColorCopyWith<$Res> {
  _$Prop_BorderColorCopyWithImpl(this._self, this._then);

  final Prop_BorderColor _self;
  final $Res Function(Prop_BorderColor) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_BorderColor(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Rgb,
  ));
}


}

/// @nodoc


class Prop_OuterSpacing extends Prop {
  const Prop_OuterSpacing(this.field0): super._();
  

@override final  int field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_OuterSpacingCopyWith<Prop_OuterSpacing> get copyWith => _$Prop_OuterSpacingCopyWithImpl<Prop_OuterSpacing>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_OuterSpacing&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.outerSpacing(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_OuterSpacingCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_OuterSpacingCopyWith(Prop_OuterSpacing value, $Res Function(Prop_OuterSpacing) _then) = _$Prop_OuterSpacingCopyWithImpl;
@useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$Prop_OuterSpacingCopyWithImpl<$Res>
    implements $Prop_OuterSpacingCopyWith<$Res> {
  _$Prop_OuterSpacingCopyWithImpl(this._self, this._then);

  final Prop_OuterSpacing _self;
  final $Res Function(Prop_OuterSpacing) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_OuterSpacing(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class Prop_Vertical extends Prop {
  const Prop_Vertical(this.field0): super._();
  

@override final  bool field0;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$Prop_VerticalCopyWith<Prop_Vertical> get copyWith => _$Prop_VerticalCopyWithImpl<Prop_Vertical>(this, _$identity);



@override
bool operator ==(Object other) {
    return identical(this, other) || (other.runtimeType == runtimeType&&other is Prop_Vertical&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode {
    return Object.hash(runtimeType,field0);
}

@override
String toString() {
    return 'Prop.vertical(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $Prop_VerticalCopyWith<$Res> implements $PropCopyWith<$Res> {
  factory $Prop_VerticalCopyWith(Prop_Vertical value, $Res Function(Prop_Vertical) _then) = _$Prop_VerticalCopyWithImpl;
@useResult
$Res call({
 bool field0
});




}
/// @nodoc
class _$Prop_VerticalCopyWithImpl<$Res>
    implements $Prop_VerticalCopyWith<$Res> {
  _$Prop_VerticalCopyWithImpl(this._self, this._then);

  final Prop_Vertical _self;
  final $Res Function(Prop_Vertical) _then;

/// Create a copy of Prop
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(Prop_Vertical(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

// dart format on
