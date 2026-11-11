use dust_ir::AnnotationOrigin;
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_text::SourceText;

use super::*;

const ORIGIN: &str = "package:annotations/marker.dart";
const APP: &str = "package:app/root.dart";
const HELPER: &str = "package:app/helper.dart";

fn parse(id: u32, text: &str) -> ParsedDartFileSurface {
    let parsed = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(FileId::new(id), text),
        ParseOptions::default(),
    );
    assert_eq!(parsed.diagnostics, []);
    parsed.library
}

fn catalog() -> SymbolCatalog {
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("Marker", "test::Marker");
    catalog
        .register_annotation_origin(AnnotationOrigin::new("test::Marker", ORIGIN, "Marker"))
        .unwrap();
    catalog
}

#[test]
fn prepared_and_immediate_inputs_keep_bindings_diagnostics_and_lexical_requests_identical() {
    let app = format!(
        "library app; import '{ORIGIN}' as p; import '{HELPER}' as h; import 'missing.dart' as bad; part 'package:app/part.dart'; @h.marker class Example {{}}"
    );
    let facts = [
        (ORIGIN, parse(1, "class Marker { const Marker(); }")),
        (
            HELPER,
            parse(
                2,
                &format!(
                    "import '{ORIGIN}' as p; typedef MarkerAlias = p.Marker; const marker = p.Marker();"
                ),
            ),
        ),
        ("dart:core", parse(3, "class Object {}")),
        (
            "package:app/part.dart",
            parse(4, &format!("part of '{APP}'; class PartMarker {{}}")),
        ),
        (APP, parse(5, &app)),
    ];
    let core = parse(0, "import 'dart:core';");
    let build = |prepared| {
        let mut graph = NamespaceGraph::new();
        for (index, (uri, parsed)) in facts.iter().enumerate() {
            let id = FileId::new(index as u32 + 1);
            let appended = (*uri != "dart:core" && *uri != "package:app/part.dart")
                .then_some(&core.directives[0]);
            let resolve = |_: &str, value: &str| {
                if value == "missing.dart" {
                    Err("unavailable source".into())
                } else {
                    Ok(value.to_owned())
                }
            };
            if prepared {
                graph
                    .insert_prepared_library(NamespaceGraph::prepare_library(
                        *uri, id, parsed, appended, resolve,
                    ))
                    .unwrap();
            } else {
                graph
                    .insert_library_with_appended_directive(*uri, id, parsed, appended, resolve)
                    .unwrap();
            }
        }
        graph
    };
    let immediate = build(false);
    let prepared = build(true);
    let catalog = catalog();
    let annotation = &facts[4].1.classes[0].annotations[0];
    let span = dust_ir::SpanIr::new(FileId::new(5), annotation.span);
    for name in [
        "p.Marker",
        "h.MarkerAlias",
        "PartMarker",
        "bad.Marker",
        "unknown.Marker",
        "h.marker",
    ] {
        assert_eq!(
            prepared.scope(APP).bind_at(name, &catalog, span),
            immediate.scope(APP).bind_at(name, &catalog, span),
            "{name}"
        );
    }
    assert_eq!(
        prepared.scope(APP).bind("p.Marker", &catalog),
        NamespaceBinding::Owned(SymbolId::new("test::Marker"))
    );
    assert_eq!(
        prepared.scope(APP).bind("h.MarkerAlias", &catalog),
        NamespaceBinding::Owned(SymbolId::new("test::Marker"))
    );
    let NamespaceBinding::Rejected { diagnostic, .. } =
        prepared.scope(APP).bind_at("h.marker", &catalog, span)
    else {
        panic!("annotation constants remain unsupported")
    };
    assert_eq!(
        diagnostic.message,
        "Dust annotation constants are unsupported; use a direct annotation with its arguments"
    );
    assert!(!prepared.required_lexical_imports().is_empty());
    assert_eq!(
        prepared.required_lexical_imports(),
        immediate.required_lexical_imports()
    );
}

#[test]
fn prepared_directives_keep_decoding_and_written_then_appended_callback_order() {
    let parsed = parse(
        1,
        "import '\\x70ackage:app/a.dart' if (dart.library.io) 'package:app/b.dart'; part 'package:app/part.dart';",
    );
    let core = parse(1, "import 'dart:core';");
    let mut calls = Vec::new();
    let library = NamespaceGraph::prepare_library(
        APP,
        FileId::new(1),
        &parsed,
        Some(&core.directives[0]),
        |owner, reference| {
            calls.push((owner.to_owned(), reference.to_owned()));
            Ok(reference.to_owned())
        },
    );
    assert_eq!(
        calls
            .iter()
            .map(|(_, reference)| reference.as_str())
            .collect::<Vec<_>>(),
        [
            "package:app/a.dart",
            "package:app/b.dart",
            "package:app/part.dart",
            "dart:core"
        ]
    );
    assert!(calls.iter().all(|(owner, _)| owner == APP));
    NamespaceGraph::new()
        .insert_prepared_library(library)
        .unwrap();
}

#[test]
fn duplicate_prepared_identities_do_not_change_existing_cached_bindings() {
    let mut graph = NamespaceGraph::new();
    graph
        .insert_prepared_library(NamespaceGraph::prepare_library(
            ORIGIN,
            FileId::new(1),
            &parse(1, "class Marker {}"),
            None,
            |_, value| Ok(value.to_owned()),
        ))
        .unwrap();
    assert_eq!(
        graph.scope(ORIGIN).bind("Marker", &catalog()),
        NamespaceBinding::Owned(SymbolId::new("test::Marker"))
    );
    for (uri, id) in [(ORIGIN, 2), (APP, 1)] {
        let prepared = NamespaceGraph::prepare_library(
            uri,
            FileId::new(id),
            &parse(id, "class Replacement {}"),
            None,
            |_, value| Ok(value.to_owned()),
        );
        assert_eq!(
            graph.insert_prepared_library(prepared).unwrap_err().message,
            format!("duplicate namespace input identity `{uri}` or file id {id}")
        );
    }
    assert_eq!(
        graph.scope(ORIGIN).bind("Marker", &catalog()),
        NamespaceBinding::Owned(SymbolId::new("test::Marker"))
    );
}

#[test]
fn inserting_prepared_dependencies_invalidates_cached_missing_exports() {
    let mut graph = NamespaceGraph::new();
    graph
        .insert_library(
            ORIGIN,
            FileId::new(1),
            &parse(1, "class Marker {}"),
            |_, value| Ok(value.to_owned()),
        )
        .unwrap();
    graph
        .insert_library(
            APP,
            FileId::new(2),
            &parse(2, &format!("import '{HELPER}' as h;")),
            |_, value| Ok(value.to_owned()),
        )
        .unwrap();
    assert!(matches!(
        graph.scope(APP).bind("h.Marker", &catalog()),
        NamespaceBinding::Rejected {
            kind: NamespaceFailureKind::MissingInput,
            ..
        }
    ));
    let prepared = NamespaceGraph::prepare_library(
        HELPER,
        FileId::new(3),
        &parse(3, &format!("export '{ORIGIN}';")),
        None,
        |_, value| Ok(value.to_owned()),
    );
    graph.insert_prepared_library(prepared).unwrap();
    assert_eq!(
        graph.scope(APP).bind("h.Marker", &catalog()),
        NamespaceBinding::Owned(SymbolId::new("test::Marker"))
    );
}
