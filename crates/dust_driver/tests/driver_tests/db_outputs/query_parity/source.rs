use std::{fs, path::Path};

use crate::support::write_file;

pub(super) fn write_probe(root: &Path, dao: bool, sql: &str, row: bool) {
    let (result, helper, terminal) = if row {
        ("List<UserProfile>", "queryAs<UserProfile>", "fetchAll")
    } else {
        ("int", "queryScalar<int>", "fetchOne")
    };
    let body = if dao {
        format!("@Query(r'{sql}') Future<Result<{result}, SqlxError>> probe();")
    } else {
        format!(
            "Future<Result<{result}, SqlxError>> probe(Executor db) => {helper}(r'{sql}', []).{terminal}(db);"
        )
    };
    write_source(root, dao, &body);
}

pub(super) fn write_terminals(root: &Path, dao: bool) {
    let body = if dao {
        r"@Query('SELECT id, display_name, bio FROM users')
Future<Result<UserProfile, SqlxError>> one();
@Query('SELECT id, display_name, bio FROM users')
Future<Result<UserProfile?, SqlxError>> optional();
@Query('SELECT id, display_name, bio FROM users')
Future<Result<List<UserProfile>, SqlxError>> all();
@Query('SELECT COUNT(*) FROM users')
Future<Result<int, SqlxError>> count();
@Query('DELETE FROM users')
Future<Result<ExecResult, SqlxError>> clear();"
    } else {
        r"Future<Result<UserProfile, SqlxError>> one(Executor db) =>
queryAs<UserProfile>('SELECT id, display_name, bio FROM users', []).fetchOne(db);
Future<Result<UserProfile?, SqlxError>> optional(Executor db) =>
queryAs<UserProfile>('SELECT id, display_name, bio FROM users', []).fetchOptional(db);
Future<Result<List<UserProfile>, SqlxError>> all(Executor db) =>
queryAs<UserProfile>('SELECT id, display_name, bio FROM users', []).fetchAll(db);
Future<Result<int, SqlxError>> count(Executor db) =>
queryScalar<int>('SELECT COUNT(*) FROM users', []).fetchOne(db);
Future<Result<ExecResult, SqlxError>> clear(Executor db) =>
queryExecute('DELETE FROM users', []).execute(db);"
    };
    write_source(root, dao, body);
}

pub(super) fn write_source(root: &Path, dao: bool, body: &str) {
    let declarations = if dao {
        format!(
            "part 'queries.g.dart';\n@SqlxDao()\nabstract final class ProbeDao {{\nconst factory ProbeDao(Executor db) = _$ProbeDao;\n{body}\n}}"
        )
    } else {
        body.to_owned()
    };
    write_file(
        &root.join("lib/queries.dart"),
        &format!(
            "import 'package:dust_dart/db.dart';\nimport 'app_database.dart';\n{declarations}\n"
        ),
    );
}

pub(super) fn query_cache(root: &Path) -> String {
    let files = fs::read_dir(root.join(".dust_sql"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("queries-")
        })
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 1, "{files:?}");
    fs::read_to_string(&files[0]).unwrap()
}
