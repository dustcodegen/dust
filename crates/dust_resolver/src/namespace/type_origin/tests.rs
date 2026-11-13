use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_text::SourceText;

use super::*;

const CORE: &str = "package:runtime/connection.dart";
const BARREL: &str = "package:runtime/ws.dart";
const FOREIGN: &str = "package:foreign/connection.dart";
const ROOT: &str = "package:app/server.dart";

fn insert(graph: &mut NamespaceGraph, id: u32, uri: &str, source: &str, lexical: bool) {
    let mut parsed = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(FileId::new(id), source),
        ParseOptions::default(),
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    if !lexical {
        parsed.library.lexical_scopes.clear();
    }
    graph
        .insert_library(uri, FileId::new(id), &parsed.library, |_, target| {
            Ok(target.to_owned())
        })
        .unwrap();
}

fn graph(source: &str, lexical: bool) -> NamespaceGraph {
    let mut graph = NamespaceGraph::new();
    insert(&mut graph, 1, CORE, "class Connection<S, R> {}", true);
    insert(
        &mut graph,
        2,
        BARREL,
        &format!("export '{CORE}' show Connection;"),
        true,
    );
    insert(&mut graph, 3, FOREIGN, "class Connection<S, R> {}", true);
    insert(&mut graph, 4, ROOT, source, lexical);
    graph
}

fn at(source: &str, needle: &str) -> SpanIr {
    let start = u32::try_from(source.find(needle).unwrap()).unwrap();
    SpanIr::new(
        FileId::new(4),
        TextRange::new(start, start + needle.len() as u32),
    )
}

fn expected(uri: &str) -> Option<TypeDeclarationOrigin> {
    Some(TypeDeclarationOrigin {
        library_uri: uri.to_owned(),
        name: "Connection".to_owned(),
    })
}

#[test]
fn direct_prefixed_and_reexported_types_keep_the_defining_origin() {
    for import in [CORE, BARREL] {
        for prefix in ["", "p."] {
            let suffix = if prefix.is_empty() { "" } else { " as p" };
            let name = format!("{prefix}Connection");
            let parameter = format!("{name}<int, String> socket");
            let source = format!("import '{import}'{suffix}; void handle({parameter}) {{}}");
            let graph = graph(&source, true);
            let span = at(&source, &parameter);
            assert_eq!(
                graph.scope(ROOT).type_declaration_origin(&name, span),
                expected(CORE)
            );
            let catalog = SymbolCatalog::new();
            let bindings = crate::BindingContext::new(&catalog, Some(graph.scope(ROOT)));
            assert_eq!(
                bindings.type_declaration_origin(&name, span),
                expected(CORE)
            );
            assert_eq!(
                crate::BindingContext::new(&catalog, None).type_declaration_origin(&name, span),
                None
            );
        }
    }
}

#[test]
fn aliases_cannot_hide_generic_argument_reordering() {
    for alias in [
        "typedef Socket<S, R> = p.Connection<S, R>;",
        "typedef Socket<S, R> = p.Connection<R, S>;",
        "typedef Socket = p.Connection<int, String>;",
        "typedef Connection<S, R> = p.Connection<R, S>;",
    ] {
        let name = if alias.starts_with("typedef Connection") {
            "Connection"
        } else {
            "Socket"
        };
        let parameter = format!("{name} socket");
        let source = format!("import '{CORE}' as p; {alias} void handle({parameter}) {{}}");
        assert_eq!(
            graph(&source, true)
                .scope(ROOT)
                .type_declaration_origin(name, at(&source, &parameter)),
            None,
            "{source}"
        );
    }
}

