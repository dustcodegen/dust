use std::fmt::Write;

use dust_dart_emit::{OBJECT_NULLABLE_TYPES, named_argument_name};
use dust_ir::{ClassIr, FieldIr};

use super::{CopyWithNames, CopyWithPlan, generics::helper_arguments, nested_target_type};

/// Renders nested copyWith getters on the public interface.
pub(super) fn render_nested_interface_getters(
    class: &ClassIr,
    names: &CopyWithNames,
    plan: &CopyWithPlan,
) -> String {
    let mut out = String::new();
    for field in &class.fields {
        let Some(target) = nested_target(field, class, names, plan) else {
            continue;
        };
        write!(
            out,
            "\n\n  {}{} get {};",
            target.interface_type,
            if target.nullable { "?" } else { "" },
            named_argument_name(&field.name)
        )
        .expect("writing to String cannot fail");
    }
    out
}

/// Renders nested copyWith getter implementations.
pub(super) fn render_nested_impl_getters(
    class: &ClassIr,
    names: &CopyWithNames,
    plan: &CopyWithPlan,
) -> String {
    let mut out = String::new();
    let pragma = names.core.get("pragma");
    for field in &class.fields {
        let Some(target) = nested_target(field, class, names, plan) else {
            continue;
        };
        if target.nullable {
            let value_name = names
                .nested_value_names
                .get(&field.name)
                .map(String::as_str)
                .unwrap_or(field.name.as_str());
            write!(out,
                "\n\n  @override\n  @{pragma}('vm:prefer-inline')\n  {}? get {} {{\n    final {} = {}.{};\n    if ({} == null) {{\n      return null;\n    }}\n\n    return {}(\n      {},\n      ({}) => call({}: {}),\n    );\n  }}",
                target.interface_type, named_argument_name(&field.name),
                value_name, names.self_name, field.name, value_name,
                target.impl_type, value_name, names.callback_value_name,
                named_argument_name(&field.name), names.callback_value_name)
                .expect("writing to String cannot fail");
        } else {
            write!(out,
                "\n\n  @override\n  @{pragma}('vm:prefer-inline')\n  {} get {} {{\n    return {}(\n      {}.{},\n      ({}) => call({}: {}),\n    );\n  }}",
                target.interface_type, named_argument_name(&field.name), target.impl_type,
                names.self_name, field.name, names.callback_value_name,
                named_argument_name(&field.name), names.callback_value_name)
                .expect("writing to String cannot fail");
        }
    }
    out
}

/// Resolves a same-library target with the field's actual generic arguments.
fn nested_target(
    field: &FieldIr,
    class: &ClassIr,
    parent: &CopyWithNames,
    plan: &CopyWithPlan,
) -> Option<NestedTarget> {
    let (name, nullable) = nested_target_type(class, &field.ty)?;
    let names = plan.names_for(name)?;
    if field.ty.args().len() != names.type_parameter_count {
        return None;
    }
    let arguments = helper_arguments(
        &parent.result_type,
        field
            .ty
            .args()
            .iter()
            .map(|arg| OBJECT_NULLABLE_TYPES.render(arg)),
    );
    Some(NestedTarget {
        interface_type: format!("{}<{arguments}>", names.interface_name),
        impl_type: format!("{}<{arguments}>", names.impl_name),
        nullable,
    })
}

/// Instantiated same-library helper types for one nested field.
struct NestedTarget {
    /// Callable interface with actual child arguments and the parent's result.
    interface_type: String,
    /// Implementation with the same arguments.
    impl_type: String,
    /// Whether the field and chained getter can be absent.
    nullable: bool,
}
