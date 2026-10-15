import 'package:dust_dart/db.dart';
import 'package:test/test.dart';

import 'support/disk_full_fixture.dart';
import 'support/expect_ok.dart';

void main() {
  for (final journal in ['delete', 'wal']) {
    for (final background in [false, true]) {
      test(
        '$journal / worker=$background / migration FULL is atomic',
        () async {
          final fixture = DiskFullFixture(journal, background: background);
          await fixture.seed();
          expectOk(await fixture.db.close());
          const migrations = {
            ...fullMigrations,
            '0002.sql': '''
CREATE TABLE upgraded (id INTEGER PRIMARY KEY);
INSERT INTO upgraded VALUES (7);
UPDATE items SET name = 'pending' WHERE id = 1;
''',
            '0003.sql': '$fullInsert;',
          };
          try {
            final unexpected = await fixture.open(
              migrations: migrations,
              pageLimit: fixture.limit,
            );
            expectOk(await unexpected.close());
            fail('expected managed startup to fail with SQLITE_FULL');
          } on SqlxError catch (error) {
            expectFull(
              error,
              '0003.sql',
              background: background,
              category: SqlxErrorCategory.migration,
            );
          }
          fixture.db = await fixture.open();
          await fixture.expectBaseline(fixture.db);
          expect(
            expectOk(
              await fixture.db.fetchScalar<int>(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'upgraded'",
                [],
              ),
            ),
            0,
          );
          expectOk(await fixture.db.close());
          fixture.db = await fixture.open(
            migrations: migrations,
            pageLimit: fixture.originalLimit,
          );
          expect(
            expectOk(
              await fixture.db.fetchScalar<int>('SELECT id FROM upgraded', []),
            ),
            7,
          );
          expect(
            expectOk(
              await fixture.db.fetchScalar<int>(
                'SELECT length(payload) FROM items WHERE id = 3',
                [],
              ),
            ),
            262144,
          );
          expect((await readHistory(fixture.db)).first, fixture.history.single);
          expect(await readHistory(fixture.db), hasLength(3));
          expectOk(await fixture.db.close());
          fixture.db = await fixture.open(migrations: migrations);
          expect(
            expectOk(
              await fixture.db.fetchScalar<String>(
                'PRAGMA integrity_check',
                [],
              ),
            ),
            'ok',
          );
        },
      );
    }
  }
}
