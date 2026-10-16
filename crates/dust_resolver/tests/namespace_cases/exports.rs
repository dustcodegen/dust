use super::*;

#[test]
fn cyclic_diamond_reexports_deduplicate_the_actual_declaration() {
    let mut graph = graph();
    let a = "package:barrel/a.dart";
    let b = "package:barrel/b.dart";
    let c = "package:barrel/c.dart";
    add(
        &mut graph,
        3,
        a,
        &format!("export '{CLIENT}' show GET; export '{b}';"),
    );
    add(&mut graph, 4, b, &format!("export '{c}';"));
    add(
        &mut graph,
        5,
        c,
        &format!("export '{a}'; export '{CLIENT}';"),
    );
    add(
        &mut graph,
        6,
        APP,
        &format!("import '{a}' as p; import '{c}' as p;"),
    );
    assert_eq!(graph.scope(APP).bind("p.GET", &catalog(false)), owned());
    // A second read exercises the memoized, cycle-aware export result.
    assert_eq!(graph.scope(APP).bind("p.GET", &catalog(true)), owned());
}

#[test]
fn export_filters_and_local_shadowing_do_not_leak_an_owner() {
    let mut graph = graph();
    let barrel = "package:barrel/a.dart";
    add(
        &mut graph,
        3,
        barrel,
        &format!("export '{CLIENT}' show GET, POST show POST;"),
    );
    add(&mut graph, 4, APP, &format!("import '{barrel}';"));
    rejection(
        graph.scope(APP).bind("GET", &catalog(false)),
        Failure::Hidden,
    );
    let shadow = "package:barrel/shadow.dart";
    add(
        &mut graph,
        5,
        shadow,
        &format!("export '{CLIENT}'; class GET {{ const GET(); }}"),
    );
    add(
        &mut graph,
        6,
        "package:app/shadow.dart",
        &format!("import '{shadow}';"),
    );
    assert_eq!(
        graph
            .scope("package:app/shadow.dart")
            .bind("GET", &catalog(false)),
        NamespaceBinding::Foreign
    );
}

#[test]
fn conditional_imports_and_exports_require_relevant_ownership_equivalence() {
    for directive in ["import", "export"] {
        for (alternative, expected) in [
            (CLIENT, None),
            (SERVER, Some(Failure::Conditional)),
            ("package:missing/m.dart", Some(Failure::MissingInput)),
        ] {
            let mut graph = graph();
            let barrel = "package:barrel/a.dart";
            let source = format!("{directive} '{CLIENT}' if (dart.library.io) '{alternative}';");
            let root = if directive == "import" { APP } else { barrel };
            add(&mut graph, 3, root, &source);
            if directive == "export" {
                add(&mut graph, 4, APP, &format!("import '{barrel}';"));
            }
            let binding = graph.scope(APP).bind("GET", &catalog(false));
            match expected {
                None => assert_eq!(binding, owned()),
                Some(kind) => {
                    rejection(binding, kind);
                }
            }
        }
    }
}

#[test]
fn unrelated_or_hidden_conditional_edges_do_not_poison_a_name() {
    let mut graph = graph();
    add(
        &mut graph,
        3,
        APP,
        &format!(
            "import '{CLIENT}'; import 'package:missing/a.dart' if (dart.library.io) 'package:missing/b.dart' as other; export 'package:missing/c.dart' hide GET;"
        ),
    );
    assert_eq!(graph.scope(APP).bind("GET", &catalog(false)), owned());
}

#[test]
fn hidden_private_declarations_and_empty_export_cycles_stay_absent() {
    let mut graph = graph();
    let a = "package:barrel/a.dart";
    let b = "package:barrel/b.dart";
    add(&mut graph, 3, a, &format!("export '{b}'; class _GET {{}}"));
    add(&mut graph, 4, b, &format!("export '{a}';"));
    add(&mut graph, 5, APP, &format!("import '{a}';"));
    for name in ["GET", "_GET"] {
        rejection(
            graph.scope(APP).bind(name, &catalog(false)),
            Failure::Hidden,
        );
    }
    assert_eq!(
        graph.scope(a).bind("_GET", &catalog(false)),
        NamespaceBinding::Foreign
    );
}

#[test]
fn nonconverging_broken_conditional_cycles_do_not_poison_independent_owners() {
    let mut graph = graph();
    let a = "package:barrel/a.dart";
    let c = "package:barrel/c.dart";
    let d = "package:barrel/d.dart";
    add(
        &mut graph,
        3,
        a,
        &format!("export '{d}'; part 'package:missing/a.dart';"),
    );
    add(
        &mut graph,
        4,
        c,
        &format!(
            "export '{c}' if (dart.library.io) '{CLIENT}'; export '{CLIENT}'; part 'package:missing/c.dart';"
        ),
    );
    add(
        &mut graph,
        5,
        d,
        &format!("export '{a}' if (dart.library.io) '{c}';"),
    );
    add(
        &mut graph,
        6,
        APP,
        &format!("import '{d}' as broken; import '{CLIENT}' as good;"),
    );
    assert_eq!(graph.scope(APP).bind("good.GET", &catalog(false)), owned());
    let diagnostic = rejection(
        graph.scope(APP).bind("broken.GET", &catalog(false)),
        Failure::MissingInput,
    );
    assert!(
        diagnostic
            .message
            .contains("conditional namespace alternatives did not stabilize in `package:barrel/")
    );
    assert_eq!(graph.scope(APP).bind("good.GET", &catalog(true)), owned());
}
