/// Structured constant emission for annotation-owned values.
mod const_value;
/// Parameter extraction and unwrapping emission.
mod params;

use dust_dart_emit::dart_string_literal;
use dust_ir::{ParamKind, TypeIr};
use dust_plugin_api::PluginContribution;

use crate::plan::{HandlerPlan, RouteMetadataPlan, ServerPlan, generated_handler_name};
use const_value::render_const;
use params::ParamEmitter;
#[cfg(test)]
use params::{is_json_map, is_string_map};

/// Emitter contract tests.
#[cfg(test)]
mod tests;

/// Emits one validated server plan through standard plugin contributions.
pub(crate) fn emit_server(plan: &ServerPlan<'_>) -> PluginContribution {
    let mut contribution = PluginContribution::default();
    for controller in &plan.controllers {
        contribution.push_mixin_member(
            &controller.class.name,
            emit_router(
                &controller.prefix,
                &controller.handlers,
                &controller.middleware,
            ),
        );
        for handler in &controller.handlers {
            contribution.push_mixin_member(
                &controller.class.name,
                emit_handler(handler, Some(&controller.class.name)),
            );
        }
    }
    if !plan.functions.is_empty() {
        contribution.top_level_functions.push(emit_named_router(
            &plan.function_name,
            &plan.function_prefix,
            &plan.functions,
            &plan.function_middleware,
        ));
        contribution.top_level_functions.extend(
            plan.functions
                .iter()
                .map(|handler| emit_handler(handler, None)),
        );
    }
    contribution
}

/// Emits a controller's conventional `routes` member.
fn emit_router(
    prefix: &str,
    handlers: &[HandlerPlan<'_>],
    middleware: &[dust_ir::AnnotationValueIr],
) -> String {
    emit_named_router("routes", prefix, handlers, middleware)
}

/// Emits a router factory with deterministic route order.
fn emit_named_router(
    name: &str,
    prefix: &str,
    handlers: &[HandlerPlan<'_>],
    middleware: &[dust_ir::AnnotationValueIr],
) -> String {
    let member_indent = "    ";
    let routes = handlers
        .iter()
        .map(|handler| {
            let handler_name = generated_handler_name(handler.callable.name());
            let route_indent = format!("{member_indent}    ");
            let argument_indent = format!("{route_indent}  ");
            format!(
                "{route_indent}Route(\n{argument_indent}{},\n{argument_indent}{},\n{argument_indent}{},\n{argument_indent}metadata: {},\n{route_indent}),",
                dart_string_literal(handler.method),
                dart_string_literal(&handler.path),
                wrap_handler(&handler_name, &handler.middleware),
                render_metadata(&handler.metadata, &argument_indent),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let layers = middleware
        .iter()
        .map(|value| format!("\n{member_indent}..layer({})", render_const(value, true)))
        .collect::<String>();
    format!(
        "Router {name}() => Router.module(\n{member_indent}  prefix: {},\n{member_indent}  routes: [\n{routes}\n{member_indent}  ],\n{member_indent})",
        dart_string_literal(prefix)
    ) + &layers
        + ";"
}

/// Emits format-neutral metadata without depending on a documentation format.
fn render_metadata(metadata: &RouteMetadataPlan, indent: &str) -> String {
    let field_indent = format!("{indent}  ");
    let mut fields = vec![format!(
        "{field_indent}operationId: {},",
        dart_string_literal(&metadata.operation_id)
    )];
    if let Some(summary) = &metadata.summary {
        fields.push(format!(
            "{field_indent}summary: {},",
            dart_string_literal(summary)
        ));
    }
    if let Some(description) = &metadata.description {
        fields.push(format!(
            "{field_indent}description: {},",
            dart_string_literal(description)
        ));
    }
    if !metadata.tags.is_empty() {
        let tags = metadata
            .tags
            .iter()
            .map(|value| dart_string_literal(value))
            .collect::<Vec<_>>()
            .join(", ");
        fields.push(format!("{field_indent}tags: [{tags}],"));
    }
    if metadata.deprecated {
        fields.push(format!("{field_indent}deprecated: true,"));
    }
    if metadata.hidden {
        fields.push(format!("{field_indent}hidden: true,"));
    }
    format!("const RouteMetadata(\n{}\n{indent})", fields.join("\n"))
}

/// Wraps one handler in route-local layers, preserving outermost-first order.
fn wrap_handler(name: &str, middleware: &[dust_ir::AnnotationValueIr]) -> String {
    middleware
        .iter()
        .rev()
        .fold(name.to_owned(), |handler, value| {
            format!("{}.toMiddleware()({handler})", render_const(value, true))
        })
}

/// Emits extraction, rejection, dispatch, and guard code for one handler.
fn emit_handler(handler: &HandlerPlan<'_>, controller: Option<&str>) -> String {
    let name = generated_handler_name(handler.callable.name());
    let mut lines = vec![format!("Future<Response> {name}(Request request) async {{")];
    let mut param_emitter = ParamEmitter::default();
    for (index, param) in handler.params.iter().enumerate() {
        param_emitter.push(&mut lines, index, param);
    }
    let arguments = handler
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| match param.param.kind {
            ParamKind::Positional => format!("_dustValue{index}"),
            ParamKind::Named => format!("{}: _dustValue{index}", param.param.name),
        })
        .collect::<Vec<_>>();
    let target = controller
        .map(|name| format!("(this as {name})."))
        .unwrap_or_default();
    if returns_void(handler.callable.return_type()) {
        lines.push("  return guard(() async {".to_owned());
        push_call(
            &mut lines,
            "    ",
            &target,
            handler.callable.name(),
            &arguments,
            ";",
        );
        if handler.status == 204 {
            lines.push("    return noContent();".to_owned());
        } else {
            lines.push(format!("    return Response({});", handler.status));
        }
        lines.push("  });".to_owned());
    } else {
        lines.push("  return guard(".to_owned());
        lines.push("    () async => responseFrom(".to_owned());
        push_call(
            &mut lines,
            "      ",
            &target,
            handler.callable.name(),
            &arguments,
            ",",
        );
        lines.push(format!("      status: {},", handler.status));
        lines.push("    ),".to_owned());
        lines.push("  );".to_owned());
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

/// Appends an awaited call using Dart formatter-compatible line breaks.
fn push_call(
    lines: &mut Vec<String>,
    indent: &str,
    target: &str,
    name: &str,
    arguments: &[String],
    suffix: &str,
) {
    let inline = format!(
        "{indent}await {target}{name}({}){suffix}",
        arguments.join(", ")
    );
    if arguments.len() <= 1 && inline.len() + 2 <= 80 {
        lines.push(inline);
        return;
    }
    lines.push(format!("{indent}await {target}{name}("));
    lines.extend(
        arguments
            .iter()
            .map(|argument| format!("{indent}  {argument},")),
    );
    lines.push(format!("{indent}){suffix}"));
}

/// Returns whether the direct or future-wrapped result is void.
fn returns_void(ty: &TypeIr) -> bool {
    let value = if ty.is_named("Future") && ty.args().len() == 1 {
        &ty.args()[0]
    } else {
        ty
    };
    value.is_named("void")
}
