use dust_plugin_api::{PluginRegistry, WorkspaceAnalysisBuilder};

use super::collect;

mod snapshots;
mod support;

use support::*;

#[test]
fn no_requests_never_create_the_lazy_binding_context() {
    let (_, file, request) = fixture(
        &format!("import '{BARREL}' as ws; void handle(ws.WebSocketConnection socket) {{}}"),
        "ws.WebSocketConnection",
        "ws.WebSocketConnection socket",
    );
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(RequestPlugin::new("empty", vec![])))
        .unwrap();
    registry
        .register_symbols_only(Box::new(RequestPlugin::new("inactive", vec![request])))
        .unwrap();
    let mut analysis = WorkspaceAnalysisBuilder::default();
    collect(
        &file,
        &registry,
        || panic!("no request needs bindings"),
        &mut analysis,
    );
    assert!(analysis.snapshot().is_empty());
}

#[test]
fn executing_requests_use_real_direct_prefixed_and_reexported_namespace_facts() {
    for import in [CORE, BARREL] {
        for (suffix, name) in [
            ("", "WebSocketConnection"),
            (" as ws", "ws.WebSocketConnection"),
        ] {
            let parameter = format!("{name}<int, String> socket");
            let source = format!("import '{import}'{suffix}; void handle({parameter}) {{}}");
            let (graph, file, request) = fixture(&source, name, &parameter);
            let analysis = collect_request(&graph, &file, &request);
            assert!(analysis.has_type_origin(&file.source_path, &request, CORE, TYPE));
            assert!(!analysis.has_type_origin("/workspace/lib/other.dart", &request, CORE, TYPE));
            assert!(!analysis.has_type_origin(&file.source_path, &request, BARREL, TYPE));
        }
    }
}

#[test]
fn unresolved_and_typedef_requests_cannot_gain_a_canonical_proof() {
    for (source, name, parameter) in [
        (
            "void handle(WebSocketConnection socket) {}".to_owned(),
            "WebSocketConnection",
            "WebSocketConnection socket",
        ),
        (
            format!(
                "import '{BARREL}' as ws; typedef Socket<S, R> = ws.WebSocketConnection<R, S>; void handle(Socket socket) {{}}"
            ),
            "Socket",
            "Socket socket",
        ),
        (
            format!(
                "import '{BARREL}' as ws; typedef WebSocketConnection<S, R> = ws.WebSocketConnection<S, R>; void handle(WebSocketConnection socket) {{}}"
            ),
            "WebSocketConnection",
            "WebSocketConnection socket",
        ),
    ] {
        let (graph, file, request) = fixture(&source, name, parameter);
        let analysis = collect_request(&graph, &file, &request);
        assert!(analysis.snapshot().is_empty(), "{source}");
        assert!(
            !analysis.has_type_origin(&file.source_path, &request, CORE, TYPE),
            "{source}"
        );
    }
}

#[test]
fn foreign_and_local_lookalikes_retain_their_actual_origin() {
    for (source, origin) in [
        (
            format!("import '{FOREIGN}'; void handle(WebSocketConnection socket) {{}}"),
            FOREIGN,
        ),
        (
            format!(
                "import '{BARREL}'; class WebSocketConnection {{}} void handle(WebSocketConnection socket) {{}}"
            ),
            APP,
        ),
    ] {
        let (graph, file, request) = fixture(&source, TYPE, "WebSocketConnection socket");
        let analysis = collect_request(&graph, &file, &request);
        assert!(analysis.has_type_origin(&file.source_path, &request, origin, TYPE));
        assert!(!analysis.has_type_origin(&file.source_path, &request, CORE, TYPE));
    }
}

#[test]
fn collecting_two_sources_does_not_share_their_spelling_or_offsets() {
    let canonical = format!("import '{CORE}'; void handle(WebSocketConnection socket) {{}}");
    let padding = " ".repeat(CORE.len() - FOREIGN.len());
    let foreign =
        format!("import '{FOREIGN}';{padding} void handle(WebSocketConnection socket) {{}}");
    let (canonical_graph, canonical_file, request) =
        fixture(&canonical, TYPE, "WebSocketConnection socket");
    let (foreign_graph, mut foreign_file, foreign_request) =
        fixture(&foreign, TYPE, "WebSocketConnection socket");
    assert_eq!(request.span, foreign_request.span);
    foreign_file.source_path = "/workspace/lib/foreign.dart".to_owned();
    let mut analysis = collect_request(&canonical_graph, &canonical_file, &request);
    analysis.merge(collect_request(
        &foreign_graph,
        &foreign_file,
        &foreign_request,
    ));
    let analysis = analysis.build();
    assert!(analysis.has_type_origin(&canonical_file.source_path, &request, CORE, TYPE));
    assert!(!analysis.has_type_origin(&foreign_file.source_path, &foreign_request, CORE, TYPE));
    assert!(analysis.has_type_origin(&foreign_file.source_path, &foreign_request, FOREIGN, TYPE));
}
