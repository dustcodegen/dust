use std::collections::BTreeSet;

use dust_diagnostics::Diagnostic;
use dust_ir::{ClassIr, EnumIr, MethodParamIr, TypeIr};

use super::{
    BodyFormat, RawBodyFormat,
    body::BodyMedia,
    model::{Args, Extraction, ExtractionWrapper, ParamPlan},
    param::custom::custom,
    types::{
        derives_deserialize, derives_validate, is_map_of, is_peer_info, is_uploaded_file,
        raw_body_format, require_body_type, require_queries_type, require_raw_query_type,
        require_scalar, require_state_type,
    },
    values::{error, optional_string, required_string, validate_no_args, validate_zero_named},
};

/// Custom extractor annotation planning.
mod custom;
#[cfg(test)]
mod local_tests;
#[cfg(test)]
mod phase_three_tests;
/// Query parameter planning.
mod query;
use query::query;
/// Extractor planning tests.
#[cfg(test)]
mod tests;

/// Normalizes one supported handler parameter extractor.
pub(super) fn parse_param<'a>(
    param: &'a MethodParamIr,
    placeholders: &BTreeSet<String>,
    media: BodyMedia,
    classes: &[ClassIr],
    enums: &[EnumIr],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ParamPlan<'a>> {
    let (value_type, wrapper) = parameter_shape(&param.ty, param, diagnostics)?;
    let configs = param
        .configs
        .iter()
        .filter(|value| value.symbol.0.starts_with("dust_server::"))
        .collect::<Vec<_>>();
    if configs.is_empty() {
        return passthrough(param, value_type, wrapper, diagnostics);
    }
    if configs.len() != 1 {
        diagnostics.push(error(
            param.span,
            "handler parameters require exactly one server extractor",
            "add one supported extractor annotation",
        ));
        return None;
    }
    let config = configs[0];
    let symbol = config.symbol.0.as_str();
    let args = Args::Config(config);
    let extraction = match symbol {
        "dust_server::Path" => path(param, value_type, placeholders, &args, diagnostics),
        "dust_server::Query" => query(param, value_type, &args, classes, enums, diagnostics)?,
        "dust_server::Queries" => {
            validate_no_args(&args, diagnostics);
            require_queries_type(value_type, param.span, diagnostics);
            Extraction::Queries
        }
        "dust_server::Header" => {
            let name = required_string(&args, 0, "header name", diagnostics)?;
            validate_zero_named(&args, diagnostics);
            require_scalar(value_type, param.span, diagnostics);
            Extraction::Header(name)
        }
        "dust_server::HeaderMap" => {
            validate_no_args(&args, diagnostics);
            if !is_map_of(value_type, "String", "String") {
                diagnostics.push(error(
                    param.span,
                    "HeaderMap requires Map<String, String>",
                    "change the parameter type to Map<String, String>",
                ));
            }
            Extraction::HeaderMap
        }
        "dust_server::Body" => {
            validate_no_args(&args, diagnostics);
            require_body_type(value_type, param.span, diagnostics);
            match media {
                BodyMedia::Json => Extraction::Body(BodyFormat::Json),
                BodyMedia::Form => Extraction::Body(BodyFormat::Form),
                BodyMedia::Multipart => {
                    diagnostics.push(error(
                        param.span,
                        "Body is not supported with MultiPart",
                        "use one or more Part parameters",
                    ));
                    Extraction::Body(BodyFormat::Json)
                }
            }
        }
        "dust_server::Extract" => custom(&args, classes, diagnostics)?,
        "dust_server::RawQuery" => {
            validate_no_args(&args, diagnostics);
            require_raw_query_type(value_type, param.span, diagnostics);
            Extraction::RawQuery
        }
        "dust_server::Field" => {
            let name = required_string(&args, 0, "form field name", diagnostics)?;
            validate_zero_named(&args, diagnostics);
            require_scalar(value_type, param.span, diagnostics);
            if media != BodyMedia::Form {
                diagnostics.push(error(
                    param.span,
                    "Field requires FormUrlEncoded on its handler",
                    "add @FormUrlEncoded() to the method or function",
                ));
            }
            Extraction::Field(name)
        }
        "dust_server::Part" => {
            let name = required_string(&args, 0, "multipart part name", diagnostics)?;
            validate_zero_named(&args, diagnostics);
            let file = is_uploaded_file(value_type);
            if !file {
                require_scalar(value_type, param.span, diagnostics);
            }
            if media != BodyMedia::Multipart {
                diagnostics.push(error(
                    param.span,
                    "Part requires MultiPart on its handler",
                    "add @MultiPart() to the method or function",
                ));
            }
            Extraction::Part { name, file }
        }
        "dust_server::RawBody" => {
            validate_no_args(&args, diagnostics);
            let format = raw_body_format(value_type).unwrap_or_else(|| {
                diagnostics.push(error(
                    param.span,
                    "RawBody requires Uint8List, String, or Stream<List<int>>",
                    "change the parameter to one of the supported raw body types",
                ));
                RawBodyFormat::Bytes
            });
            if media != BodyMedia::Json {
                diagnostics.push(error(
                    param.span,
                    "RawBody cannot be combined with a body media annotation",
                    "remove FormUrlEncoded or MultiPart",
                ));
            }
            Extraction::RawBody(format)
        }
        "dust_server::State" => {
            validate_no_args(&args, diagnostics);
            require_state_type(value_type, param.span, diagnostics);
            Extraction::State
        }
        "dust_server::Peer" => {
            validate_no_args(&args, diagnostics);
            if !is_peer_info(value_type) {
                diagnostics.push(error(
                    param.span,
                    "Peer requires PeerInfo",
                    "change the parameter type to PeerInfo",
                ));
            }
            Extraction::Peer
        }
        _ => {
            diagnostics.push(error(
                config.span,
                "extractor is not supported by generated server handlers yet",
                "use a supported dust_server extractor annotation",
            ));
            return None;
        }
    };
    let model_extraction = matches!(extraction, Extraction::QueryModel(_));
    let validated = (matches!(extraction, Extraction::Body(_)) || model_extraction)
        && derives_validate(value_type, classes);
    let serde_deserialized = (matches!(extraction, Extraction::Body(BodyFormat::Json))
        || model_extraction)
        && derives_deserialize(value_type, classes);
    Some(ParamPlan {
        param,
        value_type,
        wrapper,
        extraction,
        validated,
        serde_deserialized,
    })
}

