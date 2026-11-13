use super::*;

#[test]
fn conditional_owned_and_known_empty_namespaces_keep_distinct_ownership() {
    let empty = "package:empty/empty.dart";
    let barrel = "package:barrel/conditional.dart";
    for directive in ["import", "export"] {
        let mut graph = graph();
        add(&mut graph, 3, empty, "library empty;");
        let root = if directive == "import" { APP } else { barrel };
        add(
            &mut graph,
            4,
            root,
            &format!("{directive} '{CLIENT}' if (dart.library.io) '{empty}';"),
        );
        if directive == "export" {
            add(&mut graph, 5, APP, &format!("import '{barrel}';"));
        }
        rejection(
            graph.scope(APP).bind("GET", &catalog(false)),
            Failure::Conditional,
        );
    }
}

#[test]
fn deferred_known_empty_imports_still_reject_metadata() {
    let empty = "package:empty/empty.dart";
    let mut graph = graph();
    add(&mut graph, 3, empty, "library empty;");
    add(
        &mut graph,
        4,
        APP,
        &format!("import '{empty}' deferred as p;"),
    );
    let diagnostic = rejection(
        graph.scope(APP).bind("p.GET", &catalog(false)),
        Failure::Deferred,
    );
    assert_eq!(diagnostic.message, "metadata cannot use a deferred import");
}

#[test]
fn known_empty_leaves_never_replace_missing_parts_or_inputs() {
    for (target, expected) in [
        ("package:empty/empty.dart", Failure::Hidden),
        ("package:empty/missing.dart", Failure::MissingInput),
        ("package:empty/part.dart", Failure::MissingInput),
        ("package:empty/broken.dart", Failure::MissingInput),
    ] {
        let mut graph = graph();
        add(&mut graph, 3, "package:empty/empty.dart", "library empty;");
        add(
            &mut graph,
            4,
            "package:empty/part.dart",
            "part of 'package:empty/owner.dart';",
        );
        add(
            &mut graph,
            5,
            "package:empty/owner.dart",
            "part 'package:empty/part.dart';",
        );
        add(
            &mut graph,
            6,
            "package:empty/broken.dart",
            "part 'package:empty/missing_part.dart';",
        );
        add(&mut graph, 7, APP, &format!("import '{target}';"));
        rejection(graph.scope(APP).bind("GET", &catalog(false)), expected);
    }
}
