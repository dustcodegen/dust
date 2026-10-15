import 'dart:io';

import 'package:test/test.dart';

import '../integration_test/support/native_process.dart';

void main() {
  test(
    'failed native child build keeps the compiler failure visible',
    () async {
      final output = Directory.systemTemp.createTempSync('dust-bad-child-');
      addTearDown(() => output.deleteSync(recursive: true));
      final sibling = File('${output.path}/keep-native-shim')
        ..writeAsStringSync('keep');
      await expectLater(
        buildNativeChild(output, 'test/support/does_not_exist.dart'),
        throwsA(
          isA<StateError>().having(
            (error) => error.message,
            'compiler diagnostic',
            contains('does_not_exist.dart'),
          ),
        ),
      );
      expect(sibling.readAsStringSync(), 'keep');
    },
  );
}
