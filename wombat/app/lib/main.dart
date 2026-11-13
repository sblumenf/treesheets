import 'dart:async';
import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';

import 'demo_screen.dart';
import 'example.dart';
import 'keeper.dart';
import 'phone/log.dart';
import 'phone/phone_screen.dart';
import 'phone/variants.dart';
import 'phone/words.dart';
import 'session.dart';
import 'src/rust/api/wire.dart';
import 'src/rust/frb_generated.dart';
import 'test_documents.dart';

/// The name of the app's one document in its folder.
const documentName = 'example.wombat';

/// The Wombat app as far as it goes: the phone screen of `docs/UI_SPEC.md`, its first
/// slice, on one document, which starts as the example; the prototype's test
/// documents and the plain-text check of the bridge are behind its menu.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const WombatApp());
}

/// The folder the app keeps its documents in: `documents` in the folder that the
/// system gives the app for its own data, on every system alike. `appData` asks the
/// system for that one; without it the `path_provider` plugin does (its "application
/// support" folder, which no other app reads, the system does not clear, and which
/// stays when the app is updated).
///
/// A path that is not from the root is refused: it would be a different folder
/// whenever the app is started from somewhere else. What `appData` throws when the
/// system cannot say is thrown on. There is no other place to fall back to.
Future<Directory> documentFolder({
  Future<Directory> Function() appData = getApplicationSupportDirectory,
}) async {
  final data = await appData();
  if (!data.isAbsolute) {
    throw FileSystemException(
      'The folder for the app\'s data is not a path from the root',
      data.path,
    );
  }
  return Directory('${data.path}/documents');
}

/// Opens the app's document in `folder`: what an earlier start left there, with
/// everything done to it, or if nothing is there yet, a copy of the example that
/// comes with the app, which `example` reads. Whatever has the document's name is
/// taken for the document, also a folder or a link that leads nowhere: it cannot be
/// opened then, and [putAside] is the way out.
Future<Session> openDocument(
  Directory folder, {
  required Future<ByteData> Function() example,
}) async {
  await folder.create(recursive: true);
  final path = '${folder.path}/$documentName';
  if (FileSystemEntity.typeSync(path, followLinks: false) !=
      FileSystemEntityType.notFound) {
    return Session.open(path);
  }
  return openExampleCopy(
    bytesOf(await example()),
    folder: folder,
    to: documentName,
  );
}

/// Whether [putAside] is a way out after `error` kept the document in `folder` from
/// being opened. It is not when the file is whole and merely taken: open in this
/// app already, or made by another start of the app between this one's look and its
/// copy of the example. Nor when nothing is there to put aside.
bool canBePutAside(Object error, Directory folder) =>
    !(error is BridgeError &&
        (error.kind == ErrorKind.alreadyOpen ||
            error.kind == ErrorKind.alreadyExists)) &&
    FileSystemEntity.typeSync(
          '${folder.path}/$documentName',
          followLinks: false,
        ) !=
        FileSystemEntityType.notFound;

/// Puts the document in `folder` aside, with what belongs to its name
/// (`docs/FILE_FORMAT.md`): its image folder, and the folder of view states, which
/// are this one document's since the app keeps no other here. They go, under the
/// names they have, into a new folder `put-aside-<date>-<something>` in `folder`,
/// which is returned. Nothing is deleted and nothing replaced: the folder is made
/// new by the system, with a name that was not there, also for two calls in the same
/// instant.
Future<Directory> putAside(Directory folder, {DateTime? now}) async {
  final date = (now ?? DateTime.now()).toIso8601String().substring(0, 10);
  final aside = await folder.createTemp('put-aside-$date-');
  for (final name in [documentName, '$documentName.images', '.wombat-view']) {
    final from = '${folder.path}/$name';
    final to = '${aside.path}/$name';
    // Each as what it is: a link is moved, not what it leads to.
    switch (FileSystemEntity.typeSync(from, followLinks: false)) {
      case FileSystemEntityType.directory:
        await Directory(from).rename(to);
      case FileSystemEntityType.link:
        await Link(from).rename(to);
      case FileSystemEntityType.notFound:
        break;
      default:
        await File(from).rename(to);
    }
  }
  return aside;
}

