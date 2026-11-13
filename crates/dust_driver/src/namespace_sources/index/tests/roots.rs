use super::*;

#[test]
fn deepest_package_without_override_uses_sdk_for_files_and_relative_parts() {
    let root = workspace();
    write(
        &root.path().join(".dart_tool/package_config.json"),
        r#"{"configVersion":2,"packages":[
          {"name":"outer","rootUri":"../","languageVersion":"3.6"},
          {"name":"nested","rootUri":"../lib/nested","packageUri":"sources/"}
        ]}"#,
    );
    let index = load(root.path());
    let base = index.file(&root.path().join("lib/base.dart")).unwrap();
    assert_eq!(base.language_version.to_string(), "3.6");
    let nested = index
        .file(&root.path().join("lib/nested/sources/model.dart"))
        .unwrap();
    let relative = index.resolve(&base, "nested/model.g.dart").unwrap();
    let package = index.resolve(&base, "package:nested/model.dart").unwrap();
    for source in [nested, relative, package] {
        assert_eq!(
            source.language_version.to_string(),
            "3.13",
            "{}",
            source.uri
        );
    }
}

#[test]
fn file_roots_use_normalized_component_prefixes_and_disjoint_package_versions() {
    let root = workspace();
    write(
        &root.path().join(".dart_tool/package_config.json"),
        r#"{"configVersion":2,"packages":[
          {"name":"z_app","rootUri":"../app/./src/../","languageVersion":"3.7"},
          {"name":"a_nested","rootUri":"../app/features/../features/inner/","languageVersion":"3.10"},
          {"name":"other","rootUri":"../other/","languageVersion":"3.9"}
        ]}"#,
    );
    let index = load(root.path());
    for (path, expected, normalized) in [
        ("app/lib/source.dart", "3.7", "app/lib/source.dart"),
        (
            "app/lib/../features/inner/model.dart",
            "3.10",
            "app/features/inner/model.dart",
        ),
        ("other/source.dart", "3.9", "other/source.dart"),
        (
            "other_suffix/source.dart",
            "3.13",
            "other_suffix/source.dart",
        ),
        ("app_suffix/source.dart", "3.13", "app_suffix/source.dart"),
    ] {
        let source = index.file(&root.path().join(path)).unwrap();
        assert_eq!(source.language_version.to_string(), expected, "{path}");
        assert_eq!(source.path, root.path().join(normalized));
        assert_eq!(
            source.uri,
            crate::namespace_sources::uri::file_uri(&source.path).unwrap()
        );
    }
}

#[test]
fn relative_config_empty_root_still_matches_absolute_and_relative_files() {
    let root = workspace();
    let mut index = load(root.path());
    let packages = super::super::packages::load(
        Path::new(".dart_tool/package_config.json"),
        r#"{"configVersion":2,"packages":[
          {"name":"empty","rootUri":"../","languageVersion":"3.7"}
        ]}"#,
    )
    .unwrap();
    let package = packages.into_values().next().unwrap();
    assert!(package.root.as_os_str().is_empty());
    index
        .file_roots
        .insert(package.root, package.language_version);
    for path in [
        root.path().parent().unwrap().join("outside.dart"),
        Path::new("relative/model.dart").to_path_buf(),
        Path::new("").to_path_buf(),
    ] {
        assert_eq!(
            index.file(&path).unwrap().language_version.to_string(),
            "3.7"
        );
    }
    assert_eq!(
        index
            .file(&root.path().join("lib/model.dart"))
            .unwrap()
            .language_version
            .to_string(),
        "3.6"
    );
}

#[test]
fn ancestor_selection_matches_the_previous_sorted_component_prefix_scan() {
    let root = workspace();
    let index = load(root.path());
    let mut packages = index.packages.values().rev().collect::<Vec<_>>();
    packages.sort_by_key(|package| std::cmp::Reverse(package.root.components().count()));
    let mut paths = vec![
        root.path().to_path_buf(),
        root.path().join("lib/model.dart"),
        root.path().join("lib/nested"),
        root.path().join("lib/nested/sources/model.dart"),
        root.path().join("lib/nested/../model.g.dart"),
        root.path().join("defaulted/model.dart"),
        root.path().join("lib/nested_suffix/model.dart"),
        root.path().parent().unwrap().join("outside.dart"),
        Path::new("relative/model.dart").to_path_buf(),
        Path::new("../relative/model.dart").to_path_buf(),
        Path::new("").to_path_buf(),
    ];
    paths.extend(root.path().ancestors().map(Path::to_path_buf));
    for path in paths {
        let normalized = crate::source_uri::normalize_path(&path);
        let expected = packages
            .iter()
            .find(|package| normalized.starts_with(&package.root))
            .and_then(|package| package.language_version)
            .unwrap_or(index.sdk.language_version);
        let source = index.file(&path).unwrap();
        assert_eq!(source.language_version, expected, "{}", path.display());
        assert_eq!(source.path, normalized);
        assert_eq!(
            source.uri,
            crate::namespace_sources::uri::file_uri(&source.path).unwrap()
        );
    }
}

#[cfg(windows)]
#[test]
fn drive_relative_config_root_keeps_absolute_drive_prefix_matching() {
    let root = workspace();
    let mut index = load(root.path());
    index.packages = super::super::packages::load(
        Path::new(r"C:.dart_tool/package_config.json"),
        r#"{"configVersion":2,"packages":[
          {"name":"drive","rootUri":"../","languageVersion":"3.7"},
          {"name":"nested","rootUri":"file:///C:/nested/"},
          {"name":"deep","rootUri":"file:///C:/nested/deep/","languageVersion":"3.10"}
        ]}"#,
    )
    .unwrap();
    assert_eq!(index.packages["drive"].root, Path::new("C:"));
    index.file_roots = index
        .packages
        .values()
        .map(|package| (package.root.clone(), package.language_version))
        .collect();
    let mut packages = index.packages.values().rev().collect::<Vec<_>>();
    packages.sort_by_key(|package| std::cmp::Reverse(package.root.components().count()));
    for (path, version) in [
        (r"C:\outside\model.dart", "3.7"),
        (r"c:\outside\model.dart", "3.7"),
        (r"C:relative\model.dart", "3.7"),
        (r"C:\nested\model.dart", "3.13"),
        (r"C:\nested\deep\model.dart", "3.10"),
        (r"D:\outside\model.dart", "3.13"),
    ] {
        let path = Path::new(path);
        let normalized = crate::source_uri::normalize_path(path);
        let expected = packages
            .iter()
            .find(|package| normalized.starts_with(&package.root))
            .and_then(|package| package.language_version)
            .unwrap_or(index.sdk.language_version);
        let source = index.file(path).unwrap();
        assert_eq!(source.language_version, expected, "{}", path.display());
        assert_eq!(
            source.language_version.to_string(),
            version,
            "{}",
            path.display()
        );
        assert_eq!(source.path, normalized);
    }
}
