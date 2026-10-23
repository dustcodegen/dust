use dust_diagnostics::Diagnostic;
use dust_ir::{BuiltinType, ClassIr, SpanIr, TypeIr};

use super::{model::RawBodyFormat, values::error};

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

/// Validates the raw query's exact nullable string contract.
pub(super) fn require_raw_query_type(ty: &TypeIr, span: SpanIr, diagnostics: &mut Vec<Diagnostic>) {
    if !ty.is_named("String") || !ty.is_nullable() {
        diagnostics.push(error(
            span,
            "RawQuery requires String?",
            "change the parameter type to String?",
        ));
    }
}

/// Selects the raw-body extractor from the parameter type.
pub(super) fn raw_body_format(ty: &TypeIr) -> Option<RawBodyFormat> {
    if !ty.is_nullable() && ty.is_named("Uint8List") && ty.args().is_empty() {
        return Some(RawBodyFormat::Bytes);
    }
    if !ty.is_nullable() && ty.is_named("String") && ty.args().is_empty() {
        return Some(RawBodyFormat::Text);
    }
    if is_byte_stream(ty) {
        return Some(RawBodyFormat::Stream);
    }
    None
}

/// Validates a state value accepted by `StateExtractable<T extends Object>`.
pub(super) fn require_state_type(ty: &TypeIr, span: SpanIr, diagnostics: &mut Vec<Diagnostic>) {
    if ty.is_nullable() || matches!(ty, TypeIr::Dynamic) || ty.is_named("void") {
        diagnostics.push(error(
            span,
            "State requires a non-nullable object type",
            "use the concrete type attached with Router.withState",
        ));
    }
}

/// Returns whether the peer parameter has the runtime's exact type.
pub(super) fn is_peer_info(ty: &TypeIr) -> bool {
    !ty.is_nullable() && ty.is_named("PeerInfo") && ty.args().is_empty()
}

/// Returns whether a multipart part requests the buffered file object.
pub(super) fn is_uploaded_file(ty: &TypeIr) -> bool {
    !ty.is_nullable() && ty.is_named("UploadedFile") && ty.args().is_empty()
}

/// Returns whether a same-library model derives `Validate()`.
pub(super) fn derives_validate(ty: &TypeIr, classes: &[ClassIr]) -> bool {
    let Some(name) = ty.name().filter(|name| !name.contains('.')) else {
        return false;
    };
    classes
        .iter()
        .find(|class| class.name == name)
        .is_some_and(|class| {
            class
                .traits
                .iter()
                .any(|application| application.symbol.0 == "dust_dart::Validate")
        })
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

/// Rejects malformed response wrappers while allowing the Phase three surface.
pub(super) fn unsupported_return(ty: &TypeIr) -> bool {
    let inner = if ty.is_named("Future") {
        if ty.args().len() != 1 {
            return true;
        }
        &ty.args()[0]
    } else {
        ty
    };
    match inner.name().and_then(|name| name.rsplit('.').next()) {
        Some("Result") => inner.args().len() != 2,
        Some("Option") => inner.args().len() != 1,
        Some("Stream") => !is_byte_stream(inner),
        _ => false,
    }
}

/// Returns whether a type is the supported unbuffered byte stream.
fn is_byte_stream(ty: &TypeIr) -> bool {
    !ty.is_nullable()
        && ty.is_named("Stream")
        && ty.args().len() == 1
        && ty.args()[0].is_named("List")
        && ty.args()[0].args().len() == 1
        && ty.args()[0].args()[0].is_named("int")
}
