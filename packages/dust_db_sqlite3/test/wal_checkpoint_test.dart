import 'dart:io';

import 'package:dust_dart/db.dart';
import 'package:test/test.dart';

import 'support/expect_ok.dart';
import 'support/wal_fixture.dart';

void main() {
  for (final readerWorker in [false, true]) {
    for (final writerWorker in [false, true]) {
      for (final mode in ['PASSIVE', 'FULL', 'RESTART', 'TRUNCATE']) {
        group(
          'WAL / reader worker=$readerWorker / writer worker=$writerWorker / $mode',
          () {
            late WalFixture fixture;
            late Pool reader;
            late Pool writer;
            setUp(() async {
              fixture = WalFixture();
              await fixture.seed();
              reader = await fixture.open(background: readerWorker);
              writer = await fixture.open(background: writerWorker);
              await writeValue(writer, 2);
            });

            test(
              'held reader limits progress; release permits checkpoint and reopen',
              () async {
                expectOk(
                  await reader.transaction<Unit>((tx) async {
                    expect(await readValue(tx), 2);
                    await writeValue(writer, 3);
                    await writeValue(writer, 4);
                    final (busy, frames, copied) = await checkpoint(
                      writer,
                      mode,
                    );
                    expect(busy, mode == 'PASSIVE' ? 0 : 1);
                    expect(copied, greaterThan(0));
                    expect(frames, greaterThan(copied));
                    expect(
                      File('${fixture.path}-wal').lengthSync(),
                      greaterThan(0),
                    );
                    expect(await readValue(tx), 2);
                    expect(await readValue(writer), 4);
                    return const Ok(unit);
                  }),
                );
                final (busy, frames, copied) = await checkpoint(writer, mode);
                expect(busy, 0);
                expect(copied, frames);
                if (mode == 'TRUNCATE') {
                  expect(frames, 0);
                  expect(File('${fixture.path}-wal').lengthSync(), 0);
                } else {
                  expect(frames, greaterThan(0));
                }
                await writeValue(writer, 5);
                expect(await checkpoint(writer, 'TRUNCATE'), (0, 0, 0));
                await fixture.expectHealthy(reader, 5);
                expectOk(await reader.close());
                expectOk(await writer.close());
                await fixture.expectHealthy(
                  await fixture.open(background: readerWorker),
                  5,
                );
              },
            );

            test(
              'active writer blocks stronger checkpoint without losing data',
              () async {
                expectOk(
                  await writer.transaction<Unit>((tx) async {
                    await writeValue(tx, 3);
                    final (busy, frames, copied) = await checkpoint(
                      reader,
                      mode,
                    );
                    expect(busy, mode == 'PASSIVE' ? 0 : 1);
                    expect(frames, greaterThan(0));
                    expect(copied, inInclusiveRange(0, frames));
                    expect(await readValue(reader), 2);
                    expect(await readValue(tx), 3);
                    return const Ok(unit);
                  }),
                );
                expect(await checkpoint(reader, 'TRUNCATE'), (0, 0, 0));
                await fixture.expectHealthy(reader, 3);
                await writeValue(writer, 4);
                await fixture.expectHealthy(reader, 4);
              },
            );

            if (mode == 'RESTART' || mode == 'TRUNCATE') {
              test(
                'fully copied WAL still cannot reset under a current reader',
                () async {
                  expectOk(
                    await reader.transaction<Unit>((tx) async {
                      expect(await readValue(tx), 2);
                      final full = await checkpoint(writer, 'FULL');
                      expect(full.$1, 0);
                      expect(full.$2, greaterThan(0));
                      expect(full.$3, full.$2);
                      final blocked = await checkpoint(writer, mode);
                      expect(blocked, (1, full.$2, full.$2));
                      expect(
                        File('${fixture.path}-wal').lengthSync(),
                        greaterThan(0),
                      );
                      expect(await readValue(tx), 2);
                      return const Ok(unit);
                    }),
                  );
                  final completed = await checkpoint(writer, mode);
                  expect(completed.$1, 0);
                  expect(completed.$3, completed.$2);
                  if (mode == 'TRUNCATE') {
                    expect(completed, (0, 0, 0));
                    expect(File('${fixture.path}-wal').lengthSync(), 0);
                  }
                  await writeValue(writer, 3);
                  await fixture.expectHealthy(reader, 3);
                },
              );
            }
          },
        );
      }
    }
  }
}
