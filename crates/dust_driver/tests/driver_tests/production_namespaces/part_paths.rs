use std::fs;

use super::{MODEL, build, workspace, write_file};

#[test]
fn equivalent_relative_primary_parts_preserve_exact_generation_and_warm_hits() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let output = fs::read(&output_path).unwrap();
    for reference in [
        "./model.g.dart",
        "scratch/../model.g.dart",
        "./scratch/../../lib/model.g.dart",
        "package:dust_test/model.g.dart",
        "%6dodel.g.dart",
        "model.g%2Edart",
    ] {
        fs::remove_file(&output_path).unwrap();
        write_file(
            &model,
            &source.replace("part 'model.g.dart';", &format!("part '{reference}';")),
        );
        let cold = build(root);
        assert_eq!(cold.diagnostics, [], "{reference}");
        assert_eq!(cold.cache.unwrap().misses, 1);
        assert_eq!(fs::read(&output_path).unwrap(), output, "{reference}");
        let warm = build(root);
        assert_eq!(warm.diagnostics, [], "{reference}");
        assert_eq!(warm.cache.unwrap().hits, 1);
    }
    write_file(
        &model,
        &source.replace(
            "part 'model.g.dart';",
            &format!("part 'file://{}';", output_path.display()),
        ),
    );
    assert_eq!(build(root).diagnostics, []);
    assert_eq!(fs::read(&output_path).unwrap(), output);
    write_file(
        &model,
        &source.replace("part 'model.g.dart';", "part r'%6dodel.g.dart';"),
    );
    assert_eq!(build(root).diagnostics, []);
    assert_eq!(fs::read(&output_path).unwrap(), output);
}

#[test]
fn an_existing_same_named_foreign_part_cannot_stand_in_for_the_primary_output() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    let foreign_path = root.join("lib/foreign/model.g.dart");
    let foreign = "part of '../model.dart';\nconst foreignMarker = 1;\n";
    write_file(&foreign_path, foreign);
    write_file(
        &model,
        &source.replace("part 'model.g.dart';", "part 'foreign/model.g.dart';"),
    );
    for result in [
        build(root),
        dust_driver::run_check(dust_driver::CheckRequest {
            sdk_source_root: Some(root.join("sdk")),
            cwd: root.into(),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        }),
    ] {
        assert!(
            result.has_errors(),
            "diagnostics={:?}; primary preserved={}; cache preserved={}; foreign preserved={}",
            result.diagnostics,
            fs::read(&output_path).unwrap() == output,
            fs::read(&cache_path).unwrap() == cache,
            fs::read_to_string(&foreign_path).unwrap() == foreign
        );
        assert_eq!(fs::read(&output_path).unwrap(), output);
        assert_eq!(fs::read(&cache_path).unwrap(), cache);
        assert_eq!(fs::read_to_string(&foreign_path).unwrap(), foreign);
    }
}

#[test]
fn missing_foreign_part_is_not_a_generated_placeholder_even_with_the_same_basename() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    write_file(
        &model,
        &source.replace("part 'model.g.dart';", "part 'foreign/model.g.dart';"),
    );
    let failed = build(root);
    assert!(failed.has_errors());
    assert!(
        failed.diagnostics.iter().all(|error| error
            .message
            .contains("cannot read namespace source")
            && error.message.contains("foreign/model.g.dart")),
        "{:?}",
        failed.diagnostics
    );
    assert_eq!(fs::read(output_path).unwrap(), output);
    assert_eq!(fs::read(cache_path).unwrap(), cache);
    assert!(!root.join("lib/foreign/model.g.dart").exists());
}

#[test]
fn an_existing_same_named_foreign_part_may_precede_the_real_primary_part() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let output = fs::read(&output_path).unwrap();
    fs::remove_file(&output_path).unwrap();
    let foreign_path = root.join("lib/foreign/model.g.dart");
    let foreign = "part of '../model.dart';\nconst foreignMarker = 1;\n";
    write_file(&foreign_path, foreign);
    write_file(
        &model,
        &source.replace(
            "part 'model.g.dart';",
            "part 'foreign/model.g.dart';\npart 'model.g.dart';",
        ),
    );
    assert_eq!(build(root).diagnostics, []);
    assert_eq!(fs::read(&output_path).unwrap(), output);
    assert_eq!(fs::read_to_string(&foreign_path).unwrap(), foreign);
    assert_eq!(build(root).cache.unwrap().hits, 1);
}

#[test]
fn custom_primary_suffix_in_a_nested_directory_preserves_escaped_spaces() {
    let workspace = workspace();
    let root = workspace.path();
    write_file(
        &root.join("dust.yaml"),
        "outputs:\n  primary_suffix: .custom.dart\n",
    );
    let model = root.join("lib/nested/model space.dart");
    let source = format!(
        "import 'package:dust_dart/derive.dart';\n{}",
        MODEL.replace("model.g.dart", "model%20space.custom.dart")
    );
    write_file(&model, &source);
    let output = root.join("lib/nested/model space.custom.dart");
    let cold = build(root);
    assert_eq!(cold.diagnostics, []);
    let bytes = fs::read(&output).unwrap();
    assert_eq!(build(root).cache.unwrap().hits, 1);
    write_file(
        &model,
        &source.replace("model%20space.custom.dart", "./model%20space.custom.dart"),
    );
    assert_eq!(build(root).diagnostics, []);
    assert_eq!(fs::read(output).unwrap(), bytes);
}

#[test]
fn watch_rejects_foreign_part_and_build_recovers_without_losing_prior_success() {
    use dust_driver::{ProgressEvent, ProgressPhase, WatchRequest, run_watch_with_progress};
    use std::sync::atomic::{AtomicBool, Ordering};

    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let foreign = root.join("lib/foreign/model.g.dart");
    let foreign_bytes = "part of '../model.dart';\nconst foreignMarker = 1;\n";
    write_file(&foreign, foreign_bytes);
    assert_eq!(build(root).diagnostics, []);
    let cache = fs::read(&cache_path).unwrap();
    let changed = AtomicBool::new(false);
    let result = run_watch_with_progress(
        WatchRequest {
            cwd: root.into(),
            sdk_source_root: Some(root.join("sdk")),
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
            ) && !changed.swap(true, Ordering::SeqCst)
            {
                write_file(
                    &model,
                    &source.replace("part 'model.g.dart';", "part 'foreign/model.g.dart';"),
                );
            }
        },
    );
    assert!(changed.load(Ordering::SeqCst));
    assert!(result.has_errors(), "{:?}", result.diagnostics);
    assert_eq!(fs::read(&output_path).unwrap(), output);
    assert_eq!(fs::read(&cache_path).unwrap(), cache);
    assert_eq!(fs::read_to_string(&foreign).unwrap(), foreign_bytes);
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    assert_eq!(fs::read(&output_path).unwrap(), output);
    assert_eq!(build(root).cache.unwrap().hits, 1);
}
