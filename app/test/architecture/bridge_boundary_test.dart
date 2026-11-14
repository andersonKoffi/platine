import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// Generated bridge types must not leak into presentation, application or
/// domain code: only `data/` adapters and the bootstrap may import them.
void main() {
  test('generated bridge code is only imported by data adapters', () {
    final offenders = <String>[];
    final libDir = Directory('lib');
    final dartFiles = libDir
        .listSync(recursive: true)
        .whereType<File>()
        .where((file) => file.path.endsWith('.dart'));

    for (final file in dartFiles) {
      final path = file.path.replaceAll(r'\', '/');
      final isGenerated = path.startsWith('lib/src/rust/');
      final isAllowed =
          path.contains('/data/') ||
          path == 'lib/core/bridge/rust_bootstrap.dart';
      if (isGenerated || isAllowed) continue;

      final importsBridge = file.readAsLinesSync().any(
        (line) => line.startsWith('import') && line.contains('/src/rust/'),
      );
      if (importsBridge) offenders.add(path);
    }

    expect(offenders, isEmpty, reason: 'Use a repository in data/ instead.');
  });
}
