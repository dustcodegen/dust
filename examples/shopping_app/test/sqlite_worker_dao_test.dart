import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shopping_app/core/db/shopping_cache_database.dart';

void main() {
  test('generated shopping DAOs use worker queries, mapping and transactions',
      () async {
    final directory = await Directory.systemTemp.createTemp('dust-worker-dao-');
    addTearDown(() => directory.delete(recursive: true));
    final path = '${directory.path}/app.db';
    final migrations = {
      for (final file in Directory('migrations').listSync().whereType<File>())
        file.uri.pathSegments.last: file.readAsStringSync(),
    };
    final db = await Sqlite3WorkerDriver.open(path, migrations: migrations);
    try {
      final result = await db.transaction<int>((tx) async {
        final dao = ShoppingCacheDao(tx);
        (await dao.saveProductRow(
                7,
                'Dust Runner',
                42.5,
                'Worker cache',
                'shoes',
                'https://example.test/runner.png',
                4.8,
                128,
                '{"tags":["worker"],"syncedBy":"dust"}',
                'fake_store'))
            .unwrap();
        (await dao.saveWishlist(7, 'Dust Runner', '2026-10-15T03:30:00.000Z'))
            .unwrap();
        return dao.countCachedProducts();
      });
      expect(result.unwrap(), 1);
      final dao = ShoppingCacheDao(db);
      final product = (await dao.findCachedProduct(7)).unwrap()!;
      expect(product.price, 42.5);
      expect(product.rating.rate, 4.8);
      expect(product.rating.count, 128);
      expect(product.pinned, isFalse);
      expect(product.payload.tags, ['worker']);
      expect(product.payload.syncedBy, 'dust');
      expect(product.source, CacheSource.fakeStore);
      expect((await dao.findCachedProduct(99)).unwrap(), isNull);
      expect((await dao.listCachedProducts()).unwrap().single.title,
          'Dust Runner');
      final wishlist = (await dao.listWishlist()).unwrap();
      expect(wishlist.single.productId, 7);
      expect(wishlist.single.savedAt, DateTime.utc(2026, 10, 15, 3, 30));

      final rollback = await db.transaction<Unit>((tx) async {
        (await ShoppingCacheDao(tx)
                .saveWishlist(99, 'Roll back', '2026-10-15T03:31:00.000Z'))
            .unwrap();
        return Err(SqlxError.transaction('cancel save'));
      });
      expect(rollback.isErr, isTrue);
      expect((await dao.listWishlist()).unwrap(), hasLength(1));
    } finally {
      (await db.close()).unwrap();
    }
    final reopened =
        await Sqlite3WorkerDriver.open(path, migrations: migrations);
    try {
      expect(
          (await ShoppingCacheDao(reopened).countCachedProducts()).unwrap(), 1);
      expect((await ShoppingCacheDao(reopened).listWishlist()).unwrap(),
          hasLength(1));
    } finally {
      (await reopened.close()).unwrap();
    }
  });
}
