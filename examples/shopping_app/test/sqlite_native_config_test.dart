// Copyright (c) 2026 Ye Lin Aung
// SPDX-License-Identifier: MIT

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Flutter proof app bundles SQLite instead of requiring a host library',
      () {
    // Android does not provide the desktop libsqlite3.so used by host tests.
    final manifest = File('pubspec.yaml').readAsStringSync();
    final hooks = manifest.indexOf('\nhooks:\n');
    expect(hooks, greaterThanOrEqualTo(0));
    expect(manifest.substring(hooks + 1), '''
hooks:
  user_defines:
    sqlite3:
      source: sqlite3
''');
  });
}
