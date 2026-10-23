import 'package:dust_server/server.dart';
import 'package:test/test.dart';

import '../support.dart';

void main() {
  group('QueryExtractable', () {
    test('reads and coerces a value', () async {
      final outcome = await const QueryExtractable<bool>('done')
          .extract(request('GET', '/todos?done=true'));

      expect(expectOk(outcome), isTrue);
    });

    test('treats a bare flag as true', () async {
      final outcome = await const QueryExtractable<bool>('done')
          .extract(request('GET', '/todos?done'));

      expect(expectOk(outcome), isTrue);
    });

    test('returns null for an absent optional value', () async {
      final outcome = await const QueryExtractable<bool?>('done')
          .extract(request('GET', '/todos'));

      expect(expectOk(outcome), isNull);
    });

    test('rejects an absent required value with 400', () async {
      final outcome = await const QueryExtractable<int>('page')
          .extract(request('GET', '/todos'));

      expect(expectStatus(outcome, 400).message, 'query "page" is required');
    });

    test('rejects an uncoercible value with 400', () async {
      final outcome = await const QueryExtractable<int>('page')
          .extract(request('GET', '/todos?page=many'));

      expectStatus(outcome, 400);
    });
  });

  group('QueryListExtractable', () {
    test('reads every value for a repeated key', () async {
      final outcome = await const QueryListExtractable<String>('tag')
          .extract(request('GET', '/todos?tag=a&tag=b'));

      expect(expectOk(outcome), ['a', 'b']);
    });

    test('coerces each value', () async {
      final outcome = await const QueryListExtractable<int>('id')
          .extract(request('GET', '/todos?id=1&id=2'));

      expect(expectOk(outcome), [1, 2]);
    });

    test('returns an empty list for an absent key', () async {
      final outcome = await const QueryListExtractable<String>('tag')
          .extract(request('GET', '/todos'));

      expect(expectOk(outcome), isEmpty);
    });

    test('rejects the whole list when one value fails to coerce', () async {
      final outcome = await const QueryListExtractable<int>('id')
          .extract(request('GET', '/todos?id=1&id=x'));

      expectStatus(outcome, 400);
    });
  });

  group('QueryParameters', () {
    test('keeps the last scalar and all repeated values', () {
      const query = QueryParameters({
        'page': ['1'],
        'tag': ['a', 'b'],
        'empty': [],
      });
      final values = query.toObjectMap();

      query.coerceOne<int>(values, 'page');
      query.coerceOne<int>(values, 'missing');
      query.copyAll(values, 'tag');
      query.copyAll(values, 'missing');

      expect(values, {
        'page': 1,
        'tag': ['a', 'b'],
      });
    });

    test('coerces every repeated value', () {
      const query = QueryParameters({
        'id': ['1', '2'],
      });
      final values = query.toObjectMap();

      query.coerceAll<int>(values, 'id');
      query.coerceAll<int>(values, 'missing');

      expect(values['id'], [1, 2]);
    });

    test('uses the primary key without coercing a lower-priority alias', () {
      const query = QueryParameters({
        'done': ['true'],
        'completed': ['not-a-boolean'],
      });
      final values = query.toObjectMap();

      query.coerceOne<bool>(
        values,
        'done',
        aliases: const ['completed'],
      );

      expect(values, {
        'done': true,
        'completed': 'not-a-boolean',
      });
    });

    test('uses the first present alias for repeated values', () {
      const query = QueryParameters({
        'legacy_ids': ['1', '2'],
        'older_ids': ['not-an-integer'],
      });
      final values = query.toObjectMap();

      query.coerceAll<int>(
        values,
        'ids',
        aliases: const ['legacy_ids', 'older_ids'],
      );

      expect(values, {
        'legacy_ids': [1, 2],
        'older_ids': 'not-an-integer',
      });
    });

    test('copies only the selected repeated string key', () {
      const query = QueryParameters({
        'tag': ['new'],
        'legacy_tag': ['old'],
      });
      final values = query.toObjectMap();

      query.copyAll(
        values,
        'tag',
        aliases: const ['legacy_tag'],
      );

      expect(values, {
        'tag': ['new'],
        'legacy_tag': 'old',
      });
    });

    test('throws the field rejection when scalar coercion fails', () {
      const query = QueryParameters({
        'page': ['many'],
      });

      expect(
        () => query.coerceOne<int>(query.toObjectMap(), 'page'),
        throwsA(isA<Rejection>()),
      );
    });

    test('throws the field rejection when list coercion fails', () {
      const query = QueryParameters({
        'id': ['1', 'many'],
      });

      expect(
        () => query.coerceAll<int>(query.toObjectMap(), 'id'),
        throwsA(isA<Rejection>()),
      );
    });
  });

  group('QueryModelExtractable', () {
    test('decodes a prepared model', () async {
      final extractor = QueryModelExtractable<Map<String, Object?>>((query) {
        final values = query.toObjectMap();
        query.coerceOne<int>(values, 'page');
        query.copyAll(values, 'tag');
        return values;
      });
      final outcome = await extractor.extract(
        request('GET', '/todos?page=2&tag=a&tag=b'),
      );

      expect(expectOk(outcome), {
        'page': 2,
        'tag': ['a', 'b'],
      });
    });

    test('keeps a field coercion rejection as 400', () async {
      final extractor = QueryModelExtractable<Map<String, Object?>>((query) {
        final values = query.toObjectMap();
        query.coerceOne<int>(values, 'page');
        return values;
      });
      final outcome = await extractor.extract(
        request('GET', '/todos?page=many'),
      );

      expect(expectStatus(outcome, 400).message, contains('query "page"'));
    });

    test('reports a model shape failure as 400', () async {
      final extractor = QueryModelExtractable<int>(
        (_) => throw const FormatException('missing page'),
      );
      final outcome = await extractor.extract(request('GET', '/todos'));

      expect(expectStatus(outcome, 400).message, contains('missing page'));
    });
  });

  group('QueriesExtractable', () {
    test('decodes the whole query map', () async {
      final extractor = QueriesExtractable<Map<String, String>>((raw) => raw);
      final outcome = await extractor.extract(request('GET', '/todos?a=1&b=2'));

      expect(expectOk(outcome), {'a': '1', 'b': '2'});
    });

    test('reports a failed decode as 422', () async {
      final extractor = QueriesExtractable<int>(
        (raw) => throw const FormatException('bad shape'),
      );
      final outcome = await extractor.extract(request('GET', '/todos?a=1'));

      expectStatus(outcome, 422);
    });
  });

  group('RawQueryExtractable', () {
    test('returns the undecoded query', () async {
      final outcome = await const RawQueryExtractable()
          .extract(request('GET', '/todos?a=1&a=2'));

      expect(expectOk(outcome), 'a=1&a=2');
    });

    test('returns null when there is no query', () async {
      final outcome =
          await const RawQueryExtractable().extract(request('GET', '/todos'));

      expect(expectOk(outcome), isNull);
    });
  });
}
