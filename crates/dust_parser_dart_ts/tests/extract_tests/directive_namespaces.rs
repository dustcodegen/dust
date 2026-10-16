use dust_parser_dart::{
    ParsedCombinatorKind as Kind, ParsedConditionalUri, ParsedDirective, ParsedNamespaceDirective,
    ParsedUriLiteral, ParsedVisibilityCombinator,
};
use dust_text::TextRange;

use crate::support::parse;

const SOURCE: &str = include_str!("../../../../fixtures/namespace_directives/mixed.dart.snapshot");

fn range(source: &str, text: &str) -> TextRange {
    let start = source.find(text).expect("literal fixture text");
    TextRange::new(start as u32, (start + text.len()) as u32)
}

fn literal(source: &str, text: &str) -> ParsedUriLiteral {
    ParsedUriLiteral {
        source: text.into(),
        span: range(source, text),
    }
}

fn condition(whole: &str, test: &str, uri: &str) -> ParsedConditionalUri {
    ParsedConditionalUri {
        condition_source: test.into(),
        condition_span: range(SOURCE, test),
        uri: literal(SOURCE, uri),
        span: range(SOURCE, whole),
    }
}

fn combinator(whole: &str, kind: Kind, names: &[&str]) -> ParsedVisibilityCombinator {
    ParsedVisibilityCombinator {
        kind,
        names: names.iter().map(|name| (*name).into()).collect(),
        span: range(SOURCE, whole),
    }
}

#[test]
fn import_and_export_facts_keep_ordered_clauses_conditions_and_exact_byte_spans() {
    let parsed = parse(106, SOURCE);
    assert_eq!(parsed.diagnostics, []);
    assert_eq!(parsed.library.directives.len(), 5);
    let ParsedDirective::Import {
        namespace,
        uri,
        prefix,
        show,
        hide,
        is_deferred,
        ..
    } = &parsed.library.directives[0]
    else {
        panic!("import")
    };
    assert_eq!(uri, "base.dart");
    assert_eq!(prefix.as_deref(), Some("platform"));
    assert_eq!(show, &["GET", "POST", "GET"]);
    assert_eq!(hide, &["POST"]);
    assert!(!is_deferred);
    assert_eq!(
        namespace,
        &Some(ParsedNamespaceDirective {
            default_uri: literal(SOURCE, "'base.dart'"),
            conditional_uris: vec![
                condition(
                    "if (dart.library.io) 'io.dart'",
                    "(dart.library.io)",
                    "'io.dart'"
                ),
                condition(
                    "if (environment.feature /* condition */ == 'yes') r'web.dart'",
                    "(environment.feature /* condition */ == 'yes')",
                    "r'web.dart'"
                ),
            ],
            combinators: vec![
                combinator("show GET, POST", Kind::Show, &["GET", "POST"]),
                combinator("show /* repeated */ GET", Kind::Show, &["GET"]),
                combinator("hide POST", Kind::Hide, &["POST"]),
            ],
        })
    );
    let ParsedDirective::Export { namespace, .. } = &parsed.library.directives[1] else {
        panic!("export")
    };
    assert_eq!(
        namespace,
        &Some(ParsedNamespaceDirective {
            default_uri: literal(SOURCE, "r'exports.dart'"),
            conditional_uris: vec![condition(
                "if (dart.library.html) 'web_export.dart'",
                "(dart.library.html)",
                "'web_export.dart'"
            )],
            combinators: vec![
                combinator("show /* export */ Public", Kind::Show, &["Public"]),
                combinator("hide Private", Kind::Hide, &["Private"]),
                combinator("show Public, Extra", Kind::Show, &["Public", "Extra"]),
            ],
        })
    );
}

#[test]
fn shared_prefix_deferred_import_and_parts_keep_their_own_source_facts() {
    let parsed = parse(106, SOURCE);
    assert_eq!(parsed.diagnostics, []);
    let ParsedDirective::Import {
        namespace,
        prefix,
        is_deferred,
        ..
    } = &parsed.library.directives[2]
    else {
        panic!("shared prefix")
    };
    assert_eq!(prefix.as_deref(), Some("platform"));
    assert!(!is_deferred);
    assert_eq!(
        namespace,
        &Some(ParsedNamespaceDirective {
            default_uri: literal(SOURCE, "'shared.dart'"),
            conditional_uris: vec![],
            combinators: vec![combinator("hide Internal", Kind::Hide, &["Internal"])],
        })
    );
    let ParsedDirective::Import {
        namespace,
        prefix,
        is_deferred,
        ..
    } = &parsed.library.directives[3]
    else {
        panic!("deferred")
    };
    assert_eq!(prefix.as_deref(), Some("lazy"));
    assert!(*is_deferred);
    assert_eq!(
        namespace,
        &Some(ParsedNamespaceDirective {
            default_uri: literal(SOURCE, "'deferred.dart'"),
            conditional_uris: vec![],
            combinators: vec![combinator("show Deferred", Kind::Show, &["Deferred"])],
        })
    );
    let ParsedDirective::Part { uri_literal, .. } = &parsed.library.directives[4] else {
        panic!("part")
    };
    assert_eq!(uri_literal, &Some(literal(SOURCE, "'members.dart'")));
}

#[test]
fn uri_literals_preserve_raw_and_escaped_spelling_without_decoding() {
    for text in [
        r"import r'path/$raw.dart';",
        r"export 'path/\u0061.dart';",
        r#"part of "owner.dart";"#,
    ] {
        let parsed = parse(106, text);
        assert_eq!(parsed.diagnostics, []);
        let literal = match &parsed.library.directives[0] {
            ParsedDirective::Import { namespace, .. }
            | ParsedDirective::Export { namespace, .. } => &namespace.as_ref().unwrap().default_uri,
            ParsedDirective::PartOf { uri_literal, .. } => uri_literal.as_ref().unwrap(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(
            &text[u32::from(literal.span.start()) as usize..u32::from(literal.span.end()) as usize],
            literal.source
        );
        let expected = text.split_once(' ').unwrap().1.trim_end_matches(';');
        assert_eq!(
            literal.source,
            expected.strip_prefix("of ").unwrap_or(expected)
        );
    }
    let parsed = parse(106, "part of models.owner;");
    assert_eq!(parsed.diagnostics, []);
    assert!(
        matches!(&parsed.library.directives[0], ParsedDirective::PartOf { library_name: Some(name), uri: None, uri_literal: None, .. } if name == "models.owner")
    );
}

#[test]
fn malformed_directives_remain_diagnostic_not_valid_namespace_evidence() {
    for source in [
        "import ;",
        "import 'base.dart' if () 'io.dart';",
        "export 'base.dart' show ;",
    ] {
        let parsed = parse(106, source);
        assert!(!parsed.diagnostics.is_empty(), "{source}");
    }
}
