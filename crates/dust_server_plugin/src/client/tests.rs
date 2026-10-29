use dust_ir::{DartFileIr, FunctionIr, NameIr, SpanIr, TypeIr};
use dust_plugin_api::WorkspaceAnalysisBuilder;
use dust_text::{FileId, TextRange};
use serde_json::json;

use super::*;
use crate::{
    ServerClientDestination,
    analysis::SERVER_ROUTES_ANALYSIS_KEY,
    plan::{Callable, HandlerPlan, RouteMetadataPlan},
};

fn file(source: &str) -> DartFileIr {
    DartFileIr::empty(
        "/server",
        "server",
        source,
        "/server/lib/api.g.dart",
        SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32)),
    )
}

fn destination() -> ServerClientDestination {
    ServerClientDestination {
        package_name: "client".to_owned(),
        package_root: "/client".into(),
        tags: Vec::new(),
    }
}

fn empty_plan<'a>() -> ServerPlan<'a> {
    ServerPlan {
        controllers: Vec::new(),
        functions: Vec::new(),
        function_prefix: String::new(),
        function_name: "routes".to_owned(),
        function_middleware: Vec::new(),
    }
}

fn function(name: &str) -> FunctionIr {
    let span = SpanIr::new(FileId::new(1), TextRange::new(0_u32, 1_u32));
    FunctionIr {
        name: NameIr {
            source: name.to_owned(),
            short: name.to_owned(),
            prefix: None,
            span,
        },
        return_type: TypeIr::string(),
        params: Vec::new(),
        annotations: Vec::new(),
        span,
    }
}

fn handler<'a>(function: &'a FunctionIr, operation_id: &str) -> HandlerPlan<'a> {
    HandlerPlan {
        callable: Callable::Function(function),
        method: "GET",
        path: "/".to_owned(),
        status: 200,
        params: Vec::new(),
        middleware: Vec::new(),
        metadata: RouteMetadataPlan {
            operation_id: operation_id.to_owned(),
            summary: None,
            description: None,
            tags: Vec::new(),
            deprecated: false,
            hidden: false,
        },
    }
}

fn fact(callable: &str, method: &str, operation_id: &str) -> String {
    json!({
        "source_path": "/server/lib/api.dart",
        "controller": null,
        "prefix": "",
        "callable": callable,
        "method": method,
        "path": "/",
        "status": 200,
        "operation_id": operation_id,
        "summary": null,
        "description": null,
        "tags": [],
        "deprecated": false,
        "hidden": false,
        "params": [],
        "return_type": "String"
    })
    .to_string()
}

#[test]
fn client_paths_and_names_cover_nested_controller_and_module_shapes() {
    assert_eq!(
        file_class_name("lib/health-check.dart").unwrap(),
        "HealthCheck"
    );
    assert!(file_class_name("").is_err());
    assert_eq!(
        group_identity(Some("AdminHTTP"), "ignored.dart").unwrap(),
        ("AdminHTTPClient".to_owned(), "admin_h_t_t_p".to_owned())
    );
    assert_eq!(
        group_identity(None, "lib/health-check.dart").unwrap(),
        ("HealthCheckRoutesClient".to_owned(), "routes".to_owned())
    );
    assert_eq!(snake_case("URLValue"), "u_r_l_value");
    assert_eq!(
        main_output_path(&file("/server/lib/admin/todos.dart"), &destination()).unwrap(),
        PathBuf::from("/client/lib/src/generated/admin/todos_client.g.dart")
    );
    assert_eq!(
        main_output_path(&file("lib/todos.dart"), &destination()).unwrap(),
        PathBuf::from("/client/lib/src/generated/todos_client.g.dart")
    );
    assert!(main_output_path(&file("/outside/todos.dart"), &destination()).is_err());
    assert!(main_output_path(&file("/server/lib"), &destination()).is_err());
}

#[test]
fn client_generation_handles_empty_and_malformed_workspace_facts() {
    let empty = WorkspaceAnalysisBuilder::default().build();
    assert_eq!(
        generate_server_client(
            &file("/server/lib/api.dart"),
            &empty_plan(),
            &destination(),
            &empty,
        )
        .unwrap(),
        Vec::new()
    );

    let mut malformed = WorkspaceAnalysisBuilder::default();
    malformed.add_string_set_value(SERVER_ROUTES_ANALYSIS_KEY, "{");
    let error = generate_server_client(
        &file("/server/lib/api.dart"),
        &empty_plan(),
        &destination(),
        &malformed.build(),
    )
    .unwrap_err();
    assert!(error[0].message.contains("invalid dust_server.routes.v1"));
}

#[test]
fn client_generation_reports_duplicate_missing_and_invalid_top_level_facts() {
    let functions = [function("first"), function("second"), function("invalid")];
    let plan = ServerPlan {
        controllers: Vec::new(),
        functions: vec![
            handler(&functions[0], "same"),
            handler(&functions[1], "same"),
            handler(&functions[2], "invalid"),
        ],
        function_prefix: String::new(),
        function_name: "routes".to_owned(),
        function_middleware: Vec::new(),
    };
    let mut analysis = WorkspaceAnalysisBuilder::default();
    for value in [
        fact("first", "GET", "same"),
        fact("second", "GET", "same"),
        fact("absent", "GET", "missing"),
        fact("invalid", "TRACE", "invalid"),
    ] {
        analysis.add_string_set_value(SERVER_ROUTES_ANALYSIS_KEY, value);
    }

    let diagnostics = generate_server_client(
        &file("/server/lib/api.dart"),
        &plan,
        &destination(),
        &analysis.build(),
    )
    .unwrap_err();
    let messages = diagnostics
        .iter()
        .map(|value| value.message.as_str())
        .collect::<Vec<_>>();

    assert!(
        messages
            .iter()
            .any(|value| value.contains("more than once"))
    );
    assert!(
        messages
            .iter()
            .any(|value| value.contains("no longer matches"))
    );
    assert!(
        messages
            .iter()
            .any(|value| value.contains("unsupported HTTP method"))
    );
}
