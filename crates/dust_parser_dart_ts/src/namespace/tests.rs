use dust_parser_dart::{
    DartLanguageVersion, ParseBackend, ParseOptions, ParseResult,
    ParsedLexicalDeclarationKind as Declaration, ParsedLexicalInitializerKind as Initializer,
};
use dust_text::{FileId, SourceText};

use crate::TreeSitterDartBackend;

fn parse(
    text: &str,
    version: DartLanguageVersion,
    annotations: &[&str],
) -> (ParseResult, ParseResult) {
    let source = SourceText::new(FileId::new(7), text);
    let options = ParseOptions {
        language_version: version,
        ..Default::default()
    };
    let backend = TreeSitterDartBackend::new();
    (
        backend.parse_file(&source, options),
        backend.parse_namespace_file(&source, options, annotations),
    )
}

#[test]
fn namespace_projection_preserves_identity_aliases_constants_and_diagnostics() {
    let source = "import 'marker.dart' as m; export 'other.dart' show Other; part 'part.dart';
      typedef Alias = m.Marker;
      const marker = Holder.value;
      @Deprecated('old') class Holder<T> {
        const Holder(this.field); final T field;
        static const value = m.Marker.named();
        String method<U>(U input) => '$input';
      }
      enum Kind { first, second }
      mixin M { final int value = 1; }
      extension E on String {}
      extension type Box(int value) {}
      void function(int value) {}";
    let (mut full, projected) = parse(source, DartLanguageVersion::DART_3_12, &["Marker"]);
    assert_eq!(full.diagnostics, [], "{full:?}");
    assert!(!full.library.classes[0].fields.is_empty());
    assert!(!full.library.classes[0].methods.is_empty());
    assert!(projected.library.classes[0].fields.is_empty());
    assert!(projected.library.classes[0].methods.is_empty());
    assert_eq!(
        projected.library.lexical_scopes,
        full.library.lexical_scopes
    );
    // Class generation payload is the only intentional difference in this source.
    for class in &mut full.library.classes {
        class.type_parameters.clear();
        class.mixins.clear();
        class.superclass = None;
        class.superclass_name = None;
        class.annotations.clear();
        class.fields.clear();
        class.constructors.clear();
        class.methods.clear();
    }
    assert_eq!(projected, full);
}

#[test]
fn supported_metadata_at_every_site_keeps_the_complete_generation_surface() {
    for source in [
        "@p.Marker() library external; class C { int value = 1; }",
        "@p.Marker() class C { int value = 1; }",
        "class C { @p.Marker() int value = 1; }",
        "class C { @p.Marker() C(); int value = 1; }",
        "class C { C(@p.Marker() int value); }",
        "class C { @p.Marker() void method() {} }",
        "class C { void method(@p.Marker() int value) {} }",
        "class C { void method(void callback(@p.Marker() int value)) {} }",
        "enum E { @p.Marker() value }",
        "@p.Marker() mixin M {}",
        "@p.Marker() extension E on String {}",
        "@p.Marker() extension type E(int value) {}",
        "@p.Marker() typedef Alias = String;",
        "@p.Marker() const value = 1;",
        "void function(@p.Marker() int value) {}",
    ] {
        let (full, projected) = parse(source, DartLanguageVersion::DART_3_12, &["Marker"]);
        assert_eq!(projected, full, "{source}");
    }
}

