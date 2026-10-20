use super::*;
use dust_resolver::resolve_library_in_namespace;

#[test]
fn missing_conditional_members_do_not_hide_a_concrete_failure() {
    for initializer in ["Foreign()", "true ? Foreign() : Foreign()"] {
        let mut graph = graph();
        add(&mut graph, 4, "package:left/a.dart", "class Holder {}");
        add(
            &mut graph,
            5,
            "package:right/a.dart",
            &format!(
                "class Holder {{ static const marker = {initializer}; }} class Foreign {{ const Foreign(); }}"
            ),
        );
        let source = "import 'package:left/a.dart' if (dart.library.io) 'package:right/a.dart'; class Api { @Holder.marker void call() {} }";
        add(&mut graph, 3, APP, source);
        let parsed = parse(3, source);
        let error = rejection(
            graph.scope(APP).bind_at(
                "Holder.marker",
                &catalog(false),
                dust_ir::SpanIr::new(
                    FileId::new(3),
                    parsed.classes[0].methods[0].annotations[0].span,
                ),
            ),
            if initializer == "Foreign()" {
                Failure::Conditional
            } else {
                Failure::MissingInput
            },
        );
        assert_eq!(
            error.message,
            if initializer == "Foreign()" {
                "annotation `Holder.marker` has target-dependent conditional ownership"
            } else {
                "opaque lexical annotation constant provenance is unsupported"
            }
        );
        assert_eq!(
            error
                .labels
                .iter()
                .map(|label| label.file_id)
                .collect::<Vec<_>>(),
            if initializer == "Foreign()" {
                vec![FileId::new(4), FileId::new(5)]
            } else {
                vec![FileId::new(5)]
            }
        );
    }
}

#[test]
fn conditional_class_constants_preserve_the_actual_failure_and_declaration_label() {
    for initializer in ["true ? Foreign() : Foreign()", "client.GET('/real')"] {
        let mut graph = graph();
        for (id, uri, initializer) in [
            (4, "package:left/a.dart", "Foreign()"),
            (5, "package:right/a.dart", initializer),
        ] {
            add(
                &mut graph,
                id,
                uri,
                &format!(
                    "import '{CLIENT}' as client; class Holder {{ static const marker = {initializer}; }} class Foreign {{ const Foreign(); }}"
                ),
            );
        }
        let source = "import 'package:left/a.dart' if (dart.library.io) 'package:right/a.dart'; class Api { @Holder.marker void call() {} }";
        add(&mut graph, 3, APP, source);
        let parsed = parse(3, source);
        let result = resolve_library_in_namespace(
            FileId::new(3),
            "main.dart",
            "main.g.dart",
            &parsed,
            &catalog(false),
            graph.scope(APP),
            &["dust_dart::GET"],
        );
        assert_eq!(result.library.classes[0].methods[0].configs, []);
        let [error] = result.diagnostics.as_slice() else {
            panic!("one ownership failure expected: {:?}", result.diagnostics)
        };
        assert_eq!(
            error.message,
            if initializer.starts_with("true") {
                "opaque lexical annotation constant provenance is unsupported"
            } else {
                "Dust annotation constants are unsupported; use a direct annotation with its arguments"
            }
        );
        assert_eq!(
            error
                .labels
                .iter()
                .map(|label| label.file_id)
                .collect::<Vec<_>>(),
            [FileId::new(3), FileId::new(5)]
        );
        assert_eq!(
            error.labels[0].range,
            parsed.classes[0].methods[0].annotations[0].span
        );
        assert_eq!(
            error.labels[1].message,
            "lexical declaration `marker` shadows imported names"
        );
    }
}
