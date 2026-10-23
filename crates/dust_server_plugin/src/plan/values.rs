use std::collections::BTreeSet;

use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{AnnotationNumberKindIr, AnnotationValueIr, SpanIr};

use super::model::Args;

/// Route-documentation metadata parsing.
mod metadata;

pub(super) use metadata::{parse_route_metadata, parse_tags};

#[cfg(test)]
/// Focused annotation-value and route-path tests.
mod tests;

/// Parses and range-checks an optional HTTP success status.
pub(super) fn parse_status(args: &Args<'_>, diagnostics: &mut Vec<Diagnostic>) -> Option<u16> {
    let value = args.named().get("status")?;
    let AnnotationValueIr::Number {
        source,
        kind: AnnotationNumberKindIr::Int,
    } = value
    else {
        diagnostics.push(error(
            args.span(),
            "`status` must be an integer",
            "use an HTTP status from 100 through 599",
        ));
        return None;
    };
    let status = source
        .parse::<u16>()
        .ok()
        .filter(|value| (100..=599).contains(value));
    if status.is_none() {
        diagnostics.push(error(
            args.span(),
            "`status` is outside the HTTP status range",
            "use an HTTP status from 100 through 599",
        ));
    }
    status
}

/// Validates supported names and structured value shapes.
pub(super) fn validate_named(args: &Args<'_>, allowed: &[&str], diagnostics: &mut Vec<Diagnostic>) {
    for key in args.named().keys() {
        if !allowed.contains(&key.as_str()) {
            diagnostics.push(error(
                args.span(),
                format!("unknown server annotation argument `{key}`"),
                "remove the argument or correct its name",
            ));
        }
    }
    validate_named_scalars(args, diagnostics);
    if let Some(value) = args.named().get("tags")
        && !matches!(value, AnnotationValueIr::List(values) if values.iter().all(|value| matches!(value, AnnotationValueIr::String(_))))
    {
        diagnostics.push(error(
            args.span(),
            "`tags` must be a list of strings",
            "use a const string list",
        ));
    }
    if let Some(value) = args.named().get("middleware")
        && !matches!(value, AnnotationValueIr::List(_))
    {
        diagnostics.push(error(
            args.span(),
            "`middleware` must be a list",
            "use a const middleware list",
        ));
    }
}

