//! Actual parser facts and runtime annotation definitions drive namespace ownership.

use dust_ir::AnnotationOrigin;
use dust_parser_dart::{ParseBackend, ParseOptions, ParsedDartFileSurface};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{NamespaceBinding, NamespaceGraph, SymbolCatalog};
use dust_text::{FileId, SourceText};
use std::{fs, path::Path};

/// The real declaring libraries, not similarly named public barrels.
const CLIENT: &str = "package:dust_dart/src/http/http_client_annotations.dart";
/// The server's independently declared GET must never become a client verb.
const SERVER: &str = "package:dust_server/src/annotations/http_methods.dart";
const APP: &str = "package:app/main.dart";

mod namespace_cases;

/// Parse real source with exact source coordinates retained.
fn parse(id: u32, source: &str) -> ParsedDartFileSurface {
    let source = SourceText::new(FileId::new(id), source);
    let result = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(result.diagnostics, vec![]);
    result.library
}

/// Test edges already use canonical package URIs; this is not a filesystem resolver.
fn add(graph: &mut NamespaceGraph, id: u32, uri: &str, source: &str) {
    graph
        .insert_library(uri, FileId::new(id), &parse(id, source), |_, value| {
            if value.starts_with("package:") || value.starts_with("dart:") {
                Ok(value.to_owned())
            } else {
                Err(format!("fixture has no URI mapping for `{value}`"))
            }
        })
        .unwrap();
}

/// Legacy backends can provide declarations without newer lexical/value facts.
fn add_without_lexical_facts(graph: &mut NamespaceGraph, id: u32, uri: &str, source: &str) {
    let mut parsed = parse(id, source);
    parsed.lexical_scopes.clear();
    graph
        .insert_library(uri, FileId::new(id), &parsed, |_, value| {
            Ok(value.to_owned())
        })
        .unwrap();
}

/// Use actual runtime class definitions in the ownership regression.
fn graph() -> NamespaceGraph {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut graph = NamespaceGraph::new();
    for (id, uri, path) in [
        (
            1,
            CLIENT,
            "dust_dart/lib/src/http/http_client_annotations.dart",
        ),
        (
            2,
            SERVER,
            "dust_server/lib/src/annotations/http_methods.dart",
        ),
    ] {
        add(
            &mut graph,
            id,
            uri,
            &fs::read_to_string(root.join("packages").join(path)).unwrap(),
        );
    }
    graph
}

/// Catalog registration order is not a namespace decision.
fn catalog(reverse: bool) -> SymbolCatalog {
    let mut catalog = SymbolCatalog::new();
    let mut definitions = [("dust_dart::GET", CLIENT), ("dust_server::GET", SERVER)];
    if reverse {
        definitions.reverse();
    }
    for (symbol, uri) in definitions {
        catalog.register_config("GET", symbol);
        catalog
            .register_annotation_origin(AnnotationOrigin::new(symbol, uri, "GET"))
            .unwrap();
    }
    catalog
}

fn owned() -> NamespaceBinding {
    NamespaceBinding::Owned(dust_ir::SymbolId::new("dust_dart::GET"))
}

fn rejection(
    binding: NamespaceBinding,
    kind: dust_resolver::NamespaceFailureKind,
) -> dust_diagnostics::Diagnostic {
    let NamespaceBinding::Rejected {
        kind: actual,
        diagnostic,
    } = binding
    else {
        panic!("expected {kind:?}, got {binding:?}");
    };
    assert_eq!(actual, kind);
    diagnostic
}

#[test]
fn prefixed_real_client_and_server_declarations_keep_their_owners_in_both_orders() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        "package:app/main.dart",
        &format!(
            "import '{CLIENT}' as client; import '{SERVER}' as server; class Api {{ @client.GET('/a') void clientMethod() {{}} @server.GET('/b') void serverMethod() {{}} }}"
        ),
    );
    for reverse in [false, true] {
        let catalog = catalog(reverse);
        let scope = graph.scope("package:app/main.dart");
        assert_eq!(
            scope.bind("client.GET", &catalog),
            NamespaceBinding::Owned(dust_ir::SymbolId::new("dust_dart::GET"))
        );
        assert_eq!(
            scope.bind("server.GET", &catalog),
            NamespaceBinding::Owned(dust_ir::SymbolId::new("dust_server::GET"))
        );
    }
}
