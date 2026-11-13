use super::*;
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_text::SourceText;

/// Real parser facts exercise both positive and failing memo entries.
fn add(graph: &mut NamespaceGraph, id: u32, uri: &str, source: &str) {
    let parsed = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(FileId::new(id), source),
        ParseOptions::default(),
    );
    assert!(parsed.diagnostics.is_empty());
    graph
        .insert_library(uri, FileId::new(id), &parsed.library, |_, uri| {
            Ok(uri.to_owned())
        })
        .unwrap();
}

#[test]
fn repeated_queries_share_worlds_and_successful_insertion_invalidates_them() {
    let mut graph = NamespaceGraph::new();
    let root = "package:app/main.dart";
    add(
        &mut graph,
        1,
        "package:other/marker.dart",
        "class Marker {}",
    );
    add(
        &mut graph,
        2,
        root,
        "import 'package:other/marker.dart' as p;",
    );
    for prefix in [None, Some("p"), Some("absent")] {
        let expected = graph.compute_lookup(root, prefix, "Marker");
        let actual = graph
            .lookup(root, prefix, "Marker")
            .map(|worlds| worlds.as_ref().clone());
        assert_eq!(actual, expected);
    }
    let first = graph.lookup(root, Some("p"), "Marker").unwrap();
    assert!(Arc::ptr_eq(
        &first,
        &graph.lookup(root, Some("p"), "Marker").unwrap()
    ));
    add(&mut graph, 3, "package:extra/extra.dart", "class Extra {}");
    let inserted = graph.lookup(root, Some("p"), "Marker").unwrap();
    assert_eq!(inserted, first);
    assert!(!Arc::ptr_eq(&inserted, &first));
}

#[test]
fn concurrent_queries_keep_library_slots_distinct_and_parts_share_the_owner_slot() {
    let mut graph = NamespaceGraph::new();
    let origin = "package:other/marker.dart";
    let part = "package:app/root0_part.dart";
    add(&mut graph, 1000, origin, "class Marker {}");
    let roots = (0..32)
        .map(|id| {
            let uri = format!("package:app/root{id}.dart");
            let parts = if id == 0 {
                format!("part '{part}';")
            } else {
                String::new()
            };
            add(
                &mut graph,
                id,
                &uri,
                &format!("import '{origin}' as p; {parts}"),
            );
            uri
        })
        .collect::<Vec<_>>();
    add(
        &mut graph,
        1001,
        part,
        &format!("part of '{}'; class InPart {{}}", roots[0]),
    );
    let expected = graph
        .compute_lookup(&roots[0], Some("p"), "Marker")
        .unwrap();
    let barrier = std::sync::Barrier::new(8);
    let observed = std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    let worlds = roots
                        .iter()
                        .map(|root| {
                            let first = graph.lookup(root, Some("p"), "Marker").unwrap();
                            assert_eq!(*first, expected);
                            assert!(Arc::ptr_eq(
                                &first,
                                &graph.lookup(root, Some("p"), "Marker").unwrap()
                            ));
                            first
                        })
                        .collect::<Vec<_>>();
                    assert!(Arc::ptr_eq(
                        &worlds[0],
                        &graph.lookup(part, Some("p"), "Marker").unwrap()
                    ));
                    let local = graph.lookup(&roots[0], None, "InPart").unwrap();
                    assert!(Arc::ptr_eq(
                        &local,
                        &graph.lookup(part, None, "InPart").unwrap()
                    ));
                    assert!(local.iter().any(|world| {
                        world.declarations.iter().any(|declaration| {
                            declaration.file_id == FileId::new(1001) && declaration.uri == roots[0]
                        })
                    }));
                    worlds
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    for reader in &observed {
        for (first, concurrent) in observed[0].iter().zip(reader) {
            assert!(Arc::ptr_eq(first, concurrent));
        }
        // Distinct roots remain distinct for both different and colliding shards.
        assert!(!Arc::ptr_eq(&reader[0], &reader[1]));
        assert!(!Arc::ptr_eq(&reader[0], &reader[16]));
    }
    add(
        &mut graph,
        1002,
        "package:extra/extra.dart",
        "class Extra {}",
    );
    for (root, before) in roots.iter().zip(&observed[0]) {
        let after = graph.lookup(root, Some("p"), "Marker").unwrap();
        assert_eq!(after, *before);
        assert!(!Arc::ptr_eq(&after, before));
    }
}
