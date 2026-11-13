import 'package:dio/dio.dart';
import 'package:server_client_app/websocket_client.dart';
import 'package:test/test.dart';

final class _Attempt {
  final stopped = StateError('stop before opening a transport');
  Uri? uri;
  Map<String, Object?>? headers;
  List<String>? protocols;

  Never connect(
    Uri uri, {
    Iterable<String>? protocols,
    Map<String, Object?>? headers,
  }) {
    this.uri = uri;
    this.protocols = protocols?.toList();
    this.headers = headers == null ? null : Map.of(headers);
    throw stopped;
  }
}

Future<_Attempt> _capture(
  Dio dio, {
  String? baseUrl,
  String room = 'lobby',
}) async {
  final attempt = _Attempt();
  final client = ChatClient(
    dio,
    baseUrl: baseUrl,
    webSocketConnector: attempt.connect,
  );
  await expectLater(
    client.chat(room, sequence: 7, token: 'Bearer fixture-token'),
    throwsA(same(attempt.stopped)),
  );
  expect(attempt.headers, {'authorization': 'Bearer fixture-token'});
  expect(attempt.protocols, ['dust.chat.v1']);
  return attempt;
}

void main() {
  late Dio dio;

  setUp(() {
    dio = Dio(BaseOptions(baseUrl: 'http://chat.example/api/'));
  });
  tearDown(() => dio.close(force: true));

  for (final (scheme, expected) in [
    ('http', 'ws'),
    ('https', 'wss'),
    ('ws', 'ws'),
    ('wss', 'wss'),
  ]) {
    test('generated client maps $scheme to $expected before connecting',
        () async {
      dio.options.baseUrl = '$scheme://chat.example/api/';
      final attempt = await _capture(dio);

      expect(
        attempt.uri,
        Uri.parse('$expected://chat.example/api/rooms/lobby?sequence=7'),
      );
    });
  }

  test('leading route slash retains the Dio base path with either suffix',
      () async {
    for (final base in [
      'http://chat.example/api/v1',
      'http://chat.example/api/v1/',
    ]) {
      dio.options.baseUrl = base;

      expect((await _capture(dio)).uri?.path, '/api/v1/rooms/lobby');
    }
  });

  test('room slash and percent values are encoded exactly once', () async {
    const room = 'a/b %25';
    final uri = (await _capture(dio, room: room)).uri!;

    expect(
      uri.toString(),
      'ws://chat.example/api/rooms/a%2Fb%20%2525?sequence=7',
    );
    expect(uri.pathSegments, ['api', 'rooms', room]);
  });

  test('base query repeats survive while the bound sequence replaces its key',
      () async {
    dio.options.baseUrl =
        'http://chat.example/api?tag=one&tag=two&sequence=1&keep=base';
    final uri = (await _capture(dio)).uri!;

    expect(uri.queryParametersAll, {
      'tag': ['one', 'two'],
      'sequence': ['7'],
      'keep': ['base'],
    });
  });

  test('absolute override replaces the Dio host path and query', () async {
    dio.options.baseUrl = 'http://original.example/base/?old=discarded';
    final uri = (await _capture(
      dio,
      baseUrl: 'https://override.example/chat?selected=yes',
    ))
        .uri!;

    expect(
      uri,
      Uri.parse(
        'wss://override.example/chat/rooms/lobby?selected=yes&sequence=7',
      ),
    );
  });

  test('relative overrides resolve before Dio appends the route', () async {
    dio.options.baseUrl = 'http://chat.example/base/v1/?old=discarded';
    for (final (override, path, query) in [
      ('../v2/?selected=relative', '/base/v2/rooms/lobby', 'relative'),
      ('/alternate?selected=root', '/alternate/rooms/lobby', 'root'),
    ]) {
      final uri = (await _capture(dio, baseUrl: override)).uri!;

      expect(uri.scheme, 'ws');
      expect(uri.host, 'chat.example');
      expect(uri.path, path);
      expect(uri.queryParameters, {'selected': query, 'sequence': '7'});
    }
  });

  test('blank override retains the configured Dio base URL', () async {
    for (final override in ['', '   ']) {
      expect(
        (await _capture(dio, baseUrl: override)).uri,
        Uri.parse('ws://chat.example/api/rooms/lobby?sequence=7'),
      );
    }
  });

  for (final override in [
    'ftp://chat.example/api',
    'file:///tmp/chat',
    'http:/missing-host',
    '#fragment',
    'https://chat.example/api#fragment',
  ]) {
    test('invalid base $override rejects before invoking the connector',
        () async {
      final attempt = _Attempt();
      final client = ChatClient(
        dio,
        baseUrl: override,
        webSocketConnector: attempt.connect,
      );

      await expectLater(
        client.chat('lobby', sequence: 7, token: 'Bearer fixture-token'),
        throwsA(isArgumentError),
      );
      expect(attempt.uri, isNull);
    });
  }

  test('missing Dio host rejects before invoking the connector', () async {
    dio.options.baseUrl = '';
    final attempt = _Attempt();
    final client = ChatClient(dio, webSocketConnector: attempt.connect);

    await expectLater(
      client.chat('lobby', sequence: 7, token: 'Bearer fixture-token'),
      throwsA(isArgumentError),
    );
    expect(attempt.uri, isNull);
  });
}
