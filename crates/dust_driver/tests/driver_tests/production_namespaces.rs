use std::{fs, path::Path};

use dust_driver::{
    BuildRequest, CheckRequest, RouteTableRequest, WatchRequest, run_build, run_check,
    run_route_table, run_watch,
};

use super::support::{make_workspace, write_file};

#[path = "production_namespaces/boundaries.rs"]
mod boundaries;
#[path = "production_namespaces/cache_watch.rs"]
mod cache_watch;
#[path = "production_namespaces/callable_defaults.rs"]
mod callable_defaults;
#[path = "production_namespaces/generated_libraries.rs"]
mod generated_libraries;
#[path = "production_namespaces/imports.rs"]
mod imports;
#[path = "production_namespaces/input_failures.rs"]
mod input_failures;
#[path = "production_namespaces/metadata_sites.rs"]
mod metadata_sites;
#[path = "production_namespaces/namespace_diagnostics.rs"]
mod namespace_diagnostics;
#[path = "production_namespaces/part_paths.rs"]
mod part_paths;
#[path = "production_namespaces/watch_analysis.rs"]
mod watch_analysis;

fn workspace() -> tempfile::TempDir {
    let workspace = make_workspace();
    let root = workspace.path();
    write_file(
        &root.join(".dart_tool/package_config.json"),
        r#"{
      "configVersion": 2,
      "packages": [
        {"name":"dust_test","rootUri":"../","packageUri":"lib/","languageVersion":"3.6"},
        {"name":"dust_dart","rootUri":"../deps/dust_dart","packageUri":"lib/","languageVersion":"3.6"},
        {"name":"bridge","rootUri":"../deps/bridge","packageUri":"lib/","languageVersion":"3.6"}
      ]
    }"#,
    );
    write_file(
        &root.join("deps/dust_dart/pubspec.yaml"),
        "name: dust_dart\nversion: 0.2.1\n",
    );
    write_file(
        &root.join("deps/dust_dart/lib/derive.dart"),
        "export 'src/derive/base.dart'; export 'src/derive/traits.dart';",
    );
    write_file(
        &root.join("deps/dust_dart/lib/src/derive/base.dart"),
        "class Derive { const Derive(List<Object> traits); }",
    );
    write_file(
        &root.join("deps/dust_dart/lib/src/derive/traits.dart"),
        "class CopyWith { const CopyWith(); } class ToString { const ToString(); }",
    );
    write_file(
        &root.join("deps/bridge/lib/bridge.dart"),
        "export 'package:dust_dart/derive.dart';",
    );
    write_file(&root.join("sdk/version"), "3.6.0\n");
    write_file(
        &root.join("sdk/lib/libraries.json"),
        r#"{"vm":{"libraries":{"core":{"uri":"core/core.dart"}}}}"#,
    );
    write_file(
        &root.join("sdk/lib/core/core.dart"),
        "library dart.core; class Object {} class String {}",
    );
    workspace
}

fn build(root: &Path) -> dust_driver::CommandResult {
    run_build(BuildRequest {
        sdk_source_root: Some(root.join("sdk")),
        cwd: root.to_path_buf(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    })
}

const MODEL: &str = r#"part 'model.g.dart';
@Derive([ToString()])
class Model with _$Model { const Model(this.value); final int value; }
"#;

#[test]
fn third_party_barrel_is_discovered_and_preserves_direct_output() {
    let workspace = workspace();
    let root = workspace.path();
    let file = root.join("lib/model.dart");
    write_file(
        &file,
        &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let output = fs::read_to_string(root.join("lib/model.g.dart")).unwrap();
    fs::remove_file(root.join("lib/model.g.dart")).unwrap();
    write_file(
        &file,
        &format!("import 'package:bridge/bridge.dart';\n{MODEL}"),
    );
    let result = build(root);
    assert_eq!(result.diagnostics, []);
    assert_eq!(result.build_artifacts.len(), 1);
    assert_eq!(
        fs::read_to_string(root.join("lib/model.g.dart")).unwrap(),
        output
    );
}

#[test]
fn local_foreign_trait_is_not_hijacked_by_short_name() {
    let workspace = workspace();
    let root = workspace.path();
    let file = root.join("lib/model.dart");
    write_file(
        &file,
        &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let output = fs::read_to_string(root.join("lib/model.g.dart")).unwrap();
    let foreign = MODEL.replace("[ToString()]", "[CopyWith(), ToString()]");
    write_file(
        &file,
        &format!(
            "import 'package:dust_dart/derive.dart';\n{foreign}\nclass CopyWith {{ const CopyWith(); }}"
        ),
    );
    let result = build(root);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        ["unknown derive trait or config `CopyWith`"]
    );
    assert!(!result.has_errors());
    assert_eq!(
        fs::read_to_string(root.join("lib/model.g.dart")).unwrap(),
        output
    );
}

#[test]
fn explicit_invalid_sdk_fails_in_each_codegen_request_without_mutating_success() {
    let workspace = workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/model.dart"),
        &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    let invalid = || Some("missing-sdk".into());
    let results = [
        run_build(BuildRequest {
            sdk_source_root: invalid(),
            cwd: root.into(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        }),
        run_check(CheckRequest {
            sdk_source_root: invalid(),
            cwd: root.into(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        }),
        run_watch(WatchRequest {
            sdk_source_root: invalid(),
            cwd: root.into(),
            fail_fast: false,
            jobs: None,
            poll_interval_ms: 0,
            max_cycles: Some(0),
        }),
        run_route_table(RouteTableRequest {
            sdk_source_root: invalid(),
            cwd: root.into(),
        }),
    ];
    assert_eq!(fs::read(output_path).unwrap(), output);
    assert_eq!(fs::read(cache_path).unwrap(), cache);
    assert_eq!(
        results.each_ref().map(|result| result.has_errors()),
        [true; 4]
    );
    for result in results {
        assert!(
            result.diagnostics.iter().any(|diagnostic| {
                diagnostic.message.contains("SDK") && diagnostic.message.contains("missing-sdk")
            }),
            "{:?}",
            result.diagnostics
        );
    }
}
