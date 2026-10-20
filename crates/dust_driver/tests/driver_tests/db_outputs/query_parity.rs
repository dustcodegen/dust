use std::{fs, path::Path};

use dust_driver::{BuildRequest, CheckRequest, DbRequestOptions, run_build, run_check};

use super::helpers::write_db_workspace;
use crate::support::{make_workspace, write_file};

#[path = "query_parity/source.rs"]
mod source;

#[path = "query_parity/mode_switch.rs"]
mod mode_switch;

#[path = "query_parity/static_strings.rs"]
mod static_strings;

#[test]
fn db_mode_validates_a_query_only_library_without_annotations() {
    let workspace = make_workspace();
    write_db_workspace(workspace.path(), false);
    write_file(
        &workspace.path().join("lib/queries.dart"),
        r"import 'package:dust_dart/db.dart';

Future<Result<int, SqlxError>> broken(Executor db) => queryScalar<int>(
  'SELECT missing_column FROM users',
  [],
).fetchOne(db);
",
    );
    let checked = run_check(CheckRequest {
        sdk_source_root: Some(crate::support::sdk_source_root(workspace.path())),
        cwd: workspace.path().to_path_buf(),
        fail_fast: true,
        jobs: None,
        db: DbRequestOptions {
            only_db: true,
            offline: false,
        },
    });
    assert!(checked.has_errors(), "{:?}", checked.diagnostics);
    assert_eq!(
        checked
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "SQLx rejected `broken`: error returned from database: (code: 1) no such column: missing_column"
        ]
    );
}

#[test]
fn dao_and_inline_queries_reject_the_same_invalid_shapes() {
    for dao in [false, true] {
        let name = if dao { "ProbeDao.probe" } else { "probe" };
        for (sql, row, expected) in [
            (
                "SELECT missing_column FROM users",
                false,
                format!(
                    "SQLx rejected `{name}`: error returned from database: (code: 1) no such column: missing_column"
                ),
            ),
            (
                "SELECT id FROM users WHERE id = $1",
                false,
                "query binds 0 args but SQL expects 1 parameters".to_owned(),
            ),
            (
                "SELECT id, bio FROM users",
                true,
                format!(
                    "SQLx query `{name}` does not return required column `display_name` for row `UserProfile`"
                ),
            ),
        ] {
            let workspace = make_workspace();
            write_db_workspace(workspace.path(), false);
            source::write_probe(workspace.path(), dao, sql, row);
            let result = check(workspace.path(), false);
            let mut messages = result
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>();
            // Shape validation and SQLx can both report the same bind error.
            messages.sort_unstable();
            messages.dedup();
            assert_eq!(messages, vec![expected.as_str()]);
        }
    }
}

#[test]
fn all_five_terminals_have_online_and_offline_metadata_in_both_styles() {
    for dao in [false, true] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_db_workspace(root, false);
        source::write_terminals(root, dao);
        let unrelated = root.join("lib/unrelated.g.dart");
        write_file(&unrelated, "// preserve another generator's output\n");
        let built = run_build(BuildRequest {
            sdk_source_root: Some(crate::support::sdk_source_root(root)),
            cwd: root.to_path_buf(),
            fail_fast: true,
            jobs: None,
            db: DbRequestOptions {
                only_db: true,
                offline: false,
            },
        });
        assert_eq!(built.diagnostics, vec![]);
        let cache = source::query_cache(root);
        let json: serde_json::Value = serde_json::from_str(&cache).unwrap();
        let entries = json["entries"].as_array().unwrap();
        let mut modes = entries
            .iter()
            .map(|entry| entry["fetch_mode"].as_str().unwrap())
            .collect::<Vec<_>>();
        modes.sort();
        assert_eq!(modes, ["all", "execute", "one", "one", "optional"]);
        assert!(entries.iter().all(|entry| entry["driver"] == "sqlite3"));
        assert_eq!(check(root, true).diagnostics, vec![]);
        assert_eq!(source::query_cache(root), cache);
        assert_eq!(root.join("lib/queries.g.dart").exists(), dao);
        assert_eq!(
            fs::read_to_string(unrelated).unwrap(),
            "// preserve another generator's output\n"
        );

        write_file(
            &root.join("migrations/0002.sql"),
            "ALTER TABLE users ADD COLUMN active INTEGER;\n",
        );
        let stale = check(root, true);
        assert!(stale.has_errors());
        assert!(
            stale.diagnostics.iter().any(|d| d
                .message
                .starts_with("Database offline query metadata cache is missing entry for `")),
            "{:?}",
            stale.diagnostics
        );
        assert_eq!(source::query_cache(root), cache);
    }
}

