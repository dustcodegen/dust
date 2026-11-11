use super::*;

#[test]
fn rejected_claims_discard_lexical_requests_when_no_unit_changes() {
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    let uri = "file:///root.dart";
    let root = unit(1, uri, "const marker = Marker(); @marker class Example {}");
    let output = root.source.path.with_extension("g.dart");
    let claims = BTreeMap::from([(
        output.clone(),
        Claim {
            source: uri.into(),
            producer: SymbolId::new("dust_flutter::AppRouter"),
        },
    )]);
    let mut units = BTreeMap::from([(uri.into(), Ok(root))]);
    let original = graph::build(&units).unwrap();
    assert!(
        !owners(&claims[&output].source, &units, &original, &catalog)
            .contains(&claims[&output].producer)
    );
    assert_eq!(
        original.required_lexical_imports(),
        BTreeSet::from([uri.into()])
    );

    let verified = verify(&mut units, &claims, &catalog).unwrap();

    assert!(units[uri].is_ok());
    assert!(verified.required_lexical_imports().is_empty());
}

#[test]
fn only_surviving_producers_retain_lexical_requests() {
    let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
    let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
    let origin_uri = "package:dust_flutter/src/router/route_annotations.dart";
    let origin = unit(1, origin_uri, "class AppRouter { const AppRouter(); }");
    let kept = "file:///kept.dart";
    let dropped = "file:///dropped.dart";
    let mut root = unit(
        2,
        kept,
        &format!(
            "import '{origin_uri}'; const marker = Marker(); @AppRouter() @marker class Router {{}}"
        ),
    );
    root.targets
        .insert(origin_uri.into(), Ok(origin.source.clone()));
    let rejected = unit(
        3,
        dropped,
        "const marker = Marker(); @marker class Other {}",
    );
    let claims = BTreeMap::from([
        (
            root.source.path.with_extension("g.dart"),
            Claim {
                source: kept.into(),
                producer: SymbolId::new("dust_flutter::AppRouter"),
            },
        ),
        (
            rejected.source.path.with_extension("g.dart"),
            Claim {
                source: dropped.into(),
                producer: SymbolId::new("dust_flutter::AppRouter"),
            },
        ),
    ]);
    let mut units = BTreeMap::from([
        (origin_uri.into(), Ok(origin)),
        (kept.into(), Ok(root)),
        (dropped.into(), Ok(rejected)),
    ]);
    let core = unit(4, "dart:core", "");
    for unit in units.values_mut().filter_map(|unit| unit.as_mut().ok()) {
        unit.targets
            .insert(core.source.uri.clone(), Ok(core.source.clone()));
    }
    units.insert(core.source.uri.clone(), Ok(core));
    let verified = verify(&mut units, &claims, &catalog).unwrap();
    assert_eq!(
        verified.required_lexical_imports(),
        BTreeSet::from([kept.into()])
    );
}
