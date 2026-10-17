use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use dust_driver::{
    CheckRequest, ProgressEvent, ProgressPhase, WatchRequest, run_check, run_watch_with_progress,
};

use super::{MODEL, build, workspace, write_file};

const DEFAULT: &str = "class PlatformViewsService { static Kind initUiKitView({Kind gestureBlockingPolicy = .ready}) => gestureBlockingPolicy; }";

fn bridge(root: &Path, version: &str, declaration: &str) {
    write_file(
        &root.join("deps/bridge/lib/bridge.dart"),
        &format!(
            "// @dart={version}\nexport 'package:dust_dart/derive.dart';\nenum Kind {{ ready }}\n{declaration}"
        ),
    );
}

fn check(root: &Path) -> dust_driver::CommandResult {
    run_check(CheckRequest {
        sdk_source_root: Some(root.join("sdk")),
        cwd: root.into(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    })
}

#[test]
fn dependency_defaults_preserve_exact_generation_and_warm_checks() {
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
        DEFAULT,
        "Kind choose([Kind value = .ready]) => value;",
        "Kind choose({List<Kind> values = const [.ready]}) => values.first;",
    ] {
        bridge(root, "3.13", declaration);
        let rebuilt = build(root);
        assert_eq!(rebuilt.diagnostics, [], "{declaration}");
        assert_eq!(rebuilt.cache.unwrap().misses, 1);
        assert_eq!(fs::read(&output_path).unwrap(), output);
        assert_eq!(check(root).diagnostics, []);
        assert_eq!(build(root).cache.unwrap().hits, 1);
    }
}

#[test]
fn rejected_dependency_defaults_and_root_defaults_preserve_previous_success() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let initial = format!("import 'package:bridge/bridge.dart';\n{MODEL}");
    write_file(&model, &initial);
    bridge(root, "3.13", DEFAULT);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    for (version, declaration) in [
        ("3.6", DEFAULT),
        ("3.13", "class C { C([Kind value = .ready]); }"),
        (
            "3.13",
            "void choose([@Marker(.ready) Kind value = Kind.ready]) {}",
        ),
        ("3.13", "void choose([Kind value = .ready(,)]) {}"),
        (
            "3.13",
            "void choose(void callback([Kind value = .ready])) {}",
        ),
    ] {
        bridge(root, version, declaration);
        for result in [build(root), check(root)] {
            assert!(result.has_errors(), "{version}: {declaration}");
            assert_eq!(fs::read(&output_path).unwrap(), output);
            assert_eq!(fs::read(&cache_path).unwrap(), cache);
        }
    }
    bridge(root, "3.13", DEFAULT);
    assert_eq!(build(root).cache.unwrap().hits, 1);
    write_file(
        &model,
        &format!(
            "// @dart=3.13\n{initial}\nenum LocalKind {{ ready }}\nvoid choose([LocalKind value = .ready]) {{}}"
        ),
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

#[test]
fn watch_reloads_defaults_and_preserves_cache_when_a_constructor_default_is_rejected() {
    for (changed_source, rejects) in [
        ("Kind choose([Kind value = .ready]) => value;", false),
        ("class C { C([Kind value = .ready]); }", true),
    ] {
        let workspace = workspace();
        let root = workspace.path();
        write_file(
            &root.join("lib/model.dart"),
            &format!("import 'package:bridge/bridge.dart';\n{MODEL}"),
        );
        bridge(root, "3.13", DEFAULT);
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
                    bridge(root, "3.13", changed_source);
                }
            },
        );
        assert!(changed.load(Ordering::SeqCst));
        assert_eq!(result.has_errors(), rejects, "{:?}", result.diagnostics);
        let watch = result.watch.unwrap();
        assert_eq!(watch.rebuild_batches, 1);
        assert_eq!(watch.rebuilt_libraries, [root.join("lib/model.dart")]);
        assert_eq!(fs::read(&output_path).unwrap(), output);
        if rejects {
            assert_eq!(fs::read(&cache_path).unwrap(), cache);
            bridge(root, "3.13", DEFAULT);
        }
        assert_eq!(build(root).cache.unwrap().hits, 1);
    }
}
