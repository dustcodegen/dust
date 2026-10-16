//! Resolving an annotation and its trait application to catalog symbols.

use super::*;

/// Fields and constructors retain the same config-only role and warning policy.
pub(super) fn resolve_configs(
    file_id: FileId,
    annotations: &[ParsedAnnotation],
    context: &BindingContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    site: &str,
) -> Vec<ConfigApplicationIr> {
    let mut configs = Vec::new();
    for annotation in annotations {
        let Application::Symbol(resolved) = context.application(file_id, annotation, diagnostics)
        else {
            continue;
        };
        if resolved.kind != SymbolKind::Config {
            diagnostics.push(
                Diagnostic::warning(format!(
                    "trait annotation `{}` is not supported on {site}s",
                    annotation.name
                ))
                .with_label(SourceLabel::new(
                    file_id,
                    annotation.span,
                    format!("{site} annotations may only use Dust config symbols"),
                )),
            );
            continue;
        }
        let (positional_args, named_args) = annotation_argument_values(file_id, annotation);
        configs.push(ConfigApplicationIr::with_arguments(
            resolved.symbol.clone(),
            annotation.arguments_source.clone(),
            positional_args,
            named_args,
            SpanIr::new(file_id, annotation.span),
        ));
    }
    configs
}

/// Returns the first generated part URI from parsed directives.
pub(crate) fn first_part_uri(directives: &[ParsedDirective]) -> Option<String> {
    directives.iter().find_map(|directive| match directive {
        ParsedDirective::Part { uri, .. } => Some(uri.clone()),
        _ => None,
    })
}

/// Pushes one resolved symbol into the matching trait or config list.
pub(super) fn push_resolved_symbol(
    file_id: FileId,
    application: ResolvedAnnotationSymbol,
    traits: &mut Vec<TraitApplicationIr>,
    configs: &mut Vec<ConfigApplicationIr>,
) {
    match application.kind {
        SymbolKind::Trait => traits.push(TraitApplicationIr {
            symbol: application.symbol,
            span: SpanIr::new(file_id, application.span),
        }),
        SymbolKind::Config => {
            let (positional_args, named_args) = application.arguments;
            configs.push(ConfigApplicationIr::with_arguments(
                application.symbol,
                application.arguments_source,
                positional_args,
                named_args,
                SpanIr::new(file_id, application.span),
            ));
        }
    }
}
