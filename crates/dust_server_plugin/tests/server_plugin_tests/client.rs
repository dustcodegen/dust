use std::{path::PathBuf, sync::Arc};

use dust_ir::{AnnotationValueIr, ParamKind, PartIr, TypeIr};
use dust_plugin_api::{
    DustPlugin, GENERATED_HEADER, PluginContext, SymbolPlan, WorkspaceAnalysisBuilder,
};
use dust_server_plugin::{ServerClientDestination, register_plugin_with_clients};

mod split;

use super::{class, config, empty_library, method, param};

#[test]
fn generates_one_standalone_client_from_server_route_facts() {
    let mut library = empty_library();
    library.package_root.clear();
    let span = library.span;
    library.part_directives.push(PartIr {
        uri_literal: None,
        uri: "server.g.dart".to_owned(),
        span,
    });
    let mut id = param(vec![config("dust_server::Path", span)], span);
    id.name = "id".to_owned();
    let mut query = param(
        vec![dust_ir::ConfigApplicationIr::with_arguments(
            dust_ir::SymbolId::new("dust_server::Query"),
            None,
            vec![AnnotationValueIr::String("q".to_owned())],
            Default::default(),
            span,
        )],
        span,
    );
    query.name = "q".to_owned();
    query.kind = ParamKind::Named;
    query.is_required = false;
    query.ty = TypeIr::string().nullable();
    let mut read = method(
        vec![dust_ir::ConfigApplicationIr::with_arguments(
            dust_ir::SymbolId::new("dust_server::GET"),
            None,
            vec![AnnotationValueIr::String("/{id}".to_owned())],
            Default::default(),
            span,
        )],
        vec![id, query],
        span,
    );
    read.name = "readTodo".to_owned();
    read.return_type = TypeIr::string();
    let mut body = param(vec![config("dust_server::Body", span)], span);
    body.name = "body".to_owned();
    body.ty = TypeIr::map_of(TypeIr::string(), TypeIr::object().nullable());
    let mut state = param(vec![config("dust_server::State", span)], span);
    state.name = "state".to_owned();
    let mut create = method(
        vec![dust_ir::ConfigApplicationIr::with_arguments(
            dust_ir::SymbolId::new("dust_server::POST"),
            None,
            vec![AnnotationValueIr::String("/".to_owned())],
            Default::default(),
            span,
        )],
        vec![state, body],
        span,
    );
    create.name = "createTodo".to_owned();
    let mut delete_id = param(vec![config("dust_server::Path", span)], span);
    delete_id.name = "id".to_owned();
    let mut delete = method(
        vec![dust_ir::ConfigApplicationIr::with_arguments(
            dust_ir::SymbolId::new("dust_server::DELETE"),
            None,
            vec![AnnotationValueIr::String("/{id}".to_owned())],
            Default::default(),
            span,
        )],
        vec![delete_id],
        span,
    );
    delete.name = "deleteTodo".to_owned();
    let todos = class(
        vec![dust_ir::ConfigApplicationIr::with_arguments(
            dust_ir::SymbolId::new("dust_server::Controller"),
            None,
            vec![AnnotationValueIr::String("/todos".to_owned())],
            Default::default(),
            span,
        )],
        vec![read, create, delete],
        span,
    );
    let mut admin_method = method(
        vec![dust_ir::ConfigApplicationIr::with_arguments(
            dust_ir::SymbolId::new("dust_server::GET"),
            None,
            vec![AnnotationValueIr::String("/".to_owned())],
            Default::default(),
            span,
        )],
        Vec::new(),
        span,
    );
    admin_method.name = "listAdminTodos".to_owned();
    let mut admin = class(
        vec![dust_ir::ConfigApplicationIr::with_arguments(
            dust_ir::SymbolId::new("dust_server::Controller"),
            None,
            vec![AnnotationValueIr::String("/admin/todos".to_owned())],
            Default::default(),
            span,
        )],
        vec![admin_method],
        span,
    );
    admin.name = "AdminTodos".to_owned();
    library.classes = vec![todos, admin];
    let plugin = register_plugin_with_clients(vec![ServerClientDestination {
        package_name: "client".to_owned(),
        package_root: PathBuf::from("/client"),
        tags: Vec::new(),
    }]);
    let mut analysis = WorkspaceAnalysisBuilder::default();
    plugin.collect_workspace_analysis_ir(&library, &mut analysis);
    let mut plan = SymbolPlan::default();
    plan.set_workspace_analysis(Arc::new(analysis.build()));

    let output = plugin.generate(&library, &PluginContext { symbol_plan: &plan });

    assert_eq!(output[0].diagnostics, []);
    assert_eq!(output[0].auxiliary_outputs.len(), 3);
    let client = output[0]
        .auxiliary_outputs
        .iter()
        .find(|value| value.output_path.ends_with("server_client.todos.g.dart"))
        .unwrap();
    assert_eq!(
        client.output_path,
        PathBuf::from("/client/lib/src/generated/server_client.todos.g.dart")
    );
    assert!(client.source.contains("final class TodosClient"));
    assert!(
        client
            .source
            .contains("Future<String> readTodo(String id, {String? q}) async")
    );
    assert!(
        client
            .source
            .contains("'/todos/' + Uri.encodeComponent(id.toString())")
    );
    assert!(client.source.contains("_queryParameters['q'] = q"));
    assert!(client.source.contains("Future<String> createTodo("));
    assert!(client.source.contains("final Object? _data = body"));
    assert!(!client.source.contains("String state"));
    assert!(
        client
            .source
            .contains("Future<String> deleteTodo(String id)")
    );
    assert!(client.source.contains("method: 'DELETE'"));
    assert!(!client.source.contains("implements"));
    assert!(!client.source.contains("@override"));
    let admin = output[0]
        .auxiliary_outputs
        .iter()
        .find(|value| {
            value
                .output_path
                .ends_with("server_client.admin_todos.g.dart")
        })
        .unwrap();
    assert!(admin.source.contains("final class AdminTodosClient"));
    assert!(!admin.source.contains("readTodo"));
    assert!(!client.source.contains("listAdminTodos"));
    let barrel = output[0]
        .auxiliary_outputs
        .iter()
        .find(|value| value.output_path.ends_with("server_client.g.dart"))
        .unwrap();
    assert_eq!(
        barrel.output_path,
        PathBuf::from("/client/lib/src/generated/server_client.g.dart")
    );
    assert_eq!(
        barrel.source,
        format!(
            "{GENERATED_HEADER}\nexport 'server_client.admin_todos.g.dart';\nexport 'server_client.todos.g.dart';\n"
        )
    );
}

