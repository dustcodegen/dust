use std::{fs, path::Path};

use tempfile::{TempDir, tempdir};

use super::{SdkSources, discover, sdk_language_version};

mod maps;

#[test]
fn sdk_version_is_not_parsed_as_a_free_form_constraint() {
    for version in ["3.13.5", "3.13.0-92.0.dev", "3.13.0+build42", "3.13.5\n"] {
        assert_eq!(sdk_language_version(version).unwrap().to_string(), "3.13");
    }
    for version in [
        "",
        "3",
        "3.13",
        "3.13bad.0",
        "^3.13.5",
        "03.13.5",
        "3.013.5",
        "3.13.05",
        "3.13.5.1",
        "65536.0.0",
        "3.65536.0",
        "3.13.4294967296",
    ] {
        assert_eq!(sdk_language_version(version), None, "{version}");
    }
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn sdk() -> TempDir {
    let root = tempdir().unwrap();
    write(&root.path().join("version"), "3.13.5\n");
    write(
        &root.path().join("lib/libraries.json"),
        r#"{
          "comment:0":"synthetic SDK; never needs Dart installed",
          "base":{"libraries":{"core":{"uri":"core/core.dart"}}},
          "vm":{"include":[{"target":"base"}],"libraries":{
            "core":{"uri":"core/core.dart"},"_private":{"uri":"vm/private.dart"}}},
          "web":{"libraries":{"_private":{"uri":"web/private.dart"}}}
        }"#,
    );
    root
}

#[test]
fn explicit_sdk_loads_sources_metadata_and_version_without_executing_dart() {
    let root = sdk();
    let sources = SdkSources::load(root.path(), Some(Path::new("."))).unwrap();
    assert_eq!(sources.language_version.to_string(), "3.13");
    assert_eq!(sources.inputs.len(), 2);
    assert_eq!(sources.inputs[&root.path().join("version")], "3.13.5\n");
    assert_eq!(
        sources.resolve("core").unwrap(),
        root.path().join("lib/core/core.dart")
    );
    assert_eq!(
        sources.resolve("missing").unwrap_err(),
        "SDK source map has no `dart:missing` library"
    );
    assert_eq!(
        sources.resolve("_private").unwrap_err(),
        "SDK library `dart:_private` has target-dependent source identities"
    );
}

#[test]
fn explicit_invalid_sdk_does_not_fall_back_to_path() {
    let root = tempdir().unwrap();
    let error = SdkSources::load(root.path(), Some(Path::new("absent")))
        .err()
        .unwrap();
    assert!(error.starts_with("cannot load Dart SDK sources at `"));
    assert!(error.contains("absent/version"));
}

#[test]
fn sdk_metadata_failures_are_retained() {
    for (version, map, expected) in [
        ("invalid", "{}", "invalid SDK version"),
        ("3.13.5", "[", "invalid `"),
        ("3.13.5", "[]", "SDK source map must be an object"),
        ("3.13.5", r#"{"vm":{}}"#, "has no library map"),
        (
            "3.13.5",
            r#"{"vm":{"libraries":{"core":{}}}}"#,
            "has no source URI",
        ),
        (
            "3.13.5",
            r#"{"none":{"libraries":{}}}"#,
            "does not declare `dart:core`",
        ),
        (
            "3.13.5",
            r#"{"vm":{"libraries":{"core":{"uri":"https://invalid.test/core.dart"}}}}"#,
            "unsupported local source URI scheme",
        ),
    ] {
        let root = tempdir().unwrap();
        write(&root.path().join("version"), version);
        write(&root.path().join("lib/libraries.json"), map);
        let error = SdkSources::load(root.path(), Some(root.path()))
            .err()
            .unwrap();
        assert!(error.contains(expected), "{error}");
    }
    let root = sdk();
    fs::remove_file(root.path().join("lib/libraries.json")).unwrap();
    let error = SdkSources::load(root.path(), Some(root.path()))
        .err()
        .unwrap();
    assert!(error.contains("lib/libraries.json"));
}

fn executable(path: &Path) {
    write(
        path,
        "not an executable program; discovery must not execute this",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn command_name() -> &'static str {
    if cfg!(windows) { "dart.exe" } else { "dart" }
}

#[test]
fn discovers_standalone_and_flutter_sources_from_active_path() {
    for flutter in [false, true] {
        let root = tempdir().unwrap();
        let bin = root.path().join("bin");
        executable(&bin.join(command_name()));
        let expected = if flutter {
            bin.join("cache/dart-sdk")
        } else {
            root.path().to_path_buf()
        };
        write(&expected.join("lib/libraries.json"), "{}");
        let selected = discover(root.path(), Some(bin.as_os_str())).unwrap();
        assert_eq!(selected, fs::canonicalize(expected).unwrap());
    }
}

#[test]
fn missing_or_opaque_active_dart_is_diagnostic_not_a_different_sdk() {
    let root = tempdir().unwrap();
    assert_eq!(
        discover(root.path(), None).unwrap_err(),
        "cannot discover Dart SDK sources: no active Dart on PATH; supply sdk_source_root"
    );
    let first = root.path().join("first");
    executable(&first.join(command_name()));
    let second = sdk();
    executable(&second.path().join("bin").join(command_name()));
    let path = std::env::join_paths([first, second.path().join("bin")]).unwrap();
    let error = discover(root.path(), Some(&path)).unwrap_err();
    assert!(error.contains("cannot discover SDK sources beside active Dart"));
    assert!(error.contains("first"));
}

#[test]
fn path_search_skips_missing_entries_directories_and_non_executable_files() {
    let root = sdk();
    let missing = root.path().join("missing");
    let directory = root.path().join("directory");
    fs::create_dir_all(directory.join(command_name())).unwrap();
    let bin = root.path().join("bin");
    executable(&bin.join(command_name()));
    let mut paths = vec![missing, directory];
    #[cfg(unix)]
    {
        let inert = root.path().join("inert");
        write(&inert.join(command_name()), "not executable");
        paths.push(inert);
    }
    paths.push(bin);
    let path = std::env::join_paths(paths).unwrap();
    assert_eq!(
        discover(root.path(), Some(&path)).unwrap(),
        fs::canonicalize(root.path()).unwrap()
    );
}

#[cfg(unix)]
#[test]
fn resolves_active_dart_symlink_before_finding_source_root() {
    let root = sdk();
    let dart = root.path().join("bin/dart");
    executable(&dart);
    let link_dir = root.path().join("commands");
    fs::create_dir_all(&link_dir).unwrap();
    std::os::unix::fs::symlink(&dart, link_dir.join("dart")).unwrap();
    assert_eq!(
        discover(root.path(), Some(link_dir.as_os_str())).unwrap(),
        fs::canonicalize(root.path()).unwrap()
    );
}
