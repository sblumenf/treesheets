// The app as it starts, without a device, on the real core: where its document is, the
// first start and the next, a document that cannot be opened and the way out, and the
// document going with its screen. The core answers in real time, which a widget test
// does not let pass by itself: `until` does, and waits for the thing, not for a time.
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/example.dart';
import 'package:wombat_app/grid/grid_screen.dart';
import 'package:wombat_app/grid/stage.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/main.dart';
import 'package:wombat_app/session.dart';
import 'package:wombat_app/src/rust/api/wire.dart';

import 'fake_document.dart';
import 'support.dart';

void main() {
  late Directory root;
  var count = 0;

  setUpAll(() async {
    await loadBridge();
    root = await Directory.systemTemp.createTemp('wombat-main');
  });

  tearDownAll(() => root.delete(recursive: true));

  Finder key(String name) => find.byKey(Key(name));

  String title(WidgetTester tester) => tester.widget<Text>(key('title')).data!;

  String shown(WidgetTester tester, int index) => tester
      .widget<Text>(
        find.descendant(of: key('cell-$index'), matching: find.byType(Text)),
      )
      .data!;

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

  /// Takes the app off the screen and waits until its document is closed.
  Future<void> end(WidgetTester tester) async {
    final stage = tester.widget<GridScreen>(find.byType(GridScreen)).stage;
    await tester.pumpWidget(const SizedBox());
    await until(tester, () => stage.ended != null);
  }

  group('where the document is kept', () {
    String at(String os, Map<String, String> environment) =>
        documentFolder(os: os, environment: environment, temp: '/tmp/x');

    test('on Linux, where the user\'s data is', () {
      expect(
        at('linux', {'HOME': '/home/u', 'XDG_DATA_HOME': '/data/u'}),
        '/data/u/wombat',
      );
      expect(at('linux', {'HOME': '/home/u'}), '/home/u/.local/share/wombat');
      // Empty, or not from the root: not valid, so it is as if not set.
      expect(
        at('linux', {'HOME': '/home/u', 'XDG_DATA_HOME': ''}),
        '/home/u/.local/share/wombat',
      );
      expect(
        at('linux', {'HOME': '/home/u', 'XDG_DATA_HOME': 'data'}),
        '/home/u/.local/share/wombat',
      );
      expect(at('linux', {'HOME': ''}), '/tmp/x/wombat');
      expect(at('linux', {}), '/tmp/x/wombat');
    });

    test('on macOS, in Application Support', () {
      expect(
        at('macos', {'HOME': '/Users/u', 'XDG_DATA_HOME': '/data/u'}),
        '/Users/u/Library/Application Support/wombat',
      );
      expect(at('macos', {}), '/tmp/x/wombat');
    });

    test('on Windows, in the roaming application data', () {
      expect(
        documentFolder(
          os: 'windows',
          environment: {'APPDATA': r'C:\Users\u\AppData\Roaming', 'HOME': '/h'},
          temp: r'C:\Temp',
        ),
        r'C:\Users\u\AppData\Roaming\wombat',
      );
      expect(
        documentFolder(os: 'windows', environment: {}, temp: r'C:\Temp'),
        r'C:\Temp\wombat',
      );
    });

    test('on a phone, in the app\'s own temporary folder', () {
      // Whatever the environment says there: it is not the app's to write to.
      const environment = {
        'HOME': '/',
        'XDG_DATA_HOME': '/data',
        'APPDATA': 'x',
      };
      expect(at('android', environment), '/tmp/x/wombat');
      expect(at('ios', environment), '/tmp/x/wombat');
      expect(at('fuchsia', environment), '/tmp/x/wombat');
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
  });

  group('the document goes with its screen', () {
    Future<(Stage, FakeDocument)> standIn() async {
      final fake = FakeDocument();
      return (await Stage.open(Keeper(await Session.over(fake))), fake);
    }

    testWidgets('saved and closed', (tester) async {
      final (stage, fake) = await standIn();
      await stage.enterText('a', 'typed');
      await endKeeping(stage);
      expect((fake.saves, fake.closed, fake.disposed), (1, true, true));
    });

    testWidgets('let go of when it cannot be saved, and no error for that', (
      tester,
    ) async {
      final (stage, fake) = await standIn();
      await stage.enterText('a', 'typed');
      fake.fileChanged = true;
      await endKeeping(stage);
      expect((fake.closed, fake.disposed), (false, true));
      expect((fake.saves, fake.overwrites), (0, 0));
      expect(stage.ended, 'The document was closed.');
      await tester.pump(const Duration(minutes: 5));
      expect(fake.log.last, 'close.');
    });
  });

  group('the app', () {
    testWidgets('starts with a copy of the example, and keeps it', (
      tester,
    ) async {
      // A folder that is not there yet, as at the very first start.
      final folder = Directory('${root.path}/first-${count++}/wombat');
      final file = File('${folder.path}/example.wombat');
      await tester.pumpWidget(WombatApp(folder: folder));
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      await until(tester, () => key('title').evaluate().isNotEmpty);
      expect(title(tester), 'Example');
      expect(shown(tester, 0), 'Shopping');
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

      await press(tester, 'cell-1');
      tester.testTextInput.enterText('typed and left');
      // The app is taken away with the editor open: the text is entered, the
      // document saved and closed, and its file free.
      await end(tester);
      expect(file.readAsStringSync(), contains('"typed and left"'));

      // The next start has the document as it was left.
      await tester.pumpWidget(WombatApp(folder: folder));
      await until(tester, () => key('title').evaluate().isNotEmpty);
      expect(shown(tester, 1), 'typed and left');
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
      await tester.pumpWidget(WombatApp(folder: folder));
      await until(tester, () => key('start-error').evaluate().isNotEmpty);
      final said = tester.widget<Text>(key('start-error')).data!;
      expect(
        said,
        startsWith('The document could not be opened. Error (syntax): '),
      );
      expect(find.textContaining(file.path), findsOneWidget);

      // Trying again changes nothing while the file is as it is.
      await press(tester, 'start-again');
      await until(tester, () => key('start-error').evaluate().isNotEmpty);
      expect(file.readAsStringSync(), theirs);

      // The way out: the file gets another name, and the example is opened.
      await press(tester, 'start-over');
      await until(tester, () => key('title').evaluate().isNotEmpty);
      expect(title(tester), 'Example');
      final names = folder.listSync().map((e) => e.path).toList()..sort();
      expect(names.length, 2);
      expect(names.first, matches(RegExp(r'/example-put-aside-\d+\.wombat$')));
      expect(File(names.first).readAsStringSync(), theirs);
      expect(names.last, file.path);
      await end(tester);
    });

    testWidgets(
      'a folder that cannot be made: said, with nothing to put aside',
      (tester) async {
        final file = File('${root.path}/a-file-${count++}')
          ..writeAsStringSync('');
        await tester.pumpWidget(
          WombatApp(folder: Directory('${file.path}/in')),
        );
        await until(tester, () => key('start-error').evaluate().isNotEmpty);
        expect(
          tester.widget<Text>(key('start-error')).data,
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
      await tester.pumpWidget(WombatApp(folder: folder));
      await until(tester, () => key('start-error').evaluate().isNotEmpty);
      expect(
        tester.widget<Text>(key('start-error')).data,
        startsWith('The document could not be opened. Error (alreadyOpen): '),
      );
      // Its file is whole, so there is only trying again, which works once it is free.
      expect(key('start-over'), findsNothing);
      await tester.runAsync(holder.close);
      await press(tester, 'start-again');
      await until(tester, () => key('title').evaluate().isNotEmpty);
      expect(title(tester), 'Wombat');
      await end(tester);
    });
  });
}
