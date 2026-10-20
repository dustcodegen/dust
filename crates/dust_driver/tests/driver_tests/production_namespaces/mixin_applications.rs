use super::{MODEL, build, workspace, write_file};
use dust_driver::{CheckRequest, run_check};
use std::fs;

const ALIAS: &str = "class Base { const Base(this.value); final int value; } mixin Mix {} @Derive([ToString()]) class Model = Base with Mix;";

#[test]
fn foreign_named_mixin_metadata_preserves_ordinary_generated_bytes_and_warm_cache() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let original = format!(
        "import 'package:dust_dart/derive.dart' as d;\n{}",
        MODEL.replace("@Derive([ToString()])", "@d.Derive([d.ToString()])")
    );
    write_file(&model, &original);
    assert_eq!(build(root).diagnostics, []);
    let expected = fs::read(root.join("lib/model.g.dart")).unwrap();
    let foreign = "class Base { const Base(); } mixin Mix {} class ToString = Base with Mix;";
    for imported in [false, true] {
        let source = if imported {
            write_file(&root.join("deps/bridge/lib/bridge.dart"), foreign);
            format!(
                "import 'package:bridge/bridge.dart' as foreign;\n{}",
                original.replace("final int value;", "@foreign.ToString() final int value;")
            )
        } else {
            format!(
                "{}\n{foreign}",
                original.replace("final int value;", "@ToString() final int value;")
            )
        };
        write_file(&model, &source);
        assert_eq!(build(root).diagnostics, []);
        assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), expected);
        let warm = build(root);
        assert_eq!(warm.diagnostics, []);
        assert_eq!(warm.cache.unwrap().hits, 1);
    }
}

#[test]
fn unsupported_named_mixin_generation_preserves_output_cache_and_recovers() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let original = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &original);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    for (declarations, target) in [
        (ALIAS.to_owned(), "Model"),
        ("class Base { const Base(this.value); final int value; } mixin Mix {} class Alias = Base with Mix; @Derive([ToString()]) class Model extends Alias { const Model(super.value); }".to_owned(), "Alias"),
    ] {
        let source = format!("import 'package:dust_dart/derive.dart'; part 'model.g.dart'; {declarations}");
        write_file(&model, &source);
        for result in [
            build(root),
            run_check(CheckRequest {
                cwd: root.into(), sdk_source_root: Some(root.join("sdk")),
                fail_fast: false, jobs: None, db: Default::default(),
            }),
        ] {
            assert!(result.has_errors());
            assert_eq!(result.diagnostics.iter().map(|diagnostic| diagnostic.message.as_str()).collect::<Vec<_>>(), [format!("Dust generation does not support named mixin-application class `{target}`; implicit forwarding constructors are unsupported")]);
            let [label] = result.diagnostics[0].labels.as_slice() else { panic!("one declaration label expected") };
            assert_eq!(label.message, "named mixin application declared here");
            let start = source.find(&format!("class {target} =")).unwrap();
            let end = start + source[start..].find(';').unwrap() + 1;
            let expected_start = if target == "Model" { source.find("@Derive").unwrap() } else { start };
            assert_eq!(label.range, dust_text::TextRange::new(expected_start as u32, end as u32));
            assert_eq!(fs::read(&output_path).unwrap(), output);
            assert_eq!(fs::read(&cache_path).unwrap(), cache);
        }
    }
    write_file(&model, &original);
    assert_eq!(build(root).cache.unwrap().hits, 1);
}

#[test]
fn cold_unsupported_named_mixin_does_not_create_generated_output() {
    let workspace = workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/model.dart"),
        &format!("import 'package:dust_dart/derive.dart'; part 'model.g.dart'; {ALIAS}"),
    );
    let result = build(root);
    assert!(result.has_errors());
    assert_eq!(result.diagnostics.len(), 1);
    assert!(!root.join("lib/model.g.dart").exists());
}

#[test]
fn watch_named_mixin_failure_preserves_success_and_recovers_after_restoration() {
    use dust_driver::{ProgressEvent, ProgressPhase, WatchRequest, run_watch_with_progress};
    use std::sync::atomic::{AtomicBool, Ordering};

    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let original = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &original);
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
                    &model,
                    &format!(
                        "import 'package:dust_dart/derive.dart'; part 'model.g.dart'; {ALIAS}"
                    ),
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
        [
            "Dust generation does not support named mixin-application class `Model`; implicit forwarding constructors are unsupported"
        ]
    );
    let watch = result.watch.unwrap();
    assert_eq!(watch.rebuild_batches, 1);
    assert_eq!(
        watch.rebuilt_libraries.as_slice(),
        std::slice::from_ref(&model)
    );
    assert_eq!(fs::read(output_path).unwrap(), output);
    assert_eq!(fs::read(cache_path).unwrap(), cache);
    write_file(&model, &original);
    assert_eq!(build(root).cache.unwrap().hits, 1);
}
