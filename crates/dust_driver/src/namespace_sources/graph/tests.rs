use std::sync::Arc;

use dust_ir::{AnnotationOrigin, SymbolId};
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_resolver::{NamespaceBinding, SymbolCatalog};
use dust_text::{FileId, SourceText};

use super::*;
use crate::namespace_sources::index::Source;

const ORIGIN: &str = "package:annotations/marker.dart";
const APP: &str = "file:///app/root.dart";

fn unit(id: u32, uri: &str, text: &str) -> Unit {
    let text: Arc<str> = text.into();
    let parsed = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(FileId::new(id), Arc::clone(&text)),
        ParseOptions::default(),
    );
    assert_eq!(parsed.diagnostics, []);
    Unit {
        source: Source {
            uri: uri.into(),
            path: format!("/app/{id}.dart").into(),
            language_version: Default::default(),
        },
        file_id: FileId::new(id),
        text,
        content_digest: Default::default(),
        parsed: parsed.library,
        diagnostics: parsed.diagnostics,
        namespace_diagnostics: parsed.namespace_diagnostics,
        targets: BTreeMap::new(),
        dependencies: Default::default(),
        generated_owner: None,
        generated_library: false,
    }
}

fn serial(units: &BTreeMap<String, Result<Unit, String>>) -> NamespaceGraph {
    let mut graph = NamespaceGraph::new();
    for (uri, unit) in units {
        if let Ok(unit) = unit {
            let core = implicit_core(&unit.parsed.directives, uri);
            graph
                .insert_library_with_appended_directive(
                    uri,
                    unit.file_id,
                    &unit.parsed,
                    core.as_ref(),
                    |_, reference| resolve(units, unit, reference),
                )
                .unwrap();
        }
    }
    graph
}

#[test]
fn parallel_preparation_retains_serial_binding_and_source_failure_diagnostics() {
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("Marker", "test::Marker");
    catalog
        .register_annotation_origin(AnnotationOrigin::new("test::Marker", ORIGIN, "Marker"))
        .unwrap();
    for failure in 0..5 {
        let core = unit(3, "dart:core", "class Object {}");
        let mut origin = unit(1, ORIGIN, "class Marker { const Marker(); }");
        let mut root = unit(2, APP, &format!("import '{ORIGIN}' as p;"));
        for unit in [&mut origin, &mut root] {
            unit.targets
                .insert("dart:core".into(), Ok(core.source.clone()));
        }
        root.targets
            .insert(ORIGIN.into(), Ok(origin.source.clone()));
        match failure {
            1 => {
                origin.namespace_diagnostics = vec![
                    Diagnostic::error("first interface error"),
                    Diagnostic::error("second interface error"),
                ]
            }
            3 => {
                root.targets
                    .insert(ORIGIN.into(), Err("invalid package configuration".into()));
            }
            4 => {
                root.targets.remove(ORIGIN);
            }
            _ => {}
        }
        let origin = if failure == 2 {
            Err("source read failed".into())
        } else {
            Ok(origin)
        };
        let units = BTreeMap::from([
            (APP.into(), Ok(root)),
            (ORIGIN.into(), origin),
            ("dart:core".into(), Ok(core)),
        ]);
        let parallel = build(&units).unwrap();
        let serial = serial(&units);
        let actual = parallel.scope(APP).bind("p.Marker", &catalog);
        assert_eq!(actual, serial.scope(APP).bind("p.Marker", &catalog));
        if failure == 0 {
            assert_eq!(
                actual,
                NamespaceBinding::Owned(SymbolId::new("test::Marker"))
            );
        } else {
            assert!(matches!(
                actual,
                NamespaceBinding::Rejected {
                    kind: dust_resolver::NamespaceFailureKind::MissingInput,
                    ..
                }
            ));
        }
        assert_eq!(
            parallel.required_lexical_imports(),
            serial.required_lexical_imports()
        );
    }
}

#[test]
fn parallel_preparation_reports_the_first_duplicate_in_original_uri_order() {
    let units = BTreeMap::from([
        (
            "file:///a.dart".into(),
            Ok(unit(1, "file:///a.dart", "class A {}")),
        ),
        (
            "file:///b.dart".into(),
            Ok(unit(1, "file:///b.dart", "class B {}")),
        ),
        (
            "file:///c.dart".into(),
            Ok(unit(1, "file:///c.dart", "class C {}")),
        ),
    ]);
    assert_eq!(
        build(&units).unwrap_err().message,
        "duplicate namespace input identity `file:///b.dart` or file id 1"
    );
}

#[test]
fn dynamic_preparation_preserves_alternating_root_placeholder_and_failed_unit_facts() {
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("Marker", "test::Marker");
    catalog
        .register_annotation_origin(AnnotationOrigin::new("test::Marker", ORIGIN, "Marker"))
        .unwrap();
    let core = unit(1, "dart:core", "class Object {}");
    let mut origin = unit(2, ORIGIN, "class Marker { const Marker(); }");
    origin
        .targets
        .insert("dart:core".into(), Ok(core.source.clone()));
    let mut units = BTreeMap::new();
    for number in 0..36 {
        let uri = format!("file:///app/model_{number:02}.dart");
        let generated_uri = format!("file:///app/model_{number:02}.g.dart");
        let mut root = unit(
            3 + number * 2,
            &uri,
            &format!(
                "import '{ORIGIN}' as p; part '{generated_uri}'; @p.Marker() class Model{number} {{ const Model{number}(this.id); final int id; }}"
            ),
        );
        let mut generated = unit(4 + number * 2, &generated_uri, &format!("part of '{uri}';"));
        root.targets
            .insert(ORIGIN.into(), Ok(origin.source.clone()));
        root.targets
            .insert("dart:core".into(), Ok(core.source.clone()));
        root.targets
            .insert(generated_uri.clone(), Ok(generated.source.clone()));
        generated
            .targets
            .insert(uri.clone(), Ok(root.source.clone()));
        units.insert(uri, Ok(root));
        units.insert(generated_uri, Ok(generated));
    }
    units.insert(
        "file:///app/model_03.error.dart".into(),
        Err("unreadable source".into()),
    );
    units.insert(ORIGIN.into(), Ok(origin));
    units.insert("dart:core".into(), Ok(core));
    let expected = serial(&units);
    let actual = build(&units).unwrap();
    for (uri, unit) in &units {
        if unit.is_ok() && uri.starts_with("file:///app/model_") {
            for name in ["p.Marker", "Missing"] {
                assert_eq!(
                    actual.scope(uri).bind(name, &catalog),
                    expected.scope(uri).bind(name, &catalog),
                    "{uri}: {name}"
                );
            }
        }
    }
    assert_eq!(
        actual.required_lexical_imports(),
        expected.required_lexical_imports()
    );
}
