use super::*;
use dust_ir::{ClassKindIr, ConstructorParamIr, SpanIr, TypeIr};
use dust_text::{FileId, TextRange};

fn param(name: &str, kind: ParamKind, required: bool, default: Option<&str>) -> ConstructorParamIr {
    ConstructorParamIr {
        name: name.into(),
        ty: TypeIr::int().nullable(),
        span: SpanIr::new(FileId::new(138), TextRange::new(0_u32, 1_u32)),
        kind,
        is_required: required,
        has_default: default.is_some(),
        default_value_source: default.map(str::to_owned),
    }
}

fn class() -> ClassIr {
    ClassIr {
        name: "Value".into(),
        kind: ClassKindIr::Class,
        type_parameters: vec![],
        mixins: vec![],
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span: SpanIr::new(FileId::new(138), TextRange::new(0_u32, 1_u32)),
        fields: vec![],
        constructors: vec![],
        methods: vec![],
        traits: vec![],
        configs: vec![],
        serde: None,
    }
}

#[test]
fn decoded_calls_preserve_optional_slots_and_never_omit_required_nullables() {
    use ParamKind::{Named, Positional};
    let class = class();
    for (params, values, expected) in [
        (vec![param("first", Positional, true, None)], vec![], None),
        (vec![param("first", Named, true, None)], vec![], None),
        (
            vec![param("first", Positional, false, None)],
            vec![],
            Some("Value()"),
        ),
        (
            vec![param("first", Named, false, None)],
            vec![],
            Some("Value()"),
        ),
        (
            vec![
                param("first", Positional, false, None),
                param("second", Positional, false, None),
            ],
            vec![("second", "2".into())],
            Some("Value(null, 2)"),
        ),
        (
            vec![
                param("first", Positional, false, Some("1")),
                param("second", Positional, false, None),
            ],
            vec![("second", "2".into())],
            Some("Value(1, 2)"),
        ),
        (
            vec![
                param("first", Positional, false, None),
                param("second", Named, false, None),
            ],
            vec![("second", "2".into())],
            Some("Value(second: 2)"),
        ),
        (
            vec![
                param("first", Named, false, None),
                param("second", Named, true, None),
            ],
            vec![("second", "2".into())],
            Some("Value(second: 2)"),
        ),
    ] {
        let constructor = ConstructorIr {
            name: None,
            is_factory: false,
            is_const: false,
            redirected_target_source: None,
            redirected_target_name: None,
            redirected_target: None,
            span: class.span,
            params,
        };
        assert_eq!(
            render_constructor_call(&class, &constructor, &values).as_deref(),
            expected
        );
    }
}
