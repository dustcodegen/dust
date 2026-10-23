use dust_ir::{BuiltinType, SpanIr, TypeIr};
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
fn recognizes_deferred_response_shapes() {
    for ty in [
        TypeIr::named("Result"),
        TypeIr::named("Option"),
        TypeIr::string(),
        TypeIr::named("Uint8List"),
        TypeIr::named("Stream"),
        TypeIr::generic("Future", vec![TypeIr::string()]),
    ] {
        assert!(unsupported_return(&ty));
    }
    assert!(!unsupported_return(&TypeIr::named("Response")));
    assert!(!unsupported_return(&TypeIr::generic(
        "Future",
        vec![TypeIr::named("Todo")]
    )));
}
