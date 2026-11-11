use super::*;

fn files(entries: &[(&str, Option<u64>)]) -> BTreeMap<PathBuf, Option<u64>> {
    entries
        .iter()
        .map(|(path, hash)| (PathBuf::from(path), *hash))
        .collect()
}

#[test]
fn empty_and_disjoint_edges_keep_all_file_states() {
    let empty = BTreeMap::new();
    let first = files(&[("a.dart", None), ("b.dart", Some(0))]);
    let last = files(&[("y.dart", Some(7)), ("z.dart", None)]);
    assert_eq!(merge(empty.clone(), empty.clone()), Some(empty.clone()));
    assert_eq!(merge(first.clone(), empty.clone()), Some(first.clone()));
    assert_eq!(merge(empty, first.clone()), Some(first.clone()));
    let expected = files(&[
        ("a.dart", None),
        ("b.dart", Some(0)),
        ("y.dart", Some(7)),
        ("z.dart", None),
    ]);
    assert_eq!(merge(first.clone(), last.clone()), Some(expected.clone()));
    assert_eq!(merge(last, first), Some(expected));
}

#[test]
fn interleaved_and_equal_paths_preserve_exact_optional_hashes() {
    let left = files(&[
        ("a.dart", None),
        ("c.dart", Some(0)),
        ("e.dart", Some(9)),
        ("g.dart", None),
    ]);
    let right = files(&[
        ("b.dart", Some(4)),
        ("c.dart", Some(0)),
        ("d.dart", None),
        ("g.dart", None),
    ]);
    let mut expected = left.clone();
    expected.extend(right.clone());
    assert_eq!(merge(left, right), Some(expected));
}

#[test]
fn conflicting_presence_or_digest_rejects_the_whole_merge() {
    for (left, right) in [
        (None, Some(0)),
        (Some(0), None),
        (Some(0), Some(1)),
        (Some(u64::MAX), Some(u64::MAX - 1)),
    ] {
        assert!(
            merge(
                files(&[("a.dart", Some(1)), ("c.dart", left)]),
                files(&[("b.dart", None), ("c.dart", right), ("z.dart", Some(2))]),
            )
            .is_none()
        );
    }
}

#[test]
fn equal_paths_retain_the_left_raw_key_spelling() {
    for (left, right) in [
        ("lib/./row.dart", "lib/row.dart"),
        ("lib//row.dart", "lib/row.dart"),
        ("lib/row.dart", "lib/./row.dart"),
        ("lib/row.dart", "lib//row.dart"),
    ] {
        assert_eq!(Path::new(left), Path::new(right));
        for hash in [None, Some(0), Some(u64::MAX)] {
            let merged = merge(files(&[(left, hash)]), files(&[(right, hash)])).unwrap();
            assert_eq!(merged.len(), 1);
            let (path, actual) = merged.first_key_value().unwrap();
            assert_eq!(path.as_os_str(), Path::new(left).as_os_str());
            assert_eq!(*actual, hash);
        }
    }
}
