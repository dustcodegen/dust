use dust_dart_emit::render_type_arguments;
use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{DartFileIr, TypeIr};

use crate::merge::MergedSections;

/// Prevents implicit bound instantiation from erasing generic model arguments.
pub(crate) fn validate_generic_mixins(
    library: &DartFileIr,
    merged: &MergedSections,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for class in &library.classes {
        if class.type_parameters.is_empty()
            || (merged.members_for_class(&class.name).is_empty()
                && merged.interfaces_for_class(&class.name).is_empty())
        {
            continue;
        }
        let name = format!("_${}", class.name);
        let expected = TypeIr::generic(
            name.as_str(),
            class
                .type_parameters
                .iter()
                .map(|parameter| TypeIr::named(parameter.name.source.as_str()))
                .collect(),
        );
        let mut applications = class
            .mixins
            .iter()
            .filter(|mixin| mixin.ty.name() == Some(name.as_str()));
        let first = applications.next();
        if first.is_some_and(|mixin| mixin.ty == expected) && applications.next().is_none() {
            continue;
        }
        let location = first.map(|mixin| mixin.span).unwrap_or(class.span);
        diagnostics.push(
            Diagnostic::error(format!(
                "Generic model `{}` must apply `with {name}{}`",
                class.name,
                render_type_arguments(&class.type_parameters)
            ))
            .with_label(SourceLabel::new(
                location.file_id,
                location.range,
                "forward the class type parameters in declaration order",
            )),
        );
    }
    diagnostics
}
