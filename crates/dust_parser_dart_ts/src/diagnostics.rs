use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_parser_dart::DartLanguageVersion;
use dust_text::SourceText;
use tree_sitter::{Node, Tree, TreeCursor};

use crate::language_gates::{FeatureOccurrence, record_syntax_occurrence};
use crate::syntax::{direct_named_child, node_text, text_range};

/// Native queries visit diagnostic candidates without a Rust token walk.
mod candidates;
/// Concise constructor classification shared by both traversals.
mod constructors;
/// Dependency method/function defaults, not constructor or metadata defaults.
mod defaults;

/// Effective-version filtering preserves both complete diagnostic views.
#[cfg(test)]
mod language_tests;
#[cfg(test)]
mod tests;

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

/// Extracts both views; `None` retains every syntax-feature occurrence for callers.
/// An effective version may omit only occurrences whose language gates are enabled.
pub(crate) fn extract_diagnostics(
    tree: &Tree,
    source: &SourceText,
    occurrences: &mut Vec<FeatureOccurrence>,
    version: Option<DartLanguageVersion>,
) -> DiagnosticViews {
    let mut views = DiagnosticViews::default();
    if !tree.root_node().has_error()
        && let Some(nodes) = candidates::collect(tree, source, version)
    {
        for node in nodes {
            collect_node_diagnostics(
                node,
                node.parent().map(|parent| parent.kind()),
                source,
                &mut views,
                occurrences,
                false,
            );
        }
        return views;
    }
    collect_error_diagnostics(
        &mut tree.walk(),
        None,
        source,
        &mut views,
        occurrences,
        tree.root_node().has_error(),
    );
    views
}

/// Records parse errors and grammar forms whose generation is still unsupported.
fn collect_error_diagnostics(
    cursor: &mut TreeCursor<'_>,
    parent_kind: Option<&str>,
    source: &SourceText,
    views: &mut DiagnosticViews,
    occurrences: &mut Vec<FeatureOccurrence>,
    has_syntax_errors: bool,
) {
    let node = cursor.node();
    let kind = node.kind();
    collect_node_diagnostics(
        node,
        parent_kind,
        source,
        views,
        occurrences,
        has_syntax_errors,
    );

    if cursor.goto_first_child() {
        loop {
            collect_error_diagnostics(
                cursor,
                Some(kind),
                source,
                views,
                occurrences,
                has_syntax_errors,
            );
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Both traversals retain the same feature classification and diagnostic order.
fn collect_node_diagnostics(
    node: Node<'_>,
    parent_kind: Option<&str>,
    source: &SourceText,
    views: &mut DiagnosticViews,
    occurrences: &mut Vec<FeatureOccurrence>,
    has_syntax_errors: bool,
) {
    let kind = node.kind();
    record_syntax_occurrence(node, kind, parent_kind, source, occurrences);
    if let Some(feature) = unsupported_feature(node, kind, source) {
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
            || !(is_body_only_dot(node)
                || is_instance_initializer_dot(node)
                || defaults::is_callable_default_dot(node))
        {
            views.namespace_diagnostics.push(diagnostic.clone());
        }
        views.diagnostics.push(diagnostic);
    }
    if has_syntax_errors && (node.is_error() || node.is_missing()) {
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
}

/// Keeps newly parseable syntax gated until its generated consumers are verified.
fn unsupported_feature(
    node: Node<'_>,
    kind: &str,
    source: &SourceText,
) -> Option<UnsupportedFeature> {
    match kind {
        "dot_shorthand" if !is_generic_receiver_member(node, source) => {
            Some(UnsupportedFeature::DotShorthands)
        }
        "constructor_signature" | "constant_constructor_signature"
            if constructors::is_concise_constructor(node) =>
        {
            Some(UnsupportedFeature::ConciseConstructors)
        }
        "factory_constructor_signature" | "redirecting_factory_constructor_signature"
            if constructors::is_concise_factory(node, source) =>
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
