import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';

import 'variants.dart';

/// The prototype log (UI_SPEC section 16): every dive, way up, jump, edit and undo
/// with the time and the variant settings, to count accidental dives and the like.
///
/// One line of JSON per entry. It holds no text of any cell and no cell ID: what was
/// done, how deep, and at which column and row. It stays on the device: in memory,
/// and in `file` if one is given, to which each entry is added as it is made. A file
/// that cannot be written costs the entries of this run their place on disk and
/// nothing else; the app never fails over its log.
class ProtoLog extends ChangeNotifier {
  /// A log kept in `file`, or in memory alone without one. `now` is the clock.
  ProtoLog({this.file, this._now = DateTime.now});

  /// Where the entries are kept between runs, if anywhere.
  final File? file;
  final DateTime Function() _now;
  final List<String> _lines = [];
  Future<void> _written = Future.value();

  /// The most entries kept in memory and given out: the newest.
  static const most = 5000;

  /// The entries, the oldest first, each a line of JSON.
  List<String> get lines => List.unmodifiable(_lines);

  /// All entries as the text of a file.
  String get text => _lines.map((line) => '$line\n').join();

  /// Reads what earlier runs left in the file, and puts it before the entries of
  /// this run. Call it once, before the first entry is made: what is added while
  /// it reads is written after it has read. What cannot be read is passed over.
  Future<void> load() {
    final file = this.file;
    if (file == null) return Future.value();
    return _written = _written.then((_) async {
      try {
        // No file yet is one of the things that cannot be read.
        final old = const LineSplitter()
            .convert(await file.readAsString())
            .where((line) => line.isNotEmpty)
            .toList();
        _lines.insertAll(0, old);
        _trim();
        notifyListeners();
      } on Object {
        // Nothing of the app depends on the log.
      }
    });
  }

  void _trim() {
    if (_lines.length > most) _lines.removeRange(0, _lines.length - most);
  }

  /// Notes that `what` was done under `variants`, with whatever else says where:
  /// numbers and words, never text of the document.
  void add(
    String what,
    Variants variants, [
    Map<String, Object?> where = const {},
  ]) {
    final line = jsonEncode({
      't': _now().toUtc().toIso8601String(),
      'what': what,
      ...where,
      'variants': variants.toJson(),
    });
    _lines.add(line);
    _trim();
    final file = this.file;
    if (file != null) {
      _written = _written.then((_) async {
        try {
          await file.writeAsString('$line\n', mode: FileMode.append);
        } on Object {
          // The entry stays in memory.
        }
      });
    }
    notifyListeners();
  }

  /// Ends when every entry made so far is in the file, or could not be put there.
  Future<void> written() => _written;

  /// Writes all entries to a new file in `folder`, which is made if it is not
  /// there, and answers with the file: what the user sends on.
  Future<File> export(Directory folder) async {
    await folder.create(recursive: true);
    final stamp = _now().toUtc().toIso8601String().replaceAll(
      RegExp('[^0-9T]'),
      '',
    );
    final out = File('${folder.path}/wombat-log-$stamp.jsonl');
    await out.writeAsString(text);
    return out;
  }
}
