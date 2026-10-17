use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

#[test]
fn generic_serde_has_exact_typed_parity_for_both_headers_and_package_kinds() {
    let fixtures = [
        (
            "serde_basic",
            [
                include_str!("../../../../fixtures/generic_serde/traditional.dart.snapshot"),
                include_str!("../../../../fixtures/generic_serde/primary.dart.snapshot"),
            ],
            include_str!("../../../../fixtures/generic_serde/serde_basic_generated.dart.snapshot"),
        ),
        (
            "serde_values",
            [
                include_str!("../../../../fixtures/generic_serde/values_traditional.dart.snapshot"),
                include_str!("../../../../fixtures/generic_serde/values_primary.dart.snapshot"),
            ],
            include_str!("../../../../fixtures/generic_serde/serde_values_generated.dart.snapshot"),
        ),
        (
            "serde_scope",
            [
                include_str!("../../../../fixtures/generic_serde/scope_traditional.dart.snapshot"),
                include_str!("../../../../fixtures/generic_serde/scope_primary.dart.snapshot"),
            ],
            include_str!("../../../../fixtures/generic_serde/serde_scope_generated.dart.snapshot"),
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
            for (stem, sources, _) in fixtures {
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
            for (stem, _, expected) in fixtures {
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
            assert_eq!(checked.checked_libraries.len(), 3);
            assert!(
                checked
                    .checked_libraries
                    .iter()
                    .all(|library| !library.stale)
            );
            let warm = run_build(request);
            assert_eq!(warm.diagnostics, []);
            assert_eq!(warm.build_artifacts.len(), 3);
            assert!(warm.build_artifacts.iter().all(|artifact| artifact.cached));
        }
    }
}
