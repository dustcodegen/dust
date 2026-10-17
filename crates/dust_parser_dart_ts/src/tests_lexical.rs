use dust_parser_dart::{
    ParseBackend, ParseOptions, ParsedDartFileSurface, ParsedLexicalDeclarationKind as Declaration,
    ParsedLexicalInitializerKind as Initializer, ParsedLexicalScopeKind as Scope,
};
use dust_text::{FileId, SourceText};

use super::TreeSitterDartBackend;

/// Exercise the real native parser while keeping exact source-coordinate checks.
fn parse(text: &str) -> ParsedDartFileSurface {
    let source = SourceText::new(FileId::new(0), text);
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    parsed.library
}

#[test]
fn lexical_scopes_keep_constants_members_and_generic_binders_separate() {
    let source = "const top = Object(); class Box<T> { static const marker = Object(); final int value = 0; void call<U>(int marker) {} }";
    let parsed = parse(source);
    let scopes = &parsed.lexical_scopes;
    assert_eq!(
        scopes
            .iter()
            .map(|scope| (
                scope.parent,
                scope.kind,
                scope.name.as_ref().map(|name| name.name.as_str()),
                scope
                    .declarations
                    .iter()
                    .map(|declaration| declaration.name.name.as_str())
                    .collect::<Vec<_>>(),
            ))
            .collect::<Vec<_>>(),
        [
            (None, Scope::Library, None, vec!["top"]),
            (
                Some(0),
                Scope::Type,
                Some("Box"),
                vec!["T", "marker", "value", "call"]
            ),
            (Some(1), Scope::Callable, Some("call"), vec!["U"]),
        ]
    );
    assert_eq!(scopes[1].declarations[0].kind, Declaration::TypeParameter);
    assert_eq!(scopes[1].declarations[2].kind, Declaration::Member);
    for scope in scopes {
        for declaration in &scope.declarations {
            let span = declaration.name.span;
            assert_eq!(
                &source[span.start().to_usize()..span.end().to_usize()],
                declaration.name.name
            );
            if let Declaration::Constant(Some(value)) = &declaration.kind {
                assert_eq!(
                    &source[value.span.start().to_usize()..value.span.end().to_usize()],
                    value.source
                );
            }
        }
    }
}

#[test]
fn constant_initializers_retain_callee_arguments_references_and_opaque_shapes() {
    let parsed = parse(
        "class Markers { static const first = foreign /* q */ . /* selector */ Marker<int>('key', flag: true), alias = Markers /* q */ . first, opaque = true ? first : alias; }",
    );
    let declarations = &parsed.lexical_scopes[1].declarations;
    assert_eq!(declarations.len(), 3);
    let Declaration::Constant(Some(first)) = &declarations[0].kind else {
        panic!("constant initializer missing")
    };
    let Initializer::Constructor { callee, arguments } = &first.kind else {
        panic!("constructor identity missing: {first:?}")
    };
    assert_eq!(
        callee
            .iter()
            .map(|name| name.name.as_str())
            .collect::<Vec<_>>(),
        ["foreign", "Marker"]
    );
    assert_eq!(arguments.positional[0].source, "'key'");
    assert_eq!(arguments.named[0].name, "flag");
    assert_eq!(arguments.named[0].value_source, "true");
    assert_eq!(
        first.source,
        "foreign /* q */ . /* selector */ Marker<int>('key', flag: true)"
    );
    let Declaration::Constant(Some(alias)) = &declarations[1].kind else {
        panic!("reference initializer missing")
    };
    let Initializer::Reference(names) = &alias.kind else {
        panic!("reference identity missing")
    };
    assert_eq!(
        names
            .iter()
            .map(|name| name.name.as_str())
            .collect::<Vec<_>>(),
        ["Markers", "first"]
    );
    let Declaration::Constant(Some(opaque)) = &declarations[2].kind else {
        panic!("opaque initializer missing")
    };
    assert_eq!(opaque.kind, Initializer::Opaque);
}

#[test]
fn non_class_type_bodies_retain_their_own_local_constants() {
    let parsed = parse(
        "mixin Mix { static const marker = Object(); } enum Kind { first; static const marker = Object(); } extension Extra on int { static const marker = Object(); } extension type Other(int value) { static const marker = Object(); }",
    );
    assert_eq!(
        parsed
            .lexical_scopes
            .iter()
            .skip(1)
            .map(|scope| (
                scope.name.as_ref().map(|name| name.name.as_str()),
                scope.parent,
                scope.kind
            ))
            .collect::<Vec<_>>(),
        [
            (Some("Mix"), Some(0), Scope::Type),
            (Some("Kind"), Some(0), Scope::Type),
            (Some("Extra"), Some(0), Scope::Type),
            (Some("Other"), Some(0), Scope::Type),
        ]
    );
    for scope in parsed.lexical_scopes.iter().skip(1) {
        assert_eq!(scope.declarations.last().unwrap().name.name, "marker");
    }
}

#[test]
fn external_fields_accessors_and_nonconstant_values_remain_only_shadows() {
    let parsed = parse(
        "abstract class Fields { external final int first, second; final Object value = const Object(); int get label => 0; set label(int value) {} void call() { const local = Object(); } }",
    );
    let declarations = &parsed.lexical_scopes[1].declarations;
    assert_eq!(
        declarations
            .iter()
            .map(|decl| (decl.name.name.as_str(), &decl.kind))
            .collect::<Vec<_>>(),
        [
            ("first", &Declaration::Member),
            ("second", &Declaration::Member),
            ("value", &Declaration::Member),
            ("label", &Declaration::Member),
            ("label", &Declaration::Member),
            ("call", &Declaration::Member),
        ]
    );
}

#[test]
fn constructor_arguments_do_not_turn_other_constant_expressions_into_calls() {
    let parsed = parse(
        "class Markers { static const cascade = Object()..toString(), collection = [Object()], grouped = (Object()), number = -1; }",
    );
    assert_eq!(
        parsed.lexical_scopes[1]
            .declarations
            .iter()
            .map(|decl| {
                let Declaration::Constant(Some(value)) = &decl.kind else {
                    panic!("constant value missing")
                };
                (decl.name.name.as_str(), &value.kind)
            })
            .collect::<Vec<_>>(),
        [
            ("cascade", &Initializer::Opaque),
            ("collection", &Initializer::Opaque),
            ("grouped", &Initializer::Opaque),
            ("number", &Initializer::Opaque),
        ]
    );
}
