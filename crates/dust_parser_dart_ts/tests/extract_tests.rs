//! Integration test harness for tree-sitter Dart extraction.

#[path = "extract_tests/annotation_values.rs"]
mod annotation_values;
#[path = "extract_tests/annotations.rs"]
mod annotations;
#[path = "extract_tests/class_shapes.rs"]
mod class_shapes;
#[path = "extract_tests/constructors.rs"]
mod constructors;
#[path = "extract_tests/dart_versions.rs"]
mod dart_versions;
#[path = "extract_tests/declarations.rs"]
mod declarations;
#[path = "extract_tests/diagnostics.rs"]
mod diagnostics;
#[path = "extract_tests/directive_namespaces.rs"]
mod directive_namespaces;
#[path = "extract_tests/fields.rs"]
mod fields;
#[path = "extract_tests/generic_headers.rs"]
mod generic_headers;
#[path = "extract_tests/language_occurrences.rs"]
mod language_occurrences;
#[path = "extract_tests/lexical_primary.rs"]
mod lexical_primary;
#[path = "extract_tests/library_surface.rs"]
mod library_surface;
#[path = "extract_tests/methods.rs"]
mod methods;
#[path = "extract_tests/mixin_applications.rs"]
mod mixin_applications;
#[path = "extract_tests/namespace_defaults.rs"]
mod namespace_defaults;
#[path = "extract_tests/namespace_diagnostics.rs"]
mod namespace_diagnostics;
#[path = "extract_tests/namespace_initializers.rs"]
mod namespace_initializers;
#[path = "extract_tests/primary_constructors.rs"]
mod primary_constructors;
#[path = "extract_tests/typed_annotation_values.rs"]
mod typed_annotation_values;

#[path = "extract_tests/primary_shapes.rs"]
mod primary_shapes;
#[path = "extract_tests/redirect_targets.rs"]
mod redirect_targets;
#[path = "extract_tests/static_strings.rs"]
mod static_strings;
#[path = "extract_tests/superclasses.rs"]
mod superclasses;
#[path = "extract_tests/support.rs"]
mod support;
#[path = "extract_tests/unsupported_syntax.rs"]
mod unsupported_syntax;