#[test]
fn generic_binders_shadow_a_type_or_import_prefix() {
    for (declaration, name, parameter) in [
        (
            "void handle<Connection>(Connection socket) {}",
            "Connection",
            "Connection socket",
        ),
        (
            "void handle<p>(p.Connection socket) {}",
            "p.Connection",
            "p.Connection socket",
        ),
        (
            "class Owner<Connection> { void handle(Connection socket) {} }",
            "Connection",
            "Connection socket",
        ),
    ] {
        let source = format!("import '{CORE}'; import '{CORE}' as p; {declaration}");
        assert_eq!(
            graph(&source, true)
                .scope(ROOT)
                .type_declaration_origin(name, at(&source, parameter)),
            None,
            "{source}"
        );
    }
}

#[test]
fn visibility_failures_and_conditional_disagreement_do_not_prove_an_origin() {
    for directive in [
        format!("import '{CORE}' hide Connection;"),
        format!("import '{CORE}'; import '{FOREIGN}';"),
        format!("import '{CORE}' if (dart.library.io) '{FOREIGN}';"),
        "import 'package:missing/input.dart';".to_owned(),
    ] {
        let source = format!("{directive} void handle(Connection socket) {{}}");
        assert_eq!(
            graph(&source, true)
                .scope(ROOT)
                .type_declaration_origin("Connection", at(&source, "Connection socket")),
            None,
            "{source}"
        );
    }
    let source = format!("import '{CORE}' deferred as p; void handle(p.Connection socket) {{}}");
    assert_eq!(
        graph(&source, true)
            .scope(ROOT)
            .type_declaration_origin("p.Connection", at(&source, "p.Connection socket")),
        None
    );
    let source = format!(
        "import '{CORE}' if (dart.library.io) '{CORE}'; void handle(Connection socket) {{}}"
    );
    assert_eq!(
        graph(&source, true)
            .scope(ROOT)
            .type_declaration_origin("Connection", at(&source, "Connection socket")),
        expected(CORE)
    );
}

#[test]
fn foreign_and_local_lookalikes_keep_distinct_origins() {
    let source = format!("import '{FOREIGN}'; void handle(Connection socket) {{}}");
    assert_eq!(
        graph(&source, true)
            .scope(ROOT)
            .type_declaration_origin("Connection", at(&source, "Connection socket")),
        expected(FOREIGN)
    );
    let source =
        format!("import '{CORE}'; class Connection {{}} void handle(Connection socket) {{}}");
    assert_eq!(
        graph(&source, true)
            .scope(ROOT)
            .type_declaration_origin("Connection", at(&source, "Connection socket")),
        expected(ROOT)
    );
    let source =
        format!("import '{CORE}'; const Connection = 1; void handle(Connection socket) {{}}");
    assert_eq!(
        graph(&source, true)
            .scope(ROOT)
            .type_declaration_origin("Connection", at(&source, "Connection socket")),
        None
    );
}

#[test]
fn missing_use_site_facts_fail_closed() {
    let source = format!("import '{CORE}'; void handle(Connection socket) {{}}");
    let span = at(&source, "Connection socket");
    assert_eq!(
        graph(&source, false)
            .scope(ROOT)
            .type_declaration_origin("Connection", span),
        None
    );
    assert_eq!(
        graph(&source, true)
            .scope(ROOT)
            .type_declaration_origin("Connection", SpanIr::new(FileId::new(99), span.range)),
        None
    );
}

#[test]
fn types_declared_in_parts_use_the_owning_library_origin() {
    let source = format!("import '{CORE}'; void handle(Connection socket) {{}}");
    let mut graph = NamespaceGraph::new();
    insert(
        &mut graph,
        1,
        CORE,
        "part 'package:runtime/part.dart';",
        true,
    );
    insert(
        &mut graph,
        2,
        "package:runtime/part.dart",
        &format!("part of '{CORE}'; class Connection<S, R> {{}}"),
        true,
    );
    insert(&mut graph, 4, ROOT, &source, true);
    assert_eq!(
        graph
            .scope(ROOT)
            .type_declaration_origin("Connection", at(&source, "Connection socket")),
        expected(CORE)
    );
}
