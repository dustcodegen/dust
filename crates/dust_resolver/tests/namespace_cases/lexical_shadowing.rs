use super::*;
use dust_resolver::resolve_library_in_namespace;

#[test]
fn class_constant_metadata_cannot_become_an_imported_client_verb() {
    let mut graph = graph();
    let source = format!(
        "import '{CLIENT}'; class Api {{ static const GET = ForeignMarker(); @GET void call() {{}} }} class ForeignMarker {{ const ForeignMarker(); }}"
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
    assert_eq!(result.library.classes[0].methods[0].configs, []);
}

#[test]
fn lexical_constant_aliases_and_class_qualified_references_keep_foreign_identity() {
    for value in [
        "static const GET = ForeignMarker(), alias = GET; @alias",
        "static const GET = ForeignMarker(); @Api.GET",
        "static const GET = top; @GET",
    ] {
        let mut graph = graph();
        let source = format!(
            "import '{CLIENT}'; const top = ForeignMarker(); class Api {{ {value} void call() {{}} }} class ForeignMarker {{ const ForeignMarker(); }}"
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
        assert_eq!(result.diagnostics, [], "{value}");
        assert_eq!(result.library.classes[0].methods[0].configs, []);
    }
}

#[test]
fn unknown_cyclic_owned_and_type_parameter_shadows_fail_without_import_fallback() {
    for (header, body, expected) in [
        (
            "Api",
            "static const GET = true ? 1 : 2;",
            "opaque lexical annotation constant provenance is unsupported",
        ),
        (
            "Api",
            "static const GET = alias, alias = GET;",
            "cyclic lexical annotation constant",
        ),
        (
            "Api",
            "static const GET = client.GET('/real');",
            "Dust annotation constants are unsupported; use a direct annotation with its arguments",
        ),
        (
            "Api<GET>",
            "",
            "lexical annotation value has no supported constant provenance",
        ),
    ] {
        let mut graph = graph();
        let source = format!(
            "import '{CLIENT}'; import '{CLIENT}' as client; class {header} {{ {body} @GET void call() {{}} }}"
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
        assert_eq!(result.library.classes[0].methods[0].configs, []);
        assert_eq!(
            result
                .diagnostics
                .iter()
                .map(|error| error.message.as_str())
                .collect::<Vec<_>>(),
            [expected]
        );
        assert_eq!(result.diagnostics[0].labels.len(), 2);
        assert_eq!(result.diagnostics[0].labels[0].file_id, FileId::new(3));
        assert_eq!(result.diagnostics[0].labels[1].file_id, FileId::new(3));
    }
}

#[test]
fn named_foreign_constructors_and_type_alias_qualifiers_keep_their_actual_declaration() {
    for source in [
        format!(
            "import '{CLIENT}'; class Api {{ static const GET = ForeignMarker.named(); @GET void call() {{}} }} class ForeignMarker {{ const ForeignMarker.named(); }}"
        ),
        format!(
            "import '{CLIENT}'; class Api {{ @Alias.GET void call() {{}} }} typedef Alias = Holder; class Holder {{ static const GET = ForeignMarker(); }} class ForeignMarker {{ const ForeignMarker(); }}"
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
            &["dust_dart::GET"],
        );
        assert_eq!(result.diagnostics, []);
        assert_eq!(result.library.classes[0].methods[0].configs, []);
    }
}

#[test]
fn retained_library_constant_facts_are_not_missing_provenance_legacy_aliases() {
    for declarations in [
        "const GET = ForeignMarker();",
        "const foreign = ForeignMarker(); const GET = foreign;",
    ] {
        let mut graph = graph();
        let source = format!(
            "import '{CLIENT}'; {declarations} class Api {{ @GET void call() {{}} }} class ForeignMarker {{ const ForeignMarker(); }}"
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
        assert_eq!(result.library.classes[0].methods[0].configs, []);
    }
}

#[test]
fn known_constant_initializer_failures_do_not_use_legacy_aliases() {
    let mut graph = graph();
    let source =
        format!("import '{CLIENT}'; const GET = unknown; class Api {{ @GET void call() {{}} }}");
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
    assert_eq!(result.library.classes[0].methods[0].configs, []);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|error| error.message.as_str())
            .collect::<Vec<_>>(),
        ["annotation name `unknown` is not visible in this namespace"]
    );
    // The explicit library-only compatibility operation still lacks value resolution.
    assert_eq!(
        graph.scope(APP).bind("GET", &catalog(false)),
        NamespaceBinding::Unknown
    );
}
