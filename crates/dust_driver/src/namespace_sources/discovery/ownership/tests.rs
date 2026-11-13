use super::*;
use crate::namespace_sources::test_workspace;

#[test]
fn dynamic_binding_keeps_sorted_selection_and_exact_serial_ownership_errors() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    fs::write(root.join("pubspec.yaml"), "name: scheduling_test\n").unwrap();
    test_workspace::prepare(root, root, "scheduling_test");
    fs::create_dir_all(root.join("lib")).unwrap();
    let patterns = [
        "import 'package:dust_dart/derive.dart' as p; @p.Derive([p.Eq()]) class Model {}",
        "import 'package:missing/annotations.dart' as p; part 'foreign.dart'; @p.Derive([]) class Model {}",
        "import 'package:dust_dart/derive.dart' as p hide Derive; @p.Derive([]) class Model {}",
        "import 'package:dust_dart/derive.dart'; class Model {}",
        "part of 'owner.dart'; class Model {}",
        "import 'package:dust_dart/derive.dart'; @Unrelated() class Model {}",
        "import 'package:dust_dart/derive.dart'; class Model {",
        "import 'package:dust_dart/serde.dart' as p; @p.Serialize() class Model {}",
        "import 'package:dust_dart/derive.dart' as p; @p.Derive([p.Eq(), p.Debug()]) class Model { final int value = 1; }",
    ];
    for index in (0..36).rev() {
        fs::write(
            root.join(format!("lib/model_{index:02}.dart")),
            patterns[index % patterns.len()],
        )
        .unwrap();
    }
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    let mut workspace = dust_workspace::load_workspace(root).unwrap();
    let sources = NamespaceSources::load(
        root,
        &mut workspace,
        Some(&test_workspace::sdk_source_root(root)),
        &catalog,
        &registry,
        false,
        Some(1),
    )
    .unwrap();
    let aware = dust_aware_uris(&sources, &catalog)
        .into_iter()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    for include_unannotated in [false, true] {
        let expected = (0..36)
            .filter(|index| {
                index % patterns.len() != 4 && (include_unannotated || index % patterns.len() != 3)
            })
            .map(|index| {
                let source_path = root.join(format!("lib/model_{index:02}.dart"));
                SourceLibrary {
                    output_path: dust_workspace::primary_output_path(&source_path, ".out.dart"),
                    source_path,
                }
            })
            .collect::<Vec<_>>();
        for _ in 0..3 {
            let libraries = super::super::selection::libraries(
                &sources,
                &aware,
                ".out.dart",
                &catalog,
                include_unannotated,
            );
            assert_eq!(libraries, expected);
            let serial = libraries
                .iter()
                .filter_map(|library| {
                    library_errors(&sources, library, &catalog, ".out.dart")
                        .map(|errors| (library.source_path.clone(), errors))
                })
                .collect::<BTreeMap<_, _>>();
            assert!(!serial.is_empty());
            assert_eq!(
                collect_errors(&sources, &libraries, &catalog, ".out.dart"),
                serial
            );
        }
    }
}
