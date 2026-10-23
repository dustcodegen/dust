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
    /// Middleware applied to the top-level route group.
    pub(crate) function_middleware: Vec<AnnotationValueIr>,
}

/// Shared controller or top-level route-group settings.
#[derive(Default)]
pub(super) struct GroupPlan {
    /// Relative group prefix.
    pub(super) prefix: String,
    /// Optional generated top-level router name.
    pub(super) configured_name: Option<String>,
    /// Middleware applied to the whole group.
    pub(super) middleware: Vec<AnnotationValueIr>,
    /// Tags inherited by every route in the group.
    pub(super) tags: Vec<String>,
}

/// One controller and its generated handler plans.
pub(crate) struct ControllerPlan<'a> {
    /// Source controller class.
    pub(crate) class: &'a ClassIr,
    /// Controller route prefix.
    pub(crate) prefix: String,
    /// Validated handler methods.
    pub(crate) handlers: Vec<HandlerPlan<'a>>,
    /// Middleware applied to every route in the controller.
    pub(crate) middleware: Vec<AnnotationValueIr>,
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
    /// Middleware applied only to this route.
    pub(crate) middleware: Vec<AnnotationValueIr>,
    /// Format-neutral route documentation facts.
    pub(crate) metadata: RouteMetadataPlan,
}

/// Documentation facts emitted with one route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteMetadataPlan {
    /// Stable operation identifier.
    pub(crate) operation_id: String,
    /// Optional one-line summary.
    pub(crate) summary: Option<String>,
    /// Optional longer description.
    pub(crate) description: Option<String>,
    /// Group tags followed by route-local tags.
    pub(crate) tags: Vec<String>,
    /// Whether the route is deprecated.
    pub(crate) deprecated: bool,
    /// Whether documentation tools should hide the route.
    pub(crate) hidden: bool,
}

/// One handler parameter and its runtime extraction strategy.
pub(crate) struct ParamPlan<'a> {
    /// Canonical parameter facts.
    pub(crate) param: &'a MethodParamIr,
    /// Value type produced before an optional/fallible wrapper is applied.
    pub(crate) value_type: &'a TypeIr,
    /// Failure behavior selected by the parameter type.
    pub(crate) wrapper: ExtractionWrapper,
    /// Selected runtime extractor.
    pub(crate) extraction: Extraction,
    /// Whether the decoded body must run its generated `Validate()` contract.
    pub(crate) validated: bool,
    /// Whether the model decoder comes from Dust `Deserialize()` generation.
    pub(crate) serde_deserialized: bool,
}

/// How an extractor failure reaches the handwritten handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtractionWrapper {
    /// Stop the generated handler on the first rejection.
    Required,
    /// Collapse client rejections into `None`.
    Optional,
    /// Pass the extractor result to the handwritten handler.
    Fallible,
}

/// Runtime extraction strategies implemented through Phase three.
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
    /// Typed JSON or form body.
    Body(BodyFormat),
    /// User extractor type named by `@Extract(Type)`.
    Custom(String),
    /// Raw, undecoded query string.
    RawQuery,
    /// One field from a shared form body.
    Field(String),
    /// One value from a shared multipart body.
    Part {
        /// Multipart field name.
        name: String,
        /// Whether the target expects an uploaded file rather than text.
        file: bool,
    },
    /// Raw request body selected by the parameter type.
    RawBody(RawBodyFormat),
    /// Application state keyed by the parameter type.
    State,
    /// Connection peer information.
    Peer,
    /// The shelf request unchanged.
    Request,
    /// Immutable request-head facts.
    RequestParts,
}

/// Typed body decoder selected by handler media annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BodyFormat {
    /// JSON object decoding.
    Json,
    /// URL-encoded form decoding.
    Form,
}

/// Raw body representation selected by the parameter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawBodyFormat {
    /// `Uint8List` bytes.
    Bytes,
    /// UTF-8 `String`.
    Text,
    /// Unread `Stream<List<int>>`.
    Stream,
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
