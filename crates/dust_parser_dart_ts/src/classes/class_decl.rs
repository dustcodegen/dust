use dust_parser_dart::{ParsedClassKind, ParsedClassSurface, ParsedTypeKind};
use dust_text::SourceText;
use tree_sitter::Node;

use crate::{
    annotations::extract_annotation,
    syntax::{has_direct_child_kind, node_text, text_range},
};

use super::{
    annotated_members,
    constructors::{extract_constructor, is_constructor_signature_kind},
    fields::extract_fields_from_identifier_list,
    generics::{extract_class_mixins, extract_class_superclass, extract_class_type_parameters},
    methods::extract_method,
    primary::extract_declaring_fields,
};

/// Extracts a Dust class surface from one tree-sitter class declaration.
pub(super) fn extract_class(node: Node<'_>, source: &SourceText) -> ParsedClassSurface {
    let is_sealed = has_direct_child_kind(node, "sealed");
    let kind = if has_direct_child_kind(node, "mixin") {
        ParsedClassKind::MixinClass
    } else if is_sealed {
        ParsedClassKind::SealedClass
    } else {
        ParsedClassKind::Class
    };
    let is_abstract = has_direct_child_kind(node, "abstract");
    let is_interface = has_direct_child_kind(node, "interface");
    let class_name = node
        .child_by_field_name("name")
        .map(|name| node_text(name, source))
        .unwrap_or_default();
    let superclass = extract_class_superclass(node, source);
    let superclass_name = superclass
        .as_ref()
        .and_then(|superclass| match &superclass.kind {
            ParsedTypeKind::Named { name, .. } | ParsedTypeKind::Builtin(name) => {
                Some(name.clone())
            }
            _ => None,
        });
    let mut annotations = Vec::new();
    let mut fields = Vec::new();
    let mut constructors = Vec::new();
    let mut methods = Vec::new();
    if let Some(primary) = node.child_by_field_name("primary_constructor") {
        let constructor = extract_constructor(primary, &[], source);
        fields.extend(extract_declaring_fields(primary, &constructor.params));
        constructors.push(constructor);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor).filter(|child| child.is_named()) {
        if child.kind() == "annotation" {
            annotations.push(extract_annotation(child, source));
        }
    }

    if let Some(body) = node.child_by_field_name("body") {
        for (declaration, member_annotations) in annotated_members(body, source) {
            match classify_class_member(declaration) {
                Some(ClassMemberShape::PrimaryBody) => {
                    if let Some(constructor) = constructors.first_mut() {
                        constructor.annotations.extend(member_annotations);
                    }
                }
                Some(ClassMemberShape::Constructor(signature)) => {
                    constructors.push(extract_constructor(signature, &member_annotations, source));
                }
                Some(ClassMemberShape::Field(identifier_list)) => {
                    fields.extend(extract_fields_from_identifier_list(
                        declaration,
                        identifier_list,
                        &member_annotations,
                        source,
                    ));
                }
                Some(ClassMemberShape::Method) => {
                    if let Some(method) = extract_method(declaration, &member_annotations, source) {
                        methods.push(method);
                    }
                }
                None => {}
            }
        }
    }

    ParsedClassSurface {
        kind,
        name: class_name,
        type_parameters: extract_class_type_parameters(node, source),
        mixins: extract_class_mixins(node, source),
        is_abstract,
        is_interface,
        superclass_name,
        superclass,
        annotations,
        fields,
        constructors,
        methods,
        span: text_range(node),
    }
}

/// Shape of one class member declaration.
enum ClassMemberShape<'tree> {
    /// Metadata on the in-body part belongs to the header constructor.
    PrimaryBody,
    /// Constructor signature node.
    Constructor(Node<'tree>),
    /// Initialized identifier list for fields.
    Field(Node<'tree>),
    /// Method-like member.
    Method,
}

/// Classifies a class member into a shape Dust extracts.
fn classify_class_member<'tree>(node: Node<'tree>) -> Option<ClassMemberShape<'tree>> {
    if node.kind() == "primary_constructor_body_signature" {
        return Some(ClassMemberShape::PrimaryBody);
    }
    if is_constructor_signature_kind(node.kind()) {
        return Some(ClassMemberShape::Constructor(node));
    }
    if node.kind() == "initialized_identifier_list" {
        return Some(ClassMemberShape::Field(node));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor).filter(|child| child.is_named()) {
        if let Some(shape) = classify_class_member(child) {
            return Some(shape);
        }
    }

    matches!(
        node.kind(),
        "declaration" | "method_signature" | "function_signature" | "operator_signature"
    )
    .then_some(ClassMemberShape::Method)
}
