use dust_dart_syntax::DartLanguageVersion;

use super::*;

fn source(root: &std::path::Path, name: &str, bytes: usize) -> Source {
    let path = root.join(name);
    fs::write(&path, vec![b' '; bytes]).unwrap();
    Source {
        uri: format!("package:example/{name}"),
        path,
        language_version: DartLanguageVersion::latest_known(),
    }
}

#[test]
fn file_length_orders_work_before_uri_and_equal_lengths_keep_uri_order() {
    let root = tempfile::tempdir().unwrap();
    let small = source(root.path(), "a-small.dart", 1);
    let last = source(root.path(), "z-large.dart", 8);
    let first = source(root.path(), "b-large.dart", 8);
    let mut queue = [small.clone(), last.clone(), first.clone()]
        .into_iter()
        .collect::<Queue>();

    for expected in [first.uri, last.uri, small.uri] {
        assert_eq!(queue.entries.len(), queue.priority.len());
        assert_eq!(queue.pop_first().unwrap().1.uri, expected);
    }
    assert!(queue.entries.is_empty());
    assert!(queue.priority.is_empty());
    assert!(queue.pop_first().is_none());
}

#[test]
fn duplicate_edges_and_retention_retries_reuse_the_original_hint() {
    let root = tempfile::tempdir().unwrap();
    let large = source(root.path(), "z-large.dart", 8);
    let small = source(root.path(), "a-small.dart", 1);
    let mut queue = Queue::default();
    queue.insert(&large);
    fs::write(&large.path, "").unwrap();
    queue.insert(&large);
    queue.insert(&small);
    let entry = queue.pop_first().unwrap();
    assert_eq!((entry.0, &entry.1.uri), (8, &large.uri));
    queue.restore(entry);
    assert_eq!(queue.pop_first().unwrap().1.uri, large.uri);
    assert_eq!(queue.len(), 1);
    queue.remove(&small.uri).unwrap();
    assert!(queue.remove(&small.uri).is_none());
    assert!(queue.entries.is_empty());
    assert!(queue.priority.is_empty());
}

#[test]
fn missing_paths_and_nonregular_files_have_zero_priority_and_can_be_cleared() {
    let root = tempfile::tempdir().unwrap();
    let mut directory = source(root.path(), "z-directory.dart", 1);
    let missing = source(root.path(), "a-missing.dart", 1);
    directory.path = root.path().to_path_buf();
    fs::remove_file(&missing.path).unwrap();
    let mut queue = [directory, missing.clone()].into_iter().collect::<Queue>();
    let (bytes, first) = queue.pop_first().unwrap();
    assert_eq!(bytes, 0);
    assert_eq!(first.uri, missing.uri);
    queue.clear();
    assert!(queue.is_empty());
    assert!(queue.priority.is_empty());
    assert!(queue.pop_first().is_none());
}
