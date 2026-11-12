// What the tests of the phone screen share: how they find things by their keys, press
// and type as a user does, and put the screen on a keeper as the app does.
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:wombat_app/keeper.dart';
import 'package:wombat_app/main.dart' show Kept;
import 'package:wombat_app/phone/log.dart';
import 'package:wombat_app/phone/phone_screen.dart';
import 'package:wombat_app/phone/variants.dart';

Finder key(String name) => find.byKey(Key(name));

bool there(String name) => key(name).evaluate().isNotEmpty;

/// The text widget of a cell, by the cell's name (`x-y`, or `x-y/i-j` for a cell of
/// the Full grid inside that one).
Text cellText(WidgetTester tester, String name) => tester.widget<Text>(
  find.descendant(of: key('cell-$name'), matching: find.byType(Text)),
);

/// What a cell shows.
String shown(WidgetTester tester, String name) =>
    cellText(tester, name).textSpan!.toPlainText();

/// The text of a line of the screen (`title`, `state`, `message`, `trouble` and so
/// on), or null if the line is not there.
String? line(WidgetTester tester, String name) =>
    there(name) ? tester.widget<Text>(key(name)).data : null;

String editor(WidgetTester tester) =>
    tester.widget<TextField>(key('editor')).controller!.text;

TextEditingController editorField(WidgetTester tester) =>
    tester.widget<TextField>(key('editor')).controller!;

bool focused(WidgetTester tester) =>
    tester.widget<TextField>(key('editor')).focusNode!.hasFocus;

/// A tap on the thing with this key, and one frame. A button of a bar is scrolled
/// into view first: a narrow screen holds some of them out of sight. (A cell is
/// not: scrolled to the edge it would lie under the column that stays in view.)
Future<void> press(WidgetTester tester, String name) async {
  if (!name.startsWith('cell-') && !name.startsWith('child-')) {
    await tester.ensureVisible(key(name));
    await tester.pump();
  }
  await tester.tap(key(name));
  await tester.pump();
}

/// Scrolls the thing with this key to the middle of what is seen, both ways.
Future<void> reveal(WidgetTester tester, String name) async {
  await Scrollable.ensureVisible(tester.element(key(name)), alignment: 0.5);
  await tester.pumpAndSettle();
}

/// Opens a menu and chooses an entry of it, waiting for the menu to come and go.
Future<void> choose(WidgetTester tester, String menu, String entry) async {
  await press(tester, menu);
  await tester.pumpAndSettle();
  await tester.tap(key(entry));
  await tester.pumpAndSettle();
}

/// Types into the editor as a user can: into whatever has the keyboard, which is
/// the editor only while it has the focus. (`tester.enterText` would give it the
/// focus first, and so pass where a user's keys go nowhere.)
void type(WidgetTester tester, String text) =>
    tester.testTextInput.enterText(text);

/// The keyboard's own Return.
Future<void> pressReturn(WidgetTester tester) async {
  await tester.testTextInput.receiveAction(TextInputAction.next);
  await tester.pump();
}

/// Lets the animation of a level that was arrived at run out.
Future<void> arrive(WidgetTester tester) =>
    tester.pump(const Duration(milliseconds: 400));

Future<void> lifecycle(WidgetTester tester, AppLifecycleState state) async {
  tester.binding.handleAppLifecycleStateChanged(state);
  await tester.pump();
}

/// The system takes an app away step by step, and brings it back the same way.
const away = [
  AppLifecycleState.inactive,
  AppLifecycleState.hidden,
  AppLifecycleState.paused,
  AppLifecycleState.detached,
];

/// The app goes out of sight, as far as `to`, and comes back to the front.
Future<void> awayAndBack(WidgetTester tester, AppLifecycleState to) async {
  final steps = away.sublist(0, away.indexOf(to) + 1);
  for (final step in steps) {
    await lifecycle(tester, step);
  }
  for (final step in steps.reversed.skip(1)) {
    await lifecycle(tester, step);
  }
  await lifecycle(tester, AppLifecycleState.resumed);
}

/// The system's back, as the edge swipe or the back button of Android sends it.
/// Leaving the app is asked of the system by a message, which is counted.
Future<List<String>> back(WidgetTester tester, {int times = 1}) async {
  final left = <String>[];
  tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
    SystemChannels.platform,
    (call) async {
      if (call.method == 'SystemNavigator.pop') left.add(call.method);
      return null;
    },
  );
  for (var i = 0; i < times; i++) {
    await tester.binding.handlePopRoute();
  }
  // No frame is due for a message of the system: the first only lets the action
  // run, the second draws what it did.
  await tester.pump();
  await tester.pump();
  return left;
}

/// The size of the screen for a test, in logical pixels; put back at its end.
void screenSize(WidgetTester tester, Size size) {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.reset);
}

/// A phone held upright.
const phone = Size(400, 800);

/// The phone screen on `keeper`, kept as the app keeps its document.
Widget phoneApp(
  Keeper keeper, {
  ValueNotifier<Variants>? variants,
  ProtoLog? log,
  double textScale = 1,
  Future<Directory> Function()? exportFolder,
  List<PopupMenuEntry<VoidCallback>> menu = const [],
}) => MaterialApp(
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(context)
        .copyWith(textScaler: TextScaler.linear(textScale)),
    child: child!,
  ),
  home: Kept(
    keeper: keeper,
    child: PhoneScreen(
      key: ObjectKey(keeper),
      keeper: keeper,
      variants: variants ?? ValueNotifier(const Variants()),
      log: log ?? ProtoLog(),
      exportFolder: exportFolder,
      menu: menu,
    ),
  ),
);
