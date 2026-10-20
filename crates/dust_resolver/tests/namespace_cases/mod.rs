use super::*;
use dust_resolver::NamespaceFailureKind as Failure;

mod aliases;
mod exports;
mod inputs;
mod lexical_boundaries;
mod lexical_conditionals;
mod lexical_shadowing;
mod mixin_applications;
mod parts;
mod roles;
mod scoped;
mod surfaces;
mod typed_values;

#[test]
fn unprefixed_and_shared_prefix_conflicts_are_not_registration_order() {
    for prefix in ["", " as shared"] {
        let mut graph = graph();
        add(
            &mut graph,
            3,
            APP,
            &format!("import '{CLIENT}'{prefix}; import '{SERVER}'{prefix};"),
        );
        for reverse in [false, true] {
            let diagnostic = rejection(
                graph.scope(APP).bind(
                    if prefix.is_empty() {
                        "GET"
                    } else {
                        "shared.GET"
                    },
                    &catalog(reverse),
                ),
                Failure::Ambiguous,
            );
            assert_eq!(
                diagnostic.message,
                "annotation `GET` is ambiguous between distinct declarations"
            );
            assert_eq!(
                diagnostic
                    .labels
                    .iter()
                    .map(|label| label.file_id)
                    .collect::<Vec<_>>(),
                [FileId::new(1), FileId::new(2)]
            );
        }
    }
}

#[test]
fn ordered_visibility_and_shared_prefix_union_keep_only_real_candidates() {
    let cases = [
        (
            format!(
                "import '{CLIENT}' as p show GET, POST show GET; import '{SERVER}' as p hide GET;"
            ),
            "p.GET",
            owned(),
        ),
        (
            format!("import '{CLIENT}' show GET, POST show POST;"),
            "GET",
            NamespaceBinding::Rejected {
                kind: Failure::Hidden,
                diagnostic: dust_diagnostics::Diagnostic::error(
                    "annotation name `GET` is not visible in this namespace",
                ),
            },
        ),
        (
            format!("import '{CLIENT}' as p show GET hide GET;"),
            "p.GET",
            NamespaceBinding::Rejected {
                kind: Failure::Hidden,
                diagnostic: dust_diagnostics::Diagnostic::error(
                    "annotation name `GET` is not visible in this namespace",
                ),
            },
        ),
    ];
    for (source, name, expected) in cases {
        let mut graph = graph();
        add(&mut graph, 3, APP, &source);
        assert_eq!(graph.scope(APP).bind(name, &catalog(false)), expected);
    }
}

#[test]
fn locals_shadow_imports_and_missing_other_prefix_is_not_a_global_failure() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        APP,
        &format!(
            "import '{CLIENT}'; import 'package:absent/a.dart' as missing; class GET {{ const GET(); }}"
        ),
    );
    assert_eq!(
        graph.scope(APP).bind("GET", &catalog(false)),
        NamespaceBinding::Foreign
    );
    rejection(
        graph.scope(APP).bind("missing.GET", &catalog(false)),
        Failure::MissingInput,
    );
    rejection(
        graph.scope(APP).bind("undeclared.GET", &catalog(false)),
        Failure::UndeclaredPrefix,
    );
}

#[test]
fn prefixes_shadow_imported_names_but_cannot_also_be_local_declarations() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        APP,
        &format!("import '{CLIENT}'; import '{SERVER}' as GET;"),
    );
    rejection(
        graph.scope(APP).bind("GET", &catalog(false)),
        Failure::Hidden,
    );
    add(
        &mut graph,
        4,
        "package:app/conflict.dart",
        &format!("import '{CLIENT}' as p; class p {{}}"),
    );
    rejection(
        graph
            .scope("package:app/conflict.dart")
            .bind("p.GET", &catalog(false)),
        Failure::Ambiguous,
    );
}

#[test]
fn deferred_imports_cannot_be_recovered_as_normal_annotations() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        APP,
        &format!("import '{CLIENT}' deferred as p;"),
    );
    let diagnostic = rejection(
        graph.scope(APP).bind("p.GET", &catalog(false)),
        Failure::Deferred,
    );
    assert_eq!(diagnostic.message, "metadata cannot use a deferred import");
    assert_eq!(diagnostic.labels[0].file_id, FileId::new(3));
}

#[test]
fn system_declarations_lose_to_non_system_declarations() {
    let mut graph = graph();
    add(&mut graph, 3, "dart:fake", "class GET { const GET(); }");
    add(
        &mut graph,
        4,
        APP,
        &format!("import 'dart:fake'; import '{CLIENT}';"),
    );
    assert_eq!(graph.scope(APP).bind("GET", &catalog(false)), owned());
}
