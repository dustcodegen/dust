use super::*;

#[test]
fn configured_workspace_census_bounds_retention_without_growing_active_work() {
    let root = tempfile::tempdir().unwrap();
    for (path, text) in [
        (
            "package_config.json",
            r#"{"configVersion":2,"packages":[]}"#,
        ),
        ("sdk/version", "3.6.0"),
        (
            "sdk/lib/libraries.json",
            r#"{"vm":{"libraries":{"core":{"uri":"core/core.dart"}}}}"#,
        ),
    ] {
        let path = root.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let index = SourceIndex::load(
        root.path(),
        &root.path().join("package_config.json"),
        Some(std::path::Path::new("sdk")),
        false,
    )
    .unwrap();
    let source = |number| Source {
        uri: format!("package:example/source{number:02}.dart"),
        path: root.path().join(format!("missing{number}.dart")),
        language_version: DartLanguageVersion::latest_known(),
    };
    let roots = (0..40).map(source).collect::<Vec<_>>();
    let policy = ParsePolicy::new(&roots, &[], &BTreeMap::new());
    let queue = (0..45)
        .map(source)
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    let potential = BTreeSet::new();
    ParseAhead::run(
        2,
        &index,
        &potential,
        &policy,
        HashSet::new(),
        KnownSources::default(),
        |ahead| {
            let ahead = ahead.expect("test parser workers start");
            assert_eq!(ahead.limit, roots.len());
            assert!(ahead.active_limit <= 4);
            while ahead.pending.len() < ahead.limit {
                ahead.fill(&queue, &BTreeMap::new());
                if ahead.pending.len() < ahead.limit {
                    let uri = ahead
                        .completed
                        .recv_timeout(std::time::Duration::from_secs(5))
                        .unwrap();
                    ahead.harvest(&uri, &BTreeMap::new());
                }
                assert!(ahead.active.load(Relaxed) <= ahead.active_limit);
                assert!(ahead.pending.len() <= ahead.limit);
            }
            ahead.fill(&queue, &BTreeMap::new());
            assert_eq!(ahead.pending.len(), roots.len());
            assert!(ahead.known.len() < queue.len());
        },
    );
    let known = KnownSources {
        loaded: HashSet::new(),
        cached: roots.iter().map(|source| source.uri.clone()).collect(),
    };
    let cached_queue = roots
        .iter()
        .cloned()
        .map(|source| (source.uri.clone(), source))
        .collect::<BTreeMap<_, _>>();
    ParseAhead::run(
        2,
        &index,
        &potential,
        &policy,
        HashSet::new(),
        known,
        |ahead| {
            let ahead = ahead.expect("test parser workers start");
            assert_eq!(ahead.cached_roots.len(), roots.len());
            ahead
                .speculative
                .insert(roots[0].uri.clone(), roots[0].clone());
            ahead.cursor = Some("z-cursor".into());
            ahead.fill(&cached_queue, &BTreeMap::new());
            assert!(ahead.pending.is_empty());
            assert!(ahead.speculative.is_empty());
            assert_eq!(ahead.active.load(Relaxed), 0);
            assert_eq!(ahead.cursor.as_deref(), Some("z-cursor"));
        },
    );
}
