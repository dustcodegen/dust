use super::*;

#[test]
fn uri_and_named_parts_inherit_imports_and_rebase_declaration_identity() {
    for header in [
        format!("part of '{APP}';"),
        "part of app.library;".to_owned(),
    ] {
        let mut graph = graph();
        let part = "package:app/part.dart";
        add(
            &mut graph,
            3,
            APP,
            &format!("library app.library; import '{CLIENT}' as p; part '{part}';"),
        );
        add(
            &mut graph,
            4,
            part,
            &format!("{header} typedef GetAlias = p.GET;"),
        );
        for uri in [APP, part] {
            assert_eq!(graph.scope(uri).bind("p.GET", &catalog(false)), owned());
            assert_eq!(graph.scope(uri).bind("GetAlias", &catalog(false)), owned());
        }
    }
}

#[test]
fn private_part_declarations_are_local_but_cannot_leak_through_an_import() {
    let mut graph = graph();
    let part = "package:app/part.dart";
    add(&mut graph, 3, APP, &format!("part '{part}';"));
    add(
        &mut graph,
        4,
        part,
        &format!("part of '{APP}'; class _GET {{}}"),
    );
    add(
        &mut graph,
        5,
        "package:other/main.dart",
        &format!("import '{APP}';"),
    );
    assert_eq!(
        graph.scope(APP).bind("_GET", &catalog(false)),
        NamespaceBinding::Foreign
    );
    rejection(
        graph
            .scope("package:other/main.dart")
            .bind("_GET", &catalog(false)),
        Failure::Hidden,
    );
}

#[test]
fn missing_or_invalid_parts_are_required_inputs_not_legacy_aliases() {
    let part = "package:app/part.dart";
    for source in [
        None,
        Some("class GET {}"),
        Some("part of 'package:wrong/main.dart';"),
    ] {
        let mut graph = graph();
        add(
            &mut graph,
            3,
            APP,
            &format!("import '{CLIENT}' as p; part '{part}';"),
        );
        if let Some(source) = source {
            add(&mut graph, 4, part, source);
        }
        let diagnostic = rejection(
            graph.scope(APP).bind("p.GET", &catalog(false)),
            Failure::MissingInput,
        );
        assert_eq!(diagnostic.labels[0].file_id, FileId::new(3));
    }
}

#[test]
fn invalid_part_library_directives_are_rejected_even_for_synthetic_facts() {
    let mut graph = graph();
    let part = "package:app/part.dart";
    add(
        &mut graph,
        3,
        APP,
        &format!("import '{CLIENT}' as p; part '{part}';"),
    );
    let mut parsed = parse(4, &format!("part of '{APP}';"));
    parsed
        .directives
        .extend(parse(4, "import 'package:other/o.dart';").directives);
    graph
        .insert_library(part, FileId::new(4), &parsed, |_, value| {
            Ok(value.to_owned())
        })
        .unwrap();
    let diagnostic = rejection(
        graph.scope(APP).bind("p.GET", &catalog(false)),
        Failure::MissingInput,
    );
    assert_eq!(
        diagnostic.message,
        format!("part `{part}` contains library-only directives")
    );
}

#[test]
fn orphan_parts_and_direct_imports_of_parts_fail_clearly() {
    let mut graph = graph();
    let part = "package:app/part.dart";
    add(
        &mut graph,
        3,
        part,
        &format!("part of '{APP}'; class GET {{}}"),
    );
    rejection(
        graph.scope(part).bind("GET", &catalog(false)),
        Failure::MissingInput,
    );
    add(&mut graph, 4, APP, &format!("import '{part}' as p;"));
    let diagnostic = rejection(
        graph.scope(APP).bind("p.GET", &catalog(false)),
        Failure::MissingInput,
    );
    assert_eq!(
        diagnostic.message,
        format!("namespace import/export targets part `{part}` instead of a library")
    );
}

#[test]
fn duplicate_parts_and_declarations_remain_real_namespace_conflicts() {
    let part = "package:app/part.dart";
    for repeated in [false, true] {
        let mut graph = graph();
        let suffix = if repeated {
            format!("part '{part}';")
        } else {
            "class GET {}".to_owned()
        };
        add(&mut graph, 3, APP, &format!("part '{part}'; {suffix}"));
        add(
            &mut graph,
            4,
            part,
            &format!("part of '{APP}'; class GET {{}}"),
        );
        rejection(
            graph.scope(APP).bind("GET", &catalog(false)),
            if repeated {
                Failure::MissingInput
            } else {
                Failure::Ambiguous
            },
        );
    }
}

#[test]
fn part_owner_uri_resolution_failures_keep_their_original_context() {
    let mut graph = graph();
    let part = "package:app/part.dart";
    add(
        &mut graph,
        3,
        APP,
        &format!("import '{CLIENT}' as p; part '{part}';"),
    );
    graph
        .insert_library(
            part,
            FileId::new(4),
            &parse(4, "part of 'unresolved.dart';"),
            |_, _| Err("owner URI could not be resolved".to_owned()),
        )
        .unwrap();
    for uri in [APP, part] {
        let diagnostic = rejection(
            graph.scope(uri).bind("p.GET", &catalog(false)),
            Failure::MissingInput,
        );
        assert_eq!(diagnostic.message, "owner URI could not be resolved");
        assert_eq!(diagnostic.labels[0].file_id, FileId::new(4));
        assert_eq!(
            diagnostic.labels[0].range,
            dust_text::TextRange::new(0_u32, 26_u32)
        );
    }
}

#[test]
fn unresolved_part_edges_cannot_be_bypassed_by_static_member_legacy_lookup() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        APP,
        "part 'unresolved.dart'; class p { static const GET = unknown; }",
    );
    let diagnostic = rejection(
        graph.scope(APP).bind("p.GET", &catalog(false)),
        Failure::MissingInput,
    );
    assert_eq!(
        diagnostic.message,
        "fixture has no URI mapping for `unresolved.dart`"
    );
}
