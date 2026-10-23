use dust_ir::{
    AnnotationIr, AnnotationValueIr, ClassIr, ConfigApplicationIr, FunctionIr, MethodIr,
    MethodParamIr, SpanIr, TypeIr,
};

/// Canonical server verb symbols and emitted HTTP methods.
pub(super) const VERBS: &[(&str, &str)] = &[
    ("dust_server::GET", "GET"),
    ("dust_server::POST", "POST"),
    ("dust_server::PUT", "PUT"),
    ("dust_server::PATCH", "PATCH"),
    ("dust_server::DELETE", "DELETE"),
    ("dust_server::HEAD", "HEAD"),
    ("dust_server::OPTIONS", "OPTIONS"),
];

/// Complete generated-server plan for one source library.
pub(crate) struct ServerPlan<'a> {
    /// Controller-scoped route groups.
    pub(crate) controllers: Vec<ControllerPlan<'a>>,
    /// Top-level handlers in source order.
    pub(crate) functions: Vec<HandlerPlan<'a>>,
    /// Prefix applied to the top-level route group.
    pub(crate) function_prefix: String,
    /// Generated top-level router factory name.
    pub(crate) function_name: String,
}

/// One controller and its generated handler plans.
pub(crate) struct ControllerPlan<'a> {
    /// Source controller class.
    pub(crate) class: &'a ClassIr,
    /// Controller route prefix.
    pub(crate) prefix: String,
    /// Validated handler methods.
    pub(crate) handlers: Vec<HandlerPlan<'a>>,
}

/// A method or top-level function callable.
#[derive(Clone, Copy)]
pub(crate) enum Callable<'a> {
    /// Controller instance method.
    Method(&'a MethodIr),
    /// Top-level function.
    Function(&'a FunctionIr),
}

/// Validated route and extraction plan for one callable.
pub(crate) struct HandlerPlan<'a> {
    /// Handwritten callable invoked by generated code.
    pub(crate) callable: Callable<'a>,
    /// Uppercase HTTP method.
    pub(crate) method: &'static str,
    /// Route path relative to its group.
    pub(crate) path: String,
    /// Successful response status.
    pub(crate) status: u16,
    /// Parameters in declaration order.
    pub(crate) params: Vec<ParamPlan<'a>>,
}

/// One handler parameter and its runtime extraction strategy.
pub(crate) struct ParamPlan<'a> {
    /// Canonical parameter facts.
    pub(crate) param: &'a MethodParamIr,
    /// Selected runtime extractor.
    pub(crate) extraction: Extraction,
}

/// Runtime extraction strategies implemented by Phase one.
pub(crate) enum Extraction {
    /// Named route-path segment.
    Path(String),
    /// One query value.
    Query(String),
    /// Repeated query values.
    QueryList(String),
    /// Whole query map or decoded model.
    Queries,
    /// One HTTP header.
    Header(String),
    /// Complete header map.
    HeaderMap,
    /// Typed JSON body.
    Body,
}

/// Shared view over class configs and general annotations.
pub(super) enum Args<'a> {
    /// Lowered configuration application.
    Config(&'a ConfigApplicationIr),
    /// General top-level annotation.
    Annotation(&'a AnnotationIr),
}

impl Args<'_> {
    /// Returns the source annotation span.
    pub(super) fn span(&self) -> SpanIr {
        match self {
            Self::Config(value) => value.span,
            Self::Annotation(value) => value.span,
        }
    }

    /// Returns structured positional values.
    pub(super) fn positional(&self) -> &[AnnotationValueIr] {
        match self {
            Self::Config(value) => &value.positional_args,
            Self::Annotation(value) => &value.positional_args,
        }
    }

    /// Returns structured named values.
    pub(super) fn named(&self) -> &std::collections::BTreeMap<String, AnnotationValueIr> {
        match self {
            Self::Config(value) => &value.named_args,
            Self::Annotation(value) => &value.named_args,
        }
    }
}

impl<'a> Callable<'a> {
    /// Returns the callable's Dart name.
    pub(crate) fn name(self) -> &'a str {
        match self {
            Self::Method(value) => &value.name,
            Self::Function(value) => &value.name.source,
        }
    }

    /// Returns parameters in source order.
    pub(super) fn params(self) -> &'a [MethodParamIr] {
        match self {
            Self::Method(value) => &value.params,
            Self::Function(value) => &value.params,
        }
    }

    /// Returns the declared return type.
    pub(crate) fn return_type(self) -> &'a TypeIr {
        match self {
            Self::Method(value) => &value.return_type,
            Self::Function(value) => &value.return_type,
        }
    }

    /// Returns the callable source span.
    pub(super) fn span(self) -> SpanIr {
        match self {
            Self::Method(value) => value.span,
            Self::Function(value) => value.span,
        }
    }
}
