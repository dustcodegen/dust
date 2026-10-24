use super::*;

#[test]
fn annotation_args_support_general_annotations() {
    let annotation = dust_ir::AnnotationIr {
        raw_name: "Routes".to_owned(),
        short_name: "Routes".to_owned(),
        prefix: None,
        positional_args: vec![AnnotationValueIr::String("/notes".to_owned())],
        named_args: BTreeMap::new(),
        resolved_symbol: Some(SymbolId::new("dust_server::Routes")),
        span: span(),
    };
    let args = Args::Annotation(&annotation);
    assert_eq!(args.span(), span());
    assert!(args.named().is_empty());
    assert_eq!(
        string_value(&args.positional()[0]),
        Some("/notes".to_owned())
    );
    assert_eq!(string_value(&AnnotationValueIr::Null), None);
}
