// The app as it starts and as it ends, without a device: where its document is, the
// first start and the next, a document that cannot be opened and the ways out, the
// app put away and its window closed. On the real core, which answers in real time
// that a widget test does not let pass by itself (`until` does, and waits for the
// thing, not for a time), and on a stand-in for the core where time or a failure must
// be made. The screen is the least there can be (`ProbeScreen`), since none of this
// is a screen's; the last group has the app with the screen it really shows, and
// the test documents of the prototype that the start screen opens for it.
import 'dart:async';
import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/example.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/main.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'fake_document.dart';
import 'probe_screen.dart';
import 'support.dart';

/// A file that cannot be deleted.
class _Stuck implements File {
  _Stuck(this._real);

  final File _real;

  @override
  String get path => _real.path;

  @override
  Future<File> writeAsBytes(
    List<int> bytes, {
    FileMode mode = FileMode.write,
    bool flush = false,
  }) => _real.writeAsBytes(bytes, mode: mode, flush: flush);

  @override
  Future<FileSystemEntity> delete({bool recursive = false}) =>
      throw FileSystemException('cannot be deleted', path);

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

/// The app's assets, where reading the example first does what `before` does: what
/// another start of the app may do between this one's look and its copy.
class _Assets extends CachingAssetBundle {
  _Assets(this.before);

  final void Function() before;

