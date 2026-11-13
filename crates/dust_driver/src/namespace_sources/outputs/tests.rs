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
        content_digest: Default::default(),
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
    assert!(!owners(&claim.source, &units, &graph, &catalog).contains(&claim.producer));
    units.insert(claim.source.clone(), Err("unreadable".into()));
    assert!(!owners(&claim.source, &units, &graph, &catalog).contains(&claim.producer));
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
        assert!(!owners(&claim.source, &units, &graph, &catalog).contains(&claim.producer));
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

#[test]
fn rejected_claims_invalidate_only_generated_placeholders_at_their_paths() {
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    let root = std::env::current_dir().unwrap();
    let first_path = root.join("first.g.dart");
    let second_path = root.join("second.g.dart");
    let claims = [&first_path, &second_path]
        .into_iter()
        .map(|path| {
            (
                path.clone(),
                Claim {
                    source: "file:///missing-router.dart".into(),
                    producer: SymbolId::new("dust_flutter::AppRouter"),
                },
            )
        })
        .collect();
    let rejected = [
        ("file:///first.g.dart", &first_path),
        ("package:app/first.g.dart", &first_path),
        ("file:///second.g.dart", &second_path),
    ];
    let mut units = BTreeMap::new();
    for (index, (uri, path)) in rejected.iter().enumerate() {
        let mut placeholder = unit((index + 1) as u32, uri, "");
        placeholder.source.path = (*path).clone();
        placeholder.generated_library = true;
        units.insert((*uri).into(), Ok(placeholder));
    }
    let mut unmarked = unit(4, "file:///unmarked.dart", "class Handwritten {}");
    unmarked.source.path = first_path.clone();
    units.insert(unmarked.source.uri.clone(), Ok(unmarked));
    let mut unrelated = unit(5, "file:///unrelated.g.dart", "");
    unrelated.generated_library = true;
    units.insert(unrelated.source.uri.clone(), Ok(unrelated));

    verify(&mut units, &claims, &catalog).unwrap();

    for (uri, path) in rejected {
        assert_eq!(
            units[uri].as_ref().err(),
            Some(&format!(
                "generated library `{}` has no verified handwritten class producer `dust_flutter::AppRouter`",
                path.display()
            ))
        );
    }
    assert!(units["file:///unmarked.dart"].is_ok());
    assert!(units["file:///unrelated.g.dart"].is_ok());
}

#[test]
fn claims_from_the_same_source_still_require_their_own_producer() {
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    let origin_uri = "package:dust_flutter/src/router/route_annotations.dart";
    let origin = unit(1, origin_uri, "class AppRouter { const AppRouter(); }");
    let root_uri = "file:///route.dart";
    let mut root = unit(
        2,
        root_uri,
        &format!("import '{origin_uri}'; @AppRouter() class Router {{}}"),
    );
    root.targets
        .insert(origin_uri.into(), Ok(origin.source.clone()));
    let accepted_uri = "file:///accepted.g.dart";
    let rejected_uri = "file:///rejected.g.dart";
    let mut accepted = unit(3, accepted_uri, "");
    let mut rejected = unit(4, rejected_uri, "");
    accepted.generated_library = true;
    rejected.generated_library = true;
    let rejected_path = rejected.source.path.clone();
    let claims = BTreeMap::from([
        (
            accepted.source.path.clone(),
            Claim {
                source: root_uri.into(),
                producer: SymbolId::new("dust_flutter::AppRouter"),
            },
        ),
        (
            rejected_path.clone(),
            Claim {
                source: root_uri.into(),
                producer: SymbolId::new("dust_flutter::AppRoute"),
            },
        ),
    ]);
    let mut units = BTreeMap::from([
        (origin_uri.into(), Ok(origin)),
        (root_uri.into(), Ok(root)),
        (accepted_uri.into(), Ok(accepted)),
        (rejected_uri.into(), Ok(rejected)),
    ]);
    let core = unit(5, "dart:core", "");
    for unit in units.values_mut().filter_map(|unit| unit.as_mut().ok()) {
        unit.targets
            .insert(core.source.uri.clone(), Ok(core.source.clone()));
    }
    units.insert(core.source.uri.clone(), Ok(core));

    verify(&mut units, &claims, &catalog).unwrap();

    assert!(units[accepted_uri].is_ok());
    assert_eq!(
        units[rejected_uri].as_ref().err(),
        Some(&format!(
            "generated library `{}` has no verified handwritten class producer `dust_flutter::AppRoute`",
            rejected_path.display()
        ))
    );
}

#[test]
fn multiple_collisions_report_the_first_unit_identity() {
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    let origin_uri = "package:dust_flutter/src/router/route_annotations.dart";
    let origin = unit(1, origin_uri, "class AppRouter { const AppRouter(); }");
    let root_uri = "file:///root.dart";
    let mut root = unit(
        2,
        root_uri,
        &format!("import '{origin_uri}'; @AppRouter() class Router {{}}"),
    );
    root.targets
        .insert(origin_uri.into(), Ok(origin.source.clone()));
    let mut first = unit(3, "file:///a-output.dart", "class First {}");
    let mut last = unit(4, "file:///z-output.dart", "class Last {}");
    first.source.path = first.source.path.with_file_name("z-output.g.dart");
    last.source.path = last.source.path.with_file_name("a-output.g.dart");
    let first_path = first.source.path.clone();
    let claims = [&first, &last]
        .into_iter()
        .map(|output| {
            (
                output.source.path.clone(),
                Claim {
                    source: root_uri.into(),
                    producer: SymbolId::new("dust_flutter::AppRouter"),
                },
            )
        })
        .collect();
    let mut units = [origin, root, first, last]
        .into_iter()
        .map(|unit| (unit.source.uri.clone(), Ok(unit)))
        .collect::<BTreeMap<_, _>>();
    let core = unit(5, "dart:core", "");
    for unit in units.values_mut().filter_map(|unit| unit.as_mut().ok()) {
        unit.targets
            .insert(core.source.uri.clone(), Ok(core.source.clone()));
    }
    units.insert(core.source.uri.clone(), Ok(core));

    assert_eq!(
        verify(&mut units, &claims, &catalog).err().unwrap().message,
        format!(
            "generated library output `{}` collides with an unmarked existing source",
            first_path.display()
        )
    );
    assert!(units.values().all(Result::is_ok));
}

mod readiness;
mod requests;
