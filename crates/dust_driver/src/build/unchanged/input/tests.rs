use super::*;

/// Fresh emitted facts must preserve exact paths and empty-primary presence checks.
mod fresh;
/// Linear merging keeps the existing proof's path identity and conflict rules.
mod merge;

fn artifact() -> BuildArtifact {
    BuildArtifact {
        source_path: "source.dart".into(),
        output_path: "output.dart".into(),
        auxiliary_output_paths: vec!["first.dart".into(), "second.dart".into()],
        routed: false,
        changed: false,
        written: false,
        cached: false,
    }
}

#[test]
fn only_the_same_verified_ordered_nonempty_output_set_reuses_digests() {
    let artifact = artifact();
    let expected = 137;
    let previous = (
        artifact.source_path.clone(),
        artifact.output_path.clone(),
        artifact.auxiliary_output_paths.clone(),
        artifact.routed,
        expected,
    );
    let files = std::iter::once(&artifact.output_path)
        .chain(&artifact.auxiliary_output_paths)
        .map(|path| (path.clone(), Some(hash_text("verified output"))))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        reusable_outputs(&artifact, expected, &previous, &files),
        Some(files.clone())
    );
    assert!(reusable_outputs(&artifact, expected + 1, &previous, &files).is_none());
    let mut different = artifact.clone();
    different.output_path = "different.dart".into();
    assert!(reusable_outputs(&different, expected, &previous, &files).is_none());
    different = artifact.clone();
    different.auxiliary_output_paths.reverse();
    assert!(reusable_outputs(&different, expected, &previous, &files).is_none());
    for path in files.keys() {
        let mut missing = files.clone();
        missing.remove(path);
        assert!(reusable_outputs(&artifact, expected, &previous, &missing).is_none());
        for digest in [None, Some(hash_text(""))] {
            missing.insert(path.clone(), digest);
            assert!(reusable_outputs(&artifact, expected, &previous, &missing).is_none());
        }
    }
}

#[test]
fn sorted_component_walk_matches_path_order_independent_of_creation_order() {
    let names = [
        "z.dart",
        "a/first.dart",
        "a a.dart",
        "a-0.dart",
        "a.dart",
        "a.b/x.dart",
        ".hidden.dart",
        ".cache/a.dart",
        "a/0.dart",
        "a.b.dart",
        "å.dart",
        "é/first.dart",
        "世界/space name.dart",
        "with space/a.dart",
        "with space.dart",
        "zz/.inside.dart",
        "zz/🦀.dart",
    ];
    for reverse in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let order = if reverse {
            names.iter().rev().copied().collect::<Vec<_>>()
        } else {
            names.to_vec()
        };
        for name in order {
            let path = root.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        fs::write(root.path().join("ignored.txt"), "").unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.path().join("z.dart"), root.path().join("linked.dart"))
                .unwrap();
            std::os::unix::fs::symlink(root.path().join("a"), root.path().join("linked_directory"))
                .unwrap();
        }
        let expected = names
            .iter()
            .map(|name| root.path().join(name))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(lib_files(root.path()).unwrap(), expected);
        assert!(lib_files(&root.path().join("absent")).unwrap().is_empty());
        assert!(lib_files(&root.path().join("z.dart")).is_none());
    }
}
