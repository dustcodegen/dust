//! Canonical type ownership requests execute only for selected plugins.

#[path = "support/library.rs"]
mod library;

use dust_diagnostics::Diagnostic;
use dust_ir::DartFileIr;
use dust_plugin_api::{
    DustPlugin, PluginContext, PluginContribution, PluginRegistry, TypeOriginRequest,
};

struct RequestPlugin(&'static str);

impl DustPlugin for RequestPlugin {
    fn plugin_name(&self) -> &'static str {
        self.0
    }

    fn type_origin_requests(&self, file: &DartFileIr) -> Vec<TypeOriginRequest> {
        assert_ne!(self.0, "inactive", "symbols-only plugins must not execute");
        vec![TypeOriginRequest {
            name: self.0.to_owned(),
            span: file.span,
        }]
    }

    fn validate(&self, _: &DartFileIr) -> Vec<Diagnostic> {
        vec![]
    }

    fn generate(&self, _: &DartFileIr, _: &PluginContext<'_>) -> Vec<PluginContribution> {
        vec![]
    }
}

#[test]
fn registry_aggregates_requests_from_executing_plugins_in_order() {
    let mut registry = PluginRegistry::new();
    registry.register(Box::new(RequestPlugin("first"))).unwrap();
    registry
        .register_symbols_only(Box::new(RequestPlugin("inactive")))
        .unwrap();
    registry
        .register(Box::new(RequestPlugin("second")))
        .unwrap();
    let file = library::sample_library();
    assert_eq!(
        registry.type_origin_requests(&file),
        vec![
            TypeOriginRequest {
                name: "first".to_owned(),
                span: file.span
            },
            TypeOriginRequest {
                name: "second".to_owned(),
                span: file.span
            },
        ]
    );
}

#[test]
fn plugins_default_to_no_type_origin_requests() {
    struct DefaultPlugin;
    impl DustPlugin for DefaultPlugin {
        fn plugin_name(&self) -> &'static str {
            "default"
        }

        fn validate(&self, _: &DartFileIr) -> Vec<Diagnostic> {
            vec![]
        }

        fn generate(&self, _: &DartFileIr, _: &PluginContext<'_>) -> Vec<PluginContribution> {
            vec![]
        }
    }
    let file = library::sample_library();
    assert!(DefaultPlugin.type_origin_requests(&file).is_empty());
    let mut registry = PluginRegistry::new();
    registry.register(Box::new(DefaultPlugin)).unwrap();
    assert!(registry.type_origin_requests(&file).is_empty());
}