/// The file of the prototype log in the app's folder.
const logName = 'prototype-log.jsonl';

/// Where a log that the user exports is written: the folder that `shared` gives,
/// or `fallback` where the system has none. Without `shared` the system is asked:
/// on Android for the app's folder on the storage that a computer sees when the
/// phone is plugged in (`Android/data/<application ID>/files`), elsewhere for the
/// user's folder for downloads.
Future<Directory> exportFolder(
  Directory fallback, {
  Future<Directory?> Function()? shared,
}) async {
  try {
    final folder =
        await (shared ??
            (Platform.isAndroid
                ? getExternalStorageDirectory
                : getDownloadsDirectory))();
    if (folder != null) return folder;
  } on Object {
    // Not every system has such a folder, and some say so by throwing.
  }
  return fallback;
}

/// The Wombat app: its document, opened at the start and kept in its file, and on
/// it the phone screen, or the screen that `screen` builds.
class WombatApp extends StatelessWidget {
  const WombatApp({super.key, this.folder, this.screen});

  /// Where the app keeps its document; see [StartScreen.folder].
  final Directory? folder;

  /// See [StartScreen.screen].
  final Widget Function(Keeper keeper)? screen;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: Words.app,
    // The banner of a debug build would lie over the menu.
    debugShowCheckedModeBanner: false,
    home: StartScreen(folder: folder, screen: screen),
  );
}

/// Opens the app's document, at first a copy of the example that comes with the app,
/// keeps it ([Kept]) and shows the screen on it. A document that cannot be opened is
/// said, with the ways out.
///
/// The phone screen's menu has the prototype's test documents ([TestDocument]): the
/// one that is open is closed, and the other opened in its place, made first if it
/// is not there yet. Every start begins with the example.
///
/// Keys: `opening` while a document is opened; `start-error` with `start-again`,
/// `start-over` and `start-example`; in the menu `document-<name>` for each test
/// document and `bridge-check`.
class StartScreen extends StatefulWidget {
  const StartScreen({super.key, this.folder, this.screen});

  /// Where the document is kept. Without one it is [documentFolder]. Tests give a
  /// folder of their own, so that none writes where a user's documents are; an
  /// exported log then goes into `exports` in that folder.
  final Directory? folder;

  /// Builds the screen that works on the document through its keeper, in place of
  /// the phone screen: for tests of the start and the end of the app. The keeper
  /// is the start screen's: it is closed when the app goes, not by the screen.
  final Widget Function(Keeper keeper)? screen;

  @override
  State<StartScreen> createState() => _StartScreenState();
}

class _StartScreenState extends State<StartScreen> {
  Future<Keeper>? _opening;

  /// Where the document is kept, once that is known.
  Directory? _folder;

  /// Which of the test documents is shown.
  TestDocument _document = TestDocument.example;

  /// The prototype's switches and its log, which outlive the document shown.
  final _variants = ValueNotifier(const Variants());
  ProtoLog? _log;

  Future<Keeper> _open(AssetBundle assets) async {
    final folder = _folder ??= widget.folder ?? await documentFolder();
    _log ??= ProtoLog(file: File('${folder.path}/$logName'))..load();
    final which = _document;
    return Keeper(
      which == TestDocument.example
          ? await openDocument(
              folder,
              example: () => assets.load('assets/example.wombat'),
            )
          : await openTestDocument(which, folder),
    );
  }

  /// The way out when what an earlier start left cannot be opened: it is put aside,
  /// so that nothing is thrown away, and the app starts with a new copy of the
  /// example.
  Future<Keeper> _startOver(AssetBundle assets, Directory folder) async {
    await putAside(folder);
    return _open(assets);
  }

  void _again(Future<Keeper> opening) => setState(() {
    _opening = opening;
  });

