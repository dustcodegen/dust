//! Invalid collection shapes must report validation errors without entering emission.

use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

fn model(primary: bool, ty: &str) -> String {
    let class = if primary {
        format!("class const Captured(final {ty} value) with _$Captured;")
    } else {
        format!(
            "class Captured with _$Captured {{ const Captured(this.value); final {ty} value; }}"
        )
    };
    format!(
        "import 'package:dust_dart/derive.dart';\nimport 'package:dust_dart/serde.dart';\npart 'model.g.dart';\n@Derive([Serialize(), Deserialize()])\n{class}\n"
    )
}

#[test]
fn invalid_collections_report_errors_without_replacing_output_or_cache() {
    for primary in [false, true] {
        for fail_fast in [false, true] {
            for existing in [false, true] {
                let workspace = make_workspace();
                let root = workspace.path();
                write_file(
                    &root.join("pubspec.yaml"),
                    "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
                );
                let path = root.join("lib/model.dart");
                let output = root.join("lib/model.g.dart");
                let cache = root.join(".dart_tool/dust/build_cache_v1.json");
                let request = BuildRequest {
                    sdk_source_root: Some(crate::support::sdk_source_root(root)),
                    cwd: root.to_path_buf(),
                    fail_fast,
                    jobs: None,
                    db: Default::default(),
                };
                if existing {
                    write_file(&path, &model(primary, "int"));
                    assert_eq!(run_build(request.clone()).diagnostics, []);
                    let warm = run_build(request.clone());
                    assert_eq!(warm.diagnostics, []);
                    assert!(warm.build_artifacts[0].cached);
                }
                let before_output = fs::read(&output).ok();
                let before_cache = fs::read(&cache).ok();
                for (ty, requirement) in [
                    ("List", "one type argument for `List`"),
                    ("Set", "one type argument for `Set`"),
                    ("Map", "two type arguments for `Map`"),
                ] {
                    write_file(&path, &model(primary, ty));
                    for writing in [false, true] {
                        let result = if writing {
                            run_build(request.clone())
                        } else {
                            run_check(CheckRequest {
                                sdk_source_root: Some(crate::support::sdk_source_root(root)),
                                cwd: root.to_path_buf(),
                                fail_fast,
                                jobs: None,
                                db: Default::default(),
                            })
                        };
                        assert_eq!(
                            result
                                .diagnostics
                                .iter()
                                .map(|d| d.message.clone())
                                .collect::<Vec<_>>(),
                            ["Serialize", "Deserialize"].map(|direction| format!(
                                "`{direction}` requires {requirement} on `Captured.value`"
                            ))
                        );
                        assert!(result.diagnostics.iter().all(|d| d.is_error()));
                        assert_eq!(fs::read(&output).ok(), before_output);
                        assert_eq!(fs::read(&cache).ok(), before_cache);
                    }
                }
            }
        }
    }
}
