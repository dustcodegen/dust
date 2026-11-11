use std::fs;

use super::*;
use crate::namespace_sources::test_workspace as fixture;

fn workspace() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("pubspec.yaml"),
        "name: cache_prefetch_test\n",
    )
    .unwrap();
    fixture::prepare(root.path(), root.path(), "cache_prefetch_test");
    fs::create_dir_all(root.path().join("lib")).unwrap();
    fs::write(root.path().join("lib/item.dart"), "class Item {}\n").unwrap();
    fs::create_dir_all(root.path().join(".dart_tool/dust")).unwrap();
    root
}

fn load(root: &Path) -> Result<CachedDriverContext, Diagnostic> {
    CachedDriverContext::load(
        root,
        RegistrySelection::All,
        Some(&fixture::sdk_source_root(root)),
        Some(1),
    )
}

#[test]
fn source_read_errors_take_priority_over_regular_bad_cache_recovery() {
    let root = workspace();
    let cache = root.path().join(".dart_tool/dust/build_cache_v1.json");
    fs::write(&cache, "{invalid cache").unwrap();
    fs::write(root.path().join("lib/item.dart"), [0xff]).unwrap();
    let error = load(root.path()).err().expect("source read failure");
    assert!(error.message.contains("cannot read"));
    assert!(error.message.contains("lib/item.dart"));
    assert_eq!(fs::read_to_string(cache).unwrap(), "{invalid cache");
}

#[test]
fn nonregular_cache_errors_remain_deferred_until_sources_are_valid() {
    let root = workspace();
    let cache = root.path().join(".dart_tool/dust/build_cache_v1.json");
    fs::create_dir(&cache).unwrap();
    let source = root.path().join("lib/item.dart");
    fs::write(&source, [0xff]).unwrap();
    assert!(
        load(root.path())
            .err()
            .unwrap()
            .message
            .contains("lib/item.dart")
    );
    fs::write(&source, "class Item {}\n").unwrap();
    assert!(
        load(root.path())
            .err()
            .unwrap()
            .message
            .contains("failed to load Dust cache")
    );
}

#[test]
fn prefetched_cache_retains_warnings_and_exact_loaded_content_identity() {
    let root = workspace();
    let cache_path = root.path().join(".dart_tool/dust/build_cache_v1.json");
    fs::write(&cache_path, "{invalid cache").unwrap();
    let recovered = load(root.path()).unwrap();
    assert!(
        recovered
            .cache_warning
            .unwrap()
            .message
            .contains("ignoring invalid Dust cache")
    );
    assert_eq!(recovered.cache.persisted_hash(), None);
    let contents = "{\"schema_version\":7,\"entries\":{}}";
    fs::write(&cache_path, contents).unwrap();
    let valid = load(root.path()).unwrap();
    assert!(valid.cache_warning.is_none());
    assert_eq!(
        valid.cache.persisted_hash(),
        Some(WorkspaceCache::hash_contents(contents.as_bytes()))
    );
    fs::write(&cache_path, "{changed after load").unwrap();
    assert_eq!(
        valid.cache.persisted_hash(),
        Some(WorkspaceCache::hash_contents(contents.as_bytes()))
    );
}
