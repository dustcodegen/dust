use dust_diagnostics::Diagnostic;

use super::{
    model::{Args, GroupPlan},
    values::{
        parse_middleware, parse_tags, required_string, string_value, validate_named,
        validate_route_path,
    },
};

/// Reads a controller or top-level route-group annotation.
pub(super) fn parse_group(
    args: Args<'_>,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<GroupPlan> {
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
