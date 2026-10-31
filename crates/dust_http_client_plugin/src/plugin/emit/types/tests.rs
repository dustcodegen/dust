use super::*;

#[test]
fn renders_public_serde_deserializers_with_generic_arguments() {
    assert_eq!(serde_deserializer("Todo", &[]), "const $TodoDeserializer()");
    assert_eq!(
        serde_deserializer("contracts.Box", &[TypeIr::string()]),
        "const contracts.$BoxDeserializer<String>()"
    );

    let ty = TypeIr::generic("contracts.Box", vec![TypeIr::string()]);
    assert_eq!(
        render_decode_expr_with("data", &ty, &["contracts.Box".to_owned()]),
        "const contracts.$BoxDeserializer<String>().deserialize(data as Map<String, Object?>)"
    );
}

#[test]
fn serde_selection_propagates_through_nullable_lists() {
    let item = TypeIr::named("contracts.Todo").nullable();
    let list = TypeIr::generic("List", vec![item]);
    let rendered = render_decode_expr_with("data", &list, &["contracts.Todo".to_owned()]);

    assert!(rendered.contains("item == null"));
    assert!(rendered.contains("contracts.$TodoDeserializer"));
}
