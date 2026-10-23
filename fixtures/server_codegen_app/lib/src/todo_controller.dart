import 'package:dust_server/server.dart';

part 'todo_controller.g.dart';

/// Request payload used by the generated JSON body extractor.
final class CreateTodo {
  /// Creates a todo payload.
  const CreateTodo({required this.title});

  /// Decodes a todo payload from JSON.
  factory CreateTodo.deserialize(Map<String, Object?> json) {
    return CreateTodo(title: json['title']! as String);
  }

  /// Requested title.
  final String title;
}

/// Small controller proving generated GET and POST wiring.
@Controller('/todos')
final class TodoController with _$TodoController {
  /// Reads one todo through path, query, and header extraction.
  @GET('/{id}')
  Future<Map<String, Object?>> read(
    @Path() int id, {
    @Query('done') bool? done,
    @Header('x-trace-id') String? traceId,
  }) async =>
      {'id': id, 'done': done, 'traceId': traceId};

  /// Creates one todo through JSON body extraction.
  @POST('/', status: 201)
  Future<Map<String, Object?>> create(@Body() CreateTodo input) async {
    return {'title': input.title};
  }

  /// Clears todos and proves generated void-to-204 dispatch.
  @DELETE('/')
  Future<void> clear() async {}

  /// Returns an already-built response without replacing it.
  @OPTIONS('/raw')
  Future<Response> raw() async => Response(202, body: 'raw');
}

/// Proves top-level handler generation beside a controller.
@GET('/health')
Future<Map<String, Object?>> health() async => {'status': 'ok'};
