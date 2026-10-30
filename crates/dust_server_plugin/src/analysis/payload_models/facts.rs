use std::path::Path;

use dust_diagnostics::Diagnostic;
use dust_ir::{ClassIr, DartFileIr};
use dust_plugin_api::{WorkspaceAnalysis, WorkspaceAnalysisBuilder};
use serde::{Deserialize, Serialize};

/// Versioned workspace-analysis bucket for payload model capabilities.
pub(super) const PAYLOAD_MODELS_KEY: &str = "dust_server.payload_models.v1";

/// Public JSON and validation capabilities for one reachable model.
#[derive(Deserialize, Serialize)]
pub(super) struct PayloadModelFact {
    /// Package that owns the declaration.
    pub(super) package: String,
    /// Canonical library URI that owns the declaration.
    pub(super) library: String,
    /// Public class name.
    pub(super) name: String,
    /// Whether the model derives Dust `Deserialize`.
    pub(super) deserializable: bool,
    /// Whether the model derives Dust `Serialize`.
    pub(super) serializable: bool,
    /// Whether the model derives Dust `Validate`.
    pub(super) validatable: bool,
}

/// Collects public payload capabilities from reachable dependency libraries.
pub(crate) fn collect_payload_model_workspace_analysis_ir(
    file: &DartFileIr,
    analysis: &mut WorkspaceAnalysisBuilder,
) {
    let Some(library) = library_uri(file) else {
        return;
    };
    for class in &file.classes {
        let deserializable = has_trait(class, "dust_dart::Deserialize");
        let serializable = has_trait(class, "dust_dart::Serialize");
        let validatable = has_trait(class, "dust_dart::Validate");
        let fact = PayloadModelFact {
            package: file.package_name.clone(),
            library: library.clone(),
            name: class.name.clone(),
            deserializable,
            serializable,
            validatable,
        };
        if let Ok(value) = serde_json::to_string(&fact) {
            analysis.add_string_set_value(PAYLOAD_MODELS_KEY, value);
        }
    }
}

/// Decodes every retained payload fact or returns one deterministic error.
pub(super) fn model_facts(
    analysis: &WorkspaceAnalysis,
) -> Result<Vec<PayloadModelFact>, Diagnostic> {
    analysis
        .string_set(PAYLOAD_MODELS_KEY)
        .unwrap_or_default()
        .iter()
        .map(|value| {
            serde_json::from_str(value).map_err(|error| {
                Diagnostic::error(format!("invalid {PAYLOAD_MODELS_KEY} model fact: {error}"))
            })
        })
        .collect()
}

/// Builds the canonical package URI for one library under `lib`.
fn library_uri(file: &DartFileIr) -> Option<String> {
    let source = Path::new(&file.source_path);
    let relative = source
        .strip_prefix(Path::new(&file.package_root).join("lib"))
        .or_else(|_| source.strip_prefix("lib"))
        .ok()?;
    Some(format!(
        "package:{}/{}",
        file.package_name,
        relative.to_string_lossy().replace('\\', "/")
    ))
}

/// Returns whether a class has one resolved Dust derive trait.
pub(super) fn has_trait(class: &ClassIr, symbol: &str) -> bool {
    class.traits.iter().any(|value| value.symbol.0 == symbol)
}
