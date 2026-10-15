import 'package:dust_dart/db.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/wal_fixture.dart';

void main() {
  for (final firstWorker in [false, true]) {
    for (final secondWorker in [false, true]) {
      group(
        'WAL / first worker=$firstWorker / second worker=$secondWorker',
        () {
          late WalFixture fixture;
          late Pool first;
          late Pool second;
          setUp(() async {
            fixture = WalFixture();
            await fixture.seed();
            first = await fixture.open(background: firstWorker);
            second = await fixture.open(background: secondWorker);
          });

          test(
            'records actual native engine provenance on both connections',
            () async {
              Future<(String, String)> identity(Executor db) async => (
                expectOk(
                  await db.fetchScalar<String>('SELECT sqlite_version()', []),
                ),
                expectOk(
                  await db.fetchScalar<String>('SELECT sqlite_source_id()', []),
                ),
              );
              final native = await identity(first);
              expect(await identity(second), native);
              expect(native.$1, matches(r'^\d+\.\d+\.\d+$'));
              expect(native.$2, isNotEmpty);
              print('native SQLite: ${native.$1} / ${native.$2}');
            },
          );

          for (final timeout in [
            Duration.zero,
            const Duration(milliseconds: 20),
          ]) {
            test(
              'competing writer fails with bounded busy timeout $timeout',
              () async {
                expectOk(await second.close());
                second = await fixture.open(
                  background: secondWorker,
                  timeout: timeout,
                );
                expectOk(
                  await first.transaction<Unit>((tx) async {
                    await writeValue(tx, 2);
                    final failed = await second.transaction<Unit>(
                      (other) async => (await queryExecute(
                        'UPDATE items SET value = 3',
                        [],
                      ).execute(other)).map((_) => unit),
                    );
                    expectBusy(failed.unwrapErr(), background: secondWorker);
                    expect(
                      failed.unwrapErr().operation,
                      'UPDATE items SET value = 3',
                    );
                    expect(await readValue(tx), 2);
                    await fixture.expectHealthy(second, 1);
                    return const Ok(unit);
                  }),
                );
                await fixture.expectHealthy(second, 2);
                expectOk(
                  await second.transaction<Unit>((tx) async {
                    await writeValue(tx, 4);
                    return const Ok(unit);
                  }),
                );
                await fixture.expectHealthy(first, 4);
              },
            );
          }

          test('stale snapshot cannot write; fresh transaction can', () async {
            final failed = await first.transaction<Unit>((tx) async {
              expect(await readValue(tx), 1);
              await writeValue(second, 2);
              expect(await readValue(tx), 1);
              return (await queryExecute(
                'UPDATE items SET value = 3',
                [],
              ).execute(tx)).map((_) => unit);
            });
            expectBusy(
              failed.unwrapErr(),
              background: firstWorker,
              extended: 517,
            );
            expect(failed.unwrapErr().operation, 'UPDATE items SET value = 3');
            await fixture.expectHealthy(first, 2);
            expectOk(
              await first.transaction<Unit>((tx) async {
                await writeValue(tx, 4);
                return const Ok(unit);
              }),
            );
            await fixture.expectHealthy(second, 4);
          });
        },
      );
    }
  }
}
