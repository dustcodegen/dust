use super::*;
use dust_diagnostics::Diagnostic;
use dust_resolver::BindingContext;

#[test]
fn ownership_validation_matches_annotation_ir_diagnostics_and_import_requests() {
    const DERIVE: &str = "package:dust_dart/src/derive/base.dart";
    let cases = [
        (
            format!(
                "import '{CLIENT}' as p; @p.GET('/a', headers: {{'x': [1, true, null]}}) class Api {{}}"
            ),
            false,
            false,
        ),
        (
            "class Foreign { const Foreign(Object value); } @Foreign([1, 2]) class Api {}".into(),
            false,
            false,
        ),
        (
            format!("import '{CLIENT}'; import '{SERVER}'; @GET('/a') class Api {{}}"),
            true,
            false,
        ),
        ("@missing.GET('/a') class Api {}".into(), true, false),
        (
            format!("import '{DERIVE}'; @Derive([Missing()]) class Api {{}}"),
            true,
            false,
        ),
        (
            format!(
                "import '{DERIVE}'; class Foreign {{ const Foreign(); }} @Derive([Foreign()]) class Api {{}}"
            ),
            false,
            false,
        ),
        (
            format!(
                "import '{DERIVE}'; import '{CLIENT}'; import '{SERVER}'; @Derive([GET('/a')]) class Api {{}}"
            ),
            true,
            false,
        ),
        (
            format!(
                "import '{DERIVE}'; import '{CLIENT}' as p; @Derive([p.GET('/a')]) class Api {{}}"
            ),
            false,
            false,
        ),
        (
            format!("import '{CLIENT}' as p; typedef Marker = p.GET; @Marker('/a') class Api {{}}"),
            false,
            true,
        ),
    ];
    for (source, errors, requests) in cases {
        for scoped in [false, true] {
            let mut graph = graph();
            add(
                &mut graph,
                4,
                DERIVE,
                "class Derive { const Derive(List<Object> values); }",
            );
            add(&mut graph, 3, APP, &source);
            let parsed = parse(3, &source);
            let annotation = parsed
                .classes
                .iter()
                .flat_map(|class| &class.annotations)
                .next()
                .unwrap();
            let mut catalog = catalog(false);
            catalog.register_trait("GET", "dust_dart::GET");
            catalog.register_trait("GET", "dust_server::GET");
            let seed = vec![Diagnostic::warning("existing diagnostic")];
            let mut expected = seed.clone();
            {
                let bindings = BindingContext::new(&catalog, scoped.then(|| graph.scope(APP)));
                let _ = bindings.annotation_ir(FileId::new(3), annotation, &mut expected);
            }
            let expected_requests = graph.required_lexical_imports();
            assert_eq!(
                expected.iter().any(Diagnostic::is_error),
                scoped && errors,
                "{scoped}: {source}"
            );
            assert_eq!(
                !expected_requests.is_empty(),
                scoped && requests,
                "{scoped}: {source}"
            );
            graph.clear_required_lexical_imports();
            let mut actual = seed;
            {
                let bindings = BindingContext::new(&catalog, scoped.then(|| graph.scope(APP)));
                bindings.validate_annotation_ownership(FileId::new(3), annotation, &mut actual);
            }
            assert_eq!(actual, expected, "{scoped}: {source}");
            assert_eq!(
                graph.required_lexical_imports(),
                expected_requests,
                "{scoped}: {source}"
            );
        }
    }
}
