use std::collections::BTreeMap;

use dust_dart_emit::{
    dart_string_literal, render_class_type, render_type_arguments, render_type_parameters,
};
use dust_ir::{ClassIr, DartFileIr, FieldIr};
use minijinja::Environment;

use super::model::{ValidateConfig, field_validations, has_validate_trait};
use super::rule_snippets::rule_line;
use super::type_source::{class_input_kind, render_type};
use crate::features::{
    core_names::CoreNames,
    names::{NameAllocator, library_declaration_names, library_reference_names, upper_first},
};

/// Turning one validation config into the rules a generated validator runs.
mod rules;
use self::rules::*;

/// Rendering the per-field context a validator template reads.
mod fields;
use self::fields::*;

/// Serializable data passed to validation templates.
mod context;
use self::context::*;

/// Generated validation mixin member and support type source.
pub(crate) struct ValidateEmission {
    /// Mixin member inserted into the source class mixin.
    pub(crate) mixin_member: String,
    /// Private extension and public validator helper source.
    pub(crate) support_type: String,
}

/// Emits validation support for a class that derives `Validate`.
pub(crate) fn emit_validate(
    library: &DartFileIr,
    class: &ClassIr,
    emit_form_helpers: bool,
    core: &CoreNames,
) -> Option<ValidateEmission> {
    if !has_validate_trait(class) {
        return None;
    }

    let generic = !class.type_parameters.is_empty();
    let mut allocator = NameAllocator::new(if generic {
        library_reference_names(library)
    } else {
        library_declaration_names(library)
    });
    let extension_name = allocator.allocate(format!("_{}Validation", class.name));
    let nullable_name = allocator.allocate(format!("{extension_name}Value"));
    let mut method_allocator =
        NameAllocator::new(class.type_parameters.iter().map(|p| p.name.source.as_str()));
    let self_name = method_allocator.allocate("self");
    let errors_name = method_allocator.allocate("errors");
    let result_name = method_allocator.allocate("result");
    let mut throw_allocator =
        NameAllocator::new(class.type_parameters.iter().map(|p| p.name.source.as_str()));
    let invalid_errors_name = throw_allocator.allocate("errors");
    let runtime = [
        "String",
        "List",
        "RegExp",
        "ValidationResult",
        "ValidationError",
        "Valid",
        "Invalid",
        "ValidationException",
        "ValidationHelper",
    ]
    .into_iter()
    .map(|name| (name, core.get(name).to_owned()))
    .collect();
    let (fields, needs_nullable_alias) =
        render_fields(library, class, emit_form_helpers, &runtime, &nullable_name);
    let nullable_alias = needs_nullable_alias.then(|| {
        let item = allocator.allocate("T");
        format!("typedef {nullable_name}<{item}> = {item}?;")
    });
    let context = ValidateContext {
        class_name: &class.name,
        class_type: render_class_type(class),
        generic,
        runtime,
        nullable_alias,
        extension_name,
        self_name,
        errors_name,
        result_name,
        invalid_errors_name,
        fields,
    };

    Some(ValidateEmission {
        mixin_member: render_validate_template(&context, "validate_mixin", "validate_mixin.jinja"),
        support_type: render_validate_template(
            &context,
            "validate_support",
            "validate_support.jinja",
        ),
    })
}

/// Renders a validation template with the rule-line helper registered.
fn render_validate_template(
    context: &ValidateContext<'_>,
    name: &str,
    source_name: &str,
) -> String {
    let mut env = Environment::new();
    env.add_function("rule_line", rule_line);
    let source = match source_name {
        "validate_mixin.jinja" => include_str!("templates/validate_mixin.jinja"),
        "validate_support.jinja" => include_str!("templates/validate_support.jinja"),
        _ => unreachable!("unknown validate template"),
    };
    env.add_template(name, source)
        .expect("Dust validate template source must be valid");
    let rendered = env
        .get_template(name)
        .expect("Dust validate template must be registered")
        .render(context)
        .expect("Dust validate template context must satisfy template variables")
        .trim_matches('\n')
        .to_owned();
    collapse_excess_blank_lines(rendered)
}

/// Collapses accidental triple blank lines after template rendering.
fn collapse_excess_blank_lines(mut value: String) -> String {
    while value.contains("\n\n\n") {
        value = value.replace("\n\n\n", "\n\n");
    }
    value
}

/// Renders the private field validation helper signature.
fn helper_signature(
    helper_name: &str,
    class_name: &str,
    self_name: &str,
    value_name: &str,
    errors_parameter: &str,
    field_type: &str,
    uses_self: bool,
) -> String {
    if uses_self {
        format!(
            "static void {helper_name}(\n    {class_name} {self_name},\n    {field_type} {value_name},\n    {errors_parameter},\n  )"
        )
    } else {
        format!("static void {helper_name}({field_type} {value_name}, {errors_parameter})")
    }
}

/// Renders the private Flutter form input helper signature.
fn input_signature(
    input_helper_name: &str,
    class_name: &str,
    self_name: &str,
    uses_self: bool,
    string_type: &str,
    value_name: &str,
) -> String {
    if uses_self {
        format!(
            "static {string_type}? {input_helper_name}(\n    {class_name} {self_name},\n    {string_type}? {value_name},\n  )"
        )
    } else {
        format!("static {string_type}? {input_helper_name}({string_type}? {value_name})")
    }
}

/// Renders the public Flutter form validator helper signature.
fn public_input_signature(
    public_input_helper_name: &str,
    class_name: &str,
    uses_self: bool,
    self_name: &str,
    string_type: &str,
    value_name: &str,
) -> String {
    if uses_self {
        format!(
            "{string_type}? {public_input_helper_name}(\n  {class_name} {self_name},\n  {string_type}? {value_name},\n)"
        )
    } else {
        format!("{string_type}? {public_input_helper_name}({string_type}? {value_name})")
    }
}

/// Renders the parse-error message for numeric Flutter form inputs.
fn parse_error_message(configs: &[ValidateConfig]) -> String {
    let message = configs
        .iter()
        .find_map(|config| config.message.as_deref())
        .unwrap_or("Invalid number");
    dart_string_literal(message)
}
