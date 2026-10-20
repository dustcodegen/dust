#![cfg(test)]

use super::*;

const BASE: &str = "package:dust_dart/src/derive/base.dart";
const TRAIT: &str = "package:dust_dart/src/derive/traits.dart";

fn group_inputs(source: &str) -> (NamespaceGraph, SymbolCatalog, ParsedDartFileSurface) {
    let (mut graph, mut catalog, parsed) = inputs(source, false);
    insert(
        &mut graph,
        5,
        BASE,
        &parse(
            5,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../packages/dust_dart/lib/src/derive/base.dart"
            )),
        ),
    );
    insert(
        &mut graph,
        6,
        TRAIT,
        &parse(
            6,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../packages/dust_dart/lib/src/derive/traits.dart"
            )),
        ),
    );
    catalog.register_trait("ToString", "dust_dart::ToString");
    catalog
        .register_annotation_origin(AnnotationOrigin::new(
            "dust_dart::ToString",
            TRAIT,
            "ToString",
        ))
        .unwrap();
    (graph, catalog, parsed)
}

#[test]
fn standalone_derive_values_keep_prefixes_and_validate_their_owners() {
    let source = format!(
        "import '{BASE}' as d; import '{TRAIT}' as t; @d.Derive([t.ToString()]) void handler() {{}}"
    );
    let (graph, catalog, parsed) = group_inputs(&source);
    let (ir, diagnostics) = pipeline(&graph, &catalog, &parsed);
    assert_eq!(diagnostics, []);
    let annotation = &ir.unwrap().functions[0].annotations[0];
    assert_eq!(
        annotation.resolved_symbol,
        Some(SymbolId::new("dust_dart::Derive"))
    );
    let dust_ir::AnnotationValueIr::List(values) = &annotation.positional_args[0] else {
        panic!("list expected")
    };
    let dust_ir::AnnotationValueIr::Constructor { name, .. } = &values[0] else {
        panic!("constructor expected")
    };
    assert_eq!(name.source, "t.ToString");
    assert_eq!(name.prefix.as_deref(), Some("t"));
    assert_eq!(name.span.file_id, FileId::new(3));
    let bad = format!("import '{BASE}' as d; @d.Derive([missing.ToString()]) void handler() {{}}");
    let (graph, catalog, parsed) = group_inputs(&bad);
    let (ir, diagnostics) = pipeline(&graph, &catalog, &parsed);
    assert_eq!(ir, None);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "annotation prefix `missing` is not declared by an import"
    );
}

#[test]
fn function_parameters_retain_both_owner_roles_without_changing_mixin_field_semantics() {
    let source = "class Marker { const Marker(); } void handler(@Marker() int value) {} mixin M { @Marker() int value=0; }";
    let (graph, mut catalog, parsed) = inputs(source, false);
    catalog.register_trait("Marker", "test::Marker");
    catalog.register_config("Marker", "test::Marker");
    catalog
        .register_annotation_origin(AnnotationOrigin::new("test::Marker", APP, "Marker"))
        .unwrap();
    let (ir, diagnostics) = pipeline(&graph, &catalog, &parsed);
    assert_eq!(diagnostics, []);
    let ir = ir.unwrap();
    assert_eq!(
        ir.functions[0].params[0]
            .configs
            .iter()
            .map(|config| config.symbol.0.as_str())
            .collect::<Vec<_>>(),
        ["test::Marker"]
    );
    assert_eq!(ir.mixins[0].fields[0].configs, []);
    assert_eq!(ir.mixins[0].fields[0].serde, None);
}

#[test]
fn transparent_reexported_aliases_bind_function_metadata_and_parameter_traits() {
    let source = "import 'package:barrel/alias.dart' as alias; @alias.Pretty() void handler(@alias.Pretty() int value) {}";
    let (mut graph, catalog, parsed) = group_inputs(source);
    let alias = "package:barrel/alias.dart";
    let definition = "package:barrel/definition.dart";
    insert(
        &mut graph,
        7,
        alias,
        &parse(7, &format!("export '{definition}' show Pretty;")),
    );
    insert(
        &mut graph,
        8,
        definition,
        &parse(
            8,
            &format!("import '{TRAIT}' as t; typedef Pretty=t.ToString;"),
        ),
    );
    let (ir, diagnostics) = pipeline(&graph, &catalog, &parsed);
    assert_eq!(diagnostics, []);
    let ir = ir.unwrap();
    assert_eq!(
        ir.functions[0].annotations[0].resolved_symbol,
        Some(SymbolId::new("dust_dart::ToString"))
    );
    assert_eq!(ir.functions[0].annotations[0].raw_name, "alias.Pretty");
    assert_eq!(
        ir.functions[0].params[0]
            .traits
            .iter()
            .map(|value| value.symbol.0.as_str())
            .collect::<Vec<_>>(),
        ["dust_dart::ToString"]
    );
    assert_eq!(ir.functions[0].params[0].configs, []);
}

