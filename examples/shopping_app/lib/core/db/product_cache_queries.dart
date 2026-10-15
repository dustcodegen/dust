import 'package:dust_dart/db.dart';

import 'shopping_cache_rows.dart';

/// Finds a product using the shopping DAO's generated row decoder, without a DAO.
///
/// Dust/Rust SQLx checks the literal SQL in DB mode. JSON, converters, flattening
/// and dates belong to the row mapping, not to the DAO declaration.
Future<Result<CachedProductRow?, SqlxError>> findProductWithoutDao(
        Executor db, int id) =>
    queryAs<CachedProductRow>(
      r'''
SELECT id, title, price, description, category, image,
       rating_rate, rating_count, payload, source
FROM product_cache
WHERE id = $1
''',
      [id],
    ).fetchOptional(db);

/// Reads every product with the same ordering as the DAO.
Future<Result<List<CachedProductRow>, SqlxError>> productsWithoutDao(
        Executor db) =>
    queryAs<CachedProductRow>(
      '''
SELECT id, title, price, description, category, image,
       rating_rate, rating_count, payload, source
FROM product_cache
ORDER BY title
''',
      [],
    ).fetchAll(db);

/// Reads dates through the generated wishlist row decoder.
Future<Result<List<CachedWishlistRow>, SqlxError>> wishlistWithoutDao(
        Executor db) =>
    queryAs<CachedWishlistRow>(
      '''
SELECT product_id, title, saved_at
FROM wishlist_cache
ORDER BY saved_at DESC
''',
      [],
    ).fetchAll(db);
