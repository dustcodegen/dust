use std::{collections::BTreeMap, fs};

use tempfile::tempdir;

use super::{input, manifest::Manifest};

/// Prior output digests remain usable across general source and tool changes.
mod reuse;

/// Corruption cannot silently erase an observed foreign source.
mod incomplete;

/// Discovery provenance survives filesystem changes before proof persistence.
#[cfg(unix)]
mod provenance;

#[test]
fn exact_contents_detect_same_length_changes_and_missing_files() {
    let root = tempdir().unwrap();
    let path = root.path().join("input.dart");
    fs::write(&path, "class A {}").unwrap();
    let files = BTreeMap::from([(path.clone(), input::read_hash(&path).unwrap())]);
    assert!(input::matches(&files));
    fs::write(&path, "class B {}").unwrap();
    assert!(!input::matches(&files));
    fs::remove_file(&path).unwrap();
    assert!(!input::matches(&files));
    let missing = BTreeMap::from([(path.clone(), None)]);
    assert!(input::matches(&missing));
    fs::write(&path, "").unwrap();
    assert!(!input::matches(&missing));
}

#[test]
fn malformed_proofs_and_unmarked_generated_inputs_are_not_reused() {
    let root = tempdir().unwrap();
    let directory = root.path().join(".dart_tool/dust");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("unchanged_build_v1.json"), "{broken").unwrap();
    assert!(Manifest::load(root.path()).is_none());
    let path = root.path().join("output.dart");
    assert_eq!(input::generated_hash(&path), Some(None));
    fs::write(&path, "class Handwritten {}").unwrap();
    assert_eq!(input::generated_hash(&path), None);
    fs::write(
        &path,
        format!(
            "{}\nclass Generated {{}}",
            dust_plugin_api::GENERATED_HEADER
        ),
    )
    .unwrap();
    assert_eq!(
        input::generated_hash(&path),
        Some(input::read_hash(&path).unwrap())
    );
}

#[test]
fn dart_file_set_includes_generated_and_foreign_files() {
    let root = tempdir().unwrap();
    fs::create_dir_all(root.path().join("nested")).unwrap();
    fs::write(root.path().join("source.dart"), "").unwrap();
    fs::write(root.path().join("nested/source.g.dart"), "").unwrap();
    fs::write(root.path().join("notes.md"), "").unwrap();
    let files = input::lib_files(root.path()).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.contains(&root.path().join("nested/source.g.dart")));
    fs::write(root.path().join("foreign.dart"), "").unwrap();
    assert_ne!(files, input::lib_files(root.path()).unwrap());
}

use crate::namespace_sources::test_workspace as namespace_fixture;

fn workspace() -> (tempfile::TempDir, crate::BuildRequest) {
    let root = tempdir().unwrap();
    fs::write(root.path().join("pubspec.yaml"), "name: proof_test\n").unwrap();
    namespace_fixture::prepare(root.path(), root.path(), "proof_test");
    namespace_fixture::set_language_version(root.path(), "proof_test", "3.13");
    fs::create_dir_all(root.path().join("lib")).unwrap();
    fs::write(root.path().join("lib/item.dart"),
        "import 'package:dust_dart/derive.dart'; part 'item.g.dart'; @Derive([Eq()]) class Item with _$Item { const Item({required this.id}); final int id; }").unwrap();
    fs::write(
        root.path().join("lib/analysis.dart"),
        "import 'package:dust_dart/derive.dart'; @Deprecated('legacy') class Analysis {}",
    )
    .unwrap();
    let request = crate::BuildRequest {
        cwd: root.path().to_path_buf(),
        sdk_source_root: Some(namespace_fixture::sdk_source_root(root.path())),
        fail_fast: false,
        jobs: Some(1),
        db: Default::default(),
    };
    (root, request)
}

