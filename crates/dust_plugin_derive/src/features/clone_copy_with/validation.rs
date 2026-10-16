use std::collections::BTreeMap;

use dust_dart_emit::named_argument_name;
use dust_diagnostics::Diagnostic;
use dust_ir::ClassIr;

use crate::features::{COPY_WITH_SYMBOL, eq_hash::has_trait, writer::find_clone_constructor};

/// Validates construction and distinct public labels before emitting copyWith.
pub(crate) fn validate_copy_with(class: &ClassIr) -> Vec<Diagnostic> {
    if !has_trait(class, COPY_WITH_SYMBOL) {
        return Vec::new();
    }

    if class.is_abstract {
        return vec![Diagnostic::error(format!(
            "`CopyWith` cannot target abstract class `{}` because Dust cannot instantiate it",
            class.name
        ))];
    }

    let mut diagnostics = Vec::new();
    let mut public_names = BTreeMap::new();
    for field in &class.fields {
        let label = named_argument_name(&field.name);
        if let Some(previous) = public_names.insert(label, field.name.as_str()) {
            diagnostics.push(Diagnostic::error(format!(
                "`CopyWith` fields `{previous}` and `{}` on class `{}` share public argument name `{label}`",
                field.name, class.name,
            )).with_note("Rename a field before deriving CopyWith; Dust cannot emit duplicate named arguments."));
        }
    }

    if find_clone_constructor(class).is_none() {
        diagnostics.push(Diagnostic::error(format!(
            "`CopyWith` requires a constructor that accepts every field on class `{}`",
            class.name
        )));
    }
    diagnostics
}
