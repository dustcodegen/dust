import 'package:dust_dart/db.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shopping_app/core/db/product_cache_queries.dart';
import 'package:shopping_app/core/db/shopping_cache_database.dart';

void main() {
  for (final background in [false, true]) {
    for (final dao in [false, true]) {
      group('${background ? 'worker' : 'sync'} / ${dao ? 'DAO' : 'inline'}',
          () {
        late ShoppingCacheDatabase app;
        setUp(() async {
          app = background
              ? await ShoppingCacheDatabase.openBackground(':memory:')
              : ShoppingCacheDatabase.open(':memory:');
        });
        tearDown(() async => (await app.close()).unwrap());

        Future<Result<CachedProductRow?, SqlxError>> find(int id) => dao
            ? ShoppingCacheDao(app.connection).findCachedProduct(id)
            : findProductWithoutDao(app.connection, id);
        Future<Result<List<CachedProductRow>, SqlxError>> products() => dao
            ? ShoppingCacheDao(app.connection).listCachedProducts()
            : productsWithoutDao(app.connection);
        Future<Result<List<CachedWishlistRow>, SqlxError>> wishlist() => dao
            ? ShoppingCacheDao(app.connection).listWishlist()
            : wishlistWithoutDao(app.connection);

        test('generated mapping keeps every supported product value', () async {
          expect((await find(7)).unwrap(), isNull);
          expect((await products()).unwrap(), isEmpty);
          (await ShoppingCacheDao(app.connection).saveProductRow(
            7,
            'Shirt 🍜',
            42.5,
            'Soft cotton',
            'clothing',
            'https://example.test/shirt.png',
            4.8,
            128,
            '{"tags":["cotton","🍜"],"syncedBy":"parity"}',
            'fake_store',
          ))
              .unwrap();
          final one = (await find(7)).unwrap()!;
          final all = (await products()).unwrap();
          expect(all, hasLength(1));
          for (final row in [one, all.single]) {
            expect([
              row.id,
              row.title,
              row.price,
              row.description,
              row.category,
              row.imageUrl,
              row.rating.rate,
              row.rating.count,
              row.pinned,
              row.payload.tags,
              row.payload.syncedBy,
              row.source,
            ], [
              7,
              'Shirt 🍜',
              42.5,
              'Soft cotton',
              'clothing',
              'https://example.test/shirt.png',
              4.8,
              128,
              false,
              ['cotton', '🍜'],
              'parity',
              CacheSource.fakeStore,
            ]);
          }
        });

        test('date decoding rejects corrupt rows and remains reusable',
            () async {
          (await ShoppingCacheDao(app.connection)
                  .saveWishlist(7, 'Shirt', '2026-10-15T03:30:00Z'))
              .unwrap();
          final rows = (await wishlist()).unwrap();
          expect(rows.single.productId, 7);
          expect(rows.single.title, 'Shirt');
          expect(rows.single.savedAt, DateTime.utc(2026, 10, 15, 3, 30));
          (await app.unsafe.execute(
            'UPDATE wishlist_cache SET saved_at = ?',
            ['not-a-date'],
          ))
              .unwrap();
          expect((await wishlist()).unwrapErr().category,
              SqlxErrorCategory.decode);
          (await ShoppingCacheDao(app.connection)
                  .saveWishlist(7, 'Repaired', '2026-10-16T00:00:00Z'))
              .unwrap();
          expect((await wishlist()).unwrap().single.savedAt,
              DateTime.utc(2026, 10, 16));
        });

        test('JSON and converter errors remain decode failures', () async {
          for (final values in [
            ('invalid-json', 'fake_store'),
            ('{"tags":[],"syncedBy":"parity"}', 'unknown-source'),
          ]) {
            (await ShoppingCacheDao(app.connection).saveProductRow(
              7,
              'Shirt',
              1,
              '',
              '',
              '',
              0,
              0,
              values.$1,
              values.$2,
            ))
                .unwrap();
            expect(
                (await find(7)).unwrapErr().category, SqlxErrorCategory.decode);
            expect((await products()).unwrapErr().category,
                SqlxErrorCategory.decode);
          }
          (await ShoppingCacheDao(app.connection).saveProductRow(
            7,
            'Repaired',
            1,
            '',
            '',
            '',
            0,
            0,
            '{"tags":[],"syncedBy":"parity"}',
            'local',
          ))
              .unwrap();
          expect((await find(7)).unwrap()!.source, CacheSource.local);
        });
      });
    }
  }
}
