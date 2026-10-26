use dust_diagnostics::Diagnostic;
use dust_ir::{AnnotationValueIr, ClassIr};

use super::super::{
    model::{Args, Extraction},
    values::error,
};

/// Reads the custom extractor type from a structured annotation value.
pub(super) fn custom(
    args: &Args<'_>,
    classes: &[ClassIr],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Extraction> {
    if !args.named().is_empty() || args.positional().len() != 1 {
        diagnostics.push(error(
            args.span(),
            "Extract requires one extractor type",
            "use @Extract(MyExtractor)",
        ));
        return None;
    }
    let AnnotationValueIr::Member(name) = &args.positional()[0] else {
        diagnostics.push(error(
            args.span(),
            "Extract argument must be a type reference",
            "pass the extractor type, not an instance or expression",
        ));
        return None;
    };
    if name.prefix.is_none()
        && let Some(class) = classes.iter().find(|class| class.name == name.short)
    {
        let callable = !class.is_abstract
            && class.constructors.iter().any(|constructor| {
                constructor.name.is_none()
                    && constructor.is_const
                    && constructor.params.iter().all(|param| !param.is_required)
            });
        if !callable {
            diagnostics.push(error(
                name.span,
                format!(
                    "local extractor `{}` cannot be constructed with const {}()",
                    name.short, name.short
                ),
                "declare a concrete class with an unnamed const constructor whose parameters are optional",
            ));
            return None;
        }
    }
    Some(Extraction::Custom(name.source.clone()))
}
