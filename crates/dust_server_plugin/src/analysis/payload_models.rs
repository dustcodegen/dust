use dust_diagnostics::Diagnostic;
use dust_ir::{DartFileIr, TypeIr};
use dust_plugin_api::WorkspaceAnalysis;

use crate::plan::{BodyFormat, Extraction, ServerPlan};

/// Structured model capability facts retained across package boundaries.
mod facts;
pub(crate) use facts::collect_payload_model_workspace_analysis_ir;
use facts::{PayloadModelFact, model_facts};

#[cfg(test)]
mod tests;

/// Hydrates imported body behavior and validates generated client payloads.
pub(crate) fn hydrate_payload_models(
    file: &DartFileIr,
    plan: &mut ServerPlan<'_>,
    analysis: &WorkspaceAnalysis,
    require_client: bool,
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
            if !matches!(param.extraction, Extraction::Body(BodyFormat::Json)) {
                continue;
            }
            let Some(name) = param.value_type.name().filter(|name| name.contains('.')) else {
                continue;
            };
            match imported_fact(file, &facts, name) {
                Ok(Some(fact)) => {
                    if fact.deserializable {
                        param.serde_deserialized = true;
                    } else if !fact.deserialize_method {
                        diagnostics.push(Diagnostic::error(format!(
                            "imported body model `{name}` must derive Deserialize() or expose static deserialize"
                        )));
                    }
                    param.validated = fact.validatable;
                    if require_client && !(fact.serializable || fact.to_json) {
                        diagnostics.push(Diagnostic::error(format!(
                            "generated clients require body model `{name}` to derive Serialize() or expose toJson()"
                        )));
                    }
                }
                Ok(None) => {}
                Err(error) => diagnostics.push(error),
            }
        }
        if require_client
            && let Err(errors) =
                response_serde_types_from_facts(file, handler.callable.return_type(), &facts)
        {
            diagnostics.extend(errors);
        }
    }
    diagnostics
}

/// Returns response model names decoded through Dust's public deserializer.
pub(crate) fn response_serde_types(
    file: &DartFileIr,
    ty: &TypeIr,
    analysis: &WorkspaceAnalysis,
) -> Result<Vec<String>, Vec<Diagnostic>> {
    let facts = model_facts(analysis).map_err(|error| vec![error])?;
    response_serde_types_from_facts(file, ty, &facts)
}

/// Resolves one response shape against already decoded model facts.
fn response_serde_types_from_facts(
    file: &DartFileIr,
    ty: &TypeIr,
    facts: &[PayloadModelFact],
) -> Result<Vec<String>, Vec<Diagnostic>> {
    let mut serde_types = Vec::new();
    let mut diagnostics = Vec::new();
    collect_response_types(file, ty, facts, &mut serde_types, &mut diagnostics);
    serde_types.sort();
    serde_types.dedup();
    if diagnostics.is_empty() {
        Ok(serde_types)
    } else {
        Err(diagnostics)
    }
}

/// Recursively collects the payload models nested in a response shape.
fn collect_response_types(
    file: &DartFileIr,
    ty: &TypeIr,
    facts: &[PayloadModelFact],
    serde_types: &mut Vec<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(name) = ty.name() else {
        return;
    };
    let short = name.rsplit('.').next().unwrap_or(name);
    if matches!(short, "Future" | "Result" | "Option" | "List") && !ty.args().is_empty() {
        collect_response_types(file, &ty.args()[0], facts, serde_types, diagnostics);
        return;
    }
    if !name.contains('.') {
        return;
    }
    match imported_fact(file, facts, name) {
        Ok(Some(fact)) => {
            if !(fact.serializable || fact.to_json) {
                diagnostics.push(Diagnostic::error(format!(
                    "generated servers require response model `{name}` to derive Serialize() or expose toJson()"
                )));
            }
            if fact.deserializable {
                serde_types.push(name.to_owned());
            } else if !fact.from_json {
                diagnostics.push(Diagnostic::error(format!(
                    "generated clients require response model `{name}` to derive Deserialize() or expose a fromJson factory"
                )));
            }
        }
        Ok(None) => {}
        Err(error) => diagnostics.push(error),
    }
}

/// Finds the fact bound to one prefixed imported type.
fn imported_fact<'a>(
    file: &DartFileIr,
    facts: &'a [PayloadModelFact],
    qualified_name: &str,
) -> Result<Option<&'a PayloadModelFact>, Diagnostic> {
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
    let exact = facts
        .iter()
        .filter(|fact| fact.library == import.uri && fact.name == name)
        .collect::<Vec<_>>();
    if let [fact] = exact.as_slice() {
        return Ok(Some(fact));
    }
    let matches = facts
        .iter()
        .filter(|fact| fact.package == package && fact.name == name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [fact] => Ok(Some(fact)),
        [] => Ok(None),
        _ => Err(Diagnostic::error(format!(
            "payload model `{qualified_name}` is ambiguous; import its declaring library directly"
        ))),
    }
}
