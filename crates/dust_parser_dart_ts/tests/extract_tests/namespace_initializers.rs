use crate::support::parse;

#[test]
fn instance_initializers_keep_exact_generation_errors_only() {
    for declaration in [
        "class C { Kind value = .ready; }",
        "class C { final Kind value = .ready; }",
        "class C { late Kind value = .ready; }",
        "class C { Kind value = choose(.ready); }",
        "class C { Kind value = true ? .ready : Kind.ready; }",
        "class C { final callback = () => .ready; }",
        "class C { final values = <Kind>[.ready]; }",
        "class C { Kind first = .ready, second = Kind.ready; }",
        "mixin C { Kind value = .ready; }",
        "class C { final callback = () { Kind first = Kind.ready, second = .ready; return second; }; }",
        "class AnimationController { _AnimationDirection _direction = .forward; }",
    ] {
        let source = format!("// @dart=3.13\n{declaration}");
        let result = parse(145, &source);
        assert_eq!(result.namespace_diagnostics, [], "{declaration}");
        assert!(result.has_errors());
        assert_eq!(result.diagnostics.len(), 1, "{declaration}");
        let diagnostic = &result.diagnostics[0];
        assert_eq!(
            diagnostic.message,
            "Dust does not support dot shorthands yet"
        );
        assert_eq!(diagnostic.labels[0].file_id, dust_text::FileId::new(145));
        let label = &diagnostic.labels[0];
        let expression = &source[label.range.start().to_usize()..label.range.end().to_usize()];
        assert!(matches!(expression, ".ready" | ".forward"));
    }
}

#[test]
fn initializers_do_not_hide_headers_defaults_metadata_or_static_fields() {
    for declaration in [
        "class C { static Kind value = .ready; }",
        "class C { static final Kind value = .ready; }",
        "class C { static const Kind value = .ready; }",
        "final Kind value = .ready;",
        "Kind value = .ready;",
        "final callback = () { Kind first = Kind.ready, second = .ready; return second; };",
        "class C { @Marker(.ready) Kind value = Kind.ready; }",
        "class C { final callback = ([Kind value = .ready]) => value; }",
        "class C { final callback = () { @Marker(.ready) final value = Kind.ready; return value; }; }",
        "class C { C([Kind value = .ready]); }",
        "class C { C() : value = .ready; final Kind value; }",
        "class C { C() { Kind value = .ready; } }",
        "class C { new(); }",
    ] {
        let source = format!("// @dart=3.13\n{declaration}");
        let result = parse(145, &source);
        assert!(result.has_errors(), "{declaration}");
        assert_eq!(
            result.namespace_diagnostics, result.diagnostics,
            "{declaration}"
        );
    }
}

#[test]
fn initializer_language_and_syntax_failures_remain_fatal() {
    let result = parse(145, "// @dart=3.6\nclass C { Kind value = .ready; }");
    assert_eq!(result.diagnostics.len(), 2);
    assert_eq!(result.namespace_diagnostics.len(), 1);
    assert_eq!(
        result.namespace_diagnostics[0].message,
        "dot shorthands require Dart 3.10 or newer; current language version is 3.6"
    );
    for source in [
        "class C { Kind value = .ready(,); }",
        "class C { Kind value = choose(.ready, ; }",
    ] {
        let result = parse(145, &format!("// @dart=3.13\n{source}"));
        assert!(
            result
                .namespace_diagnostics
                .iter()
                .any(|error| error.message == "tree-sitter reported a Dart syntax error"),
            "{source}"
        );
    }
}
