use std::{fs, path::Path};

use dust_driver::{BuildRequest, CheckRequest, DbRequestOptions, run_build, run_check};

use crate::support::{make_workspace, write_file};

fn write_primary_consumer(root: &Path) {
    write_file(
        &root.join("pubspec.yaml"),
        "name: primary_sdk_consumer\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
    );
    for (name, source) in [
        (
            "lib/private_models.dart",
            include_str!("../../../../../fixtures/primary_sdk_consumer/lib/private_models.dart"),
        ),
        (
            "lib/database.dart",
            include_str!("../../../../../fixtures/primary_sdk_consumer/lib/database.dart"),
        ),
        (
            "lib/checked_queries.dart",
            include_str!("../../../../../fixtures/primary_sdk_consumer/lib/checked_queries.dart"),
        ),
        (
            "migrations/0001_accounts.sql",
            include_str!(
                "../../../../../fixtures/primary_sdk_consumer/migrations/0001_accounts.sql"
            ),
        ),
    ] {
        write_file(&root.join(name), source);
    }
}

fn cache_bytes(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut files = fs::read_dir(root.join(".dust_sql"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

#[test]
fn primary_queries_validate_required_columns_and_preserve_outputs_on_error() {
    for dao in [false, true] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_primary_consumer(root);
        for only_db in [false, true] {
            let built = run_build(BuildRequest {
                sdk_source_root: Some(crate::support::sdk_source_root(root)),
                cwd: root.to_path_buf(),
                fail_fast: true,
                jobs: None,
                db: DbRequestOptions {
                    only_db,
                    offline: false,
                },
            });
            assert_eq!(built.diagnostics, []);
        }
        let cache = cache_bytes(root);
        assert_eq!(cache.len(), 2);
        let generated = root.join("lib/database.g.dart");
        let previous = fs::read(&generated).unwrap();
        let source = root.join(if dao {
            "lib/database.dart"
        } else {
            "lib/checked_queries.dart"
        });
        let original = fs::read_to_string(&source).unwrap();
        let names = if dao {
            [
                "PrimaryAccountDao.one",
                "PrimaryAccountDao.optional",
                "PrimaryAccountDao.all",
            ]
        } else {
            ["oneAccount", "optionalAccount", "allAccounts"]
        };
        assert_eq!(
            original.matches("SELECT display_name, note").count(),
            names.len()
        );
        for ((start, matched), name) in original
            .match_indices("SELECT display_name, note")
            .zip(names)
        {
            let mut changed = original.clone();
            changed.replace_range(start..start + matched.len(), "SELECT display_name");
            write_file(&source, &changed);
            for offline in [false, true] {
                let checked = run_check(CheckRequest {
                    sdk_source_root: Some(crate::support::sdk_source_root(root)),
                    cwd: root.to_path_buf(),
                    fail_fast: true,
                    jobs: None,
                    db: DbRequestOptions {
                        only_db: true,
                        offline,
                    },
                });
                let expected = if offline {
                    format!(
                        "Database offline query metadata cache is missing entry for `{name}`; run `dust db build` online first"
                    )
                } else {
                    format!(
                        "SQLx query `{name}` does not return required column `note` for row `PrivateAccount`"
                    )
                };
                assert_eq!(
                    checked
                        .diagnostics
                        .iter()
                        .map(|d| d.message.as_str())
                        .collect::<Vec<_>>(),
                    [expected.as_str()]
                );
                assert_eq!(cache_bytes(root), cache);
                assert_eq!(fs::read(&generated).unwrap(), previous);
            }
        }
    }
}
