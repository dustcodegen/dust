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
