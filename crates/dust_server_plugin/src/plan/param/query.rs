use dust_diagnostics::Diagnostic;
use dust_ir::{AnnotationValueIr, ClassIr, EnumIr, MethodParamIr, TypeIr};

use crate::plan::{
    model::{Args, Extraction},
    types::{query_model_plan, require_scalar},
    values::{required_string, validate_zero_named},
};

/// Selects named scalar/list or unnamed model query extraction.
pub(super) fn query(
    param: &MethodParamIr,
    value_type: &TypeIr,
    args: &Args<'_>,
    classes: &[ClassIr],
    enums: &[EnumIr],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Extraction> {
    validate_zero_named(args, diagnostics);
    if args.positional().is_empty() || matches!(args.positional(), [AnnotationValueIr::Null]) {
        return Some(Extraction::QueryModel(query_model_plan(
            value_type,
            classes,
            enums,
            param.span,
            diagnostics,
        )));
    }
    let key = required_string(args, 0, "query name", diagnostics)?;
    if value_type.is_named("List") && value_type.args().len() == 1 {
        require_scalar(&value_type.args()[0], param.span, diagnostics);
        Some(Extraction::QueryList(key))
    } else {
        require_scalar(value_type, param.span, diagnostics);
        Some(Extraction::Query(key))
    }
}
