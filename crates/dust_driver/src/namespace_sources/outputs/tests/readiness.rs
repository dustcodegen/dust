use super::*;

#[test]
fn pending_alias_imports_cannot_reject_a_generated_placeholder_or_claim_a_collision() {
    for (generated, authorized) in [(false, false), (false, true), (true, false), (true, true)] {
        let registry = crate::build::registry_for_selection(crate::build::RegistrySelection::All);
        let catalog = crate::catalog::build_symbol_catalog(&registry).unwrap();
        let root_uri = "file:///root.dart";
        let alias_uri = "file:///aliases.dart";
        let origin_uri = "package:dust_flutter/src/router/route_annotations.dart";
        let mut origin = unit(
            1,
            origin_uri,
            if authorized {
                "class AppRouter { const AppRouter(); }"
            } else {
                "class Foreign {}"
            },
        );
        let mut alias = unit(
            2,
            alias_uri,
            &format!("import '{origin_uri}'; typedef RouterAnnotation = AppRouter;"),
        );
        alias
            .targets
            .insert(origin_uri.into(), Ok(origin.source.clone()));
        let mut root = unit(
            3,
            root_uri,
            &format!("import '{alias_uri}'; @RouterAnnotation() class Router {{}}"),
        );
        root.targets
            .insert(alias_uri.into(), Ok(alias.source.clone()));
        let output_uri = "file:///routes.g.dart";
        let mut output = unit(
            4,
            output_uri,
            if generated {
                ""
            } else {
                "class Handwritten {}"
            },
        );
        output.generated_library = generated;
        let claims = BTreeMap::from([(
            output.source.path.clone(),
            Claim {
                source: root_uri.into(),
                producer: SymbolId::new("dust_flutter::AppRouter"),
            },
        )]);
        let core = unit(5, "dart:core", "");
        for item in [&mut root, &mut alias, &mut origin] {
            item.targets
                .insert(core.source.uri.clone(), Ok(core.source.clone()));
        }
        let mut units = BTreeMap::from([
            (root_uri.into(), Ok(root)),
            (alias_uri.into(), Ok(alias)),
            (output_uri.into(), Ok(output)),
            ("dart:core".into(), Ok(core)),
        ]);
        let pending = verify_ready(&mut units, &claims, &catalog, |_| false).unwrap();
        let Verification::NeedsImports(graph) = pending else {
            panic!("alias import must be requested")
        };
        assert_eq!(
            graph.required_lexical_imports(),
            BTreeSet::from([alias_uri.into()])
        );
        assert!(units[output_uri].is_ok());
        units.insert(origin_uri.into(), Ok(origin));
        let verified = verify_ready(&mut units, &claims, &catalog, |_| true);
        if !authorized {
            assert!(matches!(verified.unwrap(), Verification::Ready(_)));
            assert_eq!(units[output_uri].is_err(), generated);
        } else if generated {
            assert!(matches!(verified.unwrap(), Verification::Ready(_)));
            assert!(units[output_uri].is_ok());
        } else {
            assert!(
                verified
                    .err()
                    .unwrap()
                    .message
                    .contains("collides with an unmarked existing source")
            );
            assert_eq!(
                units[output_uri].as_ref().unwrap().text.as_ref(),
                "class Handwritten {}"
            );
        }
    }
}
