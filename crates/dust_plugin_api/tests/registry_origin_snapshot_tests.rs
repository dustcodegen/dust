//! Registration must retain the exact provenance it validated, like symbol claims.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use dust_diagnostics::Diagnostic;
use dust_ir::DartFileIr;
use dust_plugin_api::{
    AnnotationOrigin, DustPlugin, PluginContext, PluginContribution, PluginRegistry,
};

/// A metadata provider that changes its answer after the first read.
struct ChangingOrigin {
    /// Observe provider calls without exposing the registered plugin itself.
    calls: Arc<AtomicUsize>,
}

/// The origin accepted during registration.
const FIRST: &[AnnotationOrigin] = &[AnnotationOrigin::new(
    "test::Marker",
    "package:test/first.dart",
    "Marker",
)];
/// A later answer must not silently replace already-validated identity.
const SECOND: &[AnnotationOrigin] = &[AnnotationOrigin::new(
    "test::Marker",
    "package:test/second.dart",
    "Marker",
)];

impl DustPlugin for ChangingOrigin {
    fn plugin_name(&self) -> &'static str {
        "changing"
    }
    fn claimed_configs(&self) -> &'static [&'static str] {
        &["test::Marker"]
    }
    fn annotation_origins(&self) -> &'static [AnnotationOrigin] {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            FIRST
        } else {
            SECOND
        }
    }
    fn validate(&self, _: &DartFileIr) -> Vec<Diagnostic> {
        vec![]
    }
    fn generate(&self, _: &DartFileIr, _: &PluginContext<'_>) -> Vec<PluginContribution> {
        vec![]
    }
}

#[test]
fn registration_snapshots_the_exact_origins_it_validated() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(ChangingOrigin {
            calls: Arc::clone(&calls),
        }))
        .unwrap();
    assert_eq!(registry.annotation_origins(), FIRST);
    assert_eq!(registry.annotation_origins(), FIRST);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
