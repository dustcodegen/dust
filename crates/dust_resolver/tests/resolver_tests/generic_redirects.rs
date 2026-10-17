use super::*;
use dust_ir::{NameIr, RedirectTargetIr, SpanIr, TypeIr};

#[test]
fn non_generic_named_targets_are_not_confused_with_declared_import_prefixes() {
    for (imports, target, expected_class, expected_constructor) in [
        ("", "Done.create", "Done", Some("create")),
        ("", "Done.new", "Done", Some("new")),
        (
            "",
            "Done /* middle */ . /* suffix */ create",
            "Done",
            Some("create"),
        ),
        ("import 'other.dart' as p;", "p.Done", "p.Done", None),
        (
            "import 'other.dart' as p;",
            "p.Done.create",
            "p.Done",
            Some("create"),
        ),
    ] {
        let text = format!(
            "{imports} part 'event.g.dart'; @SerDe(tag: 'kind') sealed class Event {{ const Event(); const factory Event.done(int value) = {target}; }}"
        );
        let source = SourceText::new(FileId::new(134), text.as_str());
        let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
        assert_eq!(parsed.diagnostics, []);
        let mut catalog = SymbolCatalog::new();
        catalog.register_config("SerDe", "dust_dart::SerDe");
        let resolved = resolve_library(
            source.file_id(),
            "lib/event.dart",
            "lib/event.g.dart",
            &parsed.library,
            &catalog,
        );
        assert_eq!(resolved.diagnostics, []);
        let variant = &resolved.library.classes[0].serde.as_ref().unwrap().variants[0];
        assert_eq!(variant.target_class_name, expected_class);
        let target = variant.target.as_ref().unwrap();
        assert_eq!(
            resolved.library.classes[0].constructors[1]
                .redirected_target
                .as_ref(),
            Some(target)
        );
        assert_eq!(target.ty, TypeIr::named(expected_class));
        assert_eq!(
            target.constructor.as_ref().map(|name| name.source.as_str()),
            expected_constructor
        );
        assert_eq!(
            resolved.library.classes[0].constructors[1]
                .surface
                .redirected_target
                .as_ref()
                .unwrap()
                .ty
                .source,
            parsed.library.classes[0].constructors[1]
                .redirected_target
                .as_ref()
                .unwrap()
                .ty
                .source
        );
    }
}

#[test]
fn sealed_targets_retain_canonical_identity_applied_arguments_and_named_constructor() {
    let text = "part 'event.g.dart'; @SerDe(tag: 'type') sealed class Event<String, T> { const Event(); factory Event.done(T label) = Done<T, String>.create; } final class Done<X, Y> extends Event<Y, X> { const Done.create(this.label); final X label; }";
    let source = SourceText::new(FileId::new(134), text);
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("SerDe", "dust_dart::SerDe");
    let resolved = resolve_library(
        source.file_id(),
        "lib/event.dart",
        "lib/event.g.dart",
        &parsed.library,
        &catalog,
    );
    assert_eq!(resolved.diagnostics, []);
    let base = &resolved.library.classes[0];
    let variant = &base.serde.as_ref().unwrap().variants[0];
    let range = |fragment: &str| {
        let start = text.find(fragment).unwrap() as u32;
        SpanIr::new(
            source.file_id(),
            TextRange::new(start, start + fragment.len() as u32),
        )
    };
    assert_eq!(variant.target_class_name, "Done");
    assert_eq!(
        variant.target,
        Some(RedirectTargetIr {
            ty: TypeIr::generic("Done", vec![TypeIr::named("T"), TypeIr::named("String")]),
            constructor: Some(NameIr {
                source: "create".into(),
                short: "create".into(),
                prefix: None,
                span: range("create")
            }),
            span: range("Done<T, String>.create"),
        })
    );
    assert!(resolved.library.classes[1].requires_lowering_diagnostics);
    assert_eq!(variant.params[0].ty, TypeIr::named("T"));
}

#[test]
fn generated_variant_parameter_metadata_uses_registered_field_binding_and_normalization() {
    let source = SourceText::new(
        FileId::new(134),
        "part 'event.g.dart'; @SerDe(tag: 'kind') sealed class Event<T> { const Event(); factory Event.at(@FieldSerde(using: epoch, aliases: ['oldDate']) DateTime date, { @FieldSerde(aliases: ['oldCount']) int? count }) = At<T>; }",
    );
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("SerDe", "dust_dart::SerDe");
    catalog.register_config("FieldSerde", "dust_dart::SerDe");
    let resolved = resolve_library(
        source.file_id(),
        "lib/event.dart",
        "lib/event.g.dart",
        &parsed.library,
        &catalog,
    );
    assert_eq!(resolved.diagnostics, []);
    let variant = &resolved.library.classes[0].serde.as_ref().unwrap().variants[0];
    assert_eq!(
        variant.field_serde,
        std::collections::BTreeMap::from([
            (
                "date".into(),
                dust_ir::SerdeFieldConfigIr {
                    codec_source: Some("epoch".into()),
                    aliases: vec!["oldDate".into()],
                    ..Default::default()
                }
            ),
            (
                "count".into(),
                dust_ir::SerdeFieldConfigIr {
                    aliases: vec!["oldCount".into()],
                    ..Default::default()
                }
            ),
        ])
    );
    assert_eq!(variant.params[1].default_value_source, None);
}

#[test]
fn non_class_redirects_and_cyclic_parent_identity_fail_without_guessing_a_target() {
    for target in ["(int, int)", "int Function(int)"] {
        let text = format!(
            "part 'event.g.dart'; @SerDe(tag: 'kind') sealed class Event<T> {{ const Event(); factory Event.bad() = {target}; }}"
        );
        let source = SourceText::new(FileId::new(134), text.as_str());
        let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
        assert_eq!(parsed.diagnostics, []);
        let mut catalog = SymbolCatalog::new();
        catalog.register_config("SerDe", "dust_dart::SerDe");
        let resolved = resolve_library(
            source.file_id(),
            "lib/event.dart",
            "lib/event.g.dart",
            &parsed.library,
            &catalog,
        );
        assert_eq!(
            resolved
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            [
                "Sealed SerDe redirect on `Event` requires a named target class",
                "Sealed SerDe class Event has no factory variants"
            ]
        );
        let label = &resolved.diagnostics[0].labels[0];
        assert_eq!(
            &text[label.range.start().to_usize()..label.range.end().to_usize()],
            target
        );
    }
    let source = SourceText::new(
        FileId::new(134),
        "part 'event.g.dart'; @SerDe(tag: 'kind') sealed class Event<T> { factory Event.bad() = Loop<T>; } class Loop<T> extends Loop<T> {}",
    );
    let parsed = TreeSitterDartBackend::new().parse_file(&source, ParseOptions::default());
    assert_eq!(parsed.diagnostics, []);
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("SerDe", "dust_dart::SerDe");
    let resolved = resolve_library(
        source.file_id(),
        "lib/event.dart",
        "lib/event.g.dart",
        &parsed.library,
        &catalog,
    );
    assert_eq!(
        resolved
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        ["Variant target class Loop does not extend Event"]
    );
}
