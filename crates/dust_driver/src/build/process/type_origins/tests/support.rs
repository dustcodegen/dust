use dust_diagnostics::Diagnostic;
use dust_ir::{DartFileIr, SpanIr};
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_plugin_api::{
    DustPlugin, PluginContext, PluginContribution, PluginRegistry, TypeOriginRequest,
    WorkspaceAnalysisBuilder,
};
use dust_resolver::{BindingContext, NamespaceGraph, SymbolCatalog};
use dust_text::{FileId, SourceText, TextRange};

pub(super) const CORE: &str = "package:dust_dart/src/ws/connection.dart";
pub(super) const BARREL: &str = "package:dust_dart/ws.dart";
pub(super) const FOREIGN: &str = "package:foreign/ws.dart";
pub(super) const APP: &str = "package:app/routes.dart";
pub(super) const TYPE: &str = "WebSocketConnection";

pub(super) struct RequestPlugin {
    name: &'static str,
    requests: Vec<TypeOriginRequest>,
}

impl RequestPlugin {
    pub(super) fn new(name: &'static str, requests: Vec<TypeOriginRequest>) -> Self {
        Self { name, requests }
    }
}

impl DustPlugin for RequestPlugin {
    fn plugin_name(&self) -> &'static str {
        self.name
    }

    fn type_origin_requests(&self, _: &DartFileIr) -> Vec<TypeOriginRequest> {
        assert_ne!(
            self.name, "inactive",
            "symbols-only plugin must not execute"
        );
        self.requests.clone()
    }

    fn validate(&self, _: &DartFileIr) -> Vec<Diagnostic> {
        vec![]
    }

    fn generate(&self, _: &DartFileIr, _: &PluginContext<'_>) -> Vec<PluginContribution> {
        vec![]
    }
}

fn insert(graph: &mut NamespaceGraph, id: u32, uri: &str, source: &str) {
    let parsed = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(FileId::new(id), source),
        ParseOptions::default(),
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    graph
        .insert_library(uri, FileId::new(id), &parsed.library, |_, target| {
            Ok(target.to_owned())
        })
        .unwrap();
}

pub(super) fn fixture(
    source: &str,
    name: &str,
    parameter: &str,
) -> (NamespaceGraph, DartFileIr, TypeOriginRequest) {
    let mut graph = NamespaceGraph::new();
    insert(&mut graph, 1, CORE, "class WebSocketConnection<S, R> {}");
    insert(&mut graph, 2, BARREL, &format!("export '{CORE}';"));
    insert(&mut graph, 3, FOREIGN, "class WebSocketConnection<S, R> {}");
    insert(&mut graph, 4, APP, source);
    let start = u32::try_from(source.find(parameter).unwrap()).unwrap();
    let span = SpanIr::new(
        FileId::new(4),
        TextRange::new(start, start + parameter.len() as u32),
    );
    let file = DartFileIr::empty(
        "/workspace",
        "app",
        "/workspace/lib/routes.dart",
        "/workspace/lib/routes.g.dart",
        SpanIr::new(FileId::new(4), TextRange::new(0u32, source.len() as u32)),
    );
    (
        graph,
        file,
        TypeOriginRequest {
            name: name.to_owned(),
            span,
        },
    )
}

pub(super) fn collect_request(
    graph: &NamespaceGraph,
    file: &DartFileIr,
    request: &TypeOriginRequest,
) -> WorkspaceAnalysisBuilder {
    let catalog = SymbolCatalog::new();
    let mut registry = PluginRegistry::new();
    registry
        .register(Box::new(RequestPlugin::new(
            "request",
            vec![request.clone()],
        )))
        .unwrap();
    let mut analysis = WorkspaceAnalysisBuilder::default();
    super::super::collect(
        file,
        &registry,
        || BindingContext::new(&catalog, Some(graph.scope(APP))),
        &mut analysis,
    );
    analysis
}
