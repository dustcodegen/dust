use dust_parser_dart::{
    ParsedDartFileSurface, ParsedDirective, ParsedNamespaceDirective, ParsedUriLiteral,
};
use dust_text::TextRange;

/// Inject core into graph facts only; handwritten parser facts stay unchanged.
pub(in crate::namespace_sources) fn namespace_surface(
    parsed: &ParsedDartFileSurface,
    uri: &str,
) -> ParsedDartFileSurface {
    let mut parsed = parsed.clone();
    if uri != "dart:core"
        && !parsed.directives.iter().any(|directive| {
            matches!(directive, ParsedDirective::PartOf { .. })
                || matches!(directive, ParsedDirective::Import { namespace: Some(namespace), .. }
                if namespace.default_uri.value().as_deref() == Some("dart:core"))
        })
    {
        parsed.directives.push(ParsedDirective::Import {
            namespace: Some(ParsedNamespaceDirective {
                default_uri: ParsedUriLiteral {
                    source: "'dart:core'".into(),
                    span: TextRange::empty(0_u32),
                },
                conditional_uris: Vec::new(),
                combinators: Vec::new(),
            }),
            uri: "dart:core".into(),
            prefix: None,
            show: Vec::new(),
            hide: Vec::new(),
            is_deferred: false,
            span: TextRange::empty(0_u32),
        });
    }
    parsed
}

/// Retains every written URI alternative without choosing a host platform.
pub(super) fn literals(directive: &ParsedDirective) -> Vec<&ParsedUriLiteral> {
    match directive {
        ParsedDirective::Import {
            namespace: Some(namespace),
            ..
        }
        | ParsedDirective::Export {
            namespace: Some(namespace),
            ..
        } => std::iter::once(&namespace.default_uri)
            .chain(namespace.conditional_uris.iter().map(|branch| &branch.uri))
            .collect(),
        ParsedDirective::Part {
            uri_literal: Some(uri),
            ..
        }
        | ParsedDirective::PartOf {
            uri_literal: Some(uri),
            ..
        } => vec![uri],
        _ => Vec::new(),
    }
}
