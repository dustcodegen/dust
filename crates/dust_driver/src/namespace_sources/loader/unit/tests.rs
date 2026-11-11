use super::*;

fn fixture() -> (tempfile::TempDir, SourceIndex, Source, Source) {
    let root = tempfile::tempdir().unwrap();
    for (path, text) in [
        (
            "package_config.json",
            r#"{"configVersion":2,"packages":[{"name":"owner","rootUri":".","packageUri":"lib/","languageVersion":"3.0"}]}"#,
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
        (
            "lib/model.dart",
            "import 'one.dart' if (dart.library.io) 'two.dart'; import 'bad:source'; part 'model.g.dart';",
        ),
        ("lib/one.dart", "class One {}"),
        ("lib/two.dart", "class Two {}"),
    ] {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let index = SourceIndex::load(
        root.path(),
        &root.path().join("package_config.json"),
        Some(std::path::Path::new("sdk")),
        false,
    )
    .unwrap();
    let base = index.file(&root.path().join("lib/base.dart")).unwrap();
    let owner = index.resolve(&base, "package:owner/model.dart").unwrap();
    let part = index.resolve(&owner, "model.g.dart").unwrap();
    (root, index, owner, part)
}

#[test]
fn prepared_directives_and_generated_owner_keep_exact_source_identities() {
    let (_root, index, owner, part) = fixture();
    let potential = BTreeSet::new();
    let prepared = PreparedRoots::empty(std::slice::from_ref(&owner), &[], &BTreeMap::new());
    let loader = Loader::load(
        &index,
        std::slice::from_ref(&owner),
        ".g.dart",
        &potential,
        4,
        prepared,
    )
    .unwrap();
    let root = loader.units[&owner.uri].as_ref().unwrap();
    assert_eq!(
        root.targets.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "bad:source",
            "dart:core",
            "model.g.dart",
            "one.dart",
            "two.dart"
        ]
    );
    assert!(root.targets["bad:source"].is_err());
    let generated = loader.units[&part.uri].as_ref().unwrap();
    let target = generated.targets[&owner.uri].as_ref().unwrap();
    assert_eq!(
        (&target.uri, &target.path, target.language_version),
        (&owner.uri, &owner.path, owner.language_version)
    );
    assert_eq!(generated.targets.len(), 1);
    assert!(
        matches!(&generated.parsed.directives[0], ParsedDirective::PartOf { uri: Some(uri), .. } if uri == &owner.uri)
    );
}

#[test]
fn generated_owner_resolution_falls_back_when_owner_facts_are_unavailable() {
    let (_root, index, owner, part) = fixture();
    let potential = BTreeSet::new();
    let prepared = PreparedRoots::empty(&[], &[], &BTreeMap::new());
    let loader = Loader::load(&index, &[], ".g.dart", &potential, 1, prepared).unwrap();
    let input = SourceInput {
        text: "".into(),
        owner: Some(owner.uri.clone()),
        generated_library: false,
    };
    let id = FileId::new(7);
    let parsed = input.parse(&part, id, &ParsePolicy::default());
    let unit = loader.resolve(part, id, input, parsed, None);
    let target = unit.targets[&owner.uri].as_ref().unwrap();
    assert_eq!(
        (&target.uri, &target.path, target.language_version),
        (&owner.uri, &owner.path, owner.language_version)
    );
    assert_eq!(unit.file_id, id);
    assert_eq!(unit.targets.len(), 1);
}
