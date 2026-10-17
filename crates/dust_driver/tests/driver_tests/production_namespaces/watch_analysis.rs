use std::{
    fs,
    sync::atomic::{AtomicBool, Ordering},
};

use dust_driver::{
    BuildRequest, ProgressEvent, ProgressPhase, WatchRequest, run_build, run_watch_with_progress,
};

use crate::support::{DustImport, make_workspace, sdk_source_root, write_dust_file};

#[test]
fn changed_consumer_keeps_unchanged_enum_workspace_facts() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_dust_file(
        &root.join("lib/status.dart"),
        &[DustImport::Derive],
        "part 'status.g.dart'; @Derive([Serialize(), Deserialize()]) enum Status { draft, published }",
    );
    let source =
        "import 'status.dart'; part 'product.g.dart'; @Derive([Serialize(), Deserialize()])
        class Product with _$Product { const Product(this.status); final Status status; }";
    let product = root.join("lib/product.dart");
    write_dust_file(&product, &[DustImport::Derive], source);
    let changed = AtomicBool::new(false);
    let result = run_watch_with_progress(
        WatchRequest {
            sdk_source_root: Some(sdk_source_root(root)),
            cwd: root.into(),
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
                write_dust_file(
                    &product,
                    &[DustImport::Derive],
                    &format!("{source}\n// consumer edit\n"),
                );
            }
        },
    );
    assert!(changed.load(Ordering::SeqCst));
    assert_eq!(result.diagnostics, []);
    assert_eq!(result.watch.unwrap().rebuilt_libraries, [product]);
    let watched = fs::read_to_string(root.join("lib/product.g.dart")).unwrap();
    let built = run_build(BuildRequest {
        sdk_source_root: Some(sdk_source_root(root)),
        cwd: root.into(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });
    assert_eq!(built.diagnostics, []);
    assert_eq!(built.cache.unwrap().hits, 2);
    assert_eq!(
        watched,
        fs::read_to_string(root.join("lib/product.g.dart")).unwrap(),
        "watch must emit exactly the same cross-library enum calls as a complete build"
    );
}
