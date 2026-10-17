#![cfg(test)]

use super::lower_library;
use dust_ir::{ClassKindIr, MixinApplicationIr, NameIr, SpanIr, SuperclassIr, TypeIr, TypeParamIr};
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{SymbolCatalog, resolve_library};
use dust_text::{FileId, SourceText, TextRange};

/// Lowers fresh source through the real parser and resolver.
fn lower(text: &str) -> dust_ir::DartFileIr {
    let source = SourceText::new(FileId::new(93), text);
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let mut resolved = resolve_library(
        FileId::new(93),
        "lib/box.dart",
        "lib/box.g.dart",
        &parsed.library,
        &SymbolCatalog::new(),
    );
    assert_eq!(resolved.diagnostics, []);
    let lowered = lower_library(&mut resolved.library);
    assert_eq!(lowered.diagnostics, []);
    lowered.value
}

/// Builds an expected source location from an unambiguous source fragment.
fn span(text: &str, fragment: &str) -> SpanIr {
    let start = text.find(fragment).unwrap() as u32;
    SpanIr::new(
        FileId::new(93),
        TextRange::new(start, start + fragment.len() as u32),
    )
}

#[test]
fn named_mixin_applications_keep_typed_header_identity_without_inventing_constructors() {
    let text = "class Alias<T extends Object> = p.Base<List<T>> with p.Mix<T>;";
    let file = lower(text);
    let class = &file.classes[0];
    assert_eq!(class.kind, ClassKindIr::MixinApplicationClass);
    assert_eq!(class.name, "Alias");
    assert_eq!(class.type_parameters[0].name.source, "T");
    assert_eq!(class.type_parameters[0].bound, Some(TypeIr::object()));
    assert_eq!(class.type_parameters[0].name.span, span(text, "T"));
    assert_eq!(
        class.type_parameters[0].span,
        span(text, "T extends Object")
    );
    assert_eq!(
        class.superclass,
        Some(SuperclassIr {
            ty: TypeIr::generic("p.Base", vec![TypeIr::list_of(TypeIr::named("T"))]),
            span: span(text, "p.Base<List<T>>"),
        })
    );
    assert_eq!(
        class.mixins,
        [MixinApplicationIr {
            ty: TypeIr::generic("p.Mix", vec![TypeIr::named("T")]),
            span: span(text, "p.Mix<T>"),
        }]
    );
    assert_eq!(class.constructors, []);
    assert_eq!(class.fields, []);
}

#[test]
fn preserves_class_parameters_bounds_and_mixin_types_in_ir() {
    let text = "class const Box<T, K extends Object, V extends Map<K, List<T?>>>(final T value) with Audit<T>, _$Box<T, K, V>;";
    let file = lower(text);
    let class = &file.classes[0];
    assert_eq!(
        class
            .type_parameters
            .iter()
            .map(|p| (&*p.name.source, p.bound.clone()))
            .collect::<Vec<_>>(),
        [
            ("T", None),
            ("K", Some(TypeIr::object())),
            (
                "V",
                Some(TypeIr::map_of(
                    TypeIr::named("K"),
                    TypeIr::list_of(TypeIr::named("T").nullable())
                ))
            ),
        ]
    );
    assert_eq!(
        class.type_parameters[1],
        TypeParamIr {
            name: NameIr {
                source: "K".to_owned(),
                short: "K".to_owned(),
                prefix: None,
                span: span(text, "K")
            },
            bound: Some(TypeIr::object()),
            span: span(text, "K extends Object"),
        }
    );
    assert_eq!(
        class.mixins,
        [
            MixinApplicationIr {
                ty: TypeIr::generic("Audit", vec![TypeIr::named("T")]),
                span: span(text, "Audit<T>")
            },
            MixinApplicationIr {
                ty: TypeIr::generic(
                    "_$Box",
                    vec![TypeIr::named("T"), TypeIr::named("K"), TypeIr::named("V")]
                ),
                span: span(text, "_$Box<T, K, V>")
            },
        ]
    );
    assert_eq!(class.fields[0].ty, TypeIr::named("T"));
    assert_eq!(class.constructors[0].params[0].ty, TypeIr::named("T"));
}

#[test]
fn class_binders_shadow_builtins_through_members_and_nested_types() {
    let file = lower(
        "class Box<String> { const Box(this.value, this.values); final String value; final List<String?> values; String echo(String input) => input; }",
    );
    let class = &file.classes[0];
    assert_eq!(class.fields[0].ty, TypeIr::named("String"));
    assert_eq!(
        class.fields[1].ty,
        TypeIr::list_of(TypeIr::named("String").nullable())
    );
    assert_eq!(class.constructors[0].params[0].ty, TypeIr::named("String"));
    assert_eq!(class.methods[0].return_type, TypeIr::named("String"));
    assert_eq!(class.methods[0].params[0].ty, TypeIr::named("String"));

    let primary = lower("class const Box<String>(final String value) with _$Box<String>;");
    assert_eq!(primary.classes[0].fields[0].ty, TypeIr::named("String"));
    assert_eq!(
        primary.classes[0].constructors[0].params[0].ty,
        TypeIr::named("String")
    );
}

#[test]
fn all_class_binders_are_in_scope_in_recursive_and_dependent_bounds() {
    let file = lower(
        "class Box<String extends Comparable<String>, T extends List<String?>, U extends T> {}",
    );
    let params = &file.classes[0].type_parameters;
    assert_eq!(
        params.iter().map(|p| p.bound.clone()).collect::<Vec<_>>(),
        [
            Some(TypeIr::generic("Comparable", vec![TypeIr::named("String")])),
            Some(TypeIr::list_of(TypeIr::named("String").nullable())),
            Some(TypeIr::named("T")),
        ]
    );

    let forward = lower("class Box<T extends U, U extends num> {}");
    assert_eq!(
        forward.classes[0]
            .type_parameters
            .iter()
            .map(|p| p.bound.clone())
            .collect::<Vec<_>>(),
        [Some(TypeIr::named("U")), Some(TypeIr::num())]
    );
}

#[test]
fn non_generic_classes_keep_builtin_types_and_empty_parameter_lists() {
    let file = lower("class Plain { const Plain(this.value); final String value; }");
    assert_eq!(file.classes[0].type_parameters, []);
    assert_eq!(file.classes[0].mixins, []);
    assert_eq!(file.classes[0].fields[0].ty, TypeIr::string());
    assert_eq!(
        file.classes[0].constructors[0].params[0].ty,
        TypeIr::string()
    );
}

#[test]
fn primary_callback_types_preserve_function_local_binders() {
    let file = lower(
        "class const Box<String>(final String Function<String>(String) transform) with _$Box<String>;",
    );
    let class = &file.classes[0];
    let expected = TypeIr::function("String Function<String>(String)");
    assert_eq!(class.fields[0].ty, expected);
    assert_eq!(class.constructors[0].params[0].ty, expected);
    assert_eq!(class.type_parameters[0].name.source, "String");
}
