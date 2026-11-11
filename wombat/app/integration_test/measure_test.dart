// Measures the bridge with a document of 101,001 cells on a real build of the app, and
// prints the report of `lib/measure.dart`. The numbers that count come from a build with
// the Rust library optimised, which a debug build is not. From `wombat/app`:
//
//   flutter drive --profile --driver=test_driver/integration_test.dart \
//     --target=integration_test/measure_test.dart -d <device>
//
// (on Linux without a screen: xvfb-run -a flutter drive ... -d linux). The first run on
// a device also builds the document; run it twice and take the second, whose process
// only opened the file. `flutter test integration_test/measure_test.dart -d <device>`
// runs the same in debug mode.
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:wombat_app/measure.dart';
import 'package:wombat_app/src/rust/frb_generated.dart';

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());

  testWidgets('measure a document of 101,001 cells', (tester) async {
    // A folder the app may write to on every platform, the same one at every run.
    final folder = Directory('${Directory.systemTemp.path}/wombat-measure');
    final report = await tester.runAsync(() async {
      await folder.create(recursive: true);
      return (await measure(folder)).report();
    });
    // Printed for `flutter test`, and handed to the driver of `flutter drive`, which
    // does not show what a profile or release build prints.
    // ignore: avoid_print
    print(report);
    binding.reportData = {'report': report};
    expect(report, contains('document: 101001 cells'));
  }, timeout: const Timeout(Duration(minutes: 30)));
}
