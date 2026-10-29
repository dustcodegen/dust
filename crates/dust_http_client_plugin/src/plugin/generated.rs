use dust_diagnostics::Diagnostic;
use dust_ir::{MethodIr, MethodParamIr};
use dust_plugin_api::GENERATED_HEADER;

use super::{
    emit::{render_shared_helpers, render_standalone_client_class},
    model::{
        ClientSpec, EndpointParam, EndpointSpec, HttpTargetMode, HttpVerb, ParseThreadMode,
        RequestMode,
    },
};
use crate::plugin::{build::classify_return_type, emit::render_isolate_helpers};

/// One complete standalone Dio client library derived from backend routes.
pub struct GeneratedClient {
    /// Public Dart class name.
    pub class_name: String,
    /// Complete Dart import directives required by exposed types.
    pub imports: Vec<String>,
    /// Routes exposed as client methods.
    pub endpoints: Vec<GeneratedEndpoint>,
}

/// One backend route represented as a generated client method.
pub struct GeneratedEndpoint {
    /// Synthetic client method with normalized parameters and `Future` return type.
    pub method: MethodIr,
    /// HTTP method used on the wire.
    pub verb: GeneratedHttpVerb,
    /// Complete route path including group prefixes.
    pub path: String,
    /// Request body encoding.
    pub request_mode: GeneratedRequestMode,
    /// Client-visible parameter bindings by method-parameter index.
    pub params: Vec<GeneratedParam>,
}

/// HTTP methods accepted by standalone generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedHttpVerb {
    /// GET.
    Get,
    /// POST.
    Post,
    /// PUT.
    Put,
    /// PATCH.
    Patch,
    /// DELETE.
    Delete,
    /// HEAD.
    Head,
    /// OPTIONS.
    Options,
}

/// Request encodings accepted by standalone generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GeneratedRequestMode {
    /// JSON or no request body.
    #[default]
    Standard,
    /// URL-encoded form fields.
    FormUrlEncoded,
    /// Multipart form data.
    MultiPart,
}

/// One client-visible wire binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratedParam {
    /// Path replacement.
    Path {
        /// Index in the generated method parameter list.
        index: usize,
        /// Wire placeholder name.
        key: String,
    },
    /// One query parameter.
    Query {
        /// Index in the generated method parameter list.
        index: usize,
        /// Wire query name.
        key: String,
    },
    /// Whole query map.
    Queries {
        /// Index in the generated method parameter list.
        index: usize,
    },
    /// One request header.
    Header {
        /// Index in the generated method parameter list.
        index: usize,
        /// Wire header name.
        key: String,
    },
    /// Whole request-header map.
    HeaderMap {
        /// Index in the generated method parameter list.
        index: usize,
    },
    /// Raw request body.
    Body {
        /// Index in the generated method parameter list.
        index: usize,
    },
    /// URL-encoded field.
    Field {
        /// Index in the generated method parameter list.
        index: usize,
        /// Wire field name.
        key: String,
    },
    /// Multipart field.
    Part {
        /// Index in the generated method parameter list.
        index: usize,
        /// Wire part name.
        key: String,
    },
}

/// Renders one standalone, analyzer-ready Dart client library.
pub fn generate_client_library(client: &GeneratedClient) -> Result<String, Diagnostic> {
    let endpoints = client
        .endpoints
        .iter()
        .map(endpoint_spec)
        .collect::<Result<Vec<_>, _>>()?;
    let spec = ClientSpec {
        class_name: &client.class_name,
        base_url: None,
        target: HttpTargetMode::Dart,
        generate_test: false,
        endpoints,
    };
    let mut blocks = vec![
        GENERATED_HEADER.to_owned(),
        "import 'package:dio/dio.dart';".to_owned(),
    ];
    blocks.extend(client.imports.iter().cloned());
    blocks.push(render_standalone_client_class(&spec));
    blocks.extend(render_isolate_helpers(&spec));
    blocks.extend(render_shared_helpers());
    Ok(format!(
        "{}\n",
        blocks
            .iter()
            .map(|block| block.trim_end())
            .collect::<Vec<_>>()
            .join("\n\n")
    ))
}

/// Converts one public standalone endpoint into the shared renderer model.
fn endpoint_spec(endpoint: &GeneratedEndpoint) -> Result<EndpointSpec<'_>, Diagnostic> {
    let return_spec = classify_return_type(&endpoint.method.return_type).ok_or_else(|| {
        Diagnostic::error(format!(
            "generated client method `{}` has an unsupported return type",
            endpoint.method.name
        ))
    })?;
    let params = endpoint
        .params
        .iter()
        .map(|binding| bind_param(&endpoint.method.params, binding))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(EndpointSpec {
        method: &endpoint.method,
        verb: endpoint.verb.into(),
        path: endpoint.path.clone(),
        parse_thread: ParseThreadMode::Main,
        headers: Vec::new(),
        request_mode: endpoint.request_mode.into(),
        params,
        return_spec,
    })
}

/// Resolves a public binding index to the method parameter used by the emitter.
fn bind_param<'a>(
    params: &'a [MethodParamIr],
    binding: &GeneratedParam,
) -> Result<EndpointParam<'a>, Diagnostic> {
    let index = match binding {
        GeneratedParam::Path { index, .. }
        | GeneratedParam::Query { index, .. }
        | GeneratedParam::Queries { index }
        | GeneratedParam::Header { index, .. }
        | GeneratedParam::HeaderMap { index }
        | GeneratedParam::Body { index }
        | GeneratedParam::Field { index, .. }
        | GeneratedParam::Part { index, .. } => *index,
    };
    let param = params.get(index).ok_or_else(|| {
        Diagnostic::error(format!(
            "generated client parameter index {index} is out of bounds"
        ))
    })?;
    Ok(match binding {
        GeneratedParam::Path { key, .. } => EndpointParam::Path {
            param,
            key: key.clone(),
        },
        GeneratedParam::Query { key, .. } => EndpointParam::Query {
            param,
            key: key.clone(),
        },
        GeneratedParam::Queries { .. } => EndpointParam::Queries { param },
        GeneratedParam::Header { key, .. } => EndpointParam::Header {
            param,
            key: key.clone(),
        },
        GeneratedParam::HeaderMap { .. } => EndpointParam::HeaderMap { param },
        GeneratedParam::Body { .. } => EndpointParam::Body { param },
        GeneratedParam::Field { key, .. } => EndpointParam::Field {
            param,
            key: key.clone(),
        },
        GeneratedParam::Part { key, .. } => EndpointParam::Part {
            param,
            key: key.clone(),
        },
    })
}

impl From<GeneratedHttpVerb> for HttpVerb {
    fn from(value: GeneratedHttpVerb) -> Self {
        match value {
            GeneratedHttpVerb::Get => Self::Get,
            GeneratedHttpVerb::Post => Self::Post,
            GeneratedHttpVerb::Put => Self::Put,
            GeneratedHttpVerb::Patch => Self::Patch,
            GeneratedHttpVerb::Delete => Self::Delete,
            GeneratedHttpVerb::Head => Self::Head,
            GeneratedHttpVerb::Options => Self::Options,
        }
    }
}

impl From<GeneratedRequestMode> for RequestMode {
    fn from(value: GeneratedRequestMode) -> Self {
        match value {
            GeneratedRequestMode::Standard => Self::Standard,
            GeneratedRequestMode::FormUrlEncoded => Self::FormUrlEncoded,
            GeneratedRequestMode::MultiPart => Self::MultiPart,
        }
    }
}
