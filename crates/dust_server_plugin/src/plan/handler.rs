use dust_diagnostics::Diagnostic;
use dust_ir::{ClassIr, EnumIr, TypeIr};

use crate::plan::{
    body::{parse_body_media, validate_body_order},
    model::{Args, Callable, HandlerPlan},
    param::parse_param,
    types::unsupported_return,
    values::{
        error, parse_middleware, parse_route_metadata, parse_status, path_keys, required_string,
        validate_named, validate_route_path,
    },
};

/// Validates shared handler arguments and builds its route plan.
pub(super) fn parse_handler<'a>(
    callable: Callable<'a>,
    verbs: Vec<(Args<'a>, &'static str)>,
    inherited_tags: &[String],
    classes: &[ClassIr],
    enums: &[EnumIr],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<HandlerPlan<'a>> {
    if verbs.len() != 1 {
        diagnostics.push(error(
            callable.span(),
            "server handlers require exactly one HTTP verb",
            "remove the extra verb annotation",
        ));
        return None;
    }
    let (args, method) = &verbs[0];
    validate_named(
        args,
        &[
            "status",
            "summary",
            "description",
            "tags",
            "middleware",
            "operationId",
            "deprecated",
            "hidden",
        ],
        diagnostics,
    );
    let path = required_string(args, 0, "verb path", diagnostics)?;
    validate_route_path(&path, args.span(), diagnostics);
    let status =
        parse_status(args, diagnostics).unwrap_or_else(|| default_status(callable.return_type()));
    let middleware = parse_middleware(args, diagnostics);
    let metadata = parse_route_metadata(args, inherited_tags, callable.name());
    let placeholders = path_keys(&path);
    let body_media = parse_body_media(callable, diagnostics);
    let params = callable
        .params()
        .iter()
        .filter_map(|param| {
            parse_param(
                param,
                &placeholders,
                body_media,
                classes,
                enums,
                diagnostics,
            )
        })
        .collect::<Vec<_>>();
    validate_body_order(callable, body_media, &params, diagnostics);
    if unsupported_return(callable.return_type()) {
        diagnostics.push(error(
            callable.span(),
            "handler return type is not supported by generated server handlers",
            "use JSON data, Result, Option, text, bytes, a byte stream, Unit, void, or Response",
        ));
    }
    Some(HandlerPlan {
        callable,
        method,
        path,
        status,
        params,
        middleware,
        metadata,
    })
}

/// Selects the response status implied by a handler return type.
fn default_status(ty: &TypeIr) -> u16 {
    let value = if ty.is_named("Future") && ty.args().len() == 1 {
        &ty.args()[0]
    } else {
        ty
    };
    if value.is_named("void") || value.is_named("Unit") {
        204
    } else {
        200
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_default_status_from_the_return_type() {
        assert_eq!(default_status(&TypeIr::named("void")), 204);
        assert_eq!(default_status(&TypeIr::named("Unit")), 204);
        assert_eq!(
            default_status(&TypeIr::generic("Future", vec![TypeIr::named("void")],)),
            204
        );
        assert_eq!(default_status(&TypeIr::named("Todo")), 200);
    }
}
