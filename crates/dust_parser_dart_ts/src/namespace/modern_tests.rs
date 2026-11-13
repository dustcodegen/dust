use dust_parser_dart::DartLanguageVersion;

use super::tests::parse;

#[test]
fn modern_ordinary_classes_project_without_losing_namespace_facts() {
    let fields = (0..128)
        .map(|index| {
            format!("final int value{index} = {index}; static const alias{index} = {index};")
        })
        .collect::<String>();
    let source = format!(
        "class Icons {{ const Icons(this.value); final int value; {fields}
         int method(int input) => input; }}"
    );
    let (full, projected) = parse(&source, DartLanguageVersion::DART_3_13, &[]);
    assert_eq!(full.diagnostics, []);
    assert_eq!(projected.diagnostics, full.diagnostics);
    assert_eq!(projected.namespace_diagnostics, full.namespace_diagnostics);
    assert_eq!(
        projected.library.lexical_scopes,
        full.library.lexical_scopes
    );
    assert_eq!(projected.library.classes[0].name, "Icons");
    assert_eq!(
        projected.library.classes[0].span,
        full.library.classes[0].span
    );
    assert_eq!(full.library.classes[0].fields.len(), 129);
    assert_eq!(full.library.classes[0].methods.len(), 1);
    assert!(projected.library.classes[0].fields.is_empty());
    assert!(projected.library.classes[0].constructors.is_empty());
    assert!(projected.library.classes[0].methods.is_empty());
}

#[test]
fn invalid_modifiers_retain_complete_facts_and_both_error_views() {
    for source in [
        "class C { void method(final int value) {} }",
        "class C { C(var value); }",
        "class C { void method(covariant final int value) {} }",
        "class C { void method({required final int value}) {} }",
        "class C { void method(@Deprecated('old') final int value) {} }",
        "class C { void method({@Deprecated('old') covariant final int value = 1}) {} }",
        "class C { void method({var value = 1}) {} }",
        "// @dart=3.13\nclass C { void method(final int value) {} }",
    ] {
        let (full, projected) = parse(source, DartLanguageVersion::DART_3_13, &[]);
        assert!(!full.diagnostics.is_empty(), "{source}");
        assert_eq!(projected, full, "{source}");
    }
}

#[test]
fn callback_modifiers_use_conservative_full_facts_without_new_errors() {
    for source in [
        "class C { void method(void callback(final int value)) {} }",
        "class C { void method(void callback(@Deprecated('old') var value)) {} }",
        "class C { C(void callback({covariant final int value = 1})); }",
    ] {
        let (full, projected) = parse(source, DartLanguageVersion::DART_3_13, &[]);
        // Nested callback scopes retain the existing full parser's diagnostic contract.
        assert_eq!(projected, full, "{source}");
    }
}

#[test]
fn primary_constructors_keep_complete_payloads() {
    for source in [
        "class C(final int value) {}",
        "class C(final void callback(int value)) {}",
    ] {
        let (full, projected) = parse(source, DartLanguageVersion::DART_3_13, &[]);
        assert_eq!(projected, full, "{source}");
    }
}

#[test]
fn valid_constructor_defaults_preserve_namespace_facts_without_generation_payloads() {
    for source in [
        "class C { C([int value = 1]); final int field = 1; }",
        "// @dart=3.13\nclass C { const C({String value = 'literal .dot'}); final int field = 1; }",
        "enum Kind { ready } class C { C({Kind value = Kind.ready}); final int field = 1; }",
        "class Holder { static const value = 1; } class C { C({int value = Holder.value}); final int field = 1; }",
        "class C { factory C({int value = 1}) = D; } class D extends C {}",
    ] {
        let (full, projected) = parse(source, DartLanguageVersion::DART_3_13, &[]);
        assert_eq!(full.diagnostics, [], "{source}");
        assert_eq!(projected.diagnostics, full.diagnostics, "{source}");
        assert_eq!(
            projected.namespace_diagnostics, full.namespace_diagnostics,
            "{source}"
        );
        assert_eq!(
            projected.library.lexical_scopes, full.library.lexical_scopes,
            "{source}"
        );
        let original = full
            .library
            .classes
            .iter()
            .find(|class| class.name == "C")
            .unwrap();
        assert!(
            original.constructors[0].params[0]
                .default_value_source
                .is_some()
        );
        if super::has_dot_shorthand_nodes() {
            let class = projected
                .library
                .classes
                .iter()
                .find(|class| class.name == "C")
                .unwrap();
            assert_eq!(class.span, original.span);
            assert!(class.fields.is_empty(), "{source}");
            assert!(class.constructors.is_empty(), "{source}");
        } else {
            // The legacy grammar lacks diagnostic dot nodes and keeps defaults conservatively.
            assert_eq!(projected, full, "{source}");
        }
    }
}

#[test]
fn modern_projection_preserves_shadowing_metadata_and_effective_language() {
    for source in [
        "class C<Marker> { void method(@Marker() int value) {} }",
        "class C { static const Marker = Other(); void method(@Marker int value) {} }",
        "class C { void method(void callback(@Marker() int value)) {} }",
    ] {
        let (full, projected) = parse(source, DartLanguageVersion::DART_3_13, &["Marker"]);
        assert_eq!(projected, full, "{source}");
    }
    for source in [
        "// @dart=3.13\nclass C { void method(final int value) {} }",
        "// @dart=3.0\nclass C { void method({int? _value}) {} }",
    ] {
        let (full, projected) = parse(source, DartLanguageVersion::DART_3_0, &[]);
        assert!(!full.diagnostics.is_empty(), "{source}");
        assert_eq!(projected, full, "{source}");
    }
}
