import 'package:flutter/foundation.dart';

/// The switches and thresholds of the prototype (UI_SPEC sections 3 and 6): what is
/// being tried out, and so can be changed while the app runs. They are this run's:
/// nothing writes them to a file.
@immutable
class Variants {
  const Variants({
    this.tapTextSelects = false,
    this.tapChildUnfolds = false,
    this.pin = true,
    this.minText = 11,
    this.previewScale = 0.7,
  });

  /// "Tap text": false edits at once; true selects first, and a tap on the selected
  /// cell edits.
  final bool tapTextSelects;

  /// "Tap child grid": false dives; true unfolds the grid in place, cell by cell.
  final bool tapChildUnfolds;

  /// Whether the first row and the first column stay in view when scrolling.
  final bool pin;

  /// The smallest text that counts as readable, in points before the system's text
  /// size is applied.
  final double minText;

  /// The size of a Preview's text against a cell's own.
  final double previewScale;

  /// The range of [minText] that the settings offer.
  static const minTextRange = (least: 8.0, most: 16.0);

  /// The range of [previewScale] that the settings offer.
  static const previewScaleRange = (least: 0.5, most: 1.0);

  Variants copyWith({
    bool? tapTextSelects,
    bool? tapChildUnfolds,
    bool? pin,
    double? minText,
    double? previewScale,
  }) => Variants(
    tapTextSelects: tapTextSelects ?? this.tapTextSelects,
    tapChildUnfolds: tapChildUnfolds ?? this.tapChildUnfolds,
    pin: pin ?? this.pin,
    minText: (minText ?? this.minText).clamp(
      minTextRange.least,
      minTextRange.most,
    ),
    previewScale: (previewScale ?? this.previewScale).clamp(
      previewScaleRange.least,
      previewScaleRange.most,
    ),
  );

  /// As the prototype log writes them with every entry.
  Map<String, Object> toJson() => {
    'tapText': tapTextSelects ? 'select' : 'edit',
    'tapChild': tapChildUnfolds ? 'unfold' : 'dive',
    'pin': pin,
    'minText': minText,
    'previewScale': previewScale,
  };

  @override
  bool operator ==(Object other) =>
      other is Variants &&
      other.tapTextSelects == tapTextSelects &&
      other.tapChildUnfolds == tapChildUnfolds &&
      other.pin == pin &&
      other.minText == minText &&
      other.previewScale == previewScale;

  @override
  int get hashCode =>
      Object.hash(tapTextSelects, tapChildUnfolds, pin, minText, previewScale);
}
