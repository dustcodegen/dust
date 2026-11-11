use super::*;
use std::collections::BTreeSet;

fn bind_marker(graph: &dust_resolver::NamespaceGraph, source: &str) -> NamespaceBinding {
    let parsed = parse(3, source);
    let span = dust_ir::SpanIr::new(FileId::new(3), parsed.classes[0].annotations[0].span);
    graph.scope(APP).bind_at("marker", &catalog(false), span)
}

#[test]
fn local_constructors_constant_chains_and_qualified_members_need_no_imports() {
    for (values, declarations) in [
        (
            "const marker = _Marker();",
            "class _Marker { const _Marker(); }",
        ),
        (
            "const first = _Marker(); const marker = first;",
            "class _Marker { const _Marker(); }",
        ),
        (
            "const marker = _Marker.named();",
            "class _Marker { const _Marker.named(); }",
        ),
        (
            "const marker = Alias();",
            "typedef Alias = _Marker; class _Marker { const _Marker(); }",
        ),
        (
            "const marker = Holder.value;",
            "class Holder { static const value = _Marker(); } class _Marker { const _Marker(); }",
        ),
    ] {
        let mut graph = graph();
        let source = format!("{values} @marker class Example {{}} {declarations}");
        add(&mut graph, 3, APP, &source);
        assert_eq!(
            bind_marker(&graph, &source),
            NamespaceBinding::Foreign,
            "{source}"
        );
        assert!(graph.required_lexical_imports().is_empty(), "{source}");
    }
}

#[test]
fn local_alias_cycles_and_local_part_declarations_need_no_imports() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        APP,
        "typedef GET = Other; typedef Other = GET;",
    );
    assert_eq!(
        graph.scope(APP).bind("GET", &catalog(false)),
        NamespaceBinding::Unknown
    );
    assert!(graph.required_lexical_imports().is_empty());

    let mut graph = super::graph();
    let part = "package:app/markers.dart";
    let source = format!(
        "part '{part}'; const marker = Alias(); @marker class Example {{}} typedef Alias = _Marker;"
    );
    add(&mut graph, 3, APP, &source);
    add(
        &mut graph,
        4,
        part,
        &format!("part of '{APP}'; class _Marker {{ const _Marker(); }}"),
    );
    assert_eq!(bind_marker(&graph, &source), NamespaceBinding::Foreign);
    assert!(graph.required_lexical_imports().is_empty());
}

#[test]
fn local_chains_request_imports_when_their_actual_target_is_nonlocal() {
    let mut graph = graph();
    let origin = "package:foreign/marker.dart";
    add(&mut graph, 4, origin, "class Foreign { const Foreign(); }");
    let source = format!(
        "import '{origin}'; typedef Alias = Foreign; const first = Alias(); const marker = first; @marker class Example {{}}"
    );
    add(&mut graph, 3, APP, &source);
    assert_eq!(bind_marker(&graph, &source), NamespaceBinding::Foreign);
    assert_eq!(
        graph.required_lexical_imports(),
        BTreeSet::from([APP.into()])
    );
}

#[test]
fn conflicting_local_facts_and_import_prefixes_keep_requests() {
    for source in [
        "class Local {} class Local {} typedef GET = Local;".to_owned(),
        format!("import '{CLIENT}' as p; class p {{}} typedef GET = p.GET;"),
    ] {
        let mut graph = graph();
        add(&mut graph, 3, APP, &source);
        assert!(matches!(
            graph.scope(APP).bind("GET", &catalog(false)),
            NamespaceBinding::Rejected { .. }
        ));
        assert_eq!(
            graph.required_lexical_imports(),
            BTreeSet::from([APP.into()]),
            "{source}"
        );
    }
}

#[test]
fn a_queried_initializer_with_missing_part_facts_still_requests_imports() {
    let mut graph = graph();
    let source = "part 'package:app/missing.dart'; class Api { static const marker = Local(); @marker void call() {} } class Local {}";
    add(&mut graph, 3, APP, source);
    let parsed = parse(3, source);
    let span = dust_ir::SpanIr::new(
        FileId::new(3),
        parsed.classes[0].methods[0].annotations[0].span,
    );
    rejection(
        graph.scope(APP).bind_at("marker", &catalog(false), span),
        Failure::MissingInput,
    );
    assert_eq!(
        graph.required_lexical_imports(),
        BTreeSet::from([APP.into()])
    );
}
