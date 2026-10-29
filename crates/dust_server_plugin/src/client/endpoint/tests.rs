use dust_ir::{FunctionIr, MethodIr, MethodParamIr, NameIr, ParamKind, SpanIr, TypeIr};
use dust_text::{FileId, TextRange};
use serde_json::json;

use super::*;
use crate::{
    analysis::{ServerParamFact, ServerRouteFact},
    plan::{BodyFormat, Extraction, RouteMetadataPlan},
};

fn span() -> SpanIr {
    SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32))
}

fn param(name: &str, ty: TypeIr) -> MethodParamIr {
    MethodParamIr {
        name: name.to_owned(),
        ty,
        span: span(),
        kind: ParamKind::Positional,
        is_required: true,
        has_default: true,
        default_value_source: Some("'default'".to_owned()),
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

fn param_fact(name: &str, binding: &str) -> ServerParamFact {
    serde_json::from_value(json!({
        "name": name,
        "type_source": "String",
        "binding": binding,
        "binding_name": null,
        "wrapper": "required"
    }))
    .unwrap()
}

fn route_fact(params: Vec<ServerParamFact>) -> ServerRouteFact {
    serde_json::from_value(json!({
        "source_path": "lib/api.dart",
        "controller": "Api",
        "prefix": "/api/",
        "callable": "read",
        "method": "POST",
        "path": "/read",
        "status": 200,
        "operation_id": "read",
        "summary": null,
        "description": null,
        "tags": ["public"],
        "deprecated": false,
        "hidden": false,
        "params": params,
        "return_type": "String"
    }))
    .unwrap()
}

fn metadata() -> RouteMetadataPlan {
    RouteMetadataPlan {
        operation_id: "read".to_owned(),
        summary: None,
        description: None,
        tags: Vec::new(),
        deprecated: false,
        hidden: false,
    }
}

#[test]
fn bindings_cover_every_wire_and_server_only_parameter() {
    let string = TypeIr::string();
    let map = TypeIr::map_of(TypeIr::string(), TypeIr::string());
    let string_param = param("value", string.clone());
    let map_param = param("values", map.clone());
    let string_plan = ParamPlan {
        param: &string_param,
        value_type: &string,
        wrapper: ExtractionWrapper::Required,
        extraction: Extraction::Body(BodyFormat::Json),
        validated: false,
        serde_deserialized: false,
    };
    let map_plan = ParamPlan {
        param: &map_param,
        value_type: &map,
        wrapper: ExtractionWrapper::Required,
        extraction: Extraction::Queries,
        validated: false,
        serde_deserialized: false,
    };
    for (kind, expected) in [
        ("path", "Path"),
        ("query", "Query"),
        ("query-list", "Query"),
        ("header", "Header"),
        ("header-map", "HeaderMap"),
        ("body", "Body"),
        ("field", "Field"),
        ("part", "Part"),
    ] {
        let value = binding(&param_fact("value", kind), &string_plan, 0)
            .unwrap()
            .unwrap();
        assert!(format!("{value:?}").contains(expected));
    }
    assert!(matches!(
        binding(&param_fact("values", "queries"), &map_plan, 0).unwrap(),
        Some(GeneratedParam::Queries { .. })
    ));
    for kind in [
        "custom",
        "raw-query",
        "raw-body",
        "state",
        "peer",
        "request",
        "request-parts",
    ] {
        assert!(
            binding(&param_fact("value", kind), &string_plan, 0)
                .unwrap()
                .is_none()
        );
    }
    for kind in ["queries", "query-model"] {
        assert!(
            binding(&param_fact("value", kind), &string_plan, 0)
                .unwrap_err()
                .message
                .contains("cannot yet be encoded")
        );
    }
    assert!(
        binding(&param_fact("value", "unknown"), &string_plan, 0)
            .unwrap_err()
            .message
            .contains("unknown binding")
    );
}

#[test]
fn parameter_wrappers_and_return_types_preserve_client_contracts() {
    let value = TypeIr::string();
    let source = param("value", TypeIr::named("Option"));
    for (wrapper, required, nullable) in [
        (ExtractionWrapper::Required, true, false),
        (ExtractionWrapper::Fallible, true, false),
        (ExtractionWrapper::Optional, false, true),
    ] {
        let plan = ParamPlan {
            param: &source,
            value_type: &value,
            wrapper,
            extraction: Extraction::State,
            validated: false,
            serde_deserialized: false,
        };
        let client = client_param(&plan);
        assert_eq!(client.is_required, required);
        assert_eq!(client.ty.is_nullable(), nullable);
        assert!(!client.has_default);
        assert!(client.default_value_source.is_none());
    }
    for (source, expected) in [
        (TypeIr::string(), TypeIr::string()),
        (
            TypeIr::generic("Future", vec![TypeIr::string()]),
            TypeIr::string(),
        ),
        (
            TypeIr::generic("Result", vec![TypeIr::int(), TypeIr::named("Failure")]),
            TypeIr::int(),
        ),
        (
            TypeIr::generic("Option", vec![TypeIr::string()]),
            TypeIr::string().nullable(),
        ),
        (TypeIr::named("Unit"), TypeIr::named("void")),
        (TypeIr::named("void"), TypeIr::named("void")),
    ] {
        assert_eq!(
            client_return_type(&source).unwrap(),
            TypeIr::generic("Future", vec![expected])
        );
    }
    assert!(client_return_type(&TypeIr::named("Response")).is_err());
}

#[test]
fn verbs_paths_and_endpoint_failures_cover_normalized_boundaries() {
    for (name, expected) in [
        ("GET", GeneratedHttpVerb::Get),
        ("POST", GeneratedHttpVerb::Post),
        ("PUT", GeneratedHttpVerb::Put),
        ("PATCH", GeneratedHttpVerb::Patch),
        ("DELETE", GeneratedHttpVerb::Delete),
        ("HEAD", GeneratedHttpVerb::Head),
        ("OPTIONS", GeneratedHttpVerb::Options),
    ] {
        assert_eq!(verb(name).unwrap(), expected);
    }
    assert!(verb("TRACE").is_err());
    assert_eq!(join_path("", "/health"), "/health");
    assert_eq!(join_path("/api/", "/health"), "/api/health");
    assert!(is_string_map(&TypeIr::map_of(
        TypeIr::string(),
        TypeIr::string()
    )));
    assert!(!is_string_map(&TypeIr::map_of(
        TypeIr::string(),
        TypeIr::int()
    )));

    let method = MethodIr {
        name: "read".to_owned(),
        is_static: false,
        is_external: false,
        return_type: TypeIr::string(),
        has_body: false,
        body_source: None,
        params: Vec::new(),
        span: span(),
        traits: Vec::new(),
        configs: Vec::new(),
    };
    let handler = HandlerPlan {
        callable: Callable::Method(&method),
        method: "GET",
        path: "/read".to_owned(),
        status: 200,
        params: Vec::new(),
        middleware: Vec::new(),
        metadata: metadata(),
    };
    assert!(
        build_endpoint(&route_fact(vec![param_fact("missing", "query")]), &handler)
            .err()
            .unwrap()
            .message
            .contains("no longer matches")
    );
}

#[test]
fn endpoint_request_modes_and_function_spans_are_derived_from_facts() {
    for (binding, expected) in [
        ("part", GeneratedRequestMode::MultiPart),
        ("field", GeneratedRequestMode::FormUrlEncoded),
    ] {
        let function = FunctionIr {
            name: NameIr {
                source: "upload".to_owned(),
                short: "upload".to_owned(),
                prefix: None,
                span: span(),
            },
            return_type: TypeIr::string(),
            params: vec![param("wire", TypeIr::string())],
            annotations: Vec::new(),
            span: span(),
        };
        let extraction = if binding == "part" {
            Extraction::Part {
                name: "wire".to_owned(),
                file: false,
            }
        } else {
            Extraction::Field("wire".to_owned())
        };
        let handler = HandlerPlan {
            callable: Callable::Function(&function),
            method: "POST",
            path: "/upload".to_owned(),
            status: 200,
            params: vec![ParamPlan {
                param: &function.params[0],
                value_type: &function.params[0].ty,
                wrapper: ExtractionWrapper::Required,
                extraction,
                validated: false,
                serde_deserialized: false,
            }],
            middleware: Vec::new(),
            metadata: metadata(),
        };
        let endpoint =
            build_endpoint(&route_fact(vec![param_fact("wire", binding)]), &handler).unwrap();
        assert_eq!(endpoint.request_mode, expected);
        assert_eq!(endpoint.method.span, function.span);
    }
}
