use dust_driver::{CheckRequest, run_check};
use std::fs;

use super::{MODEL, build, workspace, write_file};

#[test]
fn class_constant_metadata_shadows_an_import_prefix_without_changing_generation() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!(
        "import 'package:dust_dart/derive.dart' as d;\n{}",
        MODEL.replace("@Derive([ToString()])", "@d.Derive([d.ToString()])")
    );
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let output = fs::read(&output_path).unwrap();
    write_file(
        &model,
        &format!(
            "{source}\nclass Other {{
        const Other(this.value); static const d = Object(); @d final int value;
    }}"
        ),
    );
    let result = build(root);
    assert_eq!(result.diagnostics, []);
    assert_eq!(fs::read(output_path).unwrap(), output);
    let warm = build(root);
    assert_eq!(warm.diagnostics, []);
    assert_eq!(warm.cache.unwrap().hits, 1);
}

#[test]
fn dependency_class_constant_initializers_load_real_imports_and_invalidate_success() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    write_file(
        &model,
        &format!(
            "import 'package:dust_dart/derive.dart'; import 'package:bridge/bridge.dart' as bridge;\n{}",
            MODEL.replace("final int value;", "@bridge.Holder.marker final int value;")
        ),
    );
    write_file(
        &root.join("deps/bridge/lib/bridge.dart"),
        "import 'leaf.dart' as leaf; part 'markers.dart';",
    );
    write_file(
        &root.join("deps/bridge/lib/markers.dart"),
        "part of 'bridge.dart'; class Holder { static const marker = leaf.ForeignMarker(); }",
    );
    let leaf = root.join("deps/bridge/lib/leaf.dart");
    let foreign = "class ForeignMarker { const ForeignMarker(); }";
    write_file(&leaf, foreign);
    let initial = build(root);
    assert_eq!(initial.diagnostics, []);
    let output = root.join("lib/model.g.dart");
    let cache = root.join(".dart_tool/dust/build_cache_v1.json");
    let expected_output = fs::read(&output).unwrap();
    let expected_cache = fs::read(&cache).unwrap();
    assert_eq!(build(root).cache.unwrap().hits, 1);

    write_file(
        &leaf,
        "import 'package:dust_dart/derive.dart' as d; typedef ForeignMarker = d.ToString;",
    );
    for changed in [
        build(root),
        run_check(CheckRequest {
            cwd: root.into(),
            sdk_source_root: Some(root.join("sdk")),
            fail_fast: false,
            jobs: None,
            db: Default::default(),
        }),
    ] {
        assert_eq!(
            changed
                .diagnostics
                .iter()
                .map(|error| error.message.as_str())
                .collect::<Vec<_>>(),
            [
                "Dust annotation constants are unsupported; use a direct annotation with its arguments"
            ]
        );
        assert!(changed.has_errors());
        assert_eq!(fs::read(&output).unwrap(), expected_output);
        assert_eq!(fs::read(&cache).unwrap(), expected_cache);
    }
    write_file(&leaf, foreign);
    assert_eq!(build(root).cache.unwrap().hits, 1);
}

#[test]
fn watch_reloads_constant_initializer_dependencies_and_preserves_success_on_failure() {
    use dust_driver::{ProgressEvent, ProgressPhase, WatchRequest, run_watch_with_progress};
    use std::sync::atomic::{AtomicBool, Ordering};

    let workspace = workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/model.dart"),
        &format!(
            "import 'package:dust_dart/derive.dart'; import 'package:bridge/bridge.dart' as bridge;\n{}",
            MODEL.replace("final int value;", "@bridge.marker final int value;")
        ),
    );
    write_file(
        &root.join("deps/bridge/lib/bridge.dart"),
        "import 'leaf.dart' as leaf; const marker = leaf.ForeignMarker();",
    );
    let leaf = root.join("deps/bridge/lib/leaf.dart");
    let foreign = "class ForeignMarker { const ForeignMarker(); }";
    write_file(&leaf, foreign);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
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
                    &leaf,
                    "import 'package:dust_dart/derive.dart' as d; typedef ForeignMarker = d.ToString;",
                );
            }
        },
    );
    assert!(changed.load(Ordering::SeqCst));
    assert!(result.has_errors());
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|error| error.message.as_str())
            .collect::<Vec<_>>(),
        ["Dust annotation constants are unsupported; use a direct annotation with its arguments"]
    );
    let watch = result.watch.unwrap();
    assert_eq!(watch.rebuild_batches, 1);
    assert_eq!(watch.rebuilt_libraries, [root.join("lib/model.dart")]);
    assert_eq!(fs::read(output_path).unwrap(), output);
    assert_eq!(fs::read(cache_path).unwrap(), cache);
    write_file(&leaf, foreign);
    assert_eq!(build(root).cache.unwrap().hits, 1);
}
