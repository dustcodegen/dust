use std::collections::HashMap;

use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{ClassIr, FieldIr, LoweringOutcome, substitute_type};

/// Substitutes only a parent whose declaration identity is known in this library.
pub(super) fn apply_superclass(
    child: &ClassIr,
    parent: &ClassIr,
    mut fields: Vec<FieldIr>,
) -> LoweringOutcome<Vec<FieldIr>> {
    let application = child.superclass.as_ref();
    let arguments = application
        .map(|application| application.ty.args())
        .unwrap_or(&[]);
    let span = application
        .map(|application| application.span)
        .unwrap_or(child.span);
    if arguments.len() != parent.type_parameters.len() {
        return LoweringOutcome::new(fields).with_diagnostic(
            Diagnostic::error(format!(
                "Dust requires {} explicit superclass argument(s) for `{}` on `{}`; found {}",
                parent.type_parameters.len(),
                parent.name,
                child.name,
                arguments.len()
            ))
            .with_label(SourceLabel::new(
                span.file_id,
                span.range,
                "unsupported superclass application",
            )),
        );
    }
    let bindings = parent
        .type_parameters
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| (parameter.name.source.as_str(), argument))
        .collect::<HashMap<_, _>>();
    let mut diagnostics = Vec::new();
    for field in &mut fields {
        if child.fields.iter().any(|own| own.name == field.name) {
            continue;
        }
        match substitute_type(&field.ty, &bindings) {
            Ok(ty) => field.ty = ty,
            Err(shape) => diagnostics.push(
                Diagnostic::error(format!(
                    "Dust cannot substitute inherited {shape} on `{}.{}` through `{}` yet",
                    child.name, field.name, parent.name
                ))
                .with_label(SourceLabel::new(
                    span.file_id,
                    span.range,
                    "unsupported superclass substitution",
                ))
                .with_label(SourceLabel::new(
                    field.span.file_id,
                    field.span.range,
                    "inherited field declared here",
                )),
            ),
        }
    }
    LoweringOutcome {
        value: fields,
        diagnostics,
    }
}
