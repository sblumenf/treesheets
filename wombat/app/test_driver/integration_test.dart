// The driver of `flutter drive` for the tests in `integration_test/`: runs on the
// computer, not on the device, and prints the report that a test handed over (see
// `integration_test/measure_test.dart`).
import 'dart:io';

import 'package:integration_test/integration_test_driver.dart';

Future<void> main() => integrationDriver(
  responseDataCallback: (data) async {
    final report = data?['report'];
    if (report != null) stdout.writeln(report);
  },
);
