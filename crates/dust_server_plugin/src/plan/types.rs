use dust_diagnostics::Diagnostic;
use dust_ir::{BuiltinType, SpanIr, TypeIr};

use super::values::error;

#[cfg(test)]
/// Focused extractor and response type tests.
mod tests;

/// Validates a text-coercible extractor value type.
pub(super) fn require_scalar(ty: &TypeIr, span: SpanIr, diagnostics: &mut Vec<Diagnostic>) {
    if !matches!(
        ty.name(),
        Some("String" | "int" | "double" | "num" | "bool" | "BigInt" | "DateTime" | "Uri")
    ) {
        diagnostics.push(error(
            span,
            "extractor type cannot be coerced from text",
            "use String, int, double, num, bool, BigInt, DateTime, or Uri",
        ));
    }
}

/// Validates the whole-query decoder target.
pub(super) fn require_queries_type(ty: &TypeIr, span: SpanIr, diagnostics: &mut Vec<Diagnostic>) {
    let model = matches!(ty, TypeIr::Named { args, nullable: false, .. } if args.is_empty());
    if !is_map_of(ty, "String", "String") && !model {
        diagnostics.push(error(
            span,
            "Queries requires Map<String, String> or a non-nullable model",
            "use a string map or a model exposing static deserialize",
        ));
    }
}

/// Validates the JSON-body decoder target.
pub(super) fn require_body_type(ty: &TypeIr, span: SpanIr, diagnostics: &mut Vec<Diagnostic>) {
    let model = matches!(ty, TypeIr::Named { args, nullable: false, .. } if args.is_empty());
    if !is_json_map(ty) && !model {
        diagnostics.push(error(
            span,
            "Body requires a JSON map or a non-nullable model",
            "use Map<String, Object?> or a model exposing static deserialize",
        ));
    }
}

/// Returns whether a type is the exact non-nullable map shape.
pub(super) fn is_map_of(ty: &TypeIr, key: &str, value: &str) -> bool {
    !ty.is_nullable()
        && ty.is_named("Map")
        && ty.args().len() == 2
        && ty.args()[0].is_named(key)
        && ty.args()[1].is_named(value)
}

/// Returns whether a type can receive a decoded JSON object unchanged.
fn is_json_map(ty: &TypeIr) -> bool {
    !ty.is_nullable()
        && ty.is_named("Map")
        && ty.args().len() == 2
        && ty.args()[0].is_named("String")
        && matches!(
            ty.args()[1],
            TypeIr::Dynamic
                | TypeIr::Builtin {
                    kind: BuiltinType::Object,
                    nullable: true
                }
        )
}

/// Rejects response shapes intentionally deferred beyond Phase one.
pub(super) fn unsupported_return(ty: &TypeIr) -> bool {
    let inner = if ty.is_named("Future") && ty.args().len() == 1 {
        &ty.args()[0]
    } else {
        ty
    };
    matches!(
        inner.name(),
        Some("Result" | "Option" | "String" | "Uint8List" | "Stream")
    )
}
