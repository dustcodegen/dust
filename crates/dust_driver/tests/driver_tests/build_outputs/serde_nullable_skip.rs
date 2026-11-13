use std::fs;

use dust_driver::{BuildRequest, run_build};

use crate::support::{make_workspace, write_file};

#[test]
fn nullable_skipped_fields_match_traditional_and_primary_snapshots() {
    for (source, expected) in [
        (
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/serde_json_app/lib/src/skipped_nullable.dart"
            )),
            include_str!("serde_nullable_skip.dart.snapshot"),
        ),
        (
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/primary_sdk_consumer/lib/skipped_nullable.dart"
            )),
            include_str!("primary_nullable_skip.dart.snapshot"),
        ),
    ] {
        let workspace = make_workspace();
        write_file(&workspace.path().join("lib/skipped_nullable.dart"), source);
        let result = run_build(BuildRequest {
            sdk_source_root: Some(crate::support::sdk_source_root(workspace.path())),
            cwd: workspace.path().to_path_buf(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        });
        assert!(!result.has_errors(), "{:?}", result.diagnostics);
        assert_eq!(
            fs::read_to_string(workspace.path().join("lib/skipped_nullable.g.dart")).unwrap(),
            expected
        );
    }
}
