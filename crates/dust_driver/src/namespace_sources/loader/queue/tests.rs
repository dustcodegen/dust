use super::*;

fn fixture(source: &str) -> (tempfile::TempDir, SourceIndex, Source) {
    let root = tempfile::tempdir().unwrap();
    crate::namespace_sources::test_workspace::prepare(root.path(), root.path(), "example");
    fs::create_dir_all(root.path().join("lib")).unwrap();
    let path = root.path().join("lib/root.dart");
    fs::write(&path, source).unwrap();
    let index = SourceIndex::load(
        root.path(),
        &root.path().join(".dart_tool/package_config.json"),
        Some(&root.path().join(".dust-test-sdk")),
        false,
    )
    .unwrap();
    let source = index.file(&path).unwrap();
    (root, index, source)
}

fn loader<'a>(
    index: &'a SourceIndex,
    potential: &'a BTreeSet<std::path::PathBuf>,
    prepared_roots: PreparedRoots,
) -> Loader<'a> {
    Loader {
        units: BTreeMap::new(),
        revision: 0,
        index,
        queue: BTreeMap::new(),
        prepare_queue: BTreeMap::new(),
        ids: HashMap::new(),
        imports: HashSet::new(),
        part_owners: BTreeMap::new(),
        generated: BTreeMap::new(),
        primary_suffix: ".g.dart",
        potential,
        workers: 2,
        parse_policy: Arc::clone(&prepared_roots.policy),
        prepared_roots,
    }
}

#[test]
fn cached_root_exclusion_is_uri_specific_and_canonical_pops_remove_hints() {
    let (root, index, source) = fixture("class Root {}");
    let snapshots = BTreeMap::from([(
        source.path.clone(),
        Arc::<str>::from(fs::read_to_string(root.path().join("lib/root.dart")).unwrap()),
    )]);
    let (_, prepared) = prepare_roots(
        &index,
        std::slice::from_ref(&source),
        2,
        &[],
        &snapshots,
        None,
        || (),
    );
    let potential = BTreeSet::new();
    let mut loader = loader(&index, &potential, prepared);
    let mut alias = source.clone();
    alias.uri = "file:///alias/root.dart".into();
    loader.enqueue(source.clone());
    loader.enqueue(alias.clone());
    assert_eq!(loader.queue.len(), 2);
    assert_eq!(
        loader.prepare_queue.keys().collect::<Vec<_>>(),
        [&alias.uri]
    );
    assert_eq!(loader.pop_source().unwrap().0, alias.uri);
    assert!(loader.prepare_queue.is_empty());
    assert_eq!(loader.pop_source().unwrap().0, source.uri);
    assert!(loader.pop_source().is_none());
}

#[test]
fn late_generated_ownership_removes_only_the_preparation_hint() {
    let (root, index, source) = fixture("part 'root.g.dart'; class Root {}");
    fs::write(
        root.path().join("lib/root.g.dart"),
        dust_plugin_api::GENERATED_HEADER,
    )
    .unwrap();
    let target = index.resolve(&source, "root.g.dart").unwrap();
    let potential = BTreeSet::new();
    let prepared = PreparedRoots::empty(std::slice::from_ref(&source), &[], &BTreeMap::new());
    let mut loader = loader(&index, &potential, prepared);
    loader.enqueue(target.clone());
    assert!(loader.prepare_queue.contains_key(&target.uri));
    loader.imports.insert(source.uri.clone());
    let parsed = loader.read(source.clone(), FileId::new(1), None).unwrap();
    loader.units.insert(source.uri.clone(), Ok(parsed));
    loader.expand(&source.uri).unwrap();
    assert!(loader.queue.contains_key(&target.uri));
    assert!(!loader.prepare_queue.contains_key(&target.uri));
    assert_eq!(loader.generated.get(&target.uri), Some(&source.uri));
    let (_, generated) = std::iter::from_fn(|| loader.pop_source())
        .find(|(uri, _)| uri == &target.uri)
        .unwrap();
    let parsed = loader.read(generated, FileId::new(2), None).unwrap();
    assert_eq!(parsed.generated_owner, Some(source.uri));
    assert!(parsed.text.is_empty());
}

#[test]
fn repeated_loaded_root_references_preserve_serial_ids_and_do_not_return_to_either_queue() {
    let (root, index, source) = fixture("import 'other.dart';");
    fs::write(
        root.path().join("lib/other.dart"),
        "import 'root.dart'; typedef Other = Root; class Root {}",
    )
    .unwrap();
    let potential = BTreeSet::new();
    let mut previous = None;
    for workers in [1, 3] {
        let prepared = PreparedRoots::empty(std::slice::from_ref(&source), &[], &BTreeMap::new());
        let loader = Loader::load(
            &index,
            std::slice::from_ref(&source),
            ".g.dart",
            &potential,
            workers,
            prepared,
        )
        .unwrap();
        assert!(loader.queue.is_empty());
        assert!(loader.prepare_queue.is_empty());
        assert_eq!(loader.ids.len(), loader.units.len());
        let actual = loader
            .units
            .iter()
            .map(|(uri, unit)| {
                let unit = unit.as_ref().unwrap();
                (
                    uri.clone(),
                    (
                        unit.file_id,
                        unit.text.clone(),
                        unit.dependencies.clone(),
                        unit.diagnostics.clone(),
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if let Some(expected) = previous.replace(actual.clone()) {
            assert_eq!(actual, expected);
        }
    }
}
