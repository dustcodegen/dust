use std::{collections::BTreeMap, fs, path::Path};

use dust_ir::AnnotationOrigin;
use dust_parser_dart::{ParseBackend, ParseOptions};
use dust_parser_dart_ts::TreeSitterDartBackend;
use dust_plugin_api::{MetadataOutput, PluginExecutionMode};
use dust_text::{FileId, SourceText};

use super::build_symbol_catalog;
use crate::build::{RegistrySelection, registry_for_selection};

/// Parse the reviewed exact identity table, not Dart source text.
fn expected(database: bool) -> BTreeMap<&'static str, (&'static str, &'static str)> {
    let fixtures = [
        include_str!("../../../../fixtures/annotation_origins/default.tsv"),
        if database {
            include_str!("../../../../fixtures/annotation_origins/db_overrides.tsv")
        } else {
            ""
        },
    ];
    fixtures
        .into_iter()
        .flat_map(str::lines)
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(fields.len(), 3);
            (fields[0], (fields[1], fields[2]))
        })
        .collect()
}

/// Every advertised declaration must exist in real parser-owned class/typedef facts.
fn audit_source(origin: AnnotationOrigin) {
    let relative = origin.library_uri.strip_prefix("package:").unwrap();
    let (package, path) = relative.split_once('/').unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let path = root.join("packages").join(package).join("lib").join(path);
    let text = SourceText::new(FileId::new(42), fs::read_to_string(path).unwrap());
    let parsed = TreeSitterDartBackend::new().parse_file(&text, ParseOptions::default());
    assert_eq!(parsed.diagnostics, vec![], "{}", origin.library_uri);
    let classes = parsed.library.classes.iter().map(|item| item.name.as_str());
    let typedefs = parsed
        .library
        .typedefs
        .iter()
        .map(|item| item.name.as_str());
    assert_eq!(
        classes
            .chain(typedefs)
            .filter(|name| *name == origin.declaration)
            .count(),
        1,
        "{origin:?}"
    );
}

#[test]
fn real_declaration_origins_cover_every_effective_claim_in_all_profiles() {
    let modes = [
        RegistrySelection::All,
        RegistrySelection::ValidatingPlugin {
            plugin: "Database",
            mode: PluginExecutionMode::online(MetadataOutput::Write),
        },
        RegistrySelection::ValidatingPlugin {
            plugin: "Database",
            mode: PluginExecutionMode::offline(MetadataOutput::ReadOnly),
        },
    ];
    for selection in modes {
        let registry = registry_for_selection(selection);
        let catalog = build_symbol_catalog(&registry).unwrap();
        let mut actual = BTreeMap::new();
        for symbol in registry
            .claimed_trait_symbols()
            .into_iter()
            .chain(registry.claimed_config_symbols())
        {
            let origin = *catalog
                .annotation_origin(&symbol.0)
                .unwrap_or_else(|| panic!("missing {symbol:?}"));
            assert_eq!(origin.symbol, symbol.0);
            audit_source(origin);
            actual.insert(origin.symbol, (origin.library_uri, origin.declaration));
        }
        let grouping = *catalog.annotation_origin("dust_dart::Derive").unwrap();
        audit_source(grouping);
        actual.insert(
            grouping.symbol,
            (grouping.library_uri, grouping.declaration),
        );
        assert_eq!(actual, expected(selection != RegistrySelection::All));
        assert_eq!(registry.annotation_origins().len() + 1, actual.len());
        assert!(catalog.resolve_trait("Validate").is_some());
        assert!(catalog.resolve_config("Validate").is_some());
        assert_eq!(catalog.resolve("Derive"), None);
        assert_eq!(
            catalog.resolve_config("GET").unwrap().symbol.0,
            "dust_dart::GET"
        );
    }
}
