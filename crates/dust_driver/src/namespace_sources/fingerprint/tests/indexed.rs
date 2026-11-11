use super::*;

#[test]
fn exact_v2_stream_preserves_cycles_aliases_empty_owners_and_missing_roots() {
    let mut sources = snapshot();
    source_unit(&mut sources, PACKAGE)
        .dependencies
        .insert(CORE.into());
    source_unit(&mut sources, PART)
        .dependencies
        .insert(PACKAGE.into());
    source_unit(&mut sources, GENERATED).generated_owner = Some(String::new());
    let alias = "package:app/µ.dart";
    let value = sources.units.remove(PACKAGE).unwrap();
    sources.units.insert(alias.into(), value);
    source_unit(&mut sources, PART)
        .dependencies
        .insert(alias.into());
    source_unit(&mut sources, "file:///a.dart")
        .dependencies
        .insert(alias.into());
    sources
        .roots
        .insert("missing.dart".into(), "file:///missing-root.dart".into());
    sources.roots.insert("failed.dart".into(), FAILED.into());
    for name in [
        "a.dart",
        "b.dart",
        "unrelated.dart",
        "missing.dart",
        "failed.dart",
        "unknown.dart",
    ] {
        let path = Path::new(name);
        assert_eq!(
            sources.fingerprint(path),
            reference::fingerprint(&sources, path),
            "{name}"
        );
    }
}

#[test]
fn an_unknown_path_does_not_initialize_the_graph_or_unrelated_text_digests() {
    let sources = snapshot();
    let path = Path::new("unknown.dart");
    assert_eq!(
        sources.fingerprint(path),
        reference::fingerprint(&sources, path)
    );
    assert!(sources.fingerprint_index.get().is_none());
    assert!(
        sources
            .units
            .values()
            .filter_map(|unit| unit.as_ref().ok())
            .all(|unit| unit.content_digest.get().is_none())
    );
}

#[test]
fn concurrent_fingerprints_reuse_scratch_across_different_fresh_snapshots() {
    let small = snapshot();
    let mut large = snapshot();
    large.metadata_hash += 1;
    for index in 0..20 {
        let uri = format!("dart:aaa_padding_{index}");
        let (uri, value) = unit(&uri, 200 + index, "", &[], None);
        large.units.insert(uri, value);
    }
    source_unit(&mut large, "file:///b.dart")
        .dependencies
        .insert(PACKAGE.into());
    let graphs = [&small, &large];
    let expected = graphs
        .iter()
        .map(|sources| {
            ["a.dart", "b.dart", "unrelated.dart"]
                .map(|name| reference::fingerprint(sources, Path::new(name)))
        })
        .collect::<Vec<_>>();
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let expected = &expected;
            let graphs = &graphs;
            scope.spawn(move || {
                for _ in 0..3 {
                    for (index, sources) in graphs.iter().enumerate().rev() {
                        for (root, name) in
                            ["a.dart", "b.dart", "unrelated.dart"].iter().enumerate()
                        {
                            assert_eq!(sources.fingerprint(Path::new(name)), expected[index][root]);
                        }
                    }
                }
            });
        }
    });
}
