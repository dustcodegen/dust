use std::{fs, path::Path};

use super::SourceIndex;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn workspace() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    write(&root.path().join("sdk/version"), "3.13.5\n");
    write(
        &root.path().join("sdk/lib/libraries.json"),
        r#"{
      "vm":{"libraries":{"core":{"uri":"core/core.dart"},
        "_private":{"uri":"private/vm.dart"}}},
      "web":{"libraries":{"core":{"uri":"core/core.dart"},
        "_private":{"uri":"private/web.dart"}}}}"#,
    );
    write(
        &root.path().join(".dart_tool/package_config.json"),
        r#"{
      "configVersion":2,"packages":[
        {"name":"app","rootUri":"../","packageUri":"lib/","languageVersion":"3.6"},
        {"name":"nested","rootUri":"../lib/nested","packageUri":"sources/","languageVersion":"3.10"},
        {"name":"defaulted","rootUri":"../defaulted"}]}"#,
    );
    root
}

fn load(root: &Path) -> SourceIndex {
    SourceIndex::load(
        root,
        &root.join(".dart_tool/package_config.json"),
        Some(Path::new("sdk")),
    )
    .unwrap()
}

#[test]
fn actual_package_config_and_sdk_metadata_are_retained() {
    let root = workspace();
    let index = load(root.path());
    assert_eq!(index.inputs.len(), 3);
    assert!(
        index
            .inputs
            .contains_key(&root.path().join(".dart_tool/package_config.json"))
    );
    let base = index.file(&root.path().join("lib/model.dart")).unwrap();
    assert_eq!(base.language_version.to_string(), "3.6");
    let package = index.resolve(&base, "package:app/model.dart").unwrap();
    assert_eq!(package.path, base.path);
    assert_ne!(package.uri, base.uri);
    assert_eq!(package.uri, "package:app/model.dart");
    assert_eq!(
        index
            .file(&root.path().join("lib/nested/test/file.dart"))
            .unwrap()
            .language_version
            .to_string(),
        "3.10"
    );
    assert_eq!(
        index
            .resolve(&base, "package:defaulted/file.dart")
            .unwrap()
            .language_version
            .to_string(),
        "3.13"
    );
}

#[test]
fn package_relative_resolution_preserves_package_identity_and_clamps_parents() {
    let root = workspace();
    let index = load(root.path());
    let file = index.file(&root.path().join("lib/model.dart")).unwrap();
    let base = index.resolve(&file, "package:app/src/model.dart").unwrap();
    for (reference, expected) in [
        ("../other.dart", "package:app/other.dart"),
        ("../../../other.dart", "package:app/other.dart"),
        ("/other.dart", "package:app/other.dart"),
        ("./other.dart", "package:app/src/other.dart"),
        ("", "package:app/src/model.dart"),
        ("package:%61pp/%6Dodel.dart", "package:app/model.dart"),
        ("package:app/src/../model.dart", "package:app/model.dart"),
    ] {
        assert_eq!(
            index.resolve(&base, reference).unwrap().uri,
            expected,
            "{reference}"
        );
    }
    let top = index.resolve(&file, "package:app/model.dart").unwrap();
    assert_eq!(
        index.resolve(&top, "other.dart").unwrap().uri,
        "package:app/other.dart"
    );
    let nested = index.resolve(&base, "package:nested/file.dart").unwrap();
    assert_eq!(
        nested.path,
        root.path().join("lib/nested/sources/file.dart")
    );
}

#[test]
fn file_reference_identity_is_not_reconstructed_from_a_normalized_io_path() {
    let root = workspace();
    let index = load(root.path());
    let base = index.file(&root.path().join("lib/model.dart")).unwrap();
    let doubled = index.resolve(&base, "FILE:///tmp//name.dart").unwrap();
    let simple = index.resolve(&base, "file:///tmp/name.dart").unwrap();
    assert_eq!(doubled.path, simple.path);
    assert_eq!(doubled.uri, "file:///tmp//name.dart");
    assert_ne!(doubled.uri, simple.uri);
    assert_eq!(
        index.resolve(&base, "other%20file.dart").unwrap().path,
        root.path().join("lib/other file.dart")
    );
    let literal = index.file(&root.path().join("lib/a%b#c.dart")).unwrap();
    assert!(literal.uri.ends_with("/lib/a%25b%23c.dart"));
    assert_eq!(index.resolve(&literal, "").unwrap().uri, literal.uri);
}

