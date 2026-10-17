use crate::support::parse;
use dust_parser_dart::{ParsedClassKind, ParsedTypeKind};
use dust_text::TextRange;

#[test]
fn named_mixin_application_classes_retain_their_identity_and_span() {
    for (name, application) in [
        ("GET", "GET = Base with Mix;"),
        ("Alias", "Alias<T extends Object> = Base with Mix;"),
    ] {
        let source =
            format!("class Base {{ const Base(String path); }} mixin Mix {{}} class {application}");
        let parsed = parse(155, &source);
        assert_eq!(parsed.diagnostics, []);
        assert_eq!(
            parsed
                .library
                .classes
                .iter()
                .map(|class| class.name.as_str())
                .collect::<Vec<_>>(),
            ["Base", name]
        );
        assert_eq!(
            parsed.library.classes[1].span,
            TextRange::new(source.rfind("class ").unwrap() as u32, source.len() as u32)
        );
        assert_eq!(
            parsed.library.classes[1].kind,
            ParsedClassKind::MixinApplicationClass
        );
        assert_eq!(parsed.library.classes[1].fields, []);
        assert_eq!(parsed.library.classes[1].constructors, []);
        assert_eq!(parsed.library.classes[1].methods, []);
    }
}

#[test]
fn named_mixin_headers_keep_structured_applications_binders_and_source_spans() {
    let source = "@Tag() abstract class Alias<T extends Object> = p /* base */ . Base<List<T>> with p /* mixin */ . Mix<T>, Extra;";
    let parsed = parse(155, source);
    assert_eq!(parsed.diagnostics, []);
    let class = &parsed.library.classes[0];
    assert_eq!(class.name, "Alias");
    assert_eq!(class.kind, ParsedClassKind::MixinApplicationClass);
    assert!(class.is_abstract);
    assert_eq!(class.annotations[0].name, "Tag");
    let slice = |span: TextRange| &source[span.start().to_usize()..span.end().to_usize()];
    assert_eq!(slice(class.span), source);
    let parameter = &class.type_parameters[0];
    assert_eq!(parameter.name, "T");
    assert_eq!(slice(parameter.span), "T extends Object");
    assert_eq!(slice(parameter.name_span), "T");
    assert_eq!(slice(parameter.bound.as_ref().unwrap().span), "Object");
    let base = class.superclass.as_ref().unwrap();
    assert_eq!(class.superclass_name.as_deref(), Some("p.Base"));
    assert_eq!(slice(base.span), "p /* base */ . Base<List<T>>");
    assert_eq!(base.source, slice(base.span));
    let ParsedTypeKind::Named { name, args } = &base.kind else {
        panic!("structured base missing")
    };
    assert_eq!(name, "p.Base");
    assert_eq!(args[0].source, "List<T>");
    let ParsedTypeKind::Named { name, args } = &args[0].kind else {
        panic!("structured base argument missing")
    };
    assert_eq!(name, "List");
    assert_eq!(args[0].source, "T");
    assert_eq!(slice(args[0].span), "T");
    assert_eq!(
        class
            .mixins
            .iter()
            .map(|mixin| (mixin.source.as_str(), slice(mixin.span)))
            .collect::<Vec<_>>(),
        [
            ("p /* mixin */ . Mix<T>", "p /* mixin */ . Mix<T>"),
            ("Extra", "Extra")
        ]
    );
    let ParsedTypeKind::Named { name, args } = &class.mixins[0].kind else {
        panic!("structured mixin missing")
    };
    assert_eq!(name, "p.Mix");
    assert_eq!(args[0].source, "T");
    let scope = &parsed.library.lexical_scopes[1];
    assert_eq!(scope.name.as_ref().unwrap().name, "Alias");
    assert_eq!(scope.declarations[0].name.name, "T");
    assert_eq!(slice(scope.declarations[0].name.span), "T");
}
