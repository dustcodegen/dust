import 'dart:convert';
import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
import 'package:test/test.dart';

import '../integration_test/support/native_process.dart';
import 'support/crash_process.dart';
import 'support/crash_schema.dart';
import 'support/expect_ok.dart';

void main() {
  late Directory binaries;
  late String executable;
  setUpAll(() async {
    binaries = Directory.systemTemp.createTempSync('dust-crash-child-');
    addTearDown(() => binaries.deleteSync(recursive: true));
    executable = await buildNativeChild(
      binaries,
      'test/support/crash_child.dart',
    );
  });

  for (final journal in ['delete', 'wal']) {
    for (final background in [false, true]) {
      for (final stage in [
        'outer',
        'nested',
        'committed',
        'migration-body',
        'migration-commit',
        'migration-committed',
      ]) {
        test('$journal / worker=$background / kill at $stage', () async {
          final directory = Directory.systemTemp.createTempSync('dust-crash-');
          addTearDown(() => directory.deleteSync(recursive: true));
          final path = '${directory.path}/app.db';
          final options = crashOptions(journal);
          Future<Pool> open({Map<String, String>? migrations}) async =>
              background
              ? await Sqlite3WorkerDriver.open(
                  path,
                  options: options,
                  migrations: migrations,
                )
              : Sqlite3Driver.open(
                  path,
                  options: options,
                  migrations: migrations,
                );
          final first = {'0001.sql': crashMigrations['0001.sql']!};
          final seed = await open(migrations: first);
          try {
            expectOk(
              await queryExecute(
                "INSERT INTO items VALUES (1, 'baseline 🍜', NULL)",
                [],
              ).execute(seed),
            );
          } finally {
            expectOk(await seed.close());
          }

          await crashProcess(
            executable,
            path,
            journal,
            stage,
            background: background,
          );
          final committed = stage.endsWith('committed');
          final upgraded = stage == 'migration-committed';
          final db = await open();
          try {
            expect(
              expectOk(
                await queryScalar<String>(
                  'PRAGMA journal_mode',
                  [],
                ).fetchOne(db),
              ),
              journal,
            );
            expect(
              expectOk(
                await queryScalar<int>(
                  'PRAGMA synchronous',
                  [],
                ).fetchOne(db),
              ),
              2,
            );
            expect(
              expectOk(
                await queryScalar<String>(
                  'PRAGMA integrity_check',
                  [],
                ).fetchOne(db),
              ),
              'ok',
            );
            expect(
              expectOk(
                await db.fetchAll(
                  'SELECT id, name, length(payload) AS size FROM items ORDER BY id',
                  [],
                  (row) => [
                    row.read<int>('id'),
                    row.read<String>('name'),
                    row.readNullable<int>('size'),
                  ],
                ),
              ),
              [
                [1, committed ? 'changed' : 'baseline 🍜', null],
                if (committed) [upgraded ? 4 : 2, 'pending', 262144],
              ],
            );
            expect(
              expectOk(
                await queryScalar<int>(
                  "SELECT COUNT(*) FROM sqlite_master WHERE name = 'upgraded'",
                  [],
                ).fetchOne(db),
              ),
              upgraded ? 1 : 0,
            );
            expect(
              expectOk(
                await db.fetchAll(
                  'SELECT name, checksum, checksum_source '
                  'FROM __dust_schema_migrations ORDER BY name',
                  [],
                  (row) => [
                    row.read<String>('name'),
                    row.read<String>('checksum'),
                    row.read<String>('checksum_source'),
                  ],
                ),
              ),
              [
                for (final entry
                    in (upgraded ? crashMigrations : first).entries)
                  [
                    entry.key,
                    sha256.convert(utf8.encode(entry.value)).toString(),
                    'applied',
                  ],
              ],
            );
            expectOk(
              await db.transaction<Unit>((tx) async {
                return (await queryExecute(
                  "INSERT INTO items VALUES (9, 'reused', NULL)",
                  [],
                ).execute(tx)).map((_) => unit);
              }),
            );
          } finally {
            expectOk(await db.close());
          }

          // Normal managed startup resumes the exact bundle, never a replayed write.
          final resumed = await open(migrations: crashMigrations);
          try {
            expect(
              expectOk(
                await queryScalar<int>(
                  'SELECT COUNT(*) FROM __dust_schema_migrations',
                  [],
                ).fetchOne(resumed),
              ),
              2,
            );
            expect(
              expectOk(
                await queryScalar<int>(
                  'SELECT id FROM upgraded',
                  [],
                ).fetchOne(resumed),
              ),
              7,
            );
            expect(
              expectOk(
                await queryScalar<String>(
                  'SELECT name FROM items WHERE id = 9',
                  [],
                ).fetchOne(resumed),
              ),
              'reused',
            );
          } finally {
            expectOk(await resumed.close());
          }
          final repeated = await open(migrations: crashMigrations);
          try {
            expect(
              expectOk(
                await queryScalar<String>(
                  'PRAGMA integrity_check',
                  [],
                ).fetchOne(repeated),
              ),
              'ok',
            );
          } finally {
            expectOk(await repeated.close());
          }
        });
      }
    }
  }
}
