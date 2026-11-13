use super::*;

fn build(fixture: &Fixture) -> crate::CommandResult {
    crate::run_build(crate::BuildRequest {
        cwd: fixture.root.path().to_path_buf(),
        sdk_source_root: Some(fixture.root.path().join(".dust-test-sdk")),
        jobs: Some(1),
        fail_fast: false,
        db: Default::default(),
    })
}

#[test]
fn parser_diagnostics_and_generated_placeholders_are_never_stored() {
    for text in [
        "class Broken {",
        "class Model { Object method() => .named(); }",
    ] {
        let fixture = Fixture::new(text);
        let parsed = fixture.fresh(FileId::new(91));
        assert!(!parsed.diagnostics.is_empty());
        assert!(fixture.lookup().is_none());
        assert!(!fixture.uris().contains(&fixture.source.uri));
    }
    let text = "part 'source.g.dart'; class Model {}";
    let fixture = Fixture::new(text);
    let generated = fixture.root.path().join("lib/source.g.dart");
    fs::write(&generated, dust_plugin_api::GENERATED_HEADER).unwrap();
    fixture.save();
    let generated_source = index(fixture.root.path()).file(&generated).unwrap();
    assert!(
        fixture
            .cache()
            .lookup(&generated_source, "", fixture.options, false)
            .is_none()
    );
    assert!(!fixture.uris().contains(&generated_source.uri));
}

#[test]
fn cached_directives_resolve_against_the_current_package_mapping() {
    let fixture = Fixture::new("import 'package:collection/collection.dart'; class Model {}");
    assert!(fixture.lookup().is_some());
    let before = namespaces(fixture.root.path());
    let original = before.units[&fixture.source.uri].as_ref().unwrap().targets["package:collection/collection.dart"].as_ref().unwrap().path.clone();
    let replacement = fixture.root.path().join("replacement/lib/collection.dart");
    fs::create_dir_all(replacement.parent().unwrap()).unwrap();
    fs::write(&replacement, "class Replacement {}\n").unwrap();
    let config = fixture.root.path().join(".dart_tool/package_config.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    value["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|package| package["name"] == "collection")
        .unwrap()["rootUri"] = "../replacement".into();
    fs::write(config, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        fixture.lookup().is_some(),
        "package mapping is not parser syntax"
    );
    let after = namespaces(fixture.root.path());
    let unit = after.units[&fixture.source.uri].as_ref().unwrap();
    let current = unit.targets["package:collection/collection.dart"]
        .as_ref()
        .unwrap();
    assert_ne!(current.path, original);
    assert_eq!(current.path, replacement);
}

#[test]
fn codegen_invalidation_reuses_parser_facts_but_still_analyzes_and_emits() {
    let text = "import 'package:dust_dart/derive.dart'; part 'source.g.dart'; @ToString() class Model { const Model(this.value); final int value; }";
    let fixture = Fixture::new(text);
    let first = build(&fixture);
    assert_eq!(first.diagnostics, []);
    assert!(fixture.lookup().is_some());
    let output = fixture.root.path().join("lib/source.g.dart");
    let expected = fs::read(&output).unwrap();
    let mut cache = dust_cache::WorkspaceCache::load(fixture.root.path()).unwrap();
    let original = cache
        .get(fixture.root.path(), &fixture.source.path)
        .unwrap()
        .clone();
    assert!(!original.analysis_snapshot.string_sets().is_empty());
    let mut stale = original.clone();
    stale.tool_hash ^= 1;
    cache.insert(fixture.root.path(), &fixture.source.path, stale);
    cache.flush().unwrap();
    let rebuilt = build(&fixture);
    assert_eq!(rebuilt.diagnostics, []);
    assert_eq!(rebuilt.cache.as_ref().unwrap().hits, 0);
    assert_eq!(rebuilt.cache.as_ref().unwrap().misses, 1);
    assert!(!rebuilt.build_artifacts[0].cached);
    assert_eq!(fs::read(output).unwrap(), expected);
    assert_eq!(
        dust_cache::WorkspaceCache::load(fixture.root.path())
            .unwrap()
            .get(fixture.root.path(), &fixture.source.path),
        Some(&original)
    );
}

