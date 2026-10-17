use super::*;
use dust_ir::SpanIr;
use dust_text::TextRange;

/// The metadata use, not the class declaration's outer span, selects its scope.
fn use_span(source: &str) -> SpanIr {
    let start = source.rfind('@').unwrap() as u32;
    SpanIr::new(FileId::new(3), TextRange::new(start, start + 1))
}

#[test]
fn qualified_shadows_and_unresolved_constant_types_cannot_use_import_fallback() {
    for (declarations, annotation, message) in [
        (
            "class Api { static const d = Foreign(); @d.GET void call() {} } class Foreign {}",
            "d.GET",
            "qualified access through a lexical value is unsupported",
        ),
        (
            "class Api { static const GET = Alias(); @GET void call() {} } typedef Alias = Missing;",
            "GET",
            "lexical annotation constant has unknown provenance; imported-name fallback is forbidden",
        ),
        (
            "class Api { static const GET = Alias.named(); @GET void call() {} } typedef Alias = Missing;",
            "GET",
            "lexical annotation constant has unknown provenance; imported-name fallback is forbidden",
        ),
        (
            "class Api { @Holder.GET void call() {} } class Holder {}",
            "Holder.GET",
            "annotation member `GET` is not declared by `Holder`",
        ),
        (
            "class Api { @Alias.GET void call() {} } typedef Alias = Other; typedef Other = Alias;",
            "Alias.GET",
            "cyclic qualifying type alias in annotation",
        ),
        (
            "class Api { @Alias.GET void call() {} } typedef Alias = void Function();",
            "Alias.GET",
            "qualified annotation value has no supported type-member provenance",
        ),
    ] {
        let mut graph = graph();
        let source = format!("import '{CLIENT}' as d; {declarations}");
        add(&mut graph, 3, APP, &source);
        let error = rejection(
            graph
                .scope(APP)
                .bind_at(annotation, &catalog(false), use_span(&source)),
            if annotation == "Holder.GET" {
                Failure::Hidden
            } else {
                Failure::MissingInput
            },
        );
        assert_eq!(error.message, message);
    }
}

#[test]
fn enum_values_and_imported_reexported_library_constants_are_foreign() {
    for (declarations, annotation) in [
        ("enum Kind { marker }", "Kind.marker"),
        ("class Marker {} const marker = Marker();", "p.marker"),
    ] {
        let mut graph = graph();
        add(&mut graph, 4, "package:markers/source.dart", declarations);
        add(
            &mut graph,
            5,
            "package:markers/barrel.dart",
            "export 'package:markers/source.dart';",
        );
        let source = format!(
            "import 'package:markers/barrel.dart'{}; class Api {{ @{annotation} void call() {{}} }}",
            if annotation.starts_with('p') {
                " as p"
            } else {
                ""
            }
        );
        add(&mut graph, 3, APP, &source);
        assert_eq!(
            graph
                .scope(APP)
                .bind_at(annotation, &catalog(false), use_span(&source)),
            NamespaceBinding::Foreign
        );
        assert_eq!(
            graph.required_lexical_imports().is_empty(),
            annotation == "Kind.marker"
        );
    }
}

#[test]
fn conditional_constant_values_require_target_independent_ownership() {
    for qualified in [false, true] {
        for right in ["Foreign()", "client.GET('/real')"] {
            let mut graph = graph();
            for (id, uri, value) in [
                (4, "package:left/a.dart", "Foreign()"),
                (5, "package:right/a.dart", right),
            ] {
                let constant = format!("const marker = {value};");
                let declarations = if qualified {
                    format!("class Holder {{ static {constant} }}")
                } else {
                    constant
                };
                add(
                    &mut graph,
                    id,
                    uri,
                    &format!("import '{CLIENT}' as client; {declarations} class Foreign {{}}"),
                );
            }
            let annotation = if qualified { "Holder.marker" } else { "marker" };
            let source = format!(
                "import 'package:left/a.dart' if (dart.library.io) 'package:right/a.dart'; class Api {{ @{annotation} void call() {{}} }}"
            );
            add(&mut graph, 3, APP, &source);
            let result = graph
                .scope(APP)
                .bind_at(annotation, &catalog(false), use_span(&source));
            if right == "Foreign()" {
                assert_eq!(result, NamespaceBinding::Foreign);
            } else {
                rejection(result, Failure::MissingInput);
            }
        }
    }
}

