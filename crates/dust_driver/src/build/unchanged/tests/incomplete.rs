use super::*;

#[test]
fn corrupt_proof_cannot_hide_changed_foreign_sdk_source_from_the_build() {
    let (root, request) = workspace();
    let cold = crate::run_build(request.clone());
    assert_eq!(cold.diagnostics, []);
    let proof_path = root.path().join(".dart_tool/dust/unchanged_build_v1.json");
    let foreign = namespace_fixture::sdk_source_root(root.path()).join("lib/core/core.dart");
    let original = Manifest::load(root.path()).unwrap();
    assert!(original.files.contains_key(&foreign));
    assert!(!original.metadata.contains_key(&foreign));
    assert!(!original.lib_files.contains(&foreign));
    let cache_path = input::cache_path(&dust_workspace::load_workspace(root.path()).unwrap());
    let cache = fs::read(&cache_path).unwrap();
    let mut proof: serde_json::Value =
        serde_json::from_slice(&fs::read(&proof_path).unwrap()).unwrap();
    assert!(
        proof["files"]
            .as_object_mut()
            .unwrap()
            .remove(foreign.to_str().unwrap())
            .is_some()
    );
    fs::write(&proof_path, serde_json::to_vec(&proof).unwrap()).unwrap();
    fs::write(&foreign, "library dart.core; class Object {").unwrap();
    assert!(Manifest::load(root.path()).is_none());
    assert_eq!(fs::read(&cache_path).unwrap(), cache);
    assert!(super::super::try_build(&request, None).is_none());
    let complete = crate::run_build(request);
    assert!(
        complete.has_errors(),
        "the complete pipeline rejects the changed fake SDK source"
    );
}

#[test]
fn resealed_proofs_still_require_root_output_and_metadata_coverage() {
    let (root, request) = workspace();
    assert_eq!(crate::run_build(request.clone()).diagnostics, []);
    let proof_path = root.path().join(".dart_tool/dust/unchanged_build_v1.json");
    let original = fs::read(&proof_path).unwrap();
    for missing in [
        root.path().join("lib/item.dart"),
        root.path().join("lib/analysis.g.dart"),
        namespace_fixture::sdk_source_root(root.path()).join("version"),
    ] {
        let mut manifest: Manifest = serde_json::from_slice(&original).unwrap();
        assert!(manifest.files.remove(&missing).is_some());
        manifest.save(root.path());
        assert!(
            Manifest::load(root.path()).is_some(),
            "checksum remains valid"
        );
        assert!(
            super::super::try_build(&request, None).is_none(),
            "{}",
            missing.display()
        );
    }
    let mut manifest: Manifest = serde_json::from_slice(&original).unwrap();
    manifest.artifacts[0]
        .2
        .push(root.path().join("test/generated/missing.dart"));
    manifest.save(root.path());
    assert!(Manifest::load(root.path()).is_some());
    assert!(super::super::try_build(&request, None).is_none());
}