#[test]
fn projection_retains_both_diagnostic_views_for_syntax_and_language_features() {
    for (version, source, primary, legacy) in [
        (
            DartLanguageVersion::DART_2_17,
            "class C { Object method() => (1, 2); }",
            (1, 1),
            (1, 1),
        ),
        (
            DartLanguageVersion::DART_3_0,
            "extension type E(int value) {}",
            (1, 1),
            (1, 1),
        ),
        (
            DartLanguageVersion::DART_3_12,
            "class C { Object method() => .named(); }",
            (1, 0),
            (0, 0),
        ),
        (
            DartLanguageVersion::DART_3_12,
            "class C { final Object value = .named(); }",
            (1, 0),
            (0, 0),
        ),
        (
            DartLanguageVersion::DART_3_12,
            "class C { new.named(); }",
            (2, 2),
            (0, 0),
        ),
        (
            DartLanguageVersion::DART_3_12,
            "class C { void broken( {}",
            (1, 1),
            (1, 1),
        ),
        (
            DartLanguageVersion::DART_3_12,
            "// @dart=2.17\nclass C { Object method() => (1, 2); }",
            (1, 1),
            (1, 1),
        ),
    ] {
        let (full, projected) = parse(source, version, &[]);
        // Legacy 0.1 lacks the listed grammar's dot-shorthand and concise nodes.
        let expected = match env!("CARGO_PKG_NAME") {
            "dust_parser_dart_ts" => primary,
            "dust_legacy_parser_parity" => legacy,
            grammar => panic!("unexpected parser grammar harness: {grammar}"),
        };
        assert_eq!(
            (full.diagnostics.len(), full.namespace_diagnostics.len()),
            expected,
            "{source}"
        );
        assert_eq!(projected.diagnostics, full.diagnostics, "{source}");
        assert_eq!(
            projected.namespace_diagnostics, full.namespace_diagnostics,
            "{source}"
        );
    }
}

#[test]
fn parameter_language_diagnostics_keep_complete_extraction_and_original_order() {
    for source in [
        "class C { C({int? _value}); }",
        "class C { void method({int? _value}) {} }",
        "void function({int? _value}) {}",
        "class C { C({@deprecated int? _value}); void method({int? _other}) {} }",
        "class C { C(); void method(final int value) {} }",
        "class C { C(var value); }",
    ] {
        for version in [
            DartLanguageVersion::DART_3_0,
            DartLanguageVersion::DART_3_13,
        ] {
            let (full, projected) = parse(source, version, &[]);
            assert_eq!(
                projected.diagnostics, full.diagnostics,
                "{version}: {source}"
            );
            assert_eq!(
                projected.namespace_diagnostics, full.namespace_diagnostics,
                "{version}: {source}"
            );
            if version == DartLanguageVersion::DART_3_13 || source.contains('_') {
                assert_eq!(projected, full, "{version}: {source}");
            }
        }
    }
}

#[test]
fn source_language_comment_keeps_future_parameter_validation() {
    let source = "// @dart=3.13\nclass C { C(); void method(final int value) {} }";
    let (full, projected) = parse(source, DartLanguageVersion::DART_3_0, &[]);
    assert!(!full.diagnostics.is_empty());
    assert_eq!(projected, full);
}

#[test]
fn namespace_constants_keep_exact_provenance_without_constructor_arguments() {
    let source = "const top = d.Marker(1, name: 'top');
      class C { static const marker = d.Marker<int>([1, 2], name: 'class'), alias = top, opaque = marker ?? top; }
      mixin M { static const marker = d.Marker(2); }
      enum E { value; static const marker = d.Marker(3); }
      extension X on String { static const marker = d.Marker(4); }
      extension type Y(int value) { static const marker = d.Marker(5); }";
    let (mut full, projected) = parse(source, DartLanguageVersion::DART_3_12, &[]);
    assert_eq!(full.diagnostics, []);
    let mut calls = 0;
    for scope in &mut full.library.lexical_scopes {
        for declaration in &mut scope.declarations {
            if let Declaration::Constant(Some(initializer)) = &mut declaration.kind
                && let Initializer::Constructor { arguments, .. } = &mut initializer.kind
            {
                assert!(!arguments.positional.is_empty());
                *arguments = Default::default();
                calls += 1;
            }
        }
    }
    assert_eq!(calls, 6);
    // Source, byte spans, scope parents, names, kind and callees remain exact.
    assert_eq!(
        projected.library.lexical_scopes,
        full.library.lexical_scopes
    );
    assert_eq!(projected.diagnostics, full.diagnostics);
    assert_eq!(projected.namespace_diagnostics, full.namespace_diagnostics);
}
