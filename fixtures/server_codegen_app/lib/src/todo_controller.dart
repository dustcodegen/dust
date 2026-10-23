import 'dart:typed_data';

import 'package:dust_dart/serde.dart';
import 'package:dust_server/server.dart';

part 'todo_controller.g.dart';

/// Authenticated request identity produced by a custom extractor.
final class AuthUser {
  /// Creates an authenticated identity.
  const AuthUser(this.id);

  /// Stable user identifier.
  final String id;
}

/// Reads an optional bearer token from the request.
class BearerAuth implements FromRequestParts<AuthUser> {
  /// Creates a bearer extractor, optionally requiring [scope].
  const BearerAuth({this.scope});

  /// Scope required by this extractor variant.
  final String? scope;

  @override
  Future<Result<AuthUser, Rejection>> extract(Request request) async {
    final value = request.headers['authorization'];
    if (value == null || !value.startsWith('Bearer ')) {
      return const Err(Rejection.unauthorized('missing bearer token'));
    }
    final token = value.substring(7);
    final separator = token.indexOf(':');
    final id = separator < 0 ? token : token.substring(0, separator);
    final grantedScope = separator < 0 ? null : token.substring(separator + 1);
    if (scope case final requiredScope? when grantedScope != requiredScope) {
      return Err(Rejection.forbidden('requires scope $requiredScope'));
    }
    return Ok(AuthUser(id));
  }
}

/// Requires write access while keeping zero-argument construction.
final class TodosWrite extends BearerAuth {
  /// Creates the configured write extractor.
  const TodosWrite() : super(scope: 'todos:write');
}

/// Request payload used by generated JSON and form body extractors.
@Derive([Validate()])
final class CreateTodo with _$CreateTodo {
  /// Creates a todo payload.
  const CreateTodo({required this.title});

  /// Decodes a todo payload from JSON.
  factory CreateTodo.deserialize(Map<String, Object?> json) {
    return CreateTodo(title: json['title']! as String);
  }

  /// Requested title.
  @Validate(length: Length(min: 1), message: 'must not be empty')
  final String title;
}

/// State attached to the fixture router by type.
final class TodoState {
  /// Creates fixture state.
  const TodoState(this.name);

  /// Stable state marker.
  final String name;
}

/// Records generated and handwritten layer order in one request header.
final class TraceLayer implements Layer {
  /// Creates a trace marker for [name].
  const TraceLayer(this.name);

  /// Marker appended while the request enters this layer.
  final String name;

  @override
  Middleware toMiddleware() {
    return (inner) {
      return (request) {
        final current = request.headers['x-layer-order'];
        final next = current == null ? name : '$current,$name';
        return inner(request.change(headers: {'x-layer-order': next}));
      };
    };
  }
}

/// Small controller proving generated GET and POST wiring.
@Controller(
  '/todos',
  tags: ['todos'],
  middleware: [TraceLayer('controller')],
)
final class TodoController with _$TodoController {
  /// Requires a custom extractor and short-circuits its rejection.
  @GET('/required-auth')
  Future<Map<String, Object?>> requiredAuth(
    @Extract(BearerAuth) AuthUser user,
  ) async =>
      {'user': user.id};

  /// Turns a custom extractor's client rejection into `None`.
  @GET('/optional-auth')
  Future<Map<String, Object?>> optionalAuth(
    @Extract(BearerAuth) Option<AuthUser> user,
  ) async =>
      {'authenticated': user.isSome};

  /// Applies the same optional behavior to a built-in query extractor.
  @GET('/optional-page')
  Future<Map<String, Object?>> optionalPage(
    @Query('page') Option<int> page,
  ) async =>
      {'present': page.isSome};

  /// Hands a scoped extractor's result to the handwritten handler.
  @GET('/fallible-write-auth')
  Future<Map<String, Object?>> fallibleWriteAuth(
    @Extract(TodosWrite) Result<AuthUser, Rejection> user,
  ) async =>
      {'authorized': user.isOk};

