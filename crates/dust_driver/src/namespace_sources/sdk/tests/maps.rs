use super::{SdkSources, sdk, write};

#[test]
fn transitive_external_includes_apply_ordered_overrides_and_keep_read_sets() {
    let root = sdk();
    let map = root.path().join("lib/libraries.json");
    write(
        &map,
        r#"{"vm":{"include":[
      {"path":"maps/first.json","target":"first"},
      {"path":"maps/second.json","target":"second"}],
      "libraries":{"core":{"uri":"core/core.dart"}}}}"#,
    );
    write(
        &root.path().join("lib/maps/first.json"),
        r#"{
      "first":{"libraries":{"core":{"uri":"../wrong.dart"},
      "async":{"uri":"../first.dart"}}},
      "unused":{"libraries":{"core":{"uri":"../also_wrong.dart"}}}}"#,
    );
    write(
        &root.path().join("lib/maps/second.json"),
        r#"{
      "second":{"include":[{"target":"base"}],"libraries":{}},
      "base":{"libraries":{"async":{"uri":"../async/async.dart"}}}}"#,
    );
    let sources = SdkSources::load(root.path(), Some(root.path())).unwrap();
    assert_eq!(sources.inputs.len(), 4);
    assert_eq!(
        sources.resolve("core").unwrap(),
        root.path().join("lib/core/core.dart")
    );
    assert_eq!(
        sources.resolve("async").unwrap(),
        root.path().join("lib/async/async.dart")
    );
    assert!(
        sources
            .inputs
            .contains_key(&root.path().join("lib/maps/second.json"))
    );
}

#[test]
fn source_uris_follow_the_sdk_map_location_including_local_external_sources() {
    let root = sdk();
    let map = root.path().join("lib/libraries.json");
    write(
        &map,
        r#"{"vm":{"libraries":{
      "core":{"uri":"../sources/core.dart"},
      "async":{"uri":"async/a%20space.dart"}}}}"#,
    );
    let sources = SdkSources::load(root.path(), Some(root.path())).unwrap();
    assert_eq!(
        sources.resolve("core").unwrap(),
        root.path().join("sources/core.dart")
    );
    assert_eq!(
        sources.resolve("async").unwrap(),
        root.path().join("lib/async/a space.dart")
    );
}

#[test]
fn malformed_and_cyclic_includes_fail_instead_of_becoming_empty_namespaces() {
    for (include, expected) in [
        (r#"false"#, "include must be a list"),
        (r#"[null]"#, "SDK include must name a target"),
        (r#"[{"target":false}]"#, "SDK include must name a target"),
        (r#"[{"target":"missing"}]"#, "missing or invalid SDK target"),
        (r#"[{"target":"vm"}]"#, "cyclic SDK target include"),
        (
            r#"[{"target":"vm","path":"libraries.json"}]"#,
            "cyclic SDK source-map include",
        ),
        (
            r#"[{"target":"base","path":false}]"#,
            "SDK include path must be a URI string",
        ),
        (
            r#"[{"target":"base","path":"https://invalid.test/sdk.json"}]"#,
            "unsupported local source URI scheme",
        ),
        (
            r#"[{"target":"base","path":"missing.json"}]"#,
            "cannot read",
        ),
    ] {
        let root = sdk();
        write(
            &root.path().join("lib/libraries.json"),
            &format!(
                r#"{{"vm":{{"include":{include},"libraries":{{"core":{{"uri":"core/core.dart"}}}}}}}}"#
            ),
        );
        let error = SdkSources::load(root.path(), Some(root.path()))
            .err()
            .unwrap();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn external_map_include_cycles_keep_the_failing_input_path() {
    let root = sdk();
    write(
        &root.path().join("lib/libraries.json"),
        r#"{"vm":{
      "include":[{"target":"base","path":"external.json"}],"libraries":{}}}"#,
    );
    write(
        &root.path().join("lib/external.json"),
        r#"{"base":{
      "include":[{"target":"vm","path":"libraries.json"}],"libraries":{}}}"#,
    );
    let error = SdkSources::load(root.path(), Some(root.path()))
        .err()
        .unwrap();
    assert!(error.contains("cyclic SDK source-map include"));
    assert!(error.contains("lib/libraries.json"));
}

#[test]
fn null_include_is_an_omitted_include() {
    let root = sdk();
    write(
        &root.path().join("lib/libraries.json"),
        r#"{"vm":{"include":null,
      "libraries":{"core":{"uri":"core/core.dart"}}}}"#,
    );
    let sources = SdkSources::load(root.path(), Some(root.path())).unwrap();
    assert_eq!(
        sources.resolve("core").unwrap(),
        root.path().join("lib/core/core.dart")
    );
}

#[cfg(unix)]
#[test]
fn symlink_aliases_cannot_hide_a_source_map_include_cycle() {
    let root = sdk();
    let map = root.path().join("lib/libraries.json");
    write(
        &map,
        r#"{"vm":{"include":[{"path":"alias.json","target":"vm"}],
      "libraries":{"core":{"uri":"core/core.dart"}}}}"#,
    );
    std::os::unix::fs::symlink(&map, root.path().join("lib/alias.json")).unwrap();
    let error = SdkSources::load(root.path(), Some(root.path()))
        .err()
        .unwrap();
    assert!(error.contains("cyclic SDK source-map include"));
}
