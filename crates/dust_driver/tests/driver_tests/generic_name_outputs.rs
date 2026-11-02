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

#[test]
fn member_named_binders_fail_before_replacing_output_or_cache() {
    for (version, supported, rejected) in [
        (
            "3.6.0",
            traditional_probe("T", "T"),
            traditional_probe("copyWith, toString, hashCode", "copyWith"),
        ),
        (
            "3.13.0",
            primary_probe("T", "T"),
            primary_probe("copyWith, toString, hashCode", "copyWith"),
        ),
    ] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_file(
            &root.join("pubspec.yaml"),
            &format!("name: dust_test\nenvironment:\n  sdk: '>={version} <4.0.0'\n"),
        );
        let source = root.join("lib/probe.dart");
        let output = root.join("lib/probe.g.dart");
        let cache = root.join(".dart_tool/dust/build_cache_v1.json");
        write_file(&source, &supported);
        let request = BuildRequest {
            sdk_source_root: Some(crate::support::sdk_source_root(root)),
            cwd: root.to_path_buf(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        };
        assert_eq!(run_build(request.clone()).diagnostics, []);
        assert!(run_build(request.clone()).build_artifacts[0].cached);
        let previous_output = fs::read(&output).unwrap();
        let previous_cache = fs::read(&cache).unwrap();

        write_file(&source, &rejected);
        for writing in [false, true] {
            let diagnostics = if writing {
                run_build(request.clone()).diagnostics
            } else {
                run_check(CheckRequest {
                    sdk_source_root: Some(crate::support::sdk_source_root(root)),
                    cwd: root.to_path_buf(),
                    fail_fast: false,
                    jobs: None,
                    db: Default::default(),
                })
                .diagnostics
            };
            assert_eq!(diagnostics.len(), 3, "{version}: {diagnostics:?}");
            for (diagnostic, member) in diagnostics.iter().zip(["toString", "hashCode", "copyWith"])
            {
                assert_eq!(
                    diagnostic.message,
                    format!(
                        "generic parameter `{member}` on `Probe` conflicts with generated `{member}` member"
                    )
                );
                let label = &diagnostic.labels[0];
                assert_eq!(
                    &rejected[label.range.start().to_usize()..label.range.end().to_usize()],
                    member
                );
            }
            assert_eq!(fs::read(&output).unwrap(), previous_output);
            assert_eq!(fs::read(&cache).unwrap(), previous_cache);
        }
    }
}

fn traditional_probe(parameters: &str, value_type: &str) -> String {
    format!(
        "import 'package:dust_dart/derive.dart';\npart 'probe.g.dart';\n@Derive([CopyWith(), ToString(), Eq()])\nclass Probe<{parameters}> with _$Probe<{parameters}> {{\n  const Probe(this.value);\n  final {value_type} value;\n}}\n"
    )
}

fn primary_probe(parameters: &str, value_type: &str) -> String {
    format!(
        "import 'package:dust_dart/derive.dart';\npart 'probe.g.dart';\n@Derive([CopyWith(), ToString(), Eq()])\nclass const Probe<{parameters}>(final {value_type} value) with _$Probe<{parameters}> {{\n  this;\n}}\n"
    )
}
