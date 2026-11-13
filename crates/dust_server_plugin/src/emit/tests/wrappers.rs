use super::*;

/// Checks required, optional, and fallible extraction emission.
#[test]
fn emits_custom_extractors_and_parameter_wrappers() {
    let class = class(method(vec![
        param("user", TypeIr::named("AuthUser"), ParamKind::Positional),
        param(
            "viewer",
            TypeIr::generic("Option", vec![TypeIr::named("AuthUser")]),
            ParamKind::Positional,
        ),
        param(
            "outcome",
            TypeIr::generic(
                "Result",
                vec![TypeIr::named("AuthUser"), TypeIr::named("Rejection")],
            ),
            ParamKind::Positional,
        ),
        param(
            "page",
            TypeIr::generic("Option", vec![TypeIr::int()]),
            ParamKind::Positional,
        ),
    ]));
    let method = &class.methods[0];
    let params = vec![
        ParamPlan {
            param: &method.params[0],
            value_type: &method.params[0].ty,
            wrapper: ExtractionWrapper::Required,
            extraction: Extraction::Custom("BearerAuth".to_owned()),
            validated: false,
            serde_deserialized: false,
        },
        ParamPlan {
            param: &method.params[1],
            value_type: &method.params[1].ty.args()[0],
            wrapper: ExtractionWrapper::Optional,
            extraction: Extraction::Custom("BearerAuth".to_owned()),
            validated: false,
            serde_deserialized: false,
        },
        ParamPlan {
            param: &method.params[2],
            value_type: &method.params[2].ty.args()[0],
            wrapper: ExtractionWrapper::Fallible,
            extraction: Extraction::Custom("TodosWrite".to_owned()),
            validated: false,
            serde_deserialized: false,
        },
        ParamPlan {
            param: &method.params[3],
            value_type: &method.params[3].ty.args()[0],
            wrapper: ExtractionWrapper::Optional,
            extraction: Extraction::Query("page".to_owned()),
            validated: false,
            serde_deserialized: false,
        },
    ];
    let output = emit_handler(
        &HandlerPlan {
            callable: Callable::Method(method),
            websocket: None,
            method: "POST",
            path: "/".to_owned(),
            status: 200,
            params,
            middleware: Vec::new(),
            metadata: metadata("write"),
        },
        Some("Todos"),
    );

    for expected in [
        "final Result<AuthUser, Rejection> _dustResult0 =\n      await const BearerAuth().extract(request);",
        "final Result<Option<AuthUser>, Rejection> _dustResult1 =\n      await const OptionalExtractable<AuthUser>(BearerAuth())\n          .extract(request);",
        "final Result<Result<AuthUser, Rejection>, Rejection> _dustResult2 =\n      await const FallibleExtractable<AuthUser>(TodosWrite())\n          .extract(request);",
        "final _dustValue2 =\n      (_dustResult2 as Ok<Result<AuthUser, Rejection>, Rejection>).value;",
        "OptionalExtractable<int>(QueryExtractable<int>('page'))",
    ] {
        assert!(output.contains(expected), "missing {expected}");
    }
}
