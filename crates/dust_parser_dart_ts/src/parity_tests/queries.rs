//! Direct db/tx query calls must keep their complete parser-owned facts.

use super::{contract, parse};

#[test]
fn query_terminals_types_and_parameter_lists() {
    let library = parse(
        r#"
void load() {
  queryAs<UserRow>(r'SELECT * FROM users WHERE id = $1', [id]).fetchOne(db);
  queryAs<UserRow>(r'SELECT * FROM users', const <Object?>[]).fetchOptional(tx);
  queryAs<List<UserRow>>(r'SELECT * FROM users', [orgId]).fetchAll(db);
  queryAs<UserRow>(r'SELECT * FROM users', []).fetch(tx);
  queryAs<UserRow>(r'SELECT * FROM users', []).fetchOneWith(db, mapper);
  queryAs<UserRow>(r'SELECT * FROM users', []).fetchOptionalWith(tx, mapper);
  queryAs<UserRow>(r'SELECT * FROM users', []).fetchAllWith(db, mapper);
  queryScalar<int>(r'SELECT COUNT(*) FROM users', const <Object?>[]).fetchOne(tx);
  queryExecute(r'''UPDATE users
SET active = 1 WHERE id = $1''', [id]).execute(db);
}
"#,
    );
    assert_eq!(library.query_calls.len(), 9, "{:#?}", library.query_calls);
    contract("query-terminals", library);
}

#[test]
fn query_context_keeps_classes_methods_closures_and_top_level_names() {
    let library = parse(
        r#"
class OrdersService {
  Future<void> place() async {
    queryExecute(r'DELETE FROM orders').execute(db);
    run(() { queryScalar<int>('SELECT 1', []).fetchOne(tx); });
  }
  Future<void> other() => queryExecute('VACUUM').execute(db);
}
void topLevel() { queryExecute('SELECT 1').execute(tx); }
final probe = queryExecute('SELECT 2');
"#,
    );
    assert_eq!(library.query_calls.len(), 5);
    contract("query-context", library);
}

#[test]
fn unchecked_sql_markers_types_and_terminals() {
    let library = parse(
        r#"
void maintenance() {
  // dust:allow-unsafe-sql
  db.unsafe.execute('VACUUM', const []);
  tx.unsafe.fetch('PRAGMA table_info(users)', const []); // dust:allow-unsafe-sql
  // dust:allow-unsafe-sql

  db.unsafe.fetchAs<UserRow>('SELECT id FROM users', const [], mapper);
  tx.unsafe.fetch('SELECT 1', const []);
}
"#,
    );
    assert_eq!(library.query_calls.len(), 4);
    contract("query-unsafe", library);
}

#[test]
fn dynamic_sql_and_false_positives() {
    let library = parse(
        r#"
void run() {
  // 🌿 queryExecute('not SQL', []).execute(db);
  final text = 'queryExecute("also ignored", [])';
  myqueryExecute('ignored', []);
  queryExecute('DELETE FROM $table', args).execute(db);
  queryExecute('DELETE ' 'FROM users', []).execute(tx);
  queryAs<UserRow>(sql, []).fetching(db);
}
"#,
    );
    assert_eq!(library.query_calls.len(), 3);
    assert!(
        library
            .query_calls
            .iter()
            .all(|call| !call.sql_source_static)
    );
    contract("query-dynamic", library);
}
