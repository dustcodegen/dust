//! Integration tests for resolver-owned Dart type lowering.

use dust_ir::{BuiltinType, TypeIr};
use dust_parser_dart::ParsedTypeSurface;
use dust_resolver::{lower_type_ir, lower_type_ir_in_scope};
use dust_text::TextRange;

#[test]
fn lowers_builtin_and_nullable_types() {
    let outcome = lower_type_ir(None, Some("String?"));

    assert!(outcome.diagnostics.is_empty());
    assert!(outcome.value.is_builtin(BuiltinType::String));
    assert!(outcome.value.is_nullable());
}

#[test]
fn class_parameter_names_shadow_builtins_without_changing_unscoped_types() {
    let parsed =
        ParsedTypeSurface::parse("Map<String, List<Object?>>", TextRange::new(0_u32, 26_u32))
            .unwrap();
    let scoped = lower_type_ir_in_scope(Some(&parsed), Some("ignored"), &["String", "Object"]);
    assert_eq!(
        scoped.value,
        TypeIr::map_of(
            TypeIr::named("String"),
            TypeIr::list_of(TypeIr::named("Object").nullable())
        )
    );
    assert_eq!(scoped.diagnostics, []);
    assert_eq!(lower_type_ir(None, Some("String")).value, TypeIr::string());
}

#[test]
fn raw_scoped_fallback_preserves_qualified_names_and_nullability() {
    let outcome = lower_type_ir_in_scope(None, Some("p.String<String?>"), &["String"]);
    assert_eq!(
        outcome.value,
        TypeIr::generic("p.String", vec![TypeIr::named("String").nullable()])
    );
    assert_eq!(outcome.diagnostics, []);
    assert_eq!(
        lower_type_ir_in_scope(None, Some("String?"), &["String"]).value,
        TypeIr::named("String").nullable()
    );
    assert_eq!(
        lower_type_ir_in_scope(None, None, &["String"]).value,
        TypeIr::unknown()
    );
}

#[test]
fn scoped_lowering_does_not_rewrite_opaque_function_or_record_source() {
    for (source, expected) in [
        (
            "String Function(String)?",
            TypeIr::function("String Function(String)").nullable(),
        ),
        (
            "({String value, int count})?",
            TypeIr::record("({String value, int count})").nullable(),
        ),
    ] {
        let outcome = lower_type_ir_in_scope(None, Some(source), &["String", "Object"]);
        assert_eq!(outcome.value, expected);
        assert_eq!(outcome.diagnostics, []);
    }
}

#[test]
fn lowers_nested_generic_types() {
    let outcome = lower_type_ir(None, Some("Map<String, List<int?>>"));

    assert!(outcome.diagnostics.is_empty());
    assert!(outcome.value.is_named("Map"));
    assert_eq!(outcome.value.args().len(), 2);
    assert!(outcome.value.args()[1].is_named("List"));
    assert!(outcome.value.args()[1].args()[0].is_builtin(BuiltinType::Int));
    assert!(outcome.value.args()[1].args()[0].is_nullable());
}

#[test]
fn keeps_function_and_record_types_structured() {
    let function = lower_type_ir(None, Some("void Function(String, int)?"));
    let record = lower_type_ir(None, Some("({String name, int age})?"));

    assert!(function.value.is_function());
    assert!(function.value.is_nullable());
    assert!(record.value.is_record());
    assert!(record.value.is_nullable());
}

#[test]
fn lowers_parser_owned_type_surface_before_raw_source_fallback() {
    let parsed =
        ParsedTypeSurface::parse("Map<String, List<int?>>", TextRange::new(0_u32, 23_u32)).unwrap();

    let outcome = lower_type_ir(Some(&parsed), Some("Object"));

    assert!(outcome.diagnostics.is_empty());
    assert!(outcome.value.is_named("Map"));
    assert_eq!(outcome.value.args().len(), 2);
    assert!(outcome.value.args()[1].is_named("List"));
    assert!(outcome.value.args()[1].args()[0].is_builtin(BuiltinType::Int));
    assert!(outcome.value.args()[1].args()[0].is_nullable());
}
