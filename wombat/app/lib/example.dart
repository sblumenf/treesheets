import 'dart:io';
import 'dart:typed_data';

import 'session.dart';

/// The bytes that `data` shows. A [ByteData] may be a window on a longer buffer, as an
/// asset from a bundle can be, so the buffer alone is not it.
Uint8List bytesOf(ByteData data) =>
    data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);

/// Opens a copy of the example that comes with the app, whose bytes are `example`, as
/// the file `to` in `folder`, where no file of that name may be yet.
///
/// Many installs have the same example file, so the core makes the copy, with IDs of
/// its own for the document and every cell (decision A28). The core copies from a
/// file, so the example is one for a moment, beside the copy.
Future<Session> openExampleCopy(
  Uint8List example, {
  required Directory folder,
  required String to,
}) async {
  final original = File('${folder.path}/example-original.wombat');
  await original.writeAsBytes(example);
  try {
    return await Session.openCopy(
      from: original.path,
      to: '${folder.path}/$to',
    );
  } finally {
    try {
      await original.delete();
    } on FileSystemException {
      // A file left over is written again by the next copy. Throwing here would put
      // this error in the place of an open document, which no one would then close.
    }
  }
}
