use dust_workspace::{SupportedAnnotations, discover_libraries};
use tempfile::tempdir;

use crate::support::write_file;

#[test]
fn validation_opt_in_discovers_dust_aware_libraries_without_markers() {
    let root = tempdir().unwrap();
    write_file(&root.path().join("pubspec.yaml"), "name: parity\n");
    for (name, source) in [
        ("api.dart", "export 'package:dust_dart/db.dart';"),
        ("queries.dart", "import 'api.dart'; void query() {}"),
        ("plain.dart", "class Plain {}"),
        ("queries.g.dart", "import 'api.dart'; // generated"),
    ] {
        write_file(&root.path().join("lib").join(name), source);
    }
    let annotations = SupportedAnnotations::new(["SqlxDatabase"]);
    assert_eq!(
        discover_libraries(root.path(), &annotations).unwrap(),
        vec![]
    );

    let annotations = annotations.with_unannotated_libraries();
    let discovered = discover_libraries(root.path(), &annotations).unwrap();
    assert_eq!(
        discovered
            .iter()
            .map(|library| library.source_path.strip_prefix(root.path()).unwrap())
            .collect::<Vec<_>>(),
        vec![
            std::path::Path::new("lib/api.dart"),
            std::path::Path::new("lib/queries.dart"),
        ]
    );
}
