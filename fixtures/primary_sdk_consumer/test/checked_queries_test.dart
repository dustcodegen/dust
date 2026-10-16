import 'package:dust_dart/db.dart';
import 'package:primary_sdk_consumer/checked_queries.dart';
import 'package:primary_sdk_consumer/database.dart';
import 'package:primary_sdk_consumer/private_models.dart';
import 'package:test/test.dart';

void main() {
  for (final background in [false, true]) {
    for (final dao in [false, true]) {
      test(
        'checked primary rows (background: $background, DAO: $dao)',
        () async {
          final db = background
              ? await PrimaryDatabase.openBackground(':memory:')
              : PrimaryDatabase.open(':memory:');
          addTearDown(() async => (await db.close()).unwrap());
          final connection = db.connection;
          expect(
            (await (dao
                    ? PrimaryAccountDao(connection).all()
                    : allAccounts(connection)))
                .unwrap(),
            isEmpty,
          );

          final saved = (await connection.transaction((tx) async {
            final inserted =
                (await (dao
                        ? PrimaryAccountDao(tx).insert('Ada', 'saved')
                        : insertAccount(tx, 'Ada', 'saved')))
                    .unwrap();
            expect(inserted.rowsAffected, 1);
            return dao
                ? PrimaryAccountDao(tx).one('Ada')
                : oneAccount(tx, 'Ada');
          })).unwrap();
          expect(saved, const PrivateAccount(name: 'Ada', note: 'saved'));
          expect(saved.copyWith(note: null).note, isNull);
          (await (dao
                  ? PrimaryAccountDao(connection).insert('Bob', null)
                  : insertAccount(connection, 'Bob', null)))
              .unwrap();
          expect(
            (await (dao
                    ? PrimaryAccountDao(connection).one('Bob')
                    : oneAccount(connection, 'Bob')))
                .unwrap(),
            const PrivateAccount(name: 'Bob'),
          );
          expect(
            (await (dao
                    ? PrimaryAccountDao(connection).optional('missing')
                    : optionalAccount(connection, 'missing')))
                .unwrap(),
            isNull,
          );
          expect(
            (await (dao
                    ? PrimaryAccountDao(connection).all()
                    : allAccounts(connection)))
                .unwrap(),
            [saved, const PrivateAccount(name: 'Bob')],
          );

          final failed = await connection.transaction((tx) async {
            (await (dao
                    ? PrimaryAccountDao(tx).insert('Grace', 'rollback')
                    : insertAccount(tx, 'Grace', 'rollback')))
                .unwrap();
            return dao
                ? PrimaryAccountDao(tx).insert('Ada', 'duplicate')
                : insertAccount(tx, 'Ada', 'duplicate');
          });
          expect(failed.isErr, isTrue);
          expect(
            (await (dao
                    ? PrimaryAccountDao(connection).count()
                    : countAccounts(connection)))
                .unwrap(),
            2,
          );
          expect(
            (await (dao
                    ? PrimaryAccountDao(connection).optional('Grace')
                    : optionalAccount(connection, 'Grace')))
                .unwrap(),
            isNull,
          );
        },
      );
    }
  }
}