#[test]
fn query_only_library_requires_offline_metadata_before_any_online_build() {
    let workspace = make_workspace();
    write_db_workspace(workspace.path(), false);
    source::write_probe(workspace.path(), false, "SELECT COUNT(*) FROM users", false);
    let result = check(workspace.path(), true);
    assert!(result.has_errors());
    let paths = result
        .checked_libraries
        .iter()
        .map(|library| library.source_path.as_path())
        .collect::<Vec<_>>();
    assert!(paths.contains(&workspace.path().join("lib/queries.dart").as_path()));
    assert!(!workspace.path().join(".dust_sql").exists());
}

#[test]
fn type_warning_boundary_is_explicit_for_online_and_offline_queries() {
    for dao in [false, true] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_db_workspace(root, false);
        source::write_probe(
            root,
            dao,
            "SELECT display_name AS id, display_name, bio FROM users",
            true,
        );
        let built = run_build(BuildRequest {
            sdk_source_root: Some(crate::support::sdk_source_root(root)),
            cwd: root.to_path_buf(),
            fail_fast: true,
            jobs: None,
            db: DbRequestOptions {
                only_db: true,
                offline: false,
            },
        });
        let name = if dao { "ProbeDao.probe" } else { "probe" };
        assert!(!built.has_errors());
        assert_eq!(
            built
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            [format!(
                "SQLx query `{name}` reads column `id` of type `TEXT` into `int`, which cannot hold it"
            )],
        );
        assert_eq!(check(root, true).diagnostics, vec![]);
    }
}

#[test]
fn db_discovery_preserves_partless_annotations_owned_by_inactive_plugins() {
    let workspace = make_workspace();
    write_db_workspace(workspace.path(), false);
    write_file(
        &workspace.path().join("lib/page.dart"),
        "import 'package:dust_flutter/route.dart';\n@AppRoute('/home', name: 'home')\nclass HomePage {}\n",
    );
    assert_eq!(check(workspace.path(), false).diagnostics, vec![]);
}

#[test]
fn changing_a_cached_query_revalidates_sql_and_does_not_rewrite_metadata() {
    for dao in [false, true] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_db_workspace(root, false);
        source::write_probe(root, dao, "SELECT COUNT(*) FROM users", false);
        let built = run_build(BuildRequest {
            sdk_source_root: Some(crate::support::sdk_source_root(root)),
            cwd: root.to_path_buf(),
            fail_fast: true,
            jobs: None,
            db: DbRequestOptions {
                only_db: true,
                offline: false,
            },
        });
        assert_eq!(built.diagnostics, vec![]);
        let before = source::query_cache(root);
        source::write_probe(root, dao, "SELECT missing_column FROM users", false);
        let name = if dao { "ProbeDao.probe" } else { "probe" };
        assert_eq!(
            check(root, false)
                .diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>(),
            [format!(
                "SQLx rejected `{name}`: error returned from database: (code: 1) no such column: missing_column"
            )],
        );
        assert_eq!(
            check(root, true)
                .diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect::<Vec<_>>(),
            [format!(
                "Database offline query metadata cache is missing entry for `{name}`; run `dust db build` online first"
            )],
        );
        assert_eq!(source::query_cache(root), before);
    }
}

fn check(root: &Path, offline: bool) -> dust_driver::CommandResult {
    run_check(CheckRequest {
        sdk_source_root: Some(crate::support::sdk_source_root(root)),
        cwd: root.to_path_buf(),
        fail_fast: false,
        jobs: None,
        db: DbRequestOptions {
            only_db: true,
            offline,
        },
    })
}
