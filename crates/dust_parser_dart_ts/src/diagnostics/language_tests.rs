use dust_parser_dart::ParseOptions;
use dust_text::FileId;

use super::*;

fn assert_language_parity(text: &str, version: Option<DartLanguageVersion>) {
    let source = SourceText::new(FileId::new(142), text);
    let tree = crate::PARSER
        .with(|parser| parser.borrow_mut().parse(text, None))
        .unwrap();
    let options = ParseOptions {
        language_version: version.unwrap_or(DartLanguageVersion::latest_known()),
        ..Default::default()
    };
    let library = crate::extract_parse_result(Some(tree.clone()), &source, options).library;
    let effective = options.effective_language_version_for_source(text);
    let mut full_occurrences = Vec::new();
    let mut filtered_occurrences = Vec::new();
    let mut full = DiagnosticViews::default();
    collect_error_diagnostics(
        &mut tree.walk(),
        None,
        &source,
        &mut full,
        &mut full_occurrences,
        tree.root_node().has_error(),
    );
    let mut filtered = extract_diagnostics(
        &tree,
        &source,
        &mut filtered_occurrences,
        version.map(|_| effective),
    );
    if version.is_none() {
        assert_eq!(
            format!("{filtered_occurrences:?}"),
            format!("{full_occurrences:?}"),
            "{text}"
        );
    }
    let full_language = crate::language_gates::extract_language_version_diagnostics(
        tree.root_node(),
        &library,
        &source,
        effective,
        full_occurrences,
    );
    let filtered_language = crate::language_gates::extract_language_version_diagnostics(
        tree.root_node(),
        &library,
        &source,
        effective,
        filtered_occurrences,
    );
    full.diagnostics.extend(full_language.iter().cloned());
    full.namespace_diagnostics.extend(full_language);
    filtered
        .diagnostics
        .extend(filtered_language.iter().cloned());
    filtered.namespace_diagnostics.extend(filtered_language);
    assert_eq!(
        filtered.diagnostics, full.diagnostics,
        "{version:?}: {text}"
    );
    assert_eq!(
        filtered.namespace_diagnostics, full.namespace_diagnostics,
        "{version:?}: {text}"
    );
}

#[test]
fn effective_language_queries_preserve_complete_diagnostics_and_order() {
    for text in [
        "base class B {} sealed class S {} final class F {} mixin class M {}",
        "(int, String) f(Object value) { if(value case [var item]) {} return (1, 'one'); }",
        "Object f(Object? value) => {?value: ?value};",
        "Object f(Object? value) => {?value};",
        "Object f(Object? value) => [if(true) ?value];",
        "Object f(Object? value) => [for(final item in [value]) ?item];",
        "Object f(Object? value) => [?value, if(true) ?value, for(final item in [value]) ?item];",
        "class Foo { Foo(); Foo.new(); const Foo.other(); const Foo.new(); factory Foo.make() = Foo; }",
        "class Foo { new(); const new create(); factory named() => Foo(); factory redirected() = Foo; }",
        "class Foo { const new(); }",
        "class C { new(); Object method() => .named(); final Object field = .named(); static const marker = .named(); }",
        "class C { C({Object value = .named()}); void method({Object value = .named()}) {} }",
        "Object f() => Type<int>.make();",
        "String f() => '${(1, 2)} ${.named()}';",
        "extension type E(int value) {} class C { C({String? _value}); }",
        "class C(final int value) { void method(final int value) {} }",
        "class C { void method(var value) {} }",
        "// @dart=3.0\nObject f(Object? value) => [?value];",
        "// @dart=3.13\nclass C { void method(final int value) {} }",
        "class C { void broken( {}",
        "class C { new.named( ; Object f() => .named(); }",
    ] {
        for version in [
            DartLanguageVersion::DART_2_17,
            DartLanguageVersion::DART_3_0,
            DartLanguageVersion::DART_3_3,
            DartLanguageVersion::new(3, 6),
            DartLanguageVersion::DART_3_8,
            DartLanguageVersion::new(3, 9),
            DartLanguageVersion::DART_3_10,
            DartLanguageVersion::DART_3_12,
            DartLanguageVersion::DART_3_13,
        ] {
            assert_language_parity(text, Some(version));
        }
        assert_language_parity(text, None);
    }
}

