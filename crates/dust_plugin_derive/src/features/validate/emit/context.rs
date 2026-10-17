use std::collections::BTreeMap;

use serde::Serialize;

/// Template context for a class validation extension.
#[derive(Serialize)]
pub(super) struct ValidateContext<'a> {
    /// Source class name.
    pub(super) class_name: &'a str,
    /// Typed receiver at a use site, never a raw generic class.
    pub(super) class_type: String,
    /// Generic models use a static container, not an extension-scoped binder.
    pub(super) generic: bool,
    /// Generated references protected from class-scope binders.
    pub(super) runtime: BTreeMap<&'static str, String>,
    /// Transparent nullable candidate type, only when a named type needs widening.
    pub(super) nullable_alias: Option<String>,
    /// Generated private validation extension name.
    pub(super) extension_name: String,
    /// Local receiver variable name.
    pub(super) self_name: String,
    /// Local validation error list variable name.
    pub(super) errors_name: String,
    /// Local validation result variable name.
    pub(super) result_name: String,
    /// Local error list name used by `validateOrThrow`.
    pub(super) invalid_errors_name: String,
    /// Field validation contexts.
    pub(super) fields: Vec<FieldContext>,
}

/// Template context for one validated field.
#[derive(Clone, Serialize)]
pub(super) struct FieldContext {
    /// Arguments forwarded to the field helper's own generic declaration.
    pub(super) type_arguments: String,
    /// Generated references protected from helper-scope binders.
    pub(super) runtime: BTreeMap<&'static str, String>,
    /// Field name.
    pub(super) name: String,
    /// Dart string literal for the field name.
    pub(super) literal: String,
    /// Local field value variable name.
    pub(super) value_name: String,
    /// Local receiver variable name.
    pub(super) self_name: String,
    /// Local validation error list variable name.
    pub(super) errors_name: String,
    /// Local parsed Flutter form input variable name.
    pub(super) parsed_name: String,
    /// Form input variable, allocated independently of user type parameters.
    pub(super) input_value_name: String,
    /// Local nested validation result variable name.
    pub(super) nested_validation_name: String,
    /// Local nested errors variable name.
    pub(super) nested_errors_name: String,
    /// Local nested error loop variable name.
    pub(super) nested_error_name: String,
    /// Local custom validator result variable name.
    pub(super) custom_error_name: String,
    /// Private validation helper method name.
    pub(super) helper_name: String,
    /// Private Flutter form input helper method name.
    pub(super) input_helper_name: String,
    /// Public top-level Flutter form validator helper name.
    pub(super) public_input_helper_name: String,
    /// Private validation helper signature.
    pub(super) helper_signature: String,
    /// Private input helper signature.
    pub(super) input_signature: String,
    /// Public input helper signature.
    pub(super) public_input_signature: String,
    /// Rendered Dart field type source.
    pub(super) type_source: String,
    /// Whether the field type is nullable.
    pub(super) nullable: bool,
    /// Whether Flutter form input validation can be generated.
    pub(super) can_validate_input: bool,
    /// Parser kind for Flutter form input validation.
    pub(super) input_kind: Option<&'static str>,
    /// Error message used when parsing input fails.
    pub(super) parse_error_message: String,
    /// Whether validation rules need the receiver.
    pub(super) uses_self: bool,
    /// Parsed validation configs for the field.
    pub(super) configs: Vec<ConfigContext>,
}

/// Template context for one `@Validate` config.
#[derive(Clone, Serialize)]
pub(super) struct ConfigContext {
    /// Whether the field is nullable.
    pub(super) nullable: bool,
    /// Optional required-field error message.
    pub(super) required_message: Option<String>,
    /// Rule contexts rendered for this config.
    pub(super) rules: Vec<RuleContext>,
}

/// Template context for one validation rule.
#[derive(Clone, Serialize)]
pub(super) struct RuleContext {
    /// Rule kind consumed by the rule snippet renderer.
    pub(super) kind: &'static str,
    /// Dart string literal for the error message.
    pub(super) message: String,
    /// Integer rule value.
    pub(super) int_value: Option<i64>,
    /// Numeric rule value source.
    pub(super) number_value: Option<String>,
    /// Exact length value.
    pub(super) equal: Option<i64>,
    /// Dart string literal pattern.
    pub(super) pattern: Option<String>,
    /// Other field name for must-match rules.
    pub(super) other: Option<String>,
    /// Custom validator function source.
    pub(super) custom: Option<String>,
}
