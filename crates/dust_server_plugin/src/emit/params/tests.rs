use dust_ir::{MethodParamIr, ParamKind, SpanIr, TypeIr};
use dust_text::{FileId, TextRange};

use super::*;

/// Shared-body and request-passthrough emission contracts.
mod shared;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(23), TextRange::new(0_u32, 1_u32))
}

fn param(name: &str, ty: TypeIr) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty,
        span: span(),
        kind: ParamKind::Positional,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

fn plan<'a>(
    param: &'a MethodParamIr,
    value_type: &'a TypeIr,
    wrapper: ExtractionWrapper,
    extraction: Extraction,
) -> ParamPlan<'a> {
    ParamPlan {
        param,
        value_type,
        wrapper,
        extraction,
        validated: false,
        serde_deserialized: false,
    }
}

#[test]
fn renders_phase_three_extractors_exactly() {
    let raw_query = param("raw", TypeIr::string().nullable());
    let bytes = param("bytes", TypeIr::named("Uint8List"));
    let text = param("text", TypeIr::string());
    let stream = param(
        "stream",
        TypeIr::generic("Stream", vec![TypeIr::list_of(TypeIr::int())]),
    );
    let state = param("state", TypeIr::named("TodoState"));
    let peer = param("peer", TypeIr::named("PeerInfo"));

    assert_eq!(
        render_extractor(&plan(
            &raw_query,
            &raw_query.ty,
            ExtractionWrapper::Required,
            Extraction::RawQuery,
        )),
        "const RawQueryExtractable()"
    );
    assert_eq!(
        render_extractor(&plan(
            &bytes,
            &bytes.ty,
            ExtractionWrapper::Required,
            Extraction::RawBody(RawBodyFormat::Bytes),
        )),
        "const RawBodyExtractable()"
    );
    assert_eq!(
        render_extractor(&plan(
            &text,
            &text.ty,
            ExtractionWrapper::Required,
            Extraction::RawBody(RawBodyFormat::Text),
        )),
        "const TextBodyExtractable()"
    );
    assert_eq!(
        render_extractor(&plan(
            &stream,
            &stream.ty,
            ExtractionWrapper::Required,
            Extraction::RawBody(RawBodyFormat::Stream),
        )),
        "const StreamBodyExtractable()"
    );
    assert_eq!(
        render_extractor(&plan(
            &state,
            &state.ty,
            ExtractionWrapper::Required,
            Extraction::State,
        )),
        "const StateExtractable<TodoState>()"
    );
    assert_eq!(
        render_extractor(&plan(
            &peer,
            &peer.ty,
            ExtractionWrapper::Required,
            Extraction::Peer,
        )),
        "const PeerExtractable()"
    );
}

#[test]
fn renders_validated_json_and_form_bodies_exactly() {
    let input = param("input", TypeIr::named("CreateTodo"));
    let mut json = plan(
        &input,
        &input.ty,
        ExtractionWrapper::Required,
        Extraction::Body(BodyFormat::Json),
    );
    json.validated = true;
    json.serde_deserialized = true;
    assert_eq!(
        render_extractor(&json),
        "const ValidatedExtractable<CreateTodo>(\n    JsonExtractable<CreateTodo>(_$CreateTodoDeserialize),\n  )"
    );

    let mut form = plan(
        &input,
        &input.ty,
        ExtractionWrapper::Required,
        Extraction::Body(BodyFormat::Form),
    );
    form.validated = true;
    assert_eq!(
        render_extractor(&form),
        "const ValidatedExtractable<CreateTodo>(\n    FormModelExtractable<CreateTodo>(CreateTodo.deserialize),\n  )"
    );

    let queries = plan(
        &input,
        &input.ty,
        ExtractionWrapper::Required,
        Extraction::Queries,
    );
    assert_eq!(
        render_extractor(&queries),
        "const QueriesExtractable<CreateTodo>(\n    CreateTodo.deserialize,\n  )"
    );

    let fields = param(
        "fields",
        TypeIr::map_of(TypeIr::string(), TypeIr::named("Object").nullable()),
    );
    assert_eq!(
        render_extractor(&plan(
            &fields,
            &fields.ty,
            ExtractionWrapper::Required,
            Extraction::Body(BodyFormat::Form),
        )),
        "FormModelExtractable<Map<String, Object?>>((fields) => fields)"
    );

    let mut impossible_validation = plan(
        &input,
        &input.ty,
        ExtractionWrapper::Required,
        Extraction::State,
    );
    impossible_validation.validated = true;
    assert_eq!(
        render_extractor(&impossible_validation),
        "const StateExtractable<CreateTodo>()"
    );
}

#[test]
fn renders_typed_query_model_exactly() {
    let input = param("filter", TypeIr::named("TodoFilter"));
    let mut query = plan(
        &input,
        &input.ty,
        ExtractionWrapper::Required,
        Extraction::QueryModel(QueryModelPlan {
            conversions: vec![
                QueryConversionPlan {
                    key: "done".to_owned(),
                    kind: QueryConversionKind::Scalar(TypeIr::bool().nullable()),
                },
                QueryConversionPlan {
                    key: "tags".to_owned(),
                    kind: QueryConversionKind::StringList,
                },
                QueryConversionPlan {
                    key: "ids".to_owned(),
                    kind: QueryConversionKind::ScalarList(TypeIr::int()),
                },
            ],
        }),
    );
    query.validated = true;
    query.serde_deserialized = true;

    assert_eq!(
        render_extractor(&query),
        "ValidatedExtractable<TodoFilter>(\n    QueryModelExtractable<TodoFilter>((query) {\n      final values = query.toObjectMap();\n      query.coerceOne<bool?>(values, 'done');\n      query.copyAll(values, 'tags');\n      query.coerceAll<int>(values, 'ids');\n      return _$TodoFilterDeserialize(values);\n    }),\n  )"
    );
}

#[test]
#[should_panic(expected = "handled before extractor rendering")]
fn direct_rendering_rejects_shared_body_extractors() {
    let field = param("field", TypeIr::string());
    render_extractor(&plan(
        &field,
        &field.ty,
        ExtractionWrapper::Required,
        Extraction::Field("field".to_owned()),
    ));
}
