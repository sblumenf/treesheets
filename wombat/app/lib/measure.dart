import 'dart:async';
import 'dart:io';

import 'session.dart';
import 'src/rust/api/wire.dart';
import 'stage_read.dart';

/// How long one step took, each time it was done.
class Timing {
  Timing(this.what, List<Duration> runs) : runs = List.unmodifiable(runs);

  /// The step, in words.
  final String what;

  /// The time of every run, in the order they were done.
  final List<Duration> runs;

  /// The middle one (the upper of the two middle ones if there are an even number).
  Duration get median => (runs.toList()..sort())[runs.length ~/ 2];

  /// The slowest one.
  Duration get worst => runs.reduce((a, b) => a > b ? a : b);
}

/// What [measure] found.
class Measurement {
  Measurement._();

  /// The number of cells of the document, counted as it was built.
  late final int cells;

  /// The size of the saved file in bytes.
  late final int fileBytes;

  /// Whether this run built the document. If so, the process had the whole document in
  /// memory before it was opened, and [maxRss] and [rssBefore] are higher than those of
  /// a process that only opens it. Run again to measure with the file that is there.
  late final bool built;

  /// How long building and saving the document took, if this run did; one run.
  Duration? build;

  /// Whether the report of the edit listed the formula cell that reads every cell, that
  /// is, whether "one edit" here includes working that formula out again.
  late final bool recalculates;

  /// Every step that was timed.
  final List<Timing> timings = [];

  /// Resident memory of the process just before the document was opened, in bytes.
  late final int rssBefore;

  /// The same just after it was opened the first time.
  late final int rssOpen;

  /// The same at the end, with the document open, after it was opened `runs` times and
  /// edited and saved. Where memory that was let go of stays with the process, as with
  /// the allocator of desktop Linux, this is well above [rssOpen].
  late final int rssEnd;

  /// The highest resident memory seen while the document was opened the first time, by
  /// looking about once a millisecond. It can miss a short peak.
  late final int sampledPeak;

  /// The highest resident memory since just before the document was opened, as the
  /// kernel recorded it (`VmHWM` in `/proc/self/status`), or null where the mark cannot
  /// be read or set back (not Linux or Android, or not allowed).
  int? peakSinceOpen;

  /// The highest resident memory of the process since it started (`ProcessInfo.maxRss`).
  late final int maxRss;

  /// The measurement as plain text.
  String report() {
    final out = StringBuffer();
    String mib(int bytes) =>
        '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MiB';
    String ms(Duration d) => (d.inMicroseconds / 1000).toStringAsFixed(2);
    out.writeln('Wombat bridge measurement');
    out.writeln(
      'system: ${Platform.operatingSystem} ${Platform.operatingSystemVersion}, '
      '${Platform.numberOfProcessors} processors',
    );
    out.writeln(
      'Dart build mode: $dartBuildMode (an app build compiles the Rust library '
      'optimised in profile and release mode, unoptimised in debug mode)',
    );
    out.writeln('document: $cells cells, $fileBytes bytes (${mib(fileBytes)})');
    out.writeln(
      built
          ? 'built in this run in ${ms(build!)} ms: the memory figures include '
                'what building left behind; run again for those of opening alone'
          : 'the file was there from an earlier run: this process only opened it',
    );
    out.writeln(
      'one edit includes working out a formula that reads every cell: '
      '${recalculates ? 'yes' : 'NO'}',
    );
    out.writeln('times in ms: median, worst, runs');
    for (final t in timings) {
      out.writeln(
        '  ${t.what.padRight(60)} ${ms(t.median).padLeft(10)} '
        '${ms(t.worst).padLeft(10)} ${t.runs.length.toString().padLeft(3)}',
      );
    }
    out.writeln('resident memory');
    out.writeln('  before opening                     ${mib(rssBefore)}');
    out.writeln('  after opening                      ${mib(rssOpen)}');
    out.writeln('  at the end, after all of the above ${mib(rssEnd)}');
    out.writeln('  peak while opening, sampled        ${mib(sampledPeak)}');
    final peak = peakSinceOpen;
    out.writeln(
      '  peak since before opening, kernel  '
      '${peak == null ? 'not available' : mib(peak)}',
    );
    out.writeln('  peak of the process (maxRss)       ${mib(maxRss)}');
    return out.toString();
  }
}

/// The mode the Dart code was compiled in.
String get dartBuildMode => const bool.fromEnvironment('dart.vm.product')
    ? 'release'
    : const bool.fromEnvironment('dart.vm.profile')
    ? 'profile'
    : 'debug';

/// The number of cells of the document that [measure] builds.
int cellCount(int across, int down) => 1 + across * down * 101;

