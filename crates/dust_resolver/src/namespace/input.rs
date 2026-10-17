use dust_parser_dart::{ParsedDirective, ParsedNamespaceDirective, ParsedTypeKind};

use super::*;

/// Retain normalized facts and source spans; do not scan directive source strings.
pub(super) fn unit(
    uri: &str,
    file_id: FileId,
    parsed: &ParsedDartFileSurface,
    resolve: &mut impl FnMut(&str, &str) -> Result<String, String>,
) -> Unit {
    let mut unit = Unit {
        file_id,
        lexical_scopes: parsed.lexical_scopes.clone(),
        library_name: None,
        declarations: BTreeMap::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        parts: Vec::new(),
        part_of: None,
    };
    let mut declare = |name: &str, span: TextRange, kind| {
        unit.declarations.insert(
            Declaration {
                uri: uri.to_owned(),
                name: name.to_owned(),
                file_id,
                start: span.start().to_u32(),
                end: span.end().to_u32(),
            },
            kind,
        );
    };
    for class in &parsed.classes {
        declare(&class.name, class.span, DeclarationKind::Concrete);
    }
    for item in &parsed.enums {
        declare(&item.name, item.span, DeclarationKind::Concrete);
    }
    for item in &parsed.mixins {
        declare(&item.name, item.span, DeclarationKind::Concrete);
    }
    for item in &parsed.extension_types {
        declare(&item.name, item.span, DeclarationKind::Concrete);
    }
    for item in &parsed.extensions {
        if let Some(name) = &item.name {
            declare(name, item.span, DeclarationKind::Concrete);
        }
    }
    for item in &parsed.functions {
        declare(&item.name, item.span, DeclarationKind::Foreign);
    }
    for item in &parsed.variables {
        declare(&item.name, item.span, DeclarationKind::Opaque);
    }
    for item in &parsed.typedefs {
        let kind = match item.parsed_aliased_type.as_ref() {
            Some(ty) if ty.nullable => DeclarationKind::Foreign,
            Some(ty) => match &ty.kind {
                ParsedTypeKind::Named { name, .. } => DeclarationKind::Alias {
                    target: name.clone(),
                    builtin: false,
                },
                ParsedTypeKind::Builtin(name) => DeclarationKind::Alias {
                    target: name.clone(),
                    builtin: true,
                },
                ParsedTypeKind::Unknown => DeclarationKind::Opaque,
                _ => DeclarationKind::Foreign,
            },
            None => DeclarationKind::Opaque,
        };
        declare(&item.name, item.span, kind);
    }
    for directive in &parsed.directives {
        match directive {
            ParsedDirective::Library { name, .. } => unit.library_name = name.clone(),
            ParsedDirective::Import {
                namespace,
                prefix,
                is_deferred,
                span,
                ..
            } => {
                unit.imports.push(edge(
                    uri,
                    file_id,
                    namespace.as_ref(),
                    *span,
                    prefix.clone(),
                    *is_deferred,
                    resolve,
                ));
            }
            ParsedDirective::Export {
                namespace, span, ..
            } => {
                unit.exports.push(edge(
                    uri,
                    file_id,
                    namespace.as_ref(),
                    *span,
                    None,
                    false,
                    resolve,
                ));
            }
            ParsedDirective::Part {
                uri_literal, span, ..
            } => {
                unit.parts.push((
                    target(uri, file_id, uri_literal.as_ref(), *span, resolve),
                    SourceLabel::new(file_id, *span, format!("part of {uri}")),
                ));
            }
            ParsedDirective::PartOf {
                uri_literal,
                library_name,
                span,
                ..
            } => {
                let label = SourceLabel::new(file_id, *span, "part owner declared here");
                unit.part_of = Some(if let Some(name) = library_name {
                    PartOwner::Name(name.clone(), label)
                } else {
                    PartOwner::Uri(
                        target(uri, file_id, uri_literal.as_ref(), *span, resolve),
                        label,
                    )
                });
            }
        }
    }
    unit
}

/// All branch literals are decoded through the shared helper, never legacy URI text.
fn edge(
    uri: &str,
    file_id: FileId,
    namespace: Option<&ParsedNamespaceDirective>,
    span: TextRange,
    prefix: Option<String>,
    deferred: bool,
    resolve: &mut impl FnMut(&str, &str) -> Result<String, String>,
) -> Edge {
    let (targets, combinators) = match namespace {
        Some(facts) => {
            let targets = std::iter::once(&facts.default_uri)
                .chain(facts.conditional_uris.iter().map(|item| &item.uri))
                .map(|literal| target(uri, file_id, Some(literal), literal.span, resolve))
                .collect();
            (targets, facts.combinators.clone())
        }
        None => (
            vec![Err(Problem::new(
                file_id,
                span,
                format!("missing ordered namespace facts in `{uri}`"),
            ))],
            Vec::new(),
        ),
    };
    Edge {
        targets,
        combinators,
        prefix,
        deferred,
        label: SourceLabel::new(file_id, span, format!("namespace directive in {uri}")),
    }
}
