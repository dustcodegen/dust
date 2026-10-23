use dust_ir::{AnnotationValueIr, ConfigApplicationIr, ExprSourceIr, PartIr, SymbolId};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};
use dust_server_plugin::register_plugin;

use super::{class, empty_library, method};

#[test]
fn opaque_middleware_returns_diagnostics_without_generated_code() {
    let mut library = empty_library();
    let span = library.span;
    library.part_directives.push(PartIr {
        uri_literal: None,
        uri: "server.g.dart".to_owned(),
        span,
    });
    let route = ConfigApplicationIr::with_arguments(
        SymbolId::new("dust_server::GET"),
        None,
        vec![AnnotationValueIr::String("/".to_owned())],
        [(
            "middleware".to_owned(),
            AnnotationValueIr::List(vec![AnnotationValueIr::Expression(ExprSourceIr {
                source: "buildLayer()".to_owned(),
                span,
            })]),
        )]
        .into_iter()
        .collect(),
        span,
    );
    let controller = ConfigApplicationIr::with_arguments(
        SymbolId::new("dust_server::Controller"),
        None,
        vec![AnnotationValueIr::String("/todos".to_owned())],
        Default::default(),
        span,
    );
    library.classes = vec![class(
        vec![controller],
        vec![method(vec![route], Vec::new(), span)],
        span,
    )];

    let plan = SymbolPlan::default();
    let output = register_plugin().generate(&library, &PluginContext { symbol_plan: &plan });

    assert_eq!(output.len(), 1);
    assert!(output[0].mixin_members.is_empty());
    assert!(output[0].top_level_functions.is_empty());
    assert!(output[0].diagnostics.iter().any(|value| {
        value
            .message
            .contains("middleware entries must be structured const Layer values")
    }));
}
