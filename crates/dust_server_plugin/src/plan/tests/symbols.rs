use super::*;

fn function(name: &str, annotations: Vec<AnnotationIr>) -> FunctionIr {
    FunctionIr {
        name: NameIr {
            source: name.to_owned(),
            short: name.to_owned(),
            prefix: None,
            span: span(),
        },
        return_type: TypeIr::named("Todo"),
        params: Vec::new(),
        annotations,
        span: span(),
    }
}

#[test]
fn rejects_repeated_route_groups() {
    let mut library = file();
    library.library_annotations = vec![
        annotation("dust_server::Routes", "/api", []),
        annotation("dust_server::Routes", "/v2", []),
    ];
    library.functions = vec![function(
        "read",
        vec![annotation("dust_server::GET", "/read", [])],
    )];

    assert!(
        messages(&library)
            .iter()
            .any(|value| value.contains("only one @Routes"))
    );
}

#[test]
fn rejects_repeated_controller_groups() {
    let mut controller = class(vec![method(
        "read",
        vec![config("dust_server::GET", Some("/read"))],
    )]);
    controller
        .configs
        .push(config("dust_server::Controller", Some("/v2")));
    let mut library = file();
    library.classes = vec![controller];

    assert!(
        messages(&library)
            .iter()
            .any(|value| value.contains("only one @Controller"))
    );
}

#[test]
fn rejects_handwritten_controller_router_member() {
    let mut library = file();
    library.classes = vec![class(vec![
        method("routes", Vec::new()),
        method("read", vec![config("dust_server::GET", Some("/read"))]),
    ])];

    assert!(
        messages(&library)
            .iter()
            .any(|value| value.contains("generated controller member `routes`"))
    );
}

#[test]
fn rejects_controller_handler_adapter_collisions() {
    let mut library = file();
    library.classes = vec![class(vec![
        method("get", vec![config("dust_server::GET", Some("/lower"))]),
        method("Get", vec![config("dust_server::GET", Some("/upper"))]),
    ])];

    assert!(
        messages(&library)
            .iter()
            .any(|value| value.contains("generated handler member `_$handleGet`"))
    );
}

#[test]
fn rejects_handwritten_controller_handler_adapter() {
    let mut library = file();
    library.classes = vec![class(vec![
        method("_$handleGet", Vec::new()),
        method("get", vec![config("dust_server::GET", Some("/get"))]),
    ])];

    assert!(
        messages(&library)
            .iter()
            .any(|value| value.contains("generated handler member `_$handleGet`"))
    );
}

#[test]
fn rejects_handwritten_controller_field_collisions() {
    let mut controller = class(vec![method(
        "read",
        vec![config("dust_server::GET", Some("/read"))],
    )]);
    controller.fields = ["routes", "_$handleRead"]
        .into_iter()
        .map(|name| dust_ir::FieldIr {
            name: name.to_owned(),
            ty: TypeIr::named("Object"),
            span: span(),
            has_default: false,
            serde: None,
            configs: Vec::new(),
        })
        .collect();
    let mut library = file();
    library.classes = vec![controller];

    let messages = messages(&library);
    assert!(
        messages
            .iter()
            .any(|value| value.contains("generated controller member `routes`"))
    );
    assert!(
        messages
            .iter()
            .any(|value| value.contains("generated handler member `_$handleRead`"))
    );
}

#[test]
fn rejects_top_level_router_collision() {
    let mut library = file();
    library.library_annotations = vec![annotation(
        "dust_server::Routes",
        "/api",
        [("name", AnnotationValueIr::String("apiRoutes".to_owned()))],
    )];
    library.functions = vec![
        function("apiRoutes", Vec::new()),
        function("read", vec![annotation("dust_server::GET", "/read", [])]),
    ];

    assert!(
        messages(&library)
            .iter()
            .any(|value| value.contains("generated route factory `apiRoutes`"))
    );
}

#[test]
fn allows_unused_top_level_router_name() {
    let mut library = file();
    library.library_annotations = vec![annotation(
        "dust_server::Routes",
        "/api",
        [("name", AnnotationValueIr::String("apiRoutes".to_owned()))],
    )];
    library.classes = vec![class(vec![method(
        "read",
        vec![config("dust_server::GET", Some("/read"))],
    )])];
    library.functions = vec![function("apiRoutes", Vec::new())];

    assert!(plan_server(&library).is_ok());
}

#[test]
fn rejects_top_level_handler_adapter_collisions() {
    let mut library = file();
    library.library_annotations = vec![annotation("dust_server::Routes", "/api", [])];
    library.functions = vec![
        function("get", vec![annotation("dust_server::GET", "/lower", [])]),
        function("Get", vec![annotation("dust_server::GET", "/upper", [])]),
        function("_$handleGet", Vec::new()),
    ];

    assert_eq!(
        messages(&library)
            .iter()
            .filter(|value| value.contains("generated handler function `_$handleGet`"))
            .count(),
        2
    );
}
