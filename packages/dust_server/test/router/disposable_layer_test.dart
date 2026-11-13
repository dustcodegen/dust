import 'dart:async';
import 'dart:io';

import 'package:dust_server/server.dart';
import 'package:http/http.dart' as http;
import 'package:test/test.dart';

/// A layer that owns something and records when it was released.
final class _Owning implements DisposableLayer {
  _Owning(this.name, this.closed);

  final String name;
  final List<String> closed;

  @override
  Middleware toMiddleware() => (inner) => inner;

  @override
  Future<void> dispose() async => closed.add(name);
}

final class _Throwing implements DisposableLayer {
  _Throwing(this.after);

  final List<String> after;

  @override
  Middleware toMiddleware() => (inner) => inner;

  @override
  Future<void> dispose() async => throw StateError('bad teardown');
}

final class _OnDispose implements DisposableLayer {
  _OnDispose(this.callback);

  final Future<void> Function() callback;

  @override
  Middleware toMiddleware() => (inner) => inner;

  @override
  Future<void> dispose() => callback();
}

void main() {
  group('DisposableLayer', () {
    test('is disposed when the server closes', () async {
      final closed = <String>[];
      final app = Router()
        ..layer(_Owning('outer', closed))
        ..route('/', get((request) => 'ok'));

      final server = await serve(app, InternetAddress.loopbackIPv4, 0);
      expect(closed, isEmpty, reason: 'not until shutdown');

      await server.close(drain: const Duration(seconds: 1));
      expect(closed, ['outer']);
    });

    test('is found through nest and routeLayer', () async {
      final closed = <String>[];
      final inner = Router()
        ..routeLayer(_Owning('nested-route', closed))
        ..route('/deep', get((request) => 'ok'));
      final app = Router()
        ..layer(_Owning('root', closed))
        ..nest('/api', inner);

      final server = await serve(app, InternetAddress.loopbackIPv4, 0);
      await server.close(drain: const Duration(seconds: 1));

      expect(closed, containsAll(['root', 'nested-route']));
    });

    test('a plain Layer is left alone', () async {
      final app = Router()
        ..layer(RequestId())
        ..route('/', get((request) => 'ok'));

      final server = await serve(app, InternetAddress.loopbackIPv4, 0);
      await expectLater(
        server.close(drain: const Duration(seconds: 1)),
        completion(isTrue),
      );
    });

    test('one that throws does not stop the others', () async {
      final closed = <String>[];
      final app = Router()
        ..layer(_Throwing(closed))
        ..layer(_Owning('after-the-bad-one', closed))
        ..route('/', get((request) => 'ok'));

      final server = await serve(app, InternetAddress.loopbackIPv4, 0);
      await server.close(drain: const Duration(seconds: 1));

      expect(closed, ['after-the-bad-one']);
    });

    test('one instance on two routers is disposed once', () async {
      // A layer applied at the root and again on a subtree owns one resource.
      // Disposing per registration closes it twice, and the second failure is
      // swallowed by the guard around dispose, so it fails silently.
      final closed = <String>[];
      final shared = _Owning('shared', closed);
      final inner = Router()
        ..layer(shared)
        ..route('/i', get((request) => 'ok'));
      final app = Router()
        ..layer(shared)
        ..nest('/in', inner);

      final server = await serve(app, InternetAddress.loopbackIPv4, 0);
      await server.close(drain: const Duration(seconds: 1));

      expect(closed, ['shared']);
    });

    test('two layers that compare equal are both disposed', () async {
      // Deduplication is by identity. Two separate instances that happen to be
      // equal each own their own resource, and collapsing them leaks one.
      final closed = <String>[];
      final app = Router()
        ..layer(_Owning('same-name', closed))
        ..layer(_Owning('same-name', closed))
        ..route('/', get((request) => 'ok'));

      final server = await serve(app, InternetAddress.loopbackIPv4, 0);
      await server.close(drain: const Duration(seconds: 1));

      expect(closed, ['same-name', 'same-name']);
    });

    test('the deadline starts disposal with requests and tasks pending',
        () async {
      final requestGate = Completer<void>();
      final taskGate = Completer<void>();
      final entered = Completer<void>();
      final disposed = Completer<void>();
      final tasks = BackgroundTasks();
      final app = Router()
        ..layer(_OnDispose(() async => disposed.complete()))
        ..route('/hang', get((request) async {
          tasks.run('pending', () => taskGate.future);
          entered.complete();
          await requestGate.future;
          return 'late';
        }));

      final server = await serve(
        app,
        InternetAddress.loopbackIPv4,
        0,
        background: tasks,
      );
      addTearDown(() {
        if (!requestGate.isCompleted) requestGate.complete();
        if (!taskGate.isCompleted) taskGate.complete();
      });
      final request =
          http.get(Uri.parse('http://127.0.0.1:${server.port}/hang'));
      await entered.future.timeout(const Duration(seconds: 10));

      final settled =
          await server.close(drain: const Duration(milliseconds: 50));
      expect(settled, isFalse);
      expect(tasks.isClosed, isTrue);
      expect(disposed.isCompleted, isTrue);

      requestGate.complete();
      expect((await request).body, 'late');

      taskGate.complete();
      expect(await tasks.settled(const Duration(seconds: 2)), isTrue);
    });

    test('the deadline releases a stream owned by a disposable layer',
        () async {
      final events = StreamController<List<int>>();
      final app = Router()
        ..layer(_OnDispose(events.close))
        ..route('/', get((request) {
          events.add([1]);
          return Response.ok(events.stream);
        }));
      final server = await serve(app, InternetAddress.loopbackIPv4, 0);
      final client = HttpClient();
      addTearDown(() {
        client.close(force: true);
        if (!events.isClosed) unawaited(events.close());
      });
      final request = await client.getUrl(
        Uri.parse('http://127.0.0.1:${server.port}/'),
      );
      final response = await request.close();
      final responseDone = response.drain<void>();

      try {
        expect(
          await server.close(drain: const Duration(milliseconds: 50)),
          isFalse,
        );
        expect(events.isClosed, isTrue);
      } finally {
        if (!events.isClosed) unawaited(events.close());
        await responseDone.timeout(const Duration(seconds: 2));
      }
    });

    test('concurrent closes share the first deadline and one disposal',
        () async {
      final started = Completer<void>();
      final disposeGate = Completer<void>();
      final nextDisposed = Completer<void>();
      var disposeCalls = 0;
      final app = Router()
        ..layer(_OnDispose(() async {
          disposeCalls++;
          if (!started.isCompleted) started.complete();
          await disposeGate.future;
          throw StateError('late teardown failure');
        }))
        ..layer(_OnDispose(() async => nextDisposed.complete()))
        ..route('/', get((request) => 'ok'));
      final server = await serve(app, InternetAddress.loopbackIPv4, 0);
      addTearDown(() {
        if (!disposeGate.isCompleted) disposeGate.complete();
      });

      final first = server.close(drain: const Duration(milliseconds: 50));
      final second = server.close(drain: const Duration(seconds: 10));
      expect(second, same(first));
      await started.future.timeout(const Duration(seconds: 2));
      expect(await first.timeout(const Duration(seconds: 2)), isFalse);
      expect(disposeCalls, 1);
      expect(nextDisposed.isCompleted, isFalse);

      disposeGate.complete();
      await nextDisposed.future.timeout(const Duration(seconds: 2));
      expect(await server.close(), isFalse);
      expect(disposeCalls, 1);
    });

    test('is disposed once, not again on a second close', () async {
      final closed = <String>[];
      final app = Router()
        ..layer(_Owning('once', closed))
        ..route('/', get((request) => 'ok'));

      final server = await serve(app, InternetAddress.loopbackIPv4, 0);
      await server.close(drain: const Duration(seconds: 1));
      await server.close(drain: const Duration(seconds: 1));

      expect(closed, ['once']);
    });
  });

  group('Router as a Service', () {
    test('answers a request directly', () async {
      final app = Router()..route('/', get((request) => 'hello'));

      final response = await app(Request('GET', Uri.parse('http://x/')));
      expect(await response.readAsString(), 'hello');
    });

    test('tears off where a Handler is wanted', () async {
      final app = Router()..route('/', get((request) => 'hello'));

      final handler = app.call;
      final response = await handler(Request('GET', Uri.parse('http://x/')));
      expect(response.statusCode, 200);
    });

    test('is a Service', () {
      expect(Router(), isA<Service>());
    });
  });
}
