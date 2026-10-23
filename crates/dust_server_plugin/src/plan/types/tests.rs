use dust_ir::{BuiltinType, ClassIr, ClassKindIr, SpanIr, SymbolId, TraitApplicationIr, TypeIr};
use dust_text::{FileId, TextRange};

use super::*;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(4), TextRange::new(0_u32, 1_u32))
}

#[test]
fn validates_scalar_and_map_shapes() {
    let mut diagnostics = Vec::new();
    for ty in [
        TypeIr::string(),
        TypeIr::int(),
        TypeIr::double(),
        TypeIr::num(),
        TypeIr::bool(),
        TypeIr::named("BigInt"),
        TypeIr::named("DateTime"),
        TypeIr::named("Uri"),
    ] {
        require_scalar(&ty, span(), &mut diagnostics);
    }
    require_scalar(&TypeIr::named("Todo"), span(), &mut diagnostics);
    assert_eq!(diagnostics.len(), 1);
    assert!(is_map_of(
        &TypeIr::map_of(TypeIr::string(), TypeIr::string()),
        "String",
        "String"
    ));
    assert!(!is_map_of(
        &TypeIr::map_of(TypeIr::string(), TypeIr::string()).nullable(),
        "String",
        "String"
    ));
}

#[test]
fn validates_query_and_body_decoder_targets() {
    let mut diagnostics = Vec::new();
    require_queries_type(
        &TypeIr::map_of(TypeIr::string(), TypeIr::string()),
        span(),
        &mut diagnostics,
    );
    require_queries_type(&TypeIr::named("Filter"), span(), &mut diagnostics);
    require_queries_type(&TypeIr::list_of(TypeIr::string()), span(), &mut diagnostics);
    require_body_type(
        &TypeIr::map_of(TypeIr::string(), TypeIr::dynamic()),
        span(),
        &mut diagnostics,
    );
    require_body_type(
        &TypeIr::map_of(
            TypeIr::string(),
            TypeIr::Builtin {
                kind: BuiltinType::Object,
                nullable: true,
            },
        ),
        span(),
        &mut diagnostics,
    );
    require_body_type(&TypeIr::named("CreateTodo"), span(), &mut diagnostics);
    require_body_type(&TypeIr::string(), span(), &mut diagnostics);
    assert_eq!(diagnostics.len(), 2);
}

#[test]
fn recognizes_phase_three_response_shapes() {
    for ty in [
        TypeIr::named("Result"),
        TypeIr::named("Option"),
        TypeIr::named("Stream"),
        TypeIr::generic("Future", vec![TypeIr::named("Result")]),
        TypeIr::named("Future"),
    ] {
        assert!(unsupported_return(&ty));
    }
    for ty in [
        TypeIr::named("Response"),
        TypeIr::string(),
        TypeIr::named("Uint8List"),
        TypeIr::generic("Stream", vec![TypeIr::list_of(TypeIr::int())]),
        TypeIr::generic(
            "Result",
            vec![TypeIr::named("Todo"), TypeIr::named("Rejection")],
        ),
        TypeIr::generic("Option", vec![TypeIr::named("Todo")]),
        TypeIr::generic("Future", vec![TypeIr::named("Todo")]),
    ] {
        assert!(!unsupported_return(&ty));
    }
}

#[test]
fn recognizes_only_same_library_dust_derives() {
    let mut class = ClassIr {
        kind: ClassKindIr::Class,
        name: "CreateTodo".to_owned(),
        type_parameters: Vec::new(),
        mixins: Vec::new(),
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span: span(),
        fields: Vec::new(),
        constructors: Vec::new(),
        methods: Vec::new(),
        traits: vec![TraitApplicationIr {
            symbol: SymbolId::new("dust_dart::Validate"),
            span: span(),
        }],
        configs: Vec::new(),
        serde: None,
    };
    assert!(derives_validate(
        &TypeIr::named("CreateTodo"),
        std::slice::from_ref(&class)
    ));
    assert!(!derives_deserialize(
        &TypeIr::named("CreateTodo"),
        std::slice::from_ref(&class)
    ));
    assert!(!derives_validate(
        &TypeIr::named("models.CreateTodo"),
        std::slice::from_ref(&class)
    ));

    class.traits[0].symbol = SymbolId::new("other::Validate");
    assert!(!derives_validate(
        &TypeIr::named("CreateTodo"),
        std::slice::from_ref(&class)
    ));
    class.traits[0].symbol = SymbolId::new("dust_dart::Deserialize");
    assert!(derives_deserialize(&TypeIr::named("CreateTodo"), &[class]));
}

#[test]
fn recognizes_raw_context_and_validation_types() {
    let mut diagnostics = Vec::new();
    require_raw_query_type(&TypeIr::string().nullable(), span(), &mut diagnostics);
    require_raw_query_type(&TypeIr::string(), span(), &mut diagnostics);
    require_state_type(&TypeIr::named("Repo"), span(), &mut diagnostics);
    require_state_type(&TypeIr::named("Repo").nullable(), span(), &mut diagnostics);
    assert_eq!(diagnostics.len(), 2);

    assert_eq!(
        raw_body_format(&TypeIr::named("Uint8List")),
        Some(RawBodyFormat::Bytes)
    );
    assert_eq!(
        raw_body_format(&TypeIr::string()),
        Some(RawBodyFormat::Text)
    );
    assert_eq!(
        raw_body_format(&TypeIr::generic(
            "Stream",
            vec![TypeIr::list_of(TypeIr::int())],
        )),
        Some(RawBodyFormat::Stream)
    );
    assert_eq!(raw_body_format(&TypeIr::list_of(TypeIr::int())), None);
    assert!(is_peer_info(&TypeIr::named("PeerInfo")));
    assert!(is_uploaded_file(&TypeIr::named("UploadedFile")));
}
