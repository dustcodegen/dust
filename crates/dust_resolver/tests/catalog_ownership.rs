//! Order-independent canonical ownership and unique legacy lookup contracts.

use dust_ir::SymbolId;
use dust_resolver::{ResolvedSymbol, SymbolCatalog, SymbolKind};

#[test]
fn canonical_owners_survive_client_first_registration() {
    assert_canonical_owners(["dust_dart", "dust_server"]);
}

#[test]
fn canonical_owners_survive_server_first_registration() {
    assert_canonical_owners(["dust_server", "dust_dart"]);
}

fn assert_canonical_owners(owners: [&str; 2]) {
    let mut catalog = SymbolCatalog::new();
    for owner in owners {
        catalog.register_config("GET", format!("{owner}::GET"));
    }
    for owner in owners {
        let name = format!("{owner}::GET");
        assert_eq!(
            catalog
                .resolve_qualified_config(&name)
                .map(|entry| &entry.symbol),
            Some(&SymbolId::new(&name)),
            "registration order: {owners:?}"
        );
    }
}

#[test]
fn overlapping_short_names_never_choose_the_last_owner() {
    for owners in [
        ["client::GET", "server::GET"],
        ["server::GET", "client::GET"],
    ] {
        let mut catalog = SymbolCatalog::new();
        for owner in owners {
            catalog.register_config("GET", owner);
        }
        assert_eq!(catalog.resolve("GET"), None);
        assert_eq!(catalog.resolve_config("GET"), None);
        assert_eq!(catalog.resolve_trait("GET"), None);
    }
}

#[test]
fn generic_lookup_does_not_choose_between_distinct_trait_and_config_owners() {
    let mut catalog = SymbolCatalog::new();
    catalog.register_trait("Mark", "trait_owner::Mark");
    catalog.register_config("Mark", "config_owner::Mark");
    assert_eq!(catalog.resolve("Mark"), None);
    assert_eq!(
        catalog.resolve_trait("Mark"),
        Some(&resolved("trait_owner::Mark", SymbolKind::Trait))
    );
    assert_eq!(
        catalog.resolve_config("Mark"),
        Some(&resolved("config_owner::Mark", SymbolKind::Config))
    );
}

fn resolved(name: &str, kind: SymbolKind) -> ResolvedSymbol {
    ResolvedSymbol {
        symbol: SymbolId::new(name),
        kind,
    }
}

#[test]
fn both_roles_of_each_owner_survive_every_registration_order() {
    for owners in [
        ["client::Validate", "server::Validate"],
        ["server::Validate", "client::Validate"],
    ] {
        for roles in [
            [SymbolKind::Trait, SymbolKind::Config],
            [SymbolKind::Config, SymbolKind::Trait],
        ] {
            let mut catalog = SymbolCatalog::new();
            for owner in owners {
                for role in roles {
                    let previous = match role {
                        SymbolKind::Trait => catalog.register_trait("Validate", owner),
                        SymbolKind::Config => catalog.register_config("Validate", owner),
                    };
                    assert_eq!(previous, None);
                }
            }
            for owner in owners {
                assert_eq!(
                    catalog.resolve_qualified_trait(owner),
                    Some(&resolved(owner, SymbolKind::Trait))
                );
                assert_eq!(
                    catalog.resolve_qualified_config(owner),
                    Some(&resolved(owner, SymbolKind::Config))
                );
                assert_eq!(
                    catalog.resolve_qualified(owner),
                    Some(&resolved(owner, SymbolKind::Config))
                );
            }
            assert_eq!(catalog.resolve("Validate"), None);
            assert_eq!(catalog.resolve_trait("Validate"), None);
            assert_eq!(catalog.resolve_config("Validate"), None);
            assert_eq!(
                catalog.candidate_ids("Validate").collect::<Vec<_>>(),
                ["client::Validate", "server::Validate"]
            );
        }
    }
}

