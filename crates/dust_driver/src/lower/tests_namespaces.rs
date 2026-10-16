#![cfg(test)]

use dust_ir::{
    CombinatorKindIr as Kind, ConditionalUriIr, ExprSourceIr, NamespaceDirectiveIr, SpanIr,
    VisibilityCombinatorIr,
};
use dust_parser_dart::{ParseBackend, ParseOptions, ParsedDirective};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{ResolvedLibrary, SymbolCatalog, resolve_library};
use dust_text::{FileId, SourceText, TextRange};

const SOURCE: &str = include_str!("../../../../fixtures/namespace_directives/mixed.dart.snapshot");

fn span(text: &str) -> SpanIr {
    let start = SOURCE.find(text).unwrap();
    SpanIr::new(
        FileId::new(106),
        TextRange::new(start as u32, (start + text.len()) as u32),
    )
}

fn expression(text: &str) -> ExprSourceIr {
    ExprSourceIr {
        source: text.into(),
        span: span(text),
    }
}

fn combinator(text: &str, kind: Kind, names: &[&str]) -> VisibilityCombinatorIr {
    VisibilityCombinatorIr {
        kind,
        names: names.iter().map(|name| (*name).into()).collect(),
        span: span(text),
    }
}

fn resolve(text: &str) -> ResolvedLibrary {
    let id = FileId::new(106);
    let parsed = TreeSitterDartBackend::new()
        .parse_file(&SourceText::new(id, text), ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let resolved = resolve_library(
        id,
        "lib/facts.dart",
        "lib/facts.g.dart",
        &parsed.library,
        &SymbolCatalog::new(),
    );
    assert_eq!(resolved.diagnostics, []);
    resolved.library
}

#[test]
fn source_namespace_facts_survive_resolution_and_ir_for_both_class_headers() {
    for class in [
        "class Box<T> { const Box(this.value); final T value; }",
        "class const Box<T>(final T value);",
    ] {
        let mut resolved = resolve(&format!("{SOURCE}\n{class}"));
        let lowered = super::lower_library(&mut resolved);
        assert_eq!(lowered.diagnostics, []);
        let file = lowered.value;
        assert_eq!(file.imports, ["base.dart", "shared.dart", "deferred.dart"]);
        assert_eq!(file.import_directives[0].show, ["GET", "POST", "GET"]);
        assert_eq!(file.import_directives[0].hide, ["POST"]);
        assert_eq!(
            file.import_directives[0].namespace,
            Some(NamespaceDirectiveIr {
                default_uri: expression("'base.dart'"),
                conditional_uris: vec![
                    ConditionalUriIr {
                        condition: expression("(dart.library.io)"),
                        uri: expression("'io.dart'"),
                        span: span("if (dart.library.io) 'io.dart'")
                    },
                    ConditionalUriIr {
                        condition: expression("(environment.feature /* condition */ == 'yes')"),
                        uri: expression("r'web.dart'"),
                        span: span("if (environment.feature /* condition */ == 'yes') r'web.dart'")
                    },
                ],
                combinators: vec![
                    combinator("show GET, POST", Kind::Show, &["GET", "POST"]),
                    combinator("show /* repeated */ GET", Kind::Show, &["GET"]),
                    combinator("hide POST", Kind::Hide, &["POST"]),
                ],
            })
        );
        assert_eq!(
            file.export_directives[0].namespace,
            Some(NamespaceDirectiveIr {
                default_uri: expression("r'exports.dart'"),
                conditional_uris: vec![ConditionalUriIr {
                    condition: expression("(dart.library.html)"),
                    uri: expression("'web_export.dart'"),
                    span: span("if (dart.library.html) 'web_export.dart'"),
                }],
                combinators: vec![
                    combinator("show /* export */ Public", Kind::Show, &["Public"]),
                    combinator("hide Private", Kind::Hide, &["Private"]),
                    combinator("show Public, Extra", Kind::Show, &["Public", "Extra"]),
                ],
            })
        );
        assert_eq!(
            file.part_directives[0].uri_literal,
            Some(expression("'members.dart'"))
        );
    }
}

#[test]
fn synthetic_unsupplied_facts_are_not_reconstructed_from_legacy_strings() {
    let mut resolved = resolve(SOURCE);
    for directive in &mut resolved.directives {
        match directive {
            ParsedDirective::Import { namespace, .. }
            | ParsedDirective::Export { namespace, .. } => *namespace = None,
            ParsedDirective::Part { uri_literal, .. } => *uri_literal = None,
            _ => {}
        }
    }
    let lowered = super::lower_library(&mut resolved);
    assert_eq!(lowered.diagnostics, []);
    assert_eq!(lowered.value.import_directives[0].namespace, None);
    assert_eq!(lowered.value.export_directives[0].namespace, None);
    assert_eq!(lowered.value.part_directives[0].uri_literal, None);
}

#[test]
fn part_of_literal_keeps_file_ownership_and_named_form_has_no_literal() {
    for (source, expected) in [
        ("part of models.owner;", None),
        ("part of \"owner.dart\";", Some("\"owner.dart\"")),
    ] {
        let mut resolved = resolve(source);
        let lowered = super::lower_library(&mut resolved);
        assert_eq!(lowered.diagnostics, []);
        assert_eq!(
            lowered.value.part_of.unwrap().uri_literal,
            expected.map(|text| {
                let start = source.find(text).unwrap();
                ExprSourceIr {
                    source: text.into(),
                    span: SpanIr::new(
                        FileId::new(106),
                        TextRange::new(start as u32, (start + text.len()) as u32),
                    ),
                }
            })
        );
    }
}
