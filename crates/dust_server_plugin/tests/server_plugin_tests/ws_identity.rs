use std::sync::Arc;

use dust_ir::TraitApplicationIr;
use dust_plugin_api::WorkspaceAnalysisBuilder;

use super::*;

/// Builds a typed WebSocket fixture with distinct message directions.
fn socket(span: SpanIr, name: &str) -> MethodParamIr {
    let mut socket = param(Vec::new(), span);
    socket.name = "socket".to_owned();
    socket.ty = TypeIr::generic(
        name,
        vec![TypeIr::named("Notice"), TypeIr::named("Command")],
    );
    socket
}

/// Builds a server source fixture with shared message contracts.
fn source() -> DartFileIr {
    let mut source = empty_library();
    let span = source.span;
    source.part_directives.push(PartIr {
        uri_literal: None,
        uri: "server.g.dart".to_owned(),
        span,
    });
    let mut ws = annotation("dust_server::WS", span);
    ws.positional_args = vec![AnnotationValueIr::String("/chat".to_owned())];
    let mut handler = function(
        vec![ws],
        vec![socket(span, "socket.WebSocketConnection")],
        span,
    );
    handler.return_type = TypeIr::named("void");
    source.functions.push(handler);
    for (name, contract) in [("Notice", "Serialize"), ("Command", "Deserialize")] {
        let mut model = class(Vec::new(), Vec::new(), span);
        model.name = name.to_owned();
        model.traits.push(TraitApplicationIr {
            symbol: SymbolId::new(format!("dust_dart::{contract}")),
            span,
        });
        source.classes.push(model);
    }
    source
}

/// Checks origin requests are limited to actual typed upgrade handlers.
#[test]
fn only_ws_parameters_request_origins_including_potential_aliases() {
    let mut source = source();
    let span = source.span;
    let mut alias = source.functions[0].clone();
    alias.params = vec![socket(span, "SocketAlias")];
    source.functions.push(alias);
    source.functions.push(function(
        vec![annotation("dust_server::GET", span)],
        vec![socket(span, "Ignored")],
        span,
    ));
    let handler = method(
        vec![config("dust_server::WS", span)],
        vec![
            socket(span, "controller.WebSocketConnection"),
            param(Vec::new(), span),
        ],
        span,
    );
    source.classes.push(class(
        vec![config("dust_server::Controller", span)],
        vec![handler.clone()],
        span,
    ));
    source.classes.push(class(Vec::new(), vec![handler], span));
    let requests = register_plugin().type_origin_requests(&source);
    assert_eq!(
        requests
            .iter()
            .map(|request| request.name.as_str())
            .collect::<Vec<_>>(),
        [
            "controller.WebSocketConnection",
            "socket.WebSocketConnection",
            "SocketAlias"
        ]
    );
    assert!(requests.iter().all(|request| request.span == span));
    assert!(
        register_plugin()
            .type_origin_requests(&empty_library())
            .is_empty()
    );
}

/// Checks missing or foreign origins cannot produce typed upgrade output.
#[test]
fn final_generation_requires_the_concrete_canonical_origin_fact() {
    let source = source();
    let plugin = register_plugin();
    let empty = SymbolPlan::default();
    assert!(!plugin.validate_with_plan(&source, &empty).is_empty());
    assert!(
        !plugin.generate(
            &source,
            &PluginContext {
                symbol_plan: &empty
            }
        )[0]
        .diagnostics
        .is_empty()
    );
    let request = plugin.type_origin_requests(&source).remove(0);
    for library in [
        "package:other/ws.dart",
        "package:dust_dart/src/ws/connection.dart",
    ] {
        let mut builder = WorkspaceAnalysisBuilder::default();
        builder.add_type_origin(
            &source.source_path,
            &request,
            library,
            "WebSocketConnection",
        );
        plugin.collect_workspace_analysis_ir(&source, &mut builder);
        let mut symbols = SymbolPlan::default();
        symbols.set_workspace_analysis(Arc::new(builder.build()));
        let errors = plugin.validate_with_plan(&source, &symbols);
        let output = plugin.generate(
            &source,
            &PluginContext {
                symbol_plan: &symbols,
            },
        );
        if library.contains("other") {
            assert!(!errors.is_empty());
            assert!(!output[0].diagnostics.is_empty());
            assert!(output[0].top_level_functions.is_empty());
        } else {
            assert!(errors.is_empty());
            assert!(output[0].diagnostics.is_empty());
            assert_eq!(output[0].top_level_functions.len(), 2);
            assert!(
                symbols
                    .workspace_analysis()
                    .string_set("dust_server.routes.v1")
                    .is_none()
            );
            assert_eq!(
                symbols
                    .workspace_analysis()
                    .string_set("dust_server.ws.v1")
                    .unwrap()
                    .len(),
                1
            );
        }
    }
}
