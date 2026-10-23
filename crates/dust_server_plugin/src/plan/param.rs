use std::collections::BTreeSet;

use dust_diagnostics::Diagnostic;
use dust_ir::MethodParamIr;

use super::{
    model::{Args, Extraction, ParamPlan},
    types::{is_map_of, require_body_type, require_queries_type, require_scalar},
    values::{error, optional_string, required_string, validate_no_args, validate_zero_named},
};

/// Extractor planning tests.
#[cfg(test)]
mod tests;

/// Normalizes one built-in Phase-one parameter extractor.
pub(super) fn parse_param<'a>(
    param: &'a MethodParamIr,
    placeholders: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ParamPlan<'a>> {
    let configs = param
        .configs
        .iter()
        .filter(|value| value.symbol.0.starts_with("dust_server::"))
        .collect::<Vec<_>>();
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
        "dust_server::Path" => path(param, placeholders, &args, diagnostics),
        "dust_server::Query" => query(param, &args, diagnostics)?,
        "dust_server::Queries" => {
            validate_no_args(&args, diagnostics);
            require_queries_type(&param.ty, param.span, diagnostics);
            Extraction::Queries
        }
        "dust_server::Header" => {
            let name = required_string(&args, 0, "header name", diagnostics)?;
            validate_zero_named(&args, diagnostics);
            require_scalar(&param.ty, param.span, diagnostics);
            Extraction::Header(name)
        }
        "dust_server::HeaderMap" => {
            validate_no_args(&args, diagnostics);
            if !is_map_of(&param.ty, "String", "String") {
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
            require_body_type(&param.ty, param.span, diagnostics);
            Extraction::Body
        }
        _ => {
            diagnostics.push(error(
                config.span,
                "extractor is not supported in server Phase 1",
                "use Path, Query, Queries, Header, HeaderMap, or Body",
            ));
            return None;
        }
    };
    Some(ParamPlan { param, extraction })
}

/// Normalizes a path parameter and checks its route placeholder.
fn path(
    param: &MethodParamIr,
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
    require_scalar(&param.ty, param.span, diagnostics);
    Extraction::Path(key)
}

/// Selects scalar or repeated-query extraction from the declared type.
fn query(
    param: &MethodParamIr,
    args: &Args<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Extraction> {
    let key = required_string(args, 0, "query name", diagnostics)?;
    validate_zero_named(args, diagnostics);
    if param.ty.is_named("List") && param.ty.args().len() == 1 {
        require_scalar(&param.ty.args()[0], param.span, diagnostics);
        Some(Extraction::QueryList(key))
    } else {
        require_scalar(&param.ty, param.span, diagnostics);
        Some(Extraction::Query(key))
    }
}
