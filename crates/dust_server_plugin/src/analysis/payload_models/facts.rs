use std::path::Path;

use dust_diagnostics::Diagnostic;
use dust_ir::{ClassIr, DartFileIr, ParamKind, TypeIr};
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
    /// Whether a compatible static `deserialize` method exists.
    pub(super) deserialize_method: bool,
    /// Whether a compatible `fromJson` factory exists.
    pub(super) from_json: bool,
    /// Whether a compatible instance `toJson` method exists.
    pub(super) to_json: bool,
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
        let deserialize_method = has_deserialize_method(class);
        let from_json = has_from_json_factory(class);
        let to_json = has_to_json_method(class);
        if !(deserializable
            || serializable
            || validatable
            || deserialize_method
            || from_json
            || to_json)
        {
            continue;
        }
        let fact = PayloadModelFact {
            package: file.package_name.clone(),
            library: library.clone(),
            name: class.name.clone(),
            deserializable,
            serializable,
            validatable,
            deserialize_method,
            from_json,
            to_json,
        };
        let value = serde_json::to_string(&fact)
            .expect("payload model facts contain only JSON-safe values");
        analysis.add_string_set_value(PAYLOAD_MODELS_KEY, value);
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
fn has_trait(class: &ClassIr, symbol: &str) -> bool {
    class.traits.iter().any(|value| value.symbol.0 == symbol)
}

/// Returns whether a class exposes a compatible static body decoder.
fn has_deserialize_method(class: &ClassIr) -> bool {
    class.methods.iter().any(|method| {
        method.name == "deserialize"
            && method.is_static
            && method.params.len() == 1
            && is_json_map(&method.params[0].ty)
    })
}

/// Returns whether a class exposes a compatible JSON encoder.
fn has_to_json_method(class: &ClassIr) -> bool {
    class.methods.iter().any(|method| {
        method.name == "toJson"
            && !method.is_static
            && method.params.is_empty()
            && is_json_map(&method.return_type)
    })
}

/// Returns whether a class exposes a compatible JSON factory.
fn has_from_json_factory(class: &ClassIr) -> bool {
    class.constructors.iter().any(|constructor| {
        constructor.name.as_deref() == Some("fromJson")
            && constructor.is_factory
            && constructor.params.len() == 1
            && constructor.params[0].kind == ParamKind::Positional
            && is_json_map(&constructor.params[0].ty)
    })
}

/// Returns whether a type is a string-keyed JSON object map.
fn is_json_map(ty: &TypeIr) -> bool {
    ty.is_named("Map") && ty.args().len() == 2 && ty.args()[0].is_named("String")
}
