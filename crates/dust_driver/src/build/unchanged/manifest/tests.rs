use super::*;

const PROOF: &str = r#"{
  "integrity": 0,
  "schema": 3,
  "tool_hash": 7,
  "cache_hash": 11,
  "metadata": {"sdk/version": 13},
  "lib_files": ["lib/model.dart", "lib/model.g.dart"],
  "files": {"lib/model.dart": 17, "lib/model.g.dart": 19, "absent.dart": null},
  "dust_packages": ["dust_dart"],
  "artifacts": [["lib/model.dart", "lib/model.g.dart", ["test/generated/model_test.dart"], false, 23]]
}"#;

fn valid_proof() -> Vec<u8> {
    serde_json::from_str::<Manifest>(PROOF)
        .unwrap()
        .encode()
        .unwrap()
}

fn write(root: &Path, contents: &[u8]) {
    fs::create_dir_all(path(root).parent().unwrap()).unwrap();
    fs::write(path(root), contents).unwrap();
}

#[test]
fn typed_proofs_preserve_field_names_paths_and_artifact_tuple_shape() {
    let root = tempfile::tempdir().unwrap();
    let original = valid_proof();
    write(root.path(), &original);
    let manifest = Manifest::load(root.path()).unwrap();
    assert_eq!(manifest.schema, 3);
    assert_eq!(manifest.artifacts[0].4, 23);
    assert_eq!(manifest.files[Path::new("absent.dart")], None);
    let encoded = manifest.encode().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&encoded).unwrap(),
        serde_json::from_slice::<serde_json::Value>(&original).unwrap(),
    );
}

#[test]
fn single_payload_encoding_matches_the_original_flattened_representation_exactly() {
    #[derive(Serialize)]
    struct OriginalEncoding<'a> {
        integrity: u64,
        #[serde(flatten)]
        manifest: &'a Manifest,
    }

    for empty in [false, true] {
        let mut manifest: Manifest = serde_json::from_str(PROOF).unwrap();
        if empty {
            manifest.metadata.clear();
            manifest.lib_files.clear();
            manifest.files.clear();
            manifest.dust_packages.clear();
            manifest.artifacts.clear();
        } else {
            let path = PathBuf::from("lib/ünicode \"quoted\"\\model.dart");
            manifest.metadata.insert(path.clone(), u64::MAX);
            manifest.lib_files.push(path.clone());
            manifest.files.insert(path, Some(u64::MAX));
            manifest.dust_packages.push("escaped\npackage".into());
        }
        let payload = serde_json::to_vec(&manifest).unwrap();
        let original = serde_json::to_vec(&OriginalEncoding {
            integrity: WorkspaceCache::hash_contents(&payload),
            manifest: &manifest,
        })
        .unwrap();
        assert_eq!(manifest.encode().unwrap(), original);
        assert!(Manifest::decode(&original).is_some());
    }
}

#[test]
fn missing_or_malformed_proof_fields_and_unknown_schemas_are_misses() {
    let root = tempfile::tempdir().unwrap();
    let original: serde_json::Value = serde_json::from_slice(&valid_proof()).unwrap();
    for key in original.as_object().unwrap().keys() {
        let mut missing = original.clone();
        missing.as_object_mut().unwrap().remove(key);
        write(root.path(), &serde_json::to_vec(&missing).unwrap());
        assert!(Manifest::load(root.path()).is_none(), "{key}");
    }
    for (key, invalid) in [
        ("schema", serde_json::json!(1)),
        ("schema", serde_json::json!(2)),
        ("schema", serde_json::json!(4)),
        ("schema", serde_json::json!("3")),
        ("files", serde_json::json!(["not a map"])),
        (
            "artifacts",
            serde_json::json!([["source", "output", [], false]]),
        ),
    ] {
        let mut malformed = original.clone();
        malformed[key] = invalid;
        write(root.path(), &serde_json::to_vec(&malformed).unwrap());
        assert!(Manifest::load(root.path()).is_none(), "{key}");
    }
}

