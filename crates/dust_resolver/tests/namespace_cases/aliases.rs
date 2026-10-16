use super::*;

#[test]
fn transparent_aliases_preserve_owner_across_conditional_branches() {
    let mut graph = graph();
    let alias = "package:alias/a.dart";
    add(
        &mut graph,
        3,
        alias,
        &format!("import '{CLIENT}' as p; typedef GET = p.GET;"),
    );
    add(
        &mut graph,
        4,
        APP,
        &format!("import '{CLIENT}' if (dart.library.io) '{alias}';"),
    );
    assert_eq!(graph.scope(APP).bind("GET", &catalog(false)), owned());
}

#[test]
fn aliases_do_not_collapse_distinct_visible_declarations_before_conflict_checking() {
    let mut graph = graph();
    let alias = "package:alias/a.dart";
    add(
        &mut graph,
        3,
        alias,
        &format!("import '{CLIENT}' as p; typedef GET = p.GET;"),
    );
    add(
        &mut graph,
        4,
        APP,
        &format!("import '{CLIENT}'; import '{alias}';"),
    );
    rejection(
        graph.scope(APP).bind("GET", &catalog(false)),
        Failure::Ambiguous,
    );
}

#[test]
fn opaque_and_cyclic_aliases_are_unknown_not_proven_foreign() {
    for source in [
        "const GET = unknown;",
        "typedef GET = Missing;",
        "typedef GET = Other; typedef Other = GET;",
    ] {
        let mut graph = graph();
        add(&mut graph, 3, APP, source);
        assert_eq!(
            graph.scope(APP).bind("GET", &catalog(false)),
            NamespaceBinding::Unknown
        );
    }
}

#[test]
fn known_non_annotation_alias_shapes_and_functions_are_foreign() {
    for source in [
        "typedef GET = int;",
        "typedef GET = void Function();",
        "typedef GET = (int, String);",
        "typedef GET = Object?;",
        "void GET() {}",
    ] {
        let mut graph = graph();
        add(&mut graph, 3, APP, source);
        let mut legacy = SymbolCatalog::new();
        legacy.register_config("GET", "legacy::GET");
        assert_eq!(
            graph.scope(APP).bind("GET", &legacy),
            NamespaceBinding::Foreign
        );
    }
}

#[test]
fn missing_registration_provenance_remains_unknown_for_legacy_compatibility() {
    let mut graph = graph();
    add(&mut graph, 3, APP, "class GET { const GET(); }");
    let mut legacy = SymbolCatalog::new();
    legacy.register_config("GET", "legacy::GET");
    assert_eq!(
        graph.scope(APP).bind("GET", &legacy),
        NamespaceBinding::Unknown
    );
    assert_eq!(
        graph.scope(APP).bind("GET", &catalog(false)),
        NamespaceBinding::Foreign
    );
}

#[test]
fn static_member_provenance_is_not_invented_from_a_local_class_name() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        APP,
        "class p { static const GET = unknown; }",
    );
    assert_eq!(
        graph.scope(APP).bind("p.GET", &catalog(false)),
        NamespaceBinding::Unknown
    );
}

#[test]
fn local_declarations_can_shadow_scalar_type_spellings_in_transparent_aliases() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        APP,
        "class int { const int(); } typedef GET = int;",
    );
    let mut catalog = catalog(false);
    catalog.register_config("int", "test::ScalarMarker");
    catalog
        .register_annotation_origin(AnnotationOrigin::new("test::ScalarMarker", APP, "int"))
        .unwrap();
    assert_eq!(
        graph.scope(APP).bind("GET", &catalog),
        NamespaceBinding::Owned(dust_ir::SymbolId::new("test::ScalarMarker"))
    );
}
