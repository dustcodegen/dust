use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_text::SourceText;
use tree_sitter::{Node, Tree};

use crate::language_gates::{FeatureOccurrence, record_syntax_occurrence};
use crate::syntax::{direct_named_child, node_text, text_range};

/// Two diagnostic projections of the same immutable tree and source facts.
#[derive(Default)]
pub(crate) struct DiagnosticViews {
    /// Complete parser and generation-limit diagnostics.
    pub(crate) diagnostics: Vec<Diagnostic>,
    /// Failures that prevent trusting a required namespace interface.
    pub(crate) namespace_diagnostics: Vec<Diagnostic>,
    /// Syntax guidance belongs only to the first real syntax error in both views.
    syntax_guidance_added: bool,
}

/// Distinguishes generation limits without interpreting diagnostic message text.
#[derive(Clone, Copy)]
enum UnsupportedFeature {
    /// A context-dependent dot expression not yet supported for generation.
    DotShorthands,
    /// A concise constructor whose interface facts are not yet supported.
    ConciseConstructors,
}

/// Extracts both diagnostic views in the existing syntax-occurrence walk.
pub(crate) fn extract_diagnostics(
    tree: &Tree,
    source: &SourceText,
    occurrences: &mut Vec<FeatureOccurrence>,
) -> DiagnosticViews {
    let mut views = DiagnosticViews::default();
    collect_error_diagnostics(tree.root_node(), source, &mut views, occurrences);
    views
}

/// Records parse errors and grammar forms whose generation is still unsupported.
fn collect_error_diagnostics(
    node: Node<'_>,
    source: &SourceText,
    views: &mut DiagnosticViews,
    occurrences: &mut Vec<FeatureOccurrence>,
) {
    record_syntax_occurrence(node, source, occurrences);
    if let Some(feature) = unsupported_feature(node, source) {
        let (name, guidance) = match feature {
            UnsupportedFeature::DotShorthands => (
                "dot shorthands",
                "Use an explicit type name instead of a dot shorthand.",
            ),
            UnsupportedFeature::ConciseConstructors => (
                "concise constructors",
                "Use traditional in-body constructors with the class name and explicit fields.",
            ),
        };
        let diagnostic = Diagnostic::error(format!("Dust does not support {name} yet"))
                .with_label(SourceLabel::new(source.file_id(), text_range(node), "unsupported code generation"))
                .with_note("Check this file with `dart analyze`. Dust's bundled parser accepts this syntax, but code generation is not implemented yet.")
                .with_note(guidance);
        if !matches!(feature, UnsupportedFeature::DotShorthands)
            || !(is_body_only_dot(node) || is_instance_initializer_dot(node))
        {
            views.namespace_diagnostics.push(diagnostic.clone());
        }
        views.diagnostics.push(diagnostic);
    }
    if node.is_error() || node.is_missing() {
        let mut diagnostic =
            Diagnostic::error("tree-sitter reported a Dart syntax error").with_label(
                SourceLabel::new(source.file_id(), text_range(node), "invalid syntax"),
            );
        if !views.syntax_guidance_added {
            diagnostic.notes.push("Check this file with `dart analyze`. Valid Dart may use syntax not supported by Dust's bundled parser.".to_owned());
            diagnostic.notes.push("Named concise `new` constructors are not supported yet. Use an in-body constructor with the class name.".to_owned());
            views.syntax_guidance_added = true;
        }
        views.namespace_diagnostics.push(diagnostic.clone());
        views.diagnostics.push(diagnostic);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_error_diagnostics(child, source, views, occurrences);
    }
}

/// Keeps newly parseable syntax gated until its generated consumers are verified.
fn unsupported_feature(node: Node<'_>, source: &SourceText) -> Option<UnsupportedFeature> {
    match node.kind() {
        "dot_shorthand" if !is_generic_receiver_member(node, source) => {
            Some(UnsupportedFeature::DotShorthands)
        }
        "constructor_signature" | "constant_constructor_signature"
            if is_concise_constructor(node) =>
        {
            Some(UnsupportedFeature::ConciseConstructors)
        }
        "factory_constructor_signature" | "redirecting_factory_constructor_signature"
            if is_concise_factory(node, source) =>
        {
            Some(UnsupportedFeature::ConciseConstructors)
        }
        _ => None,
    }
}

/// Only complete declared method/function bodies may hide a generation-only dot.
fn is_body_only_dot(node: Node<'_>) -> bool {
    for ancestor in std::iter::successors(node.parent(), |parent| parent.parent()) {
        if matches!(ancestor.kind(), "annotation" | "formal_parameter_list") {
            return false;
        }
        if ancestor.kind() != "function_body" || ancestor.has_error() {
            continue;
        }
        if !matches!(
            ancestor.parent().map(|parent| parent.kind()),
            Some("program" | "class_body" | "extension_body" | "enum_body")
        ) {
            continue;
        }
        let is_function = |node: Node<'_>| {
            matches!(
                node.kind(),
                "function_signature"
                    | "getter_signature"
                    | "setter_signature"
                    | "operator_signature"
            )
        };
        if ancestor.prev_named_sibling().is_some_and(|signature| {
            let mut cursor = signature.walk();
            is_function(signature)
                || (signature.kind() == "method_signature"
                    && signature.named_children(&mut cursor).any(is_function))
        }) {
            return true;
        }
    }
    false
}

/// Instance initializer expressions do not supply annotation namespace identities.
fn is_instance_initializer_dot(node: Node<'_>) -> bool {
    for ancestor in std::iter::successors(node.parent(), |parent| parent.parent()) {
        if matches!(ancestor.kind(), "annotation" | "formal_parameter_list") {
            return false;
        }
        if ancestor.kind() != "initialized_identifier" || ancestor.has_error() {
            continue;
        }
        let Some(declaration) = ancestor
            .parent()
            .filter(|parent| parent.kind() == "initialized_identifier_list")
            .and_then(|list| list.parent())
            .filter(|parent| parent.kind() == "declaration" && !parent.has_error())
            .filter(|declaration| {
                matches!(
                    declaration.parent().map(|parent| parent.kind()),
                    Some("class_body" | "enum_body")
                )
            })
        else {
            continue;
        };
        let mut cursor = declaration.walk();
        if declaration
            .children(&mut cursor)
            .any(|child| matches!(child.kind(), "static" | "const_builtin"))
        {
            return false;
        }
        let mut cursor = ancestor.walk();
        return ancestor
            .children(&mut cursor)
            .any(|child| child.kind() == "=" && child.end_byte() <= node.start_byte());
    }
    false
}

/// The grammar can parse `Type<T>.method` as nested relational expressions.
pub(crate) fn is_generic_receiver_member(node: Node<'_>, source: &SourceText) -> bool {
    let Some(close) = node.prev_named_sibling() else {
        return false;
    };
    let Some(receiver) = close
        .prev_named_sibling()
        .filter(|_| close.kind() == "relational_operator" && node_text(close, source) == ">")
    else {
        return false;
    };
    let mut cursor = receiver.walk();
    receiver.kind() == "relational_expression"
        && receiver
            .named_children(&mut cursor)
            .any(|child| child.kind() == "relational_operator" && node_text(child, source) == "<")
}

/// Only a leading `new` is concise; a traditional `Type.new` is not.
fn is_concise_constructor(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| matches!(child.kind(), "new" | "identifier"))
        .is_some_and(|child| child.kind() == "new")
}

/// Concise factories omit the containing class name from the signature.
fn is_concise_factory(node: Node<'_>, source: &SourceText) -> bool {
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