#[test]
#[cfg(unix)]
fn non_utf8_path_serialization_keeps_the_existing_optional_proof() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let root = tempfile::tempdir().unwrap();
    let original = valid_proof();
    write(root.path(), &original);
    let mut manifest = Manifest::load(root.path()).unwrap();
    manifest
        .files
        .insert(PathBuf::from(OsString::from_vec(vec![0xff])), Some(29));
    manifest.save(root.path());
    assert_eq!(fs::read(path(root.path())).unwrap(), original);
}

#[test]
fn integrity_covers_every_known_payload_field_without_requiring_json_byte_order() {
    let root = tempfile::tempdir().unwrap();
    let original: serde_json::Value = serde_json::from_slice(&valid_proof()).unwrap();
    write(root.path(), &serde_json::to_vec_pretty(&original).unwrap());
    assert!(Manifest::load(root.path()).is_some());
    for (key, changed) in [
        (
            "integrity",
            serde_json::json!(original["integrity"].as_u64().unwrap() ^ 1),
        ),
        ("tool_hash", serde_json::json!(8)),
        ("cache_hash", serde_json::json!(12)),
        ("metadata", serde_json::json!({"sdk/version": 14})),
        ("lib_files", serde_json::json!(["lib/model.dart"])),
        ("files", serde_json::json!({"lib/model.dart": 17})),
        ("dust_packages", serde_json::json!(["dust_flutter"])),
        (
            "artifacts",
            serde_json::json!([["lib/model.dart", "lib/model.g.dart", [], true, 23]]),
        ),
    ] {
        let mut corrupted = original.clone();
        corrupted[key] = changed;
        write(root.path(), &serde_json::to_vec(&corrupted).unwrap());
        assert!(Manifest::load(root.path()).is_none(), "{key}");
    }
}

#[test]
fn a_non_regular_proof_path_is_a_cache_miss() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(path(root.path())).unwrap();
    assert!(Manifest::load(root.path()).is_none());
    assert!(Manifest::load_matching(root.path(), 7, 11).is_none());
}

#[test]
fn header_hints_reject_wrong_tool_cache_or_schema_before_reuse() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), &valid_proof());
    assert!(Manifest::load_matching(root.path(), 7, 11).is_some());
    assert!(Manifest::load_matching(root.path(), 8, 11).is_none());
    assert!(Manifest::load_matching(root.path(), 7, 12).is_none());
    let mut proof: Manifest = serde_json::from_slice(&valid_proof()).unwrap();
    proof.schema = 2;
    proof.save(root.path());
    assert!(Manifest::load_matching(root.path(), 7, 11).is_none());
}

#[test]
fn a_matching_header_never_accepts_invalid_payloads_or_integrity() {
    let root = tempfile::tempdir().unwrap();
    let original: serde_json::Value = serde_json::from_slice(&valid_proof()).unwrap();
    for (key, invalid) in [
        ("files", serde_json::json!(["not a map"])),
        ("metadata", serde_json::json!({"sdk/version": 29})),
        ("integrity", serde_json::json!(null)),
    ] {
        let mut proof = original.clone();
        proof[key] = invalid;
        write(root.path(), &serde_json::to_vec(&proof).unwrap());
        assert!(
            Manifest::load_matching(root.path(), 7, 11).is_none(),
            "{key}"
        );
    }
}

#[test]
#[cfg(unix)]
fn atomic_proof_save_replaces_a_symlink_without_touching_handwritten_source() {
    let root = tempfile::tempdir().unwrap();
    let source_path = root.path().join("handwritten.dart");
    let source = "class Handwritten {}\n";
    fs::write(&source_path, source).unwrap();
    fs::create_dir_all(path(root.path()).parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&source_path, path(root.path())).unwrap();
    let manifest: Manifest = serde_json::from_str(PROOF).unwrap();
    manifest.save(root.path());
    assert_eq!(fs::read_to_string(&source_path).unwrap(), source);
    assert!(
        !fs::symlink_metadata(path(root.path()))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(Manifest::load(root.path()).is_some());
}
