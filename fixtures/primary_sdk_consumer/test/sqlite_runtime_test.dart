import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:primary_sdk_consumer/private_models.dart';
import 'package:test/test.dart';

void main() {
  for (final background in [false, true]) {
    test(
      'primary row queries work with db and tx (background: $background)',
      () async {
        const migrations = <String, String>{
          '0001_accounts.sql':
              'CREATE TABLE accounts (display_name TEXT NOT NULL, note TEXT)',
        };
        final Connection db = background
            ? await Sqlite3WorkerDriver.open(':memory:', migrations: migrations)
            : Sqlite3Driver.open(':memory:', migrations: migrations);
        addTearDown(() async => (await db.close()).unwrap());

        final saved = (await db.transaction((tx) async {
          (await queryExecute(r'INSERT INTO accounts VALUES ($1, $2)', [
            'Ada',
            'saved',
          ]).execute(tx)).unwrap();
          return queryAs<PrivateAccount>(
            r'SELECT display_name, note FROM accounts WHERE display_name = $1',
            ['Ada'],
          ).fetchOne(tx);
        })).unwrap();
        expect(saved, const PrivateAccount(name: 'Ada', note: 'saved'));
        expect(saved.copyWith(note: null).note, isNull);

        expect(
          (await queryAs<PrivateAccount>(
            'SELECT display_name, note FROM accounts',
            [],
          ).fetchAll(db)).unwrap(),
          [saved],
        );
        expect(
          (await queryAs<PrivateAccount>(
            r'SELECT display_name, note FROM accounts WHERE display_name = $1',
            ['missing'],
          ).fetchOptional(db)).unwrap(),
          isNull,
        );

        final failed = await db.transaction((tx) async {
          (await queryExecute(
            r'INSERT INTO accounts VALUES ($1, $2)',
            ['Grace', null],
          ).execute(tx)).unwrap();
          return queryExecute(
            'INSERT INTO accounts VALUES (NULL, NULL)',
            [],
          ).execute(tx);
        });
        expect(failed.isErr, isTrue);
        expect(
          (await queryScalar<int>(
            'SELECT COUNT(*) FROM accounts',
            [],
          ).fetchOne(db)).unwrap(),
          1,
        );
      },
    );
  }
}
