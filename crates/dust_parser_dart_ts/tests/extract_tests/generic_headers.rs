//! Generic header facts must survive without changing source text or spans.

use crate::support::parse;
use dust_parser_dart::{ParsedTypeKind, ParsedTypeSurface};
use dust_text::TextRange;

fn slice(source: &str, span: TextRange) -> &str {
    &source[u32::from(span.start()) as usize..u32::from(span.end()) as usize]
}

fn named(
    text: &str,
    start: u32,
    name: &str,
    args: Vec<ParsedTypeSurface>,
    nullable: bool,
) -> ParsedTypeSurface {
    ParsedTypeSurface {
        source: text.to_owned(),
        span: TextRange::new(start, start + text.len() as u32),
        kind: ParsedTypeKind::Named {
            name: name.to_owned(),
            args,
        },
        nullable,
    }
}

#[test]
fn preserves_ordered_parameters_bounds_and_mixin_arguments() {
    let source = "/* π */ class Box<T, K extends Object, V extends Map<K, List<T?>>> extends Base with Audit<T>, p.Store<K, V> implements Hidden {}";
    let parsed = parse(393, source);
    assert_eq!(parsed.diagnostics, []);
    let class = &parsed.library.classes[0];
    assert_eq!(
        class
            .type_parameters
            .iter()
            .map(|p| (
                p.name.as_str(),
                p.bound.as_ref().map(|b| b.source.as_str()),
                slice(source, p.span),
                slice(source, p.name_span),
            ))
            .collect::<Vec<_>>(),
        [
            ("T", None, "T", "T"),
            ("K", Some("Object"), "K extends Object", "K"),
            (
                "V",
                Some("Map<K, List<T?>>"),
                "V extends Map<K, List<T?>>",
                "V"
            ),
        ],
    );
    let bound = class.type_parameters[2].bound.as_ref().unwrap();
    assert_eq!(slice(source, bound.span), "Map<K, List<T?>>");
    let start = source.find("Map<K, List<T?>>").unwrap() as u32;
    assert_eq!(
        bound,
        &named(
            "Map<K, List<T?>>",
            start,
            "Map",
            vec![
                named("K", start + 4, "K", vec![], false),
                named(
                    "List<T?>",
                    start + 7,
                    "List",
                    vec![named("T?", start + 12, "T", vec![], true)],
                    false
                ),
            ],
            false
        )
    );
    assert_eq!(
        class
            .mixins
            .iter()
            .map(|m| (m.source.as_str(), slice(source, m.span)))
            .collect::<Vec<_>>(),
        [("Audit<T>", "Audit<T>"), ("p.Store<K, V>", "p.Store<K, V>")],
    );
    let second = &class.mixins[1];
    let start = source.find("p.Store<K, V>").unwrap() as u32;
    assert_eq!(
        second,
        &named(
            "p.Store<K, V>",
            start,
            "p.Store",
            vec![
                named("K", start + 8, "K", vec![], false),
                named("V", start + 11, "V", vec![], false),
            ],
            false
        )
    );
}

#[test]
fn primary_headers_keep_recursive_bounds_metadata_and_trivia() {
    let source = "class const Box<@Tag() T extends Comparable<T>>(final T value) with _$Box<T>;";
    let parsed = parse(393, source);
    assert_eq!(parsed.diagnostics, []);
    let class = &parsed.library.classes[0];
    let parameter = &class.type_parameters[0];
    assert_eq!(parameter.name, "T");
    assert_eq!(
        slice(source, parameter.span),
        "@Tag() T extends Comparable<T>"
    );
    assert_eq!(slice(source, parameter.name_span), "T");
    let bound = parameter.bound.as_ref().unwrap();
    assert_eq!(slice(source, bound.span), "Comparable<T>");
    let start = source.find("Comparable<T>").unwrap() as u32;
    assert_eq!(
        bound,
        &named(
            "Comparable<T>",
            start,
            "Comparable",
            vec![named("T", start + 11, "T", vec![], false)],
            false
        )
    );
    assert_eq!(class.mixins[0].source, "_$Box<T>");
    assert_eq!(class.fields[0].type_source.as_deref(), Some("T"));
}

#[test]
fn a_method_binder_is_not_a_class_parameter() {
    let parsed = parse(393, "class Plain { void method<T>(T value) {} }");
    assert_eq!(parsed.diagnostics, []);
    assert_eq!(parsed.library.classes[0].type_parameters, []);
    assert_eq!(parsed.library.classes[0].mixins, []);
}

#[test]
fn mixin_bounds_retain_function_and_record_type_kinds() {
    let source =
        "class Shapes<F extends Object Function(int), R extends (int, String)> with Shape<F, R> {}";
    let parsed = parse(393, source);
    assert_eq!(parsed.diagnostics, []);
    let params = &parsed.library.classes[0].type_parameters;
    assert_eq!(
        params[0].bound.as_ref().unwrap().kind,
        ParsedTypeKind::Function
    );
    assert_eq!(
        params[1].bound.as_ref().unwrap().kind,
        ParsedTypeKind::Record
    );
    assert_eq!(
        params[0].bound.as_ref().unwrap().source,
        "Object Function(int)"
    );
    assert_eq!(params[1].bound.as_ref().unwrap().source, "(int, String)");
}

#[test]
fn header_comments_do_not_become_names_or_change_bound_nullability() {
    let source = "class Box<T /* before bound */ extends Object?, U> with /* before mixin */ Audit, // between mixins\n p.Store<U> {}";
    let parsed = parse(393, source);
    assert_eq!(parsed.diagnostics, []);
    let class = &parsed.library.classes[0];
    assert_eq!(
        slice(source, class.type_parameters[0].span),
        "T /* before bound */ extends Object?"
    );
    let bound = class.type_parameters[0].bound.as_ref().unwrap();
    assert_eq!(bound.source, "Object?");
    assert!(bound.nullable);
    assert_eq!(slice(source, bound.span), "Object?");
    assert_eq!(class.type_parameters[1].name, "U");
    assert_eq!(class.type_parameters[1].bound, None);
    assert_eq!(
        class
            .mixins
            .iter()
            .map(|m| m.source.as_str())
            .collect::<Vec<_>>(),
        ["Audit", "p.Store<U>"]
    );
}
