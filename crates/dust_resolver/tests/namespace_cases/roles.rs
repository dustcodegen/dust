use super::*;
use dust_resolver::{resolve_annotation_ir_in_namespace, resolve_library_in_namespace};

#[test]
fn unknown_nested_alias_considers_only_eligible_trait_aliases() {
    let mut graph = graph();
    let base = "package:dust_dart/src/derive/base.dart";
    add(
        &mut graph,
        3,
        base,
        "class Derive { const Derive(List<Object> x); }",
    );
    let source = format!(
        "import '{base}'; part 'package:app/main.g.dart'; const GET = unknown; @Derive([GET()]) class Api {{}}"
    );
    add_without_lexical_facts(&mut graph, 4, APP, &source);
    add(
        &mut graph,
        5,
        "package:app/main.g.dart",
        &format!("part of '{APP}';"),
    );
    let mut catalog = catalog(false);
    // A trait role registered under another alias must not make GET eligible.
    catalog.register_trait("Other", "dust_dart::GET");
    catalog.register_trait("GET", "test::Trait");
    let result = resolve_library_in_namespace(
        FileId::new(4),
        "main.dart",
        "main.g.dart",
        &parse(4, &source),
        &catalog,
        graph.scope(APP),
        &[],
    );
    assert_eq!(result.diagnostics, []);
    assert_eq!(
        result.library.classes[0]
            .traits
            .iter()
            .map(|application| application.symbol.0.as_str())
            .collect::<Vec<_>>(),
        ["test::Trait"]
    );
}

#[test]
fn opaque_group_uses_intrinsic_only_when_its_owner_is_unique() {
    let mut graph = graph();
    let source = "const Derive = unknown; @Derive([]) class Api {}";
    add_without_lexical_facts(&mut graph, 3, APP, source);
    let parsed = parse(3, source);
    let annotation = &parsed.classes[0].annotations[0];
    let mut diagnostics = Vec::new();
    let catalog = catalog(false);
    let ir = resolve_annotation_ir_in_namespace(
        FileId::new(3),
        annotation,
        &catalog,
        graph.scope(APP),
        &mut diagnostics,
    );
    assert_eq!(diagnostics, []);
    assert_eq!(
        ir.resolved_symbol,
        Some(dust_ir::SymbolId::new("dust_dart::Derive"))
    );
    let mut catalog = catalog;
    catalog.register_config("Derive", "test::Derive");
    let ir = resolve_annotation_ir_in_namespace(
        FileId::new(3),
        annotation,
        &catalog,
        graph.scope(APP),
        &mut diagnostics,
    );
    assert_eq!(ir.resolved_symbol, None);
    assert_eq!(
        diagnostics[0].message,
        "annotation `Derive` has unknown provenance and multiple registered owners: dust_dart::Derive, test::Derive"
    );
}

#[test]
fn unknown_unregistered_marker_stays_unowned_without_a_warning() {
    let mut graph = graph();
    let source = "const Other = unknown; @Other() class Api {}";
    add_without_lexical_facts(&mut graph, 3, APP, source);
    let parsed = parse(3, source);
    let mut diagnostics = Vec::new();
    let ir = resolve_annotation_ir_in_namespace(
        FileId::new(3),
        &parsed.classes[0].annotations[0],
        &catalog(false),
        graph.scope(APP),
        &mut diagnostics,
    );
    assert_eq!(ir.resolved_symbol, None);
    assert_eq!(diagnostics, []);
}
