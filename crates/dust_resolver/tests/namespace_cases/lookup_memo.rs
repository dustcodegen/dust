use super::*;

#[test]
fn repeated_namespace_queries_do_not_cache_catalog_ownership() {
    let mut graph = graph();
    add(&mut graph, 3, APP, &format!("import '{CLIENT}' as p;"));
    for _ in 0..2 {
        assert_eq!(graph.scope(APP).bind("p.GET", &catalog(false)), owned());
        assert_eq!(
            graph.scope(APP).bind("p.GET", &SymbolCatalog::new()),
            NamespaceBinding::Foreign
        );
        rejection(
            graph.scope(APP).bind("GET", &catalog(false)),
            Failure::Hidden,
        );
        rejection(
            graph.scope(APP).bind("absent.GET", &catalog(false)),
            Failure::UndeclaredPrefix,
        );
    }
    let mut conflicting = catalog(false);
    conflicting.register_config("Other", "other::GET");
    conflicting
        .register_annotation_origin(AnnotationOrigin::new("other::GET", CLIENT, "GET"))
        .unwrap();
    rejection(
        graph.scope(APP).bind("p.GET", &conflicting),
        Failure::Ambiguous,
    );
    assert_eq!(graph.scope(APP).bind("p.GET", &catalog(true)), owned());
}

#[test]
fn repeated_constant_queries_reemit_lexical_import_requests() {
    let mut graph = graph();
    let source =
        "import 'package:foreign/marker.dart'; const marker = Foreign(); @marker class Example {}";
    add(
        &mut graph,
        4,
        "package:foreign/marker.dart",
        "class Foreign { const Foreign(); }",
    );
    add(&mut graph, 3, APP, source);
    let parsed = parse(3, source);
    let span = dust_ir::SpanIr::new(FileId::new(3), parsed.classes[0].annotations[0].span);
    for _ in 0..2 {
        assert_eq!(
            graph.scope(APP).bind_at("marker", &catalog(false), span),
            NamespaceBinding::Foreign
        );
        assert_eq!(
            graph.required_lexical_imports(),
            std::collections::BTreeSet::from([APP.to_owned()])
        );
        graph.clear_required_lexical_imports();
    }
}

#[test]
fn repeated_cached_prefix_failures_keep_each_annotation_use_label_separate() {
    let mut graph = graph();
    let source = "class Api { @absent.GET void first() {} @absent.GET void second() {} }";
    add(&mut graph, 3, APP, source);
    let parsed = parse(3, source);
    let catalog = catalog(false);
    let bindings = dust_resolver::BindingContext::new(&catalog, Some(graph.scope(APP)));
    for method in &parsed.classes[0].methods {
        let annotation = &method.annotations[0];
        let mut diagnostics = Vec::new();
        bindings.annotation_ir(FileId::new(3), annotation, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].labels.len(), 1);
        assert_eq!(diagnostics[0].labels[0].range, annotation.span);
    }
}
