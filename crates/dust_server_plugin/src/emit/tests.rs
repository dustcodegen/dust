use dust_ir::{
    ClassIr, ClassKindIr, FunctionIr, MethodIr, MethodParamIr, NameIr, ParamKind, SpanIr, TypeIr,
};
use dust_text::{FileId, TextRange};

use super::*;
use crate::plan::{Callable, ControllerPlan};

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
        Extraction::Body,
        Extraction::Body,
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
            .map(|(param, extraction)| ParamPlan { param, extraction })
            .collect(),
    };
    let function = function();
    let function_handler = HandlerPlan {
        callable: Callable::Function(&function),
        method: "GET",
        path: "/health".to_owned(),
        status: 202,
        params: Vec::new(),
    };
    let plan = ServerPlan {
        controllers: vec![ControllerPlan {
            class: &class,
            prefix: "/todos".to_owned(),
            handlers: vec![controller_handler],
        }],
        functions: vec![function_handler],
        function_prefix: "/api".to_owned(),
        function_name: "apiRoutes".to_owned(),
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
