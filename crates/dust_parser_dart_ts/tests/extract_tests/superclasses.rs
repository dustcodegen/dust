use crate::support::parse;
use dust_parser_dart::{ParsedTypeKind, ParsedTypeSurface};
use dust_text::TextRange;

fn expected(source: &str, fragment: &str, kind: ParsedTypeKind) -> ParsedTypeSurface {
    let start = source.find(fragment).unwrap() as u32;
    ParsedTypeSurface {
        source: fragment.to_owned(),
        span: TextRange::new(start, start + fragment.len() as u32),
        kind,
        nullable: fragment.ends_with('?'),
    }
}

#[test]
fn retains_applied_superclass_identity_arguments_and_exact_spans() {
    for header in ["class Child<T>", "class Child<T>()"] {
        let source = format!(
            "/* π */ {header} extends /* before */ p.Base<Map<String, T?>> /* after */ with Audit<T> {{}}"
        );
        let parsed = parse(133, &source);
        assert_eq!(parsed.diagnostics, []);
        let parameter = expected(
            &source,
            "T?",
            ParsedTypeKind::Named {
                name: "T".into(),
                args: vec![],
            },
        );
        let string = expected(&source, "String", ParsedTypeKind::Builtin("String".into()));
        let map = expected(
            &source,
            "Map<String, T?>",
            ParsedTypeKind::Named {
                name: "Map".into(),
                args: vec![string, parameter],
            },
        );
        let superclass = expected(
            &source,
            "p.Base<Map<String, T?>>",
            ParsedTypeKind::Named {
                name: "p.Base".into(),
                args: vec![map],
            },
        );
        let class = &parsed.library.classes[0];
        assert_eq!(class.superclass, Some(superclass));
        assert_eq!(class.superclass_name.as_deref(), Some("p.Base"));
        assert_eq!(class.mixins[0].source, "Audit<T>");
    }
}

#[test]
fn distinguishes_extends_from_with_only_and_implements() {
    let parsed = parse(
        133,
        "class Plain {} class Mixed<T> with Audit<T> {} class Contract implements Base<int> {} class Root extends Object {}",
    );
    assert_eq!(parsed.diagnostics, []);
    for class in &parsed.library.classes[..3] {
        assert_eq!(class.superclass, None);
        assert_eq!(class.superclass_name, None);
    }
    let root = &parsed.library.classes[3];
    assert_eq!(
        root.superclass.as_ref().unwrap().kind,
        ParsedTypeKind::Builtin("Object".into())
    );
    assert_eq!(root.superclass_name.as_deref(), Some("Object"));
}

#[test]
fn opaque_superclass_shapes_do_not_invent_a_declaration_identity() {
    let parsed = parse(133, "class Invalid extends dynamic {}");
    assert_eq!(parsed.diagnostics, []);
    let class = &parsed.library.classes[0];
    assert_eq!(class.superclass_name, None);
    assert_eq!(
        class.superclass.as_ref().unwrap().kind,
        ParsedTypeKind::Dynamic
    );
}

#[test]
fn qualifier_and_argument_trivia_never_become_type_identity() {
    let source =
        "class Child<T> extends p /* left */ . /* right */ Base<Map /* args */ <String, T?>> {}";
    let parsed = parse(133, source);
    assert_eq!(parsed.diagnostics, []);
    let application = parsed.library.classes[0].superclass.as_ref().unwrap();
    assert_eq!(
        application.source,
        "p /* left */ . /* right */ Base<Map /* args */ <String, T?>>"
    );
    let ParsedTypeKind::Named { name, args } = &application.kind else {
        panic!("expected a named application");
    };
    assert_eq!(name, "p.Base");
    let ParsedTypeKind::Named { name, args } = &args[0].kind else {
        panic!("expected a named argument");
    };
    assert_eq!(name, "Map");
    assert_eq!(args[0].source, "String");
    assert_eq!(args[1].source, "T?");
}
