import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'demo_screen.dart';
import 'example.dart';
import 'grid/grid_screen.dart';
import 'grid/stage.dart';
import 'keeper.dart';
import 'session.dart';
import 'src/rust/api/wire.dart';
import 'src/rust/frb_generated.dart';

/// The Wombat app as far as it goes: a simple grid screen on one document, which starts
/// as the example, and the plain-text check of the bridge behind a menu entry. Not yet
/// the interface that `docs/UI_SPEC.md` specifies.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const WombatApp());
}

/// The folder the app keeps its document in (decision A156).
///
/// On Linux, macOS and Windows it is a folder `wombat` where the system keeps the data
/// of this user's programs, found by the environment as the system defines it. On
/// Android and iOS there is no finding the app's documents folder without a plugin;
/// there, and wherever the environment does not say, it is `wombat` in `temp`, the
/// system's temporary folder, which on a phone is the app's own cache folder and which
/// the system may clear.
String documentFolder({
  required String os,
  required Map<String, String> environment,
  required String temp,
}) {
  String? at(String name) {
    final value = environment[name];
    return value == null || value.isEmpty ? null : value;
  }

  final home = at('HOME');
  final xdg = at('XDG_DATA_HOME');
  final data = switch (os) {
    // A path that is not from the root is not valid there and is passed over, as
    // the XDG Base Directory specification says.
    'linux' =>
      xdg != null && xdg.startsWith('/')
          ? xdg
          : home == null
          ? null
          : '$home/.local/share',
    'macos' => home == null ? null : '$home/Library/Application Support',
    'windows' => at('APPDATA'),
    _ => null,
  };
  final separator = os == 'windows' ? r'\' : '/';
  return '${data ?? temp}${separator}wombat';
}

/// The stage of the first screen on a document that was just opened, which a new
/// keeper keeps from here on. If there is no stage to be had, the document is let go
/// of, so that its file does not stay taken by something nobody holds.
Future<Stage> onStage(Session doc) async {
  final keeper = Keeper(doc);
  try {
    return await Stage.open(keeper);
  } catch (_) {
    await keeper.letGo();
    rethrow;
  }
}

class WombatApp extends StatelessWidget {
  const WombatApp({super.key, this.folder});

  /// Where the app keeps its document; see [StartScreen.folder].
  final Directory? folder;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Wombat',
    home: StartScreen(folder: folder),
  );
}

/// Opens the app's document, at first a copy of the example that comes with the app,
/// and shows it on the grid screen.
class StartScreen extends StatefulWidget {
  const StartScreen({super.key, this.folder});

  /// Where the document is kept. Without one it is [documentFolder]. Tests give a
  /// folder of their own.
  final Directory? folder;

  @override
  State<StartScreen> createState() => _StartScreenState();
}

class _StartScreenState extends State<StartScreen> {
  late Future<Stage> _stage = _open();

  Directory get _folder =>
      widget.folder ??
      Directory(
        documentFolder(
          os: Platform.operatingSystem,
          environment: Platform.environment,
          temp: Directory.systemTemp.path,
        ),
      );

  File get _file => File('${_folder.path}/example.wombat');

  /// The document is the one that the last start left, with everything done to it, or
  /// if there is none yet, a copy of the example.
  Future<Stage> _open() async {
    await _folder.create(recursive: true);
    if (_file.existsSync()) return onStage(await Session.open(_file.path));
    final example = await rootBundle.load('assets/example.wombat');
    return onStage(
      await openExampleCopy(
        bytesOf(example),
        folder: _folder,
        to: 'example.wombat',
      ),
    );
  }

  /// The way out when the document that an earlier start left cannot be opened: its
  /// file gets another name, so that nothing is thrown away, and the app starts with
  /// a new copy of the example.
  Future<Stage> _startOver() async {
    final stamp = DateTime.now().millisecondsSinceEpoch;
    await _file.rename('${_folder.path}/example-put-aside-$stamp.wombat');
    return _open();
  }

  void _again(Future<Stage> Function() open) => setState(() {
    _stage = open();
  });

  /// The document goes with the screen; see [Kept].
  @override
  void dispose() {
    _stage.then(endKeeping, onError: (Object _) {});
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<Stage>(
      future: _stage,
      builder: (context, snapshot) {
        final stage = snapshot.data;
        if (snapshot.connectionState == ConnectionState.done && stage != null) {
          return Kept(
            keeper: stage.keeper,
            child: GridScreen(
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
            ),
          );
        }
        final error = snapshot.connectionState == ConnectionState.done
            ? snapshot.error
            : null;
        if (error == null) {
          return const Scaffold(
            body: Center(child: CircularProgressIndicator()),
          );
        }
        // A file that this app has open already, or a fault, is not the file's doing.
        final unreadable =
            !isFatal(error) &&
            !(error is BridgeError && error.kind == ErrorKind.alreadyOpen) &&
            _file.existsSync();
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
                      onPressed: () => _again(_open),
                      child: const Text('Try again'),
                    ),
                    if (unreadable) ...[
                      const SizedBox(height: 16),
                      Text('Its file is ${_file.path}'),
                      OutlinedButton(
                        key: const Key('start-over'),
                        onPressed: () => _again(_startOver),
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

/// Ends the keeping of a document when its last screen has gone: saves and closes it.
/// If it cannot be saved there is no one left to ask, so it is let go of, which never
/// writes over what another program put in the file.
Future<void> endKeeping(Stage stage) =>
    stage.keeper.close().catchError((Object _) => stage.keeper.letGo());

/// Tells the keeper of the document on the screen below it when the app is no longer
/// in front. It is where the app owns its document, and not in a screen, so that
/// every screen on the document is covered by it (decision A155).
class Kept extends StatefulWidget {
  const Kept({super.key, required this.keeper, required this.child});

  final Keeper keeper;
  final Widget child;

  @override
  State<Kept> createState() => _KeptState();
}

class _KeptState extends State<Kept> with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  /// A window that loses the focus, a dialog of the system or the list of apps make
  /// the app `inactive`: it is still there and the user has not left, so only what is
  /// in the document already is saved. Out of sight (`hidden`, `paused`, `detached`)
  /// the system may end the app without another word, so what is being typed goes
  /// into the document first.
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    switch (state) {
      case AppLifecycleState.resumed:
        break;
      case AppLifecycleState.inactive:
        widget.keeper.save();
      case AppLifecycleState.hidden ||
          AppLifecycleState.paused ||
          AppLifecycleState.detached:
        widget.keeper.putAway();
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
