//! Constructor lookup names come from syntax identifiers, not applied type source.

use super::*;

/// Keeps the written callee while extracting only its actual name identifiers.
pub(super) fn constructor_kind<'tree>(
    nodes: impl IntoIterator<Item = Node<'tree>>,
    source: &SourceText,
) -> Option<ParsedAnnotationValueRootKind> {
    let mut identifiers = Vec::new();
    let arguments = nodes
        .into_iter()
        .find_map(|node| callee_identifiers(node, &mut identifiers))?;
    let start = identifiers.first()?.start_byte();
    let name = source.as_str()[start..arguments.start_byte()]
        .trim_end()
        .to_owned();
    let mut identifiers: Vec<_> = identifiers
        .into_iter()
        .map(|node| node_text(node, source))
        .collect();
    let short_name = identifiers.pop()?;
    let prefix = (!identifiers.is_empty()).then(|| identifiers.join("."));
    Some(ParsedAnnotationValueRootKind::Constructor {
        name,
        short_name,
        prefix,
        arguments: Box::new(extract_annotation_arguments(arguments, source)),
    })
}

/// Stops at the invocation's argument list; type-argument identifiers never qualify it.
fn callee_identifiers<'tree>(
    node: Node<'tree>,
    names: &mut Vec<Node<'tree>>,
) -> Option<Node<'tree>> {
    if node.is_extra() || matches!(node.kind(), "type_arguments" | "label") {
        return None;
    }
    if node.kind() == "arguments" {
        return Some(node);
    }
    if matches!(node.kind(), "identifier" | "type_identifier" | "new") {
        names.push(node);
        return None;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find_map(|child| callee_identifiers(child, names))
}
