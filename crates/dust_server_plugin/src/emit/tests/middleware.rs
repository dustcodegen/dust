use std::collections::BTreeMap;

use dust_ir::AnnotationValueIr;

use super::*;

#[test]
fn emits_group_and_route_middleware_in_declared_order() {
    let function = function();
    let layer = |name: &str| AnnotationValueIr::Constructor {
        name: NameIr {
            source: "TraceLayer".to_owned(),
            short: "TraceLayer".to_owned(),
            prefix: None,
            span: span(),
        },
        positional_args: vec![AnnotationValueIr::String(name.to_owned())],
        named_args: BTreeMap::new(),
    };
    let handler = HandlerPlan {
        callable: Callable::Function(&function),
        method: "GET",
        path: "/health".to_owned(),
        status: 200,
        params: Vec::new(),
        middleware: vec![layer("route-first"), layer("route-second")],
        metadata: metadata("health"),
    };

    assert_eq!(
        emit_named_router(
            "apiRoutes",
            "/api",
            &[handler],
            &[layer("group-first"), layer("group-second")],
        ),
        "Router apiRoutes() => Router.module(\n      prefix: '/api',\n      routes: [\n        Route(\n          'GET',\n          '/health',\n          const TraceLayer('route-first').toMiddleware()(const TraceLayer('route-second').toMiddleware()(_$handleHealth)),\n          metadata: const RouteMetadata(\n            operationId: 'health',\n          ),\n        ),\n      ],\n    )\n    ..layer(const TraceLayer('group-first'))\n    ..layer(const TraceLayer('group-second'));"
    );
}
