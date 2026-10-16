use std::fmt::Write;

use dust_dart_emit::render_class_type;
use dust_ir::ClassIr;

use crate::features::{
    DEBUG_SYMBOL, TO_STRING_SYMBOL, core_names::CoreNames, eq_hash::has_trait, names::NameAllocator,
};

/// Emits a generated `toString` mixin member for `ToString` or `Debug`.
pub(crate) fn emit_debug_mixin(class: &ClassIr, core: &CoreNames) -> Option<String> {
    if !has_to_string_trait(class) {
        return None;
    }

    Some(if class.fields.is_empty() {
        render_empty_debug(class, core)
    } else {
        render_field_debug(class, core)
    })
}

/// Renders `toString` for classes without fields.
fn render_empty_debug(class: &ClassIr, core: &CoreNames) -> String {
    let string = core.get("String");
    format!(
        "@override\n{string} toString() {{\n  return '{}()';\n}}",
        class.name
    )
}

/// Renders `toString` for classes with fields.
fn render_field_debug(class: &ClassIr, core: &CoreNames) -> String {
    let string = core.get("String");
    let self_name =
        NameAllocator::new(class.type_parameters.iter().map(|p| p.name.source.as_str()))
            .allocate("self");
    format!(
        "@override\n{string} toString() {{\n  final {self_name} = this as {};\n  return '{}('\n{}\n      ')';\n}}",
        render_class_type(class),
        class.name,
        render_debug_field_lines(class, &self_name)
    )
}

/// Renders interpolated field lines for generated `toString`.
fn render_debug_field_lines(class: &ClassIr, self_name: &str) -> String {
    let mut out = String::with_capacity(class.fields.len() * 32);
    for (index, field) in class.fields.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let suffix = if index + 1 == class.fields.len() {
            ""
        } else {
            ", "
        };
        write!(
            out,
            "      '{}: ${{{self_name}.{}}}{}'",
            field.name, field.name, suffix
        )
        .expect("writing to String cannot fail");
    }
    out
}

/// Returns true when a class requests `ToString` or `Debug`.
fn has_to_string_trait(class: &ClassIr) -> bool {
    has_trait(class, TO_STRING_SYMBOL) || has_trait(class, DEBUG_SYMBOL)
}
