/// Handler body-media and ordering validation.
mod body;
/// Shared handler validation and planning.
mod handler;
/// Dart identifier validation for generated declarations.
mod identifier;
/// Library-level lookup and validation.
mod library;
/// Internal normalized generation plan.
mod model;
/// Parameter extractor normalization.
mod param;
/// Type-shape validation.
mod types;
/// Annotation-value and path validation.
mod values;

/// End-to-end planning tests for private validation branches.
#[cfg(test)]
mod tests;

use dust_diagnostics::Diagnostic;
use dust_ir::{ClassIr, DartFileIr, EnumIr, FunctionIr, MethodIr};

pub(crate) use model::{
    BodyFormat, Callable, ControllerPlan, Extraction, ExtractionWrapper, HandlerPlan, ParamPlan,
    QueryConversionKind, QueryConversionPlan, QueryModelPlan, RawBodyFormat, RouteMetadataPlan,
    ServerPlan,
};

use handler::parse_handler;
use identifier::is_emittable_identifier;
use library::{
    default_routes_name, find_config, first_server_span, is_server_config, validate_part,
    validate_unique_routes, verb_config,
};
use model::{Args, GroupPlan, VERBS};
use values::{
    error, parse_middleware, parse_tags, required_string, string_value, validate_named,
    validate_route_path,
};

/// Builds the validated server plan for one Dart library.
pub(crate) fn plan_server(file: &DartFileIr) -> Result<Option<ServerPlan<'_>>, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut controllers = Vec::new();
    let mut routes = None;

    for annotation in &file.library_annotations {
        if annotation
            .resolved_symbol
            .as_ref()
            .map(|value| value.0.as_str())
            == Some("dust_server::Routes")
        {
            routes = parse_group(Args::Annotation(annotation), "Routes", &mut diagnostics);
        }
    }

    for class in &file.classes {
        let Some(config) = find_config(&class.configs, "dust_server::Controller") else {
            continue;
        };
        if !class.type_parameters.is_empty() {
            diagnostics.push(error(
                class.span,
                "generic controllers are not supported by generated server handlers",
                "use a non-generic controller until typed controller casts are implemented",
            ));
        }
        let group =
            parse_group(Args::Config(config), "Controller", &mut diagnostics).unwrap_or_default();
        let handlers = class
            .methods
            .iter()
            .filter_map(|method| {
                parse_method(
                    method,
                    &group.tags,
                    &file.classes,
                    &file.enums,
                    &mut diagnostics,
                )
            })
            .collect();
        controllers.push(ControllerPlan {
            class,
            prefix: group.prefix,
            handlers,
            middleware: group.middleware,
        });
    }

    let function_group = routes.unwrap_or_default();
    let functions = file
        .functions
        .iter()
        .filter_map(|function| {
            parse_function(
                function,
                &function_group.tags,
                &file.classes,
                &file.enums,
                &mut diagnostics,
            )
        })
        .collect::<Vec<_>>();
    let has_output =
        controllers.iter().any(|value| !value.handlers.is_empty()) || !functions.is_empty();
    if !has_output {
        if let Some(span) = first_server_span(file) {
            diagnostics.push(error(
                span,
                "server annotations do not declare a supported handler",
                "add a supported HTTP verb to a controller method or top-level function",
            ));
        }
        return if diagnostics.is_empty() {
            Ok(None)
        } else {
            Err(diagnostics)
        };
    }

    validate_part(file, &mut diagnostics);
    validate_unique_routes(&controllers, &functions, &mut diagnostics);
    let function_name = function_group
        .configured_name
        .unwrap_or_else(|| default_routes_name(&file.source_path));
    if !is_emittable_identifier(&function_name) {
        diagnostics.push(error(
            file.span,
            format!("route factory name `{function_name}` is not a valid Dart identifier"),
            "use a non-reserved Dart identifier in @Routes(name: ...)",
        ));
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    Ok(Some(ServerPlan {
        controllers,
        functions,
        function_prefix: function_group.prefix,
        function_name,
        function_middleware: function_group.middleware,
    }))
}

/// Normalizes one controller method when it declares a server verb.
fn parse_method<'a>(
    method: &'a MethodIr,
    inherited_tags: &[String],
    classes: &[ClassIr],
    enums: &[EnumIr],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<HandlerPlan<'a>> {
    let verbs = method
        .configs
        .iter()
        .filter_map(|config| verb_config(config).map(|verb| (config, verb)))
        .collect::<Vec<_>>();
    if verbs.is_empty() {
        if method
            .configs
            .iter()
            .chain(method.params.iter().flat_map(|value| value.configs.iter()))
            .any(is_server_config)
        {
            diagnostics.push(error(
                method.span,
                "server handler is missing an HTTP verb",
                "add exactly one server verb annotation",
            ));
        }
        return None;
    }
    if method.is_static || method.is_external || !method.has_body {
        diagnostics.push(error(
            method.span,
            "controller handlers must be concrete instance methods",
            "remove static/external or add a method body",
        ));
    }
    parse_handler(
        Callable::Method(method),
        verbs
            .into_iter()
            .map(|(args, verb)| (Args::Config(args), verb))
            .collect(),
        inherited_tags,
        classes,
        enums,
        diagnostics,
    )
}

/// Normalizes one top-level handler function.
fn parse_function<'a>(
    function: &'a FunctionIr,
    inherited_tags: &[String],
    classes: &[ClassIr],
    enums: &[EnumIr],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<HandlerPlan<'a>> {
    let verbs = function
        .annotations
        .iter()
        .filter_map(|annotation| {
            let symbol = annotation.resolved_symbol.as_ref()?.0.as_str();
            VERBS
                .iter()
                .find(|(candidate, _)| *candidate == symbol)
                .map(|(_, verb)| (Args::Annotation(annotation), *verb))
        })
        .collect::<Vec<_>>();
    if verbs.is_empty() {
        let has_server_metadata = function.annotations.iter().any(|annotation| {
            annotation
                .resolved_symbol
                .as_ref()
                .is_some_and(|symbol| symbol.0.starts_with("dust_server::"))
        }) || function
            .params
            .iter()
            .flat_map(|param| &param.configs)
            .any(is_server_config);
        if has_server_metadata {
            diagnostics.push(error(
                function.span,
                "server handler is missing an HTTP verb",
                "add exactly one server verb annotation",
            ));
        }
        return None;
    }
    parse_handler(
        Callable::Function(function),
        verbs,
        inherited_tags,
        classes,
        enums,
        diagnostics,
    )
}

/// Reads a controller or top-level route-group annotation.
fn parse_group(args: Args<'_>, name: &str, diagnostics: &mut Vec<Diagnostic>) -> Option<GroupPlan> {
    let allowed = if name == "Routes" {
        &["name", "tags", "middleware"][..]
    } else {
        &["tags", "middleware"][..]
    };
    validate_named(&args, allowed, diagnostics);
    let path = required_string(&args, 0, "route prefix", diagnostics)?;
    validate_route_path(&path, args.span(), diagnostics);
    let configured_name = args.named().get("name").and_then(string_value);
    let middleware = parse_middleware(&args, diagnostics);
    let tags = parse_tags(&args);
    Some(GroupPlan {
        prefix: path,
        configured_name,
        middleware,
        tags,
    })
}
