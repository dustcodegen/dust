use std::fs;

use dust_driver::{BuildRequest, DbRequestOptions, run_build};

use crate::support::{generated_output, make_workspace, write_file};

const OPENER: &str = r#"
  static Future<AppDatabase> openBackground(
    String path, {SqliteConnectOptions? options}
  ) => _$AppDatabase.openBackground(path, options: options);
"#;

#[test]
fn static_opt_in_is_lowered_and_invalidates_cached_facade_output() {
    let workspace = make_workspace();
    write_file(
        &workspace.path().join("migrations/0001.sql"),
        "CREATE TABLE items (id INTEGER);\n",
    );
    let source = |opener: &str| {
        format!(
            r#"import 'package:dust_dart/db.dart';
import 'package:dust_db_sqlite3/dust_db_sqlite3.dart';
part 'db.g.dart';
@SqlxDatabase(type: SqlxDatabaseType.sqlite, migrations: './migrations')
abstract class AppDatabase implements DatabaseClient {{
  factory AppDatabase.open(String path, {{SqliteConnectOptions? options}}) = _$AppDatabase.open;
{opener}
  @override
  Connection get connection;
}}
"#
        )
    };
    let input = workspace.path().join("lib/db.dart");
    let output = workspace.path().join("lib/db.g.dart");
    let build = || {
        let result = run_build(BuildRequest {
            cwd: workspace.path().to_path_buf(),
            fail_fast: true,
            jobs: None,
            db: DbRequestOptions {
                only_db: true,
                offline: false,
            },
        });
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        fs::read_to_string(&output).unwrap()
    };
    write_file(&input, &source(""));
    let synchronous = build();
    write_file(&input, &source(OPENER));
    assert_eq!(
        build(),
        generated_output(include_str!("background.dart.snapshot"))
    );
    assert_eq!(
        build(),
        generated_output(include_str!("background.dart.snapshot"))
    );
    write_file(&input, &source(""));
    assert_eq!(build(), synchronous);
}
