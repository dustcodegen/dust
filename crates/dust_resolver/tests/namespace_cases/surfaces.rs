use super::*;

#[test]
fn every_named_declaration_shape_contributes_and_unnamed_extensions_do_not() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        APP,
        "mixin Marker {} extension Named on String {} extension on int {} extension type Wrapped(int x) {} enum Kind { one }",
    );
    for name in ["Marker", "Named", "Wrapped", "Kind"] {
        assert_eq!(
            graph.scope(APP).bind(name, &catalog(false)),
            NamespaceBinding::Foreign
        );
    }
}

#[test]
fn unavailable_alias_type_facts_do_not_claim_foreign_provenance() {
    for unknown_shape in [false, true] {
        let mut graph = graph();
        let mut parsed = parse(3, "typedef GET = Object;");
        if unknown_shape {
            parsed.typedefs[0]
                .parsed_aliased_type
                .as_mut()
                .unwrap()
                .kind = dust_parser_dart::ParsedTypeKind::Unknown;
        } else {
            parsed.typedefs[0].parsed_aliased_type = None;
        }
        graph
            .insert_library(
                APP,
                FileId::new(3),
                &parsed,
                |_, value| Ok(value.to_owned()),
            )
            .unwrap();
        assert_eq!(
            graph.scope(APP).bind("GET", &catalog(false)),
            NamespaceBinding::Unknown
        );
    }
}

#[test]
fn unknown_scoped_trait_warnings_retain_the_full_member_name_and_span() {
    let mut graph = graph();
    let base = "package:dust_dart/src/derive/base.dart";
    add(
        &mut graph,
        3,
        base,
        "class Derive { const Derive(List<Object> x); }",
    );
    let source = format!(
        "import '{base}'; const missing = unknown; @Derive([missing.Other()]) class Api {{}}"
    );
    add(&mut graph, 4, APP, &source);
    let result = dust_resolver::resolve_library_in_namespace(
        FileId::new(4),
        "main.dart",
        "main.g.dart",
        &parse(4, &source),
        &catalog(false),
        graph.scope(APP),
        &[],
    );
    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert_eq!(
        diagnostic.message,
        "unknown derive trait or config `missing.Other`"
    );
    let start = source.find("missing.Other()").unwrap() as u32;
    assert_eq!(
        diagnostic.labels[0].range,
        dust_text::TextRange::new(start, start + 15)
    );
}

#[test]
fn unowned_annotations_on_config_only_sites_are_not_promoted() {
    let mut graph = graph();
    let source =
        "class Other { const Other(); } class Api { @Other() final int x = 0; @Other() Api(); }";
    add(&mut graph, 3, APP, source);
    let result = dust_resolver::resolve_library_in_namespace(
        FileId::new(3),
        "main.dart",
        "main.g.dart",
        &parse(3, source),
        &catalog(false),
        graph.scope(APP),
        &[],
    );
    assert_eq!(result.diagnostics, []);
    assert_eq!(result.library.classes[1].fields[0].configs, []);
    assert_eq!(result.library.classes[1].constructors[0].configs, []);
}
