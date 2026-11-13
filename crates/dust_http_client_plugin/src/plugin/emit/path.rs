use std::collections::BTreeMap;

use crate::plugin::model::{EndpointParam, EndpointSpec};

/// Segment in a generated HTTP path expression.
pub(super) enum PathSegment<'a> {
    /// Literal path text copied from the annotation template.
    Literal(&'a str),
    /// Placeholder replaced by an encoded Dart parameter value.
    Binding {
        /// Placeholder key from the path template.
        key: &'a str,
        /// Dart parameter name bound to the placeholder.
        param_name: &'a str,
    },
}

/// Splits an endpoint path template into literals and parameter bindings.
pub(super) fn path_segments<'a>(endpoint: &'a EndpointSpec<'a>) -> Vec<PathSegment<'a>> {
    path_segments_from(&endpoint.path, &endpoint.params)
}

/// Splits a path using shared HTTP or WebSocket parameter bindings.
pub(super) fn path_segments_from<'a>(
    path: &'a str,
    params: &'a [EndpointParam<'a>],
) -> Vec<PathSegment<'a>> {
    let path_bindings = params
        .iter()
        .filter_map(|param| match param {
            EndpointParam::Path { param, key } => Some((key.as_str(), param.name.as_str())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();

    let mut segments = Vec::new();
    let mut cursor = 0_usize;
    while let Some(start) = path[cursor..].find('{') {
        let start = cursor + start;
        let Some(end) = path[start + 1..].find('}') else {
            break;
        };
        let end = start + 1 + end;
        if start > cursor {
            segments.push(PathSegment::Literal(&path[cursor..start]));
        }
        let key = &path[start + 1..end];
        let param_name = path_bindings.get(key).copied().unwrap_or(key);
        segments.push(PathSegment::Binding { key, param_name });
        cursor = end + 1;
    }

    if cursor < path.len() {
        segments.push(PathSegment::Literal(&path[cursor..]));
    }

    segments
}
