#![cfg(test)]

use super::*;

#[test]
fn unknown_provenance_never_chooses_between_overlapping_legacy_owners() {
    let source = "class Marker { const Marker(); } @Marker() void handler() {}";
    let (graph, mut catalog, parsed) = inputs(source, false);
    catalog.register_config("Marker", "legacy_a::Marker");
    catalog.register_config("Marker", "legacy_b::Marker");
    let (ir, diagnostics) = pipeline(&graph, &catalog, &parsed);
    assert_eq!(ir, None);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "annotation `Marker` has unknown provenance and multiple registered owners: legacy_a::Marker, legacy_b::Marker"
    );
}

#[test]
fn hidden_ownership_is_fatal_at_every_raw_metadata_site() {
    for declaration in [
        "@s.GET('/m') mixin M {}",
        "@s.GET('/e') extension E on String {}",
        "@s.GET('/t') extension type W(int value) {}",
        "@s.GET('/f') void handler() {}",
        "@s.GET('/v') final int value=0;",
        "@s.GET('/a') typedef Alias=int;",
    ] {
        let source = format!("import '{SERVER}' as s hide GET; {declaration}");
        let (ir, diagnostics) = lower(&source, false);
        assert_eq!(ir, None, "{declaration}");
        assert_eq!(diagnostics.len(), 1, "{declaration}: {diagnostics:?}");
        assert_eq!(
            diagnostics[0].message,
            "annotation name `GET` is not visible in this namespace"
        );
    }
    let source = format!("@s.GET('/lib') library app; import '{SERVER}' as s hide GET;");
    let (ir, diagnostics) = lower(&source, false);
    assert_eq!(ir, None);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "annotation name `GET` is not visible in this namespace"
    );
}

#[test]
fn ambiguous_missing_deferred_and_conditional_owners_cannot_reach_driver_ir() {
    for (imports, message) in [
        (
            format!("import '{CLIENT}' as p; import '{SERVER}' as p;"),
            "annotation `GET` is ambiguous between distinct declarations",
        ),
        (
            "import 'package:missing/source.dart' as p;".to_owned(),
            "missing required namespace input `package:missing/source.dart`",
        ),
        (
            format!("import '{SERVER}' deferred as p;"),
            "metadata cannot use a deferred import",
        ),
        (
            format!("import '{CLIENT}' if (dart.library.io) '{SERVER}' as p;"),
            "annotation `p.GET` has target-dependent conditional ownership",
        ),
    ] {
        for reverse in [false, true] {
            let source = format!("{imports} @p.GET('/handler') void handler() {{}}");
            let (ir, diagnostics) = lower(&source, reverse);
            assert_eq!(ir, None);
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].message, message);
            assert_eq!(diagnostics[0].labels[0].file_id, FileId::new(3));
        }
    }
}

#[test]
fn known_foreign_metadata_retains_source_but_has_no_dust_owner() {
    let source = "class GET { const GET(String path); }
@GET('/mixin') mixin M {}
@GET('/function') void handler(@GET('/parameter') int value) {}
@GET('/variable') final int value=0;";
    let (ir, diagnostics) = lower(source, false);
    assert_eq!(diagnostics, []);
    let ir = ir.unwrap();
    for annotation in [
        &ir.mixins[0].annotations[0],
        &ir.functions[0].annotations[0],
        &ir.variables[0].annotations[0],
    ] {
        assert_eq!(annotation.raw_name, "GET");
        assert_eq!(annotation.resolved_symbol, None);
        assert_eq!(annotation.positional_args.len(), 1);
    }
    assert_eq!(ir.functions[0].params[0].configs, []);
    assert_eq!(ir.functions[0].params[0].traits, []);
}

#[test]
fn scoped_resolver_errors_stop_before_unrelated_raw_metadata_is_lowered() {
    let source = format!(
        "import '{SERVER}' as p hide GET; @p.GET('/class') class Broken {{}} @missing.GET('/function') void handler() {{}}"
    );
    let (ir, diagnostics) = lower(&source, false);
    assert_eq!(ir, None);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "annotation name `GET` is not visible in this namespace"
    );
}
