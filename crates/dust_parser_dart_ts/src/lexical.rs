//! Metadata lookup regions and lexical names, independent of generated model fields.

use dust_parser_dart::{
    ParsedLexicalDeclaration, ParsedLexicalDeclarationKind as Declaration, ParsedLexicalName,
    ParsedLexicalScope, ParsedLexicalScopeKind as Scope,
};
use dust_text::{SourceText, TextRange};
use tree_sitter::Node;

use crate::syntax::{direct_named_child, node_text, text_range};

/// Original initializer values and exact callee/reference identifiers.
mod initializers;
/// Type/header visibility and callable binder scopes.
mod scopes;

/// Extracts a deterministic parent-first scope tree without a second parse.
pub(crate) fn extract(root: Node<'_>, source: &SourceText) -> Vec<ParsedLexicalScope> {
    let mut scopes = vec![ParsedLexicalScope {
        parent: None,
        name: None,
        kind: Scope::Library,
        visibility: text_range(root),
        span: text_range(root),
        declarations: Vec::new(),
    }];
    members(root, 0, source, &mut scopes);
    scopes
}

/// Walk only declaration containers, never names hidden in values or method bodies.
fn members(
    node: Node<'_>,
    parent: usize,
    source: &SourceText,
    scopes: &mut Vec<ParsedLexicalScope>,
) {
    let mut constant = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "const" | "const_builtin" => constant = true,
            ";" => constant = false,
            "class_definition"
            | "class_declaration"
            | "mixin_declaration"
            | "enum_declaration"
            | "extension_declaration"
            | "extension_type_declaration" => {
                scopes::type_scope(child, parent, source, scopes);
                constant = false;
            }
            "class_member" | "declaration" | "top_level_definition" => {
                members(child, parent, source, scopes);
            }
            "initialized_identifier_list" | "static_final_declaration_list" => {
                values(child, parent, constant, source, scopes);
            }
            "identifier_list" if scopes[parent].kind == Scope::Type => {
                let mut cursor = child.walk();
                for name in child
                    .named_children(&mut cursor)
                    .filter(|name| name.kind() == "identifier")
                {
                    declare(
                        parent,
                        name,
                        text_range(name),
                        Declaration::Member,
                        source,
                        scopes,
                    );
                }
            }
            "method_signature" | "function_signature" | "getter_signature" | "setter_signature" => {
                scopes::callable(child, parent, source, scopes);
            }
            "enum_constant" => {
                if let Some(name) = child.child_by_field_name("name") {
                    declare(
                        parent,
                        name,
                        text_range(child),
                        Declaration::EnumValue,
                        source,
                        scopes,
                    );
                }
            }
            _ => {}
        }
    }
}

/// Constness belongs to the declaration, never to a nested const initializer.
fn values(
    node: Node<'_>,
    parent: usize,
    constant: bool,
    source: &SourceText,
    scopes: &mut [ParsedLexicalScope],
) {
    let mut cursor = node.walk();
    for value in node.named_children(&mut cursor).filter(|node| {
        matches!(
            node.kind(),
            "initialized_identifier" | "static_final_declaration"
        )
    }) {
        if let Some(name) = value
            .child_by_field_name("name")
            .or_else(|| direct_named_child(value, "identifier"))
        {
            let kind = if constant {
                Declaration::Constant(initializers::extract(value, source))
            } else {
                Declaration::Member
            };
            declare(parent, name, text_range(value), kind, source, scopes);
        }
    }
}

/// Append facts in source order without modifying generation surfaces.
fn declare(
    parent: usize,
    name: Node<'_>,
    span: TextRange,
    kind: Declaration,
    source: &SourceText,
    scopes: &mut [ParsedLexicalScope],
) {
    scopes[parent].declarations.push(ParsedLexicalDeclaration {
        name: identifier(name, source),
        kind,
        span,
    });
}

/// Semantic identifiers exclude comments while their spans remain source-exact.
fn identifier(node: Node<'_>, source: &SourceText) -> ParsedLexicalName {
    ParsedLexicalName {
        name: node_text(node, source),
        span: text_range(node),
    }
}
