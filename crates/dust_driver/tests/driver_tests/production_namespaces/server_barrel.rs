//! Real public runtime exports, with no default server registration or network IO.

use super::*;

/// Actual runtime trees shared with the source-loader ownership unit contract.
#[path = "../../../../../fixtures/namespace_server_barrel/workspace.rs"]
mod public_fixture;

/// Replace only owned fixture dependencies with the actual runtime source trees.
fn public_workspace() -> tempfile::TempDir {
    let workspace = super::super::support::make_workspace();
    public_fixture::prepare(workspace.path());
    workspace
}

/// Use the shared hermetic SDK source map, never the machine's selected SDK.
fn build(root: &Path) -> dust_driver::CommandResult {
    run_build(BuildRequest {
        cwd: root.into(),
        sdk_source_root: Some(root.join(".dust-test-sdk")),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    })
}

#[test]
fn public_server_annotations_never_become_client_configs_or_change_derive_output() {
    let workspace = public_workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    write_file(
        &model,
        &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let expected = fs::read(root.join("lib/model.g.dart")).unwrap();
    write_file(
        &model,
        include_str!("../../../../../fixtures/namespace_server_barrel/model.dart.snapshot"),
    );
    assert_eq!(build(root).diagnostics, []);
    assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), expected);
    assert_eq!(build(root).cache.unwrap().hits, 1);
    assert_eq!(
        run_check(CheckRequest {
            cwd: root.into(),
            sdk_source_root: Some(root.join(".dust-test-sdk")),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        })
        .diagnostics,
        []
    );
}

#[test]
fn actual_server_barrel_and_definition_edits_preserve_success_and_recover() {
    for relative in ["server.dart", "src/annotations/http_methods.dart"] {
        let workspace = public_workspace();
        let root = workspace.path();
        write_file(
            &root.join("lib/model.dart"),
            include_str!("../../../../../fixtures/namespace_server_barrel/model.dart.snapshot"),
        );
        assert_eq!(build(root).diagnostics, []);
        let output_path = root.join("lib/model.g.dart");
        let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
        let output = fs::read(&output_path).unwrap();
        let cache = fs::read(&cache_path).unwrap();
        let dependency = root.join("deps/dust_server/lib").join(relative);
        let original = fs::read_to_string(&dependency).unwrap();
        write_file(&dependency, "library empty;\n");
        for result in [
            build(root),
            run_check(CheckRequest {
                cwd: root.into(),
                sdk_source_root: Some(root.join(".dust-test-sdk")),
                fail_fast: false,
                jobs: None,
                db: Default::default(),
            }),
        ] {
            assert!(result.has_errors());
            assert_eq!(
                result
                    .diagnostics
                    .iter()
                    .map(|error| error.message.as_str())
                    .collect::<Vec<_>>(),
                if relative == "server.dart" {
                    vec![
                        "annotation name `GET` is not visible in this namespace",
                        "annotation name `Path` is not visible in this namespace",
                        "annotation name `Query` is not visible in this namespace",
                    ]
                } else {
                    vec!["annotation name `GET` is not visible in this namespace"]
                }
            );
            assert_eq!(fs::read(&output_path).unwrap(), output);
            assert_eq!(fs::read(&cache_path).unwrap(), cache);
        }
        write_file(&dependency, &original);
        assert_eq!(build(root).cache.unwrap().hits, 1);
        assert_eq!(fs::read(&output_path).unwrap(), output);
    }
}
