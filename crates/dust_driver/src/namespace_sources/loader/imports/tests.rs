use super::*;

#[test]
fn lexical_part_requests_promote_owner_imports_and_are_idempotent() {
    let root = tempfile::tempdir().unwrap();
    for (relative, source) in [
        (
            "package_config.json",
            r#"{"configVersion":2,"packages":[]}"#,
        ),
        ("sdk/version", "3.6.0"),
        (
            "sdk/lib/libraries.json",
            r#"{"vm":{"libraries":{"core":{"uri":"core/core.dart"}}}}"#,
        ),
        (
            "sdk/lib/core/core.dart",
            "library dart.core; class Object {}",
        ),
        ("lib/root.dart", "import 'bridge.dart';"),
        (
            "lib/bridge.dart",
            "import 'leaf.dart'; part 'markers.dart';",
        ),
        (
            "lib/markers.dart",
            "part of 'bridge.dart'; class Holder { static const marker = Foreign(); }",
        ),
        ("lib/leaf.dart", "class Foreign { const Foreign(); }"),
    ] {
        let file = root.path().join(relative);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, source).unwrap();
    }
    let index = SourceIndex::load(
        root.path(),
        &root.path().join("package_config.json"),
        Some(std::path::Path::new("sdk")),
    )
    .unwrap();
    let source = index.file(&root.path().join("lib/root.dart")).unwrap();
    let owner = index.resolve(&source, "bridge.dart").unwrap();
    let part = index.resolve(&source, "markers.dart").unwrap();
    let leaf = index.resolve(&source, "leaf.dart").unwrap();
    let potential = BTreeSet::new();
    let mut loader = Loader::load(&index, &[source], ".g.dart", &potential).unwrap();
    assert!(loader.units.contains_key(&part.uri));
    assert!(!loader.units.contains_key(&leaf.uri));
    assert!(!loader.imports.contains(&owner.uri));
    let required = BTreeSet::from([part.uri.clone()]);
    assert!(loader.require_lexical_imports(&required).unwrap());
    assert!(loader.imports.contains(&owner.uri));
    assert!(!loader.imports.contains(&part.uri));
    let leaf_unit = loader.units[&leaf.uri].as_ref().unwrap();
    assert_eq!(leaf_unit.parsed.classes[0].name, "Foreign");
    assert!(
        loader.units[&owner.uri]
            .as_ref()
            .unwrap()
            .dependencies
            .contains(&leaf.uri)
    );
    assert!(!loader.require_lexical_imports(&required).unwrap());
}