/// Measures the bridge on a large document: opening, reading, editing, undoing and
/// saving, and the memory of the process. Needs nothing but a folder it may write to.
///
/// The document is shaped like the one of the core's `tests/large.rs`: the root has
/// `across` by `down` cells, each with a grid of 10 by 10 cells with text; 10 by 100
/// gives 101,001 cells. One of the innermost cells is a formula that reads every cell.
/// It is built through the bridge, saved in `folder` and let go of; a file that an
/// earlier run left there is used as it is, so that a second run measures a process
/// that never built anything. Every step is done `runs` times.
///
/// The read a screen draws from ([Session.stage]) is timed on that document's root,
/// whole and as a window of 40 rows, and on the root of a second document that is
/// nested as deep as the interface is specified for: `nested` gives the size of the
/// grids on each level, as [nestedTable] says. For each: the whole call, how long
/// of it the Dart thread was held (the answer is decoded on the thread that draws),
/// and what comparing the answer with the one before costs ([StageRead.since]).
Future<Measurement> measure(
  Directory folder, {
  int across = 10,
  int down = 100,
  int runs = 5,
  List<(int, int)> nested = sixLevels,
}) async {
  final m = Measurement._();
  m.cells = cellCount(across, down);
  final large = File('${folder.path}/large-${across}x$down.wombat');
  m.built = !large.existsSync();
  if (m.built) {
    final watch = Stopwatch()..start();
    await _build(large, across, down);
    m.build = watch.elapsed;
  }
  // The edits are saved to a copy, so that the next run finds the same file. The copy is
  // made by hand, which decision A28 forbids for documents: its cells have the IDs of
  // the original. A copy by the core (`Session.openCopy`) would read the whole document
  // into this process before the open that is measured, and the memory figures would
  // be those of a process that had already held it. The copy exists only during the
  // run, is never open together with the original, and is deleted at the end.
  final work = File('${folder.path}/work.wombat');
  if (work.existsSync()) work.deleteSync();
  large.copySync(work.path);
  m.fileBytes = work.lengthSync();

  // Opening, the first time with the memory watched.
  final resettable = _resetPeak();
  m.rssBefore = ProcessInfo.currentRss;
  var sampled = m.rssBefore;
  final sampler = Timer.periodic(const Duration(milliseconds: 1), (_) {
    final now = ProcessInfo.currentRss;
    if (now > sampled) sampled = now;
  });
  final opens = <Duration>[];
  var watch = Stopwatch()..start();
  Session doc;
  try {
    doc = await Session.open(work.path);
  } finally {
    // Also when the document cannot be opened: a timer left running keeps looking.
    sampler.cancel();
  }
  opens.add(watch.elapsed);
  m.rssOpen = ProcessInfo.currentRss;
  m.sampledPeak = sampled > m.rssOpen ? sampled : m.rssOpen;
  if (resettable) m.peakSinceOpen = _peak();
  for (var i = 1; i < runs; i++) {
    await doc.close();
    watch = Stopwatch()..start();
    doc = await Session.open(work.path);
    opens.add(watch.elapsed);
  }
  m.timings.add(Timing('open', opens));

  Future<List<Duration>> timed(Future<void> Function(int i) step) async {
    final times = <Duration>[];
    for (var i = 0; i < runs; i++) {
      final watch = Stopwatch()..start();
      await step(i);
      times.add(watch.elapsed);
    }
    return times;
  }

  final root = doc.root;
  late GridInfo rootGrid;
  m.timings.add(
    Timing(
      'read the root grid (${across * down} cells)',
      await timed((_) async => rootGrid = await doc.grid(root)),
    ),
  );
  if (rootGrid.cells.length != across * down) {
    throw StateError('the root grid has ${rootGrid.cells.length} cells');
  }
  final section = rootGrid.cells[rootGrid.cells.length ~/ 2].id;
  late GridInfo inner;
  m.timings.add(
    Timing(
      'read an inner grid (100 cells)',
      await timed((_) async => inner = await doc.grid(section)),
    ),
  );
  await _stage(m, doc, 'stage of the root, whole', phoneAsk, runs);
  await _stage(
    m,
    doc,
    'stage of the root, 40 rows',
    windowOf(40, y0: down ~/ 3),
    runs,
  );
  final last = await doc.grid(rootGrid.cells.last.id);
  final formula = last.cells.last;
  if (formula.kind != CellKind.formula || formula.failed) {
    throw StateError('the formula shows ${formula.result}');
  }

  // One text edit and its undo. The text is a number, so the formula's result changes.
  final leaf = inner.cells[3].id;
  final undos = <Duration>[];
  Future<void> undo() async {
    final watch = Stopwatch()..start();
    if (await doc.undo() == null) throw StateError('nothing to undo');
    undos.add(watch.elapsed);
  }

  // Each run of the two is a command and its undo; the undo's own time comes off.
  Timing less(String what, List<Duration> both) =>
      Timing(what, [for (var i = 0; i < runs; i++) both[i] - undos[i]]);
  var recalculates = true;
  Future<void> editAndInsert(String how) async {
    undos.clear();
    final edits = await timed((i) async {
      final report = await doc.apply(
        Command.setText(cell: leaf, text: '${i + 1}'),
      );
      recalculates = recalculates && report.recalculated.contains(formula.id);
      await undo();
    });
    m.timings.add(less('set the text of one cell$how', edits));
    m.timings.add(Timing('undo that', undos.toList()));

    // One structural command and its undo.
    undos.clear();
    final inserts = await timed((_) async {
      final report = await doc.apply(
        Command.insertRows(cell: root, at: down, count: 1),
      );
      if (report.created.length != across) {
        throw StateError('inserting a row made ${report.created.length} cells');
      }
      await undo();
    });
    m.timings.add(less('insert a row in the root grid$how', inserts));
    m.timings.add(Timing('undo that', undos.toList()));
  }

  await editAndInsert('');
  m.recalculates = recalculates;
  // The same without the formula, to tell the cost of working it out from the cost of
  // the command and the bridge: a document without formula cells pays for none.
  await doc.apply(Command.clearCells(cells: [formula.id]));
  await editAndInsert(', no formula');

  // Saving, each time after an edit, or there is nothing to save.
  final saves = <Duration>[];
  for (var i = 0; i < runs; i++) {
    await doc.apply(Command.setText(cell: leaf, text: '${i + 1}'));
    final watch = Stopwatch()..start();
    await doc.save();
    saves.add(watch.elapsed);
  }
  m.timings.add(Timing('save', saves));

  m.rssEnd = ProcessInfo.currentRss;
  m.maxRss = ProcessInfo.maxRss;
  await doc.close();
  work.deleteSync();

  // The nested document, after the memory was noted: it is a second document in
  // the same process. Opened from its file, as an app has it.
  final deep = File(
    '${folder.path}/nested-${nested.map((s) => '${s.$1}x${s.$2}').join('-')}.wombat',
  );
  if (!deep.existsSync()) {
    await _write(deep, nestedTable(nested), 'Nested');
  }
  final levels = await Session.open(deep.path);
  final count = nestedTable(nested).cells.length + 1;
  await _stage(
    m,
    levels,
    'nested, $count cells: stage of the root',
    phoneAsk,
    runs,
  );
  await _stage(
    m,
    levels,
    'nested: the same, two levels cell by cell',
    windowOf(4294967295, fullDepth: 2),
    runs,
  );
  await levels.close();
  return m;
}

