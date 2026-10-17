use dust_driver::{BuildRequest, DbRequestOptions, run_build};

use super::{check, source};
use crate::{db_outputs::helpers::write_db_workspace, support::make_workspace};

#[test]
fn escaped_sql_has_identical_online_and_offline_meaning_for_direct_and_dao_calls() {
    for style in ["direct", "positional", "named"] {
        for literal in [r"'\u0053ELECT \x31'", "''' \nSELECT 1'''", "'SELECT 1'"] {
            let workspace = make_workspace();
            let root = workspace.path();
            write_db_workspace(root, false);
            write_query(root, style, literal);
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
            assert_eq!(built.diagnostics, vec![], "{style}: {literal}");
            let cache = source::query_cache(root);
            let json: serde_json::Value = serde_json::from_str(&cache).unwrap();
            assert_eq!(json["entries"][0]["sql"], "SELECT 1");
            assert_eq!(check(root, true).diagnostics, vec![]);
            assert_eq!(source::query_cache(root), cache);

            for unsupported in [r"'\uD800'", r"'$table'", "'SELECT ' '1'"] {
                write_query(root, style, unsupported);
                let expected = if style == "direct" {
                    "Database query SQL must be a static string literal"
                } else {
                    "SqlxDao query method `ProbeDao.probe` SQL must be a static raw string literal"
                };
                for offline in [false, true] {
                    let mut messages = vec![expected];
                    if style != "direct" {
                        messages.push("Database query SQL must be a static string literal");
                    }
                    let name = if style == "direct" {
                        "probe"
                    } else {
                        "ProbeDao.probe"
                    };
                    let missing = format!(
                        "Database offline query metadata cache is missing entry for `{name}`; run `dust db build` online first"
                    );
                    if offline {
                        messages.push(&missing);
                    }
                    let result = check(root, offline);
                    assert_eq!(
                        result
                            .diagnostics
                            .iter()
                            .map(|d| d.message.as_str())
                            .collect::<Vec<_>>(),
                        messages,
                        "{style}: {unsupported}"
                    );
                }
                assert_eq!(source::query_cache(root), cache);
            }
        }
    }
}

/// The same literal must travel through direct, positional and named SQL paths.
fn write_query(root: &std::path::Path, style: &str, literal: &str) {
    let body = match style {
        "direct" => format!(
            "Future<Result<int, SqlxError>> probe(Executor db) => queryScalar<int>({literal}, []).fetchOne(db);"
        ),
        "named" => format!("@Query(sql: {literal}) Future<Result<int, SqlxError>> probe();"),
        _ => format!("@Query({literal}) Future<Result<int, SqlxError>> probe();"),
    };
    source::write_source(root, style != "direct", &body);
}
