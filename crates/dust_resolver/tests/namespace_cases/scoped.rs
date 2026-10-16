use super::*;
use dust_ir::{HttpConfigIr, HttpVerbIr, NormalizedConfigIr, SymbolId};
use dust_resolver::{resolve_annotation_ir_in_namespace, resolve_library_in_namespace};

#[test]
fn server_get_never_becomes_a_dio_client_verb_in_either_registration_order() {
    let mut graph = graph();
    let source = format!(
        "import '{CLIENT}' as client; import '{SERVER}' as server; class Api {{ @client.GET('/a') void a() {{}} @server.GET('/b') void b() {{}} }}"
    );
    add(&mut graph, 3, APP, &source);
    let parsed = parse(3, &source);
    for reverse in [false, true] {
        let catalog = catalog(reverse);
        let result = resolve_library_in_namespace(
            FileId::new(3),
            "main.dart",
            "main.g.dart",
            &parsed,
            &catalog,
            graph.scope(APP),
            &["dust_dart::GET", "dust_server::GET"],
        );
        assert_eq!(result.diagnostics, []);
        let methods = &result.library.classes[0].methods;
        assert_eq!(
            methods
                .iter()
                .map(|method| (&method.configs[0].symbol.0, &method.configs[0].normalized))
                .collect::<Vec<_>>(),
            [
                (
                    &"dust_dart::GET".to_owned(),
                    &Some(NormalizedConfigIr::Http(HttpConfigIr::Verb {
                        verb: HttpVerbIr::Get,
                        path: "/a".to_owned()
                    }))
                ),
                (&"dust_server::GET".to_owned(), &None),
            ]
        );
        for (index, id) in ["dust_dart::GET", "dust_server::GET"].iter().enumerate() {
            let annotation = &parsed.classes[0].methods[index].annotations[0];
            assert_eq!(methods[index].surface.annotations[0], *annotation);
            let mut diagnostics = Vec::new();
            let ir = resolve_annotation_ir_in_namespace(
                FileId::new(3),
                annotation,
                &catalog,
                graph.scope(APP),
                &mut diagnostics,
            );
            assert_eq!(diagnostics, []);
            assert_eq!(ir.resolved_symbol, Some(SymbolId::new(*id)));
            assert_eq!(ir.raw_name, annotation.qualified_name);
            assert_eq!(ir.span.range, annotation.span);
            assert_eq!(ir.prefix, annotation.prefix);
        }
    }
}

#[test]
fn foreign_and_hidden_annotations_cannot_reach_normalization() {
    for (source, failure) in [
        (
            format!("import '{CLIENT}' as p hide GET; class Api {{ @p.GET('/a') void a() {{}} }}"),
            true,
        ),
        (
            "class GET { const GET(String path); } class Api { @GET('/a') void a() {} }".to_owned(),
            false,
        ),
    ] {
        let mut graph = graph();
        add(&mut graph, 3, APP, &source);
        let result = resolve_library_in_namespace(
            FileId::new(3),
            "main.dart",
            "main.g.dart",
            &parse(3, &source),
            &catalog(false),
            graph.scope(APP),
            &[],
        );
        assert_eq!(
            result.library.classes.last().unwrap().methods[0].configs,
            []
        );
        assert_eq!(result.diagnostics.len(), usize::from(failure));
        if failure {
            assert_eq!(
                result.diagnostics[0].message,
                "annotation name `GET` is not visible in this namespace"
            );
            assert_eq!(result.diagnostics[0].labels[0].file_id, FileId::new(3));
        }
    }
}

