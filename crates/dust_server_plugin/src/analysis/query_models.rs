use std::path::Path;

use dust_dart_emit::DYNAMIC_TYPES;
use dust_diagnostics::Diagnostic;
use dust_ir::{ClassIr, DartFileIr, TypeIr};
use dust_plugin_api::{WorkspaceAnalysis, WorkspaceAnalysisBuilder};
use serde::{Deserialize, Serialize};

use crate::plan::{
    Extraction, QueryConversionKind, QueryConversionPlan, QueryModelPlan, ServerPlan,
    query_model_plan_for_class,
};

#[cfg(test)]
mod tests;

/// Versioned workspace-analysis bucket for Serde query models.
const QUERY_MODELS_ANALYSIS_KEY: &str = "dust_server.query_models.v1";

/// Serializable shape of one query model.
#[derive(Deserialize, Serialize)]
struct QueryModelFact {
    /// Package that owns the model.
    package: String,
    /// Canonical library URI that declares the model.
    library: String,
    /// Declared class name.
    name: String,
    /// Whether generated clients may encode the model.
    serializable: bool,
    /// Whether generated servers validate the decoded model.
    validatable: bool,
    /// Planning errors retained for the consuming route.
    errors: Vec<String>,
    /// Query-string conversions applied before Serde decoding.
    conversions: Vec<QueryConversionFact>,
}

/// Serializable form of one query-string conversion.
#[derive(Deserialize, Serialize)]
struct QueryConversionFact {
    /// Canonical serialized field name.
    key: String,
    /// Accepted decode-only aliases.
    aliases: Vec<String>,
    /// Required scalar or list conversion.
    kind: QueryConversionFactKind,
}

/// JSON-safe representation of a query conversion kind.
#[derive(Deserialize, Serialize)]
enum QueryConversionFactKind {
    /// One scalar value rendered as its Dart type.
    Scalar(String),
    /// Repeated string values copied without coercion.
    StringList,
    /// Repeated values coerced to the rendered scalar type.
    ScalarList(String),
}

/// Collects query-model shapes from local or imported annotated libraries.
pub(crate) fn collect_query_model_workspace_analysis_ir(
    file: &DartFileIr,
    analysis: &mut WorkspaceAnalysisBuilder,
) {
    let Some(library) = library_uri(file) else {
        return;
    };
    for class in &file.classes {
        if !has_trait(class, "dust_dart::Deserialize") {
            continue;
        }
        let mut diagnostics = Vec::new();
        let plan = query_model_plan_for_class(class, &file.enums, &mut diagnostics);
        let fact = QueryModelFact {
            package: file.package_name.clone(),
            library: library.clone(),
            name: class.name.clone(),
            serializable: has_trait(class, "dust_dart::Serialize"),
            validatable: has_trait(class, "dust_dart::Validate"),
            errors: diagnostics.into_iter().map(|value| value.message).collect(),
            conversions: plan
                .conversions
                .into_iter()
                .map(QueryConversionFact::from)
                .collect(),
        };
        if let Ok(value) = serde_json::to_string(&fact) {
            analysis.add_string_set_value(QUERY_MODELS_ANALYSIS_KEY, value);
        }
    }
}

/// Resolves imported query models and enforces client serialization support.
pub(crate) fn hydrate_query_models(
    file: &DartFileIr,
    plan: &mut ServerPlan<'_>,
    analysis: &WorkspaceAnalysis,
    require_serialize: bool,
) -> Vec<Diagnostic> {
    let facts = match model_facts(analysis) {
        Ok(value) => value,
        Err(error) => return vec![error],
    };
    let mut diagnostics = Vec::new();
    for handler in plan
        .controllers
        .iter_mut()
        .flat_map(|value| value.handlers.iter_mut())
        .chain(plan.functions.iter_mut())
    {
        for param in &mut handler.params {
            let Extraction::QueryModel(query) = &mut param.extraction else {
                continue;
            };
            let Some(name) = param.value_type.name() else {
                continue;
            };
            if let Some((prefix, short)) = name.split_once('.') {
                match imported_fact(file, &facts, prefix, short) {
                    Ok(fact) => {
                        diagnostics.extend(fact.errors.iter().map(|message| {
                            Diagnostic::error(format!(
                                "query model `{name}` is not supported: {message}"
                            ))
                        }));
                        if require_serialize && !fact.serializable {
                            diagnostics.push(Diagnostic::error(format!(
                                "generated clients require query model `{name}` to derive Serialize()"
                            )));
                        }
                        *query = fact.to_plan();
                        param.validated = fact.validatable;
                        param.serde_deserialized = false;
                    }
                    Err(error) => diagnostics.push(error),
                }
            } else if require_serialize
                && file
                    .classes
                    .iter()
                    .find(|class| class.name == name)
                    .is_some_and(|class| !has_trait(class, "dust_dart::Serialize"))
            {
                diagnostics.push(Diagnostic::error(format!(
                    "generated clients require query model `{name}` to derive Serialize()"
                )));
            }
        }
    }
    diagnostics
}

