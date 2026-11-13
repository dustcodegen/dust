use std::collections::BTreeMap;

use dust_ir::SpanIr;
use dust_text::{FileId, TextRange};
use serde::{Deserialize, de::value::MapDeserializer};

use crate::{LibraryAnalysisSnapshot, TypeOriginRequest, WorkspaceAnalysisBuilder};

const SOURCE: &str = "lib/routes.dart";
const LIBRARY: &str = "package:dust_dart/ws.dart";
const DECLARATION: &str = "WebSocketConnection";

fn request(file_id: u32, name: &str, start: u32, end: u32) -> TypeOriginRequest {
    TypeOriginRequest {
        name: name.to_owned(),
        span: SpanIr::new(FileId::new(file_id), TextRange::new(start, end)),
    }
}

fn recorded(request: &TypeOriginRequest) -> WorkspaceAnalysisBuilder {
    let mut builder = WorkspaceAnalysisBuilder::default();
    builder.add_type_origin(SOURCE, request, LIBRARY, DECLARATION);
    builder
}

#[test]
fn ownership_requires_source_name_and_exact_byte_bounds() {
    let reference = request(1, "socket.WebSocketConnection", 10, 30);
    let builder = recorded(&reference);
    assert!(builder.has_type_origin(SOURCE, &reference, LIBRARY, DECLARATION));
    assert!(!builder.has_type_origin("lib/other.dart", &reference, LIBRARY, DECLARATION));
    assert!(!builder.has_type_origin(SOURCE, &reference, "package:foreign/ws.dart", DECLARATION));
    assert!(!builder.has_type_origin(SOURCE, &reference, LIBRARY, "ForeignSocket"));
    for different in [
        request(1, "WebSocketConnection", 10, 30),
        request(1, "socket.WebSocketConnection", 11, 30),
        request(1, "socket.WebSocketConnection", 10, 31),
    ] {
        assert!(!builder.has_type_origin(SOURCE, &different, LIBRARY, DECLARATION));
    }
    let analysis = builder.build();
    assert!(analysis.has_type_origin(SOURCE, &reference, LIBRARY, DECLARATION));
    assert!(!analysis.has_type_origin("lib/other.dart", &reference, LIBRARY, DECLARATION));
}

#[test]
fn source_identity_survives_file_id_reassignment_and_duplicate_proofs() {
    let original = request(1, "socket.WebSocketConnection", 10, 30);
    let reassigned = request(99, "socket.WebSocketConnection", 10, 30);
    let mut builder = recorded(&original);
    let original_snapshot = builder.snapshot();
    let original_hash = builder.clone().build().content_hash();
    builder.add_type_origin(SOURCE, &reassigned, LIBRARY, DECLARATION);
    assert_eq!(builder.snapshot(), original_snapshot);
    assert_eq!(builder.clone().build().content_hash(), original_hash);
    assert!(builder.has_type_origin(SOURCE, &reassigned, LIBRARY, DECLARATION));

    let mut restored = WorkspaceAnalysisBuilder::default();
    restored.merge_snapshot(&original_snapshot);
    assert!(
        restored
            .build()
            .has_type_origin(SOURCE, &reassigned, LIBRARY, DECLARATION)
    );
}

#[test]
fn builder_and_snapshot_merges_preserve_type_and_string_facts() {
    let first = request(1, "WebSocketConnection", 10, 30);
    let second = request(2, "other.WebSocketConnection", 40, 60);
    let mut builder = recorded(&first);
    builder.add_string_set_value("feature", "http");
    let mut other = recorded(&second);
    other.add_string_set_value("feature", "ws");
    builder.merge(other);
    assert!(!builder.snapshot().is_empty());

    let mut restored = WorkspaceAnalysisBuilder::default();
    restored.merge_snapshot(&builder.snapshot());
    restored.merge_snapshot(&builder.snapshot());
    assert_eq!(restored.snapshot(), builder.snapshot());
    let analysis = restored.build();
    assert_eq!(
        analysis.string_set("feature"),
        Some(&["http".to_owned(), "ws".to_owned()][..])
    );
    for reference in [first, second] {
        assert!(analysis.has_type_origin(SOURCE, &reference, LIBRARY, DECLARATION));
    }
}

