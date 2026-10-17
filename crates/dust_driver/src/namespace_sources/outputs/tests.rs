use std::sync::Arc;

use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_text::{FileId, SourceText};

use super::*;

/// Builds retained facts without a real SDK or file IO.
fn unit(id: u32, uri: &str, text: &str) -> Unit {
    let text: Arc<str> = text.into();
    let parsed = TreeSitterDartBackend::new().parse_file(
        &SourceText::new(FileId::new(id), Arc::clone(&text)),
        ParseOptions::default(),
    );
    Unit {
        source: Source {
            uri: uri.into(),
            path: std::env::current_dir()
                .unwrap()
                .join(format!("source-{id}.dart")),
            language_version: Default::default(),
        },
        file_id: FileId::new(id),
        text,
        parsed: parsed.library,
        diagnostics: parsed.diagnostics,
        namespace_diagnostics: parsed.namespace_diagnostics,
        targets: BTreeMap::new(),
        dependencies: BTreeSet::new(),
        generated_owner: None,
        generated_library: false,
    }
}

#[test]
fn identical_claims_deduplicate_but_distinct_library_identities_conflict() {
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let source = unit(1, "file:///app/lib/route.dart", "").source;
    assert_eq!(
        plan(&[source.clone(), source.clone()], &registry)
            .unwrap()
            .len(),
        5
    );
    let alias = Source {
        uri: "package:app/route.dart".into(),
        ..source.clone()
    };
    let message = plan(&[source.clone(), alias], &registry)
        .unwrap_err()
        .message;
    assert_eq!(
        message,
        format!(
            "generated library `{}` has conflicting producers",
            source
                .path
                .with_extension("")
                .join("routes.g.dart")
                .display()
        )
    );
}

#[test]
fn missing_failed_generated_and_part_units_cannot_authorize_outputs() {
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    let claim = Claim {
        source: "file:///root.dart".into(),
        producer: SymbolId::new("dust_flutter::AppRouter"),
    };
    let mut units = BTreeMap::new();
    let graph = graph::build(&units).unwrap();
    assert!(!owned(&claim, &units, &graph, &catalog));
    units.insert(claim.source.clone(), Err("unreadable".into()));
    assert!(!owned(&claim, &units, &graph, &catalog));
    for source in [
        "",
        "class {",
        "part of 'root.dart'; @AppRouter() class Router {}",
    ] {
        let mut root = unit(1, &claim.source, source);
        if source.is_empty() {
            root.generated_library = true;
        }
        units.insert(claim.source.clone(), Ok(root));
        let graph = graph::build(&units).unwrap();
        assert!(!owned(&claim, &units, &graph, &catalog));
    }
}

#[test]
fn invalid_graph_fails_before_any_placeholder_mutation() {
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    let mut units = BTreeMap::from([
        (
            "file:///one.dart".into(),
            Ok(unit(1, "file:///one.dart", "")),
        ),
        (
            "file:///two.dart".into(),
            Ok(unit(1, "file:///two.dart", "")),
        ),
    ]);
    assert!(verify(&mut units, &BTreeMap::new(), &catalog).is_err());
    assert!(units.values().all(Result::is_ok));
}
