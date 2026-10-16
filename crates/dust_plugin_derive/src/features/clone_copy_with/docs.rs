use std::fmt::Write;

use dust_dart_emit::named_argument_name;
use dust_ir::ClassIr;

use super::{
    CopyWithPlan, ValueSampleKind, nested_target_type, sample_replacement_field, sample_value,
    support::needs_copy_with_sentinel,
};
use crate::features::names::lower_first;

/// Renders Dartdoc for the generated copyWith getter.
pub(super) fn render_getter_docs(class: &ClassIr, plan: &CopyWithPlan) -> String {
    let receiver = lower_first(&class.name);
    let null_docs = if class.type_parameters.is_empty() {
        ""
    } else {
        "/// Null clears a field only when its instantiated type accepts null.\n"
    };
    let mut docs = format!(
        "/// Creates a copy of this `{}` with selected fields replaced.\n{null_docs}///\n/// Usage:\n/// ```dart",
        class.name
    );

    if let Some(field) = sample_replacement_field(class) {
        writeln!(
            docs,
            "\n/// final updated = {receiver}.copyWith({}: {});",
            named_argument_name(&field.name),
            sample_value(&field.ty, ValueSampleKind::Replacement)
        )
        .expect("writing to String cannot fail");
    } else {
        writeln!(docs, "\n/// final updated = {receiver}.copyWith();")
            .expect("writing to String cannot fail");
    }

    if let Some(field) = class
        .fields
        .iter()
        .find(|field| needs_copy_with_sentinel(&field.ty))
    {
        writeln!(
            docs,
            "/// final cleared = {receiver}.copyWith({}: null);",
            named_argument_name(&field.name)
        )
        .expect("writing to String cannot fail");
    }

    if let Some((field, target)) = class
        .fields
        .iter()
        .filter_map(|field| {
            nested_target_type(class, &field.ty).map(|(target_name, _)| (field, target_name))
        })
        .find_map(|(field, target_name)| {
            let sample = plan.samples_by_class.get(target_name)?;
            Some((field, (sample.field_name.as_str(), sample.nested_value)))
        })
    {
        writeln!(
            docs,
            "/// final nested = {receiver}.copyWith.{}({}: {});",
            named_argument_name(&field.name),
            target.0,
            target.1
        )
        .expect("writing to String cannot fail");
    }

    docs.push_str("/// ```");
    docs
}
