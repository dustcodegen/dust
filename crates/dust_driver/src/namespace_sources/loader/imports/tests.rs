use super::*;

fn fixture(root_source: &str) -> (tempfile::TempDir, SourceIndex, Source) {
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
        ("lib/root.dart", root_source),
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
        false,
    )
    .unwrap();
    let source = index.file(&root.path().join("lib/root.dart")).unwrap();
    (root, index, source)
}

#[test]
fn lexical_part_requests_promote_owner_imports_and_are_idempotent() {
    let (_root, index, source) = fixture("import 'bridge.dart';");
    let owner = index.resolve(&source, "bridge.dart").unwrap();
    let part = index.resolve(&source, "markers.dart").unwrap();
    let leaf = index.resolve(&source, "leaf.dart").unwrap();
    let potential = BTreeSet::new();
    let prepared = PreparedRoots::empty(std::slice::from_ref(&source), &[], &BTreeMap::new());
    let mut loader = Loader::load(&index, &[source], ".g.dart", &potential, 1, prepared).unwrap();
    assert!(loader.units.contains_key(&part.uri));
    assert!(!loader.units.contains_key(&leaf.uri));
    assert!(!loader.imports.contains(&owner.uri));
    let revision = loader.revision;
    let required = BTreeSet::from([part.uri.clone()]);
    assert!(!loader.imports_ready(&required));
    assert!(loader.require_lexical_imports(&required).unwrap());
    assert!(loader.imports_ready(&required));
    assert!(loader.revision > revision);
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

#[test]
fn unused_named_aliases_do_not_eagerly_load_their_imports() {
    let (root, index, source) = fixture("import 'bridge.dart';");
    fs::write(
        root.path().join("lib/bridge.dart"),
        "import 'leaf.dart'; typedef Unused = Foreign;",
    )
    .unwrap();
    let owner = index.resolve(&source, "bridge.dart").unwrap();
    let leaf = index.resolve(&source, "leaf.dart").unwrap();
    let potential = BTreeSet::new();
    let prepared = PreparedRoots::empty(std::slice::from_ref(&source), &[], &BTreeMap::new());
    let mut loader = Loader::load(&index, &[source], ".g.dart", &potential, 2, prepared).unwrap();
    assert!(loader.units.contains_key(&owner.uri));
    assert!(!loader.units.contains_key(&leaf.uri));
    let required = BTreeSet::from([owner.uri]);
    assert!(!loader.imports_ready(&required));
    assert!(loader.require_lexical_imports(&required).unwrap());
    assert!(loader.imports_ready(&required));
    assert!(loader.units.contains_key(&leaf.uri));
}

#[test]
fn promoting_already_loaded_imports_changes_edges_without_refreshing_source_facts() {
    let (_root, index, source) = fixture("import 'bridge.dart'; import 'leaf.dart';");
    let owner = index.resolve(&source, "bridge.dart").unwrap();
    let part = index.resolve(&source, "markers.dart").unwrap();
    let leaf = index.resolve(&source, "leaf.dart").unwrap();
    let potential = BTreeSet::new();
    let prepared = PreparedRoots::empty(std::slice::from_ref(&source), &[], &BTreeMap::new());
    let mut loader = Loader::load(&index, &[source], ".g.dart", &potential, 1, prepared).unwrap();
    let revision = loader.revision;
    let units = loader.units.len();
    assert!(
        !loader.units[&owner.uri]
            .as_ref()
            .unwrap()
            .dependencies
            .contains(&leaf.uri)
    );
    fs::remove_file(&leaf.path).unwrap();

    assert!(
        loader
            .require_lexical_imports(&BTreeSet::from([part.uri]))
            .unwrap()
    );
    assert_eq!(loader.revision, revision);
    assert_eq!(loader.units.len(), units);
    assert_eq!(
        loader.units[&leaf.uri].as_ref().unwrap().parsed.classes[0].name,
        "Foreign"
    );
    assert!(
        loader.units[&owner.uri]
            .as_ref()
            .unwrap()
            .dependencies
            .contains(&leaf.uri)
    );
}
