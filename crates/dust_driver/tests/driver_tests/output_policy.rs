use dust_driver::{BuildRequest, run_build};

use super::support::{DustImport, make_workspace, write_dust_file, write_file};

#[test]
fn build_uses_configured_primary_suffix() {
    let workspace = make_workspace();
    write_file(
        &workspace.path().join("dust.yaml"),
        "outputs:\n  primary_suffix: .d.dart\n",
    );
    write_dust_file(
        &workspace.path().join("lib/user.dart"),
        &[DustImport::Derive],
        "part 'user.d.dart';\n\
         @ToString()\n\
         class User {\n\
           final String id;\n\
           const User(this.id);\n\
         }\n",
    );

    let result = run_build(BuildRequest {
        sdk_source_root: Some(crate::support::sdk_source_root(workspace.path())),
        cwd: workspace.path().to_path_buf(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });

    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert!(workspace.path().join("lib/user.d.dart").exists());
    assert!(!workspace.path().join("lib/user.g.dart").exists());
}

#[test]
fn build_reports_part_suffix_mismatches_from_dust_config() {
    let workspace = make_workspace();
    write_file(
        &workspace.path().join("dust.yaml"),
        "outputs:\n  primary_suffix: .d.dart\n",
    );
    write_dust_file(
        &workspace.path().join("lib/user.dart"),
        &[DustImport::Derive],
        "part 'user.g.dart';\n\
         @ToString()\n\
         class User {\n\
           final String id;\n\
           const User(this.id);\n\
         }\n",
    );

    let result = run_build(BuildRequest {
        sdk_source_root: Some(crate::support::sdk_source_root(workspace.path())),
        cwd: workspace.path().to_path_buf(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });

    assert!(result.has_errors());
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.message.contains("cannot read namespace source")
            && diagnostic.message.contains("user.g.dart")
            && diagnostic.notes == ["Dust's configured primary output for this library is `user.d.dart`; other parts must already exist"]
    }), "{:?}", result.diagnostics);
    assert!(!workspace.path().join("lib/user.d.dart").exists());
    assert!(!workspace.path().join("lib/user.g.dart").exists());
}
