use crate::{FieldIr, NameIr, SpanIr, TypeIr};

/// The parameter style used by a lowered constructor parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamKind {
    /// A positional parameter.
    Positional,
    /// A named parameter.
    Named,
}

/// One lowered constructor parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructorParamIr {
    /// The parameter name.
    pub name: String,
    /// The normalized parameter type.
    pub ty: TypeIr,
    /// The source span for the parameter.
    pub span: SpanIr,
    /// The parameter style.
    pub kind: ParamKind,
    /// Whether callers must supply this argument, independently of nullability.
    pub is_required: bool,
    /// Whether the parameter has a default value.
    pub has_default: bool,
    /// The raw default value expression source, if explicitly written.
    pub default_value_source: Option<String>,
}

/// One lowered constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructorIr {
    /// The named constructor suffix, if present.
    pub name: Option<String>,
    /// Whether the constructor is declared with the `factory` modifier.
    pub is_factory: bool,
    /// The redirected target symbol reference, if the constructor redirects.
    pub redirected_target_source: Option<String>,
    /// The redirected target base name, if it could be extracted.
    pub redirected_target_name: Option<String>,
    /// Structured target type and named constructor, when available.
    pub redirected_target: Option<RedirectTargetIr>,
    /// The source span for the constructor.
    pub span: SpanIr,
    /// The lowered constructor parameters.
    pub params: Vec<ConstructorParamIr>,
}

/// An applied redirect target, separate from its declaration name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedirectTargetIr {
    /// The applied target type in the owning class's parameter scope.
    pub ty: TypeIr,
    /// Optional named constructor with its original span.
    pub constructor: Option<NameIr>,
    /// The complete redirect target span.
    pub span: SpanIr,
}

impl ConstructorIr {
    /// Returns `true` if this constructor can initialize every field.
    ///
    /// The current rule is intentionally simple:
    /// a field is considered constructible if it either has a default value
    /// or there is a parameter with the same name.
    pub fn can_construct_all_fields(&self, fields: &[FieldIr]) -> bool {
        fields.iter().all(|field| {
            field.has_default || self.params.iter().any(|param| param.name == field.name)
        })
    }
}
