use tree_sitter::Node;

/// Only defaults owned by declared functions/methods are namespace-only values.
pub(super) fn is_callable_default_dot(node: Node<'_>) -> bool {
    let mut expression = node;
    while let Some(parent) = expression.parent() {
        if matches!(
            parent.kind(),
            "annotation" | "formal_parameter" | "formal_parameter_list"
        ) {
            return false;
        }
        if parent.kind() == "optional_formal_parameters" {
            let signature = parent
                .parent()
                .filter(|list| list.kind() == "formal_parameter_list")
                .and_then(|list| list.parent());
            if parent.has_error()
                || !signature.is_some_and(|signature| {
                    signature.kind() == "function_signature" && !signature.has_error()
                })
            {
                return false;
            }
            // The grammar flattens defaults beside parameters; do not cross a sibling parameter.
            return std::iter::successors(expression.prev_sibling(), |sibling| {
                sibling.prev_sibling()
            })
            .take_while(|sibling| {
                !matches!(sibling.kind(), "," | "formal_parameter" | "annotation")
            })
            .any(|sibling| sibling.kind() == "=");
        }
        expression = parent;
    }
    false
}
