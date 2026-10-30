use dust_ir::{
    ClassIr, ClassKindIr, ConstructorIr, ConstructorParamIr, FunctionIr, ImportIr, MethodIr,
    MethodParamIr, NameIr, ParamKind, SpanIr, SymbolId, TraitApplicationIr, TypeIr,
};
use dust_plugin_api::WorkspaceAnalysisBuilder;
use dust_text::{FileId, TextRange};

use super::facts::PAYLOAD_MODELS_KEY;
use super::*;
use crate::plan::{Callable, ExtractionWrapper, HandlerPlan, ParamPlan, RouteMetadataPlan};

mod edges;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(9), TextRange::new(0_u32, 1_u32))
}

fn file() -> DartFileIr {
    let mut file = DartFileIr::empty(
        "/server",
        "server",
        "/server/lib/api.dart",
        "/server/lib/api.g.dart",
        span(),
    );
    file.import_directives.push(ImportIr {
        namespace: None,
        uri: "package:contracts/models.dart".to_owned(),
        prefix: Some("contracts".to_owned()),
        show: Vec::new(),
        hide: Vec::new(),
        is_deferred: false,
        span: span(),
    });
    file
}

fn fact(name: &str) -> PayloadModelFact {
    PayloadModelFact {
        package: "contracts".to_owned(),
        library: "package:contracts/models.dart".to_owned(),
        name: name.to_owned(),
        deserializable: true,
        serializable: true,
        validatable: false,
        deserialize_method: false,
        from_json: false,
        to_json: false,
    }
}

