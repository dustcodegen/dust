use dust_ir::{
    AnnotationValueIr, ClassIr, ClassKindIr, ConfigApplicationIr, ConstructorIr,
    ConstructorParamIr, MethodParamIr, NameIr, ParamKind, SpanIr, SymbolId, TypeIr,
};
use dust_text::{FileId, TextRange};

use super::*;
use crate::plan::body::BodyMedia;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(13), TextRange::new(0_u32, 1_u32))
}

fn extractor_param(source: &str) -> MethodParamIr {
    let (prefix, short) = source
        .rsplit_once('.')
        .map(|(prefix, short)| (Some(prefix.to_owned()), short.to_owned()))
        .unwrap_or_else(|| (None, source.to_owned()));
    MethodParamIr {
        name: "user".to_owned(),
        ty: TypeIr::named("AuthUser"),
        span: span(),
        kind: ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: vec![ConfigApplicationIr::with_arguments(
            SymbolId::new("dust_server::Extract"),
            None,
            vec![AnnotationValueIr::Member(NameIr {
                source: source.to_owned(),
                short,
                prefix,
                span: span(),
            })],
            Default::default(),
            span(),
        )],
    }
}

fn constructor(
    name: Option<&str>,
    is_const: bool,
    parameter_required: Option<bool>,
) -> ConstructorIr {
    ConstructorIr {
        name: name.map(str::to_owned),
        is_factory: false,
        is_const,
        redirected_target_source: None,
        redirected_target_name: None,
        redirected_target: None,
        span: span(),
        params: parameter_required
            .map(|is_required| ConstructorParamIr {
                name: "scope".to_owned(),
                ty: TypeIr::string(),
                span: span(),
                kind: ParamKind::Named,
                is_required,
                has_default: !is_required,
                default_value_source: None,
            })
            .into_iter()
            .collect(),
    }
}

fn extractor_class(constructor: ConstructorIr) -> ClassIr {
    ClassIr {
        kind: ClassKindIr::Class,
        name: "BearerAuth".to_owned(),
        type_parameters: Vec::new(),
        mixins: Vec::new(),
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span: span(),
        fields: Vec::new(),
        constructors: vec![constructor],
        methods: Vec::new(),
        traits: Vec::new(),
        configs: Vec::new(),
        serde: None,
    }
}

#[test]
fn accepts_callable_local_and_deferred_imported_extractors() {
    let local = extractor_class(constructor(None, true, Some(false)));
    let mut diagnostics = Vec::new();
    assert!(
        parse_param(
            &extractor_param("BearerAuth"),
            &BTreeSet::new(),
            BodyMedia::Json,
            std::slice::from_ref(&local),
            &mut diagnostics,
        )
        .is_some()
    );
    assert!(
        parse_param(
            &extractor_param("auth.BearerAuth"),
            &BTreeSet::new(),
            BodyMedia::Json,
            &[extractor_class(constructor(None, false, None))],
            &mut diagnostics,
        )
        .is_some()
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn rejects_local_extractors_that_const_call_cannot_construct() {
    let mut abstract_class = extractor_class(constructor(None, true, None));
    abstract_class.is_abstract = true;
    let classes = [
        extractor_class(constructor(None, false, None)),
        extractor_class(constructor(Some("named"), true, None)),
        extractor_class(constructor(None, true, Some(true))),
        abstract_class,
    ];

    for class in classes {
        let mut diagnostics = Vec::new();
        assert!(
            parse_param(
                &extractor_param("BearerAuth"),
                &BTreeSet::new(),
                BodyMedia::Json,
                &[class],
                &mut diagnostics,
            )
            .is_none()
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("cannot be constructed"));
        assert_eq!(diagnostics[0].labels[0].range, span().range);
    }
}