#[test]
fn unknown_provenance_uses_unique_legacy_owner_but_never_overlapping_owners() {
    let mut graph = graph();
    let source = "const GET = unknown; @GET('/a') class Api {}";
    add(&mut graph, 3, APP, source);
    let parsed = parse(3, source);
    let annotation = &parsed.classes[0].annotations[0];
    let mut unique = SymbolCatalog::new();
    unique.register_config("GET", "legacy::GET");
    let mut diagnostics = Vec::new();
    let ir = resolve_annotation_ir_in_namespace(
        FileId::new(3),
        annotation,
        &unique,
        graph.scope(APP),
        &mut diagnostics,
    );
    assert_eq!(ir.resolved_symbol, Some(SymbolId::new("legacy::GET")));
    assert_eq!(diagnostics, []);
    let ir = resolve_annotation_ir_in_namespace(
        FileId::new(3),
        annotation,
        &catalog(false),
        graph.scope(APP),
        &mut diagnostics,
    );
    assert_eq!(ir.resolved_symbol, None);
    assert_eq!(
        diagnostics[0].message,
        "annotation `GET` has unknown provenance and multiple registered owners: dust_dart::GET, dust_server::GET"
    );
}

#[test]
fn derive_group_and_nested_member_bind_by_origin_without_discarding_prefixes() {
    let mut graph = graph();
    let base = "package:dust_dart/src/derive/base.dart";
    let traits = "package:traits/a.dart";
    add(
        &mut graph,
        3,
        base,
        "class Derive { const Derive(List<Object> traits); }",
    );
    add(
        &mut graph,
        4,
        traits,
        "class Validate { const Validate(); }",
    );
    let source = format!(
        "import '{base}' as d; import '{traits}' as t; part 'package:app/main.g.dart'; @d.Derive([t.Validate()]) @t.Validate() class Api {{ @t.Validate() final int x = 0; @t.Validate() Api(); @t.Validate() void a(@t.Validate() int x) {{}} }}"
    );
    add(&mut graph, 5, APP, &source);
    add(
        &mut graph,
        6,
        "package:app/main.g.dart",
        &format!("part of '{APP}';"),
    );
    let mut catalog = catalog(false);
    catalog.register_trait("Validate", "test::Validate");
    catalog.register_config("Validate", "test::Validate");
    catalog
        .register_annotation_origin(AnnotationOrigin::new("test::Validate", traits, "Validate"))
        .unwrap();
    let parsed = parse(5, &source);
    let result = resolve_library_in_namespace(
        FileId::new(5),
        "main.dart",
        "main.g.dart",
        &parsed,
        &catalog,
        graph.scope(APP),
        &[],
    );
    assert_eq!(result.diagnostics, []);
    let class = &result.library.classes[0];
    assert_eq!(
        class
            .traits
            .iter()
            .map(|application| application.symbol.0.as_str())
            .collect::<Vec<_>>(),
        ["test::Validate"]
    );
    for config in [
        &class.configs[0],
        &class.fields[0].configs[0],
        &class.constructors[0].configs[0],
        &class.methods[0].configs[0],
        &class.methods[0].params[0].configs[0],
    ] {
        assert_eq!(config.symbol, SymbolId::new("test::Validate"));
    }
}

#[test]
fn foreign_derive_is_not_an_intrinsic_and_wrong_nested_prefix_is_diagnosed() {
    let mut graph = graph();
    let source =
        "class Derive { const Derive(List<Object> x); } @Derive([missing.GET()]) class Api {}";
    add(&mut graph, 3, APP, source);
    let result = resolve_library_in_namespace(
        FileId::new(3),
        "main.dart",
        "main.g.dart",
        &parse(3, source),
        &catalog(false),
        graph.scope(APP),
        &[],
    );
    assert_eq!(result.diagnostics, []);
    assert_eq!(result.library.classes[1].traits, []);
    let base = "package:dust_dart/src/derive/base.dart";
    add(
        &mut graph,
        4,
        base,
        "class Derive { const Derive(List<Object> x); }",
    );
    let source = format!("import '{base}'; @Derive([missing.GET()]) class Api {{}}");
    let uri = "package:app/nested.dart";
    add(&mut graph, 5, uri, &source);
    let parsed = parse(5, &source);
    let result = resolve_library_in_namespace(
        FileId::new(5),
        "nested.dart",
        "nested.g.dart",
        &parsed,
        &catalog(false),
        graph.scope(uri),
        &[],
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        ["annotation prefix `missing` is not declared by an import",]
    );
}