/// The sizes of the nested document that is measured by default, the six-level
/// document of the core's and the bridge's own measurements (68,749 cells).
const sixLevels = [(12, 20), (4, 5), (3, 4), (2, 3), (2, 2), (1, 2)];

/// [phoneAsk] with a window of `rows` rows from row `y0`, or with more levels given
/// cell by cell.
StageAsk windowOf(int rows, {int y0 = 0, int fullDepth = 1}) => StageAsk(
  x0: phoneAsk.x0,
  y0: y0,
  cols: phoneAsk.cols,
  rows: rows,
  fullDepth: fullDepth,
  fullCells: phoneAsk.fullCells,
  previewCols: phoneAsk.previewCols,
  previewRows: phoneAsk.previewRows,
  shapeCols: phoneAsk.shapeCols,
  shapeRows: phoneAsk.shapeRows,
  textCut: phoneAsk.textCut,
  previewCut: phoneAsk.previewCut,
);

/// A document nested `sizes.length` levels deep, as a table to import: the grid of
/// the root, and on each level the grid of the first cell of the level before, has
/// the columns and rows of `sizes.first`; every other grid on level `n` has those of
/// `sizes[n]`. In every grid each third cell, from the first on, holds a grid of the
/// next level.
TableData nestedTable(List<(int, int)> sizes) {
  final cells = <EntryData>[];
  void grid(int depth, bool first) {
    final (xs, ys) = first ? sizes.first : sizes[depth];
    for (var n = 0; n < xs * ys; n++) {
      final inner = n % 3 == 0 && depth + 1 < sizes.length;
      final within = first && n == 0
          ? sizes.first
          : sizes[inner ? depth + 1 : 0];
      cells.add(
        EntryData(
          text: 'Level $depth cell ${n % xs}.${n ~/ xs} text',
          formula: false,
          grid: inner
              ? GridSize(xs: within.$1, cells: within.$1 * within.$2)
              : null,
        ),
      );
      if (inner) grid(depth + 1, first && n == 0);
    }
  }

  grid(0, true);
  return TableData(xs: sizes.first.$1, cells: cells);
}

