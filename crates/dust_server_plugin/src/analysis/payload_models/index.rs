use std::collections::BTreeMap;

use dust_diagnostics::Diagnostic;
use dust_ir::DartFileIr;
use dust_plugin_api::WorkspaceAnalysis;

use super::facts::{PayloadModelFact, has_trait, model_facts};

/// Indexed payload facts decoded once for one server library.
pub(crate) struct PayloadCapabilityIndex {
    /// Decoded capability facts addressed by the lookup indexes.
    facts: Vec<PayloadModelFact>,
    /// Fact indexes grouped by canonical library URI and model name.
    by_library: BTreeMap<String, BTreeMap<String, Vec<usize>>>,
    /// Fact indexes grouped by package and model name for re-export lookup.
    by_package: BTreeMap<String, BTreeMap<String, Vec<usize>>>,
}

/// Dust contracts available for one payload model.
#[derive(Clone, Copy)]
pub(super) struct PayloadCapabilities {
    /// Whether Dust generates a `Deserializer` implementation.
    pub(super) deserializable: bool,
    /// Whether Dust generates the `Serializable` model contract.
    pub(super) serializable: bool,
    /// Whether Dust generates the `Validatable` model contract.
    pub(super) validatable: bool,
}

impl PayloadCapabilityIndex {
    /// Decodes and indexes all payload facts in workspace analysis.
    pub(crate) fn from_analysis(analysis: &WorkspaceAnalysis) -> Result<Self, Diagnostic> {
        let facts = model_facts(analysis)?;
        let mut by_library = BTreeMap::<String, BTreeMap<String, Vec<usize>>>::new();
        let mut by_package = BTreeMap::<String, BTreeMap<String, Vec<usize>>>::new();
        for (index, fact) in facts.iter().enumerate() {
            by_library
                .entry(fact.library.clone())
                .or_default()
                .entry(fact.name.clone())
                .or_default()
                .push(index);
            by_package
                .entry(fact.package.clone())
                .or_default()
                .entry(fact.name.clone())
                .or_default()
                .push(index);
        }
        Ok(Self {
            facts,
            by_library,
            by_package,
        })
    }
}

/// Resolves the declared Dust contracts for a local or imported model.
pub(super) fn payload_capabilities(
    file: &DartFileIr,
    models: &PayloadCapabilityIndex,
    name: &str,
) -> Result<Option<PayloadCapabilities>, Diagnostic> {
    if name.contains('.') {
        return imported_fact(file, models, name).map(|fact| Some(fact.into()));
    }
    Ok(file
        .classes
        .iter()
        .find(|class| class.name == name)
        .map(|class| PayloadCapabilities {
            deserializable: has_trait(class, "dust_dart::Deserialize"),
            serializable: has_trait(class, "dust_dart::Serialize"),
            validatable: has_trait(class, "dust_dart::Validate"),
        }))
}

/// Finds the fact bound to one prefixed imported type.
pub(super) fn imported_fact<'a>(
    file: &DartFileIr,
    models: &'a PayloadCapabilityIndex,
    qualified_name: &str,
) -> Result<&'a PayloadModelFact, Diagnostic> {
    let (prefix, name) = qualified_name.split_once('.').ok_or_else(|| {
        Diagnostic::error(format!(
            "payload model `{qualified_name}` requires a prefixed import"
        ))
    })?;
    let import = file
        .import_directives
        .iter()
        .find(|value| value.prefix.as_deref() == Some(prefix))
        .ok_or_else(|| {
            Diagnostic::error(format!("payload model prefix `{prefix}` is not imported"))
        })?;
    let Some(package) = import
        .uri
        .strip_prefix("package:")
        .and_then(|value| value.split('/').next())
    else {
        return Err(Diagnostic::error(
            "imported payload models require a prefixed package import",
        ));
    };
    if let Some(exact) = models
        .by_library
        .get(&import.uri)
        .and_then(|map| map.get(name))
    {
        if let [index] = exact.as_slice() {
            return Ok(&models.facts[*index]);
        }
        return Err(Diagnostic::error(format!(
            "payload model `{qualified_name}` has conflicting facts"
        )));
    }
    match models
        .by_package
        .get(package)
        .and_then(|map| map.get(name))
        .map(Vec::as_slice)
        .unwrap_or_default()
    {
        [index] => Ok(&models.facts[*index]),
        [] => Err(Diagnostic::error(format!(
            "payload model `{qualified_name}` must derive Dust Serde contracts in its source package"
        ))),
        _ => Err(Diagnostic::error(format!(
            "payload model `{qualified_name}` is ambiguous; import its declaring library directly"
        ))),
    }
}

impl From<&PayloadModelFact> for PayloadCapabilities {
    fn from(fact: &PayloadModelFact) -> Self {
        Self {
            deserializable: fact.deserializable,
            serializable: fact.serializable,
            validatable: fact.validatable,
        }
    }
}