#[test]
fn complete_build_proof_reuses_outputs_and_rejects_every_changed_input_kind() {
    let (root, request) = workspace();
    let cold = crate::run_build(request.clone());
    assert_eq!(cold.diagnostics, []);
    assert!(Manifest::load(root.path()).is_some());
    let events = std::sync::Mutex::new(Vec::new());
    let progress = |event| events.lock().unwrap().push(event);
    let warm = super::try_build(&request, Some(&progress)).expect("complete successful proof");
    assert_eq!(events.lock().unwrap().len(), warm.build_artifacts.len() + 1);
    assert!(events.lock().unwrap().iter().skip(1).all(|event| matches!(
        event,
        crate::ProgressEvent::FinishedLibrary {
            cached: true,
            written: false,
            changed: false,
            ..
        }
    )));
    assert_eq!(warm.build_artifacts.len(), cold.build_artifacts.len());
    assert!(
        warm.build_artifacts
            .iter()
            .all(|artifact| artifact.cached && !artifact.changed && !artifact.written)
    );
    for relative in [
        "lib/item.dart",
        "lib/item.g.dart",
        "deps/dust_dart/lib/src/derive/traits.dart",
        ".dust-test-sdk/version",
        "deps/dust_dart/pubspec.yaml",
        "pubspec.yaml",
        ".dart_tool/package_config.json",
        ".dart_tool/dust/build_cache_v1.json",
    ] {
        let path = root.path().join(relative);
        let before = fs::read(&path).unwrap();
        let mut changed = before.clone();
        changed.extend_from_slice(b" ");
        fs::write(&path, changed).unwrap();
        assert!(super::try_build(&request, None).is_none(), "{relative}");
        fs::write(&path, before).unwrap();
    }
    for relative in ["lib/new.dart", "dust.yaml"] {
        let path = root.path().join(relative);
        fs::write(&path, "").unwrap();
        assert!(super::try_build(&request, None).is_none(), "{relative}");
        fs::remove_file(path).unwrap();
    }
    let proof_path = root.path().join(".dart_tool/dust/unchanged_build_v1.json");
    let proof = fs::read(&proof_path).unwrap();
    let mut incomplete = Manifest::load(root.path()).unwrap();
    incomplete.files.remove(&root.path().join("lib/item.dart"));
    incomplete.save(root.path());
    assert!(super::try_build(&request, None).is_none());
    fs::write(proof_path, proof).unwrap();
    let cache_path = root.path().join(".dart_tool/dust/build_cache_v1.json");
    let cache = fs::read(&cache_path).unwrap();
    let mut invalidated: serde_json::Value = serde_json::from_slice(&cache).unwrap();
    for entry in invalidated["entries"].as_object_mut().unwrap().values_mut() {
        entry["tool_hash"] = 0.into();
    }
    fs::write(&cache_path, serde_json::to_vec(&invalidated).unwrap()).unwrap();
    assert!(super::try_build(&request, None).is_none());
    fs::write(cache_path, cache).unwrap();
    assert!(super::try_build(&request, None).is_some());
    let mut database = request.clone();
    database.db.only_db = true;
    assert!(super::try_build(&database, None).is_none());
    database.db.only_db = false;
    database.db.offline = true;
    assert!(super::try_build(&database, None).is_none());
}

#[test]
fn proof_creation_never_reuses_changed_sources_or_outputs() {
    let (root, request) = workspace();
    let result = crate::run_build(request.clone());
    assert_eq!(result.diagnostics, []);
    let context = crate::context::CachedDriverContext::load(
        &request.cwd,
        crate::build::RegistrySelection::for_build(request.db),
        request.sdk_source_root.as_deref(),
        request.jobs,
    )
    .unwrap();
    let proof = root.path().join(".dart_tool/dust/unchanged_build_v1.json");
    let original_proof = fs::read(&proof).unwrap();
    for relative in ["lib/item.dart", "lib/item.g.dart"] {
        fs::write(&proof, &original_proof).unwrap();
        let path = root.path().join(relative);
        let before = fs::read(&path).unwrap();
        fs::write(&path, "changed after the original source snapshot").unwrap();
        super::save(
            &request,
            &context.workspace,
            &context.namespaces,
            &context.cache,
            &result,
            &Default::default(),
        );
        assert!(super::try_build(&request, None).is_none(), "{relative}");
        fs::write(path, before).unwrap();
    }
    fs::remove_file(&proof).unwrap_or_default();
    let mut warning = result;
    warning
        .diagnostics
        .push(dust_diagnostics::Diagnostic::warning("retained diagnostic"));
    super::save(
        &request,
        &context.workspace,
        &context.namespaces,
        &context.cache,
        &warning,
        &Default::default(),
    );
    assert!(Manifest::load(root.path()).is_none());
}
