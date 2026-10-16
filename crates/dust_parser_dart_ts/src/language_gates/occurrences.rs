//! Language feature locations come from syntax, never documentation/string text.

use super::*;

/// One detected language-versioned feature use.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FeatureOccurrence {
    /// Detected feature.
    pub(super) feature: DartLanguageFeature,
    /// Original UTF-8 byte range of the feature.
    pub(super) range: TextRange,
}

/// Token kinds keep strings inert while still visiting their interpolation code.
pub(crate) fn record_syntax_occurrence(
    node: Node<'_>,
    source: &SourceText,
    occurrences: &mut Vec<FeatureOccurrence>,
) {
    let feature = match node.kind() {
        "record_type" | "record_literal" => Some(DartLanguageFeature::Records),
        "pattern_variable_declaration"
        | "pattern_assignment"
        | "switch_expression"
        | "variable_pattern"
        | "list_pattern"
        | "map_pattern"
        | "record_pattern"
        | "object_pattern"
        | "cast_pattern"
        | "null_check_pattern"
        | "null_assert_pattern" => Some(DartLanguageFeature::Patterns),
        "case"
            if matches!(
                node.parent().map(|parent| parent.kind()),
                Some("if_statement" | "if_element")
            ) =>
        {
            Some(DartLanguageFeature::Patterns)
        }
        "base" | "interface" | "sealed" | "final" | "mixin" => {
            let parent = node.parent().map(|parent| parent.kind());
            (matches!(parent, Some("class_definition" | "class_declaration"))
                || node.kind() == "base" && parent == Some("mixin_declaration"))
            .then_some(DartLanguageFeature::ClassModifiers)
        }
        "?" if matches!(
            node.parent().map(|parent| parent.kind()),
            Some("list_literal" | "set_or_map_literal" | "pair" | "if_element" | "for_element")
        ) =>
        {
            Some(DartLanguageFeature::NullAwareCollectionElements)
        }
        "dot_shorthand" if !crate::diagnostics::is_generic_receiver_member(node, source) => {
            Some(DartLanguageFeature::DotShorthands)
        }
        _ => None,
    };
    if let Some(feature) = feature {
        push_range_occurrence(
            occurrences,
            feature,
            Some(TextRange::at(node.start_byte(), 1usize)),
        );
    }
}

/// Records the first real use, not the first matching word in raw source.
pub(super) fn push_range_occurrence(
    occurrences: &mut Vec<FeatureOccurrence>,
    feature: DartLanguageFeature,
    range: Option<TextRange>,
) {
    if occurrences
        .iter()
        .any(|occurrence| occurrence.feature == feature)
    {
        return;
    }
    if let Some(range) = range {
        occurrences.push(FeatureOccurrence { feature, range });
    }
}

/// Finds private named parameters in parser-owned constructor/method/function facts.
pub(super) fn find_private_named_parameter(library: &ParsedDartFileSurface) -> Option<TextRange> {
    library
        .classes
        .iter()
        .flat_map(|class| class.constructors.iter())
        .flat_map(|constructor| constructor.params.iter())
        .find(|param| param.kind == ParameterKind::Named && param.name.starts_with('_'))
        .map(|param| param.span)
        .or_else(|| {
            library
                .classes
                .iter()
                .flat_map(|class| class.methods.iter())
                .flat_map(|method| method.params.iter())
                .find(|param| param.kind == ParameterKind::Named && param.name.starts_with('_'))
                .map(|param| param.span)
        })
        .or_else(|| {
            library
                .functions
                .iter()
                .flat_map(|function| function.params.iter())
                .find(|param| param.kind == ParameterKind::Named && param.name.starts_with('_'))
                .map(|param| param.span)
        })
}