#[test]
fn parser_hits_do_not_reuse_a_previous_graphs_semantic_diagnostic_ids() {
    let text = "import 'package:dust_dart/derive.dart'; @ToString() class Model { const Model(this.value); final int value; }";
    let fixture = Fixture::new(text);
    assert!(
        fixture.lookup().is_some(),
        "syntax is valid even though generation fails"
    );
    let first = build(&fixture);
    assert!(first.has_errors());
    let first_id = first
        .diagnostic_files
        .iter()
        .find(|file| file.path == fixture.source.path)
        .unwrap()
        .file_id;
    fs::write(
        fixture.root.path().join("lib/a_earlier.dart"),
        "class Earlier {}\n",
    )
    .unwrap();
    let cached = build(&fixture);
    assert!(cached.has_errors());
    let current_id = cached
        .diagnostic_files
        .iter()
        .find(|file| file.path == fixture.source.path)
        .unwrap()
        .file_id;
    assert_ne!(first_id, current_id);
    assert!(
        cached
            .diagnostics
            .iter()
            .flat_map(|diagnostic| &diagnostic.labels)
            .all(|label| label.file_id == current_id)
    );
    fs::remove_file(fixture.path()).unwrap();
    let fresh = build(&fixture);
    assert_eq!(cached.diagnostics, fresh.diagnostics);
    assert_eq!(cached.diagnostic_files, fresh.diagnostic_files);
}

#[test]
fn excluded_foreign_body_diagnostics_do_not_rewrite_a_complete_good_snapshot() {
    let text = "import 'package:collection/collection.dart'; import 'package:dust_dart/derive.dart'; part 'source.g.dart'; @ToString() class Model { const Model(this.value); final int value; }";
    let fixture = Fixture::new(text);
    let foreign = index(fixture.root.path())
        .resolve(&fixture.source, "package:collection/collection.dart")
        .unwrap();
    fs::write(
        &foreign.path,
        "// @dart=3.13\nclass Foreign { Object method() => .named(); }",
    )
    .unwrap();
    fixture.save();
    let before = namespaces(fixture.root.path());
    let unit = before.units[&foreign.uri].as_ref().unwrap();
    assert!(!unit.diagnostics.is_empty());
    assert!(unit.namespace_diagnostics.is_empty());
    let foreign_diagnostics = unit.diagnostics.clone();
    assert!(fixture.lookup().is_some());
    assert!(!fixture.uris().contains(&foreign.uri));

    let first = build(&fixture);
    assert_eq!(first.diagnostics, []);
    let output = fixture.root.path().join("lib/source.g.dart");
    let generated = fs::read(&output).unwrap();
    let snapshot = fs::read(fixture.path()).unwrap();
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(fixture.path()).unwrap();
        (metadata.dev(), metadata.ino())
    };
    let mut cache = dust_cache::WorkspaceCache::load(fixture.root.path()).unwrap();
    let mut entry = cache
        .get(fixture.root.path(), &fixture.source.path)
        .unwrap()
        .clone();
    entry.tool_hash ^= 1;
    cache.insert(fixture.root.path(), &fixture.source.path, entry);
    cache.flush().unwrap();

    let rebuilt = build(&fixture);
    assert_eq!(rebuilt.diagnostics, first.diagnostics);
    assert_eq!(rebuilt.cache.as_ref().unwrap().hits, 0);
    assert_eq!(rebuilt.cache.as_ref().unwrap().misses, 1);
    assert!(!rebuilt.build_artifacts[0].cached);
    assert_eq!(fs::read(output).unwrap(), generated);
    assert_eq!(fs::read(fixture.path()).unwrap(), snapshot);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(fixture.path()).unwrap();
        assert_eq!(
            (metadata.dev(), metadata.ino()),
            identity,
            "no atomic replacement occurred"
        );
    }
    let after = namespaces(fixture.root.path());
    let unit = after.units[&foreign.uri].as_ref().unwrap();
    assert_eq!(unit.diagnostics, foreign_diagnostics);
    assert!(unit.namespace_diagnostics.is_empty());
}
