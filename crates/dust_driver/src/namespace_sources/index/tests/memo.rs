use super::*;

#[test]
fn absolute_results_are_copied_across_bases_without_caching_relative_references() {
    let root = workspace();
    let index = load(root.path());
    let bases = [
        index.file(&root.path().join("lib/model.dart")).unwrap(),
        index
            .file(&root.path().join("lib/nested/sources/model.dart"))
            .unwrap(),
    ];
    std::thread::scope(|scope| {
        for base in &bases {
            let index = &index;
            scope.spawn(move || {
                for reference in [
                    "package:app/shared.dart",
                    "PACKAGE:app/shared.dart",
                    "dart:core",
                    "DART:core",
                ] {
                    let first = index.resolve(base, reference).unwrap();
                    let expected = (
                        first.uri.clone(),
                        first.path.clone(),
                        first.language_version,
                    );
                    let mut copy = index.resolve(base, reference).unwrap();
                    copy.uri.push_str("changed");
                    copy.path.push("changed");
                    let again = index.resolve(base, reference).unwrap();
                    assert_eq!((again.uri, again.path, again.language_version), expected);
                }
                assert!(index.resolve(base, "package:absent/file.dart").is_err());
            });
        }
    });
    for reference in ["package:app/shared.dart?x", "DART:core#x"] {
        assert_eq!(
            index.resolve(&bases[0], reference).unwrap_err(),
            "source URI must not contain a query or fragment"
        );
    }
    let first = index.resolve(&bases[0], "shared.dart").unwrap();
    let second = index.resolve(&bases[1], "shared.dart").unwrap();
    assert_eq!(first.path, root.path().join("lib/shared.dart"));
    assert_eq!(
        second.path,
        root.path().join("lib/nested/sources/shared.dart")
    );
    assert_eq!(first.language_version.to_string(), "3.6");
    assert_eq!(second.language_version.to_string(), "3.10");
    for reference in ["package:app/shared.dart", "dart:core"] {
        let first = index.resolve(&bases[0], reference).unwrap();
        let second = index.resolve(&bases[1], reference).unwrap();
        assert_eq!(
            (first.uri, first.path, first.language_version),
            (second.uri, second.path, second.language_version)
        );
    }
}

#[test]
fn reloading_metadata_discards_both_cached_sources_and_cached_errors() {
    let root = workspace();
    let original = load(root.path());
    let base = original.file(&root.path().join("lib/model.dart")).unwrap();
    let before = original.resolve(&base, "package:app/shared.dart").unwrap();
    assert!(original.resolve(&base, "package:added/file.dart").is_err());
    write(
        &root.path().join(".dart_tool/package_config.json"),
        r#"{
      "configVersion":2,"packages":[
        {"name":"app","rootUri":"../moved","packageUri":"sources/","languageVersion":"3.10"},
        {"name":"added","rootUri":"../added","packageUri":"lib/","languageVersion":"3.11"}]}
    "#,
    );
    let reloaded = load(root.path());
    let moved = reloaded.resolve(&base, "package:app/shared.dart").unwrap();
    assert_eq!(moved.uri, before.uri);
    assert_eq!(moved.path, root.path().join("moved/sources/shared.dart"));
    assert_eq!(moved.language_version.to_string(), "3.10");
    assert_ne!(moved.path, before.path);
    assert_eq!(
        original
            .resolve(&base, "package:app/shared.dart")
            .unwrap()
            .path,
        before.path
    );
    assert!(original.resolve(&base, "package:added/file.dart").is_err());
    assert_eq!(
        reloaded
            .resolve(&base, "package:added/file.dart")
            .unwrap()
            .path,
        root.path().join("added/lib/file.dart")
    );
}
