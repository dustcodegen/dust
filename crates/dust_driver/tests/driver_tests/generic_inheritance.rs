use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

#[test]
fn inherited_codegen_has_complete_traditional_primary_and_package_kind_contracts() {
    let cases = [
        (
            "inheritance_concrete",
            [
                include_str!(
                    "../../../../fixtures/generic_inheritance/concrete_traditional.dart.snapshot"
                ),
                include_str!(
                    "../../../../fixtures/generic_inheritance/concrete_primary.dart.snapshot"
                ),
            ],
            include_str!(
                "../../../../fixtures/generic_inheritance/inheritance_concrete_generated.dart.snapshot"
            ),
            Some(include_str!(
                "../../../../fixtures/generic_inheritance/inheritance_concrete_flutter_generated.dart.snapshot"
            )),
        ),
        (
            "inheritance_nested",
            [
                include_str!(
                    "../../../../fixtures/generic_inheritance/nested_traditional.dart.snapshot"
                ),
                include_str!(
                    "../../../../fixtures/generic_inheritance/nested_primary.dart.snapshot"
                ),
            ],
            include_str!(
                "../../../../fixtures/generic_inheritance/inheritance_nested_generated.dart.snapshot"
            ),
            None,
        ),
        (
            "inheritance_generic",
            [
                include_str!(
                    "../../../../fixtures/generic_inheritance/generic_traditional.dart.snapshot"
                ),
                include_str!(
                    "../../../../fixtures/generic_inheritance/generic_primary.dart.snapshot"
                ),
            ],
            include_str!(
                "../../../../fixtures/generic_inheritance/inheritance_generic_generated.dart.snapshot"
            ),
            None,
        ),
        (
            "inheritance_override",
            [
                include_str!(
                    "../../../../fixtures/generic_inheritance/override_traditional.dart.snapshot"
                ),
                include_str!(
                    "../../../../fixtures/generic_inheritance/override_primary.dart.snapshot"
                ),
            ],
            include_str!(
                "../../../../fixtures/generic_inheritance/inheritance_override_generated.dart.snapshot"
            ),
            None,
        ),
    ];
    for flutter in [false, true] {
        for (index, version) in ["3.6.0", "3.13.0"].into_iter().enumerate() {
            let workspace = make_workspace();
            let root = workspace.path();
            let dependencies = if flutter {
                "dependencies:\n  flutter:\n    sdk: flutter\n"
            } else {
                ""
            };
            write_file(
                &root.join("pubspec.yaml"),
                &format!(
                    "name: dust_test\nenvironment:\n  sdk: '>={version} <4.0.0'\n{dependencies}"
                ),
            );
            for (stem, sources, _, _) in cases {
                write_file(&root.join(format!("lib/{stem}.dart")), sources[index]);
            }
            let request = BuildRequest {
                sdk_source_root: Some(crate::support::sdk_source_root(root)),
                cwd: root.to_path_buf(),
                fail_fast: false,
                jobs: None,
                db: Default::default(),
            };
            assert_eq!(run_build(request.clone()).diagnostics, []);
            for (stem, _, expected, flutter_expected) in cases {
                let expected = if flutter {
                    flutter_expected.unwrap_or(expected)
                } else {
                    expected
                };
                assert_eq!(
                    fs::read_to_string(root.join(format!("lib/{stem}.g.dart"))).unwrap(),
                    expected
                );
            }
            let checked = run_check(CheckRequest {
                sdk_source_root: Some(crate::support::sdk_source_root(root)),
                cwd: root.to_path_buf(),
                fail_fast: false,
                jobs: None,
                db: Default::default(),
            });
            assert_eq!(checked.diagnostics, []);
            assert_eq!(checked.checked_libraries.len(), cases.len());
            assert!(
                checked
                    .checked_libraries
                    .iter()
                    .all(|library| !library.stale)
            );
            let warm = run_build(request);
            assert_eq!(warm.diagnostics, []);
            assert_eq!(warm.build_artifacts.len(), cases.len());
            assert!(warm.build_artifacts.iter().all(|artifact| artifact.cached));
        }
    }
}