#[test]
fn duplicate_roles_and_aliases_count_as_one_owner() {
    let mut catalog = SymbolCatalog::new();
    assert_eq!(catalog.register_trait("Validate", "dust::Validate"), None);
    assert_eq!(catalog.register_config("Validate", "dust::Validate"), None);
    assert_eq!(
        catalog.register_trait("Validate", "dust::Validate"),
        Some(resolved("dust::Validate", SymbolKind::Trait))
    );
    assert_eq!(
        catalog.register_config("Validate", "dust::Validate"),
        Some(resolved("dust::Validate", SymbolKind::Config))
    );
    assert_eq!(
        catalog.register_trait("LegacyValidate", "dust::Validate"),
        Some(resolved("dust::Validate", SymbolKind::Trait))
    );
    for name in ["Validate", "LegacyValidate"] {
        assert_eq!(
            catalog.candidate_ids(name).collect::<Vec<_>>(),
            ["dust::Validate"]
        );
        assert_eq!(
            catalog.resolve(name),
            Some(&resolved("dust::Validate", SymbolKind::Trait))
        );
        assert_eq!(
            catalog.resolve_trait(name),
            Some(&resolved("dust::Validate", SymbolKind::Trait))
        );
    }
    // A role registered for another alias is not silently granted to this one.
    assert_eq!(catalog.resolve_config("LegacyValidate"), None);
    assert_eq!(
        catalog.resolve_config("Validate"),
        Some(&resolved("dust::Validate", SymbolKind::Config))
    );
}

#[test]
fn short_alias_ambiguity_does_not_contaminate_other_aliases() {
    let mut catalog = SymbolCatalog::new();
    catalog.register_config("GET", "client::GET");
    catalog.register_config("ClientGET", "client::GET");
    assert_eq!(catalog.register_config("GET", "server::GET"), None);
    assert_eq!(catalog.resolve("GET"), None);
    assert_eq!(
        catalog.resolve("ClientGET"),
        Some(&resolved("client::GET", SymbolKind::Config))
    );
    assert_eq!(
        catalog.candidate_ids("GET").collect::<Vec<_>>(),
        ["client::GET", "server::GET"]
    );
    assert_eq!(
        catalog.candidate_ids("ClientGET").collect::<Vec<_>>(),
        ["client::GET"]
    );
}

#[test]
fn missing_names_and_ineligible_roles_stay_missing() {
    let mut catalog = SymbolCatalog::new();
    for registered in [false, true] {
        if registered {
            catalog.register_trait("Trait", "owner::Trait");
            catalog.register_config("Config", "owner::Config");
        }
        assert_eq!(
            catalog.candidate_ids("missing").collect::<Vec<_>>(),
            Vec::<&str>::new()
        );
        assert_eq!(catalog.resolve("missing"), None);
        assert_eq!(catalog.resolve_trait("missing"), None);
        assert_eq!(catalog.resolve_config("missing"), None);
        assert_eq!(catalog.resolve_qualified("missing::Unknown"), None);
        assert_eq!(catalog.resolve_qualified_trait("missing::Unknown"), None);
        assert_eq!(catalog.resolve_qualified_config("missing::Unknown"), None);
    }
    assert_eq!(catalog.resolve_trait("Config"), None);
    assert_eq!(catalog.resolve_config("Trait"), None);
    assert_eq!(catalog.resolve_qualified_trait("owner::Config"), None);
    assert_eq!(catalog.resolve_qualified_config("owner::Trait"), None);
    assert_eq!(
        catalog.resolve_qualified("owner::Trait"),
        Some(&resolved("owner::Trait", SymbolKind::Trait))
    );
}

#[test]
fn catalog_and_candidate_order_are_deterministic() {
    let mut forward = SymbolCatalog::new();
    let mut reverse = SymbolCatalog::new();
    for owner in ["z::GET", "a::GET", "m::GET"] {
        forward.register_trait("GET", owner);
        forward.register_config("GET", owner);
    }
    for owner in ["m::GET", "a::GET", "z::GET"] {
        reverse.register_config("GET", owner);
        reverse.register_trait("GET", owner);
    }
    assert_eq!(forward, reverse);
    assert_eq!(
        forward.candidate_ids("GET").collect::<Vec<_>>(),
        ["a::GET", "m::GET", "z::GET"]
    );
}
