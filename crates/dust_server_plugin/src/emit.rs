use dust_dart_emit::{DYNAMIC_TYPES, dart_string_literal};
use dust_ir::{ParamKind, TypeIr};
use dust_plugin_api::PluginContribution;

use crate::plan::{Extraction, HandlerPlan, ParamPlan, ServerPlan};

/// Emitter contract tests.
#[cfg(test)]
mod tests;

/// Emits one validated server plan through standard plugin contributions.
pub(crate) fn emit_server(plan: &ServerPlan<'_>) -> PluginContribution {
    let mut contribution = PluginContribution::default();
    for controller in &plan.controllers {
        contribution.push_mixin_member(
            &controller.class.name,
            emit_router(&controller.prefix, &controller.handlers),
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
fn emit_router(prefix: &str, handlers: &[HandlerPlan<'_>]) -> String {
    emit_named_router("routes", prefix, handlers)
}

/// Emits a router factory with deterministic route order.
fn emit_named_router(name: &str, prefix: &str, handlers: &[HandlerPlan<'_>]) -> String {
    let member_indent = "    ";
    let routes = handlers
        .iter()
        .map(|handler| {
            let handler_name = generated_handler_name(handler.callable.name());
            format!(
                "{member_indent}    Route({}, {}, {}),",
                dart_string_literal(handler.method),
                dart_string_literal(&handler.path),
                handler_name
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Router {name}() => Router.module(\n{member_indent}  prefix: {},\n{member_indent}  routes: [\n{routes}\n{member_indent}  ],\n{member_indent});",
        dart_string_literal(prefix)
    )
}

/// Emits extraction, rejection, dispatch, and guard code for one handler.
fn emit_handler(handler: &HandlerPlan<'_>, controller: Option<&str>) -> String {
    let name = generated_handler_name(handler.callable.name());
    let mut lines = vec![format!("Future<Response> {name}(Request request) async {{")];
    for (index, param) in handler.params.iter().enumerate() {
        let ty = DYNAMIC_TYPES.render(&param.param.ty);
        let extractor = render_extractor(param);
        lines.push(format!(
            "  final Result<{ty}, Rejection> _dustResult{index} ="
        ));
        lines.push(format!("      await {extractor}.extract(request);"));
        lines.push(format!(
            "  if (_dustResult{index} case Err(:final error)) return error.intoResponse();"
        ));
        lines.push(format!(
            "  final _dustValue{index} = (_dustResult{index} as Ok<{ty}, Rejection>).value;"
        ));
        lines.push(String::new());
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
        lines.push("    return noContent();".to_owned());
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

/// Renders the runtime extractor selected for a parameter.
fn render_extractor(plan: &ParamPlan<'_>) -> String {
    let ty = DYNAMIC_TYPES.render(&plan.param.ty);
    match &plan.extraction {
        Extraction::Path(key) => {
            format!("const PathExtractable<{ty}>({})", dart_string_literal(key))
        }
        Extraction::Query(key) => {
            format!("const QueryExtractable<{ty}>({})", dart_string_literal(key))
        }
        Extraction::QueryList(key) => {
            let item = DYNAMIC_TYPES.render(&plan.param.ty.args()[0]);
            format!(
                "const QueryListExtractable<{item}>({})",
                dart_string_literal(key)
            )
        }
        Extraction::Queries if is_string_map(&plan.param.ty) => {
            "QueriesExtractable<Map<String, String>>((queries) => queries)".to_owned()
        }
        Extraction::Queries => {
            format!("const QueriesExtractable<{ty}>(\n    {ty}.deserialize,\n  )")
        }
        Extraction::Header(name) => format!(
            "const HeaderExtractable<{ty}>({})",
            dart_string_literal(name)
        ),
        Extraction::HeaderMap => "const HeaderMapExtractable()".to_owned(),
        Extraction::Body if is_json_map(&plan.param.ty) => {
            format!("JsonExtractable<{ty}>((json) => json)")
        }
        Extraction::Body => {
            format!("const JsonExtractable<{ty}>(\n    {ty}.deserialize,\n  )")
        }
    }
}

/// Derives a private generated handler name from a Dart callable.
fn generated_handler_name(name: &str) -> String {
    let mut chars = name.chars();
    let title = chars
        .next()
        .map(|value| value.to_uppercase().collect::<String>())
        .unwrap_or_default()
        + chars.as_str();
    format!("_$handle{title}")
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

/// Returns whether a type is a string-to-string map.
fn is_string_map(ty: &TypeIr) -> bool {
    ty.is_named("Map")
        && ty.args().len() == 2
        && ty.args()[0].is_named("String")
        && ty.args()[1].is_named("String")
}

/// Returns whether a type is a JSON-object map.
fn is_json_map(ty: &TypeIr) -> bool {
    ty.is_named("Map") && ty.args().len() == 2 && ty.args()[0].is_named("String")
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