#[test]
fn rejects_unreachable_server_model_before_client_output() {
    let mut library = empty_library();
    let span = library.span;
    library.part_directives.push(PartIr {
        uri_literal: None,
        uri: "server.g.dart".to_owned(),
        span,
    });
    let mut read = method(
        vec![dust_ir::ConfigApplicationIr::with_arguments(
            dust_ir::SymbolId::new("dust_server::GET"),
            None,
            vec![AnnotationValueIr::String("/".to_owned())],
            Default::default(),
            span,
        )],
        Vec::new(),
        span,
    );
    read.return_type = TypeIr::named("Todo");
    library.classes = vec![class(
        vec![dust_ir::ConfigApplicationIr::with_arguments(
            dust_ir::SymbolId::new("dust_server::Controller"),
            None,
            vec![AnnotationValueIr::String("/api".to_owned())],
            Default::default(),
            span,
        )],
        vec![read],
        span,
    )];
    let mut todo = library.classes[0].clone();
    todo.name = "Todo".to_owned();
    todo.configs.clear();
    todo.methods.clear();
    todo.traits = ["Serialize", "Deserialize"]
        .into_iter()
        .map(|name| dust_ir::TraitApplicationIr {
            symbol: dust_ir::SymbolId::new(format!("dust_dart::{name}")),
            span,
        })
        .collect();
    library.classes.push(todo);
    let plugin = register_plugin_with_clients(vec![ServerClientDestination {
        package_name: "client".to_owned(),
        package_root: PathBuf::from("/client"),
        tags: Vec::new(),
    }]);
    let mut analysis = WorkspaceAnalysisBuilder::default();
    plugin.collect_workspace_analysis_ir(&library, &mut analysis);
    let mut plan = SymbolPlan::default();
    plan.set_workspace_analysis(Arc::new(analysis.build()));

    let output = plugin.generate(&library, &PluginContext { symbol_plan: &plan });

    assert!(output[0].auxiliary_outputs.is_empty());
    assert!(
        output[0]
            .diagnostics
            .iter()
            .any(|value| value.message.contains("not reachable")),
        "{:?}",
        output[0].diagnostics
    );
}
