use super::*;
use crate::namespace_sources::{NamespaceSources, parser_cache, test_workspace};

#[test]
fn dynamic_roots_match_serial_facts_targets_and_diagnostic_ids_with_mixed_cache_inputs() {
    let (root, _, _, mut snapshots) =
        fixture("import 'target.dart'; class Model<T> { Model(this.value); final T value; }");
    let root = root.path();
    std::fs::write(root.join("pubspec.yaml"), "name: example\n").unwrap();
    test_workspace::set_language_version(root, "example", "3.12");
    let large = format!(
        "import 'target.dart'; class Large {{ {} }}",
        (0..160)
            .map(|n| format!("int method{n}(int value) => value + {n};"))
            .collect::<String>()
    );
    for (name, text) in [
        ("large.dart", large.as_str()),
        ("target.dart", "class Target {}"),
        ("changed.dart", "class Before {}"),
    ] {
        let path = root.join("lib").join(name);
        std::fs::write(&path, text).unwrap();
        snapshots.insert(path, Arc::from(text));
    }
    let index = SourceIndex::load(
        root,
        &root.join(".dart_tool/package_config.json"),
        Some(&root.join(".dust-test-sdk")),
        false,
    )
    .unwrap();
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let annotations = registry.all_supported_annotations();
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    let mut workspace = dust_workspace::load_workspace(root).unwrap();
    let namespaces = NamespaceSources::load(
        root,
        &mut workspace,
        Some(&root.join(".dust-test-sdk")),
        &catalog,
        &registry,
        false,
        Some(1),
    )
    .unwrap();
    parser_cache::save(&namespaces, root, &annotations).unwrap();
    let cache = Arc::new(parser_cache::Cache::load(root, &annotations));
    for (name, text) in [
        (
            "changed.dart",
            "export 'target.dart' show Target; class After { final int id = 1; }",
        ),
        (
            "new.dart",
            "import 'package:missing/absent.dart'; typedef New<T> = T Function(T);",
        ),
        ("broken.dart", "import 'target.dart'; class Broken {"),
        (
            "limited.dart",
            "class Limited { Object method() => .named(); }",
        ),
    ] {
        let path = root.join("lib").join(name);
        std::fs::write(&path, text).unwrap();
        snapshots.insert(path, Arc::from(text));
    }
    let roots = snapshots
        .keys()
        .rev()
        .map(|path| index.file(path).unwrap())
        .collect::<Vec<_>>();
    for source in &roots {
        let hit = cache.lookup(
            source,
            &snapshots[&source.path],
            SourceInput::options(source),
            true,
        );
        let name = source.path.file_name().unwrap().to_str().unwrap();
        assert_eq!(
            hit.is_some(),
            matches!(name, "model.dart" | "large.dart" | "target.dart"),
            "{name}"
        );
    }
    let prepare = |workers| {
        prepare_roots(
            &index,
            &roots,
            workers,
            &annotations,
            &snapshots,
            Some(Arc::clone(&cache)),
            || "planned",
        )
    };
    let (serial_plan, mut serial) = prepare(1);
    let (dynamic_plan, mut dynamic) = prepare(4);
    assert_eq!(serial_plan, dynamic_plan);
    assert_eq!(
        serial.uris().collect::<Vec<_>>(),
        dynamic.uris().collect::<Vec<_>>()
    );
    assert_eq!(dynamic.uris().count(), roots.len());
    let mut generation_only = false;
    let mut namespace_error = false;
    for (order, source) in roots.iter().enumerate() {
        let input = SourceInput::read(source, None, &Default::default()).unwrap();
        let id = FileId::new(31 + u32::try_from(order).unwrap());
        let (expected, expected_targets) = serial.take(source, Some(&input), id).unwrap();
        let (actual, actual_targets) = dynamic.take(source, Some(&input), id).unwrap();
        assert_eq!(actual, expected);
        assert_targets_eq(&actual_targets, &expected_targets);
        generation_only |=
            !actual.diagnostics.is_empty() && actual.namespace_diagnostics.is_empty();
        namespace_error |= !actual.namespace_diagnostics.is_empty();
        assert!(
            actual
                .diagnostics
                .iter()
                .chain(&actual.namespace_diagnostics)
                .flat_map(|diagnostic| &diagnostic.labels)
                .all(|label| label.file_id == id)
        );
    }
    assert!(generation_only && namespace_error);
    assert_eq!(serial.uris().count(), 0);
    assert_eq!(dynamic.uris().count(), 0);
}

fn assert_targets_eq(actual: &ResolvedTargets, expected: &ResolvedTargets) {
    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>()
    );
    for (reference, target) in actual {
        match (target, &expected[reference]) {
            (Ok(actual), Ok(expected)) => {
                assert_eq!(actual.uri, expected.uri);
                assert_eq!(actual.path, expected.path);
                assert_eq!(actual.language_version, expected.language_version);
            }
            (Err(actual), Err(expected)) => assert_eq!(actual, expected),
            _ => panic!("different resolution result for {reference}"),
        }
    }
}
