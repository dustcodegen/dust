use crate::support::parse;

#[test]
fn valid_executable_bodies_have_distinct_namespace_and_generation_diagnostics() {
    for declaration in [
        "Kind choose() => .ready;",
        "Kind choose() { return .ready; }",
        "Kind get current => .ready;",
        "class C { Kind choose() => .ready; }",
        "class C { Kind get current => .ready; }",
        "class C { set current(Kind value) { Kind next = .ready; } }",
        "class C { Kind operator +(C other) => .ready; }",
        "mixin C { Kind choose() => .ready; }",
        "extension C on Kind { Kind choose() => .ready; }",
        "extension type C(Kind value) { Kind choose() => .ready; }",
        "enum Extra { value; Kind choose() => .ready; }",
        "Kind choose() { final callback = () => .ready; return callback(); }",
        "Kind choose() { Kind local() => .ready; return local(); }",
    ] {
        let source = format!("// @dart=3.13\nenum Kind {{ ready }}\n{declaration}");
        let result = parse(142, &source);
        assert_eq!(result.namespace_diagnostics, [], "{declaration}");
        assert!(result.has_errors());
        assert_eq!(result.diagnostics.len(), 1, "{declaration}");
        assert_eq!(
            result.diagnostics[0].message,
            "Dust does not support dot shorthands yet"
        );
        let label = &result.diagnostics[0].labels[0];
        assert_eq!(label.file_id, dust_text::FileId::new(142));
        assert_eq!(
            &source[label.range.start().to_usize()..label.range.end().to_usize()],
            ".ready"
        );
    }
}

#[test]
fn metadata_initializers_defaults_and_constructor_signatures_stay_namespace_errors() {
    for declaration in [
        "final Kind value = .ready;",
        "@Marker(.ready) class C {}",
        "void choose([Kind value = .ready]) {}",
        "class C { void choose([Kind value = .ready]) {} }",
        "final callback = () => .ready;",
        "class C { new(); }",
        "class C { factory create() = C; }",
        "class C { C() : value = .ready; final Kind value; }",
        "class C { C() { Kind value = .ready; } }",
        "class C { C(@Marker(.ready) Kind value); }",
        "Kind choose() { @Marker(.ready) final value = Kind.ready; return value; }",
        "class C(final Kind value) { this : assert(value != .ready); }",
    ] {
        let source = format!("// @dart=3.13\nenum Kind {{ ready }}\n{declaration}");
        let result = parse(142, &source);
        assert!(result.has_errors(), "{declaration}");
        assert_eq!(
            result.namespace_diagnostics, result.diagnostics,
            "{declaration}"
        );
    }
}

#[test]
fn too_new_dependency_body_syntax_still_has_exact_language_version_errors() {
    let source = "// @dart=3.6\nenum Kind { ready } Kind choose() => .ready;";
    let result = parse(142, source);
    let offset = source.find(".ready").unwrap() as u32;
    assert_eq!(
        result.namespace_diagnostics,
        [dust_diagnostics::Diagnostic::error(
            "dot shorthands require Dart 3.10 or newer; current language version is 3.6"
        )
        .with_label(dust_diagnostics::SourceLabel::new(
            dust_text::FileId::new(142),
            dust_text::TextRange::new(offset, offset + 1),
            "requires Dart 3.10"
        ))]
    );
    assert_eq!(result.diagnostics.len(), 2);
}

#[test]
fn broken_dependency_bodies_remain_namespace_errors() {
    for declaration in [
        "Kind choose() { return .ready; broken( ; }",
        "Kind choose() => .ready(,);",
        "Kind choose() { return .ready;",
    ] {
        let source = format!("// @dart=3.13\nenum Kind {{ ready }}\n{declaration}");
        let result = parse(142, &source);
        assert!(result.has_errors(), "{declaration}");
        assert!(
            result
                .namespace_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message == "tree-sitter reported a Dart syntax error"),
            "{declaration}: {:?}",
            result.namespace_diagnostics
        );
    }
}

#[test]
fn supported_sources_have_no_errors_in_either_view() {
    let result = parse(142, "class C { const C(); int choose() => 1; }");
    assert_eq!(result.diagnostics, []);
    assert_eq!(result.namespace_diagnostics, []);
    assert!(!result.has_errors());
}