fn analysis(facts: &[PayloadModelFact]) -> WorkspaceAnalysis {
    let mut analysis = WorkspaceAnalysisBuilder::default();
    for fact in facts {
        analysis.add_string_set_value(PAYLOAD_MODELS_KEY, serde_json::to_string(fact).unwrap());
    }
    analysis.build()
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

fn function(return_type: TypeIr) -> FunctionIr {
    FunctionIr {
        name: NameIr {
            source: "create".to_owned(),
            short: "create".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type,
        params: Vec::new(),
        annotations: Vec::new(),
        span: span(),
    }
}

fn plan<'a>(function: &'a FunctionIr, body: Option<&'a MethodParamIr>) -> ServerPlan<'a> {
    let params = body
        .into_iter()
        .map(|param| ParamPlan {
            param,
            value_type: &param.ty,
            wrapper: ExtractionWrapper::Required,
            extraction: Extraction::Body(BodyFormat::Json),
            validated: false,
            serde_deserialized: false,
        })
        .collect();
    ServerPlan {
        controllers: Vec::new(),
        functions: vec![HandlerPlan {
            callable: Callable::Function(function),
            method: "POST",
            path: "/".to_owned(),
            status: 200,
            params,
            middleware: Vec::new(),
            metadata: RouteMetadataPlan {
                operation_id: "create".to_owned(),
                summary: None,
                description: None,
                tags: Vec::new(),
                deprecated: false,
                hidden: false,
            },
        }],
        function_prefix: String::new(),
        function_name: "routes".to_owned(),
        function_middleware: Vec::new(),
    }
}

#[test]
fn collects_serde_and_compatibility_capabilities() {
    let json = TypeIr::map_of(TypeIr::string(), TypeIr::object().nullable());
    let mut source = DartFileIr::empty(
        "/contracts",
        "contracts",
        "/contracts/lib/models.dart",
        "/contracts/lib/models.g.dart",
        span(),
    );
    source.classes.push(ClassIr {
        kind: ClassKindIr::Class,
        name: "Todo".to_owned(),
        type_parameters: Vec::new(),
        mixins: Vec::new(),
        is_abstract: false,
        is_interface: false,
        superclass_name: None,
        superclass: None,
        span: span(),
        fields: Vec::new(),
        constructors: vec![ConstructorIr {
            name: Some("fromJson".to_owned()),
            is_factory: true,
            is_const: false,
            redirected_target_source: None,
            redirected_target_name: None,
            redirected_target: None,
            span: span(),
            params: vec![ConstructorParamIr {
                name: "json".to_owned(),
                ty: json.clone(),
                span: span(),
                kind: ParamKind::Positional,
                is_required: true,
                has_default: false,
                default_value_source: None,
            }],
        }],
        methods: vec![
            method(
                "deserialize",
                true,
                json.clone(),
                vec![param("json", json.clone())],
            ),
            method("toJson", false, json, Vec::new()),
        ],
        traits: ["Deserialize", "Serialize", "Validate"]
            .into_iter()
            .map(|name| TraitApplicationIr {
                symbol: SymbolId::new(format!("dust_dart::{name}")),
                span: span(),
            })
            .collect(),
        configs: Vec::new(),
        serde: None,
    });
    let mut builder = WorkspaceAnalysisBuilder::default();
    collect_payload_model_workspace_analysis_ir(&source, &mut builder);
    let values = model_facts(&builder.build()).unwrap();
    let value = &values[0];
    assert!(value.deserializable && value.serializable && value.validatable);
    assert!(value.deserialize_method && value.from_json && value.to_json);

    let outside = DartFileIr::empty("/pkg", "pkg", "outside.dart", "out.g.dart", span());
    let mut builder = WorkspaceAnalysisBuilder::default();
    collect_payload_model_workspace_analysis_ir(&outside, &mut builder);
    assert!(builder.build().string_set(PAYLOAD_MODELS_KEY).is_none());
}

fn method(
    name: &str,
    is_static: bool,
    return_type: TypeIr,
    params: Vec<MethodParamIr>,
) -> MethodIr {
    MethodIr {
        name: name.to_owned(),
        is_static,
        is_external: false,
        return_type,
        has_body: true,
        body_source: None,
        params,
        span: span(),
        traits: Vec::new(),
        configs: Vec::new(),
    }
}

#[test]
fn hydrates_imported_body_and_reports_missing_capabilities() {
    let body = param("input", TypeIr::named("contracts.CreateTodo"));
    let handler = function(TypeIr::named("Future"));
    let mut server = plan(&handler, Some(&body));
    let mut model = fact("CreateTodo");
    model.validatable = true;
    let diagnostics = hydrate_payload_models(&file(), &mut server, &analysis(&[model]), true);
    assert!(diagnostics.is_empty());
    assert!(server.functions[0].params[0].serde_deserialized);
    assert!(server.functions[0].params[0].validated);

    let mut invalid = fact("CreateTodo");
    invalid.deserializable = false;
    invalid.serializable = false;
    let mut server = plan(&handler, Some(&body));
    let diagnostics = hydrate_payload_models(&file(), &mut server, &analysis(&[invalid]), true);
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("Deserialize"))
    );
    assert!(
        diagnostics
            .iter()
            .any(|value| value.message.contains("Serialize"))
    );

    let mut server = plan(&handler, Some(&body));
    assert!(hydrate_payload_models(&file(), &mut server, &analysis(&[]), true).is_empty());

    let response = function(TypeIr::named("contracts.CreateTodo"));
    let mut server = plan(&response, None);
    let mut invalid = fact("CreateTodo");
    invalid.deserializable = false;
    invalid.serializable = false;
    assert_eq!(
        hydrate_payload_models(&file(), &mut server, &analysis(&[invalid]), true).len(),
        2
    );
}

#[test]
fn validates_response_shapes_and_fact_failures() {
    let response = TypeIr::generic(
        "Future",
        vec![TypeIr::generic(
            "List",
            vec![TypeIr::named("contracts.Todo").nullable()],
        )],
    );
    let mut duplicate = fact("Todo");
    duplicate.library = "package:contracts/other.dart".to_owned();
    let types = response_serde_types(&file(), &response, &analysis(&[fact("Todo")])).unwrap();
    assert_eq!(types, ["contracts.Todo"]);
    assert!(
        response_serde_types(&file(), &response, &analysis(&[fact("Todo"), duplicate])).is_ok()
    );

    let mut missing = fact("Todo");
    missing.deserializable = false;
    missing.serializable = false;
    let errors = response_serde_types(&file(), &response, &analysis(&[missing])).unwrap_err();
    assert_eq!(errors.len(), 2);

    let mut compatible = fact("Todo");
    compatible.deserializable = false;
    compatible.serializable = false;
    compatible.from_json = true;
    compatible.to_json = true;
    assert!(
        response_serde_types(&file(), &response, &analysis(&[compatible]))
            .unwrap()
            .is_empty()
    );

    let mut builder = WorkspaceAnalysisBuilder::default();
    builder.add_string_set_value(PAYLOAD_MODELS_KEY, "{");
    assert!(response_serde_types(&file(), &response, &builder.build()).is_err());
}
