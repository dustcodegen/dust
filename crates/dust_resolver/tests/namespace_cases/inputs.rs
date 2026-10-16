use super::*;

#[test]
fn callback_gets_decoded_literals_and_preserves_uri_resolution_failures() {
    let mut graph = graph();
    let source = r"import '\x70ackage:dust_dart/src/http/http_client_annotations.dart' as p;";
    let parsed = parse(3, source);
    let mut calls = Vec::new();
    graph
        .insert_library(APP, FileId::new(3), &parsed, |owner, value| {
            calls.push((owner.to_owned(), value.to_owned()));
            Ok(value.to_owned())
        })
        .unwrap();
    assert_eq!(calls, [(APP.to_owned(), CLIENT.to_owned())]);
    assert_eq!(graph.scope(APP).bind("p.GET", &catalog(false)), owned());

    let parsed = parse(4, "import 'relative.dart' as p;");
    graph
        .insert_library("package:app/bad.dart", FileId::new(4), &parsed, |_, _| {
            Err("package config resolution failed".to_owned())
        })
        .unwrap();
    let diagnostic = rejection(
        graph
            .scope("package:app/bad.dart")
            .bind("p.GET", &catalog(false)),
        Failure::MissingInput,
    );
    assert_eq!(diagnostic.message, "package config resolution failed");
    assert_eq!(
        diagnostic.labels[0].range,
        dust_text::TextRange::new(7_u32, 22_u32)
    );
}

#[test]
fn missing_structured_facts_never_fall_back_to_legacy_uri_text() {
    let mut graph = graph();
    let mut parsed = parse(3, &format!("import '{CLIENT}' as p;"));
    let dust_parser_dart::ParsedDirective::Import { namespace, .. } = &mut parsed.directives[0]
    else {
        panic!("import expected")
    };
    *namespace = None;
    graph
        .insert_library(
            APP,
            FileId::new(3),
            &parsed,
            |_, value| Ok(value.to_owned()),
        )
        .unwrap();
    let diagnostic = rejection(
        graph.scope(APP).bind("p.GET", &catalog(false)),
        Failure::MissingInput,
    );
    assert_eq!(
        diagnostic.message,
        format!("missing ordered namespace facts in `{APP}`")
    );
}

#[test]
fn duplicate_identity_is_rejected_without_changing_graph_or_cached_results() {
    let mut graph = graph();
    add(&mut graph, 3, APP, &format!("import '{CLIENT}' as p;"));
    assert_eq!(graph.scope(APP).bind("p.GET", &catalog(false)), owned());
    for (uri, id) in [(APP, 9), ("package:app/duplicate.dart", 3)] {
        let error = graph
            .insert_library(
                uri,
                FileId::new(id),
                &parse(id, "class GET {}"),
                |_, value| Ok(value.to_owned()),
            )
            .unwrap_err();
        assert_eq!(
            error.message,
            format!("duplicate namespace input identity `{uri}` or file id {id}")
        );
    }
    assert_eq!(graph.scope(APP).bind("p.GET", &catalog(false)), owned());
}

#[test]
fn adding_required_input_invalidates_previously_cached_export_failures() {
    let mut graph = graph();
    let barrel = "package:barrel/a.dart";
    add(&mut graph, 3, APP, &format!("import '{barrel}' as p;"));
    rejection(
        graph.scope(APP).bind("p.GET", &catalog(false)),
        Failure::MissingInput,
    );
    add(&mut graph, 4, barrel, &format!("export '{CLIENT}';"));
    assert_eq!(graph.scope(APP).bind("p.GET", &catalog(false)), owned());
    rejection(
        graph
            .scope("package:missing/a.dart")
            .bind("GET", &catalog(false)),
        Failure::MissingInput,
    );
}

#[test]
fn conflicting_owner_metadata_is_not_resolved_by_registration_order() {
    let mut graph = graph();
    add(&mut graph, 3, APP, &format!("import '{CLIENT}' as p;"));
    let mut catalog = catalog(false);
    catalog.register_config("Other", "other::GET");
    catalog
        .register_annotation_origin(AnnotationOrigin::new("other::GET", CLIENT, "GET"))
        .unwrap();
    let diagnostic = rejection(graph.scope(APP).bind("p.GET", &catalog), Failure::Ambiguous);
    assert_eq!(
        diagnostic.message,
        "declaration `GET` has multiple registered owners: dust_dart::GET, other::GET"
    );
}
