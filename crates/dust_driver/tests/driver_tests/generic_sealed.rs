use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

#[test]
fn sealed_codegen_has_complete_traditional_primary_and_package_kind_contracts() {
    let fixtures = [
        (
            "sealed_plain",
            [
                include_str!("../../../../fixtures/generic_sealed/plain_traditional.dart.snapshot"),
                include_str!("../../../../fixtures/generic_sealed/plain_primary.dart.snapshot"),
            ],
            include_str!(
                "../../../../fixtures/generic_sealed/sealed_plain_generated.dart.snapshot"
            ),
        ),
        (
            "sealed_declared",
            [
                include_str!(
                    "../../../../fixtures/generic_sealed/declared_traditional.dart.snapshot"
                ),
                include_str!("../../../../fixtures/generic_sealed/declared_primary.dart.snapshot"),
            ],
            include_str!(
                "../../../../fixtures/generic_sealed/sealed_declared_generated.dart.snapshot"
            ),
        ),
        (
            "sealed_generated",
            [
                include_str!(
                    "../../../../fixtures/generic_sealed/generated_traditional.dart.snapshot"
                ),
                include_str!("../../../../fixtures/generic_sealed/generated_primary.dart.snapshot"),
            ],
            include_str!(
                "../../../../fixtures/generic_sealed/sealed_generated_generated.dart.snapshot"
            ),
        ),
        (
            "sealed_modes",
            [
                include_str!("../../../../fixtures/generic_sealed/modes_traditional.dart.snapshot"),
                include_str!("../../../../fixtures/generic_sealed/modes_primary.dart.snapshot"),
            ],
            include_str!(
                "../../../../fixtures/generic_sealed/sealed_modes_generated.dart.snapshot"
            ),
        ),
        (
            "sealed_scope",
            [
                include_str!("../../../../fixtures/generic_sealed/scope_traditional.dart.snapshot"),
                include_str!("../../../../fixtures/generic_sealed/scope_primary.dart.snapshot"),
            ],
            include_str!(
                "../../../../fixtures/generic_sealed/sealed_scope_generated.dart.snapshot"
            ),
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
                cwd: root.to_path_buf(),
                fail_fast: false,
                jobs: None,
                db: Default::default(),
            });
            assert_eq!(checked.diagnostics, []);
            assert_eq!(checked.checked_libraries.len(), fixtures.len());
            assert!(
                checked
                    .checked_libraries
                    .iter()
                    .all(|library| !library.stale)
            );
            let warm = run_build(request);
            assert_eq!(warm.diagnostics, []);
            assert_eq!(warm.build_artifacts.len(), fixtures.len());
            assert!(warm.build_artifacts.iter().all(|artifact| artifact.cached));
        }
    }
}
