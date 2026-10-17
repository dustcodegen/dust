use super::*;
use dust_resolver::resolve_library_in_namespace;

#[test]
fn named_mixin_application_classes_shadow_imported_annotation_spelling() {
    let mut graph = graph();
    let source = format!(
        "import '{CLIENT}'; class Base {{ const Base(String path); }} mixin Mix {{}} class GET = Base with Mix; class Api {{ @GET('/probe') void call() {{}} }}"
    );
    add(&mut graph, 3, APP, &source);
    let result = resolve_library_in_namespace(
        FileId::new(3),
        "main.dart",
        "main.g.dart",
        &parse(3, &source),
        &catalog(false),
        graph.scope(APP),
        &["dust_dart::GET"],
    );
    assert_eq!(result.diagnostics, []);
    let api = result
        .library
        .classes
        .iter()
        .find(|class| class.name == "Api")
        .unwrap();
    assert_eq!(api.methods[0].configs, []);
}

#[test]
fn explicit_annotation_owners_remain_distinct_beside_a_named_mixin_class() {
    let mut graph = graph();
    let source = format!(
        "import '{CLIENT}' as client; import '{SERVER}' as server; class Base {{ const Base(String path); }} mixin Mix {{}} class GET = Base with Mix;"
    );
    add(&mut graph, 3, APP, &source);
    for reverse in [false, true] {
        for (annotation, owner) in [
            ("client.GET", "dust_dart::GET"),
            ("server.GET", "dust_server::GET"),
        ] {
            assert_eq!(
                graph.scope(APP).bind(annotation, &catalog(reverse)),
                NamespaceBinding::Owned(dust_ir::SymbolId::new(owner))
            );
        }
    }
}

#[test]
fn imported_reexported_and_part_owned_named_mixins_keep_foreign_identity() {
    let definitions =
        "class Base { const Base(String path); } mixin Mix {} class GET<T> = Base with Mix;";
    for placement in ["import", "export", "part"] {
        let mut graph = graph();
        let (directive, annotation) = if placement == "part" {
            add(
                &mut graph,
                4,
                "package:app/markers.dart",
                &format!("part of '{APP}'; {definitions}"),
            );
            ("part 'package:app/markers.dart';".to_owned(), "GET")
        } else {
            add(
                &mut graph,
                4,
                "package:markers/definitions.dart",
                definitions,
            );
            let import = if placement == "export" {
                add(
                    &mut graph,
                    5,
                    "package:markers/barrel.dart",
                    "export 'package:markers/definitions.dart';",
                );
                "package:markers/barrel.dart"
            } else {
                "package:markers/definitions.dart"
            };
            (format!("import '{import}' as foreign;"), "foreign.GET")
        };
        let source = format!(
            "import '{CLIENT}'; {directive} class Api {{ @{annotation}('/probe') void call() {{}} }}"
        );
        add(&mut graph, 3, APP, &source);
        let parsed = parse(3, &source);
        let result = resolve_library_in_namespace(
            FileId::new(3),
            "main.dart",
            "main.g.dart",
            &parsed,
            &catalog(false),
            graph.scope(APP),
            &["dust_dart::GET"],
        );
        assert_eq!(result.diagnostics, [], "{placement}");
        assert_eq!(
            result.library.classes[0].methods[0].configs,
            [],
            "{placement}"
        );
        assert_eq!(
            graph.scope(APP).bind(annotation, &catalog(false)),
            NamespaceBinding::Foreign
        );
    }
}
