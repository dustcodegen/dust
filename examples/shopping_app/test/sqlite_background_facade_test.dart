import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shopping_app/core/db/shopping_cache_database.dart';

void main() {
  test(
      'generated worker facade supports every non-DAO terminal and async unsafe',
      () async {
    final app = await ShoppingCacheDatabase.openBackground(
      ':memory:',
      options: const SqliteConnectOptions.memory(
        foreignKeys: true,
        busyTimeout: Duration(milliseconds: 123),
      ),
    );
    addTearDown(app.close);
    expect(app.connection, same(app.pool));
    expect(app.pool, isA<Sqlite3WorkerDriver>());
    expect(app.pool, isNot(isA<Sqlite3Executor>()));
    (await app.migrate()).unwrap();
    final pragmas =
        (await app.unsafe.fetch('PRAGMA busy_timeout', [])).unwrap();
    expect(pragmas.single.readIndex<int>(0), 123);
    final foreignKeys =
        (await app.unsafe.fetch('PRAGMA foreign_keys', [])).unwrap();
    expect(foreignKeys.single.readIndex<int>(0), 1);

    (await queryExecute(
      r'INSERT INTO wishlist_cache VALUES ($1, $2, $3)',
      [7, 'Background', '2026-10-15T03:30:00Z'],
    ).execute(app.connection))
        .unwrap();
    const sql = 'SELECT product_id, title, saved_at FROM wishlist_cache';
    final query = queryAs<CachedWishlistRow>(sql, []);
    final one = (await query.fetchOne(app.connection)).unwrap();
    expect(one.productId, 7);
    expect(one.savedAt, DateTime.utc(2026, 10, 15, 3, 30));
    expect((await query.fetchAll(app.connection)).unwrap(), hasLength(1));
    expect((await query.fetchOptional(app.connection)).unwrap()!.title,
        'Background');
    expect(
        (await queryAs<CachedWishlistRow>(
          '$sql WHERE product_id = 99',
          [],
        ).fetchOptional(app.connection))
            .unwrap(),
        isNull);
    expect(
        (await queryScalar<int>(
          'SELECT COUNT(*) FROM wishlist_cache',
          [],
        ).fetchOne(app.connection))
            .unwrap(),
        1);
    expect(
        (await app.unsafe.fetchAs(
          'SELECT title FROM wishlist_cache',
          [],
          (row) => row.read<String>('title'),
        ))
            .unwrap(),
        ['Background']);

    final rolledBack = await app.transaction<Unit>((tx) async {
      expect((await app.unsafe.fetch('SELECT 1', [])).unwrapErr().category,
          SqlxErrorCategory.transaction);
      (await tx.transaction<Unit>((child) async {
        (await ShoppingCacheDao(child)
                .saveWishlist(8, 'Nested', '2026-10-15T03:31:00Z'))
            .unwrap();
        return const Ok(unit);
      }))
          .unwrap();
      return Err(SqlxError.transaction('cancel'));
    });
    expect(rolledBack.isErr, isTrue);
    expect((await ShoppingCacheDao(app.connection).listWishlist()).unwrap(),
        hasLength(1));
    (await app.unsafe.execute('DELETE FROM wishlist_cache', [])).unwrap();
    expect((await query.fetchAll(app.connection)).unwrap(), isEmpty);
    final close = app.close();
    expect(app.close(), same(close));
    (await close).unwrap();
    expect((await app.unsafe.fetch('SELECT 1', [])).unwrapErr().category,
        SqlxErrorCategory.connection);
  });

  test('generated worker startup rejects invalid options and migration drift',
      () async {
    final directory =
        await Directory.systemTemp.createTemp('dust-facade-drift-');
    addTearDown(() => directory.delete(recursive: true));
    final path = '${directory.path}/app.db';
    await expectLater(
      ShoppingCacheDatabase.openBackground(path,
          options: const SqliteConnectOptions(
              busyTimeout: Duration(milliseconds: -1))),
      throwsA(isA<SqlxError>().having(
          (error) => error.category, 'category', SqlxErrorCategory.driver)),
    );
    final db = ShoppingCacheDatabase.open(path);
    try {
      (await ShoppingCacheDao(db.connection)
              .saveWishlist(7, 'Preserved', '2026-10-15T03:30:00Z'))
          .unwrap();
      (await db.unsafe.execute(
        "UPDATE __dust_schema_migrations SET checksum = ? WHERE name = '0001_shopping_cache.sql'",
        ['0000000000000000000000000000000000000000000000000000000000000000'],
      ))
          .unwrap();
    } finally {
      (await db.close()).unwrap();
    }
    await expectLater(
        ShoppingCacheDatabase.openBackground(path),
        throwsA(
          isA<SqlxError>()
              .having((error) => error.category, 'category',
                  SqlxErrorCategory.migration)
              .having((error) => error.operation, 'operation',
                  '0001_shopping_cache.sql')
              .having((error) => error.message, 'message',
                  contains('checksum mismatch')),
        ));
    final inspect =
        await Sqlite3WorkerDriver.connect(SqliteConnectOptions.readOnly(path));
    try {
      expect(
          (await inspect
                  .fetchScalar<String>('SELECT title FROM wishlist_cache', []))
              .unwrap(),
          'Preserved');
    } finally {
      (await inspect.close()).unwrap();
    }
  });

  test(
      'generated worker requires explicit reviewed baseline for legacy history',
      () async {
    final directory =
        await Directory.systemTemp.createTemp('dust-facade-legacy-');
    addTearDown(() => directory.delete(recursive: true));
    final path = '${directory.path}/app.db';
    final seed = Sqlite3Driver.open(path);
    try {
      seed.database.execute(
          File('migrations/0001_shopping_cache.sql').readAsStringSync());
      seed.database.execute('''
INSERT INTO wishlist_cache VALUES (7, 'Before upgrade', '2025-01-01T00:00:00Z');
CREATE TABLE __dust_schema_migrations (name TEXT PRIMARY KEY, applied_at TEXT NOT NULL);
INSERT INTO __dust_schema_migrations VALUES ('0001_shopping_cache.sql', '2025-01-01T00:00:00Z');
''');
    } finally {
      (await seed.close()).unwrap();
    }
    await expectLater(
        ShoppingCacheDatabase.openBackground(path),
        throwsA(
          isA<SqlxError>().having((error) => error.message, 'message',
              contains('baselineLegacyMigrations')),
        ));
    final app = await ShoppingCacheDatabase.openBackground(path,
        options: const SqliteConnectOptions(baselineLegacyMigrations: true));
    try {
      final rows =
          (await ShoppingCacheDao(app.connection).listWishlist()).unwrap();
      expect(rows.single.title, 'Before upgrade');
      final history = (await app.unsafe.fetchAs(
        'SELECT checksum_source FROM __dust_schema_migrations ORDER BY name',
        [],
        (row) => row.read<String>('checksum_source'),
      ))
          .unwrap();
      expect(history, ['legacy-baseline', 'applied']);
    } finally {
      (await app.close()).unwrap();
    }
    final reopened = await ShoppingCacheDatabase.openBackground(path);
    (await reopened.close()).unwrap();
  });
}
