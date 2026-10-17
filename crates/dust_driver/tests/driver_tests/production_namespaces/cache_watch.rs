use std::{
    fs,
    sync::atomic::{AtomicBool, Ordering},
};

use dust_driver::{
    CheckRequest, ProgressEvent, ProgressPhase, WatchRequest, run_check, run_watch_with_progress,
};

use super::{MODEL, build, workspace, write_file};

const BARREL: &str = "export 'package:dust_dart/derive.dart';";
const HIDDEN: &str = "export 'package:dust_dart/derive.dart' hide ToString;";

#[test]
fn dependency_edits_invalidate_only_affected_hits_and_errors_preserve_success() {
    let workspace = workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/a.dart"),
        &format!(
            "import 'package:bridge/bridge.dart';\n{}",
            MODEL
                .replace("model.g.dart", "a.g.dart")
                .replace("Model", "A")
        ),
    );
    write_file(
        &root.join("lib/b.dart"),
        &format!(
            "import 'package:dust_dart/derive.dart';\n{}",
            MODEL
                .replace("model.g.dart", "b.g.dart")
                .replace("Model", "B")
        ),
    );
    let cold = build(root);
    assert_eq!(cold.diagnostics, []);
    assert_eq!(cold.cache.unwrap().misses, 2);
    let warm = build(root);
    assert_eq!(warm.diagnostics, []);
    assert_eq!(warm.cache.unwrap().hits, 2);
    let barrel = root.join("deps/bridge/lib/bridge.dart");
    let edited = format!("{BARREL}\n// interface dependency changed\n");
    write_file(&barrel, &edited);
    let refreshed = build(root);
    assert_eq!(refreshed.diagnostics, []);
    let cache = refreshed.cache.unwrap();
    assert_eq!((cache.hits, cache.misses), (1, 1));
    let a = fs::read(root.join("lib/a.g.dart")).unwrap();
    let b = fs::read(root.join("lib/b.g.dart")).unwrap();
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let cache = fs::read(&cache_path).unwrap();
    write_file(&barrel, HIDDEN);
    for result in [
        build(root),
        run_check(CheckRequest {
            cwd: root.into(),
            sdk_source_root: Some(root.join("sdk")),
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
            ["annotation name `ToString` is not visible in this namespace"]
        );
        assert_eq!(fs::read(root.join("lib/a.g.dart")).unwrap(), a);
        assert_eq!(fs::read(root.join("lib/b.g.dart")).unwrap(), b);
        assert_eq!(fs::read(&cache_path).unwrap(), cache);
    }
    write_file(&barrel, &edited);
    let recovered = build(root);
    assert_eq!(recovered.diagnostics, []);
    assert_eq!(recovered.cache.unwrap().hits, 2);
}

#[test]
fn watch_reloads_barrel_ownership_without_a_source_edit() {
    let workspace = workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/model.dart"),
        &format!("import 'package:bridge/bridge.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let output = fs::read(root.join("lib/model.g.dart")).unwrap();
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
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
                write_file(&root.join("deps/bridge/lib/bridge.dart"), HIDDEN);
            }
        },
    );
    assert!(changed.load(Ordering::SeqCst));
    assert!(result.has_errors(), "{:?}", result.diagnostics);
    let watch = result.watch.unwrap();
    assert_eq!(watch.rebuild_batches, 1);
    assert_eq!(watch.rebuilt_libraries, [root.join("lib/model.dart")]);
    assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), output);
    assert_eq!(fs::read(cache_path).unwrap(), cache);
}
