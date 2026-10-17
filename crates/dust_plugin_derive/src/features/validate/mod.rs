/// Renders validation mixin and support source.
mod emit;
/// Parses and stores validation annotation config.
mod model;
/// Renders rule snippets used by validation templates.
mod rule_snippets;
/// Validates validation annotation shapes.
mod rules;
/// Validates structured validation annotation configuration.
mod shape;
/// Renders Dart type source and input kind metadata.
mod type_source;

use dust_plugin_api::{PACKAGE_FEATURE_FLUTTER, PACKAGE_FEATURES_ANALYSIS_KEY, SymbolPlan};

pub(crate) use emit::emit_validate;
pub(crate) use rules::validate_validate;

/// Names actually emitted by generic validation inside the owning binder scope.
pub(crate) fn runtime_references(
    library: &dust_ir::DartFileIr,
    class: &dust_ir::ClassIr,
    forms: bool,
) -> Vec<&'static str> {
    if class.type_parameters.is_empty() || !model::has_validate_trait(class) {
        return Vec::new();
    }
    let fields = model::field_validations(library, class);
    let mut names = vec![
        "Validatable",
        "ValidationResult",
        "ValidationError",
        "Valid",
        "Invalid",
        "ValidationException",
    ];
    if !fields.is_empty() {
        names.push("List");
    }
    if forms
        && fields
            .iter()
            .any(|field| type_source::class_input_kind(class, &field.field.ty).is_some())
    {
        names.push("String");
    }
    if fields
        .iter()
        .flat_map(|f| &f.annotations)
        .any(|config| config.email || config.url)
    {
        names.push("ValidationHelper");
    }
    if fields
        .iter()
        .flat_map(|f| &f.annotations)
        .any(|config| config.regex.is_some())
    {
        names.push("RegExp");
    }
    names
}

/// Returns whether validation should emit Flutter form helper APIs.
pub(crate) fn emit_flutter_form_helpers(plan: &SymbolPlan) -> bool {
    plan.workspace_analysis()
        .contains_string_value(PACKAGE_FEATURES_ANALYSIS_KEY, PACKAGE_FEATURE_FLUTTER)
}