#[test]
fn conflicting_candidates_fail_closed_through_both_merge_paths() {
    let reference = request(1, "WebSocketConnection", 10, 30);
    let mut foreign = WorkspaceAnalysisBuilder::default();
    foreign.add_type_origin(SOURCE, &reference, "package:foreign/ws.dart", DECLARATION);
    for merge_snapshot in [false, true] {
        let mut builder = recorded(&reference);
        if merge_snapshot {
            builder.merge_snapshot(&foreign.snapshot());
        } else {
            builder.merge(foreign.clone());
        }
        assert!(!builder.has_type_origin(SOURCE, &reference, LIBRARY, DECLARATION));
        assert!(!builder.has_type_origin(
            SOURCE,
            &reference,
            "package:foreign/ws.dart",
            DECLARATION
        ));
        let mut restored = WorkspaceAnalysisBuilder::default();
        restored.merge_snapshot(&builder.snapshot());
        assert!(
            !restored
                .build()
                .has_type_origin(SOURCE, &reference, LIBRARY, DECLARATION)
        );
    }

    let mut renamed = recorded(&reference);
    renamed.add_type_origin(SOURCE, &reference, LIBRARY, "AnotherDeclaration");
    assert!(
        !renamed
            .build()
            .has_type_origin(SOURCE, &reference, LIBRARY, DECLARATION)
    );
}

#[test]
fn typed_hash_and_snapshot_are_stable_across_insertion_order() {
    let first = request(1, "a.WebSocketConnection", 10, 30);
    let second = request(2, "z.WebSocketConnection", 40, 60);
    let mut left = recorded(&first);
    left.add_type_origin(SOURCE, &second, LIBRARY, DECLARATION);
    let mut right = recorded(&second);
    right.add_type_origin(SOURCE, &first, LIBRARY, DECLARATION);
    assert_eq!(left.snapshot(), right.snapshot());
    assert_eq!(left.build().content_hash(), right.build().content_hash());
}

#[test]
fn typed_hash_includes_every_identity_component_and_conflicts() {
    let reference = request(1, "WebSocketConnection", 10, 30);
    let initial = recorded(&reference).build().content_hash();
    for (source, name, start, end, library, declaration) in [
        (
            "lib/other.dart",
            "WebSocketConnection",
            10,
            30,
            LIBRARY,
            DECLARATION,
        ),
        (
            SOURCE,
            "other.WebSocketConnection",
            10,
            30,
            LIBRARY,
            DECLARATION,
        ),
        (SOURCE, "WebSocketConnection", 11, 30, LIBRARY, DECLARATION),
        (SOURCE, "WebSocketConnection", 10, 31, LIBRARY, DECLARATION),
        (
            SOURCE,
            "WebSocketConnection",
            10,
            30,
            "package:other/ws.dart",
            DECLARATION,
        ),
        (
            SOURCE,
            "WebSocketConnection",
            10,
            30,
            LIBRARY,
            "AnotherDeclaration",
        ),
    ] {
        let mut builder = WorkspaceAnalysisBuilder::default();
        builder.add_type_origin(source, &request(7, name, start, end), library, declaration);
        assert_ne!(builder.build().content_hash(), initial);
    }
    let mut conflicting = recorded(&reference);
    conflicting.add_type_origin(SOURCE, &reference, LIBRARY, "AnotherDeclaration");
    assert_ne!(conflicting.build().content_hash(), initial);
    assert_ne!(
        WorkspaceAnalysisBuilder::default().build().content_hash(),
        initial
    );
}

#[test]
fn empty_and_legacy_snapshots_retain_old_hashes_and_no_type_proofs() {
    let empty = WorkspaceAnalysisBuilder::default();
    assert!(empty.snapshot().is_empty());
    assert_eq!(empty.build().content_hash(), 1469598103934665603);

    let legacy = BTreeMap::from([(
        "string_sets",
        BTreeMap::from([("key".to_owned(), vec!["val".to_owned()])]),
    )]);
    let snapshot = LibraryAnalysisSnapshot::deserialize(MapDeserializer::<
        _,
        serde::de::value::Error,
    >::new(legacy.into_iter()))
    .unwrap();
    let mut restored = WorkspaceAnalysisBuilder::default();
    restored.merge_snapshot(&snapshot);
    assert!(!restored.has_type_origin(
        SOURCE,
        &request(1, "WebSocketConnection", 10, 30),
        LIBRARY,
        DECLARATION
    ));
    assert_eq!(restored.snapshot(), snapshot);
    assert_eq!(restored.build().content_hash(), 1569222009174286751);
}
