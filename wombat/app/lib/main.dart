import 'dart:async';
import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';

import 'demo_screen.dart';
import 'example.dart';
import 'grid/grid_screen.dart';
import 'grid/stage.dart';
import 'keeper.dart';
import 'session.dart';
import 'src/rust/api/wire.dart';
import 'src/rust/frb_generated.dart';

/// The name of the app's one document in its folder.
const documentName = 'example.wombat';

/// The Wombat app as far as it goes: a simple grid screen on one document, which starts
/// as the example, and the plain-text check of the bridge behind a menu entry. Not yet
/// the interface that `docs/UI_SPEC.md` specifies.
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

/// The Wombat app: its one document, opened at the start and kept in its file, and
/// on it the screen that `screen` builds.
class WombatApp extends StatelessWidget {
  const WombatApp({super.key, this.folder, this.screen = GridOnKeeper.new});

  /// Where the app keeps its document; see [StartScreen.folder].
  final Directory? folder;

  /// See [StartScreen.screen].
  final Widget Function(Keeper keeper) screen;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Wombat',
    home: StartScreen(folder: folder, screen: screen),
  );
}

/// Opens the app's document, at first a copy of the example that comes with the app,
/// keeps it ([Kept]) and shows the screen on it. A document that cannot be opened is
/// said, with the ways out.
class StartScreen extends StatefulWidget {
  const StartScreen({super.key, this.folder, this.screen = GridOnKeeper.new});

  /// Where the document is kept. Without one it is [documentFolder]. Tests give a
  /// folder of their own, so that none writes where a user's documents are.
  final Directory? folder;

  /// Builds the screen that works on the document through its keeper. The keeper is
  /// the start screen's: it is closed when the app goes, not by the screen.
  final Widget Function(Keeper keeper) screen;

  @override
  State<StartScreen> createState() => _StartScreenState();
}

class _StartScreenState extends State<StartScreen> {
  Future<Keeper>? _opening;

  /// Where the document is kept, once that is known.
  Directory? _folder;

  Future<Keeper> _open(AssetBundle assets) async => Keeper(
    await openDocument(
      _folder ??= widget.folder ?? await documentFolder(),
      example: () => assets.load('assets/example.wombat'),
    ),
  );

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

  /// The document goes with the app.
  @override
  void dispose() {
    _opening?.then(endKeeping, onError: (Object _) {});
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final assets = DefaultAssetBundle.of(context);
    return FutureBuilder<Keeper>(
      future: _opening ??= _open(assets),
      builder: (context, snapshot) {
        final keeper = snapshot.data;
        if (keeper != null) {
          return Kept(keeper: keeper, child: widget.screen(keeper));
        }
        final error = snapshot.connectionState == ConnectionState.done
            ? snapshot.error
            : null;
        if (error == null) {
          return const Scaffold(
            body: Center(child: CircularProgressIndicator()),
          );
        }
        final folder = _folder;
        return Scaffold(
          body: SafeArea(
            child: Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      'The document could not be opened. ${describeError(error)}',
                      key: const Key('start-error'),
                    ),
                    const SizedBox(height: 16),
                    FilledButton(
                      key: const Key('start-again'),
                      onPressed: () => _again(_open(assets)),
                      child: const Text('Try again'),
                    ),
                    if (folder != null && canBePutAside(error, folder)) ...[
                      const SizedBox(height: 16),
                      Text('Its file is ${folder.path}/$documentName'),
                      OutlinedButton(
                        key: const Key('start-over'),
                        onPressed: () => _again(_startOver(assets, folder)),
                        child: const Text(
                          'Put that file aside and start with the example',
                        ),
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
  const Kept({
    super.key,
    required this.keeper,
    required this.child,
    this.exitWithin = const Duration(seconds: 5),
  });

  final Keeper keeper;
  final Widget child;

  /// How long an exit that the system asks for waits for the document to be put
  /// away. After that the app leaves all the same, so that a save that hangs cannot
  /// keep the app from ever being closed.
  final Duration exitWithin;

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

  /// Nothing waits for what the keeper is asked here, and after the end of the work
  /// there is nothing to put away.
  void _ask(Future<void> Function() what) {
    if (widget.keeper.ended != null) return;
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

  /// The system asks whether the app may exit: on Linux, macOS and Windows when its
  /// window is closed. (Android and iOS do not ask; there the app is put away
  /// first, which is the case above.) The answer waits until the document is put
  /// away and closed, but no longer than [Kept.exitWithin].
  ///
  /// If the document is then not in its file, or what was being typed could not be
  /// entered, the exit is held back once, so that the user sees what the screen says
  /// about it. Asked again with nothing better, the app leaves.
  @override
  Future<AppExitResponse> didRequestAppExit() => _leave().timeout(
    widget.exitWithin,
    onTimeout: () {
      // Whether everything was saved cannot be known here.
      debugPrint(
        'Wombat: leaving after waiting ${widget.exitWithin.inSeconds} s for '
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

/// The stand-in for the app's screen until the interface of `docs/UI_SPEC.md` is
/// built: the simple grid screen on a stage of its own, with the plain-text check of
/// the bridge behind its menu.
class GridOnKeeper extends StatefulWidget {
  const GridOnKeeper(this.keeper, {super.key});

  final Keeper keeper;

  @override
  State<GridOnKeeper> createState() => _GridOnKeeperState();
}

class _GridOnKeeperState extends State<GridOnKeeper> {
  late final Future<Stage> _stage;

  @override
  void initState() {
    super.initState();
    _stage = Stage.open(widget.keeper);
  }

  /// The stage goes with the screen, when the steps asked of it are done.
  @override
  void dispose() {
    _stage.then((stage) => stage.leave(), onError: (Object _) {});
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => FutureBuilder<Stage>(
    future: _stage,
    builder: (context, snapshot) {
      final stage = snapshot.data;
      if (stage != null) {
        return GridScreen(
          stage: stage,
          menu: [
            PopupMenuItem(
              key: const Key('bridge-check'),
              value: () => Navigator.of(context).push(
                MaterialPageRoute<void>(builder: (_) => const DemoScreen()),
              ),
              child: const Text('Bridge check'),
            ),
          ],
        );
      }
      final error = snapshot.error;
      return Scaffold(
        body: Center(
          child: error == null
              ? const CircularProgressIndicator()
              : Padding(
                  padding: const EdgeInsets.all(24),
                  child: Text(
                    'The document could not be read. ${describeError(error)}',
                    key: const Key('stage-error'),
                  ),
                ),
        ),
      );
    },
  );
}
