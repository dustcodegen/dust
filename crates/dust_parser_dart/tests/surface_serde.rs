//! Parser-fact serialization preserves nested source facts and checked byte ranges.

use dust_parser_dart::*;
use dust_text::{TextRange, TextSize};

fn span() -> TextRange {
    TextRange::new(3_u32, 128_u32)
}

fn ty(source: &str) -> ParsedTypeSurface {
    ParsedTypeSurface::parse(source, span()).unwrap()
}

fn arguments() -> ParsedAnnotationArguments {
    let value = ParsedAnnotationValue {
        source: "7".into(),
        span: span(),
        kind: ParsedAnnotationValueRootKind::Number(ParsedAnnotationNumberKind::Int),
    };
    ParsedAnnotationArguments {
        positional: vec![ParsedAnnotationArgument {
            source: "7".into(),
            value: Some(value),
            span: span(),
        }],
        named: Vec::new(),
    }
}

fn annotation() -> ParsedAnnotation {
    let value = ParsedAnnotationValue {
        source: "d.Marker<int>(7)".into(),
        span: span(),
        kind: ParsedAnnotationValueRootKind::Constructor {
            name: "d.Marker<int>".into(),
            short_name: "Marker".into(),
            prefix: Some("d".into()),
            arguments: Box::new(arguments()),
        },
    };
    let list = ParsedAnnotationValue {
        source: "[d.Marker<int>(7)]".into(),
        span: span(),
        kind: ParsedAnnotationValueRootKind::List(vec![value]),
    };
    ParsedAnnotation {
        name: "Derive".into(),
        prefix: Some("d".into()),
        qualified_name: "d.Derive".into(),
        arguments_source: Some("([d.Marker<int>(7)])".into()),
        parsed_arguments: Some(ParsedAnnotationArguments {
            positional: vec![ParsedAnnotationArgument {
                source: list.source.clone(),
                value: Some(list),
                span: span(),
            }],
            named: Vec::new(),
        }),
        span: span(),
    }
}

fn surface() -> ParsedDartFileSurface {
    let name = |name: &str| ParsedLexicalName {
        name: name.into(),
        span: span(),
    };
    ParsedDartFileSurface {
        span: TextRange::new(0_u32, 512_u32),
        lexical_scopes: vec![ParsedLexicalScope {
            parent: None,
            name: None,
            kind: ParsedLexicalScopeKind::Library,
            visibility: span(),
            span: span(),
            declarations: vec![ParsedLexicalDeclaration {
                name: name("marker"),
                span: span(),
                kind: ParsedLexicalDeclarationKind::Constant(Some(ParsedLexicalInitializer {
                    source: "d.Marker(7)".into(),
                    span: span(),
                    kind: ParsedLexicalInitializerKind::Constructor {
                        callee: vec![name("d"), name("Marker")],
                        arguments: arguments(),
                    },
                })),
            }],
        }],
        directives: vec![ParsedDirective::Library {
            name: Some("models".into()),
            annotations: vec![annotation()],
            span: span(),
        }],
        classes: vec![ParsedClassSurface {
            kind: ParsedClassKind::Class,
            name: "Model".into(),
            type_parameters: vec![ParsedTypeParamSurface {
                name: "T".into(),
                name_span: span(),
                bound: Some(ty("Object")),
                span: span(),
            }],
            mixins: vec![ty("Comparable<T>")],
            is_abstract: false,
            is_interface: false,
            superclass_name: Some("Base".into()),
            superclass: Some(ty("Base<T>")),
            annotations: vec![annotation()],
            fields: vec![ParsedFieldSurface {
                name: "values".into(),
                annotations: vec![annotation()],
                type_source: Some("Map<String, List<int?>>".into()),
                parsed_type: Some(ty("Map<String, List<int?>>")),
                function_formal: None,
                has_default: false,
                span: span(),
            }],
            constructors: vec![ParsedConstructorSurface {
                name: Some("from".into()),
                is_factory: true,
                is_const: false,
                annotations: vec![annotation()],
                redirected_target_source: Some("Other<T>.from".into()),
                redirected_target_name: Some("Other".into()),
                redirected_target: Some(ParsedRedirectTargetSurface {
                    ty: ty("Other<T>"),
                    type_name_parts: vec![ParsedRedirectIdentifierSurface {
                        name: "Other".into(),
                        span: span(),
                    }],
                    constructor: Some(ParsedRedirectIdentifierSurface {
                        name: "from".into(),
                        span: span(),
                    }),
                    span: span(),
                }),
                params: vec![ParsedConstructorParamSurface {
                    name: "values".into(),
                    annotations: vec![annotation()],
                    type_source: Some("Map<String, List<int?>>".into()),
                    parsed_type: Some(ty("Map<String, List<int?>>")),
                    function_formal: None,
                    kind: ParameterKind::Named,
                    is_required: true,
                    has_default: false,
                    default_value_source: None,
                    span: span(),
                }],
                span: span(),
            }],
            methods: Vec::new(),
            span: span(),
        }],
        enums: Vec::new(),
        mixins: Vec::new(),
        extensions: Vec::new(),
        extension_types: Vec::new(),
        functions: Vec::new(),
        variables: Vec::new(),
        typedefs: Vec::new(),
        query_calls: vec![ParsedQueryCallSurface {
            function: ParsedQueryFunction::As,
            type_arg_source: Some("Model<int>".into()),
            sql: "SELECT id FROM models".into(),
            sql_source_static: true,
            parameter_count: 0,
            params_source_is_list: true,
            fetch_method: Some("fetchAll".into()),
            has_row_mapper_argument: false,
            unsafe_sql_allowed: false,
            enclosing_name: Some("load".into()),
            span: span(),
        }],
    }
}

#[test]
fn nested_parser_surface_roundtrips_without_losing_source_or_typed_facts() {
    let original = surface();
    let encoded = serde_json::to_vec(&original).unwrap();
    let decoded = serde_json::from_slice::<ParsedDartFileSurface>(&encoded).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), encoded);
}

#[test]
fn deserialization_rejects_reversed_ranges_and_out_of_range_offsets() {
    for encoded in [
        r#"{"start":8,"end":7}"#,
        r#"{"start":0,"end":4294967296}"#,
        r#"{"start":-1,"end":0}"#,
    ] {
        assert!(
            serde_json::from_str::<TextRange>(encoded).is_err(),
            "{encoded}"
        );
    }
    for encoded in ["4294967296", "-1", "1.5"] {
        assert!(
            serde_json::from_str::<TextSize>(encoded).is_err(),
            "{encoded}"
        );
    }
    let maximum = format!(r#"{{"start":{},"end":{}}}"#, u32::MAX, u32::MAX);
    assert_eq!(
        serde_json::from_str::<TextRange>(&maximum).unwrap(),
        TextRange::empty(u32::MAX)
    );
    let mut invalid = serde_json::to_value(surface()).unwrap();
    invalid["classes"][0]["constructors"][0]["params"][0]["span"] =
        serde_json::json!({"start":9,"end":8});
    assert!(serde_json::from_value::<ParsedDartFileSurface>(invalid).is_err());
}
