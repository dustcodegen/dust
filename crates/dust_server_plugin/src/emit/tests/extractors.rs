use crate::emit::params::render_extractor;

use super::*;

#[test]
fn non_const_decoder_extractors_keep_wrappers_non_const() {
    let optional_type = TypeIr::generic(
        "Option",
        vec![TypeIr::map_of(TypeIr::string(), TypeIr::dynamic())],
    );
    let fallible_type = TypeIr::generic(
        "Result",
        vec![
            TypeIr::map_of(TypeIr::string(), TypeIr::string()),
            TypeIr::named("Rejection"),
        ],
    );
    let params = [
        param("body", optional_type, ParamKind::Positional),
        param("queries", fallible_type, ParamKind::Positional),
    ];
    let optional = ParamPlan {
        param: &params[0],
        value_type: &params[0].ty.args()[0],
        wrapper: ExtractionWrapper::Optional,
        extraction: Extraction::Body(BodyFormat::Json),
        validated: false,
        serde_deserialized: false,
    };
    let fallible = ParamPlan {
        param: &params[1],
        value_type: &params[1].ty.args()[0],
        wrapper: ExtractionWrapper::Fallible,
        extraction: Extraction::Queries,
        validated: false,
        serde_deserialized: false,
    };

    assert_eq!(
        render_extractor(&optional),
        "OptionalExtractable<Map<String, dynamic>>(JsonExtractable<Map<String, dynamic>>((json) => json))"
    );
    assert_eq!(
        render_extractor(&fallible),
        "FallibleExtractable<Map<String, String>>(QueriesExtractable<Map<String, String>>((queries) => queries))"
    );
}
