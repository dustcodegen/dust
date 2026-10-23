@Routes('/todos', name: 'todoLookupRoutes')
library;

import 'package:dust_dart/serde.dart';
import 'package:dust_server/server.dart';
import 'package:server_codegen/src/run_example.dart';

part 'path_and_query.g.dart';

/// Runs generated path and query extraction.
///
/// ```bash
/// dart run lib/path_and_query.dart
/// curl 'localhost:8080/todos/7?completed=true'
/// curl -i localhost:8080/todos/not-an-int
/// ```
Future<void> main() => runExample(buildApp()); // coverage:ignore-line

/// Mounts the generated route group.
Router buildApp() => todoLookupRoutes();

/// Sort choices accepted by [TodoFilter].
@Derive([Deserialize()])
enum TodoOrder {
  /// Newest todos first.
  recent,

  /// Oldest todos first.
  oldest,
}

/// Typed query values for [searchTodos].
@Derive([Deserialize(), Validate()])
@SerDe(disallowUnrecognizedKeys: true)
final class TodoFilter with _$TodoFilter {
  /// Creates a query filter with stable defaults.
  const TodoFilter({
    required this.owner,
    this.completed,
    this.page = 1,
    this.tags = const [],
    this.order = TodoOrder.recent,
  });

  /// Required owner filter.
  final String owner;

  /// Optional completion state, accepting the older `completed` alias.
  @SerDe(rename: 'done', aliases: ['completed'])
  final bool? completed;

  /// One-based result page.
  @SerDe(defaultValue: 1)
  @Validate(range: Range(min: 1), message: 'must be at least 1')
  final int page;

  /// Repeated tag filters.
  @SerDe(defaultValue: <String>[], aliases: ['labels'])
  final List<String> tags;

  /// Result order.
  @SerDe(defaultValue: TodoOrder.recent)
  final TodoOrder order;
}

/// Reads a whole typed query model without a handwritten decoder.
@GET('/search')
Future<Map<String, Object?>> searchTodos(@Query() TodoFilter filter) async => {
      'owner': filter.owner,
      'completed': filter.completed,
      'page': filter.page,
      'tags': filter.tags,
      'order': filter.order.name,
    };

/// Reads typed values without touching the raw request.
@GET('/{id}')
Future<Map<String, Object?>> readTodo(
  @Path() int id, {
  @Query('completed') bool? completed,
}) async =>
    {'id': id, 'completed': completed};
