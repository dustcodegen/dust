use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

#[test]
fn generic_name_collisions_preserve_complete_output_for_both_header_forms() {
    let expected = include_str!("../../../../fixtures/generic_names/generated.dart.snapshot");
    for (version, source) in [
        (
            "3.6.0",
            include_str!("../../../../fixtures/generic_names/traditional.dart.snapshot"),
        ),
        (
            "3.13.0",
            include_str!("../../../../fixtures/generic_names/primary.dart.snapshot"),
        ),
    ] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_file(
            &root.join("pubspec.yaml"),
            &format!("name: dust_test\nenvironment:\n  sdk: '>={version} <4.0.0'\n"),
        );
        write_file(&root.join("lib/generic_names.dart"), source);
        let request = BuildRequest {
            sdk_source_root: Some(crate::support::sdk_source_root(root)),
            cwd: root.to_path_buf(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        };
        let built = run_build(request.clone());
        assert_eq!(built.diagnostics, []);
        assert_eq!(built.build_artifacts.len(), 1);
        assert_eq!(
            fs::read_to_string(root.join("lib/generic_names.g.dart")).unwrap(),
            expected,
            "{version}"
        );
        let checked = run_check(CheckRequest {
            sdk_source_root: Some(crate::support::sdk_source_root(root)),
            cwd: root.to_path_buf(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        });
        assert_eq!(checked.diagnostics, []);
        assert_eq!(checked.checked_libraries.len(), 1);
        assert!(!checked.checked_libraries[0].stale);
        let cached = run_build(request);
        assert_eq!(cached.diagnostics, []);
        assert_eq!(cached.build_artifacts.len(), 1);
        assert!(cached.build_artifacts[0].cached);
    }
}
