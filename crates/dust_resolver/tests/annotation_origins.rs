//! Declaration metadata cannot replace ownership, roles or intrinsic identity.

use dust_ir::AnnotationOrigin;
use dust_resolver::SymbolCatalog;

#[test]
fn origins_do_not_guess_from_a_canonical_package_or_alias() {
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("Marker", "semantic::Marker");
    assert_eq!(catalog.annotation_origin("semantic::Marker"), None);
    assert_eq!(
        catalog
            .annotation_owners("package:actual/src/decl.dart", "Actual")
            .count(),
        0
    );
    let origin =
        AnnotationOrigin::new("semantic::Marker", "package:actual/src/decl.dart", "Actual");
    catalog.register_annotation_origin(origin).unwrap();
    catalog.register_annotation_origin(origin).unwrap();
    assert_eq!(catalog.annotation_origin("semantic::Marker"), Some(&origin));
    assert_eq!(catalog.annotation_origin("Marker"), None);
    assert_eq!(catalog.annotation_origin("actual::Actual"), None);
    assert_eq!(
        catalog
            .annotation_owners(origin.library_uri, origin.declaration)
            .collect::<Vec<_>>(),
        [origin.symbol]
    );
    assert_eq!(
        catalog
            .annotation_owners(origin.library_uri, "Marker")
            .count(),
        0
    );
    assert_eq!(
        catalog
            .annotation_owners("package:other/decl.dart", origin.declaration)
            .count(),
        0
    );
    assert_eq!(
        catalog.resolve_config("Marker").unwrap().symbol.0,
        "semantic::Marker"
    );
}

#[test]
fn conflicting_or_unregistered_origins_never_change_catalog_state() {
    let mut catalog = SymbolCatalog::new();
    let origin =
        AnnotationOrigin::new("semantic::Marker", "package:actual/src/decl.dart", "Actual");
    assert_eq!(
        catalog
            .register_annotation_origin(origin)
            .unwrap_err()
            .message,
        "annotation origin references unregistered symbol `semantic::Marker`"
    );
    assert_eq!(catalog, SymbolCatalog::new());
    catalog.register_trait("Marker", origin.symbol);
    catalog.register_config("Marker", origin.symbol);
    catalog.register_annotation_origin(origin).unwrap();
    let before = catalog.clone();
    let conflict = AnnotationOrigin::new(origin.symbol, "package:other/decl.dart", "Actual");
    assert_eq!(
        catalog
            .register_annotation_origin(conflict)
            .unwrap_err()
            .message,
        "conflicting declaration origins for symbol `semantic::Marker`"
    );
    assert_eq!(catalog, before);
    assert!(catalog.resolve_trait("Marker").is_some());
    assert!(catalog.resolve_config("Marker").is_some());
}

#[test]
fn intrinsic_grouping_origin_is_not_a_plugin_claim_and_cannot_be_rebound() {
    let mut catalog = SymbolCatalog::new();
    let origin = AnnotationOrigin::new(
        "dust_dart::Derive",
        "package:dust_dart/src/derive/base.dart",
        "Derive",
    );
    assert_eq!(catalog.annotation_origin(origin.symbol), Some(&origin));
    assert_eq!(
        catalog
            .annotation_owners(origin.library_uri, origin.declaration)
            .collect::<Vec<_>>(),
        [origin.symbol]
    );
    assert_eq!(catalog.resolve("Derive"), None);
    assert_eq!(catalog.resolve_qualified(origin.symbol), None);
    assert_eq!(
        catalog.candidate_ids("Derive").collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
    catalog.register_config("Derive", origin.symbol);
    catalog.register_annotation_origin(origin).unwrap();
    let before = catalog.clone();
    let other = AnnotationOrigin::new(origin.symbol, "package:foreign/derive.dart", "Derive");
    assert!(catalog.register_annotation_origin(other).is_err());
    assert_eq!(catalog, before);
}

#[test]
fn declaration_index_preserves_multiple_owners_roles_and_intrinsic_identity() {
    let uri = "package:dust_dart/src/derive/base.dart";
    let ids = ["test::OtherDerive", "dust_dart::Derive"];
    for reverse in [false, true] {
        let mut catalog = SymbolCatalog::new();
        let order = if reverse { [ids[1], ids[0]] } else { ids };
        for id in order {
            catalog.register_config("Grouping", id);
            catalog.register_trait("GroupingTrait", id);
            let origin = AnnotationOrigin::new(id, uri, "Derive");
            catalog.register_annotation_origin(origin).unwrap();
            catalog.register_annotation_origin(origin).unwrap();
        }
        assert_eq!(
            catalog.annotation_owners(uri, "Derive").collect::<Vec<_>>(),
            ["dust_dart::Derive", "test::OtherDerive"]
        );
        assert_eq!(
            catalog
                .annotation_origins()
                .map(|origin| origin.symbol)
                .collect::<Vec<_>>(),
            ["dust_dart::Derive", "test::OtherDerive"]
        );
        assert!(catalog.resolve_trait("GroupingTrait").is_none());
        assert!(catalog.resolve_config("Grouping").is_none());
    }
}