/// Times one read of the stage of the root of `doc`, `runs` times: the whole call,
/// how long of it the Dart thread was held, and the comparison of each answer with
/// the one before. Nothing changes between the reads, which is the comparison that
/// has most to do: no cell differs, so every cell is compared to its end.
Future<void> _stage(
  Measurement m,
  Session doc,
  String what,
  StageAsk ask,
  int runs,
) async {
  var before = StageRead(doc.root, ask, await doc.stage(doc.root, ask));
  final calls = <Duration>[];
  final holds = <Duration>[];
  final compares = <Duration>[];
  for (var i = 0; i < runs; i++) {
    final watch = Stopwatch()..start();
    final (info, held) = await heldDuring(() => doc.stage(doc.root, ask));
    calls.add(watch.elapsed);
    holds.add(held);
    watch.reset();
    final read = StageRead(doc.root, ask, info);
    final change = read.since(before);
    compares.add(watch.elapsed);
    if (!change.isEmpty) throw StateError('two reads of $what differ');
    before = read;
  }
  final shapes = before.summaries.fold(0, (n, s) => n + s.shape.length ~/ 4);
  m.timings
    ..add(
      Timing(
        '$what (${before.cells.length} cells, $shapes shape cells)',
        calls,
      ),
    )
    ..add(Timing('  of which the Dart thread was held', holds))
    ..add(Timing('  and comparing it with the read before', compares));
}

/// Runs `call` and says, with its answer, the longest time the thread it was called
/// on did nothing else while the call was under way. The core works on a thread of
/// its own; what holds this one is the decoding of the answer, which is what a
/// screen pays for a read in frames. Found by a timer that runs again as soon as it
/// has run: the longest wait between two of its turns.
Future<(T, Duration)> heldDuring<T>(Future<T> Function() call) async {
  final watch = Stopwatch()..start();
  var last = Duration.zero;
  var longest = Duration.zero;
  var under = true;
  void turn() {
    final now = watch.elapsed;
    if (now - last > longest) longest = now - last;
    last = now;
    if (under) Timer.run(turn);
  }

  Timer.run(turn);
  final T answer;
  try {
    answer = await call();
  } finally {
    under = false;
  }
  // This runs in the turn in which the answer came, right after it was decoded.
  turn();
  return (answer, longest);
}

/// Builds the document with one import command: one call across the bridge, and the
/// core makes all the cells in one step. A hundred thousand commands would measure the
/// bridge a hundred thousand times before the measuring starts, and fill the history.
Future<void> _build(File file, int across, int down) {
  final sections = across * down;
  // A table is one flat list: each section is followed by the cells of its grid.
  final table = TableData(
    xs: across,
    cells: [
      for (var n = 0; n < sections; n++) ...[
        EntryData(
          text: 'Section $n',
          formula: false,
          grid: const GridSize(xs: 10, cells: 100),
        ),
        for (var m = 0; m < 100; m++)
          n == sections - 1 && m == 99
              ? const EntryData(text: '=TREESUM(Large)', formula: true)
              : EntryData(
                  text: 'Item $n.$m with some ordinary text in it',
                  formula: false,
                ),
      ],
    ],
  );
  // The formula finds the root by this text.
  return _write(file, table, 'Large');
}

/// Makes the document whose root has the text `name` and `table` as its grid, and
/// saves it at `file`.
Future<void> _write(File file, TableData table, String name) async {
  // Under another name until it is whole, so that a run that was stopped half-way does
  // not leave a file that the next run takes for the document.
  final part = File('${file.path}.part');
  if (part.existsSync()) part.deleteSync();
  final doc = await Session.create(part.path);
  final root = doc.root;
  await doc.apply(Command.import_(cell: root, table: table, replace: false));
  // After the import: a formula finds the root by this text.
  await doc.apply(Command.setText(cell: root, text: name));
  await doc.close();
  part.renameSync(file.path);
}

/// The kernel's high-water mark of resident memory in bytes, or null where there is none.
int? _peak() {
  try {
    for (final line in File('/proc/self/status').readAsLinesSync()) {
      if (!line.startsWith('VmHWM:')) continue;
      final kb = int.tryParse(line.split(RegExp(r'\s+'))[1]);
      return kb == null ? null : kb * 1024;
    }
  } on Object {
    // No such file: not Linux or Android.
  }
  return null;
}

/// Sets the kernel's high-water mark back to the present resident memory, and says
/// whether that worked.
bool _resetPeak() {
  try {
    File('/proc/self/clear_refs').writeAsStringSync('5');
  } on Object {
    return false;
  }
  final peak = _peak();
  // Half a megabyte of slack: the two are read one after the other.
  return peak != null && peak <= ProcessInfo.currentRss + 512 * 1024;
}
