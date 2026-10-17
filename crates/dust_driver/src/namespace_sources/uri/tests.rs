use super::*;

#[test]
fn uri_paths_preserve_identity_but_normalize_dart_escapes_and_dots() {
    for (path, expected) in [
        ("/a//b", "/a//b"),
        ("/a//", "/a//"),
        ("/a/..", "/"),
        ("/a/./b", "/a/b"),
        ("/../a", "/a"),
        ("../a", "../a"),
        ("/%61/%2e%2e/☃%2f.dart", "/%E2%98%83%2F.dart"),
    ] {
        assert_eq!(normalize(&encode_path(path).unwrap()), expected);
    }
    assert!(encode_path("%GG").is_err());
    assert_eq!(
        relative_path("src/model.dart", "../../x.dart").unwrap(),
        "x.dart"
    );
    assert_eq!(
        relative_path("src/model.dart", "//x.dart").unwrap(),
        "/x.dart"
    );
    assert_eq!(
        file_reference("file:///tmp/a.dart", "b.dart").unwrap(),
        "file:///tmp/b.dart"
    );
    assert_eq!(
        file_reference("file:///tmp/a.dart", "FILE:///tmp//b.dart").unwrap(),
        "file:///tmp//b.dart"
    );
}

#[test]
fn uri_references_preserve_empty_paths_and_absolute_file_identities() {
    for (base, reference, expected) in [
        ("file:///a/model.dart", "", "file:///a/model.dart"),
        ("file:///a/model.dart", "/b/../x.dart", "file:///x.dart"),
        ("file:///a/model.dart", "file:/b/x.dart", "file:///b/x.dart"),
        ("file:///a/model.dart", "file:b/x.dart", "file:///b/x.dart"),
        ("file:///a/model.dart", "../%78.dart", "file:///x.dart"),
        ("file:///a/model.dart", "file:///", "file:///"),
    ] {
        assert_eq!(file_reference(base, reference).unwrap(), expected);
    }
    for reference in [
        "https://invalid.test/x.dart",
        "file://host/x.dart",
        "file://host",
    ] {
        assert!(
            file_reference("file:///a/model.dart", reference).is_err(),
            "{reference}"
        );
    }
    assert_eq!(
        file_reference("not-a-file-base", "x.dart").unwrap_err(),
        "missing file URI base"
    );
    assert_eq!(
        relative_path("model.dart", "other.dart").unwrap(),
        "other.dart"
    );
    assert_eq!(relative_path("model.dart", "").unwrap(), "model.dart");
    assert_eq!(
        relative_path("model.dart", "%GG").unwrap_err(),
        "invalid percent escape in source URI"
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_source_paths_fail_without_lossy_library_identities() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
    let path = Path::new(OsStr::from_bytes(b"/source/\xff.dart"));
    assert_eq!(
        file_uri(path).unwrap_err(),
        "source path is not valid UTF-8"
    );
}
