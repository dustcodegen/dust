use dust_ir::{
    ClassIr, ClassKindIr, FunctionIr, MethodIr, MethodParamIr, NameIr, ParamKind, SpanIr, TypeIr,
};
use dust_text::{FileId, TextRange};

use super::*;
use crate::plan::{
    BodyFormat, Callable, ControllerPlan, Extraction, ExtractionWrapper, ParamPlan,
    RouteMetadataPlan,
};

mod extractors;
mod metadata;
mod middleware;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32))
}

fn param(name: &str, ty: TypeIr, kind: ParamKind) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty,
        span: span(),
        kind,
        is_required: true,
        has_default: false,
        default_value_source: None,
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

fn method(params: Vec<MethodParamIr>) -> MethodIr {
    MethodIr {
        name: "write".to_owned(),
        is_static: false,
        is_external: false,
        return_type: TypeIr::named("void"),
        has_body: true,
        body_source: Some("{}".to_owned()),
        params,
        span: span(),
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

fn class(method: MethodIr) -> ClassIr {
    ClassIr {
        kind: ClassKindIr::Class,
        name: "Todos".to_owned(),
        type_parameters: Vec::new(),
        mixins: Vec::new(),
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span: span(),
        fields: Vec::new(),
        constructors: Vec::new(),
        methods: vec![method],
        traits: Vec::new(),
        configs: Vec::new(),
        serde: None,
    }
}

fn function() -> FunctionIr {
    FunctionIr {
        name: NameIr {
            source: "health".to_owned(),
            short: "health".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: TypeIr::generic("Future", vec![TypeIr::named("Health")]),
        params: Vec::new(),
        annotations: Vec::new(),
        span: span(),
    }
}

fn metadata(operation_id: &str) -> RouteMetadataPlan {
    RouteMetadataPlan {
        operation_id: operation_id.to_owned(),
        summary: None,
        description: None,
        tags: Vec::new(),
        deprecated: false,
        hidden: false,
    }
}

#[test]
fn emits_every_phase_one_extractor_and_both_callable_kinds() {
    let class = class(method(vec![
        param("id", TypeIr::string(), ParamKind::Positional),
        param("page", TypeIr::int(), ParamKind::Named),
        param("tags", TypeIr::list_of(TypeIr::string()), ParamKind::Named),
        param(
            "queryMap",
            TypeIr::map_of(TypeIr::string(), TypeIr::string()),
            ParamKind::Named,
        ),
        param("filter", TypeIr::named("Filter"), ParamKind::Named),
        param("token", TypeIr::string(), ParamKind::Named),
        param(
            "headers",
            TypeIr::map_of(TypeIr::string(), TypeIr::string()),
            ParamKind::Named,
        ),
        param(
            "json",
            TypeIr::map_of(TypeIr::string(), TypeIr::dynamic()),
            ParamKind::Named,
        ),
        param("todo", TypeIr::named("Todo"), ParamKind::Named),
    ]));
    let extractions = [
        Extraction::Path("id".to_owned()),
        Extraction::Query("page".to_owned()),
        Extraction::QueryList("tag".to_owned()),
        Extraction::Queries,
        Extraction::Queries,
        Extraction::Header("authorization".to_owned()),
        Extraction::HeaderMap,
        Extraction::Body(BodyFormat::Json),
        Extraction::Body(BodyFormat::Json),
    ];
    let method = &class.methods[0];
    let controller_handler = HandlerPlan {
        callable: Callable::Method(method),
        method: "POST",
        path: "/{id}".to_owned(),
        status: 204,
        params: method
            .params
            .iter()
            .zip(extractions)
            .map(|(param, extraction)| ParamPlan {
                param,
                value_type: &param.ty,
                wrapper: ExtractionWrapper::Required,
                extraction,
                validated: false,
                serde_deserialized: false,
            })
            .collect(),
        middleware: Vec::new(),
        metadata: metadata("write"),
    };
    let function = function();
    let function_handler = HandlerPlan {
        callable: Callable::Function(&function),
        method: "GET",
        path: "/health".to_owned(),
        status: 202,
        params: Vec::new(),
        middleware: Vec::new(),
        metadata: metadata("health"),
    };
    let mut plan = ServerPlan {
        controllers: vec![ControllerPlan {
            class: &class,
            prefix: "/todos".to_owned(),
            handlers: vec![controller_handler],
            middleware: Vec::new(),
        }],
        functions: vec![function_handler],
        function_prefix: "/api".to_owned(),
        function_name: "apiRoutes".to_owned(),
        function_middleware: Vec::new(),
    };

    let output = emit_server(&plan);
    let controller = output.mixin_members[0].members.join("\n");
    let top_level = output.top_level_functions.join("\n");
    for expected in [
        "PathExtractable<String>('id')",
        "QueryExtractable<int>('page')",
        "QueryListExtractable<String>('tag')",
        "QueriesExtractable<Map<String, String>>((queries) => queries)",
        "QueriesExtractable<Filter>",
        "HeaderExtractable<String>('authorization')",
        "HeaderMapExtractable()",
        "JsonExtractable<Map<String, dynamic>>((json) => json)",
        "JsonExtractable<Todo>",
        "return noContent();",
    ] {
        assert!(controller.contains(expected), "missing {expected}");
    }
    assert!(controller.contains("await (this as Todos).write("));
    assert!(top_level.contains("Router apiRoutes()"));
    assert!(top_level.contains("await health(),"));
    assert!(top_level.contains("status: 202,"));

    plan.controllers[0].handlers[0].status = 200;
    let explicit_void_status = emit_server(&plan).mixin_members[0].members.join("\n");
    assert!(explicit_void_status.contains("return Response(200);"));

    plan.functions.clear();
    assert!(emit_server(&plan).top_level_functions.is_empty());
}

#[test]
fn helper_boundaries_are_deterministic() {
    assert_eq!(generated_handler_name(""), "_$handle");
    let mut lines = Vec::new();
    push_call(&mut lines, "  ", "", "read", &["id".to_owned()], ";");
    assert_eq!(lines, ["  await read(id);"]);
    assert!(!is_string_map(&TypeIr::named("Map")));
    assert!(!is_json_map(&TypeIr::named("Map")));
    assert!(returns_void(&TypeIr::generic(
        "Future",
        vec![TypeIr::named("void")]
    )));
}

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
