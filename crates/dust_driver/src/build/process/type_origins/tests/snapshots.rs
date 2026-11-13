use dust_ir::SpanIr;
use dust_plugin_api::{LibraryAnalysisSnapshot, TypeOriginRequest, WorkspaceAnalysisBuilder};
use dust_text::FileId;
use serde_json::json;

use super::support::*;

#[test]
fn empty_and_http_only_snapshots_keep_the_exact_legacy_json_shape() {
    let empty = WorkspaceAnalysisBuilder::default().snapshot();
    assert_eq!(
        serde_json::to_string(&empty).unwrap(),
        r#"{"string_sets":{}}"#
    );
    let mut http = WorkspaceAnalysisBuilder::default();
    http.add_string_set_value("key", "val");
    let json = r#"{"string_sets":{"key":["val"]}}"#;
    assert_eq!(serde_json::to_string(&http.snapshot()).unwrap(), json);
    let restored: LibraryAnalysisSnapshot = serde_json::from_str(json).unwrap();
    assert_eq!(restored, http.snapshot());
    let mut builder = WorkspaceAnalysisBuilder::default();
    builder.merge_snapshot(&restored);
    assert_eq!(builder.build().content_hash(), http.build().content_hash());
    let legacy_empty: LibraryAnalysisSnapshot =
        serde_json::from_str(r#"{"string_sets":{}}"#).unwrap();
    assert!(legacy_empty.is_empty());
}

#[test]
fn typed_snapshot_round_trip_retains_source_identity_after_file_ids_change() {
    let source =
        format!("import '{BARREL}' as ws; void handle(ws.WebSocketConnection socket) {{}}");
    let (graph, file, request) = fixture(
        &source,
        "ws.WebSocketConnection",
        "ws.WebSocketConnection socket",
    );
    let analysis = collect_request(&graph, &file, &request);
    let snapshot = analysis.snapshot();
    let encoded = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(
        encoded,
        json!({
            "string_sets": {},
            "type_origins": [{
                "source_path": file.source_path,
                "name": request.name,
                "start": request.span.range.start().to_u32(),
                "end": request.span.range.end().to_u32(),
                "library_uri": CORE,
                "declaration_name": TYPE,
            }],
        })
    );
    let restored: LibraryAnalysisSnapshot = serde_json::from_value(encoded).unwrap();
    let mut builder = WorkspaceAnalysisBuilder::default();
    builder.merge_snapshot(&restored);
    assert_eq!(builder.snapshot(), snapshot);
    assert_eq!(
        builder.clone().build().content_hash(),
        analysis.build().content_hash()
    );
    let reassigned = TypeOriginRequest {
        name: request.name,
        span: SpanIr::new(FileId::new(900), request.span.range),
    };
    assert!(builder.has_type_origin(&file.source_path, &reassigned, CORE, TYPE));
    assert!(!builder.has_type_origin("/workspace/lib/other.dart", &reassigned, CORE, TYPE));
    assert!(
        builder
            .build()
            .has_type_origin(&file.source_path, &reassigned, CORE, TYPE)
    );
}

#[test]
fn serialized_conflicting_origins_do_not_become_trusted_on_restore() {
    let source = format!("import '{BARREL}'; void handle(WebSocketConnection socket) {{}}");
    let (graph, file, request) = fixture(&source, TYPE, "WebSocketConnection socket");
    let mut analysis = collect_request(&graph, &file, &request);
    analysis.add_type_origin(&file.source_path, &request, FOREIGN, TYPE);
    let encoded = serde_json::to_vec(&analysis.snapshot()).unwrap();
    let restored: LibraryAnalysisSnapshot = serde_json::from_slice(&encoded).unwrap();
    let mut builder = WorkspaceAnalysisBuilder::default();
    builder.merge_snapshot(&restored);
    assert!(!builder.has_type_origin(&file.source_path, &request, CORE, TYPE));
    assert!(
        !builder
            .build()
            .has_type_origin(&file.source_path, &request, FOREIGN, TYPE)
    );
}
