use super::*;

#[test]
fn only_followed_aliases_request_their_declaring_imports_on_every_query() {
    let mut graph = graph();
    let alias = "package:alias/bridge.dart";
    add(
        &mut graph,
        3,
        alias,
        &format!("import '{CLIENT}' as p; typedef Pretty = p.GET; typedef Unused = Missing;"),
    );
    add(
        &mut graph,
        4,
        APP,
        &format!("import '{alias}'; class Foreign {{}}"),
    );
    assert_eq!(
        graph.scope(APP).bind("Foreign", &catalog(false)),
        NamespaceBinding::Foreign
    );
    assert!(graph.required_lexical_imports().is_empty());
    for _ in 0..2 {
        assert_eq!(graph.scope(APP).bind("Pretty", &catalog(false)), owned());
        assert_eq!(
            graph.required_lexical_imports(),
            std::collections::BTreeSet::from([alias.into()])
        );
        graph.clear_required_lexical_imports();
    }
}

#[test]
fn missing_and_builtin_alias_targets_still_emit_requests() {
    for (source, expected) in [
        ("typedef GET = Missing;", NamespaceBinding::Unknown),
        ("typedef GET = int;", NamespaceBinding::Foreign),
    ] {
        let mut graph = graph();
        add(&mut graph, 3, APP, source);
        assert_eq!(graph.scope(APP).bind("GET", &catalog(false)), expected);
        assert_eq!(
            graph.required_lexical_imports(),
            std::collections::BTreeSet::from([APP.into()])
        );
    }
}

#[test]
fn conditional_alias_worlds_request_imports_even_when_final_ownership_disagrees() {
    let mut graph = graph();
    let alias = "package:alias/bridge.dart";
    add(
        &mut graph,
        3,
        alias,
        &format!("import '{SERVER}' as p; typedef GET = p.GET;"),
    );
    add(
        &mut graph,
        4,
        APP,
        &format!("import '{CLIENT}' if (dart.library.io) '{alias}';"),
    );
    rejection(
        graph.scope(APP).bind("GET", &catalog(false)),
        Failure::Conditional,
    );
    assert_eq!(
        graph.required_lexical_imports(),
        std::collections::BTreeSet::from([alias.into()])
    );
}

#[test]
fn aliases_declared_in_parts_request_the_owning_library_imports() {
    let mut graph = graph();
    let part = "package:app/markers.dart";
    add(
        &mut graph,
        3,
        APP,
        &format!("import '{CLIENT}' as p; part '{part}';"),
    );
    add(
        &mut graph,
        4,
        part,
        &format!("part of '{APP}'; typedef Pretty = p.GET;"),
    );
    assert_eq!(graph.scope(APP).bind("Pretty", &catalog(false)), owned());
    assert_eq!(
        graph.required_lexical_imports(),
        std::collections::BTreeSet::from([APP.into()])
    );
}

#[test]
fn imported_declarations_can_shadow_a_followed_scalar_alias_target() {
    let mut graph = graph();
    let origin = "package:other/scalar.dart";
    add(&mut graph, 3, origin, "class int { const int(); }");
    add(
        &mut graph,
        4,
        APP,
        &format!("import '{origin}'; typedef GET = int;"),
    );
    let mut catalog = catalog(false);
    catalog.register_config("int", "test::ScalarMarker");
    catalog
        .register_annotation_origin(AnnotationOrigin::new("test::ScalarMarker", origin, "int"))
        .unwrap();
    assert_eq!(
        graph.scope(APP).bind("GET", &catalog),
        NamespaceBinding::Owned(dust_ir::SymbolId::new("test::ScalarMarker"))
    );
    assert_eq!(
        graph.required_lexical_imports(),
        std::collections::BTreeSet::from([APP.into()])
    );
}