  /// The last document asked for with [_show], shown or on its way.
  Future<void> _shown = Future.value();

  /// Shows another test document: the one that is open is closed first, with
  /// what is being typed on its screen entered, and saved. One after the other:
  /// a second one asked for meanwhile waits until the first is open, and then
  /// closes that, so that no document is opened twice or left open unseen.
  void _show(TestDocument which, AssetBundle assets) =>
      _shown = _shown.then((_) => _showNext(which, assets));

  Future<void> _showNext(TestDocument which, AssetBundle assets) async {
    Keeper? open;
    try {
      // While its screen is still there: what is typed on it goes into the
      // document, and where it is into the view state.
      open = await _opening;
      await open?.putAway();
    } on Object {
      // Nothing was open, or its work has ended.
    }
    if (!mounted) return;
    _document = which;
    _again(() async {
      if (open != null) await endKeeping(open);
      return _open(assets);
    }());
  }

  /// The document goes with the app.
  @override
  void dispose() {
    _opening?.then(endKeeping, onError: (Object _) {});
    _variants.dispose();
    super.dispose();
  }

  /// The phone screen on `keeper`, with the test documents and the bridge check
  /// in its menu.
  Widget _phone(BuildContext context, Keeper keeper, AssetBundle assets) {
    final folder = _folder!;
    final given = widget.folder;
    return PhoneScreen(
      keeper: keeper,
      variants: _variants,
      log: _log!,
      exportFolder: given != null
          ? () async => Directory('${given.path}/exports')
          : () => exportFolder(folder),
      menu: [
        const PopupMenuDivider(),
        for (final which in TestDocument.values)
          PopupMenuItem(
            key: Key('document-${which.name}'),
            enabled: which != _document,
            value: () => _show(which, assets),
            child: Text(which.label),
          ),
        const PopupMenuDivider(),
        PopupMenuItem(
          key: const Key('bridge-check'),
          value: () => Navigator.of(
            context,
          ).push(MaterialPageRoute<void>(builder: (_) => const DemoScreen())),
          child: const Text(Words.bridgeCheck),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final assets = DefaultAssetBundle.of(context);
    return FutureBuilder<Keeper>(
      future: _opening ??= _open(assets),
      builder: (context, snapshot) {
        // While another document is opened, the snapshot still has the one before.
        final done = snapshot.connectionState == ConnectionState.done;
        final keeper = done ? snapshot.data : null;
        if (keeper != null) {
          final screen = widget.screen;
          return Kept(
            keeper: keeper,
            // A screen is of one keeper: another keeper, another screen.
            child: KeyedSubtree(
              key: ObjectKey(keeper),
              child: screen != null
                  ? screen(keeper)
                  : _phone(context, keeper, assets),
            ),
          );
        }
        final error = done ? snapshot.error : null;
        if (error == null) {
          return const Scaffold(
            body: Center(child: CircularProgressIndicator(key: Key('opening'))),
          );
        }
        final folder = _folder;
        final example = _document == TestDocument.example;
        return Scaffold(
          body: SafeArea(
            child: Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      Words.notOpened(describeError(error)),
                      key: const Key('start-error'),
                    ),
                    const SizedBox(height: 16),
                    FilledButton(
                      key: const Key('start-again'),
                      onPressed: () => _again(_open(assets)),
                      child: const Text(Words.tryAgain),
                    ),
                    if (!example) ...[
                      const SizedBox(height: 16),
                      OutlinedButton(
                        key: const Key('start-example'),
                        onPressed: () => _show(TestDocument.example, assets),
                        child: const Text(Words.openExample),
                      ),
                    ] else if (folder != null &&
                        canBePutAside(error, folder)) ...[
                      const SizedBox(height: 16),
                      Text(Words.itsFile('${folder.path}/$documentName')),
                      OutlinedButton(
                        key: const Key('start-over'),
                        onPressed: () => _again(_startOver(assets, folder)),
                        child: const Text(Words.putAside),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

/// Where the app owns its document, and not in a screen, so that every screen on the
/// document is covered by it (decision A155): tells the keeper when the app is no
/// longer in front, and holds the app's exit until the document is in its file.
class Kept extends StatefulWidget {
  const Kept({super.key, required this.keeper, required this.child});

  final Keeper keeper;
  final Widget child;

  /// How long an exit that the system asks for waits for the document to be put
  /// away. After that the app leaves all the same, so that a save that hangs cannot
  /// keep the app from ever being closed.
  static const exitWithin = Duration(seconds: 5);

  @override
  State<Kept> createState() => _KeptState();
}

class _KeptState extends State<Kept> with WidgetsBindingObserver {
  /// An exit was held back because the document was not in its file, and it still
  /// is not: the next one is let through.
  bool _heldBack = false;
  late Attachment _attachment;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _attach();
  }

  @override
  void didUpdateWidget(Kept old) {
    super.didUpdateWidget(old);
    if (identical(old.keeper, widget.keeper)) return;
    _attachment.detach();
    _attach();
  }

  void _attach() {
    _heldBack = false;
    final keeper = widget.keeper;
    _attachment = keeper.attach(
      onChange: () {
        if (keeper.state == KeeperState.saved) _heldBack = false;
      },
    );
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _attachment.detach();
    super.dispose();
  }

  /// Nothing waits for what the keeper is asked here, and after the end of the work,
  /// when the keeper refuses, there is nothing to put away.
  void _ask(Future<void> Function() what) {
    what()
        .then<void>((_) {})
        .catchError(
          (Object _) {},
          test: (error) =>
              error is BridgeError && error.kind == ErrorKind.closed,
        );
  }

  /// A window that loses the focus, a dialog of the system or the list of apps make
  /// the app `inactive`: it is still there and the user has not left, so only what is
  /// in the document already is saved. Out of sight (`hidden`, `paused`, `detached`)
  /// the system may end the app without another word, so what is being typed goes
  /// into the document as well. Flutter passes through `hidden` on the way to
  /// `paused` and `detached`, so the first of the three does the work and the others
  /// find nothing left to do; all three are named so that this does not rest on that.
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    switch (state) {
      case AppLifecycleState.resumed:
        break;
      case AppLifecycleState.inactive:
        _ask(widget.keeper.save);
      case AppLifecycleState.hidden ||
          AppLifecycleState.paused ||
          AppLifecycleState.detached:
        _ask(widget.keeper.putAway);
    }
  }

  /// The system asks whether the app may exit, when its window is closed: on Linux,
  /// where that was seen, and by Flutter's documentation on macOS; for Windows the
  /// documentation of Flutter 3.47 does not say so, and it was not tried. (Android
  /// and iOS do not ask; there the app is put away first, which is the case above.)
  /// The answer waits until the document is put away and closed, but no longer than
  /// [exitWithin].
  ///
  /// If the document is then not in its file, or what was being typed could not be
  /// entered, the exit is held back once, so that the user sees what the screen says
  /// about it. Asked again with nothing better, the app leaves.
  @override
  Future<AppExitResponse> didRequestAppExit() => _leave().timeout(
    Kept.exitWithin,
    onTimeout: () {
      // Whether everything was saved cannot be known here.
      debugPrint(
        'Wombat: leaving after waiting ${Kept.exitWithin.inSeconds} s for '
        'the document to be saved; whether it was saved is not known.',
      );
      return AppExitResponse.exit;
    },
  );

  Future<AppExitResponse> _leave() async {
    final keeper = widget.keeper;
    try {
      final unfinished = await keeper.putAway();
      final safe = unfinished.isEmpty && keeper.state == KeeperState.saved;
      if (!safe && !_heldBack) {
        _heldBack = true;
        return AppExitResponse.cancel;
      }
    } on BridgeError catch (error) {
      // The work had ended before: there is nothing to keep.
      if (error.kind != ErrorKind.closed) rethrow;
    }
    await endKeeping(keeper);
    return AppExitResponse.exit;
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
