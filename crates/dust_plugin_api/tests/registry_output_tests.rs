//! Potential outputs are metadata, never authority to execute a producer.
use std::path::{Path, PathBuf};

use dust_diagnostics::Diagnostic;
use dust_ir::{DartFileIr, SymbolId};
use dust_plugin_api::{
    DustPlugin, GeneratedLibrary, PluginContext, PluginContribution, PluginRegistry,
};

struct Probe {
    claim: &'static str,
    producer: &'static str,
    path: PathBuf,
}

impl DustPlugin for Probe {
    fn plugin_name(&self) -> &'static str {
        "probe"
    }
    fn claimed_configs(&self) -> &'static [&'static str] {
        if self.claim == "one::Producer" {
            &["one::Producer"]
        } else {
            &["two::Producer"]
        }
    }
    fn potential_generated_libraries(&self, _: &Path) -> Vec<GeneratedLibrary> {
        vec![GeneratedLibrary {
            path: self.path.clone(),
            producer: SymbolId::new(self.producer),
        }]
    }
    fn validate(&self, _: &DartFileIr) -> Vec<Diagnostic> {
        panic!("metadata must not validate")
    }
    fn generate(&self, _: &DartFileIr, _: &PluginContext<'_>) -> Vec<PluginContribution> {
        panic!("metadata must not generate")
    }
}

fn probe(root: &Path) -> Probe {
    Probe {
        claim: "one::Producer",
        producer: "one::Producer",
        path: root.join("output.g.dart"),
    }
}

#[test]
fn active_and_symbols_only_registrations_expose_the_same_metadata() {
    let root = std::env::current_dir().unwrap();
    for active in [true, false] {
        let mut registry = PluginRegistry::new();
        if active {
            registry.register(Box::new(probe(&root))).unwrap();
        } else {
            registry
                .register_symbols_only(Box::new(probe(&root)))
                .unwrap();
        }
        assert_eq!(
            registry
                .potential_generated_libraries(&root.join("source.dart"))
                .unwrap(),
            [GeneratedLibrary {
                path: root.join("output.g.dart"),
                producer: SymbolId::new("one::Producer"),
            }]
        );
    }
}

#[test]
fn invalid_output_metadata_fails_without_running_the_plugin() {
    let root = std::env::current_dir().unwrap();
    for (producer, path, message) in [
        (
            "other::Producer",
            root.join("output.g.dart"),
            "plugin `probe` declares an output for unclaimed producer `other::Producer`",
        ),
        (
            "one::Producer",
            PathBuf::from("output.g.dart"),
            "plugin `probe` declares a non-absolute generated library path",
        ),
    ] {
        let mut registry = PluginRegistry::new();
        registry
            .register(Box::new(Probe {
                producer,
                path,
                ..probe(&root)
            }))
            .unwrap();
        assert_eq!(
            registry
                .potential_generated_libraries(&root)
                .unwrap_err()
                .message,
            message
        );
    }
}

#[test]
fn shadowed_fallback_claim_cannot_plan_outputs() {
    let root = std::env::current_dir().unwrap();
    let mut registry = PluginRegistry::new();
    registry.register(Box::new(probe(&root))).unwrap();
    registry
        .register_symbols_only(Box::new(probe(&root)))
        .unwrap();
    assert_eq!(
        registry.potential_generated_libraries(&root).unwrap().len(),
        1
    );
    registry
        .register_symbols_only(Box::new(Probe {
            claim: "two::Producer",
            producer: "two::Producer",
            path: root.join("two.g.dart"),
        }))
        .unwrap();
    assert_eq!(
        registry.potential_generated_libraries(&root).unwrap(),
        [
            GeneratedLibrary {
                path: root.join("output.g.dart"),
                producer: SymbolId::new("one::Producer")
            },
            GeneratedLibrary {
                path: root.join("two.g.dart"),
                producer: SymbolId::new("two::Producer")
            },
        ]
    );
}
