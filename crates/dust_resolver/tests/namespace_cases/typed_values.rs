//! Applied type names must never become annotation owners or import qualifiers.

use super::*;
use dust_ir::{AnnotationValueIr, SymbolId};
use dust_resolver::{resolve_annotation_ir_in_namespace, resolve_library_in_namespace};

const BASE: &str = "package:dust_dart/src/derive/base.dart";
const TRAIT: &str = "package:traits/validate.dart";

fn setup() -> (NamespaceGraph, SymbolCatalog) {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        BASE,
        "class Derive { const Derive(List<Object> values); }",
    );
    add(
        &mut graph,
        4,
        TRAIT,
        "class Validate<T> { const Validate(); }",
    );
    add(
        &mut graph,
        7,
        "package:app/model.g.dart",
        &format!("part of '{APP}';"),
    );
    let mut catalog = catalog(false);
    catalog.register_trait("Validate", "test::Validate");
    catalog
        .register_annotation_origin(AnnotationOrigin::new("test::Validate", TRAIT, "Validate"))
        .unwrap();
    (graph, catalog)
}

#[test]
fn typed_nested_traits_bind_actual_names_and_retain_raw_spelling_and_spans() {
    for (written, import, prefix) in [
        ("Validate<Object?>", "", None),
        (
            "t.Validate<List<Map<foreign.Marker, int?>>>",
            " as t",
            Some("t"),
        ),
        (
            "t /* owner . ( */ . Validate<void Function<Q>(Q)>",
            " as t",
            Some("t"),
        ),
        (
            "t // owner ( .\n . Validate<({int field})>",
            " as t",
            Some("t"),
        ),
    ] {
        let (mut graph, catalog) = setup();
        let call = format!("{written}()");
        let source = format!(
            "import '{BASE}'; import '{TRAIT}'{import}; part 'package:app/model.g.dart'; @Derive([{call}]) class Model {{}}"
        );
        add(&mut graph, 5, APP, &source);
        let parsed = parse(5, &source);
        let result = resolve_library_in_namespace(
            FileId::new(5),
            "model.dart",
            "model.g.dart",
            &parsed,
            &catalog,
            graph.scope(APP),
            &[],
        );
        assert_eq!(result.diagnostics, []);
        assert_eq!(
            result.library.classes[0]
                .traits
                .iter()
                .map(|t| &t.symbol)
                .collect::<Vec<_>>(),
            [&SymbolId::new("test::Validate")]
        );
        let mut diagnostics = Vec::new();
        let ir = resolve_annotation_ir_in_namespace(
            FileId::new(5),
            &parsed.classes[0].annotations[0],
            &catalog,
            graph.scope(APP),
            &mut diagnostics,
        );
        assert_eq!(diagnostics, []);
        let AnnotationValueIr::List(values) = &ir.positional_args[0] else {
            panic!("expected list")
        };
        let AnnotationValueIr::Constructor {
            name,
            positional_args,
            named_args,
        } = &values[0]
        else {
            panic!("expected constructor")
        };
        assert_eq!(
            (
                name.source.as_str(),
                name.short.as_str(),
                name.prefix.as_deref()
            ),
            (written, "Validate", prefix)
        );
        let start = source.find(&call).unwrap() as u32;
        assert_eq!(
            name.span.range,
            dust_text::TextRange::new(start, start + call.len() as u32)
        );
        assert_eq!(positional_args, &[]);
        assert_eq!(named_args.len(), 0);
    }
}

#[test]
fn typed_nested_traits_never_choose_an_ambiguous_owner() {
    for reverse in [false, true] {
        let (mut graph, _) = setup();
        let other = "package:other/validate.dart";
        add(
            &mut graph,
            6,
            other,
            "class Validate<T> { const Validate(); }",
        );
        let source = format!(
            "import '{BASE}'; import '{TRAIT}' as t; import '{other}' as t; @Derive([t.Validate<List<int>>()]) class Model {{}}"
        );
        add(&mut graph, 5, APP, &source);
        let mut catalog = catalog(false);
        let mut owners = [("test::Validate", TRAIT), ("other::Validate", other)];
        if reverse {
            owners.reverse();
        }
        for (id, uri) in owners {
            catalog.register_trait("Validate", id);
            catalog
                .register_annotation_origin(AnnotationOrigin::new(id, uri, "Validate"))
                .unwrap();
        }
        let result = resolve_library_in_namespace(
            FileId::new(5),
            "model.dart",
            "model.g.dart",
            &parse(5, &source),
            &catalog,
            graph.scope(APP),
            &[],
        );
        assert_eq!(result.library.classes[0].traits, []);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].message,
            "annotation `Validate` is ambiguous between distinct declarations"
        );
        let start = source.find("t.Validate<List<int>>()").unwrap() as u32;
        assert_eq!(
            result.diagnostics[0].labels[0].range,
            dust_text::TextRange::new(start, start + 23)
        );
    }
}

#[test]
fn typed_nested_traits_keep_visibility_and_foreign_ownership_boundaries() {
    for (import, expected) in [
        (
            format!("import '{TRAIT}' as t hide Validate;"),
            "annotation name `Validate` is not visible in this namespace",
        ),
        (
            format!("import '{TRAIT}' deferred as t;"),
            "metadata cannot use a deferred import",
        ),
        (
            "import 'package:foreign/validate.dart' as t;".into(),
            "unknown derive trait or config `t.Validate<int>`",
        ),
    ] {
        let (mut graph, catalog) = setup();
        add(
            &mut graph,
            6,
            "package:foreign/validate.dart",
            "class Validate<T> { const Validate(); }",
        );
        let source =
            format!("import '{BASE}'; {import} @Derive([t.Validate<int>()]) class Model {{}}");
        add(&mut graph, 5, APP, &source);
        let result = resolve_library_in_namespace(
            FileId::new(5),
            "model.dart",
            "model.g.dart",
            &parse(5, &source),
            &catalog,
            graph.scope(APP),
            &[],
        );
        assert_eq!(result.library.classes[0].traits, []);
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>(),
            [expected]
        );
    }
}
