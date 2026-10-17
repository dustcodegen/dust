use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

#[test]
fn generic_validation_has_the_same_typed_contract_for_both_headers() {
    let expected = include_str!("../../../../fixtures/generic_validation/generated.dart.snapshot");
    for (version, source) in [
        (
            "3.6.0",
            include_str!("../../../../fixtures/generic_validation/traditional.dart.snapshot"),
        ),
        (
            "3.13.0",
            include_str!("../../../../fixtures/generic_validation/primary.dart.snapshot"),
        ),
    ] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_file(
            &root.join("pubspec.yaml"),
            &format!("name: dust_test\nenvironment:\n  sdk: '>={version} <4.0.0'\n"),
        );
        write_file(&root.join("lib/model.dart"), source);
        let request = BuildRequest {
            cwd: root.to_path_buf(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        };
        let built = run_build(request.clone());
        assert_eq!(built.diagnostics, []);
        assert_eq!(built.build_artifacts.len(), 1);
        assert_eq!(
            fs::read_to_string(root.join("lib/model.g.dart")).unwrap(),
            expected
        );
        let checked = run_check(CheckRequest {
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
        assert!(cached.build_artifacts[0].cached);
    }
}

#[test]
fn generic_validation_shapes_retain_exact_parity_with_and_without_flutter() {
    let fixtures = [
        (
            "shapes",
            [
                include_str!(
                    "../../../../fixtures/generic_validation/shapes_traditional.dart.snapshot"
                ),
                include_str!(
                    "../../../../fixtures/generic_validation/shapes_primary.dart.snapshot"
                ),
            ],
        ),
        (
            "scope",
            [
                include_str!(
                    "../../../../fixtures/generic_validation/scope_traditional.dart.snapshot"
                ),
                include_str!("../../../../fixtures/generic_validation/scope_primary.dart.snapshot"),
            ],
        ),
    ];
    for flutter in [false, true] {
        let mut expected = None;
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
            for (name, sources) in fixtures {
                write_file(
                    &root.join(format!("lib/validation_{name}.dart")),
                    sources[index],
                );
            }
            let result = run_build(BuildRequest {
                cwd: root.to_path_buf(),
                fail_fast: false,
                jobs: None,
                db: Default::default(),
            });
            assert_eq!(result.diagnostics, []);
            assert_eq!(result.build_artifacts.len(), 2);
            let generated: Vec<_> = fixtures
                .iter()
                .map(|(name, _)| {
                    fs::read_to_string(root.join(format!("lib/validation_{name}.g.dart"))).unwrap()
                })
                .collect();
            match &expected {
                Some(expected) => assert_eq!(&generated, expected),
                None => expected = Some(generated),
            }
        }
    }
}

#[test]
fn generic_binders_named_like_builtins_do_not_enable_builtin_rules() {
    for (binder, config, message) in [
        (
            "String",
            "email: true",
            "`@Validate` string validators on `value` require `String` or `String?`",
        ),
        (
            "List",
            "length: Length(min: 2)",
            "`@Validate(length: ...)` on `value` requires String, List, Set, or Map",
        ),
        (
            "int",
            "range: Range(min: 1)",
            "`@Validate(range: ...)` on `value` requires int, double, or num",
        ),
    ] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_file(
            &root.join("lib/model.dart"),
            &format!(
                r#"
import 'package:dust_dart/derive.dart';
part 'model.g.dart';
@Derive([Validate()])
class Box<{binder}> with _$Box<{binder}> {{
  const Box(this.value);
  @Validate({config})
  final {binder}? value;
}}
"#
            ),
        );
        let result = run_build(BuildRequest {
            cwd: root.to_path_buf(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        });
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>(),
            [message]
        );
        assert!(!root.join("lib/model.g.dart").exists());
    }
}

#[test]
fn typed_traits_preserve_good_outputs_and_cache_when_mixin_validation_fails() {
    for (version, source) in [
        (
            "3.6.0",
            include_str!("../../../../fixtures/generic_validation/traditional.dart.snapshot"),
        ),
        (
            "3.13.0",
            include_str!("../../../../fixtures/generic_validation/primary.dart.snapshot"),
        ),
    ] {
        for fail_fast in [false, true] {
            let workspace = make_workspace();
            let root = workspace.path();
            write_file(
                &root.join("pubspec.yaml"),
                &format!("name: dust_test\nenvironment:\n  sdk: '>={version} <4.0.0'\n"),
            );
            let path = root.join("lib/model.dart");
            let output = root.join("lib/model.g.dart");
            let cache = root.join(".dart_tool/dust/build_cache_v1.json");
            write_file(&path, source);
            let request = BuildRequest {
                cwd: root.to_path_buf(),
                fail_fast,
                jobs: None,
                db: Default::default(),
            };
            assert_eq!(run_build(request.clone()).diagnostics, []);
            let warm = run_build(request.clone());
            assert_eq!(warm.diagnostics, []);
            assert!(warm.build_artifacts[0].cached);
            let before_output = fs::read(&output).unwrap();
            let before_cache = fs::read(&cache).unwrap();
            write_file(&path, &source.replace("with _$Holder<T>", "with _$Holder"));
            for writing in [false, true, false, true] {
                let result = if writing {
                    run_build(request.clone())
                } else {
                    run_check(CheckRequest {
                        cwd: root.to_path_buf(),
                        fail_fast,
                        jobs: None,
                        db: Default::default(),
                    })
                };
                assert_eq!(result.diagnostics.len(), 1);
                assert!(result.diagnostics[0].is_error());
                assert_eq!(
                    result.diagnostics[0].message,
                    "Generic model `Holder` must apply `with _$Holder<T>`"
                );
                assert_eq!(fs::read(&output).unwrap(), before_output);
                assert_eq!(fs::read(&cache).unwrap(), before_cache);
            }
        }
    }
}