  /// Requires one integer query value.
  @GET('/query')
  Future<int> query(@Query('page') int page) async => page;

  /// Creates one todo through JSON body extraction.
  @POST('/', status: 201)
  Future<Map<String, Object?>> create(@Body() CreateTodo input) async {
    return {'title': input.title};
  }

  /// Decodes one model from a URL-encoded body and validates it.
  @FormUrlEncoded()
  @POST('/form-model')
  Future<Map<String, Object?>> formModel(@Body() CreateTodo input) async =>
      {'title': input.title};

  /// Reads several URL-encoded fields from one body pass.
  @FormUrlEncoded()
  @POST('/form-fields')
  Future<Map<String, Object?>> formFields(
    @Field('title') String title,
    @Field('priority') Option<int> priority,
  ) async =>
      {'title': title, 'priority': priority.isSome};

  /// Reads text and file parts from one multipart body pass.
  @MultiPart()
  @POST('/multipart')
  Future<Map<String, Object?>> multipart(
    @Part('title') String title,
    @Part('file') UploadedFile file,
  ) async =>
      {'title': title, 'filename': file.filename, 'size': file.bytes.length};

  /// Passes raw text to the handler and returns plain text.
  @POST('/raw-text')
  Future<String> rawText(@RawBody() String body) async => body;

  /// Passes raw bytes to the handler and returns bytes.
  @POST('/raw-bytes', status: 202)
  Future<Uint8List> rawBytes(@RawBody() Uint8List body) async => body;

  /// Passes an unread request stream through to the response.
  @POST('/raw-stream')
  Future<Stream<List<int>>> rawStream(
    @RawBody() Stream<List<int>> body,
  ) async =>
      body;

  /// Extracts raw query, state, peer, and immutable request parts.
  @GET('/context')
  Future<Map<String, Object?>> context(
    @RawQuery() String? raw,
    @State() TodoState state,
    @Peer() PeerInfo peer,
    RequestParts parts,
  ) async =>
      {
        'raw': raw,
        'state': state.name,
        'peer': peer.remoteAddress,
        'method': parts.method,
      };

  /// Passes the shelf request itself through unchanged.
  @GET('/request')
  Future<String> request(Request request) async => request.method;

  /// Exposes root, controller, and route middleware order.
  @GET(
    '/layers',
    summary: 'Show middleware order',
    description: 'Returns the layer names observed by the generated handler.',
    tags: ['diagnostics'],
    middleware: [TraceLayer('route')],
    operationId: 'traceLayers',
    deprecated: true,
    hidden: true,
  )
  Future<String> layers(Request request) async =>
      request.headers['x-layer-order'] ?? '';

  /// Proves response dispatch for Result, Option, and Unit.
  @GET('/result')
  Future<Result<String, Rejection>> result() async => const Ok('ok');

  /// Proves a missing optional result becomes 404.
  @GET('/option')
  Future<Option<String>> option() async => const None();

  /// Proves an explicit empty success becomes 204.
  @POST('/unit')
  Future<Unit> unitResult() async => unit;

  /// Proves unexpected handler failures are guarded as 500 responses.
  @GET('/failure')
  Future<String> failure() async => throw StateError('fixture failure');

  /// Reads one todo after literal GET routes, so the placeholder is fallback.
  @GET('/{id}')
  Future<Map<String, Object?>> read(
    @Path() int id, {
    @Query('done') bool? done,
    @Header('x-trace-id') String? traceId,
  }) async =>
      {'id': id, 'done': done, 'traceId': traceId};

  /// Clears todos and proves generated void-to-204 dispatch.
  @DELETE('/')
  Future<void> clear() async {}

  /// Returns an already-built response without replacing it.
  @OPTIONS('/raw')
  Future<Response> raw() async => Response(202, body: 'raw');
}

/// Proves top-level handler generation beside a controller.
@GET('/health', summary: 'Check fixture health', tags: ['operations'])
Future<Map<String, Object?>> health() async => {'status': 'ok'};
