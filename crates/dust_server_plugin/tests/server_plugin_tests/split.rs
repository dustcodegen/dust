use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use dust_ir::{AnnotationValueIr, PartIr, SymbolId};
use dust_plugin_api::{
    DustPlugin, GENERATED_HEADER, PluginContext, SymbolPlan, WorkspaceAnalysisBuilder,
};
use dust_server_plugin::{ServerClientDestination, register_plugin_with_clients};

use super::{class, empty_library, method};

#[test]
fn filters_existing_route_tags_into_separate_client_packages() {
    let mut library = empty_library();
    library.package_root.clear();
    let span = library.span;
    library.part_directives.push(PartIr {
        uri_literal: None,
        uri: "server.g.dart".to_owned(),
        span,
    });
    let controller = |name: &str, controller_tag: Option<&str>, route_tag: Option<&str>| {
        let named = tags(controller_tag);
        let mut value = class(
            vec![dust_ir::ConfigApplicationIr::with_arguments(
                SymbolId::new("dust_server::Controller"),
                None,
                vec![AnnotationValueIr::String(format!("/{name}"))],
                named,
                span,
            )],
            vec![method(
                vec![dust_ir::ConfigApplicationIr::with_arguments(
                    SymbolId::new("dust_server::GET"),
                    None,
                    vec![AnnotationValueIr::String("/".to_owned())],
                    tags(route_tag),
                    span,
                )],
                Vec::new(),
                span,
            )],
            span,
        );
        value.name = name.to_owned();
        value
    };
    library.classes = vec![
        controller("AdminRoutes", Some("admin"), None),
        controller("PublicRoutes", None, Some("public")),
        controller("InternalRoutes", None, None),
    ];
    let plugin = register_plugin_with_clients(vec![
        ServerClientDestination {
            package_name: "admin_client".to_owned(),
            package_root: PathBuf::from("/admin"),
            tags: vec!["admin".to_owned()],
        },
        ServerClientDestination {
            package_name: "public_client".to_owned(),
            package_root: PathBuf::from("/public"),
            tags: vec!["public".to_owned()],
        },
    ]);
    let mut analysis = WorkspaceAnalysisBuilder::default();
    plugin.collect_workspace_analysis_ir(&library, &mut analysis);
    let mut plan = SymbolPlan::default();
    plan.set_workspace_analysis(Arc::new(analysis.build()));

    let output = plugin.generate(&library, &PluginContext { symbol_plan: &plan });
    let paths = output[0]
        .auxiliary_outputs
        .iter()
        .map(|value| value.output_path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert_eq!(paths.len(), 4, "{:?}", output[0].diagnostics);
    assert!(paths.iter().any(|path| path.contains("/admin/")));
    assert!(paths.iter().any(|path| path.contains("/public/")));
    assert!(
        !paths
            .iter()
            .any(|path| path.contains("admin_routes") && path.contains("/public/"))
    );
    assert!(
        !paths
            .iter()
            .any(|path| path.contains("public_routes") && path.contains("/admin/"))
    );
    assert!(!paths.iter().any(|path| path.contains("internal_routes")));
    for auxiliary in &output[0].auxiliary_outputs {
        assert!(auxiliary.source.starts_with(GENERATED_HEADER));
    }
}

fn tags(tag: Option<&str>) -> BTreeMap<String, AnnotationValueIr> {
    tag.map(|tag| {
        BTreeMap::from([(
            "tags".to_owned(),
            AnnotationValueIr::List(vec![AnnotationValueIr::String(tag.to_owned())]),
        )])
    })
    .unwrap_or_default()
}
