//! Exact shorthand/preferred callback parity through real source loading.

use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, sdk_source_root, write_file};

#[test]
fn primary_callback_copy_output_matches_its_explicit_function_type() {
    for (preferred, shorthand) in [
        (
            "bool Function(String value) predicate",
            "bool predicate(String value)",
        ),
        (
            "bool? Function(String value) predicate",
            "bool? predicate(String value)",
        ),
        (
            "bool Function(String value)? predicate",
            "bool predicate(String value)?",
        ),
        (
            "T Function<T extends Object>(T value) echo",
            "T echo<T extends Object>(T value)",
        ),
        (
            "bool Function(bool Function(String value) next, [int? count]) predicate",
            "bool predicate(bool next(String value), [int? count])",
        ),
        (
            "bool Function({required String value, int? count}) predicate",
            "bool predicate({required String value, int? count})",
        ),
    ] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_file(
            &root.join("pubspec.yaml"),
            "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
        );
        let model = root.join("lib/callback_probe.dart");
        let request = BuildRequest {
            cwd: root.into(),
            sdk_source_root: Some(sdk_source_root(root)),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        };
        let source = |parameter| {
            format!(
                "import 'package:dust_dart/derive.dart'; part 'callback_probe.g.dart'; @Derive([CopyWith(), ToString()]) class const CallbackProbe(final {parameter}) with _$CallbackProbe;"
            )
        };
        write_file(&model, &source(preferred));
        assert_eq!(run_build(request.clone()).diagnostics, []);
        let expected = fs::read_to_string(root.join("lib/callback_probe.g.dart")).unwrap();
        write_file(&model, &source(shorthand));
        assert_eq!(run_build(request).diagnostics, []);
        assert_eq!(
            fs::read_to_string(root.join("lib/callback_probe.g.dart")).unwrap(),
            expected,
            "{shorthand}"
        );
    }
}

#[test]
fn invalid_callback_signatures_preserve_successful_output_and_cache_then_recover() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_file(
        &root.join("pubspec.yaml"),
        "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
    );
    let model = root.join("lib/callback_probe.dart");
    let request = BuildRequest {
        cwd: root.into(),
        sdk_source_root: Some(sdk_source_root(root)),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    };
    let source = |parameter| {
        format!(
            "import 'package:dust_dart/derive.dart'; part 'callback_probe.g.dart'; @Derive([CopyWith()]) class const CallbackProbe(final {parameter}) with _$CallbackProbe;"
        )
    };
    let valid = source("bool predicate(String value)");
    write_file(&model, &valid);
    assert_eq!(run_build(request.clone()).diagnostics, []);
    let output_path = root.join("lib/callback_probe.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    assert_eq!(run_build(request.clone()).cache.unwrap().hits, 1);
    for parameter in [
        "predicate(String value)",
        "bool predicate(value)",
        "bool predicate([String value = 'yes'])",
        "bool predicate(bool nested([String value = 'yes']))",
    ] {
        write_file(&model, &source(parameter));
        for result in [
            run_build(request.clone()),
            run_check(CheckRequest {
                cwd: root.into(),
                sdk_source_root: request.sdk_source_root.clone(),
                fail_fast: false,
                jobs: None,
                db: Default::default(),
            }),
        ] {
            assert!(result.has_errors(), "{parameter}");
            assert!(
                result
                    .diagnostics
                    .iter()
                    .all(|error| error.message == "Cannot lower declaring callback type"),
                "{:?}",
                result.diagnostics
            );
            assert_eq!(fs::read(&output_path).unwrap(), output);
            assert_eq!(fs::read(&cache_path).unwrap(), cache);
        }
    }
    write_file(&model, &valid);
    assert_eq!(run_build(request).cache.unwrap().hits, 1);
}

#[test]
fn unused_inferred_callbacks_do_not_block_an_unrelated_model() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/model.dart"),
        "import 'package:dust_dart/derive.dart'; part 'model.g.dart'; @Derive([ToString()]) class Model { final int value; const Model(this.value); } class Other { Other(callback(value)); void call(callback(value)) {} } void call(callback(value)) {}",
    );
    let result = run_build(BuildRequest {
        cwd: root.into(),
        sdk_source_root: Some(sdk_source_root(root)),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });
    assert_eq!(result.diagnostics, []);
}

#[test]
fn watch_preserves_callback_outputs_on_invalid_edits_and_recovers() {
    use dust_driver::{ProgressEvent, ProgressPhase, WatchRequest, run_watch_with_progress};
    use std::sync::atomic::{AtomicBool, Ordering};

    let workspace = make_workspace();
    let root = workspace.path();
    write_file(
        &root.join("pubspec.yaml"),
        "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
    );
    let model = root.join("lib/model.dart");
    let valid = "import 'package:dust_dart/derive.dart'; part 'model.g.dart'; @Derive([CopyWith()]) class const Model(final bool predicate(String value)) with _$Model;";
    write_file(&model, valid);
    let request = BuildRequest {
        cwd: root.into(),
        sdk_source_root: Some(sdk_source_root(root)),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    };
    assert_eq!(run_build(request.clone()).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    let edited = AtomicBool::new(false);
    let result = run_watch_with_progress(
        WatchRequest {
            cwd: root.into(),
            sdk_source_root: request.sdk_source_root.clone(),
            fail_fast: false,
            jobs: None,
            poll_interval_ms: 0,
            max_cycles: Some(1),
        },
        |event| {
            if matches!(
                event,
                ProgressEvent::FinishedLibrary {
                    phase: ProgressPhase::WatchInitial,
                    ..
                }
            ) && !edited.swap(true, Ordering::SeqCst)
            {
                write_file(&model, &valid.replace("String value", "value"));
            }
        },
    );
    assert!(edited.load(Ordering::SeqCst));
    assert!(result.has_errors());
    assert!(
        result
            .diagnostics
            .iter()
            .all(|error| error.message == "Cannot lower declaring callback type")
    );
    assert_eq!(result.watch.unwrap().rebuild_batches, 1);
    assert_eq!(fs::read(output_path).unwrap(), output);
    assert_eq!(fs::read(cache_path).unwrap(), cache);
    write_file(&model, valid);
    assert_eq!(run_build(request).cache.unwrap().hits, 1);
}

#[test]
fn callbacks_do_not_become_scalar_json_or_row_fields() {
    for (import, derive, expected) in [
        (
            "serde.dart",
            "Serialize()",
            "`Serialize` does not support function types on `Model.predicate`",
        ),
        (
            "serde.dart",
            "Deserialize()",
            "`Deserialize` does not support function types on `Model.predicate`",
        ),
        (
            "db.dart",
            "FromRow()",
            "unsupported SQLx row field type `bool Function(String value)`",
        ),
    ] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_file(
            &root.join("pubspec.yaml"),
            "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
        );
        write_file(
            &root.join("lib/model.dart"),
            &format!(
                "import 'package:dust_dart/{import}'; part 'model.g.dart'; @Derive([{derive}]) class const Model(final bool predicate(String value)) with _$Model;"
            ),
        );
        let result = run_build(BuildRequest {
            cwd: root.into(),
            sdk_source_root: Some(sdk_source_root(root)),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        });
        assert!(result.has_errors());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|error| error.message.as_str())
                .collect::<Vec<_>>(),
            [expected]
        );
        assert!(!root.join("lib/model.g.dart").exists());
        assert!(!root.join(".dart_tool/dust/build_cache_v1.json").exists());
    }
}
