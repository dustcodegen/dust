use std::{collections::BTreeSet, path::Path};

use dust_diagnostics::Diagnostic;
use dust_ir::{ConfigApplicationIr, DartFileIr, SpanIr};

use super::{
    model::{ControllerPlan, HandlerPlan, VERBS},
    values::error,
};

/// Library helper tests.
#[cfg(test)]
mod tests;

/// Requires the generated part before emitting handler code.
pub(super) fn validate_part(file: &DartFileIr, diagnostics: &mut Vec<Diagnostic>) {
    let expected = Path::new(&file.output_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&file.output_path);
    if !file.part_directives.iter().any(|part| part.uri == expected) {
        diagnostics.push(error(
            file.span,
            "missing generated `part` directive for server handlers",
            format!("add `part '{expected}';`"),
        ));
    }
}

/// Rejects duplicate method-path pairs within one generated route group.
pub(super) fn validate_unique_routes(
    controllers: &[ControllerPlan<'_>],
    functions: &[HandlerPlan<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for handlers in controllers
        .iter()
        .map(|value| value.handlers.as_slice())
        .chain(std::iter::once(functions))
    {
        let mut seen = BTreeSet::new();
        for handler in handlers {
            if !seen.insert((handler.method, handler.path.as_str())) {
                diagnostics.push(error(
                    handler.callable.span(),
                    "duplicate server route",
                    "change the HTTP method or path",
                ));
            }
        }
    }
}

/// Rejects source and generated declarations that share an emitted name.
pub(super) fn validate_generated_names(
    file: &DartFileIr,
    controllers: &[ControllerPlan<'_>],
    functions: &[HandlerPlan<'_>],
    function_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for controller in controllers {
        let source_names = controller
            .class
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .chain(
                controller
                    .class
                    .methods
                    .iter()
                    .map(|method| method.name.as_str()),
            )
            .collect::<BTreeSet<_>>();
        if source_names.contains("routes") {
            diagnostics.push(error(
                controller.class.span,
                "handwritten declaration conflicts with generated controller member `routes`",
                "rename the handwritten member",
            ));
        }
        validate_handler_names(&controller.handlers, &source_names, "member", diagnostics);
    }

    if !functions.is_empty()
        && let Some(span) = top_level_declaration_span(file, function_name)
    {
        diagnostics.push(error(
            span,
            format!(
                "handwritten declaration conflicts with generated route factory `{function_name}`"
            ),
            "rename the declaration or configure a different @Routes name",
        ));
    }
    let source_names = top_level_declarations(file)
        .map(|(name, _)| name)
        .collect::<BTreeSet<_>>();
    validate_handler_names(functions, &source_names, "function", diagnostics);
}

/// Rejects handler adapters that collide with source or earlier generated names.
fn validate_handler_names(
    handlers: &[HandlerPlan<'_>],
    source_names: &BTreeSet<&str>,
    declaration_kind: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut generated = BTreeSet::new();
    for handler in handlers {
        let name = generated_handler_name(handler.callable.name());
        let conflict = source_names.contains(name.as_str()) || !generated.insert(name.clone());
        if conflict {
            diagnostics.push(error(
                handler.callable.span(),
                format!("handler conflicts with generated handler {declaration_kind} `{name}`"),
                "rename the handler or conflicting handwritten declaration",
            ));
        }
    }
}

/// Finds the source span for one handwritten top-level declaration.
fn top_level_declaration_span(file: &DartFileIr, name: &str) -> Option<SpanIr> {
    top_level_declarations(file).find_map(|(candidate, span)| (candidate == name).then_some(span))
}

/// Returns every named declaration in the library's top-level namespace.
fn top_level_declarations(file: &DartFileIr) -> impl Iterator<Item = (&str, SpanIr)> {
    file.functions
        .iter()
        .map(|value| (value.name.short.as_str(), value.span))
        .chain(
            file.variables
                .iter()
                .map(|value| (value.name.short.as_str(), value.span)),
        )
        .chain(
            file.classes
                .iter()
                .map(|value| (value.name.as_str(), value.span)),
        )
        .chain(
            file.mixins
                .iter()
                .map(|value| (value.name.short.as_str(), value.span)),
        )
        .chain(file.extensions.iter().filter_map(|value| {
            value
                .name
                .as_ref()
                .map(|name| (name.short.as_str(), value.span))
        }))
        .chain(
            file.extension_types
                .iter()
                .map(|value| (value.name.short.as_str(), value.span)),
        )
        .chain(
            file.typedefs
                .iter()
                .map(|value| (value.name.short.as_str(), value.span)),
        )
        .chain(
            file.enums
                .iter()
                .map(|value| (value.name.as_str(), value.span)),
        )
}

/// Derives a private generated handler name from a Dart callable.
pub(crate) fn generated_handler_name(name: &str) -> String {
    let mut chars = name.chars();
    let title = chars
        .next()
        .map(|value| value.to_uppercase().collect::<String>())
        .unwrap_or_default()
        + chars.as_str();
    format!("_$handle{title}")
}

/// Derives the top-level router factory from a source basename.
pub(super) fn default_routes_name(source_path: &str) -> String {
    let stem = Path::new(source_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("server");
    let singular = stem.strip_suffix('s').unwrap_or(stem);
    let mut result = String::new();
    let mut uppercase = false;
    for ch in singular.chars() {
        if ch == '_' || ch == '-' {
            uppercase = true;
        } else if uppercase {
            result.extend(ch.to_uppercase());
            uppercase = false;
        } else {
            result.push(ch);
        }
    }
    format!("{result}Routes")
}

/// Maps a server verb configuration to its HTTP method.
pub(super) fn verb_config(config: &ConfigApplicationIr) -> Option<&'static str> {
    VERBS
        .iter()
        .find(|(symbol, _)| *symbol == config.symbol.0)
        .map(|(_, method)| *method)
}

/// Returns whether a configuration belongs to the server namespace.
pub(super) fn is_server_config(config: &ConfigApplicationIr) -> bool {
    config.symbol.0.starts_with("dust_server::")
}

/// Finds the first server-owned annotation at any supported coordinate.
pub(super) fn first_server_span(file: &DartFileIr) -> Option<SpanIr> {
    file.library_annotations
        .iter()
        .find(|value| {
            value
                .resolved_symbol
                .as_ref()
                .is_some_and(|symbol| symbol.0.starts_with("dust_server::"))
        })
        .map(|value| value.span)
        .or_else(|| config_span(file))
        .or_else(|| {
            file.functions
                .iter()
                .flat_map(|value| value.annotations.iter())
                .find(|value| {
                    value
                        .resolved_symbol
                        .as_ref()
                        .is_some_and(|symbol| symbol.0.starts_with("dust_server::"))
                })
                .map(|value| value.span)
        })
        .or_else(|| {
            file.functions
                .iter()
                .flat_map(|value| value.params.iter())
                .flat_map(|value| value.configs.iter())
                .find(|value| value.symbol.0.starts_with("dust_server::"))
                .map(|value| value.span)
        })
}

/// Finds the first class, method, or method-parameter server config.
fn config_span(file: &DartFileIr) -> Option<SpanIr> {
    file.classes
        .iter()
        .flat_map(|value| value.configs.iter())
        .chain(
            file.classes
                .iter()
                .flat_map(|value| value.methods.iter())
                .flat_map(|value| {
                    value
                        .configs
                        .iter()
                        .chain(value.params.iter().flat_map(|param| param.configs.iter()))
                }),
        )
        .find(|value| value.symbol.0.starts_with("dust_server::"))
        .map(|value| value.span)
}
