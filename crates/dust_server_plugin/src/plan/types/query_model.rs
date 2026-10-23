use dust_diagnostics::Diagnostic;
use dust_ir::{ClassIr, EnumIr, SpanIr, TypeIr, serde_field_name};

use crate::plan::{
    model::{QueryConversionKind, QueryConversionPlan, QueryModelPlan},
    values::error,
};

use super::derives_deserialize;

#[cfg(test)]
mod tests;

/// Validates and plans an unnamed typed query model.
pub(in crate::plan) fn query_model_plan(
    ty: &TypeIr,
    classes: &[ClassIr],
    enums: &[EnumIr],
    span: SpanIr,
    diagnostics: &mut Vec<Diagnostic>,
) -> QueryModelPlan {
    let Some(name) = ty
        .name()
        .filter(|name| !name.contains('.') && !ty.is_nullable() && ty.args().is_empty())
    else {
        diagnostics.push(error(
            span,
            "Query without a name requires a non-nullable same-library model",
            "use @Query('name') for a scalar or @Query() on a local Deserialize model",
        ));
        return empty_plan();
    };
    let Some(class) = classes.iter().find(|class| class.name == name) else {
        diagnostics.push(error(
            span,
            "Query without a name requires a same-library model",
            "use @Query('name') for a scalar or move the query model into this library",
        ));
        return empty_plan();
    };
    if !class.type_parameters.is_empty() {
        diagnostics.push(error(
            class.span,
            "generic query models are not supported",
            "use a concrete non-generic query model",
        ));
    }
    if !derives_deserialize(ty, classes) {
        diagnostics.push(error(
            class.span,
            "Query model must derive Deserialize()",
            "add Deserialize() to the model's @Derive list",
        ));
    }

    let mut conversions = Vec::new();
    for field in &class.fields {
        let serde = field.serde.as_ref();
        if serde.is_some_and(|value| value.skip_deserializing) {
            continue;
        }
        if serde.is_some_and(|value| value.codec_source.is_some()) {
            diagnostics.push(error(
                field.span,
                "query model fields cannot use a custom SerDe codec",
                "use a flat supported scalar field without SerDe(using: ...)",
            ));
            continue;
        }
        let Some(kind) = query_conversion(&field.ty, enums, field.span, diagnostics).flatten()
        else {
            continue;
        };
        let primary = serde_field_name(&field.name, serde, class.serde.as_ref());
        conversions.push(QueryConversionPlan {
            key: primary,
            kind: kind.clone(),
        });
        for alias in serde.into_iter().flat_map(|value| &value.aliases) {
            conversions.push(QueryConversionPlan {
                key: alias.clone(),
                kind: kind.clone(),
            });
        }
    }
    QueryModelPlan { conversions }
}

/// Returns a placeholder plan after a model-level diagnostic.
fn empty_plan() -> QueryModelPlan {
    QueryModelPlan {
        conversions: Vec::new(),
    }
}

/// Returns the conversion needed before JSON-shaped SerDe decoding.
fn query_conversion(
    ty: &TypeIr,
    enums: &[EnumIr],
    span: SpanIr,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Option<QueryConversionKind>> {
    if ty.is_named("List") && ty.args().len() == 1 {
        let item = &ty.args()[0];
        if item.is_nullable() {
            diagnostics.push(error(
                span,
                "query model lists cannot contain nullable items",
                "use List<T> or make the whole list nullable",
            ));
            return None;
        }
        return query_scalar(item, enums, span, diagnostics).map(|kind| {
            Some(match kind {
                Some(QueryConversionKind::Scalar(ty)) => QueryConversionKind::ScalarList(ty),
                _ => QueryConversionKind::StringList,
            })
        });
    }
    query_scalar(ty, enums, span, diagnostics)
}

/// Validates one flat query scalar and returns its required conversion.
fn query_scalar(
    ty: &TypeIr,
    enums: &[EnumIr],
    span: SpanIr,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Option<QueryConversionKind>> {
    match ty.name() {
        Some("int" | "double" | "num" | "bool") => {
            return Some(Some(QueryConversionKind::Scalar(ty.clone())));
        }
        Some("String" | "BigInt" | "DateTime" | "Uri") => return Some(None),
        Some(name) if !name.contains('.') && ty.args().is_empty() => {
            if let Some(value) = enums.iter().find(|value| value.name == name) {
                if value
                    .traits
                    .iter()
                    .any(|trait_| trait_.symbol.0 == "dust_dart::Deserialize")
                {
                    return Some(None);
                }
                diagnostics.push(error(
                    value.span,
                    "query model enum fields must derive Deserialize()",
                    "add Deserialize() to the enum's @Derive list",
                ));
                return None;
            }
        }
        _ => {}
    }
    diagnostics.push(error(
        span,
        "query model fields must be flat scalar, enum, or List values",
        "use String, int, double, num, bool, BigInt, DateTime, Uri, an enum, or List<T>",
    ));
    None
}
