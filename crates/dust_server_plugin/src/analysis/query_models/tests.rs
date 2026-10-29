use dust_ir::{
    ClassIr, ClassKindIr, FunctionIr, ImportIr, MethodParamIr, NameIr, ParamKind, SpanIr, SymbolId,
    TraitApplicationIr, TypeIr,
};
use dust_plugin_api::WorkspaceAnalysisBuilder;
use dust_text::{FileId, TextRange};

use super::*;
use crate::plan::{Callable, ExtractionWrapper, HandlerPlan, ParamPlan, RouteMetadataPlan};

mod hydration;

fn span() -> SpanIr {
    SpanIr::new(FileId::new(3), TextRange::new(0_u32, 1_u32))
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

fn fact(library: &str) -> QueryModelFact {
    QueryModelFact {
        package: "contracts".to_owned(),
        library: library.to_owned(),
        name: "Filter".to_owned(),
        serializable: true,
        validatable: false,
        errors: Vec::new(),
        conversions: Vec::new(),
    }
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

fn param_plan(param: &MethodParamIr) -> ParamPlan<'_> {
    ParamPlan {
        param,
        value_type: &param.ty,
        wrapper: ExtractionWrapper::Required,
        extraction: Extraction::QueryModel(QueryModelPlan {
            conversions: Vec::new(),
        }),
        validated: false,
        serde_deserialized: true,
    }
}

fn function() -> FunctionIr {
    FunctionIr {
        name: NameIr {
            source: "search".to_owned(),
            short: "search".to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: TypeIr::string(),
        params: Vec::new(),
        annotations: Vec::new(),
        span: span(),
    }
}

fn plan<'a>(function: &'a FunctionIr, params: Vec<ParamPlan<'a>>) -> ServerPlan<'a> {
    ServerPlan {
        controllers: Vec::new(),
        functions: vec![HandlerPlan {
            callable: Callable::Function(function),
            method: "GET",
            path: "/search".to_owned(),
            status: 200,
            params,
            middleware: Vec::new(),
            metadata: RouteMetadataPlan {
                operation_id: "search".to_owned(),
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
fn resolves_direct_barrel_missing_and_ambiguous_imports() {
    let file = file();
    let direct = fact("package:contracts/models.dart");
    assert_eq!(
        imported_fact(&file, std::slice::from_ref(&direct), "contracts", "Filter")
            .unwrap()
            .library,
        direct.library
    );

    let barrel = fact("package:contracts/src/filter.dart");
    assert_eq!(
        imported_fact(&file, std::slice::from_ref(&barrel), "contracts", "Filter")
            .unwrap()
            .library,
        barrel.library
    );
    assert!(
        imported_fact(&file, &[], "contracts", "Filter")
            .err()
            .unwrap()
            .message
            .contains("must derive Deserialize")
    );
    assert!(
        imported_fact(
            &file,
            &[barrel, fact("package:contracts/src/other.dart")],
            "contracts",
            "Filter",
        )
        .err()
        .unwrap()
        .message
        .contains("ambiguous")
    );
    assert!(
        imported_fact(&file, &[], "missing", "Filter")
            .err()
            .unwrap()
            .message
            .contains("not imported")
    );

    let mut relative = file;
    relative.import_directives[0].uri = "models.dart".to_owned();
    assert!(
        imported_fact(&relative, &[], "contracts", "Filter")
            .err()
            .unwrap()
            .message
            .contains("package import")
    );
}

#[test]
fn reports_invalid_facts_and_ignores_non_library_paths() {
    let mut analysis = WorkspaceAnalysisBuilder::default();
    analysis.add_string_set_value(QUERY_MODELS_ANALYSIS_KEY, "{");
    let analysis = analysis.build();
    assert!(
        model_facts(&analysis)
            .err()
            .unwrap()
            .message
            .contains("invalid dust_server.query_models.v1")
    );
    let handler_function = function();
    let mut server_plan = plan(&handler_function, Vec::new());
    assert_eq!(
        hydrate_query_models(&file(), &mut server_plan, &analysis, false).len(),
        1
    );

    let invalid = DartFileIr::empty("/pkg", "pkg", "outside.dart", "outside.g.dart", span());
    let mut analysis = WorkspaceAnalysisBuilder::default();
    collect_query_model_workspace_analysis_ir(&invalid, &mut analysis);
    assert!(
        analysis
            .build()
            .string_set(QUERY_MODELS_ANALYSIS_KEY)
            .is_none()
    );
}