/// Returns middleware constants after rejecting shapes that cannot be emitted safely.
pub(super) fn parse_middleware(
    args: &Args<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<AnnotationValueIr> {
    let Some(AnnotationValueIr::List(values)) = args.named().get("middleware") else {
        return Vec::new();
    };
    values
        .iter()
        .filter_map(|value| {
            if matches!(
                value,
                AnnotationValueIr::Constructor { .. } | AnnotationValueIr::Member(_)
            ) && is_emittable_const(value)
            {
                Some(value.clone())
            } else {
                diagnostics.push(error(
                    args.span(),
                    "middleware entries must be structured const Layer values",
                    "use a const constructor or constant reference",
                ));
                None
            }
        })
        .collect()
}

/// Returns whether a constant can be rebuilt entirely from structured IR.
fn is_emittable_const(value: &AnnotationValueIr) -> bool {
    match value {
        AnnotationValueIr::Null
        | AnnotationValueIr::Bool(_)
        | AnnotationValueIr::String(_)
        | AnnotationValueIr::Number { .. }
        | AnnotationValueIr::Member(_) => true,
        AnnotationValueIr::List(values) | AnnotationValueIr::Set(values) => {
            values.iter().all(is_emittable_const)
        }
        AnnotationValueIr::Map(values) => values
            .iter()
            .all(|(key, value)| is_emittable_const(key) && is_emittable_const(value)),
        AnnotationValueIr::Record(values) => {
            values.iter().all(|(_, value)| is_emittable_const(value))
        }
        AnnotationValueIr::Constructor {
            positional_args,
            named_args,
            ..
        } => {
            positional_args.iter().all(is_emittable_const)
                && named_args.values().all(is_emittable_const)
        }
        AnnotationValueIr::Expression(_) => false,
    }
}

/// Validates scalar named metadata arguments.
fn validate_named_scalars(args: &Args<'_>, diagnostics: &mut Vec<Diagnostic>) {
    for key in ["summary", "description", "operationId", "name"] {
        if let Some(value) = args.named().get(key)
            && !matches!(
                value,
                AnnotationValueIr::String(_) | AnnotationValueIr::Null
            )
        {
            diagnostics.push(error(
                args.span(),
                format!("`{key}` must be a string"),
                "use a string literal",
            ));
        }
    }
    for key in ["deprecated", "hidden"] {
        if let Some(value) = args.named().get(key)
            && !matches!(value, AnnotationValueIr::Bool(_))
        {
            diagnostics.push(error(
                args.span(),
                format!("`{key}` must be a bool"),
                "use true or false",
            ));
        }
    }
}

/// Reads one required positional string and rejects extra values.
pub(super) fn required_string(
    args: &Args<'_>,
    index: usize,
    label: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    if args.positional().len() != index + 1 {
        diagnostics.push(error(
            args.span(),
            format!("{label} must be one string literal"),
            "supply the documented positional argument",
        ));
        return None;
    }
    string_value(&args.positional()[index]).or_else(|| {
        diagnostics.push(error(
            args.span(),
            format!("{label} must be a string literal"),
            "use a string literal",
        ));
        None
    })
}

/// Reads one optional positional string.
pub(super) fn optional_string(
    args: &Args<'_>,
    index: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    if args.positional().len() > 1 {
        diagnostics.push(error(
            args.span(),
            "Path accepts at most one name",
            "remove the extra argument",
        ));
    }
    args.positional().get(index).and_then(|value| match value {
        AnnotationValueIr::Null => None,
        value => string_value(value).or_else(|| {
            diagnostics.push(error(
                args.span(),
                "Path name must be a string",
                "use a string literal",
            ));
            None
        }),
    })
}

/// Rejects any positional or named arguments.
pub(super) fn validate_no_args(args: &Args<'_>, diagnostics: &mut Vec<Diagnostic>) {
    if !args.positional().is_empty() || !args.named().is_empty() {
        diagnostics.push(error(
            args.span(),
            "this extractor accepts no arguments",
            "remove the arguments",
        ));
    }
}

/// Rejects named arguments while permitting positional values.
pub(super) fn validate_zero_named(args: &Args<'_>, diagnostics: &mut Vec<Diagnostic>) {
    if !args.named().is_empty() {
        diagnostics.push(error(
            args.span(),
            "this extractor accepts no named arguments",
            "remove the named arguments",
        ));
    }
}

/// Validates the static shape of a native server route path.
pub(super) fn validate_route_path(path: &str, span: SpanIr, diagnostics: &mut Vec<Diagnostic>) {
    if !path.starts_with('/') || path.contains(['?', '#']) || path_keys_checked(path).is_none() {
        diagnostics.push(error(
            span,
            "invalid server route path",
            "use a leading slash and balanced `{name}` placeholders without query or fragment text",
        ));
    }
}

/// Returns placeholder keys from an already validated route.
pub(super) fn path_keys(path: &str) -> BTreeSet<String> {
    path_keys_checked(path).unwrap_or_default()
}

/// Parses placeholder keys while rejecting malformed braces.
fn path_keys_checked(path: &str) -> Option<BTreeSet<String>> {
    let mut keys = BTreeSet::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        if rest[..start].contains('}') {
            return None;
        }
        rest = &rest[start + 1..];
        let end = rest.find('}')?;
        let key = rest[..end].split('|').next()?.trim();
        if key.is_empty() || key.contains(['{', '/']) {
            return None;
        }
        keys.insert(key.to_owned());
        rest = &rest[end + 1..];
    }
    (!rest.contains('}')).then_some(keys)
}

/// Returns a structured string annotation value.
pub(super) fn string_value(value: &AnnotationValueIr) -> Option<String> {
    if let AnnotationValueIr::String(value) = value {
        Some(value.clone())
    } else {
        None
    }
}

/// Creates a source-labelled build diagnostic.
pub(super) fn error(
    span: SpanIr,
    message: impl Into<String>,
    label: impl Into<String>,
) -> Diagnostic {
    Diagnostic::error(message).with_label(SourceLabel::new(span.file_id, span.range, label))
}
