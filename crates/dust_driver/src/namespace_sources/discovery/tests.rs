use super::*;

#[test]
fn source_scan_keeps_foreign_outputs_and_sorted_handwritten_paths() {
    let root = tempfile::tempdir().unwrap();
    let lib = root.path().join("lib");
    fs::create_dir_all(lib.join("nested")).unwrap();
    fs::write(lib.join("z.dart"), "class Z {} { // parser owns validation").unwrap();
    fs::write(lib.join("nested/a.dart"), "class A {}").unwrap();
    fs::write(
        lib.join("foreign.g.dart"),
        "// Foreign generator\nclass Foreign {}",
    )
    .unwrap();
    fs::write(lib.join("owned.g.dart"), dust_plugin_api::GENERATED_HEADER).unwrap();
    fs::write(lib.join("ignore.txt"), "text").unwrap();
    let mut sources = BTreeMap::new();
    collect_sources(&lib, &mut sources).unwrap();
    assert_eq!(
        sources.keys().cloned().collect::<Vec<_>>(),
        [
            lib.join("foreign.g.dart"),
            lib.join("nested/a.dart"),
            lib.join("z.dart")
        ]
    );
    assert_eq!(sources[&lib.join("nested/a.dart")].as_ref(), "class A {}");
    assert_eq!(
        sources[&lib.join("z.dart")].as_ref(),
        "class Z {} { // parser owns validation"
    );
    assert_eq!(
        sources[&lib.join("foreign.g.dart")].as_ref(),
        "// Foreign generator\nclass Foreign {}"
    );
    collect_sources(&root.path().join("absent"), &mut sources).unwrap();
    fs::write(lib.join("broken.dart"), [0xff]).unwrap();
    assert!(
        collect_sources(&lib, &mut BTreeMap::new())
            .unwrap_err()
            .message
            .contains("broken.dart")
    );
}

#[test]
fn parallel_source_reads_return_the_first_sorted_read_failure() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("z.dart"), [0xff]).unwrap();
    fs::write(root.path().join("a.dart"), [0xff]).unwrap();
    let error = collect_sources(root.path(), &mut BTreeMap::new()).unwrap_err();
    assert!(error.message.contains("a.dart"), "{error:?}");
}

#[test]
fn relative_keys_preserve_full_path_order_across_nested_prefixes_and_unicode() {
    let root = tempfile::tempdir().unwrap();
    let lib = root.path().join("long common directory").join("lib");
    let names = [
        "flat.dart",
        "flat/nested.dart",
        "flat.a.dart",
        "flat space.dart",
        ".dot.dart",
        "éclair.dart",
        "資料.dart",
        "sub/é.dart",
        "sub/a b.dart",
        "sub.deep/a.dart",
        "sub/deeper/x.dart",
        "subx.dart",
        "sub/x.dart",
    ];
    let mut paths = names
        .iter()
        .rev()
        .map(|name| lib.join(name))
        .collect::<Vec<_>>();
    for path in &paths {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "class Model {}").unwrap();
    }
    let mut expected = paths.clone();
    expected.sort();
    sort_paths(&lib, &mut paths);
    assert_eq!(paths, expected);
    let mut sources = BTreeMap::new();
    collect_sources(&lib, &mut sources).unwrap();
    assert_eq!(
        sources.keys().collect::<Vec<_>>(),
        expected.iter().collect::<Vec<_>>()
    );
}

#[test]
fn successful_bulk_collection_overwrites_matching_entries_and_preserves_other_sources() {
    let root = tempfile::tempdir().unwrap();
    let updated = root.path().join("a.dart");
    let added = root.path().join("z.dart");
    let outside = root.path().join("../outside.dart");
    let original_spelling = root.path().join(".").join("a.dart");
    assert_eq!(original_spelling, updated);
    assert_ne!(original_spelling.as_os_str(), updated.as_os_str());
    fs::write(&updated, "class Updated {}").unwrap();
    fs::write(&added, "class Added {}").unwrap();
    let mut sources = BTreeMap::from([
        (original_spelling.clone(), Arc::<str>::from("class Old {}")),
        (outside.clone(), Arc::<str>::from("class Retained {}")),
    ]);
    collect_sources(root.path(), &mut sources).unwrap();
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[&updated].as_ref(), "class Updated {}");
    assert_eq!(sources[&added].as_ref(), "class Added {}");
    assert_eq!(sources[&outside].as_ref(), "class Retained {}");
    assert_eq!(
        sources.get_key_value(&updated).unwrap().0.as_os_str(),
        original_spelling.as_os_str()
    );
}

#[test]
fn sorted_read_failure_keeps_exact_successful_prefix_overwrites() {
    let root = tempfile::tempdir().unwrap();
    let first = root.path().join("a.dart");
    let bad = root.path().join("b.dart");
    let later = root.path().join("c.dart");
    fs::write(&first, "class Updated {}").unwrap();
    fs::write(&bad, [0xff]).unwrap();
    fs::write(&later, "class Later {}").unwrap();
    let mut sources = BTreeMap::from([
        (first.clone(), Arc::<str>::from("class Old {}")),
        (later.clone(), Arc::<str>::from("class Retained {}")),
    ]);
    let error = collect_sources(root.path(), &mut sources).unwrap_err();
    assert!(error.message.contains("b.dart"));
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[&first].as_ref(), "class Updated {}");
    assert_eq!(sources[&later].as_ref(), "class Retained {}");
    assert!(!sources.contains_key(&bad));
}

#[test]
fn dynamic_reads_keep_varied_sources_complete_and_sorted_beside_generated_files() {
    let root = tempfile::tempdir().unwrap();
    let expected = (0..24)
        .rev()
        .map(|index| {
            let text = format!(
                "// {}\nclass Item{index} {{}}",
                "varied read contents ".repeat((index % 5) * 100)
            );
            let source = root.path().join(format!("{index:02}.dart"));
            fs::write(&source, &text).unwrap();
            fs::write(
                root.path().join(format!("{index:02}.g.dart")),
                dust_plugin_api::GENERATED_HEADER,
            )
            .unwrap();
            (source, Arc::<str>::from(text))
        })
        .collect::<BTreeMap<_, _>>();
    let mut snapshots = BTreeMap::new();
    collect_sources(root.path(), &mut snapshots).unwrap();
    assert_eq!(snapshots, expected);
}
