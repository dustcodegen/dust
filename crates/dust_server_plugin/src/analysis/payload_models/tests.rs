use dust_ir::{FunctionIr, ImportIr, MethodParamIr, NameIr, ParamKind, SpanIr, TypeIr};
use dust_plugin_api::WorkspaceAnalysisBuilder;
use dust_text::{FileId, TextRange};

use super::facts::PAYLOAD_MODELS_KEY;
use super::*;
use crate::plan::{Callable, ExtractionWrapper, HandlerPlan, ParamPlan, RouteMetadataPlan};

mod collection;
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
    }
}

fn analysis(facts: &[PayloadModelFact]) -> WorkspaceAnalysis {
    let mut analysis = WorkspaceAnalysisBuilder::default();
    for fact in facts {
        analysis.add_string_set_value(PAYLOAD_MODELS_KEY, serde_json::to_string(fact).unwrap());
    }
    analysis.build()
}

fn models(facts: &[PayloadModelFact]) -> PayloadCapabilityIndex {
    PayloadCapabilityIndex::from_analysis(&analysis(facts)).unwrap()
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
fn hydrates_imported_body_and_reports_missing_capabilities() {
    let body = param("input", TypeIr::named("contracts.CreateTodo"));
    let handler = function(TypeIr::named("Future"));
    let mut server = plan(&handler, Some(&body));
    let mut model = fact("CreateTodo");
    model.validatable = true;
    let diagnostics = hydrate_payload_models(&file(), &mut server, &models(&[model]), true);
    assert!(diagnostics.is_empty());
    assert!(server.functions[0].params[0].serde_deserialized);
    assert!(server.functions[0].params[0].validated);

    let mut invalid = fact("CreateTodo");
    invalid.deserializable = false;
    invalid.serializable = false;
    let mut server = plan(&handler, Some(&body));
    let diagnostics = hydrate_payload_models(&file(), &mut server, &models(&[invalid]), true);
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(Diagnostic::has_labels));
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
    assert_eq!(
        hydrate_payload_models(&file(), &mut server, &models(&[]), true).len(),
        1
    );

    let response = function(TypeIr::named("contracts.CreateTodo"));
    let mut server = plan(&response, None);
    let mut invalid = fact("CreateTodo");
    invalid.deserializable = false;
    invalid.serializable = false;
    assert_eq!(
        hydrate_payload_models(&file(), &mut server, &models(&[invalid]), true).len(),
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
    let types = response_serde_types(&file(), &response, span(), &models(&[fact("Todo")])).unwrap();
    assert_eq!(types, ["contracts.Todo"]);
    assert!(
        response_serde_types(
            &file(),
            &response,
            span(),
            &models(&[fact("Todo"), duplicate])
        )
        .is_ok()
    );

    let mut missing = fact("Todo");
    missing.deserializable = false;
    missing.serializable = false;
    let errors = response_serde_types(&file(), &response, span(), &models(&[missing])).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(Diagnostic::has_labels));

    let mut builder = WorkspaceAnalysisBuilder::default();
    builder.add_string_set_value(PAYLOAD_MODELS_KEY, "{");
    assert!(PayloadCapabilityIndex::from_analysis(&builder.build()).is_err());
}
