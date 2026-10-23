use dust_diagnostics::Diagnostic;

use super::{
    Callable, Extraction, ParamPlan,
    model::Args,
    values::{error, validate_no_args},
};

#[cfg(test)]
mod tests;

/// Body media selected at the handler declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BodyMedia {
    /// Default JSON body handling.
    Json,
    /// `application/x-www-form-urlencoded`.
    Form,
    /// `multipart/form-data`.
    Multipart,
}

/// Reads and validates the handler-level body media annotation.
pub(super) fn parse_body_media(
    callable: Callable<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> BodyMedia {
    let markers = match callable {
        Callable::Method(method) => method
            .configs
            .iter()
            .filter_map(|config| {
                media(config.symbol.0.as_str()).map(|value| (value, Args::Config(config)))
            })
            .collect::<Vec<_>>(),
        Callable::Function(function) => function
            .annotations
            .iter()
            .filter_map(|annotation| {
                let symbol = annotation.resolved_symbol.as_ref()?.0.as_str();
                media(symbol).map(|value| (value, Args::Annotation(annotation)))
            })
            .collect::<Vec<_>>(),
    };
    for (_, args) in &markers {
        validate_no_args(args, diagnostics);
    }
    if markers.len() > 1 {
        diagnostics.push(error(
            callable.span(),
            "server handlers accept only one body media annotation",
            "remove either FormUrlEncoded or MultiPart",
        ));
    }
    markers
        .first()
        .map(|(value, _)| *value)
        .unwrap_or(BodyMedia::Json)
}

/// Enforces one body source occupying the final parameter group.
pub(super) fn validate_body_order(
    callable: Callable<'_>,
    media: BodyMedia,
    params: &[ParamPlan<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let body = params
        .iter()
        .enumerate()
        .filter_map(|(index, param)| body_source(&param.extraction).map(|source| (index, source)))
        .collect::<Vec<_>>();
    let incompatible = body
        .first()
        .is_some_and(|(_, first)| body.iter().any(|(_, source)| source != first));
    if incompatible
        || body
            .iter()
            .filter(|(_, source)| *source == BodySource::Single)
            .count()
            > 1
    {
        diagnostics.push(error(
            callable.span(),
            "server handlers accept only one request-body source",
            "use one Body or RawBody parameter, one form field group, or one multipart part group",
        ));
    }
    if let Some((first, _)) = body.first()
        && params[*first..]
            .iter()
            .any(|param| body_source(&param.extraction).is_none())
    {
        diagnostics.push(error(
            callable.span(),
            "body-reading parameters must be the final handler parameter group",
            "move Body, RawBody, Field, or Part parameters after request-parts extractors",
        ));
    }

    let compatible_media = match media {
        BodyMedia::Json => true,
        BodyMedia::Form => body
            .iter()
            .any(|(_, source)| matches!(source, BodySource::Single | BodySource::FormFields)),
        BodyMedia::Multipart => body
            .iter()
            .any(|(_, source)| *source == BodySource::MultipartParts),
    };
    if !compatible_media {
        diagnostics.push(error(
            callable.span(),
            "body media annotation has no compatible body parameter",
            "use Body or Field with FormUrlEncoded, or Part with MultiPart",
        ));
    }
}

/// Maps a resolved server annotation to its body media.
fn media(symbol: &str) -> Option<BodyMedia> {
    match symbol {
        "dust_server::FormUrlEncoded" => Some(BodyMedia::Form),
        "dust_server::MultiPart" => Some(BodyMedia::Multipart),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Request-body ownership used for ordering and compatibility checks.
enum BodySource {
    /// A complete typed or raw body.
    Single,
    /// Fields sharing one URL-encoded body.
    FormFields,
    /// Parts sharing one multipart body.
    MultipartParts,
}

/// Classifies the request-body source used by an extraction.
fn body_source(extraction: &Extraction) -> Option<BodySource> {
    match extraction {
        Extraction::Body(_) | Extraction::RawBody(_) => Some(BodySource::Single),
        Extraction::Field(_) => Some(BodySource::FormFields),
        Extraction::Part { .. } => Some(BodySource::MultipartParts),
        _ => None,
    }
}
