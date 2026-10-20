use std::collections::BTreeMap;

use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{AnnotationValueIr, ConfigApplicationIr, SpanIr, SymbolId, TraitApplicationIr};
use dust_parser_dart::{
    ParsedAnnotation, ParsedConstructorSurface, ParsedDirective, ParsedFieldSurface,
    ParsedMethodParamSurface, ParsedMethodSurface,
};
use dust_text::{FileId, TextRange};

use crate::{
    ResolvedConstructor, ResolvedField, ResolvedMethod, ResolvedMethodParam, SymbolKind,
    annotations::annotation_argument_values,
    binding::{Application, BindingContext},
    serde::normalize_field_serde,
};

/// Resolving an annotation and its trait application to catalog symbols.
mod annotations;
pub(crate) use self::annotations::*;

/// Positional and named annotation argument values.
type AnnotationArguments = (Vec<AnnotationValueIr>, BTreeMap<String, AnnotationValueIr>);

/// One parsed annotation matched to a registered symbol.
struct ResolvedAnnotationSymbol {
    /// Annotation source span.
    span: TextRange,
    /// Matched symbol kind.
    kind: SymbolKind,
    /// Fully qualified symbol id.
    symbol: SymbolId,
    /// Original argument source when available.
    arguments_source: Option<String>,
    /// Lowered annotation arguments.
    arguments: AnnotationArguments,
}

/// Resolves annotations and parameters for one method.
pub(crate) fn resolve_method(
    file_id: FileId,
    method: &ParsedMethodSurface,
    context: &BindingContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedMethod {
    let mut traits = Vec::new();
    let mut configs = Vec::new();

    for annotation in &method.annotations {
        let Application::Symbol(resolved) = context.application(file_id, annotation, diagnostics)
        else {
            continue;
        };

        push_resolved_symbol(
            file_id,
            ResolvedAnnotationSymbol {
                span: annotation.span,
                kind: resolved.kind,
                symbol: resolved.symbol.clone(),
                arguments_source: annotation.arguments_source.clone(),
                arguments: annotation_argument_values(file_id, annotation),
            },
            &mut traits,
            &mut configs,
        );
    }

    let params = method
        .params
        .iter()
        .map(|param| resolve_method_param(file_id, param, context, diagnostics))
        .collect();

    ResolvedMethod {
        surface: method.clone(),
        span: SpanIr::new(file_id, method.span),
        traits,
        configs,
        params,
    }
}

/// Resolves annotations for one method parameter.
pub(crate) fn resolve_method_param(
    file_id: FileId,
    param: &ParsedMethodParamSurface,
    context: &BindingContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedMethodParam {
    let mut traits = Vec::new();
    let mut configs = Vec::new();

    if let Some(formal) = &param.function_formal {
        for annotation in formal.annotations() {
            let _ = context.annotation_ir(file_id, annotation, diagnostics);
        }
    }

    for annotation in &param.annotations {
        let Application::Symbol(resolved) = context.application(file_id, annotation, diagnostics)
        else {
            continue;
        };

        push_resolved_symbol(
            file_id,
            ResolvedAnnotationSymbol {
                span: annotation.span,
                kind: resolved.kind,
                symbol: resolved.symbol.clone(),
                arguments_source: annotation.arguments_source.clone(),
                arguments: annotation_argument_values(file_id, annotation),
            },
            &mut traits,
            &mut configs,
        );
    }

    ResolvedMethodParam {
        surface: param.clone(),
        span: SpanIr::new(file_id, param.span),
        traits,
        configs,
    }
}

/// Resolves declaration-level Dust trait and config annotations.
pub(crate) fn resolve_declaration_annotations(
    file_id: FileId,
    annotations: &[ParsedAnnotation],
    context: &BindingContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    traits: &mut Vec<TraitApplicationIr>,
    configs: &mut Vec<ConfigApplicationIr>,
) {
    for annotation in annotations {
        let application = context.application(file_id, annotation, diagnostics);
        if let Application::Group(mut group_traits) = application {
            if context.namespace.is_none() {
                let (values, _) = annotation_argument_values(file_id, annotation);
                group_traits = context.group_traits(file_id, annotation, &values, diagnostics);
            }
            traits.extend(group_traits);
        } else if let Application::Symbol(resolved) = application {
            push_resolved_symbol(
                file_id,
                ResolvedAnnotationSymbol {
                    span: annotation.span,
                    kind: resolved.kind,
                    symbol: resolved.symbol.clone(),
                    arguments_source: annotation.arguments_source.clone(),
                    arguments: annotation_argument_values(file_id, annotation),
                },
                traits,
                configs,
            );
        }
    }
}

/// Returns constructor names from structured `Derive` positional values.
pub(crate) fn derive_member_names(values: &[AnnotationValueIr]) -> Vec<&dust_ir::NameIr> {
    values
        .iter()
        .flat_map(|value| match value {
            AnnotationValueIr::List(items) => items.as_slice(),
            value => std::slice::from_ref(value),
        })
        .filter_map(|value| match value {
            AnnotationValueIr::Constructor { name, .. } => Some(name),
            _ => None,
        })
        .collect()
}

/// Resolves constructor-level Dust config annotations.
pub(crate) fn resolve_constructor(
    file_id: FileId,
    constructor: &ParsedConstructorSurface,
    context: &BindingContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedConstructor {
    ResolvedConstructor {
        surface: constructor.clone(),
        redirected_target: None,
        configs: resolve_configs(
            file_id,
            &constructor.annotations,
            context,
            diagnostics,
            "constructor",
        ),
    }
}

/// Resolves field-level Dust config annotations.
pub(crate) fn resolve_field(
    file_id: FileId,
    field: &ParsedFieldSurface,
    context: &BindingContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedField {
    let configs = resolve_configs(file_id, &field.annotations, context, diagnostics, "field");

    let serde = normalize_field_serde(&field.name, &configs, diagnostics);
    ResolvedField {
        name: field.name.clone(),
        type_source: field.type_source.clone(),
        parsed_type: field.parsed_type.clone(),
        function_formal: field.function_formal.clone(),
        has_default: field.has_default,
        span: SpanIr::new(file_id, field.span),
        configs,
        serde,
    }
}

#[cfg(test)]
#[path = "resolve_support/tests.rs"]
mod tests;