#[test]
fn constructor_query_keeps_traditional_named_new_and_concise_factories_distinct() {
    let legacy = super::tests::uses_legacy_grammar();
    for (text, expected) in [
        ("class Foo { Foo(); }", 0),
        ("class Foo { Foo.new(); }", 0),
        ("class Foo { const Foo.new(); }", 0),
        ("class Foo { new(); }", 1),
        ("class Foo { const new(); }", 1),
        ("class Foo { const new create(); }", 1),
        ("class Foo { factory Foo.make() = Foo; }", 0),
        ("class Foo { factory make() => Foo(); }", 1),
        ("class Foo { factory make() = Foo; }", 1),
    ] {
        let source = SourceText::new(FileId::new(145), text);
        let tree = crate::PARSER
            .with(|parser| parser.borrow_mut().parse(text, None))
            .unwrap();
        if !legacy || expected == 0 {
            assert!(!tree.root_node().has_error(), "{text}");
        }
        for version in [
            None,
            Some(DartLanguageVersion::new(3, 6)),
            Some(DartLanguageVersion::DART_3_8),
        ] {
            if legacy {
                assert!(candidates::collect(&tree, &source, version).is_none());
                super::tests::assert_full_walk_parity(&tree, &source, version);
            } else {
                let views = extract_diagnostics(&tree, &source, &mut Vec::new(), version);
                assert_eq!(views.diagnostics.len(), expected, "{version:?}: {text}");
                assert_eq!(
                    views.namespace_diagnostics.len(),
                    expected,
                    "{version:?}: {text}"
                );
            }
        }
    }
}

#[test]
fn production_queries_retain_only_disabled_gates_and_generation_limits() {
    let legacy = super::tests::uses_legacy_grammar();
    let source = SourceText::new(
        FileId::new(144),
        "final class C { C(); Object f(Object? value) { if(value case [var item]) {} return [(1, 2), ?value, .named()]; } }",
    );
    let tree = crate::PARSER
        .with(|parser| parser.borrow_mut().parse(source.as_str(), None))
        .unwrap();
    assert!(!tree.root_node().has_error());
    for (version, count, null_aware) in [
        (None, 5, true),
        (Some(DartLanguageVersion::DART_3_0), 2, true),
        (Some(DartLanguageVersion::new(3, 6)), 2, true),
        (Some(DartLanguageVersion::DART_3_8), 1, false),
        (Some(DartLanguageVersion::new(3, 9)), 1, false),
        (Some(DartLanguageVersion::DART_3_10), 1, false),
    ] {
        if legacy {
            assert!(candidates::collect(&tree, &source, version).is_none());
            let (_, occurrences) = super::tests::assert_full_walk_parity(&tree, &source, version);
            assert!(!occurrences.is_empty(), "{version:?}");
        } else {
            let mut occurrences = Vec::new();
            let views = extract_diagnostics(&tree, &source, &mut occurrences, version);
            assert_eq!(occurrences.len(), count, "{version:?}");
            let facts = format!("{occurrences:?}");
            assert!(facts.contains("DotShorthands"), "{version:?}");
            assert_eq!(
                facts.contains("NullAwareCollectionElements"),
                null_aware,
                "{version:?}"
            );
            assert!(
                views
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains("dot shorthands"))
            );
        }
    }
}

#[test]
fn enabled_syntax_features_need_no_occurrences_in_the_production_mode() {
    let source = SourceText::new(
        FileId::new(143),
        "final class C {} Object f(Object? value) { if(value case [var item]) {} return [(1, 2), ?value]; }",
    );
    let tree = crate::PARSER
        .with(|parser| parser.borrow_mut().parse(source.as_str(), None))
        .unwrap();
    assert!(!tree.root_node().has_error());
    let mut full = Vec::new();
    let mut filtered = Vec::new();
    extract_diagnostics(&tree, &source, &mut full, None);
    extract_diagnostics(
        &tree,
        &source,
        &mut filtered,
        Some(DartLanguageVersion::DART_3_10),
    );
    assert!(!full.is_empty());
    if super::tests::uses_legacy_grammar() {
        assert!(
            candidates::collect(&tree, &source, Some(DartLanguageVersion::DART_3_10)).is_none()
        );
        let (_, occurrences) = super::tests::assert_full_walk_parity(
            &tree,
            &source,
            Some(DartLanguageVersion::DART_3_10),
        );
        assert_eq!(format!("{filtered:?}"), format!("{occurrences:?}"));
        assert_eq!(format!("{filtered:?}"), format!("{full:?}"));
    } else {
        assert!(filtered.is_empty());
    }
}
