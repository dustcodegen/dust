use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

const TRADITIONAL: [&str; 2] = [
    include_str!("../../../../examples/product_showcase/lib/models/generic_models.dart"),
    include_str!("../../../../examples/product_showcase/lib/models/generic_imported.dart"),
];
const PRIMARY: [&str; 2] = [
    include_str!("../../../../fixtures/primary_sdk_consumer/lib/generic_models.dart"),
    include_str!("../../../../fixtures/primary_sdk_consumer/lib/generic_imported.dart"),
];
const EXPECTED: [&str; 2] = [
    include_str!("../../../../examples/product_showcase/lib/models/generic_models.g.dart"),
    include_str!("../../../../examples/product_showcase/lib/models/generic_imported.g.dart"),
];

#[test]
fn traditional_and_primary_generic_copies_match_complete_generated_contracts() {
    for (version, sources) in [("3.6.0", TRADITIONAL), ("3.13.0", PRIMARY)] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_file(
            &root.join("pubspec.yaml"),
            &format!("name: dust_test\nenvironment:\n  sdk: '>={version} <4.0.0'\n"),
        );
        for (name, source) in ["generic_models", "generic_imported"]
            .into_iter()
            .zip(sources)
        {
            write_file(&root.join(format!("lib/{name}.dart")), source);
        }
        let request = BuildRequest {
            sdk_source_root: Some(crate::support::sdk_source_root(root)),
            cwd: root.to_path_buf(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        };
        let built = run_build(request.clone());
        assert_eq!(built.diagnostics, []);
        assert_eq!(built.build_artifacts.len(), 2);
        for (name, expected) in ["generic_models", "generic_imported"]
            .into_iter()
            .zip(EXPECTED)
        {
            assert_eq!(
                fs::read_to_string(root.join(format!("lib/{name}.g.dart"))).unwrap(),
                expected,
                "{version}/{name}"
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
        assert!(
            checked
                .checked_libraries
                .iter()
                .all(|library| !library.stale)
        );
        let cached = run_build(request);
        assert_eq!(cached.diagnostics, []);
        assert_eq!(cached.build_artifacts.len(), 2);
        assert!(
            cached
                .build_artifacts
                .iter()
                .all(|artifact| artifact.cached)
        );
    }
}
