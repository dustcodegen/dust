use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use dust_diagnostics::Diagnostic;
use dust_emitter::emit_library;
use dust_ir::DartFileIr;
use dust_plugin_api::{DustPlugin, PluginContext, PluginContribution, PluginRegistry};

use super::support::{FakePlugin, sample_library};

struct ValidationProbe {
    diagnostic: Diagnostic,
    calls: Arc<AtomicUsize>,
}

impl DustPlugin for ValidationProbe {
    fn plugin_name(&self) -> &'static str {
        "validation_probe"
    }

    fn validate(&self, _: &DartFileIr) -> Vec<Diagnostic> {
        vec![self.diagnostic.clone()]
    }

    fn generate(&self, _: &DartFileIr, _: &PluginContext<'_>) -> Vec<PluginContribution> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        vec![PluginContribution {
            primary_source: Some("const answer = 42;\n".into()),
            ..Default::default()
        }]
    }
}

#[test]
fn validation_errors_stop_generation_and_preserve_previous_output() {
    for previous in [None, Some("previous generated output\n")] {
        let calls = Arc::new(AtomicUsize::new(0));
        let error = Diagnostic::error("invalid input");
        let warning = Diagnostic::warning("another validation warning");
        let mut registry = PluginRegistry::new();
        registry
            .register(Box::new(ValidationProbe {
                diagnostic: error.clone(),
                calls: Arc::clone(&calls),
            }))
            .unwrap();
        registry
            .register(Box::new(FakePlugin {
                name: "second_validation",
                requested: vec![],
                diagnostics: vec![warning.clone()],
                contribution: Default::default(),
            }))
            .unwrap();
        let result = emit_library(
            &sample_library("lib/user.g.dart".into()),
            &registry,
            previous,
        );
        assert_eq!(result.diagnostics, [error, warning]);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(result.source, previous.unwrap_or_default());
        assert!(!result.changed);
        assert!(!result.suppress_primary_output);
        assert_eq!(result.auxiliary_outputs, []);
    }
}

#[test]
fn validation_warnings_do_not_stop_generation() {
    let calls = Arc::new(AtomicUsize::new(0));
    let warning = Diagnostic::warning("valid input with warning");
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(ValidationProbe {
            diagnostic: warning.clone(),
            calls: Arc::clone(&calls),
        }))
        .unwrap();
    let result = emit_library(&sample_library("lib/user.g.dart".into()), &registry, None);
    assert_eq!(result.diagnostics, [warning]);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(result.source, "const answer = 42;\n");
    assert!(result.changed);
}

#[test]
fn validation_errors_do_not_delete_primary_or_replace_auxiliary_files() {
    let directory = tempfile::tempdir().unwrap();
    let primary = directory.path().join("user.g.dart");
    let auxiliary = directory.path().join("metadata.dart");
    std::fs::write(&primary, "previous primary\n").unwrap();
    std::fs::write(&auxiliary, "previous auxiliary\n").unwrap();
    let error = Diagnostic::error("invalid input");
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(FakePlugin {
            name: "invalid_suppression",
            requested: vec![],
            diagnostics: vec![error.clone()],
            contribution: PluginContribution {
                suppress_primary_output: true,
                auxiliary_outputs: vec![dust_plugin_api::AuxiliaryOutputContribution {
                    output_path: auxiliary.clone(),
                    source: "replacement\n".into(),
                }],
                ..Default::default()
            },
        }))
        .unwrap();
    let result =
        dust_emitter::write_library(&sample_library(primary.display().to_string()), &registry)
            .unwrap();
    assert_eq!(result.diagnostics, [error]);
    assert!(!result.changed);
    assert!(!result.written);
    assert!(!result.suppress_primary_output);
    assert_eq!(result.auxiliary_outputs, []);
    assert_eq!(
        std::fs::read_to_string(primary).unwrap(),
        "previous primary\n"
    );
    assert_eq!(
        std::fs::read_to_string(auxiliary).unwrap(),
        "previous auxiliary\n"
    );
}
