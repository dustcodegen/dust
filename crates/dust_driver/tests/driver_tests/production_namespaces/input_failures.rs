use std::fs;

use dust_driver::{CheckRequest, run_check};

use super::{MODEL, build, workspace, write_file};

#[test]
fn required_source_and_uri_failures_preserve_success_without_short_name_fallback() {
    let workspace = workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/model.dart"),
        &format!("import 'package:bridge/bridge.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    let barrel = root.join("deps/bridge/lib/bridge.dart");
    write_file(&root.join("deps/bridge/lib/broken.dart"), "class {");
    for (source, expected) in [
        ("export 'broken.dart';", "cannot parse namespace source"),
        (
            "export 'https://example.invalid/missing.dart';",
            "unsupported local source URI scheme `https`",
        ),
        (
            "export 'bad%GG.dart';",
            "invalid percent escape in source URI",
        ),
        ("export 'absent.dart';", "cannot read namespace source"),
    ] {
        write_file(&barrel, source);
        for result in [
            build(root),
            run_check(CheckRequest {
                sdk_source_root: Some(root.join("sdk")),
                cwd: root.into(),
                fail_fast: false,
                jobs: None,
                db: Default::default(),
            }),
        ] {
            assert!(result.has_errors(), "{source}");
            assert!(!result.diagnostics.is_empty());
            assert!(
                result
                    .diagnostics
                    .iter()
                    .all(|error| error.message.contains(expected)),
                "{source}: {:?}",
                result.diagnostics
            );
            assert_eq!(fs::read(&output_path).unwrap(), output);
            assert_eq!(fs::read(&cache_path).unwrap(), cache);
        }
    }
}

#[test]
fn malformed_package_config_cannot_replace_successful_output_or_cache() {
    let workspace = workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/model.dart"),
        &format!("import 'package:bridge/bridge.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    write_file(&root.join(".dart_tool/package_config.json"), "[");
    for result in [
        build(root),
        run_check(CheckRequest {
            sdk_source_root: Some(root.join("sdk")),
            cwd: root.into(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        }),
    ] {
        assert!(result.has_errors());
        assert_eq!(result.diagnostics.len(), 1);
        assert!(
            result.diagnostics[0]
                .message
                .contains("invalid package configuration"),
            "{:?}",
            result.diagnostics
        );
        assert_eq!(fs::read(&output_path).unwrap(), output);
        assert_eq!(fs::read(&cache_path).unwrap(), cache);
    }
}
