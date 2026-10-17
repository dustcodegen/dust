use crate::support::parse;
use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_text::{FileId, TextRange};

fn generation_error(source: &str) -> Diagnostic {
    let start = source.find(".ready").unwrap() as u32;
    Diagnostic::error("Dust does not support dot shorthands yet")
        .with_label(SourceLabel::new(FileId::new(148), TextRange::new(start, start + 6),
            "unsupported code generation"))
        .with_note("Check this file with `dart analyze`. Dust's bundled parser accepts this syntax, but code generation is not implemented yet.")
        .with_note("Use an explicit type name instead of a dot shorthand.")
}

#[test]
fn declared_callable_defaults_keep_exact_root_errors_only() {
    for declaration in [
        "Kind choose([Kind value = .ready]) => value;",
        "Kind choose({Kind value = .ready}) => value;",
        "Kind choose({int other = 1, Kind value = .ready}) => value;",
        "class C { Kind choose([Kind value = .ready]) => value; }",
        "class C { static Kind choose({Kind value = .ready}) => value; }",
        "abstract class C { Kind choose({Kind value = .ready}); }",
        "class C { external Kind choose({Kind value = .ready}); }",
        "mixin C { Kind choose({Kind value = .ready}) => value; }",
        "extension C on Kind { Kind choose({Kind value = .ready}) => value; }",
        "enum C { value; Kind choose({Kind value = .ready}) => value; }",
        "Kind choose([List<Kind> value = const [.ready]]) => value.first;",
        "Kind choose([Holder value = const Holder(.ready)]) => value.kind;",
        "Kind choose([Kind value /* default */ = /* value */ .ready]) => value;",
        "Kind choose() { Kind local([Kind value = .ready]) => value; return local(); }",
    ] {
        let source = format!("// @dart=3.13\n{declaration}");
        let result = parse(148, &source);
        assert_eq!(result.namespace_diagnostics, [], "{declaration}");
        assert_eq!(
            result.diagnostics,
            [generation_error(&source)],
            "{declaration}"
        );
        assert!(result.has_errors());
    }
}

#[test]
fn constructor_and_nested_metadata_or_callable_type_defaults_stay_strict() {
    for declaration in [
        "class C { C([Kind value = .ready]); }",
        "class C { const C({Kind value = .ready}); }",
        "class C { factory C([Kind value = .ready]) = D; }",
        "class C({final Kind value = .ready});",
        "@Marker(.ready) void choose([Kind value = Kind.ready]) {}",
        "void choose([@Marker(.ready) Kind value = Kind.ready]) {}",
        "void choose({@Marker(.ready) Kind value = Kind.ready}) {}",
        "void choose({Kind value: .ready}) {}",
        "void choose(void callback([Kind value = .ready])) {}",
        "void choose([void Function([Kind value = .ready])? callback]) {}",
        "final callback = ([Kind value = .ready]) => value;",
        "Kind choose() { final callback = ([Kind value = .ready]) => value; return callback(); }",
        "class C { C() : value = .ready; final Kind value; }",
        "Kind value = .ready;",
    ] {
        let source = format!("// @dart=3.13\n{declaration}");
        let result = parse(148, &source);
        assert!(result.has_errors(), "{declaration}");
        assert_eq!(
            result.namespace_diagnostics, result.diagnostics,
            "{declaration}"
        );
    }
}

#[test]
fn callable_default_syntax_and_version_errors_remain_namespace_failures() {
    let source = "// @dart=3.6\nKind choose([Kind value = .ready]) => value;";
    let result = parse(148, source);
    let start = source.find(".ready").unwrap() as u32;
    assert_eq!(result.diagnostics[0], generation_error(source));
    assert_eq!(
        result.namespace_diagnostics,
        [Diagnostic::error(
            "dot shorthands require Dart 3.10 or newer; current language version is 3.6"
        )
        .with_label(SourceLabel::new(
            FileId::new(148),
            TextRange::new(start, start + 1),
            "requires Dart 3.10"
        ))]
    );
    assert_eq!(result.diagnostics.len(), 2);
    for declaration in [
        "void choose([Kind value = .ready(,)]) {}",
        "void choose([Kind value = .ready) {}",
        "void choose([Kind value = .ready]) { broken( ; }",
        "void choose([@Marker(.ready(,)) Kind value = Kind.ready]) {}",
    ] {
        let source = format!("// @dart=3.13\n{declaration}");
        let result = parse(148, &source);
        assert!(
            result
                .namespace_diagnostics
                .iter()
                .any(|error| error.message == "tree-sitter reported a Dart syntax error"),
            "{declaration}"
        );
    }
}

#[test]
fn dependency_default_projection_keeps_callable_parameter_facts() {
    let source = "// @dart=3.13\nclass C { void choose({int before = 1, @Other() Kind value = .ready, int after = 2}) {} }";
    let result = parse(148, source);
    assert_eq!(result.namespace_diagnostics, []);
    assert_eq!(result.diagnostics, [generation_error(source)]);
    let parameters = &result.library.classes[0].methods[0].params;
    assert_eq!(
        parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>(),
        ["before", "value", "after"]
    );
    let parameter = &parameters[1];
    assert_eq!(parameter.type_source.as_deref(), Some("Kind"));
    assert_eq!(parameter.parsed_type.as_ref().unwrap().source, "Kind");
    assert_eq!(parameter.kind, dust_parser_dart::ParameterKind::Named);
    assert!(!parameter.is_required);
    assert!(parameter.has_default);
    assert_eq!(parameter.default_value_source.as_deref(), Some(".ready"));
    assert_eq!(parameter.annotations.len(), 1);
    assert_eq!(parameter.annotations[0].name, "Other");
    assert_eq!(
        &source[parameter.span.start().to_usize()..parameter.span.end().to_usize()],
        "Kind value"
    );
}
