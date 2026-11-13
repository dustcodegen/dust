import 'package:dust_server/server.dart';
import 'package:test/test.dart';

final class _Bearer implements FromRequestParts<String> {
  const _Bearer();

  @override
  Future<Result<String, Rejection>> extract(Request request) async {
    return const Ok('ada');
  }
}

void main() {
  group('verbs', () {
    test('report their HTTP method', () {
      expect(const GET('/').method, 'GET');
      expect(const POST('/').method, 'POST');
      expect(const PUT('/').method, 'PUT');
      expect(const PATCH('/').method, 'PATCH');
      expect(const DELETE('/').method, 'DELETE');
      expect(const HEAD('/').method, 'HEAD');
      expect(const OPTIONS('/').method, 'OPTIONS');
    });

    test('default every optional argument', () {
      const verb = GET('/todos');

      expect(verb.path, '/todos');
      expect(verb.status, isNull);
      expect(verb.summary, isNull);
      expect(verb.description, isNull);
      expect(verb.tags, isEmpty);
      expect(verb.middleware, isEmpty);
      expect(verb.operationId, isNull);
      expect(verb.deprecated, isFalse);
      expect(verb.hidden, isFalse);
    });

    test('carry the full argument set on one annotation', () {
      const verb = POST(
        '/',
        status: 201,
        summary: 'Create a todo',
        description: 'Creates a todo owned by the authenticated user.',
        tags: ['todos'],
        operationId: 'createTodo',
        deprecated: true,
        hidden: true,
      );

      expect(verb.status, 201);
      expect(verb.summary, 'Create a todo');
      expect(verb.description, startsWith('Creates a todo'));
      expect(verb.tags, ['todos']);
      expect(verb.operationId, 'createTodo');
      expect(verb.deprecated, isTrue);
      expect(verb.hidden, isTrue);
    });

    test('are a sealed family', () {
      const Verb verb = DELETE('/{id}');

      expect(verb, isA<Verb>());
    });
  });

  group('Controller', () {
    test('defaults tags, middleware, and responses', () {
      const controller = Controller('/todos');

      expect(controller.path, '/todos');
      expect(controller.tags, isEmpty);
      expect(controller.middleware, isEmpty);
    });
  });

  group('WS', () {
    test('defaults typed upgrade options and uses GET', () {
      const route = WS('/chat');

      expect(route.path, '/chat');
      expect(route.method, 'GET');
      expect(route.status, isNull);
      expect(route.protocols, isNull);
      expect(route.allowedOrigins, isNull);
      expect(route.summary, isNull);
      expect(route.description, isNull);
      expect(route.tags, isEmpty);
      expect(route.middleware, isEmpty);
      expect(route.operationId, isNull);
      expect(route.deprecated, isFalse);
      expect(route.hidden, isFalse);
    });

    test('retains protocol, origin, and inherited route metadata', () {
      const route = WS(
        '/chat',
        protocols: ['chat.v1'],
        allowedOrigins: ['https://chat.example'],
        summary: 'Join chat',
        description: 'Exchanges typed messages after authentication.',
        tags: ['chat'],
        middleware: [RequestId()],
        operationId: 'joinChat',
        deprecated: true,
        hidden: true,
      );

      expect(route.protocols, ['chat.v1']);
      expect(route.allowedOrigins, ['https://chat.example']);
      expect(route.summary, 'Join chat');
      expect(route.description, startsWith('Exchanges typed messages'));
      expect(route.tags, ['chat']);
      expect(route.middleware.single, isA<RequestId>());
      expect(route.operationId, 'joinChat');
      expect(route.deprecated, isTrue);
      expect(route.hidden, isTrue);
    });
  });

  group('Routes', () {
    test('defaults its optional arguments', () {
      const routes = Routes('/api');

      expect(routes.path, '/api');
      expect(routes.name, isNull);
      expect(routes.tags, isEmpty);
      expect(routes.middleware, isEmpty);
    });

    test('carries its complete argument set', () {
      const routes = Routes('/api', name: 'apiRoutes', tags: ['public']);

      expect(routes.name, 'apiRoutes');
      expect(routes.tags, ['public']);
    });
  });

  group('parameter annotations', () {
    test('Extract carries the extractor type, not an instance', () {
      const extract = Extract(_Bearer);

      expect(extract.extractor, _Bearer);
      expect(extract.extractor, isA<Type>());
    });

    test('Path defaults its key to the parameter name', () {
      expect(const Path().name, isNull);
      expect(const Path('todo_id').name, 'todo_id');
    });

    test('Query supports named values and typed models', () {
      expect(const Query().name, isNull);
      expect(const Query('done').name, 'done');
    });

    test('other keyed annotations require an explicit key', () {
      expect(const Header('x-request-id').name, 'x-request-id');
      expect(const Field('email').name, 'email');
      expect(const Part('avatar').name, 'avatar');
    });

    test('State is keyed by the parameter type alone', () {
      expect(const State(), isA<State>());
    });

    test('marker annotations are const-constructible', () {
      expect(const Queries(), isA<Queries>());
      expect(const RawQuery(), isA<RawQuery>());
      expect(const HeaderMap(), isA<HeaderMap>());
      expect(const Body(), isA<Body>());
      expect(const RawBody(), isA<RawBody>());
      expect(const Peer(), isA<Peer>());
      expect(const FormUrlEncoded(), isA<FormUrlEncoded>());
      expect(const MultiPart(), isA<MultiPart>());
    });
  });

  group('extractor contract', () {
    test('an extractor named in @Extract has a zero-argument const ctor', () {
      const extractor = _Bearer();

      expect(extractor, isA<FromRequestParts<String>>());
    });
  });
}