/// Accepts the two unannotated request passthrough types.
fn passthrough<'a>(
    param: &'a MethodParamIr,
    value_type: &'a TypeIr,
    wrapper: ExtractionWrapper,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ParamPlan<'a>> {
    let extraction = if wrapper == ExtractionWrapper::Required
        && !value_type.is_nullable()
        && value_type.args().is_empty()
        && value_type.is_named("Request")
    {
        Extraction::Request
    } else if wrapper == ExtractionWrapper::Required
        && !value_type.is_nullable()
        && value_type.args().is_empty()
        && value_type.is_named("RequestParts")
    {
        Extraction::RequestParts
    } else {
        diagnostics.push(error(
            param.span,
            "handler parameters require exactly one server extractor",
            "add one supported extractor annotation; only Request and RequestParts are unannotated",
        ));
        return None;
    };
    Some(ParamPlan {
        param,
        value_type,
        wrapper,
        extraction,
        validated: false,
        serde_deserialized: false,
    })
}

/// Selects required, optional, or fallible extraction from a parameter type.
fn parameter_shape<'a>(
    ty: &'a TypeIr,
    param: &MethodParamIr,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(&'a TypeIr, ExtractionWrapper)> {
    let short = ty.name().and_then(|name| name.rsplit('.').next());
    match short {
        Some("Option") if !ty.is_nullable() && ty.args().len() == 1 => {
            Some((&ty.args()[0], ExtractionWrapper::Optional))
        }
        Some("Result")
            if !ty.is_nullable()
                && ty.args().len() == 2
                && ty.args()[1].name().and_then(|name| name.rsplit('.').next())
                    == Some("Rejection") =>
        {
            Some((&ty.args()[0], ExtractionWrapper::Fallible))
        }
        Some("Option" | "Result") => {
            diagnostics.push(error(
                param.span,
                "invalid optional or fallible extractor parameter",
                "use Option<T> or Result<T, Rejection>",
            ));
            None
        }
        _ => Some((ty, ExtractionWrapper::Required)),
    }
}

/// Normalizes a path parameter and checks its route placeholder.
fn path(
    param: &MethodParamIr,
    value_type: &TypeIr,
    placeholders: &BTreeSet<String>,
    args: &Args<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Extraction {
    validate_zero_named(args, diagnostics);
    let key = optional_string(args, 0, diagnostics).unwrap_or_else(|| param.name.clone());
    if !placeholders.contains(&key) {
        diagnostics.push(error(
            param.span,
            format!("path parameter `{key}` is missing from the route"),
            "add the matching `{key}` placeholder or change @Path",
        ));
    }
    require_scalar(value_type, param.span, diagnostics);
    Extraction::Path(key)
}
