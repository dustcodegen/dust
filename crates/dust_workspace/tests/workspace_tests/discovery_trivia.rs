use dust_workspace::{SourceLibrary, SupportedAnnotations, discover_libraries};
use tempfile::tempdir;

use crate::support::write_file;

#[test]
fn discovery_keeps_files_with_a_single_qualified_annotation_across_trivia() {
    for written in [
        "d . ToString",
        "d /* . ( */ . ToString",
        "d . /* ) . */ ToString",
        "d // . (\n . ToString",
        "d /* outer /* nested */ end */ . ToString",
    ] {
        let root = tempdir().unwrap();
        write_file(&root.path().join("pubspec.yaml"), "name: app\n");
        write_file(
            &root.path().join("lib/model.dart"),
            &format!(
                "import 'package:dust_dart/derive.dart' as d; part 'model.g.dart'; @{written}() class Model {{}}"
            ),
        );
        assert_eq!(
            discover_libraries(root.path(), &SupportedAnnotations::new(["ToString"])).unwrap(),
            [SourceLibrary {
                source_path: root.path().join("lib/model.dart"),
                output_path: root.path().join("lib/model.g.dart")
            }],
            "{written}"
        );
    }
}

#[test]
fn discovery_finds_only_actual_constructor_calls_inside_qualified_group_arguments() {
    for (annotation, selected) in [
        (
            "@d /* marker */ . Derive([d /* value */ . FromRow()])",
            true,
        ),
        ("@d.Derive([d . FromRow /* args */ ()])", true),
        ("@d.Derive(['FromRow()'])", false),
        ("@d.Derive([/* FromRow() */ d.Other()])", false),
        ("@d.Derive([d.FromRow])", false),
        ("@d . /* unterminated FromRow()", false),
        ("@d . // no identifier", false),
        ("@d.Derive([ /* unterminated FromRow()", false),
        ("@d.Derive([d . /* unterminated FromRow()", false),
        ("@d . ", false),
    ] {
        let root = tempdir().unwrap();
        write_file(&root.path().join("pubspec.yaml"), "name: app\n");
        write_file(
            &root.path().join("lib/model.dart"),
            &format!(
                "import 'package:dust_dart/derive.dart' as d; part 'model.g.dart'; {annotation} class Model {{}}"
            ),
        );
        let discovered =
            discover_libraries(root.path(), &SupportedAnnotations::new(["FromRow"])).unwrap();
        assert_eq!(discovered.len(), usize::from(selected), "{annotation}");
    }
}
