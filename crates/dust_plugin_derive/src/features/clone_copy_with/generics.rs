use dust_dart_emit::render_type_parameter;
use dust_ir::ClassIr;

use super::CopyWithNames;

/// Introduces the result binder and the class's own parameters/bounds.
pub(super) fn helper_parameters(class: &ClassIr, names: &CopyWithNames) -> String {
    std::iter::once(names.result_type.clone())
        .chain(class.type_parameters.iter().map(render_type_parameter))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Applies a result type and the chosen actual model arguments.
pub(super) fn helper_arguments(result: &str, arguments: impl Iterator<Item = String>) -> String {
    std::iter::once(result.to_owned())
        .chain(arguments)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Forwards the source class binders at a helper use site.
pub(super) fn class_helper_arguments(class: &ClassIr, result: &str) -> String {
    helper_arguments(
        result,
        class.type_parameters.iter().map(|p| p.name.source.clone()),
    )
}
