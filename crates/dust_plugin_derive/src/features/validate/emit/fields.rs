//! Rendering the per-field context a validator template reads.

use super::*;

/// Builds template contexts for all validated fields on a class.
pub(super) fn render_fields(
    library: &DartFileIr,
    class: &ClassIr,
    emit_form_helpers: bool,
    runtime: &BTreeMap<&'static str, String>,
    nullable_name: &str,
) -> (Vec<FieldContext>, bool) {
    let generic = !class.type_parameters.is_empty();
    let class_type = render_class_type(class);
    let mut helper_allocator =
        NameAllocator::new(class.type_parameters.iter().map(|p| p.name.source.as_str()));
    let mut needs_nullable_alias = false;
    let fields = field_validations(library, class)
        .into_iter()
        .map(|validation| {
            let input_kind = class_input_kind(class, &validation.field.ty);
            let can_validate_input = emit_form_helpers && input_kind.is_some();
            let uses_self = validation
                .annotations
                .iter()
                .any(|config| config.must_match.is_some())
                || (generic && !can_validate_input);
            let helper_generic = generic && uses_self;
            let type_parameters = if helper_generic {
                render_type_parameters(&class.type_parameters)
            } else {
                String::new()
            };
            let type_arguments = if helper_generic {
                render_type_arguments(&class.type_parameters)
            } else {
                String::new()
            };
            let field_name = &validation.field.name;
            let field_type = render_type(&validation.field.ty);
            // A nullable candidate preserves T while letting Dart handle its effective nullability.
            let helper_type = if helper_generic && !validation.field.ty.is_nullable() {
                if matches!(validation.field.ty, dust_ir::TypeIr::Named { .. }) {
                    // Dart normalizes alias/bound nullability; avoid inferring or duplicating it.
                    needs_nullable_alias = true;
                    format!("{nullable_name}<{field_type}>")
                } else {
                    render_type(&validation.field.ty.clone().nullable())
                }
            } else {
                field_type.clone()
            };
            let mut allocator =
                NameAllocator::new(class.type_parameters.iter().map(|p| p.name.source.as_str()));
            let self_name = if uses_self {
                allocator.allocate("self")
            } else {
                "self".to_owned()
            };
            let value_name = allocator.allocate(field_name);
            let errors_name = allocator.allocate("errors");
            let nested_validation_name = allocator.allocate(format!("{value_name}Validation"));
            let nested_errors_name = allocator.allocate("nestedErrors");
            let nested_error_name = allocator.allocate("error");
            let custom_error_name = allocator.allocate(format!("{value_name}CustomError"));
            let mut input_allocator =
                NameAllocator::new(class.type_parameters.iter().map(|p| p.name.source.as_str()));
            let input_value_name = input_allocator.allocate("value");
            let parsed_name = input_allocator.allocate(field_name);
            let helper_name = format!("_validate{}", upper_first(field_name));
            let helper_name = if generic {
                helper_allocator.allocate(helper_name)
            } else {
                helper_name
            };
            let input_helper_name = format!("validate{}Input", upper_first(field_name));
            let input_helper_name = if generic {
                helper_allocator.allocate(input_helper_name)
            } else {
                input_helper_name
            };
            let public_input_helper_name =
                format!("validate{}{}Input", class.name, upper_first(field_name));
            FieldContext {
                type_arguments,
                runtime: runtime.clone(),
                name: validation.field.name.clone(),
                literal: dart_string_literal(&validation.field.name),
                value_name: value_name.clone(),
                self_name: self_name.clone(),
                errors_name: errors_name.clone(),
                parsed_name: parsed_name.clone(),
                input_value_name: input_value_name.clone(),
                nested_validation_name: nested_validation_name.clone(),
                nested_errors_name: nested_errors_name.clone(),
                nested_error_name: nested_error_name.clone(),
                custom_error_name: custom_error_name.clone(),
                helper_signature: helper_signature(
                    &format!("{helper_name}{type_parameters}"),
                    &class_type,
                    &self_name,
                    &value_name,
                    &format!(
                        "{}<{}> {errors_name}",
                        runtime["List"], runtime["ValidationError"]
                    ),
                    &helper_type,
                    uses_self,
                ),
                input_signature: input_signature(
                    &format!("{input_helper_name}{type_parameters}"),
                    &class_type,
                    &self_name,
                    uses_self,
                    &runtime["String"],
                    &input_value_name,
                ),
                public_input_signature: public_input_signature(
                    &format!("{public_input_helper_name}{type_parameters}"),
                    &class_type,
                    uses_self,
                    &self_name,
                    &runtime["String"],
                    &input_value_name,
                ),
                helper_name,
                input_helper_name,
                public_input_helper_name,
                type_source: field_type,
                nullable: validation.field.ty.is_nullable(),
                can_validate_input,
                input_kind,
                parse_error_message: parse_error_message(&validation.annotations),
                uses_self,
                configs: validation
                    .annotations
                    .iter()
                    .map(|config| render_config(validation.field, config, helper_generic))
                    .collect(),
            }
        })
        .collect();
    (fields, needs_nullable_alias)
}