/// Decodes every retained query-model fact.
fn model_facts(analysis: &WorkspaceAnalysis) -> Result<Vec<QueryModelFact>, Diagnostic> {
    analysis
        .string_set(QUERY_MODELS_ANALYSIS_KEY)
        .unwrap_or_default()
        .iter()
        .map(|value| {
            serde_json::from_str(value).map_err(|error| {
                Diagnostic::error(format!(
                    "invalid {QUERY_MODELS_ANALYSIS_KEY} model fact: {error}"
                ))
            })
        })
        .collect()
}

/// Resolves a prefixed model against its direct library or unique package export.
fn imported_fact<'a>(
    file: &DartFileIr,
    facts: &'a [QueryModelFact],
    prefix: &str,
    name: &str,
) -> Result<&'a QueryModelFact, Diagnostic> {
    let import = file
        .import_directives
        .iter()
        .find(|value| value.prefix.as_deref() == Some(prefix))
        .ok_or_else(|| {
            Diagnostic::error(format!("query model prefix `{prefix}` is not imported"))
        })?;
    let package = import
        .uri
        .strip_prefix("package:")
        .and_then(|value| value.split('/').next())
        .ok_or_else(|| {
            Diagnostic::error("imported query models require a prefixed package import")
        })?;
    let exact = facts
        .iter()
        .filter(|fact| fact.library == import.uri && fact.name == name)
        .collect::<Vec<_>>();
    if let [fact] = exact.as_slice() {
        return Ok(fact);
    }
    let package_matches = facts
        .iter()
        .filter(|fact| fact.package == package && fact.name == name)
        .collect::<Vec<_>>();
    match package_matches.as_slice() {
        [fact] => Ok(fact),
        [] => Err(Diagnostic::error(format!(
            "query model `{prefix}.{name}` must derive Deserialize() in an imported source package"
        ))),
        _ => Err(Diagnostic::error(format!(
            "query model `{prefix}.{name}` is ambiguous; import its declaring library directly"
        ))),
    }
}

/// Computes the canonical package URI for one lowered library.
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

/// Returns whether the class derives the requested trait.
fn has_trait(class: &ClassIr, symbol: &str) -> bool {
    class.traits.iter().any(|value| value.symbol.0 == symbol)
}

impl QueryModelFact {
    /// Rehydrates query conversions for the consuming route plan.
    fn to_plan(&self) -> QueryModelPlan {
        QueryModelPlan {
            conversions: self
                .conversions
                .iter()
                .map(QueryConversionPlan::from)
                .collect(),
        }
    }
}

impl From<QueryConversionPlan> for QueryConversionFact {
    fn from(value: QueryConversionPlan) -> Self {
        Self {
            key: value.key,
            aliases: value.aliases,
            kind: match value.kind {
                QueryConversionKind::Scalar(ty) => {
                    QueryConversionFactKind::Scalar(DYNAMIC_TYPES.render(&ty))
                }
                QueryConversionKind::StringList => QueryConversionFactKind::StringList,
                QueryConversionKind::ScalarList(ty) => {
                    QueryConversionFactKind::ScalarList(DYNAMIC_TYPES.render(&ty))
                }
            },
        }
    }
}

impl From<&QueryConversionFact> for QueryConversionPlan {
    fn from(value: &QueryConversionFact) -> Self {
        let scalar = |name: &str| {
            let (name, nullable) = name
                .strip_suffix('?')
                .map_or((name, false), |value| (value, true));
            let ty = match name {
                "int" => TypeIr::int(),
                "double" => TypeIr::double(),
                "num" => TypeIr::num(),
                "bool" => TypeIr::bool(),
                _ => TypeIr::unknown(),
            };
            if nullable { ty.nullable() } else { ty }
        };
        Self {
            key: value.key.clone(),
            aliases: value.aliases.clone(),
            kind: match &value.kind {
                QueryConversionFactKind::Scalar(ty) => QueryConversionKind::Scalar(scalar(ty)),
                QueryConversionFactKind::StringList => QueryConversionKind::StringList,
                QueryConversionFactKind::ScalarList(ty) => {
                    QueryConversionKind::ScalarList(scalar(ty))
                }
            },
        }
    }
}
