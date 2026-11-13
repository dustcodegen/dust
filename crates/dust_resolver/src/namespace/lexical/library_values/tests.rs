use super::*;
use dust_ir::{AnnotationOrigin, SymbolId};
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
        ("void Callback() {}".into(), "Callback", true),
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

#[test]
fn exact_kind_gate_keeps_part_aliases_constants_and_original_definition_labels() {
    const OWNER: &str = "package:parts/owner.dart";
    const PART: &str = "package:parts/piece.dart";
    const BARREL: &str = "package:parts/barrel.dart";
    let owner = format!("import '{OWNED}' as d; part '{PART}'; const local = d.Marker();");
    let part = format!(
        "part of '{OWNER}'; typedef Alias = d.Marker; class Foreign {{ const Foreign(); }} \
         const marker = Foreign(); const forbidden = d.Marker();"
    );
    for reexported in [false, true] {
        let mut graph = NamespaceGraph::new();
        add(
            &mut graph,
            1,
            OWNED,
            "class Marker { const Marker(); }",
            true,
        );
        add(&mut graph, 2, OWNER, &owner, true);
        add(&mut graph, 3, PART, &part, true);
        add(&mut graph, 4, BARREL, &format!("export '{OWNER}';"), true);
        let target = if reexported { BARREL } else { OWNER };
        add(
            &mut graph,
            5,
            ROOT,
            &format!("import '{target}' as p;"),
            true,
        );
        let mut catalog = SymbolCatalog::new();
        catalog.register_config("Marker", "owned::Marker");
        catalog
            .register_annotation_origin(AnnotationOrigin::new("owned::Marker", OWNED, "Marker"))
            .unwrap();
        assert_eq!(
            graph.library_constant(ROOT, "p.Alias", &catalog, &mut BTreeSet::new()),
            Some(NamespaceBinding::Owned(SymbolId::new("owned::Marker")))
        );
        assert_eq!(
            graph.library_constant(ROOT, "p.marker", &catalog, &mut BTreeSet::new()),
            Some(NamespaceBinding::Foreign)
        );
        for (name, id, source) in [("local", 2, &owner), ("forbidden", 3, &part)] {
            let declaration = format!("{name} = d.Marker()");
            let start = source.find(&declaration).unwrap() as u32;
            let expected = rejected(
                NamespaceFailureKind::MissingInput,
                "Dust annotation constants are unsupported; use a direct annotation with its arguments",
                vec![SourceLabel::new(
                    FileId::new(id),
                    TextRange::new(start, start + declaration.len() as u32),
                    format!("lexical declaration `{name}` shadows imported names"),
                )],
            );
            assert_eq!(
                graph.library_constant(ROOT, &format!("p.{name}"), &catalog, &mut BTreeSet::new()),
                Some(expected),
                "reexported={reexported}, name={name}"
            );
        }
    }
}
