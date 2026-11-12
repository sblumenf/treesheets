// The plain-text check of the bridge on a real device (or desktop): presses its three
// buttons and reads the screen.
// Run with: flutter test integration_test/app_test.dart -d linux   (or -d <a device>)
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:wombat_app/demo_screen.dart';
import 'package:wombat_app/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  String output(WidgetTester tester) =>
      tester.widget<SelectableText>(find.byKey(const Key('output'))).data!;

  Future<void> press(WidgetTester tester, String key) async {
    await tester.tap(find.byKey(Key(key)));
    await tester.pumpAndSettle();
  }

  testWidgets('open, change and undo on the screen', (tester) async {
    await tester.pumpWidget(const MaterialApp(home: DemoScreen()));
    expect(output(tester), 'Press Open.');

    await press(tester, 'open');
    expect(output(tester), contains('"Example"  (2 x 3)'));
    expect(output(tester), contains('"=SUM(Costs)" = "10"'));

    await press(tester, 'change');
    expect(output(tester), contains('change report\n  changed: '));
    expect(output(tester), contains('"=SUM(Costs)" = "38"'));

    await press(tester, 'undo');
    expect(output(tester), startsWith('Undone'));
    expect(output(tester), contains('"=SUM(Costs)" = "10"'));
  });
}
