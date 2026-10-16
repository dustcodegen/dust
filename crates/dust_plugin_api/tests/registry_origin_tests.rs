//! Origin metadata must follow effective claims and fail before registry mutation.

use dust_diagnostics::Diagnostic;
use dust_ir::DartFileIr;
use dust_plugin_api::{
    AnnotationOrigin, DustPlugin, PluginContext, PluginContribution, PluginRegistry,
};

/// Minimal plugin: metadata is the only behavior under test.
struct Probe {
    /// Stable registration name.
    name: &'static str,
    /// Claimed canonical traits, independent of config roles.
    traits: &'static [&'static str],
    /// Claimed canonical configs.
    configs: &'static [&'static str],
    /// Explicit declaration origins, never inferred from canonical spelling.
    origins: &'static [AnnotationOrigin],
}

impl DustPlugin for Probe {
    fn plugin_name(&self) -> &'static str {
        self.name
    }
    fn claimed_configs(&self) -> &'static [&'static str] {
        self.configs
    }
    fn claimed_traits(&self) -> &'static [&'static str] {
        self.traits
    }
    fn annotation_origins(&self) -> &'static [AnnotationOrigin] {
        self.origins
    }
    fn validate(&self, _: &DartFileIr) -> Vec<Diagnostic> {
        vec![]
    }
    fn generate(&self, _: &DartFileIr, _: &PluginContext<'_>) -> Vec<PluginContribution> {
        vec![]
    }
}

/// These share an intentional legacy canonical ID, not a Dart declaration.
const HTTP_QUERY: AnnotationOrigin = AnnotationOrigin::new(
    "dust_dart::Query",
    "package:dust_dart/src/http/http_client_annotations.dart",
    "Query",
);
/// DB mode must keep its selected declaration when HTTP claims are fallback-only.
const DB_QUERY: AnnotationOrigin = AnnotationOrigin::new(
    "dust_dart::Query",
    "package:dust_dart/src/db/annotations.dart",
    "Query",
);

/// Stable slices keep descriptors alive for the plugin's static metadata contract.
const HTTP_ORIGINS: &[AnnotationOrigin] = &[HTTP_QUERY];
/// The alternative profile owns the same semantic symbol through another library.
const DB_ORIGINS: &[AnnotationOrigin] = &[DB_QUERY];

/// A conflict can exist within a single plugin's metadata.
const CONFLICTING: &[AnnotationOrigin] = &[HTTP_QUERY, DB_QUERY];
/// An origin cannot grant ownership of an unclaimed semantic symbol.
const UNCLAIMED: &[AnnotationOrigin] = &[AnnotationOrigin::new(
    "other::GET",
    "package:other/a.dart",
    "GET",
)];

#[test]
fn symbols_only_origins_do_not_override_selected_claims() {
    for (selected, fallback) in [(HTTP_ORIGINS, DB_ORIGINS), (DB_ORIGINS, HTTP_ORIGINS)] {
        let mut registry = PluginRegistry::new();
        registry
            .register(Box::new(Probe {
                name: "selected",
                traits: &[],
                configs: &["dust_dart::Query"],
                origins: selected,
            }))
            .unwrap();
        registry
            .register_symbols_only(Box::new(Probe {
                name: "fallback",
                traits: &[],
                configs: &["dust_dart::Query"],
                origins: fallback,
            }))
            .unwrap();
        assert_eq!(registry.annotation_origins(), selected);
        assert_eq!(
            registry
                .claimed_config_symbols()
                .iter()
                .map(|s| s.0.as_str())
                .collect::<Vec<_>>(),
            ["dust_dart::Query"]
        );
    }
}

#[test]
fn conflicting_duplicate_and_unclaimed_metadata_fail_before_any_claim_is_retained() {
    for (origins, expected) in [
        (
            CONFLICTING,
            "conflicting declaration origins for symbol `dust_dart::Query`",
        ),
        (
            UNCLAIMED,
            "plugin `bad` declares an origin for unclaimed symbol `other::GET`",
        ),
    ] {
        let mut registry = PluginRegistry::new();
        let error = registry
            .register(Box::new(Probe {
                name: "bad",
                traits: &[],
                configs: &["dust_dart::Query"],
                origins,
            }))
            .unwrap_err();
        assert_eq!(error.message, expected);
        assert_eq!(registry.plugin_names(), Vec::<&str>::new());
        assert_eq!(registry.annotation_origins(), []);
        assert_eq!(registry.claimed_config_symbols(), []);
        registry
            .register(Box::new(Probe {
                name: "good",
                traits: &[],
                configs: &["dust_dart::Query"],
                origins: &[HTTP_QUERY],
            }))
            .unwrap();
        assert_eq!(registry.annotation_origins(), [HTTP_QUERY]);
    }
}

#[test]
fn repeated_identical_origins_are_deduplicated() {
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(Probe {
            name: "same",
            traits: &[],
            configs: &["dust_dart::Query"],
            origins: &[HTTP_QUERY, HTTP_QUERY],
        }))
        .unwrap();
    assert_eq!(registry.annotation_origins(), [HTTP_QUERY]);
}

#[test]
fn both_roles_share_one_origin_and_cross_plugin_conflicts_leave_claims_intact() {
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(Probe {
            name: "trait",
            traits: &["dust_dart::Query"],
            configs: &[],
            origins: HTTP_ORIGINS,
        }))
        .unwrap();
    let conflict = registry
        .register(Box::new(Probe {
            name: "wrong-config",
            traits: &[],
            configs: &["dust_dart::Query"],
            origins: DB_ORIGINS,
        }))
        .unwrap_err();
    assert_eq!(
        conflict.message,
        "conflicting declaration origins for symbol `dust_dart::Query`"
    );
    assert_eq!(registry.plugin_names(), ["trait"]);
    assert_eq!(registry.claimed_config_symbols(), []);
    registry
        .register(Box::new(Probe {
            name: "config",
            traits: &[],
            configs: &["dust_dart::Query"],
            origins: HTTP_ORIGINS,
        }))
        .unwrap();
    registry
        .register_symbols_only(Box::new(Probe {
            name: "fallback",
            traits: &["dust_dart::Query"],
            configs: &["dust_dart::Query"],
            origins: DB_ORIGINS,
        }))
        .unwrap();
    assert_eq!(registry.annotation_origins(), [HTTP_QUERY]);
    assert_eq!(registry.claimed_trait_symbols()[0].0, "dust_dart::Query");
    assert_eq!(registry.claimed_config_symbols()[0].0, "dust_dart::Query");
}
