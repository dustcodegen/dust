use super::*;

#[test]
fn source_changes_with_identical_generated_bytes_record_a_new_valid_proof() {
    let (root, request) = workspace();
    let result = crate::run_build(request.clone());
    assert_eq!(result.diagnostics, []);
    let before = Manifest::load(root.path()).unwrap();
    let output_path = root.path().join("lib/item.g.dart");
    let output = fs::read(&output_path).unwrap();
    let source_path = root.path().join("lib/item.dart");
    let mut source = fs::read_to_string(&source_path).unwrap();
    source.push_str("\n// Changed source, identical generation.\n");
    fs::write(&source_path, source).unwrap();
    assert!(super::super::try_build(&request, None).is_none());
    let changed = crate::run_build(request.clone());
    assert_eq!(changed.diagnostics, []);
    assert_eq!(fs::read(&output_path).unwrap(), output);
    let after = Manifest::load(root.path()).unwrap();
    assert_ne!(before.files[&source_path], after.files[&source_path]);
    assert_eq!(before.artifacts, after.artifacts);
    assert!(super::super::try_build(&request, None).is_some());
}

#[test]
fn prior_proof_schemas_require_fresh_output_validation() {
    let (root, request) = workspace();
    assert_eq!(crate::run_build(request).diagnostics, []);
    let path = root.path().join(".dart_tool/dust/unchanged_build_v1.json");
    let original = fs::read(&path).unwrap();
    for schema in [1, 2, 4] {
        let mut previous: Manifest = serde_json::from_slice(&original).unwrap();
        previous.schema = schema;
        previous.save(root.path());
        assert!(Manifest::load(root.path()).is_none());
    }
}

#[test]
fn proof_save_cannot_bless_cache_corruption_after_the_original_load() {
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
    let cache_path = context.cache.path();
    fs::write(cache_path, "{corrupted after load").unwrap();
    super::super::save(
        &request,
        &context.workspace,
        &context.namespaces,
        &context.cache,
        &result,
        &Default::default(),
    );
    assert_eq!(
        Manifest::load(root.path()).unwrap().cache_hash,
        context.cache.persisted_hash().unwrap()
    );
    assert!(super::super::try_build(&request, None).is_none());
    let rebuilt = crate::run_build(request);
    assert!(!rebuilt.has_errors());
    assert!(
        rebuilt
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("ignoring invalid Dust cache"))
    );
}

#[test]
fn unsorted_or_duplicate_library_paths_are_not_reused() {
    let (root, request) = workspace();
    assert_eq!(crate::run_build(request.clone()).diagnostics, []);
    let path = root.path().join(".dart_tool/dust/unchanged_build_v1.json");
    let original = fs::read(&path).unwrap();
    for duplicate in [false, true] {
        fs::write(&path, &original).unwrap();
        let mut manifest = Manifest::load(root.path()).unwrap();
        if duplicate {
            manifest.lib_files.insert(0, manifest.lib_files[0].clone());
        } else {
            manifest.lib_files.reverse();
        }
        manifest.save(root.path());
        assert!(super::super::try_build(&request, None).is_none());
    }
    fs::write(path, original).unwrap();
    assert!(super::super::try_build(&request, None).is_some());
}

#[test]
fn unknown_marked_files_cannot_expand_the_saved_discovery_set() {
    let (root, request) = workspace();
    let unknown = root.path().join("lib/unreferenced.g.dart");
    fs::write(&unknown, dust_plugin_api::GENERATED_HEADER).unwrap();
    assert_eq!(crate::run_build(request.clone()).diagnostics, []);
    let manifest = Manifest::load(root.path()).unwrap();
    assert!(!manifest.lib_files.contains(&unknown));
    assert!(!manifest.files.contains_key(&unknown));
    assert!(super::super::try_build(&request, None).is_none());
}
