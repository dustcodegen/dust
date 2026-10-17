use super::*;
use crate::syntax::{direct_child, has_direct_child_kind};

/// A type's own outer metadata sees its parent; header/body metadata sees the type.
pub(super) fn type_scope(
    node: Node<'_>,
    parent: usize,
    source: &SourceText,
    scopes: &mut Vec<ParsedLexicalScope>,
) {
    let header = crate::classes::class_header(node);
    let body = node
        .child_by_field_name("body")
        .or_else(|| direct_named_child(node, "class_body"));
    let parameters = direct_named_child(header, "type_parameters").or_else(|| {
        header
            .child_by_field_name("name")
            .and_then(|name| direct_named_child(name, "type_parameters"))
    });
    let primary = node
        .child_by_field_name("primary_constructor")
        .or_else(|| node.child_by_field_name("representation"));
    let primary_visibility = primary.map(|primary| {
        primary
            .child_by_field_name("parameters")
            .or_else(|| direct_child(primary, "("))
            .unwrap_or(primary)
    });
    let Some(first) = parameters.or(primary_visibility).or(body) else {
        return;
    };
    let owner = header
        .child_by_field_name("name")
        .or_else(|| direct_named_child(header, "identifier"))
        .map(|name| direct_named_child(name, "identifier").unwrap_or(name));
    let index = scopes.len();
    scopes.push(ParsedLexicalScope {
        parent: Some(parent),
        name: owner.map(|name| identifier(name, source)),
        kind: Scope::Type,
        visibility: TextRange::new(text_range(first).start(), text_range(node).end()),
        span: text_range(node),
        declarations: Vec::new(),
    });
    binders(parameters, index, source, scopes);
    if let Some(primary) = primary {
        primary_fields(primary, index, source, scopes);
    }
    if let Some(body) = body {
        members(body, index, source, scopes);
    }
}

/// Ordinary formal parameters are not visible in their own metadata expressions.
pub(super) fn callable(
    node: Node<'_>,
    parent: usize,
    source: &SourceText,
    scopes: &mut Vec<ParsedLexicalScope>,
) {
    let signature = crate::classes::methods::callable_signature(node);
    let name = signature
        .child_by_field_name("name")
        .or_else(|| direct_named_child(signature, "identifier"));
    if scopes[parent].kind == Scope::Type {
        if let Some(name) = name {
            declare(
                parent,
                name,
                text_range(node),
                Declaration::Member,
                source,
                scopes,
            );
        }
    }
    let Some(parameters) = direct_named_child(signature, "type_parameters") else {
        return;
    };
    let index = scopes.len();
    scopes.push(ParsedLexicalScope {
        parent: Some(parent),
        name: name.map(|name| identifier(name, source)),
        kind: Scope::Callable,
        visibility: TextRange::new(text_range(parameters).start(), text_range(signature).end()),
        span: text_range(node),
        declarations: Vec::new(),
    });
    binders(Some(parameters), index, source, scopes);
}

/// Binders retain their own identifier spans, not names found in bound arguments.
fn binders(
    parameters: Option<Node<'_>>,
    parent: usize,
    source: &SourceText,
    scopes: &mut [ParsedLexicalScope],
) {
    let Some(parameters) = parameters else { return };
    let mut cursor = parameters.walk();
    for parameter in parameters
        .named_children(&mut cursor)
        .filter(|node| node.kind() == "type_parameter")
    {
        if let Some(name) = direct_named_child(parameter, "type_identifier")
            .or_else(|| direct_named_child(parameter, "identifier"))
        {
            declare(
                parent,
                name,
                text_range(parameter),
                Declaration::TypeParameter,
                source,
                scopes,
            );
        }
    }
}

/// Only declaring primary parameters introduce member names, not ordinary formals.
fn primary_fields(
    node: Node<'_>,
    parent: usize,
    source: &SourceText,
    scopes: &mut [ParsedLexicalScope],
) {
    if node.kind() == "formal_parameter" {
        if has_direct_child_kind(node, "final_builtin")
            || has_direct_child_kind(node, "inferred_type")
        {
            if let Some(name) = crate::classes::parameter_name_node(node) {
                declare(
                    parent,
                    name,
                    text_range(node),
                    Declaration::Member,
                    source,
                    scopes,
                );
            }
        }
        return;
    }
    if matches!(
        node.kind(),
        "representation_declaration" | "extension_type_representation"
    ) {
        if let Some(name) = node.child_by_field_name("name") {
            let span = node
                .child_by_field_name("type")
                .map_or(text_range(name), |ty| {
                    TextRange::new(text_range(ty).start(), text_range(name).end())
                });
            declare(parent, name, span, Declaration::Member, source, scopes);
            return;
        }
    }
    let mut cursor = node.walk();
    for child in node
        .named_children(&mut cursor)
        .filter(|node| !matches!(node.kind(), "annotation" | "type_arguments"))
    {
        primary_fields(child, parent, source, scopes);
    }
}
