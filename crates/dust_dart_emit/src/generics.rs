use dust_ir::{ClassIr, TypeParamIr};

use crate::DYNAMIC_TYPES;

/// Renders a declaration's ordered type parameters and written upper bounds.
pub fn render_type_parameters(parameters: &[TypeParamIr]) -> String {
    bracketed(parameters.iter().map(render_type_parameter))
}

/// Renders one declaration binder, including its optional written upper bound.
pub fn render_type_parameter(parameter: &TypeParamIr) -> String {
    match &parameter.bound {
        Some(bound) => format!(
            "{} extends {}",
            parameter.name.source,
            DYNAMIC_TYPES.render(bound)
        ),
        None => parameter.name.source.clone(),
    }
}

/// Renders a class at a use site, forwarding its owning type parameters.
pub fn render_class_type(class: &ClassIr) -> String {
    format!(
        "{}{}",
        class.name,
        render_type_arguments(&class.type_parameters)
    )
}

/// Renders a use site's ordered type-parameter names, without declaration bounds.
pub fn render_type_arguments(parameters: &[TypeParamIr]) -> String {
    bracketed(
        parameters
            .iter()
            .map(|parameter| parameter.name.source.clone()),
    )
}

/// Omits angle brackets for non-generic declarations and use sites.
fn bracketed(items: impl Iterator<Item = String>) -> String {
    let inner = items.collect::<Vec<_>>().join(", ");
    if inner.is_empty() {
        String::new()
    } else {
        format!("<{inner}>")
    }
}
