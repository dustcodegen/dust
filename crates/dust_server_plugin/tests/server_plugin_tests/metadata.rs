use dust_ir::{AnnotationValueIr, ConfigApplicationIr, PartIr, SymbolId};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan, WorkspaceAnalysisBuilder};
use dust_server_plugin::register_plugin;

use super::{annotation, class, empty_library, function, method, param};

fn configured(
    symbol: &str,
    path: &str,
    named: impl IntoIterator<Item = (&'static str, AnnotationValueIr)>,
    span: dust_ir::SpanIr,
) -> ConfigApplicationIr {
    ConfigApplicationIr::with_arguments(
        SymbolId::new(symbol),
        None,
        vec![AnnotationValueIr::String(path.to_owned())],
        named
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
        span,
    )
}

#[test]
fn emits_metadata_and_collects_structured_route_facts() {
    let mut library = empty_library();
    let span = library.span;
    library.part_directives.push(PartIr {
        uri_literal: None,
        uri: "server.g.dart".to_owned(),
        span,
    });
    let controller = configured(
        "dust_server::Controller",
        "/todos",
        [(
            "tags",
            AnnotationValueIr::List(vec![AnnotationValueIr::String("group".to_owned())]),
        )],
        span,
    );
    let route = configured(
        "dust_server::GET",
        "/{id}",
        [
            ("summary", AnnotationValueIr::String("Read".to_owned())),
            (
                "tags",
                AnnotationValueIr::List(vec![AnnotationValueIr::String("route".to_owned())]),
            ),
            ("deprecated", AnnotationValueIr::Bool(true)),
        ],
        span,
    );
    let path = configured("dust_server::Path", "id", [], span);
    library.classes = vec![class(
        vec![controller],
        vec![method(vec![route], vec![param(vec![path], span)], span)],
        span,
    )];

    let mut routes = annotation("dust_server::Routes", span);
    routes.positional_args = vec![AnnotationValueIr::String("/health".to_owned())];
    library.library_annotations = vec![routes];
    let mut top_level = annotation("dust_server::GET", span);
    top_level.positional_args = vec![AnnotationValueIr::String("/".to_owned())];
    library.functions = vec![function(vec![top_level], Vec::new(), span)];

    let plugin = register_plugin();
    let output = plugin.generate(
        &library,
        &PluginContext {
            symbol_plan: &SymbolPlan::default(),
        },
    );
    let generated = output[0]
        .mixin_members
        .iter()
        .flat_map(|value| &value.members)
        .chain(output[0].top_level_functions.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(generated.contains("operationId: 'read'"));
    assert!(generated.contains("summary: 'Read'"));
    assert!(generated.contains("tags: ['group', 'route']"));
    assert!(generated.contains("deprecated: true"));

    let mut analysis = WorkspaceAnalysisBuilder::default();
    plugin.collect_workspace_analysis_ir(&library, &mut analysis);
    let snapshot = analysis.snapshot();
    let facts = snapshot
        .string_set("dust_server.routes.v1")
        .unwrap()
        .iter()
        .map(|value| serde_json::from_str::<serde_json::Value>(value).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(facts.len(), 2);
    let controller = facts
        .iter()
        .find(|value| value["controller"] == "Todos")
        .unwrap();
    assert_eq!(controller["prefix"], "/todos");
    assert_eq!(controller["path"], "/{id}");
    assert_eq!(controller["operation_id"], "read");
    assert_eq!(controller["tags"], serde_json::json!(["group", "route"]));
    assert_eq!(controller["params"][0]["binding"], "path");
    assert_eq!(controller["params"][0]["type_source"], "String");
    assert!(facts.iter().any(|value| value["controller"].is_null()));
}

#[test]
fn invalid_libraries_do_not_publish_workspace_facts() {
    let mut library = empty_library();
    library.library_annotations = vec![annotation("dust_server::Routes", library.span)];
    let mut analysis = WorkspaceAnalysisBuilder::default();

    register_plugin().collect_workspace_analysis_ir(&library, &mut analysis);

    assert!(analysis.snapshot().is_empty());
}

#[test]
fn malformed_metadata_returns_diagnostics_without_generated_code() {
    let mut library = empty_library();
    let span = library.span;
    library.part_directives.push(PartIr {
        uri_literal: None,
        uri: "server.g.dart".to_owned(),
        span,
    });
    let controller = configured("dust_server::Controller", "/todos", [], span);
    let route = configured(
        "dust_server::GET",
        "/",
        [("summary", AnnotationValueIr::Bool(true))],
        span,
    );
    library.classes = vec![class(
        vec![controller],
        vec![method(vec![route], Vec::new(), span)],
        span,
    )];

    let output = register_plugin().generate(
        &library,
        &PluginContext {
            symbol_plan: &SymbolPlan::default(),
        },
    );

    assert!(output[0].mixin_members.is_empty());
    assert!(output[0].top_level_functions.is_empty());
    assert!(
        output[0]
            .diagnostics
            .iter()
            .any(|value| value.message.contains("`summary` must be a string"))
    );
}