#[test]
fn sdk_parts_use_the_declaring_sdk_library_and_metadata_language() {
    let root = workspace();
    let index = load(root.path());
    let base = index.file(&root.path().join("lib/model.dart")).unwrap();
    let core = index.resolve(&base, "dart:core").unwrap();
    assert_eq!(core.path, root.path().join("sdk/lib/core/core.dart"));
    assert_eq!(core.language_version.to_string(), "3.13");
    let part = index.resolve(&core, "object.dart").unwrap();
    assert_eq!(part.uri, "dart:core/object.dart");
    assert_eq!(index.resolve(&part, "core.dart").unwrap().uri, "dart:core");
    assert!(
        index
            .resolve(&core, "dart:_private")
            .unwrap_err()
            .contains("target-dependent")
    );
    assert!(
        index
            .resolve(&core, "dart:absent")
            .unwrap_err()
            .contains("no `dart:absent`")
    );
}

#[test]
fn missing_or_unrepresentable_source_references_never_become_local_guesses() {
    let root = workspace();
    let index = load(root.path());
    let base = index.file(&root.path().join("lib/model.dart")).unwrap();
    for reference in [
        "package:absent/file.dart",
        "package:app",
        "package:/app/file.dart",
        "package:app/../outside.dart",
        "package:app/%2e%2e/outside.dart",
        "https://example.invalid/x.dart",
        "source.dart?x",
        "source.dart#x",
        "//remote.invalid/x.dart",
        "bad%FF.dart",
        "bad%2Fpath.dart",
        "file://remote.invalid/x.dart",
    ] {
        assert!(index.resolve(&base, reference).is_err(), "{reference}");
    }
}

#[test]
fn invalid_package_metadata_is_not_legacy_annotation_provenance() {
    for config in [
        "{}",
        "[]",
        "[",
        r#"{"configVersion":1,"packages":[]}"#,
        r#"{"configVersion":2}"#,
        r#"{"configVersion":2,"packages":[{}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p"}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"..","rootUri":"../"}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"https://invalid.test/"}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"../","packageUri":"../outside"}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"../","languageVersion":"03.6"}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"../","languageVersion":"3"}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"../","languageVersion":"3.65536"}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"../","languageVersion":null}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"../","packageUri":false}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"../p"},{"name":"q","rootUri":"../p"}]}"#,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"../p"},{"name":"p","rootUri":"../q"}]}"#,
    ] {
        let root = workspace();
        let path = root.path().join(".dart_tool/package_config.json");
        write(&path, config);
        assert!(
            SourceIndex::load(root.path(), &path, Some(Path::new("sdk"))).is_err(),
            "{config}"
        );
    }
}

#[test]
fn empty_package_uri_uses_the_declared_root_and_missing_config_is_not_ignored() {
    let root = workspace();
    let path = root.path().join(".dart_tool/package_config.json");
    write(
        &path,
        r#"{"configVersion":2,"packages":[{"name":"p","rootUri":"../","packageUri":""}]}"#,
    );
    let index = load(root.path());
    let base = index.file(&root.path().join("entry.dart")).unwrap();
    let target = index.resolve(&base, "package:p/source.dart").unwrap();
    assert_eq!(target.path, root.path().join("source.dart"));
    assert_eq!(target.uri, "package:p/source.dart");
    assert_eq!(target.language_version.to_string(), "3.13");
    fs::remove_file(&path).unwrap();
    let error = SourceIndex::load(root.path(), &path, Some(Path::new("sdk")))
        .err()
        .unwrap();
    assert!(
        error.starts_with("cannot read package configuration `"),
        "{error}"
    );
    assert!(error.contains("package_config.json"), "{error}");
}

#[test]
fn sdk_relative_sources_cannot_escape_the_declaring_library_directory() {
    let root = workspace();
    let index = load(root.path());
    let base = index.file(&root.path().join("lib/model.dart")).unwrap();
    let core = index.resolve(&base, "dart:core").unwrap();
    assert_eq!(
        index.resolve(&core, "../outside.dart").unwrap_err(),
        "relative SDK URI escapes its declaring library directory"
    );
}
