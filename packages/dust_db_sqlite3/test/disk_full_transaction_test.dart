import 'package:dust_dart/db.dart';
import 'package:test/test.dart';

import 'support/disk_full_fixture.dart';
import 'support/expect_ok.dart';

void main() {
  for (final journal in ['delete', 'wal']) {
    for (final background in [false, true]) {
      group('$journal / worker=$background', () {
        late DiskFullFixture fixture;
        setUp(() async {
          fixture = DiskFullFixture(journal, background: background);
          await fixture.seed();
        });

        test(
          'standalone FULL preserves data and supports explicit retry',
          () async {
            expectFull(
              (await queryExecute(
                fullInsert,
                [],
              ).execute(fixture.db)).unwrapErr(),
              fullInsert,
              background: background,
            );
            await fixture.expectBaseline(fixture.db);
            await fixture.retry();
          },
        );

        for (final nested in [false, true]) {
          for (final outcome in ['return error', 'ignore error', 'throw']) {
            test('FULL nested=$nested / $outcome', () async {
              late Transaction escaped;
              final result = await fixture.db.transaction<Unit>((tx) async {
                expectOk(
                  await tx.execute(
                    "INSERT INTO items VALUES (2, 'prefix', NULL)",
                    [],
                  ),
                );
                Future<Result<Unit, SqlxError>> fail(
                  Transaction current,
                ) async {
                  escaped = current;
                  expectOk(
                    await current.execute(
                      "UPDATE items SET name = 'pending' WHERE id = 1",
                      [],
                    ),
                  );
                  final failed = await queryExecute(
                    fullInsert,
                    [],
                  ).execute(current);
                  expectFull(
                    failed.unwrapErr(),
                    fullInsert,
                    background: background,
                  );
                  if (outcome == 'throw') {
                    throw StateError('callback failed after FULL');
                  }
                  if (outcome == 'ignore error') {
                    expectFull(
                      (await current.execute(
                        "INSERT INTO items VALUES (5, 'must not commit', NULL)",
                        [],
                      )).unwrapErr(),
                      fullInsert,
                      background: background,
                    );
                    expectFull(
                      (await current.fetchScalar<int>(
                        'SELECT COUNT(*) FROM items',
                        [],
                      )).unwrapErr(),
                      fullInsert,
                      background: background,
                    );
                    var invoked = false;
                    expectFull(
                      (await current.transaction<Unit>((_) async {
                        invoked = true;
                        return const Ok(unit);
                      })).unwrapErr(),
                      fullInsert,
                      background: background,
                    );
                    expect(invoked, isFalse);
                    return const Ok(unit);
                  }
                  return failed.map((_) => unit);
                }

                if (!nested) return fail(tx);
                final child = await tx.transaction<Unit>(fail);
                expectFull(
                  child.unwrapErr(),
                  fullInsert,
                  background: background,
                );
                if (outcome == 'ignore error') {
                  expectFull(
                    (await tx.execute(
                      "INSERT INTO items VALUES (6, 'parent must not commit', NULL)",
                      [],
                    )).unwrapErr(),
                    fullInsert,
                    background: background,
                  );
                  return const Ok(unit);
                }
                return child;
              });

              expectFull(
                result.unwrapErr(),
                fullInsert,
                background: background,
              );
              expect((await escaped.execute('SELECT 1', [])).isErr, isTrue);
              await fixture.expectBaseline(fixture.db);
              await fixture.retry();
            });
          }
        }

        test('caught FULL cannot autocommit a following write', () async {
          late Result<ExecResult, SqlxError> following;
          final result = await fixture.db.transaction<Unit>((tx) async {
            expectOk(
              await tx.execute(
                "UPDATE items SET name = 'pending' WHERE id = 1",
                [],
              ),
            );
            final failed = await queryExecute(fullInsert, []).execute(tx);
            following = await tx.execute(
              "INSERT INTO items VALUES (5, 'must not commit', NULL)",
              [],
            );
            return failed.map((_) => unit);
          });
          expect(following.isErr, isTrue);
          expectFull(result.unwrapErr(), fullInsert, background: background);
          await fixture.expectBaseline(fixture.db);
          await fixture.retry();
        });

        test('RETURNING path retains FULL and permits reuse', () async {
          final result = await fixture.db.transaction<Unit>((tx) async {
            expectOk(
              await tx.execute(
                "UPDATE items SET name = 'pending' WHERE id = 1",
                [],
              ),
            );
            return (await queryScalar<int>(
              fullReturning,
              [],
            ).fetchOne(tx)).map((_) => unit);
          });
          expectFull(result.unwrapErr(), fullReturning, background: background);
          await fixture.expectBaseline(fixture.db);
          await fixture.retry();
        });
      });
    }
  }
}
