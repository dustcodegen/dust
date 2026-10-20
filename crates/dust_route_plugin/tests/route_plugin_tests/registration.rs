use dust_plugin_api::DustPlugin;
use dust_route_plugin::register_plugin;

#[test]
fn registers_route_annotation_symbols() {
    let plugin = register_plugin();

    assert_eq!(plugin.plugin_name(), "Route");
    assert_eq!(
        plugin.claimed_configs(),
        vec![
            "dust_flutter::AppRouter",
            "dust_flutter::AppRoute",
            "dust_flutter::GeneratedRoute"
        ]
    );
    assert_eq!(
        plugin.supported_annotations(),
        vec!["AppRouter", "AppRoute", "GeneratedRoute"]
    );
}

#[test]
fn planned_paths_match_emission_for_absolute_and_relative_nested_sources() {
    use crate::support::{library_with_classes, route_page_class, router_class};
    use dust_plugin_api::{PluginContext, SymbolPlan};

    let root = std::env::current_dir().unwrap();
    for relative in [false, true] {
        let mut library = library_with_classes(vec![
            router_class("(initial: '/', notFound: '/404')"),
            route_page_class("HomePage", "('/', name: 'home')", Vec::new()),
        ]);
        let source = root.join("nested/lib/app.route.dart");
        library.source_path = if relative {
            "nested/lib/app.route.dart".into()
        } else {
            source.to_string_lossy().into_owned()
        };
        library.package_root = root.to_string_lossy().into_owned();
        library.output_path = "ignored.custom.dart".into();
        let plugin = register_plugin();
        let claims = plugin.potential_generated_libraries(&source);
        let contributions = plugin.generate(
            &library,
            &PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        );
        assert_eq!(
            contributions
                .iter()
                .flat_map(|contribution| &contribution.diagnostics)
                .collect::<Vec<_>>(),
            Vec::<&dust_diagnostics::Diagnostic>::new()
        );
        assert_eq!(
            claims
                .iter()
                .map(|claim| claim.path.clone())
                .collect::<Vec<_>>(),
            contributions
                .into_iter()
                .flat_map(|contribution| contribution.auxiliary_outputs)
                .map(|output| output.output_path)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            claims
                .into_iter()
                .map(|claim| claim.producer.0)
                .collect::<Vec<_>>(),
            vec!["dust_flutter::AppRouter"; 5]
        );
    }
}
