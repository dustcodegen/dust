use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{DartFileIr, SpanIr, TypeIr};

use crate::plan::{BodyFormat, Extraction, ServerPlan};

/// Structured model capability facts retained across package boundaries.
mod facts;
pub(crate) use facts::collect_payload_model_workspace_analysis_ir;
#[cfg(test)]
use facts::{PayloadModelFact, model_facts};
/// One-pass lookup of local and imported payload contracts.
mod index;
#[cfg(test)]
use dust_plugin_api::WorkspaceAnalysis;
pub(crate) use index::PayloadCapabilityIndex;
#[cfg(test)]
use index::imported_fact;
use index::payload_capabilities;

#[cfg(test)]
mod tests;

/// Hydrates body behavior and validates server and client payload contracts.
pub(crate) fn hydrate_payload_models(
    file: &DartFileIr,
    plan: &mut ServerPlan<'_>,
    models: &PayloadCapabilityIndex,
    require_client: bool,
) -> Vec<Diagnostic> {
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
            let Some(name) = param.value_type.name() else {
                continue;
            };
            match payload_capabilities(file, models, name) {
                Ok(Some(capabilities)) => {
                    if capabilities.deserializable {
                        param.serde_deserialized = true;
                    } else {
                        diagnostics.push(payload_error(
                            param.param.span,
                            format!("body model `{name}` must derive Deserialize()"),
                        ));
                    }
                    param.validated = capabilities.validatable;
                    if require_client && !capabilities.serializable {
                        diagnostics.push(payload_error(
                            param.param.span,
                            format!(
                                "generated clients require body model `{name}` to derive Serialize()"
                            ),
                        ));
                    }
                }
                Ok(None) if param.value_type.is_named("Map") => {}
                Ok(None) => diagnostics.push(payload_error(
                    param.param.span,
                    format!(
                        "body model `{name}` must be declared in this library or imported with a prefix and derive Deserialize()"
                    ),
                )),
                Err(error) => diagnostics.push(label_payload_error(error, param.param.span)),
            }
        }
        if let Err(errors) = response_payload_types(
            file,
            handler.callable.return_type(),
            handler.callable.span(),
            models,
            require_client,
        ) {
            diagnostics.extend(errors);
        }
    }
    diagnostics
}

/// Returns response model names decoded through Dust's public deserializer.
pub(crate) fn response_serde_types(
    file: &DartFileIr,
    ty: &TypeIr,
    span: SpanIr,
    models: &PayloadCapabilityIndex,
) -> Result<Vec<String>, Vec<Diagnostic>> {
    response_payload_types(file, ty, span, models, true)
}

/// Validates response contracts and returns client deserializer model names.
fn response_payload_types(
    file: &DartFileIr,
    ty: &TypeIr,
    span: SpanIr,
    models: &PayloadCapabilityIndex,
    require_deserialize: bool,
) -> Result<Vec<String>, Vec<Diagnostic>> {
    let mut serde_types = Vec::new();
    let mut diagnostics = Vec::new();
    collect_response_types(
        file,
        ty,
        span,
        models,
        require_deserialize,
        &mut serde_types,
        &mut diagnostics,
    );
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
    span: SpanIr,
    models: &PayloadCapabilityIndex,
    require_deserialize: bool,
    serde_types: &mut Vec<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(name) = ty.name() else {
        return;
    };
    let short = name.rsplit('.').next().unwrap_or(name);
    if matches!(short, "Future" | "Result" | "Option" | "List") && !ty.args().is_empty() {
        collect_response_types(
            file,
            &ty.args()[0],
            span,
            models,
            require_deserialize,
            serde_types,
            diagnostics,
        );
        return;
    }
    if is_native_response(name) {
        return;
    }
    match payload_capabilities(file, models, name) {
        Ok(Some(capabilities)) => {
            if !capabilities.serializable {
                diagnostics.push(payload_error(
                    span,
                    format!(
                        "generated servers require response model `{name}` to derive Serialize()"
                    ),
                ));
            }
            if capabilities.deserializable {
                serde_types.push(name.to_owned());
            } else if require_deserialize {
                diagnostics.push(payload_error(
                    span,
                    format!(
                        "generated clients require response model `{name}` to derive Deserialize()"
                    ),
                ));
            }
        }
        Ok(None) => diagnostics.push(payload_error(
            span,
            format!(
                "response model `{name}` must be declared in this library or imported with a prefix and derive Serialize()"
            ),
        )),
        Err(error) => diagnostics.push(label_payload_error(error, span)),
    }
}

/// Returns whether a response type is encoded without a model contract.
fn is_native_response(name: &str) -> bool {
    matches!(
        name,
        "String"
            | "int"
            | "double"
            | "num"
            | "bool"
            | "BigInt"
            | "DateTime"
            | "Uri"
            | "Object"
            | "Map"
            | "Unit"
            | "void"
            | "Response"
            | "ResponseBody"
            | "Uint8List"
            | "Future"
            | "Result"
            | "Option"
            | "List"
            | "Stream"
    )
}

/// Creates a source-labelled payload diagnostic.
fn payload_error(span: SpanIr, message: String) -> Diagnostic {
    Diagnostic::error(message).with_label(SourceLabel::new(
        span.file_id,
        span.range,
        "payload capability is required here",
    ))
}

/// Attaches a source location to an import-resolution diagnostic.
fn label_payload_error(error: Diagnostic, span: SpanIr) -> Diagnostic {
    error.with_label(SourceLabel::new(
        span.file_id,
        span.range,
        "payload type is used here",
    ))
}
