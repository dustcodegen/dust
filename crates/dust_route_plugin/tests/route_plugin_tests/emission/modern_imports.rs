use std::sync::Arc;

use dust_ir::ImportIr;
use dust_plugin_api::{DustPlugin, SymbolPlan, WorkspaceAnalysisBuilder};
use dust_route_plugin::register_plugin;

use crate::support::{library_with_classes, route_page_class, router_class, span};

use super::support::{
    assert_route_snapshot, generate_route_output_with_plan, route_outputs_snapshot,
};

#[test]
fn prefixed_standalone_transition_keeps_its_import_and_combinators() {
    let mut page = library_with_classes(vec![
        route_page_class(
            "DetailsPage",
            "('/details', name: 'details', transition: transitions.FadeUpwardsPageTransitionsBuilder())",
            Vec::new(),
        ),
        route_page_class(
            "PlainPage",
            "('/plain', name: 'plain', transition: CupertinoPageTransitionsBuilder())",
            Vec::new(),
        ),
    ]);
    page.source_path = "lib/pages/details.dart".to_owned();
    let plain_import = ImportIr {
        namespace: None,
        uri: "package:cupertino_ui/cupertino_ui.dart".to_owned(),
        prefix: None,
        show: vec!["CupertinoPageTransitionsBuilder".to_owned()],
        hide: Vec::new(),
        is_deferred: false,
        span: span(0, 0),
    };
    page.import_directives = vec![
        ImportIr {
            uri: "package:material_ui/material_ui.dart".to_owned(),
            prefix: Some("transitions".to_owned()),
            show: vec![
                "FadeUpwardsPageTransitionsBuilder".to_owned(),
                "ZoomPageTransitionsBuilder".to_owned(),
            ],
            hide: vec!["ZoomPageTransitionsBuilder".to_owned()],
            ..plain_import.clone()
        },
        plain_import,
    ];
    let mut analysis = WorkspaceAnalysisBuilder::default();
    register_plugin().collect_workspace_analysis_ir(&page, &mut analysis);
    let mut plan = SymbolPlan::default();
    plan.set_workspace_analysis(Arc::new(analysis.build()));
    let router = library_with_classes(vec![router_class(
        "(initial: '/details', notFound: '/404')",
    )]);
    let contribution = generate_route_output_with_plan(&router, &plan);

    assert_route_snapshot(
        "modern_prefixed_transition.dart.snapshot",
        &route_outputs_snapshot(&contribution),
    );
}
