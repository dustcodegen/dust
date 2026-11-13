use super::*;
use dust_ir::AnnotationOrigin;
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_text::SourceText;

const ROOT: &str = "package:app/root.dart";
const OWNED: &str = "package:owned/marker.dart";
const FOREIGN: &str = "package:foreign/marker.dart";

/// Real declaration facts drive both the optimized path and its ordinary-name oracle.
fn add(graph: &mut NamespaceGraph, id: u32, uri: &str, source: &str, lexical: bool) {
    let mut parsed = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(FileId::new(id), source),
        ParseOptions::default(),
    );
    assert_eq!(parsed.diagnostics, []);
    if !lexical {
        parsed.library.lexical_scopes.clear();
    }
    graph
        .insert_library(uri, FileId::new(id), &parsed.library, |_, uri| {
            Ok(uri.to_owned())
        })
        .unwrap();
}

#[test]
fn ordinary_library_values_reuse_worlds_with_identical_bindings_and_requests() {
    let cases = [
        (format!("import '{OWNED}';"), "Marker", true),
        (format!("import '{OWNED}' as p;"), "p.Marker", true),
        (format!("import '{FOREIGN}';"), "Marker", true),
        (
            format!("import '{OWNED}' as p; typedef Alias = p.Marker;"),
            "Alias",
            true,
        ),
        (
            "typedef First = Second; typedef Second = First;".into(),
            "First",
            true,
        ),
        ("const Marker = unknown;".into(), "Marker", false),
        (
            format!("import '{OWNED}'; import '{FOREIGN}';"),
            "Marker",
            true,
        ),
        (
            format!("import '{OWNED}' if (dart.library.io) '{FOREIGN}';"),
            "Marker",
            true,
        ),
        ("class Other {}".into(), "Absent", true),
    ];
    for (source, name, lexical) in cases {
        let mut graph = NamespaceGraph::new();
        add(&mut graph, 1, OWNED, "class Marker {}", true);
        add(&mut graph, 2, FOREIGN, "class Marker {}", true);
        add(&mut graph, 3, ROOT, &source, lexical);
        let mut catalog = SymbolCatalog::new();
        catalog.register_config("Marker", "owned::Marker");
        catalog
            .register_annotation_origin(AnnotationOrigin::new("owned::Marker", OWNED, "Marker"))
            .unwrap();
        let expected = graph.bind_name(ROOT, name, &catalog, &mut BTreeSet::new());
        let requests = graph.required_lexical_imports();
        graph.clear_required_lexical_imports();
        assert_eq!(
            graph.library_constant(ROOT, name, &catalog, &mut BTreeSet::new()),
            Some(expected),
            "{source}: {name}"
        );
        assert_eq!(
            graph.required_lexical_imports(),
            requests,
            "{source}: {name}"
        );
    }
}
