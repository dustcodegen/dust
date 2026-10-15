import 'package:dust_dart/db.dart';
import 'package:server_app/server_app.dart';
import 'package:server_app/src/shared/db/checked_query_example.dart';

/// All five terminals without a DAO, on a connection or transaction.
///
/// `dart run example/checked_queries.dart` (sync), or append `--background`.
Future<void> main(List<String> arguments) async {
  final database = arguments.contains('--background')
      ? await AppDatabase.openBackground(':memory:')
      : AppDatabase.open(':memory:');
  try {
    for (final line in await runCheckedQueries(database.connection)) {
      print(line);
    }
  } finally {
    (await database.close()).unwrap();
  }
}
