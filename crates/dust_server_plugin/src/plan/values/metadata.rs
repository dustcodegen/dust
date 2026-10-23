use dust_ir::AnnotationValueIr;

use super::string_value;
use crate::plan::model::{Args, RouteMetadataPlan};

/// Returns string tags after shape validation has run.
pub(in crate::plan) fn parse_tags(args: &Args<'_>) -> Vec<String> {
    let Some(AnnotationValueIr::List(values)) = args.named().get("tags") else {
        return Vec::new();
    };
    values.iter().filter_map(string_value).collect()
}

/// Returns a structured bool value.
pub(super) fn bool_value(value: &AnnotationValueIr) -> Option<bool> {
    match value {
        AnnotationValueIr::Bool(value) => Some(*value),
        _ => None,
    }
}

/// Builds route metadata with group tags before route-local tags.
pub(in crate::plan) fn parse_route_metadata(
    args: &Args<'_>,
    inherited_tags: &[String],
    default_operation_id: &str,
) -> RouteMetadataPlan {
    let mut tags = inherited_tags.to_vec();
    tags.extend(parse_tags(args));
    RouteMetadataPlan {
        operation_id: args
            .named()
            .get("operationId")
            .and_then(string_value)
            .unwrap_or_else(|| default_operation_id.to_owned()),
        summary: args.named().get("summary").and_then(string_value),
        description: args.named().get("description").and_then(string_value),
        tags,
        deprecated: args
            .named()
            .get("deprecated")
            .and_then(bool_value)
            .unwrap_or(false),
        hidden: args
            .named()
            .get("hidden")
            .and_then(bool_value)
            .unwrap_or(false),
    }
}