#[test]
fn foreign_derive_does_not_bind_its_constructor_values_as_intrinsic_traits() {
    let source = "class Derive { const Derive(List<Object> values); } @Derive([missing.ToString()]) void handler() {}";
    let (graph, catalog, parsed) = group_inputs(source);
    let (ir, diagnostics) = pipeline(&graph, &catalog, &parsed);
    assert_eq!(diagnostics, []);
    assert_eq!(
        ir.unwrap().functions[0].annotations[0].resolved_symbol,
        None
    );
}

#[test]
fn ignored_group_roles_still_validate_nested_names_at_method_and_parameter_sites() {
    for declaration in [
        "void handler(@d.Derive([missing.ToString()]) int value) {}",
        "class C { void handler(@d.Derive([missing.ToString()]) int value) {} }",
        "class C { @d.Derive([missing.ToString()]) void handler() {} }",
        "class C { C(@d.Derive([missing.ToString()]) int value); }",
        "@d.Derive([missing.ToString()]) class C {}",
    ] {
        let source = format!("import '{BASE}' as d; {declaration}");
        let (graph, catalog, parsed) = group_inputs(&source);
        let (ir, diagnostics) = pipeline(&graph, &catalog, &parsed);
        assert_eq!(ir, None, "{declaration}");
        assert_eq!(diagnostics.len(), 1, "{declaration}");
        assert_eq!(
            diagnostics[0].message,
            "annotation prefix `missing` is not declared by an import"
        );
    }
}

#[test]
fn validated_groups_do_not_enable_derivation_at_method_or_parameter_sites() {
    let source = format!(
        "import '{BASE}' as d; import '{TRAIT}' as t;\n\
         void handler(@d.Derive([t.ToString()]) int value) {{}}\n\
         class C {{ @d.Derive([t.ToString()]) void method(@d.Derive([t.ToString()]) int value) {{}} }}"
    );
    let (graph, catalog, parsed) = group_inputs(&source);
    let (ir, diagnostics) = pipeline(&graph, &catalog, &parsed);
    assert_eq!(diagnostics, []);
    let ir = ir.unwrap();
    assert_eq!(ir.functions[0].params[0].traits, []);
    assert_eq!(ir.functions[0].params[0].configs, []);
    let method = &ir.classes[0].methods[0];
    assert_eq!(method.traits, []);
    assert_eq!(method.configs, []);
    assert_eq!(method.params[0].traits, []);
    assert_eq!(method.params[0].configs, []);
}

#[test]
fn outer_derive_trivia_preserves_intrinsic_binding_and_nested_trait_application() {
    let written = "d /* owner . ( */ . Derive";
    let source = format!(
        "import '{BASE}' as d; import '{TRAIT}' as t; part 'main.g.dart';
@{written}([t.ToString()]) class C {{}}
@{written}([t.ToString()]) void handler() {{}}"
    );
    let (graph, catalog, parsed) = group_inputs(&source);
    let (ir, diagnostics) = pipeline(&graph, &catalog, &parsed);
    assert_eq!(diagnostics, []);
    let ir = ir.unwrap();
    let start = source.find(&format!("@{written}")).unwrap() as u32;
    assert_eq!(
        ir.classes[0].traits,
        [dust_ir::TraitApplicationIr {
            symbol: SymbolId::new("dust_dart::ToString"),
            span: dust_ir::SpanIr::new(
                FileId::new(3),
                dust_text::TextRange::new(
                    start,
                    start + format!("@{written}([t.ToString()])").len() as u32
                )
            )
        }]
    );
    assert_eq!(ir.functions[0].annotations[0].raw_name, written);
    assert_eq!(
        ir.functions[0].annotations[0].resolved_symbol,
        Some(SymbolId::new("dust_dart::Derive"))
    );
}