#[test]
fn malformed_scope_parents_or_missing_use_units_fail_clearly() {
    let source = "class Api { @GET void call() {} }";
    for parent in [1, usize::MAX] {
        let mut graph = graph();
        let mut parsed = parse(3, source);
        parsed.lexical_scopes[1].parent = Some(parent);
        graph
            .insert_library(APP, FileId::new(3), &parsed, |_, uri| Ok(uri.to_owned()))
            .unwrap();
        let error = rejection(
            graph
                .scope(APP)
                .bind_at("GET", &catalog(false), use_span(source)),
            Failure::MissingInput,
        );
        assert_eq!(error.message, "invalid lexical scope parent chain");
    }
    let error = rejection(
        graph()
            .scope(APP)
            .bind_at("GET", &catalog(false), use_span(source)),
        Failure::MissingInput,
    );
    assert_eq!(
        error.message,
        "metadata source is missing from the namespace graph"
    );
}

#[test]
fn conditional_known_constants_cannot_agree_with_an_actual_dust_owner() {
    let mut graph = graph();
    add(
        &mut graph,
        4,
        "package:foreign/a.dart",
        "const GET = Foreign(); class Foreign { const Foreign(); }",
    );
    let source = format!(
        "import 'package:foreign/a.dart' if (dart.library.io) '{CLIENT}'; class Api {{ @GET void call() {{}} }}"
    );
    add(&mut graph, 3, APP, &source);
    let error = rejection(
        graph
            .scope(APP)
            .bind_at("GET", &catalog(false), use_span(&source)),
        Failure::Conditional,
    );
    assert_eq!(
        error.message,
        "annotation `GET` has target-dependent conditional ownership"
    );
    assert_eq!(error.labels.len(), 2);
}

#[test]
fn qualifying_types_preserve_missing_inputs_and_ambiguity() {
    for (source, expected) in [
        (
            "import 'package:missing/a.dart' as p; class Api { @p.Holder.marker void call() {} }"
                .to_owned(),
            Failure::MissingInput,
        ),
        (
            format!(
                "import '{CLIENT}'; import '{SERVER}'; class Api {{ @GET.marker void call() {{}} }}"
            ),
            Failure::Ambiguous,
        ),
    ] {
        let mut graph = graph();
        add(&mut graph, 3, APP, &source);
        let annotation = if source.contains("p.Holder") {
            "p.Holder.marker"
        } else {
            "GET.marker"
        };
        rejection(
            graph
                .scope(APP)
                .bind_at(annotation, &catalog(false), use_span(&source)),
            expected,
        );
    }
}

#[test]
fn incomplete_library_type_aliases_retain_the_explicit_legacy_unknown_contract() {
    let mut graph = graph();
    let source = "typedef Holder = Missing; class Api { @Holder.marker void call() {} }";
    add(&mut graph, 3, APP, source);
    assert_eq!(
        graph
            .scope(APP)
            .bind_at("Holder.marker", &catalog(false), use_span(source)),
        NamespaceBinding::Unknown
    );
    // No class/member shadow was proved; this is the pre-existing partial-graph adapter.
    assert_eq!(
        graph.scope(APP).bind("Holder.marker", &catalog(false)),
        NamespaceBinding::Unknown
    );
}

#[test]
fn missing_owner_namespace_is_not_rescued_by_a_present_metadata_source() {
    let mut graph = graph();
    let source = "class Api { @Holder.marker void call() {} }";
    add(&mut graph, 3, APP, source);
    let error = rejection(
        graph.scope("package:missing/root.dart").bind_at(
            "Holder.marker",
            &catalog(false),
            use_span(source),
        ),
        Failure::MissingInput,
    );
    assert_eq!(
        error.message,
        "missing required namespace input `package:missing/root.dart`"
    );
}
