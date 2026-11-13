use super::*;

#[test]
fn persisted_digest_tracks_successful_io_and_ignores_later_external_edits() {
    let root = tempfile::tempdir().unwrap();
    let mut cache = WorkspaceCache::load(root.path()).unwrap();
    assert_eq!(cache.persisted_hash(), None);
    fs::create_dir_all(cache.path().parent().unwrap()).unwrap();
    let json = format!(
        r#"{{"schema_version":{},"entries":{{"lib/item.dart":{{"source_hash":1,"package_config_hash":2,"tool_hash":3,"expected_output_hash":4}}}}}}"#,
        CACHE_SCHEMA_VERSION
    );
    fs::write(cache.path(), &json).unwrap();
    cache = WorkspaceCache::load(root.path()).unwrap();
    let original = Some(WorkspaceCache::hash_contents(json.as_bytes()));
    assert_eq!(cache.persisted_hash(), original);
    let source = root.path().join("lib/item.dart");
    let mut entry = cache.get(root.path(), &source).unwrap().clone();
    entry.tool_hash = 8;
    cache.insert(root.path(), &source, entry);
    assert_eq!(cache.persisted_hash(), original);
    cache.flush().unwrap();
    let persisted = Some(WorkspaceCache::hash_contents(
        &fs::read(cache.path()).unwrap(),
    ));
    assert_ne!(persisted, original);
    assert_eq!(cache.persisted_hash(), persisted);
    assert_eq!(WorkspaceCache::load(root.path()).unwrap(), cache);
    fs::write(cache.path(), "{broken").unwrap();
    assert_eq!(cache.persisted_hash(), persisted);
    let (recovered, warning) = WorkspaceCache::load_with_recovery(root.path()).unwrap();
    assert_eq!(recovered.persisted_hash(), None);
    assert!(warning.is_some());
    assert_eq!(WorkspaceCache::hash_contents(b""), 1469598103934665603);
}
