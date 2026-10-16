/// Class declaration extraction from tree-sitter nodes.
mod class_decl;
/// Constructor extraction from class members.
mod constructors;
/// Field extraction shared by classes and mixins.
pub(crate) mod fields;
/// Shared metadata attachment for wrapped and flat class bodies.
mod members;
/// Method and parameter extraction shared by classes and top-level functions.
pub(crate) mod methods;
/// Source-text helpers for parameter parsing.
mod parse_text;
/// Fields declared by primary-constructor parameters.
mod primary;

/// Cross-grammar contracts for the staged extraction migration.
#[cfg(test)]
mod parity_tests;

pub(crate) use members::annotated_members;

use dust_parser_dart::ParsedClassSurface;
use dust_text::SourceText;
use tree_sitter::Node;

/// Extracts all class declarations from a parsed Dart compilation unit.
pub(crate) fn extract_classes(root: Node<'_>, source: &SourceText) -> Vec<ParsedClassSurface> {
    let mut classes = Vec::new();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor).filter(|node| node.is_named()) {
        if matches!(child.kind(), "class_declaration" | "class_definition") {
            classes.push(class_decl::extract_class(child, source));
        }
    }

    classes
}
