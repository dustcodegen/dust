//! Declaration metadata cannot replace ownership, roles or intrinsic identity.

use dust_ir::AnnotationOrigin;
use dust_resolver::SymbolCatalog;

#[test]
fn origins_do_not_guess_from_a_canonical_package_or_alias() {
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("Marker", "semantic::Marker");
    assert_eq!(catalog.annotation_origin("semantic::Marker"), None);
    let origin =
        AnnotationOrigin::new("semantic::Marker", "package:actual/src/decl.dart", "Actual");
    catalog.register_annotation_origin(origin).unwrap();
    catalog.register_annotation_origin(origin).unwrap();
    assert_eq!(catalog.annotation_origin("semantic::Marker"), Some(&origin));
    assert_eq!(catalog.annotation_origin("Marker"), None);
    assert_eq!(catalog.annotation_origin("actual::Actual"), None);
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
