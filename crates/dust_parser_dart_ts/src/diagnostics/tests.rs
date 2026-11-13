use dust_text::FileId;

use super::*;

/// Both separately linked packages assert their own explicit grammar contract.
pub(super) fn uses_legacy_grammar() -> bool {
    match env!("CARGO_PKG_NAME") {
        "dust_parser_dart_ts" => false,
        "dust_legacy_parser_parity" => true,
        name => panic!("unexpected parser test harness: {name}"),
    }
}

/// Compare both diagnostic views and the first occurrence/order of every feature.
fn assert_parity(text: &str, valid: bool) {
    let source = SourceText::new(FileId::new(23), text);
    let tree = crate::PARSER
        .with(|parser| parser.borrow_mut().parse(source.as_str(), None))
        .unwrap();
    let legacy = uses_legacy_grammar();
    // New grammar forms may be represented differently by the legacy parser.
    if !legacy || !valid {
        assert_eq!(!tree.root_node().has_error(), valid, "{text}");
    }
    if legacy {
        assert!(candidates::collect(&tree, &source, None).is_none());
    } else if valid {
        assert!(
            candidates::collect(&tree, &source, None).is_some(),
            "query must compile"
        );
    }
    assert_full_walk_parity(&tree, &source, None);
}

/// Full-query and fallback paths must preserve both views and every occurrence.
pub(super) fn assert_full_walk_parity(
    tree: &Tree,
    source: &SourceText,
    version: Option<DartLanguageVersion>,
) -> (DiagnosticViews, Vec<FeatureOccurrence>) {
    let mut expected = DiagnosticViews::default();
    let mut expected_occurrences = Vec::new();
    collect_error_diagnostics(
        &mut tree.walk(),
        None,
        source,
        &mut expected,
        &mut expected_occurrences,
        tree.root_node().has_error(),
    );
    let mut actual_occurrences = Vec::new();
    let actual = extract_diagnostics(tree, source, &mut actual_occurrences, version);
    assert_eq!(
        actual.diagnostics,
        expected.diagnostics,
        "{}",
        source.as_str()
    );
    assert_eq!(
        actual.namespace_diagnostics,
        expected.namespace_diagnostics,
        "{}",
        source.as_str()
    );
    assert_eq!(
        format!("{actual_occurrences:?}"),
        format!("{expected_occurrences:?}"),
        "{}",
        source.as_str()
    );
    (actual, actual_occurrences)
}

#[test]
fn native_query_preserves_all_feature_candidates_and_source_preorder() {
    for source in [
        "base class B {} interface class I {} sealed class S {} final class F {} mixin class M {} base mixin N {}",
        "(int, String) record((int, String) value) => (1, 'one');",
        "void f(Object value) { if (value case [var item]) {} final result = switch(value) { _ => 1 }; }",
        "void f(Object value) { var (a, b) = (1, 2); (a, b) = (3, 4); }",
        "void f(Object value) { if (value case {'key': var item}) {} if(value case (var a, var b)) {} }",
        "class C { const C(); } void f(Object value) { if(value case C()) {} if(value case var a as int) {} }",
        "void f(Object value) { if(value case var a?) {} if(value case var a!) {} }",
        "Object f(Object? value) => [?value, if(true) ?value, for(final item in [value]) ?item];",
        "Object f(Object? value) => {?value: ?value, 'key': ?value};",
        "Object f(Object value) => [if(value case int item) item];",
        "class C { C(); C.named(); const C.other(); factory C.make() = C; }",
        "class C { new(); }",
        "class C { const new create(); }",
        "class C { Object f() => .named(); final Object value = .named(); static const marker = .named(); }",
        "class C { void f({Object value = .named()}) {} C({Object value = .named()}); }",
        "Object f() => Type<int>.make();",
        "String f() => '${(1, 2)} ${.named()}';",
        "// .named() case ? final\nconst message = 'case ? final .named()';",
    ] {
        assert_parity(source, true);
    }
}

#[test]
fn invalid_trees_keep_the_original_error_walk_and_guidance_order() {
    for source in [
        "class C { void broken( {}",
        "class C { new.named( ; Object f() => .named(); }",
        "void f() { if(value case ) {} final value = (1, ); }",
    ] {
        assert_parity(source, false);
    }
}
