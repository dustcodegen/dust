use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{ClassKindIr, DartFileIr};
use dust_plugin_api::SymbolPlan;

use crate::features::{
    COPY_WITH_SYMBOL, EQ_SYMBOL, TO_STRING_SYMBOL, VALIDATE_SYMBOL,
    clone_copy_with::validate_copy_with,
    eq_hash::{has_trait, validate_eq_hash},
    validate::{emit_flutter_form_helpers, validate_validate},
};
use crate::plugin::CLAIMED_TRAITS;

/// Validates derive requests for every class in a library.
pub(crate) fn validate_library(library: &DartFileIr) -> Vec<Diagnostic> {
    validate_library_inner(library, false)
}

/// Validates derive requests for every class with generation-plan context.
pub(crate) fn validate_library_with_plan(
    library: &DartFileIr,
    plan: &SymbolPlan,
) -> Vec<Diagnostic> {
    validate_library_inner(library, emit_flutter_form_helpers(plan))
}

/// Validates derive requests for every class in a library.
fn validate_library_inner(library: &DartFileIr, emit_form_helpers: bool) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for class in &library.classes {
        if matches!(
            class.kind,
            ClassKindIr::MixinClass | ClassKindIr::MixinApplicationClass
        ) && CLAIMED_TRAITS.iter().any(|symbol| has_trait(class, symbol))
        {
            let kind = if class.kind == ClassKindIr::MixinClass {
                "mixin class"
            } else {
                "named mixin-application class"
            };
            diagnostics.push(Diagnostic::error(format!(
                "Dust derive generation does not support `{kind}` targets like `{}`",
                class.name
            )));
            continue;
        }
        diagnostics.extend(validate_eq_hash(library, class));
        diagnostics.extend(validate_member_binders(class));
        diagnostics.extend(validate_copy_with(class));
        diagnostics.extend(validate_validate(library, class, emit_form_helpers));
    }
    diagnostics
}

/// Rejects class binders that Dart cannot combine with generated mixin members.
fn validate_member_binders(class: &dust_ir::ClassIr) -> Vec<Diagnostic> {
    [
        (TO_STRING_SYMBOL, "toString"),
        (EQ_SYMBOL, "hashCode"),
        (COPY_WITH_SYMBOL, "copyWith"),
        (VALIDATE_SYMBOL, "validate"),
    ]
    .into_iter()
    .filter(|(symbol, _)| has_trait(class, symbol))
    .filter_map(|(_, member)| {
        class
            .type_parameters
            .iter()
            .find(|parameter| parameter.name.source == member)
            .map(|parameter| {
                Diagnostic::error(format!(
                    "generic parameter `{member}` on `{}` conflicts with generated `{member}` member",
                    class.name
                ))
                .with_label(SourceLabel::new(
                    parameter.name.span.file_id,
                    parameter.name.span.range,
                    "Dust cannot safely rename this handwritten type parameter",
                ))
            })
    })
    .collect()
}
