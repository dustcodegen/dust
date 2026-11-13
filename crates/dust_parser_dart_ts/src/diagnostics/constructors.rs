use super::*;

/// Only a leading `new` is concise; a traditional `Type.new` is not.
pub(super) fn is_concise_constructor(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| matches!(child.kind(), "new" | "identifier"))
        .is_some_and(|child| child.kind() == "new")
}

/// Concise factories omit the containing class name from the signature.
pub(super) fn is_concise_factory(node: Node<'_>, source: &SourceText) -> bool {
    std::iter::successors(node.parent(), |parent| parent.parent())
        .find(|parent| {
            matches!(
                parent.kind(),
                "class_definition" | "class_declaration" | "enum_declaration"
            )
        })
        .is_some_and(|parent| {
            let class_name = parent
                .child_by_field_name("name")
                .or_else(|| direct_named_child(parent, "identifier"));
            let constructor_name = direct_named_child(node, "identifier");
            match (class_name, constructor_name) {
                (Some(class), Some(constructor)) => {
                    node_text(class, source) != node_text(constructor, source)
                }
                _ => true,
            }
        })
}
