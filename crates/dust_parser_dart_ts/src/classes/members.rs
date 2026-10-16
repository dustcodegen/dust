//! Keep metadata attached identically in class and mixin bodies.

use dust_parser_dart::ParsedAnnotation;
use dust_text::SourceText;
use tree_sitter::Node;

use crate::{
    annotations::{extract_annotation, extract_member_annotations},
    syntax::first_non_annotation_named_child,
};

/// Reads both grammars' member shapes without treating comments or bodies as members.
pub(crate) fn annotated_members<'tree>(
    body: Node<'tree>,
    source: &SourceText,
) -> Vec<(Node<'tree>, Vec<ParsedAnnotation>)> {
    let mut members = Vec::new();
    let mut pending = Vec::new();
    let mut cursor = body.walk();
    for member in body.named_children(&mut cursor) {
        if member.kind() == "annotation" {
            pending.push(extract_annotation(member, source));
            continue;
        }
        let declaration = match member.kind() {
            "class_member" => first_non_annotation_named_child(member),
            "declaration" | "method_signature" => Some(member),
            _ => None,
        };
        if let Some(declaration) = declaration {
            let annotations = if member.kind() == "class_member" {
                extract_member_annotations(member, source)
            } else {
                std::mem::take(&mut pending)
            };
            members.push((declaration, annotations));
        }
    }
    members
}
