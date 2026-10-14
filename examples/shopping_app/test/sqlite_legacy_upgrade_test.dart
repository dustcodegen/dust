import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shopping_app/core/db/shopping_cache_database.dart';

void main() {
  test('generated facade rejects legacy history until explicitly baselined',
      () async {
    final directory =
        Directory.systemTemp.createTempSync('dust-shopping-legacy-');
    addTearDown(() => directory.deleteSync(recursive: true));
    final path = '${directory.path}/app.db';
    final seed = Sqlite3Driver.open(path);
    try {
      seed.database.execute(
          File('migrations/0001_shopping_cache.sql').readAsStringSync());
      seed.database.execute('''
INSERT INTO wishlist_cache VALUES (7, 'Saved before upgrade', '2025-01-01T00:00:00.000Z');
CREATE TABLE __dust_schema_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL);
INSERT INTO __dust_schema_migrations VALUES ('0001_shopping_cache.sql', '2025-01-01T00:00:00.000Z');
''');
    } finally {
      (await seed.close()).unwrap();
    }
    expect(
      () => ShoppingCacheDatabase.open(path),
      throwsA(isA<SqlxError>()
          .having((e) => e.category, 'category', SqlxErrorCategory.migration)
          .having((e) => e.operation, 'operation', '0001_shopping_cache.sql')
          .having((e) => e.message, 'message',
              contains('explicitly enable baselineLegacyMigrations'))),
    );
    final unchanged = Sqlite3Driver.open(path);
    try {
      expect(
          unchanged.database
              .select('PRAGMA table_info(__dust_schema_migrations)')
              .map((row) => row['name']),
          ['name', 'applied_at']);
      expect(
          unchanged.database
              .select('PRAGMA table_info(product_cache)')
              .map((row) => row['name']),
          isNot(contains('last_synced_at')));
    } finally {
      (await unchanged.close()).unwrap();
    }

    final app = ShoppingCacheDatabase.open(
      path,
      options: const SqliteConnectOptions(
        baselineLegacyMigrations: true,
        busyTimeout: Duration(milliseconds: 500),
      ),
    );
    try {
      final wishlist = await app.connection.listWishlist();
      expect(wishlist.single.productId, 7);
      expect(wishlist.single.title, 'Saved before upgrade');
      final history = (await app.unsafe.fetch(
        'SELECT name, checksum_source FROM __dust_schema_migrations ORDER BY name',
        [],
      ))
          .unwrap();
      expect(history.map((row) => row.read<String>('checksum_source')),
          ['legacy-baseline', 'applied']);
      final timestamp = (await app.unsafe.fetch(
        "SELECT applied_at FROM __dust_schema_migrations WHERE name = '0001_shopping_cache.sql'",
        [],
      ))
          .unwrap();
      expect(timestamp.single.read<String>('applied_at'),
          '2025-01-01T00:00:00.000Z');
    } finally {
      (await app.close()).unwrap();
    }
    final reopened = ShoppingCacheDatabase.open(path);
    try {
      expect((await reopened.connection.listWishlist()).single.productId, 7);
      expect(
          (await reopened.unsafe
                  .fetch('SELECT name FROM __dust_schema_migrations', []))
              .unwrap(),
          hasLength(2));
    } finally {
      (await reopened.close()).unwrap();
    }
  });
}
