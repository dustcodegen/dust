use std::fs;

use dust_driver::{CheckRequest, run_check};

use super::{MODEL, build, workspace, write_file};

#[test]
fn dependency_body_limit_does_not_change_generation_or_warm_check() {
    let workspace = workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/model.dart"),
        &format!("import 'package:bridge/bridge.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let output = fs::read(&output_path).unwrap();
    for declaration in [
        "Kind choose() => .ready;",
        "class AnimationState { Kind direction = .ready; }",
        "class AnimationState { final direction = choose(.ready); }",
        "class AnimationState { final callback = () => .ready; }",
    ] {
        write_file(
            &root.join("deps/bridge/lib/bridge.dart"),
            &format!(
                "// @dart=3.13\nexport 'package:dust_dart/derive.dart';\nenum Kind {{ ready }}\n{declaration}"
            ),
        );
        let rebuilt = build(root);
        assert_eq!(rebuilt.diagnostics, [], "{declaration}");
        assert_eq!(fs::read(&output_path).unwrap(), output);
        assert_eq!(check(root).diagnostics, []);
        assert_eq!(build(root).cache.unwrap().hits, 1);
    }
}

#[test]
fn namespace_projection_cannot_hide_invalid_interfaces_or_root_generation_errors() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let dependency = root.join("deps/bridge/lib/bridge.dart");
    let initial = format!("import 'package:bridge/bridge.dart';\n{MODEL}");
    write_file(&model, &initial);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    for (version, declaration, expected) in [
        ("3.6", "Kind choose() => .ready;", "require Dart 3.10"),
        (
            "3.6",
            "class C { Kind value = .ready; }",
            "require Dart 3.10",
        ),
        (
            "3.13",
            "class C { static Kind value = .ready; }",
            "does not support dot shorthands",
        ),
        (
            "3.13",
            "class C { @Marker(.ready) Kind value = Kind.ready; }",
            "does not support dot shorthands",
        ),
        (
            "3.13",
            "class C { final callback = ([Kind value = .ready]) => value; }",
            "does not support dot shorthands",
        ),
        (
            "3.13",
            "class C { Kind value = .ready(,); }",
            "syntax error",
        ),
        (
            "3.13",
            "Kind choose([Kind value = .ready]) => value;",
            "does not support dot shorthands",
        ),
        (
            "3.13",
            "final Kind value = .ready;",
            "does not support dot shorthands",
        ),
        (
            "3.13",
            "Kind choose() { return .ready; broken( ; }",
            "syntax error",
        ),
        (
            "3.13",
            "class C { new(); }",
            "does not support concise constructors",
        ),
    ] {
        write_file(
            &dependency,
            &format!(
                "// @dart={version}\nexport 'package:dust_dart/derive.dart';\nenum Kind {{ ready }}\n{declaration}"
            ),
        );
        for result in [build(root), check(root)] {
            assert!(result.has_errors(), "{declaration}");
            assert!(
                result
                    .diagnostics
                    .iter()
                    .any(|error| error.message.contains(expected)),
                "{:?}",
                result.diagnostics
            );
            assert_eq!(fs::read(&output_path).unwrap(), output);
            assert_eq!(fs::read(&cache_path).unwrap(), cache);
        }
    }
    write_file(&dependency, "export 'package:dust_dart/derive.dart';");
    for declaration in [
        "Kind choose() => .ready;",
        "class C { Kind value = .ready; }",
    ] {
        write_file(
            &model,
            &format!("// @dart=3.13\n{initial}\nenum Kind {{ ready }}\n{declaration}"),
        );
        for result in [build(root), check(root)] {
            assert!(result.has_errors());
            assert_eq!(result.diagnostics.len(), 1);
            assert_eq!(
                result.diagnostics[0].message,
                "Dust does not support dot shorthands yet"
            );
            assert_eq!(fs::read(&output_path).unwrap(), output);
            assert_eq!(fs::read(&cache_path).unwrap(), cache);
        }
    }
}

fn check(root: &std::path::Path) -> dust_driver::CommandResult {
    run_check(CheckRequest {
        sdk_source_root: Some(root.join("sdk")),
        cwd: root.into(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    })
}
