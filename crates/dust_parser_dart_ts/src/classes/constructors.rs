use dust_parser_dart::{
    ParameterKind, ParsedAnnotation, ParsedConstructorParamSurface, ParsedConstructorSurface,
};
use dust_text::{SourceText, TextRange};
use tree_sitter::Node;

use crate::annotations::{extract_annotation, extract_direct_annotations};
use crate::syntax::{
    direct_named_child, find_first_descendant, find_first_descendant_by, node_text, text_range,
};
use crate::types::extract_type_before;

use super::parse_text::{
    default_value_source, is_field_formal_parameter, optional_parameter_kind, parameter_name_node,
};

/// Extracts constructor metadata from a class member declaration or signature.
pub(super) fn extract_constructor(
    node: Node<'_>,
    annotations: &[ParsedAnnotation],
    source: &SourceText,
) -> ParsedConstructorSurface {
    let signature = if is_constructor_signature_kind(node.kind()) {
        Some(node)
    } else {
        find_first_descendant_by(node, |candidate| {
            is_constructor_signature_kind(candidate.kind())
        })
    };
    let Some(signature) = signature else {
        return ParsedConstructorSurface {
            name: None,
            is_factory: false,
            annotations: annotations.to_vec(),
            redirected_target_source: None,
            redirected_target_name: None,
            params: Vec::new(),
            span: text_range(node),
        };
    };
    let is_factory = matches!(
        signature.kind(),
        "factory_constructor_signature" | "redirecting_factory_constructor_signature"
    );
    let name = constructor_name(signature, source);

    let params = find_first_descendant(signature, "formal_parameter_list")
        .map(|list| extract_constructor_params(list, source))
        .unwrap_or_default();
    let (redirected_target_source, redirected_target_name) =
        redirecting_factory_target(signature, source);

    ParsedConstructorSurface {
        name,
        is_factory,
        annotations: annotations.to_vec(),
        redirected_target_source,
        redirected_target_name,
        params,
        span: text_range(signature),
    }
}

/// Returns whether a tree-sitter kind represents a constructor signature.
pub(super) fn is_constructor_signature_kind(kind: &str) -> bool {
    matches!(
        kind,
        "constant_constructor_signature"
            | "constructor_signature"
            | "factory_constructor_signature"
            | "redirecting_factory_constructor_signature"
    )
}

/// Extracts the named-constructor suffix from a constructor signature.
fn constructor_name(signature: Node<'_>, source: &SourceText) -> Option<String> {
    let boundary = direct_named_child(signature, "formal_parameter_list")?.start_byte();
    let mut cursor = signature.walk();
    let mut identifiers = signature.children(&mut cursor).filter(|child| {
        child.is_named() && child.kind() == "identifier" && child.start_byte() < boundary
    });
    identifiers.next();
    identifiers
        .next()
        .map(|identifier| node_text(identifier, source))
}

/// Extracts the target expression from a redirecting factory constructor.
fn redirecting_factory_target(
    signature: Node<'_>,
    source: &SourceText,
) -> (Option<String>, Option<String>) {
    if signature.kind() != "redirecting_factory_constructor_signature" {
        return (None, None);
    }

    let mut first_target = None;
    let mut last_target = None;
    let mut cursor = signature.walk();
    for target in signature.children_by_field_name("target", &mut cursor) {
        first_target.get_or_insert(target);
        last_target = Some(target);
    }
    let Some(first_target) = first_target else {
        let boundary = direct_named_child(signature, "formal_parameter_list")
            .map(|params| params.end_byte())
            .unwrap_or(signature.end_byte());
        let mut cursor = signature.walk();
        let first_target = signature
            .named_children(&mut cursor)
            .find(|child| child.kind() == "type_identifier" && child.start_byte() >= boundary);
        let target = first_target.map(|first| {
            source.as_str()[first.start_byte()..signature.end_byte()]
                .trim()
                .to_owned()
        });
        return (target.clone(), target);
    };
    let Some(last_target) = last_target else {
        return (None, None);
    };

    let target_end = signature
        .child_by_field_name("target_constructor")
        .map(|constructor| constructor.end_byte())
        .unwrap_or_else(|| last_target.end_byte());
    let target_source = source
        .as_str()
        .get(first_target.start_byte()..target_end)
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(ToOwned::to_owned);

    let target_name = target_source.clone();
    (target_source, target_name)
}

/// Extracts constructor parameters from a formal parameter list.
fn extract_constructor_params(
    node: Node<'_>,
    source: &SourceText,
) -> Vec<ParsedConstructorParamSurface> {
    let mut params = Vec::new();
    collect_formal_parameters(node, source, &mut params, ParameterKind::Positional);
    params
}

/// Recursively collects constructor formal parameters and their parameter kind.
fn collect_formal_parameters(
    node: Node<'_>,
    source: &SourceText,
    out: &mut Vec<ParsedConstructorParamSurface>,
    kind: ParameterKind,
) {
    let mut pending_annotations = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor).filter(|child| child.is_named()) {
        match child.kind() {
            "annotation" => pending_annotations.push(extract_annotation(child, source)),
            "formal_parameter" | "default_formal_parameter" => {
                let mut param = extract_formal_parameter(child, source, kind);
                if let Some(first) = pending_annotations.first() {
                    param.span = TextRange::new(first.span.start(), param.span.end());
                    let mut annotations = std::mem::take(&mut pending_annotations);
                    annotations.extend(param.annotations);
                    param.annotations = annotations;
                }
                out.push(param);
            }
            "optional_formal_parameters" => {
                collect_optional_formal_parameters(child, source, out);
            }
            _ => {}
        }
    }
}

/// Collects parameters inside optional positional or named parameter groups.
fn collect_optional_formal_parameters(
    node: Node<'_>,
    source: &SourceText,
    out: &mut Vec<ParsedConstructorParamSurface>,
) {
    let kind = optional_parameter_kind(node, source);
    collect_formal_parameters(node, source, out, kind);
}

/// Converts one tree-sitter parameter node into constructor parameter metadata.
fn extract_formal_parameter(
    node: Node<'_>,
    source: &SourceText,
    kind: ParameterKind,
) -> ParsedConstructorParamSurface {
    let name_node = parameter_name_node(node);
    let name = name_node
        .map(|node| node_text(node, source))
        .unwrap_or_default();
    let parsed_type = if is_field_formal_parameter(node) {
        None
    } else {
        name_node.and_then(|name| extract_type_before(node, name.start_byte(), source))
    };
    let type_source = parsed_type.as_ref().map(|ty| ty.source.clone());
    let default_value_source = default_value_source(node, source);
    let annotations = extract_direct_annotations(node, source);

    ParsedConstructorParamSurface {
        name,
        annotations,
        type_source,
        parsed_type,
        kind,
        has_default: default_value_source.is_some(),
        default_value_source,
        span: text_range(node),
    }
}
