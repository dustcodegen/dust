use std::fmt::Write;

use dust_dart_emit::{named_argument_name, render_class_type};
use dust_ir::{ClassIr, FieldIr};

use super::{
    CopyWithNames, CopyWithPlan,
    docs::render_getter_docs,
    generics::{class_helper_arguments, helper_parameters},
    nested::{render_nested_impl_getters, render_nested_interface_getters},
    support::{
        copy_with_impl_param_type, copy_with_interface_param_type, needs_copy_with_sentinel,
    },
};

/// Renders the per-class sentinel helper type and value.
pub(super) fn render_sentinel_helper(class_name: &str, value_name: &str) -> String {
    format!(
        "final class {class_name} {{\n  const {class_name}();\n}}\n\nconst {value_name} = {class_name}();"
    )
}

/// Renders the generated public `copyWith` getter for a class.
pub(super) fn render_copy_with_getter(
    class: &ClassIr,
    names: &CopyWithNames,
    plan: &CopyWithPlan,
) -> String {
    let docs = render_getter_docs(class, plan);
    let class_type = render_class_type(class);
    let arguments = class_helper_arguments(class, &class_type);
    format!(
        "{docs}\n@pragma('vm:prefer-inline')\n{}<{}> get copyWith => {}<{}>(this as {}, ({}) => {});",
        names.interface_name,
        arguments,
        names.impl_name,
        arguments,
        class_type,
        names.callback_value_name,
        names.callback_value_name
    )
}

/// Renders the copyWith callable interface and implementation types.
pub(super) fn render_copy_with_support(
    class: &ClassIr,
    names: &CopyWithNames,
    constructor_call: &str,
    plan: &CopyWithPlan,
    include_credit: bool,
) -> String {
    let interface_params = render_interface_params(class);
    let interface_getters = render_nested_interface_getters(class, names, plan);
    let impl_params = render_impl_params(class, names);
    let return_call = render_then_return(constructor_call, &names.then_name);
    let impl_getters = render_nested_impl_getters(class, names, plan);
    let parameters = helper_parameters(class, names);
    let arguments = class_helper_arguments(class, &names.result_type);
    let class_type = render_class_type(class);
    let (self_parameter, self_field) =
        if class.fields.is_empty() && !class.type_parameters.is_empty() {
            (format!("{class_type} _"), String::new())
        } else {
            (
                format!("this.{}", names.self_name),
                format!("  final {class_type} {};\n", names.self_name),
            )
        };
    let credit = if include_credit {
        "// CopyWith API inspired by Freezed.\n\n"
    } else {
        ""
    };

    format!(
        "{credit}/// @nodoc\nabstract class {}<{}> {{\n  {} call({});{}\n}}\n\n/// @nodoc\nfinal class {}<{}> implements {}<{}> {{\n  const {}({}, this.{});\n\n{self_field}  final {} Function({}) {};\n\n  @override\n  @pragma('vm:prefer-inline')\n  {} call({}) {{\n{}\n  }}{}\n}}",
        names.interface_name,
        parameters,
        names.result_type,
        interface_params,
        interface_getters,
        names.impl_name,
        parameters,
        names.interface_name,
        arguments,
        names.impl_name,
        self_parameter,
        names.then_name,
        names.result_type,
        class_type,
        names.then_name,
        names.result_type,
        impl_params,
        return_call,
        impl_getters
    )
}

/// Renders the public call parameters for the copyWith interface.
fn render_interface_params(class: &ClassIr) -> String {
    render_call_params(class, |field| {
        format!(
            "{} {}",
            copy_with_interface_param_type(&field.ty),
            named_argument_name(&field.name)
        )
    })
}

/// Renders implementation call parameters with sentinel defaults.
fn render_impl_params(class: &ClassIr, names: &CopyWithNames) -> String {
    render_call_params(class, |field| {
        let default = if names.accepts_null_helper.is_some() || needs_copy_with_sentinel(&field.ty)
        {
            names
                .sentinel
                .as_ref()
                .map(|sentinel| sentinel.value_name.as_str())
                .unwrap_or("null")
        } else {
            "null"
        };
        format!(
            "{} {} = {}",
            copy_with_impl_param_type(),
            named_argument_name(&field.name),
            default
        )
    })
}

/// Renders named call parameters for an interface or implementation.
fn render_call_params(class: &ClassIr, render_param: impl Fn(&FieldIr) -> String) -> String {
    if class.fields.is_empty() {
        return String::new();
    }

    let mut params = String::from("{\n");
    for field in &class.fields {
        writeln!(params, "    {},", render_param(field)).expect("writing to String cannot fail");
    }
    params.push_str("  }");
    params
}

/// Wraps the constructor call with the generated return callback.
fn render_then_return(constructor_call: &str, then_name: &str) -> String {
    let mut out = format!("    return {then_name}(\n");
    for line in constructor_call.lines() {
        out.push_str("      ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("    );");
    out
}
