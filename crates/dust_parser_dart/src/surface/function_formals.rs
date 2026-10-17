//! Source-owned signatures of declaring function-formal parameters.

use dust_text::TextRange;

use super::{ParameterKind, ParsedAnnotation, ParsedLexicalName, ParsedTypeParamSurface};
use crate::ParsedTypeSurface;

/// A callback declared as `Return name<T>(parameters)` rather than `Function`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFunctionFormalTypeSurface {
    /// The callback's original name and identifier span.
    pub name: ParsedLexicalName,
    /// The written return type; absent types are not inferred here.
    pub return_type: Option<ParsedTypeSurface>,
    /// Callback-local generic binders and their written bounds.
    pub type_parameters: Vec<ParsedTypeParamSurface>,
    /// Only the parameters owned by this callback signature.
    pub parameters: Vec<ParsedFunctionFormalParameterSurface>,
    /// Whether the function itself, independently of its return type, is nullable.
    pub nullable: bool,
    /// The original declaring parameter span.
    pub span: TextRange,
}

/// One parameter inside a declaring callback signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFunctionFormalParameterSurface {
    /// The original parameter name and identifier span.
    pub name: ParsedLexicalName,
    /// The written parameter type, or return type for a nested callback.
    pub parsed_type: Option<ParsedTypeSurface>,
    /// A nested declaring callback, if present.
    pub function_formal: Option<ParsedFunctionFormalTypeSurface>,
    /// Positional or named invocation style.
    pub kind: ParameterKind,
    /// Whether invocation must supply this argument.
    pub is_required: bool,
    /// Original metadata attached to this signature parameter.
    pub annotations: Vec<ParsedAnnotation>,
    /// A written default, retained even when invalid in a function type.
    pub default_value_source: Option<String>,
    /// The original parameter span, including attached metadata.
    pub span: TextRange,
}

impl ParsedFunctionFormalTypeSurface {
    /// Collects signature metadata without attaching it to the outer parameter.
    pub fn annotations(&self) -> Vec<&ParsedAnnotation> {
        let mut annotations = Vec::new();
        for parameter in &self.parameters {
            annotations.extend(&parameter.annotations);
            if let Some(nested) = &parameter.function_formal {
                annotations.extend(nested.annotations());
            }
        }
        annotations
    }
}
