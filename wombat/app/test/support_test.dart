// What the tests stand on: where the bridge's library is taken from.
import 'package:flutter_test/flutter_test.dart';

import 'support.dart';

void main() {
  test('the library is where cargo says it built it', () {
    // As cargo prints it: other crates, the bridge with its three files, the end.
    const messages = '''
{"reason":"compiler-artifact","target":{"name":"wombat-core","kind":["lib"]},"filenames":["/x/t/debug/libwombat_core.rlib"]}
warning: something for people
{"reason":"build-script-executed","package_id":"wombat_bridge"}
{"reason":"compiler-artifact","target":{"name":"wombat_bridge","kind":["lib","cdylib","staticlib"]},"filenames":["/x/t/arm/debug/libwombat_bridge.rlib","/x/t/arm/debug/libwombat_bridge.so","/x/t/arm/debug/libwombat_bridge.a"]}
{"reason":"build-finished","success":true}
''';
    expect(bridgeLibrary(messages), '/x/t/arm/debug/libwombat_bridge.so');
    expect(
      bridgeLibrary(
        '{"reason":"compiler-artifact","target":{"name":"wombat_bridge"},'
        r'"filenames":["C:\\t\\wombat_bridge.dll.lib","C:\\t\\wombat_bridge.dll"]}',
      ),
      r'C:\t\wombat_bridge.dll',
    );
    expect(
      bridgeLibrary(
        '{"reason":"compiler-artifact","target":{"name":"wombat_bridge"},'
        '"filenames":["/t/libwombat_bridge.dylib"]}',
      ),
      '/t/libwombat_bridge.dylib',
    );
    // Nothing built, or only something else: said, not guessed.
    expect(() => bridgeLibrary(''), throwsStateError);
    expect(
      () => bridgeLibrary(
        '{"reason":"compiler-artifact","target":{"name":"other"},'
        '"filenames":["/t/libother.so"]}',
      ),
      throwsStateError,
    );
  });
}
