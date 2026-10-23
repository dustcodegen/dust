//! Real public runtime exports with Phase-0 ownership and no network IO.

use super::*;

/// Actual runtime trees shared with the source-loader ownership unit contract.
#[path = "../../../../../fixtures/namespace_server_barrel/workspace.rs"]
mod public_fixture;

const UNSUPPORTED: &str =
    "Dust server annotations are recognized, but handler generation is not available";

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
fn server_annotations_fail_clearly_without_replacing_client_output_or_cache() {
    let workspace = public_workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    write_file(
        &model,
        &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let expected = fs::read(root.join("lib/model.g.dart")).unwrap();
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let cache = fs::read(&cache_path).unwrap();
    write_file(
        &model,
        include_str!("../../../../../fixtures/namespace_server_barrel/model.dart.snapshot"),
    );
    let first = build(root);
    assert_eq!(
        first
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        [UNSUPPORTED]
    );
    assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), expected);
    assert_eq!(fs::read(&cache_path).unwrap(), cache);
    let checked = run_check(CheckRequest {
        cwd: root.into(),
        sdk_source_root: Some(root.join(".dust-test-sdk")),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });
    assert_eq!(
        checked
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        [UNSUPPORTED]
    );
    assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), expected);
    assert_eq!(fs::read(&cache_path).unwrap(), cache);

    write_file(
        &model,
        &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).cache.unwrap().hits, 1);
    assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), expected);
}

#[test]
fn server_dependency_edits_preserve_success_and_recover_ownership_diagnostics() {
    for relative in ["server.dart", "src/annotations/http_methods.dart"] {
        let workspace = public_workspace();
        let root = workspace.path();
        let model = root.join("lib/model.dart");
        write_file(
            &model,
            &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
        );
        assert_eq!(build(root).diagnostics, []);
        let output_path = root.join("lib/model.g.dart");
        let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
        let output = fs::read(&output_path).unwrap();
        let cache = fs::read(&cache_path).unwrap();
        write_file(
            &model,
            include_str!("../../../../../fixtures/namespace_server_barrel/model.dart.snapshot"),
        );
        assert_eq!(
            build(root)
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            [UNSUPPORTED]
        );
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
        assert_eq!(
            build(root)
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            [UNSUPPORTED]
        );
        assert_eq!(fs::read(&output_path).unwrap(), output);
        assert_eq!(fs::read(&cache_path).unwrap(), cache);
    }
}

#[test]
fn library_and_top_level_server_annotations_are_discovered() {
    for source in [
        "@Routes('/todos')\nlibrary todos;\nimport 'package:dust_server/server.dart';\n",
        "import 'package:dust_server/server.dart';\n@GET('/todos')\nString listTodos() => 'ok';\n",
    ] {
        let workspace = public_workspace();
        write_file(&workspace.path().join("lib/model.dart"), source);
        let result = build(workspace.path());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            [UNSUPPORTED],
            "source: {source}"
        );
        assert!(!workspace.path().join("lib/model.g.dart").exists());
    }
}
