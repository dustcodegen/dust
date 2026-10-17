//! Effective callable types derived separately from source-preserving facts.

use dust_diagnostics::{Diagnostic, SourceLabel};
use dust_ir::{LoweringOutcome, TypeIr};
use dust_parser_dart::{
    ParameterKind, ParsedFunctionFormalTypeSurface, ParsedTypeKind, ParsedTypeSurface,
};
use dust_text::{FileId, TextRange};

/// Lowers a declaring callback without mistaking its return type for its value type.
pub fn lower_parameter_type_ir(
    file_id: FileId,
    parsed: Option<&ParsedTypeSurface>,
    source: Option<&str>,
    function_formal: Option<&ParsedFunctionFormalTypeSurface>,
    type_parameters: &[&str],
) -> LoweringOutcome<TypeIr> {
    let Some(formal) = function_formal else {
        return crate::lower_type_ir_in_scope(parsed, source, type_parameters);
    };
    match signature(formal) {
        Ok(signature) => {
            let ty = TypeIr::function(signature);
            LoweringOutcome::new(if formal.nullable { ty.nullable() } else { ty })
        }
        Err((span, message)) => LoweringOutcome {
            value: TypeIr::unknown(),
            diagnostics: vec![Diagnostic::error("Cannot lower declaring callback type")
                .with_label(SourceLabel::new(file_id, span, message))
                .with_note("Write an explicit, supported Function(...) type instead; Dust does not infer callback types.")],
        },
    }
}

/// An original source range and the reason a callable cannot be lowered.
type Failure = (TextRange, &'static str);

/// Builds a separate effective type, never a replacement for any raw source fact.
fn signature(formal: &ParsedFunctionFormalTypeSurface) -> Result<String, Failure> {
    let return_type = explicit_type(formal.return_type.as_ref(), formal.name.span)?;
    let mut result = format!("{return_type} Function");
    if !formal.type_parameters.is_empty() {
        let mut binders = Vec::new();
        for parameter in &formal.type_parameters {
            let mut binder = parameter.name.clone();
            if let Some(bound) = &parameter.bound {
                binder.push_str(" extends ");
                binder.push_str(explicit_type(Some(bound), parameter.span)?);
            }
            binders.push(binder);
        }
        result.push('<');
        result.push_str(&binders.join(", "));
        result.push('>');
    }
    let mut required = Vec::new();
    let mut optional = Vec::new();
    let mut named = Vec::new();
    for parameter in &formal.parameters {
        if parameter.name.name.is_empty() {
            return Err((parameter.span, "The callback parameter has no source name"));
        }
        if parameter.default_value_source.is_some() {
            return Err((
                parameter.span,
                "Defaults are not valid inside a function type",
            ));
        }
        let ty = if let Some(nested) = &parameter.function_formal {
            let mut ty = signature(nested)?;
            if nested.nullable {
                ty.push('?');
            }
            ty
        } else {
            explicit_type(parameter.parsed_type.as_ref(), parameter.name.span)?.to_owned()
        };
        let value = format!("{ty} {}", parameter.name.name);
        match (parameter.kind, parameter.is_required) {
            (ParameterKind::Positional, true) => required.push(value),
            (ParameterKind::Positional, false) => optional.push(value),
            (ParameterKind::Named, true) => named.push(format!("required {value}")),
            (ParameterKind::Named, false) => named.push(value),
        }
    }
    if !optional.is_empty() {
        required.push(format!("[{}]", optional.join(", ")));
    }
    if !named.is_empty() {
        required.push(format!("{{{}}}", named.join(", ")));
    }
    result.push('(');
    result.push_str(&required.join(", "));
    result.push(')');
    Ok(result)
}

/// Inference and opaque structured types never fall back to a scalar return type.
fn explicit_type(parsed: Option<&ParsedTypeSurface>, span: TextRange) -> Result<&str, Failure> {
    let Some(parsed) = parsed else {
        return Err((
            span,
            "An explicit callback return/parameter type is required",
        ));
    };
    if unsupported(parsed) {
        return Err((parsed.span, "This callback type is not supported"));
    }
    Ok(&parsed.source)
}

/// Unknown nested arguments must not be hidden by a known generic outer type.
fn unsupported(parsed: &ParsedTypeSurface) -> bool {
    match &parsed.kind {
        ParsedTypeKind::Unknown => true,
        ParsedTypeKind::Named { args, .. } => args.iter().any(unsupported),
        _ => false,
    }
}