  @override
  Future<ByteData> load(String key) {
    before();
    return rootBundle.load(key);
  }
}

void main() {
  late Directory root;
  var count = 0;

  setUpAll(() async {
    await loadBridge();
    root = await Directory.systemTemp.createTemp('wombat-main');
  });

  tearDownAll(() => root.delete(recursive: true));

  Finder key(String name) => find.byKey(Key(name));

  String text(WidgetTester tester, String name) =>
      tester.widget<Text>(key(name)).data!;

  Keeper keeperOf(WidgetTester tester) =>
      tester.widget<Kept>(find.byType(Kept)).keeper;

  Widget app(Directory folder) =>
      WombatApp(folder: folder, screen: ProbeScreen.new);

  List<String> names(Directory folder) =>
      folder.listSync().map((e) => e.path.split('/').last).toList()..sort();

  /// Lets real time pass until `done` says so, ten seconds at most.
  Future<void> until(WidgetTester tester, bool Function() done) async {
    final watch = Stopwatch()..start();
    while (!done() && watch.elapsed < const Duration(seconds: 10)) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 2)),
      );
      await tester.pump();
    }
    expect(done(), isTrue, reason: 'waited ten seconds');
  }

  Future<void> press(WidgetTester tester, String name) async {
    await tester.tap(key(name));
    await tester.pump();
  }

  /// The app on a folder of its own, started and showing its document.
  Future<(Directory, File)> started(WidgetTester tester) async {
    final folder = Directory('${root.path}/app-${count++}');
    await tester.pumpWidget(app(folder));
    await until(tester, () => key('probe-shown').evaluate().isNotEmpty);
    await until(tester, () => text(tester, 'probe-shown') == 'Shopping');
    return (folder, File('${folder.path}/example.wombat'));
  }

  /// The system asks the app whether it may exit, and gets its answer.
  Future<AppExitResponse> exitAsked(WidgetTester tester) async {
    AppExitResponse? answer;
    unawaited(
      tester.binding.handleRequestAppExit().then((said) => answer = said),
    );
    await until(tester, () => answer != null);
    return answer!;
  }

  /// Takes the app off the screen and waits until its document is closed.
  Future<void> end(WidgetTester tester) async {
    final keeper = keeperOf(tester);
    await tester.pumpWidget(const SizedBox());
    await until(tester, () => keeper.ended != null);
  }

  group('where the document is kept', () {
    test('in the folder the system gives the app for its data', () async {
      for (final data in [
        '/data/user/0/io.github.sblumenf.burrow/files',
        '/home/u/.local/share/io.github.sblumenf.burrow',
      ]) {
        final folder = await documentFolder(
          appData: () async => Directory(data),
        );
        expect(folder.path, '$data/documents');
      }
    });

    test('nowhere when the system gives no path from the root', () async {
      // As with a HOME that is not one: a folder that depends on where the app was
      // started from.
      await expectLater(
        documentFolder(appData: () async => Directory('.local/share/burrow')),
        throwsA(
          isA<FileSystemException>().having(
            (e) => e.path,
            'path',
            '.local/share/burrow',
          ),
        ),
      );
      // Nor when it cannot say at all: there is no second place.
      await expectLater(
        documentFolder(appData: () => throw StateError('no HOME')),
        throwsStateError,
      );
    });

    const channel = MethodChannel('plugins.flutter.io/path_provider');

    testWidgets('the app asks the system, through the plugin', (tester) async {
      final data = Directory('${root.path}/data-${count++}');
      final asked = <String>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
        call,
      ) async {
        asked.add(call.method);
        return data.path;
      });
      await tester.pumpWidget(const WombatApp(screen: ProbeScreen.new));
      await until(tester, () => key('probe-shown').evaluate().isNotEmpty);
      expect(asked, ['getApplicationSupportDirectory']);
      expect(keeperOf(tester).path, '${data.path}/documents/example.wombat');
      await end(tester);
    });

    testWidgets('a system that cannot say: said, with nothing to put aside', (
      tester,
    ) async {
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        channel,
        (call) async => throw PlatformException(code: 'none'),
      );
      await tester.pumpWidget(const WombatApp(screen: ProbeScreen.new));
      await until(tester, () => key('start-error').evaluate().isNotEmpty);
      expect(
        text(tester, 'start-error'),
        startsWith('The document could not be opened. Error: '),
      );
      expect(key('start-again'), findsOneWidget);
      expect(key('start-over'), findsNothing);
      await tester.pumpWidget(const SizedBox());
    });
  });

  test(
    'the bytes of an asset are its own, not those of the buffer it is in',
    () {
      final buffer = Uint8List.fromList([9, 9, 1, 2, 3, 9]).buffer;
      expect(bytesOf(ByteData.view(buffer, 2, 3)), [1, 2, 3]);
      expect(bytesOf(ByteData.view(buffer)), [9, 9, 1, 2, 3, 9]);
    },
  );

  group('a copy of the example', () {
    test('is opened, and the file it was copied from is gone again', () async {
      final folder = await root.createTemp('copy');
      final example = File('assets/example.wombat').readAsBytesSync();
      final doc = await openExampleCopy(
        example,
        folder: folder,
        to: 'a.wombat',
      );
      expect(doc.path, '${folder.path}/a.wombat');
      expect((await doc.grid(doc.root)).cell.text, 'Example');
      expect(folder.listSync().map((e) => e.path), [doc.path]);
      await doc.close();
      // Where there is a file already nothing is opened, and nothing else is left.
      await expectLater(
        openExampleCopy(example, folder: folder, to: 'a.wombat'),
        throwsA(
          isA<BridgeError>().having(
            (e) => e.kind,
            'kind',
            ErrorKind.alreadyExists,
          ),
        ),
      );
      expect(folder.listSync().map((e) => e.path), [doc.path]);
    });

    // The third review's fix, which the fourth found untested.
    test('is opened all the same when that file cannot be deleted', () async {
      final folder = await root.createTemp('stuck');
      final example = File('assets/example.wombat').readAsBytesSync();
      final doc = await IOOverrides.runZoned(
        () => openExampleCopy(example, folder: folder, to: 'a.wombat'),
        createFile: (path) {
          final real = Zone.root.run(() => File(path));
          return path.endsWith('/example-original.wombat')
              ? _Stuck(real)
              : real;
        },
      );
      expect((await doc.grid(doc.root)).cell.text, 'Example');
      expect(names(folder), ['a.wombat', 'example-original.wombat']);
      await doc.close();
    });
  });

  group('the app', () {
    testWidgets('starts with a copy of the example, and keeps it', (
      tester,
    ) async {
      // A folder that is not there yet, as at the very first start.
      final folder = Directory('${root.path}/first-${count++}/wombat');
      final file = File('${folder.path}/example.wombat');
      await tester.pumpWidget(app(folder));
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      await until(tester, () => key('probe-shown').evaluate().isNotEmpty);
      await until(tester, () => text(tester, 'probe-shown') == 'Shopping');
      // The copy, and nothing beside it.
      expect(folder.listSync().map((e) => e.path), [file.path]);
      // The file is the app's while it runs.
      await tester.runAsync(() async {
        try {
          await Session.open(file.path);
          fail('opened the file of the running app');
        } on BridgeError catch (error) {
          expect(error.kind, ErrorKind.alreadyOpen);
        }
      });

      // The app is taken away with something typed: the text is entered, the
      // document saved and closed, and its file free.
      await tester.enterText(key('probe-typing'), 'typed and left');
      await end(tester);
      expect(file.readAsStringSync(), contains('"typed and left"'));

      // The next start has the document as it was left.
      await tester.pumpWidget(app(folder));
      await until(tester, () => key('probe-shown').evaluate().isNotEmpty);
      await until(
        tester,
        () => text(tester, 'probe-shown') == 'typed and left',
      );
      expect(key('start-error'), findsNothing);
      await end(tester);
    });

    testWidgets('a document that cannot be opened: said, with a way out', (
      tester,
    ) async {
      final folder = Directory('${root.path}/broken-${count++}')..createSync();
      final file = File('${folder.path}/example.wombat');
      const theirs = 'half written by another program';
      file.writeAsStringSync(theirs);
      // What belongs to the document's name, and something that does not.
      Directory('${file.path}.images').createSync();
      File('${file.path}.images/a.png').writeAsStringSync('a picture');
      Directory('${folder.path}/.wombat-view').createSync();
      File('${folder.path}/.wombat-view/an-id.json').writeAsStringSync('{}');
      File('${folder.path}/other.txt').writeAsStringSync('not ours');
      await tester.pumpWidget(app(folder));
      await until(tester, () => key('start-error').evaluate().isNotEmpty);
      expect(
        text(tester, 'start-error'),
        startsWith('The document could not be opened. Error (syntax): '),
      );
      expect(find.textContaining(file.path), findsOneWidget);

      // Trying again changes nothing while the file is as it is.
      await press(tester, 'start-again');
      await until(tester, () => key('start-error').evaluate().isNotEmpty);
      expect(file.readAsStringSync(), theirs);

      // The way out: the file is put aside with what belongs to it (the fourth
      // review's M4: its pictures stayed, under the new document's name), and the
      // example is opened.
      await press(tester, 'start-over');
      await until(tester, () => key('probe-shown').evaluate().isNotEmpty);
      await until(tester, () => text(tester, 'probe-shown') == 'Shopping');
      final left = names(folder);
      expect(left, ['example.wombat', 'other.txt', startsWith('put-aside-')]);
      final aside = '${folder.path}/${left.last}';
      expect(File('$aside/example.wombat').readAsStringSync(), theirs);
      expect(
        File('$aside/example.wombat.images/a.png').readAsStringSync(),
        'a picture',
      );
      expect(File('$aside/.wombat-view/an-id.json').readAsStringSync(), '{}');
      await end(tester);
    });

    // The fourth review's (M1, M2): these got "already exists" and no way out.
    for (final (what, make) in <(String, void Function(String))>[
      ('a folder', (path) => Directory(path).createSync()),
      ('a link that leads nowhere', (path) => Link(path).createSync('$path-x')),
    ]) {
      testWidgets('$what in the document\'s place: the same way out', (
        tester,
      ) async {
        final folder = Directory('${root.path}/odd-${count++}')..createSync();
        final path = '${folder.path}/example.wombat';
        make(path);
        await tester.pumpWidget(app(folder));
        await until(tester, () => key('start-error').evaluate().isNotEmpty);
        expect(
          text(tester, 'start-error'),
          startsWith('The document could not be opened. Error (io): '),
        );
        await press(tester, 'start-over');
        await until(tester, () => key('probe-shown').evaluate().isNotEmpty);
        final left = names(folder);
        expect(left, ['example.wombat', startsWith('put-aside-')]);
        // Moved as what it is, not followed and not emptied.
        expect(
          FileSystemEntity.typeSync(
            '${folder.path}/${left.last}/example.wombat',
            followLinks: false,
          ),
          what == 'a folder'
              ? FileSystemEntityType.directory
              : FileSystemEntityType.link,
        );
        await end(tester);
      });
    }

    testWidgets(
      'a folder that cannot be made: said, with nothing to put aside',
      (tester) async {
        final file = File('${root.path}/a-file-${count++}')
          ..writeAsStringSync('');
        await tester.pumpWidget(app(Directory('${file.path}/in')));
        await until(tester, () => key('start-error').evaluate().isNotEmpty);
        expect(
          text(tester, 'start-error'),
          startsWith('The document could not be opened. Error: '),
        );
        expect(key('start-again'), findsOneWidget);
        expect(key('start-over'), findsNothing);
        await tester.pumpWidget(const SizedBox());
      },
    );

    testWidgets('a document that is open already is not put aside', (
      tester,
    ) async {
      final folder = Directory('${root.path}/taken-${count++}')..createSync();
      final file = '${folder.path}/example.wombat';
      final holder = (await tester.runAsync(() => Session.create(file)))!;
      await tester.pumpWidget(app(folder));
      await until(tester, () => key('start-error').evaluate().isNotEmpty);
      expect(
        text(tester, 'start-error'),
        startsWith('The document could not be opened. Error (alreadyOpen): '),
      );
      // Its file is whole, so there is only trying again, which works once it is free.
      expect(key('start-over'), findsNothing);
      await tester.runAsync(holder.close);
      await press(tester, 'start-again');
      await until(tester, () => find.byType(Kept).evaluate().isNotEmpty);
      await end(tester);
    });

    // Read by the fourth review: the second of two first starts was offered to put
    // the first one's live document aside.
    testWidgets('a document that another start made meanwhile is not either', (
      tester,
    ) async {
      final folder = Directory('${root.path}/twice-${count++}')..createSync();
      final file = File('${folder.path}/example.wombat');
      final example = File('assets/example.wombat').readAsBytesSync();
      var first = true;
      await tester.pumpWidget(
        DefaultAssetBundle(
          bundle: _Assets(() {
            if (first) file.writeAsBytesSync(example);
            first = false;
          }),
          child: app(folder),
        ),
      );
      await until(tester, () => key('start-error').evaluate().isNotEmpty);
      expect(
        text(tester, 'start-error'),
        startsWith('The document could not be opened. Error (alreadyExists): '),
      );
      expect(key('start-over'), findsNothing);
      // Trying again opens what is there now.
      await press(tester, 'start-again');
      await until(tester, () => key('probe-shown').evaluate().isNotEmpty);
      expect(names(folder), ['example.wombat']);
      await end(tester);
    });
  });

  group('putting a document aside', () {
    test('twice in the same instant: two folders, nothing replaced', () async {
      final folder = await root.createTemp('aside');
      final file = File('${folder.path}/example.wombat');
      final now = DateTime(2026, 11, 12, 9, 30);
      file.writeAsStringSync('first');
      final first = await putAside(folder, now: now);
      file.writeAsStringSync('second');
      final second = await putAside(folder, now: now);
      expect(first.path, isNot(second.path));
      for (final aside in [first, second]) {
        expect(aside.path, startsWith('${folder.path}/put-aside-2026-11-12-'));
      }
      expect(File('${first.path}/example.wombat').readAsStringSync(), 'first');
      expect(
        File('${second.path}/example.wombat').readAsStringSync(),
        'second',
      );
      // With nothing there, an empty folder is all.
      final third = await putAside(folder, now: now);
      expect(third.listSync(), isEmpty);
    });

    test('is a way out only when something is there that is not taken', () {
      final folder = root.createTempSync('whether');
      const syntax = BridgeError(kind: ErrorKind.syntax, message: 'x');
      expect(canBePutAside(syntax, folder), isFalse);
      File('${folder.path}/example.wombat').writeAsStringSync('x');
      expect(canBePutAside(syntax, folder), isTrue);
      expect(canBePutAside(StateError('x'), folder), isTrue);
      for (final kind in [ErrorKind.alreadyOpen, ErrorKind.alreadyExists]) {
        expect(
          canBePutAside(BridgeError(kind: kind, message: 'x'), folder),
          isFalse,
        );
      }
    });
  });

  group('the app is closed', () {
    // The fourth review's (G1, M3): closing the window lost the editor's text and
    // every change younger than the save clock.
    testWidgets('the system asks to exit: all is in the file first', (
      tester,
    ) async {
      final (_, file) = await started(tester);
      final keeper = keeperOf(tester);
      await tester.enterText(
        key('probe-typing'),
        'typed before the window closed',
      );
      expect(await exitAsked(tester), AppExitResponse.exit);
      // When the answer is given, not some time after it.
      expect(
        file.readAsStringSync(),
        contains('"typed before the window closed"'),
      );
      expect(keeper.ended, 'The document was closed.');
      await tester.pumpWidget(const SizedBox());
    });

    testWidgets('a document that cannot be saved holds the exit back, once', (
      tester,
    ) async {
      final (_, file) = await started(tester);
      final keeper = keeperOf(tester);
      await tester.enterText(key('probe-typing'), 'mine');
      final theirs = '${file.readAsStringSync()}\n';
      file.writeAsStringSync(theirs);
      expect(await exitAsked(tester), AppExitResponse.cancel);
      await tester.pump();
      // The user sees why, and the document is open for the choice.
      expect(text(tester, 'probe-state'), 'changedOnDisk');
      expect(keeper.ended, isNull);
      // Asked again with nothing better, the app leaves; the other program's file
      // is not written over.
      expect(await exitAsked(tester), AppExitResponse.exit);
      expect(file.readAsStringSync(), theirs);
      expect(keeper.ended, isNotNull);
      await tester.pumpWidget(const SizedBox());
    });

    testWidgets('held back again after a save that worked in between', (
      tester,
    ) async {
      final (_, file) = await started(tester);
      final keeper = keeperOf(tester);
      for (final mine in ['mine', 'mine again']) {
        await tester.enterText(key('probe-typing'), mine);
        file.writeAsStringSync('${file.readAsStringSync()}\n');
        expect(await exitAsked(tester), AppExitResponse.cancel);
        await tester.runAsync(keeper.keepMine);
        expect(keeper.state, KeeperState.saved);
      }
      expect(await exitAsked(tester), AppExitResponse.exit);
      expect(file.readAsStringSync(), contains('"mine again"'));
      await tester.pumpWidget(const SizedBox());
    });
  });

  // On a stand-in for the core, with the clock of the test.
  group('the document is kept', () {
    const out = [
      AppLifecycleState.inactive,
      AppLifecycleState.hidden,
      AppLifecycleState.paused,
      AppLifecycleState.detached,
    ];

    int calls(FakeDocument fake, String call) =>
        fake.log.where((name) => name == call).length;

    void keptTest(
      String name,
      Future<void> Function(
        WidgetTester tester,
        Keeper keeper,
        FakeDocument fake,
      )
      body,
    ) {
      testWidgets(name, (tester) async {
        addTearDown(
          () => tester.binding.handleAppLifecycleStateChanged(
            AppLifecycleState.resumed,
          ),
        );
        final fake = FakeDocument();
        final keeper = Keeper(await Session.over(fake));
        await tester.pumpWidget(
          MaterialApp(
            home: Kept(keeper: keeper, child: ProbeScreen(keeper)),
          ),
        );
        await tester.pump();
        expect(text(tester, 'probe-shown'), 'one');
        await body(tester, keeper, fake);
        await tester.pumpWidget(const SizedBox());
        await endKeeping(keeper);
      });
    }

    Future<void> lifecycle(WidgetTester tester, AppLifecycleState state) async {
      tester.binding.handleAppLifecycleStateChanged(state);
      await tester.pump();
    }

    keptTest(
      'saved when the app is no longer in front, without what is typed',
      (tester, keeper, fake) async {
        await keeper.apply(const Command.enterText(cell: 'd', text: 'entered'));
        await tester.enterText(key('probe-typing'), 'half a sentence');
        // A window that loses the focus, a dialog of the system: not leaving.
        await lifecycle(tester, AppLifecycleState.inactive);
        expect((fake.saves, fake.saved), (1, true));
        expect((calls(fake, 'apply'), fake.cells['a']!.text), (1, 'one'));
        await lifecycle(tester, AppLifecycleState.resumed);
        expect(calls(fake, 'save'), 1);
      },
    );

    for (var steps = 2; steps <= out.length; steps++) {
      final state = out[steps - 1];
      keptTest('put away when the app is ${state.name}, with what is typed', (
        tester,
        keeper,
        fake,
      ) async {
        await tester.enterText(key('probe-typing'), 'half a sentence');
        for (final step in out.take(steps)) {
          await lifecycle(tester, step);
        }
        expect(fake.cells['a']!.text, 'half a sentence');
        // One editing, held and written once, however many steps the way has.
        expect(
          (calls(fake, 'beginGroup'), fake.saves, fake.saved),
          (1, 1, true),
        );
        expect(calls(fake, 'endGroup'), 0);
        expect(fake.log, containsAllInOrder(['retype.', 'apply.', 'save']));
        expect(text(tester, 'probe-state'), 'saved');
      });
    }

    // No test noticed when `paused` and `detached` did nothing: Flutter passes
    // through `hidden` first. Each is named so that this does not rest on that.
    for (final state in [
      AppLifecycleState.paused,
      AppLifecycleState.detached,
    ]) {
      keptTest('put away when the app is ${state.name} without more ado', (
        tester,
        keeper,
        fake,
      ) async {
        await tester.enterText(key('probe-typing'), 'half a sentence');
        // As an observer is told, with no states made up in between.
        final kept = tester.state<State<Kept>>(find.byType(Kept));
        (kept as WidgetsBindingObserver).didChangeAppLifecycleState(state);
        await tester.pump();
        expect((fake.cells['a']!.text, fake.saved), ('half a sentence', true));
      });
    }

    keptTest('what has gone is no longer told that the app is put away', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(const Command.enterText(cell: 'd', text: 'entered'));
      await tester.pumpWidget(const SizedBox());
      // ignore: invalid_use_of_protected_member
      expect(keeper.hasListeners, isFalse);
      await lifecycle(tester, AppLifecycleState.inactive);
      await lifecycle(tester, AppLifecycleState.hidden);
      expect(calls(fake, 'save'), 0);
      expect(await tester.binding.handleRequestAppExit(), AppExitResponse.exit);
      expect((calls(fake, 'save'), keeper.ended), (0, null));
    });

    keptTest('after the end of the work nothing is asked, and nothing thrown', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.close();
      final before = fake.log.length;
      for (final state in [...out, AppLifecycleState.resumed]) {
        await lifecycle(tester, state);
      }
      expect(await tester.binding.handleRequestAppExit(), AppExitResponse.exit);
      expect(fake.log.length, before);
    });

    keptTest(
      'the work ends before the turn of what was asked: nothing thrown',
      (tester, keeper, fake) async {
        // The document is being let go of, behind a call that is still at work, when
        // the app is put away.
        final gate = holdFirst(fake, 'grid');
        final read = keeper.grid('a');
        final gone = keeper.letGo();
        await lifecycle(tester, AppLifecycleState.inactive);
        await lifecycle(tester, AppLifecycleState.hidden);
        gate.complete();
        await read;
        await gone;
        await tester.pump();
        expect(keeper.ended, 'The document was closed.');
        expect(calls(fake, 'save'), 0);
      },
    );

    keptTest('the exit waits for the document, which is closed', (
      tester,
      keeper,
      fake,
    ) async {
      await tester.enterText(key('probe-typing'), 'typed');
      final gate = holdFirst(fake, 'apply');
      AppExitResponse? answer;
      unawaited(
        tester.binding.handleRequestAppExit().then((said) => answer = said),
      );
      await tester.pump(const Duration(seconds: 1));
      expect(answer, isNull);
      gate.complete();
      await tester.pump();
      expect(answer, AppExitResponse.exit);
      expect(
        (fake.cells['a']!.text, fake.saves, fake.closed),
        ('typed', 1, true),
      );
    });

    keptTest('a save that hangs does not keep the app from being closed', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(const Command.enterText(cell: 'd', text: 'entered'));
      final gate = holdFirst(fake, 'save');
      AppExitResponse? answer;
      unawaited(
        tester.binding.handleRequestAppExit().then((said) => answer = said),
      );
      // Five seconds, and not one more.
      await tester.pump(const Duration(milliseconds: 4999));
      expect(answer, isNull);
      await tester.pump(const Duration(milliseconds: 1));
      expect(answer, AppExitResponse.exit);
      gate.complete();
    });

    keptTest('what cannot be entered holds the exit back, once', (
      tester,
      keeper,
      fake,
    ) async {
      final wrong = keeper.attach(finish: () async => throw StateError('no'));
      expect(
        await tester.binding.handleRequestAppExit(),
        AppExitResponse.cancel,
      );
      expect((keeper.ended, fake.closed), (null, false));
      expect(await tester.binding.handleRequestAppExit(), AppExitResponse.exit);
      // With no one to ask it was let go of.
      expect((keeper.ended, fake.disposed), ('The document was closed.', true));
      wrong.detach();
    });

    keptTest('a fault while it is put away holds the exit back, once', (
      tester,
      keeper,
      fake,
    ) async {
      await keeper.apply(const Command.enterText(cell: 'd', text: 'entered'));
      failFirst(
        fake,
        'save',
        const BridgeError(kind: ErrorKind.internal, message: 'half changed'),
      );
      expect(
        await tester.binding.handleRequestAppExit(),
        AppExitResponse.cancel,
      );
      expect(keeper.ended, faultSentence);
      expect(await tester.binding.handleRequestAppExit(), AppExitResponse.exit);
    });

    testWidgets('given another keeper, it tells that one', (tester) async {
      final fakes = [FakeDocument(), FakeDocument()];
      final keepers = [
        for (final fake in fakes) Keeper(await Session.over(fake)),
      ];
      for (final keeper in keepers) {
        await tester.pumpWidget(
          MaterialApp(
            home: Kept(keeper: keeper, child: const SizedBox()),
          ),
        );
        await keeper.apply(const Command.enterText(cell: 'a', text: 'typed'));
      }
      // ignore: invalid_use_of_protected_member
      expect(keepers.first.hasListeners, isFalse);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      await tester.pump();
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      expect((fakes.first.saves, fakes.last.saves), (0, 1));
      // An exit held back for one document is not one held back for the next.
      Future<AppExitResponse> asked() => tester.binding.handleRequestAppExit();
      await keepers.last.undo();
      fakes.last.fileChanged = true;
      expect(await asked(), AppExitResponse.cancel);
      final third = FakeDocument()..fileChanged = true;
      final another = Keeper(await Session.over(third));
      await tester.pumpWidget(
        MaterialApp(
          home: Kept(keeper: another, child: const SizedBox()),
        ),
      );
      await another.apply(const Command.enterText(cell: 'a', text: 'typed'));
      expect(await asked(), AppExitResponse.cancel);
      for (final keeper in [...keepers, another]) {
        await endKeeping(keeper);
      }
    });
  });

  // The app with the screen it really shows, and what the start screen does for it:
  // the test documents of the prototype, and its log.
  group('with the phone screen', () {
    String? line(WidgetTester tester, String name) =>
        key(name).evaluate().isEmpty ? null : text(tester, name);

    /// Asks the core something, and waits for it as `until` waits.
    Future<T> real<T>(WidgetTester tester, Future<T> Function() work) async {
      T? value;
      var done = false;
      unawaited(
        work().then((answer) {
          value = answer;
          done = true;
        }),
      );
      await until(tester, () => done);
      return value as T;
    }

    Future<Directory> startedPhone(WidgetTester tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(400, 800);
      addTearDown(tester.view.reset);
      final folder = Directory('${root.path}/phone-${count++}');
      await tester.pumpWidget(WombatApp(folder: folder));
      await until(tester, () => line(tester, 'title') == 'Example');
      await until(tester, () => key('stage-across').evaluate().isNotEmpty);
      return folder;
    }

    /// Chooses a test document from the menu, and waits until its level shows.
    Future<void> show(WidgetTester tester, String name, String title) async {
      await tester.tap(key('menu'));
      await tester.pumpAndSettle();
      await tester.tap(key('document-$name'));
      await until(
        tester,
        () =>
            key('opening').evaluate().isEmpty &&
            key('reading').evaluate().isEmpty &&
            line(tester, 'title') == title &&
            line(tester, 'state') == 'Saved',
      );
      await tester.pumpAndSettle();
    }

    testWidgets('the app shows its document, and keeps what is typed on it', (
      tester,
    ) async {
      final folder = await startedPhone(tester);
      // No banner of the debug build over the menu.
      expect(
        tester.widget<MaterialApp>(find.byType(MaterialApp)),
        isA<MaterialApp>().having(
          (app) => app.debugShowCheckedModeBanner,
          'banner',
          isFalse,
        ),
      );
      await tester.tap(key('cell-1-0'));
      await until(tester, () => key('editor').evaluate().isNotEmpty);
      tester.testTextInput.enterText('typed and left');
      // The app is taken away with the editor open: the text is entered, which
      // the keeper waits for before it closes the document.
      await end(tester);
      expect(
        File('${folder.path}/example.wombat').readAsStringSync(),
        contains('"typed and left"'),
      );
      // What was done is in the prototype log, which is in the app's folder.
      final log = File('${folder.path}/$logName');
      await until(tester, log.existsSync);
      expect(log.readAsLinesSync().first, contains('"what":"edit"'));
      expect(log.readAsStringSync(), isNot(contains('typed and left')));
    });

    testWidgets('the test documents are in the menu, each made when first asked '
        'for', (tester) async {
      final folder = await startedPhone(tester);
      // Something typed and not said yes to goes with the document it was typed
      // in, when another is shown.
      await tester.tap(key('cell-1-0'));
      await until(tester, () => key('editor').evaluate().isNotEmpty);
      tester.testTextInput.enterText('left for another document');
      final example = keeperOf(tester);
      await show(tester, 'forms', 'Depth forms');
      expect(example.ended, 'The document was closed.');
      expect(key('editor'), findsNothing);
      expect(
        File('${folder.path}/example.wombat').readAsStringSync(),
        contains('"left for another document"'),
      );
      expect(keeperOf(tester).path, '${folder.path}/depth-forms.wombat');
      // Each form of the depth budget is on its first screen, at this width.
      for (final form in ['full-1-0', 'preview-1-1']) {
        expect(key(form), findsOneWidget, reason: form);
      }
      await tester.drag(key('stage-down-0'), const Offset(0, -300));
      await tester.pumpAndSettle();
      expect(key('shape-1-2/0-0'), findsOneWidget);
      // A Badge is what a grid folded on this device shows: its count.
      expect(
        find.descendant(of: key('badge-1-3'), matching: find.text('12')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);

      await show(tester, 'empty', 'Wombat');
      expect(key('empty-root'), findsOneWidget);
      // The one that is shown cannot be chosen again.
      await tester.tap(key('menu'));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<PopupMenuItem<VoidCallback>>(key('document-empty'))
            .enabled,
        isFalse,
      );
      expect(
        tester
            .widget<PopupMenuItem<VoidCallback>>(key('document-example'))
            .enabled,
        isTrue,
      );
      await tester.tap(key('document-example'));
      await until(tester, () => line(tester, 'title') == 'Example');
      await until(tester, () => key('cell-1-0').evaluate().isNotEmpty);
      await until(
        tester,
        () =>
            tester
                .widget<Text>(
                  find.descendant(
                    of: key('cell-1-0'),
                    matching: find.byType(Text),
                  ),
                )
                .textSpan!
                .toPlainText() ==
            'left for another document',
      );
      expect(
        names(folder),
        containsAll(['depth-forms.wombat', 'empty.wombat', 'example.wombat']),
      );
      // Made once: the next time it is the file as it was left.
      final made = File('${folder.path}/depth-forms.wombat').lastModifiedSync();
      await show(tester, 'forms', 'Depth forms');
      expect(
        File('${folder.path}/depth-forms.wombat').lastModifiedSync(),
        made,
      );
      await end(tester);
    });

    testWidgets('the large and the six-level document open, and are what they '
        'say', (tester) async {
      await startedPhone(tester);
      await show(tester, 'nested', 'Six levels');
      var keeper = keeperOf(tester);
      var grid = await real(tester, () => keeper.stage(keeper.root, phoneAsk));
      expect((grid.grids.first.xs, grid.grids.first.ys), (12, 20));
      // Six levels of grids below the root's own.
      var cell = keeper.root;
      for (var level = 0; level < 6; level++) {
        final at = cell;
        final cells = (await real(tester, () => keeper.grid(at))).cells;
        expect(cells, isNotEmpty, reason: 'level $level');
        cell = cells.first.id;
      }
      final last = cell;
      expect((await real(tester, () => keeper.grid(last))).cells, isEmpty);
      await show(tester, 'large', 'Large');
      keeper = keeperOf(tester);
      final large = keeper;
      grid = await real(tester, () => large.stage(large.root, phoneAsk));
      expect((grid.grids.first.xs, grid.grids.first.ys), (10, 100));
      expect(grid.summaries.first.xs * grid.summaries.first.ys, 100);
      expect(key('preview-0-0'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await end(tester);
    });

    testWidgets(
      'a test document that cannot be opened: said, with the way back '
      'to the example',
      (tester) async {
        final folder = await startedPhone(tester);
        File('${folder.path}/empty.wombat').writeAsStringSync('half written');
        await tester.tap(key('menu'));
        await tester.pumpAndSettle();
        await tester.tap(key('document-empty'));
        await until(tester, () => key('start-error').evaluate().isNotEmpty);
        expect(
          text(tester, 'start-error'),
          startsWith('The document could not be opened. Error (syntax): '),
        );
        // Not put aside: only the app's own document is.
        expect(key('start-over'), findsNothing);
        await tester.tap(key('start-again'));
        await until(tester, () => key('start-error').evaluate().isNotEmpty);
        await tester.tap(key('start-example'));
        await until(tester, () => line(tester, 'title') == 'Example');
        expect(
          File('${folder.path}/empty.wombat').readAsStringSync(),
          'half written',
        );
        await end(tester);
      },
    );

    test(
      'a log that is exported goes where the system has room for it',
      () async {
        // The folder the system shares with the user, and the app's own where
        // the system has none, or cannot say.
        final fallback = Directory('/fallback');
        expect(
          (await exportFolder(
            fallback,
            shared: () async => Directory('/downloads'),
          )).path,
          '/downloads',
        );
        expect(
          (await exportFolder(fallback, shared: () async => null)).path,
          '/fallback',
        );
        expect(
          (await exportFolder(
            fallback,
            shared: () => throw UnsupportedError('no such folder here'),
          )).path,
          '/fallback',
        );
      },
    );

    testWidgets('exported from the app: into a folder beside its documents', (
      tester,
    ) async {
      final folder = await startedPhone(tester);
      await tester.tap(key('menu'));
      await tester.pumpAndSettle();
      await tester.tap(key('menu-settings'));
      await tester.pumpAndSettle();
      await tester.tap(key('variant-pin'));
      await tester.pump();
      await tester.ensureVisible(key('log-export'));
      await tester.tap(key('log-export'));
      await until(tester, () => key('log-said').evaluate().isNotEmpty);
      final exported = Directory('${folder.path}/exports').listSync();
      expect(exported.single.path, endsWith('.jsonl'));
      expect(
        File(exported.single.path).readAsStringSync(),
        contains('"what":"variant"'),
      );
      await tester.tap(key('settings-close'));
      await tester.pumpAndSettle();
      // The switches are the app's, not a document's: they stay when another
      // document is shown.
      await show(tester, 'forms', 'Depth forms');
      expect(key('stage-first-row'), findsNothing);
      await end(tester);
    });

    testWidgets('the bridge check is behind the menu', (tester) async {
      await startedPhone(tester);
      await tester.tap(key('menu'));
      await tester.pumpAndSettle();
      await tester.tap(key('bridge-check'));
      await tester.pumpAndSettle();
      expect(find.text('Press Open.'), findsOneWidget);
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(line(tester, 'title'), 'Example');
      await end(tester);
    });
  });
}
