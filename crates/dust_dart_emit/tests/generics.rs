//! Shared generic declaration/use rendering contracts.

use dust_dart_emit::{render_type_arguments, render_type_parameters};
use dust_ir::{NameIr, SpanIr, TypeIr, TypeParamIr};

fn parameter(name: &str, bound: Option<TypeIr>) -> TypeParamIr {
    let span = SpanIr {
        file_id: Default::default(),
        range: Default::default(),
    };
    TypeParamIr {
        name: NameIr {
            source: name.to_owned(),
            short: name.to_owned(),
            prefix: None,
            span,
        },
        bound,
        span,
    }
}

#[test]
fn non_generic_lists_have_no_angle_brackets() {
    assert_eq!(render_type_parameters(&[]), "");
    assert_eq!(render_type_arguments(&[]), "");
}

#[test]
fn declarations_keep_bounds_while_use_sites_only_forward_binders() {
    let parameters = [
        parameter("T", None),
        parameter("U", Some(TypeIr::named("V"))),
        parameter(
            "V",
            Some(TypeIr::generic("Comparable", vec![TypeIr::named("V")])),
        ),
        parameter("K", Some(TypeIr::object())),
        parameter(
            "W",
            Some(TypeIr::map_of(
                TypeIr::named("K"),
                TypeIr::list_of(TypeIr::named("T").nullable()),
            )),
        ),
    ];
    assert_eq!(
        render_type_parameters(&parameters),
        "<T, U extends V, V extends Comparable<V>, K extends Object, W extends Map<K, List<T?>>>",
    );
    assert_eq!(render_type_arguments(&parameters), "<T, U, V, K, W>");
}

#[test]
fn function_and_record_bound_source_is_not_rewritten() {
    let parameters = [
        parameter(
            "String",
            Some(TypeIr::function("String Function<String>(String)")),
        ),
        parameter(
            "R",
            Some(TypeIr::record("({String value, int count})").nullable()),
        ),
    ];
    assert_eq!(
        render_type_parameters(&parameters),
        "<String extends String Function<String>(String), R extends ({String value, int count})?>"
    );
    assert_eq!(render_type_arguments(&parameters), "<String, R>");
}
