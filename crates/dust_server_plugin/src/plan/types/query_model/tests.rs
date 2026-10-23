use dust_ir::{
    ClassIr, ClassKindIr, EnumIr, FieldIr, SerdeClassConfigIr, SerdeFieldConfigIr,
    SerdeRenameRuleIr, SpanIr, SymbolId, TraitApplicationIr, TypeIr,
};
use dust_text::{FileId, TextRange};

use super::*;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(4), TextRange::new(0_u32, 1_u32))
}

fn trait_(name: &str) -> TraitApplicationIr {
    TraitApplicationIr {
        symbol: SymbolId::new(name),
        span: span(),
    }
}

fn field(name: &str, ty: TypeIr, serde: Option<SerdeFieldConfigIr>) -> FieldIr {
    FieldIr {
        name: name.to_owned(),
        ty,
        span: span(),
        has_default: false,
        serde,
        configs: Vec::new(),
    }
}

fn query_class(fields: Vec<FieldIr>, traits: Vec<TraitApplicationIr>) -> ClassIr {
    ClassIr {
        kind: ClassKindIr::Class,
        name: "Filter".to_owned(),
        type_parameters: Vec::new(),
        mixins: Vec::new(),
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span: span(),
        fields,
        constructors: Vec::new(),
        methods: Vec::new(),
        traits,
        configs: Vec::new(),
        serde: Some(SerdeClassConfigIr {
            rename_all: Some(SerdeRenameRuleIr::SnakeCase),
            ..Default::default()
        }),
    }
}

#[test]
fn plans_flat_models_with_serde_names_and_enums() {
    let class = query_class(
        vec![
            field(
                "queryPage",
                TypeIr::int(),
                Some(SerdeFieldConfigIr {
                    aliases: vec!["page".to_owned()],
                    ..Default::default()
                }),
            ),
            field("tags", TypeIr::list_of(TypeIr::string()), None),
            field("ids", TypeIr::list_of(TypeIr::int()), None),
            field("order", TypeIr::named("Order"), None),
            field("at", TypeIr::named("DateTime").nullable(), None),
            field(
                "ignored",
                TypeIr::named("Nested"),
                Some(SerdeFieldConfigIr {
                    skip_deserializing: true,
                    ..Default::default()
                }),
            ),
        ],
        vec![trait_("dust_dart::Deserialize")],
    );
    let order = EnumIr {
        name: "Order".to_owned(),
        span: span(),
        variants: Vec::new(),
        traits: vec![trait_("dust_dart::Deserialize")],
        serde: None,
    };
    let mut diagnostics = Vec::new();

    let plan = query_model_plan(
        &TypeIr::named("Filter"),
        &[class],
        &[order],
        span(),
        &mut diagnostics,
    );

    assert!(diagnostics.is_empty());
    assert_eq!(
        plan.conversions
            .iter()
            .map(|value| value.key.as_str())
            .collect::<Vec<_>>(),
        ["query_page", "tags", "ids"]
    );
    assert_eq!(plan.conversions[0].aliases, ["page"]);
    assert!(matches!(
        plan.conversions[0].kind,
        QueryConversionKind::Scalar(_)
    ));
    assert!(matches!(
        plan.conversions[1].kind,
        QueryConversionKind::StringList
    ));
    assert!(matches!(
        plan.conversions[2].kind,
        QueryConversionKind::ScalarList(_)
    ));
}

#[test]
fn rejects_unsupported_shapes() {
    let class = query_class(
        vec![
            field("nested", TypeIr::named("Nested"), None),
            field(
                "coded",
                TypeIr::string(),
                Some(SerdeFieldConfigIr {
                    codec_source: Some("codec".to_owned()),
                    ..Default::default()
                }),
            ),
            field(
                "nullableItems",
                TypeIr::list_of(TypeIr::string().nullable()),
                None,
            ),
            field("order", TypeIr::named("Order"), None),
        ],
        Vec::new(),
    );
    let order = EnumIr {
        name: "Order".to_owned(),
        span: span(),
        variants: Vec::new(),
        traits: Vec::new(),
        serde: None,
    };
    let mut diagnostics = Vec::new();

    let _ = query_model_plan(
        &TypeIr::named("Filter"),
        &[class],
        &[order],
        span(),
        &mut diagnostics,
    );
    let _ = query_model_plan(
        &TypeIr::generic("Filter", vec![TypeIr::string()]),
        &[],
        &[],
        span(),
        &mut diagnostics,
    );

    assert_eq!(diagnostics.len(), 6);
    for expected in [
        "derive Deserialize",
        "custom SerDe codec",
        "flat scalar",
        "nullable items",
        "enum fields",
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|value| value.message.contains(expected)),
            "missing diagnostic: {expected}"
        );
    }
}
