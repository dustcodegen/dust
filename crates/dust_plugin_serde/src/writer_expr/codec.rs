use super::*;
use crate::writer_type::access_receiver;
use dust_dart_emit::OBJECT_NULLABLE_TYPES;

/// Renders JSON encoding through a user-provided codec.
pub(super) fn encode_with_codec(expr: &str, ty: &TypeIr, codec: &str, names: &JsonNames) -> String {
    let codec = access_receiver(codec);
    let value_ty = OBJECT_NULLABLE_TYPES.render_non_nullable(ty);
    let helper = names.reference("JsonHelper");
    let object = names.reference("Object");
    if ty.is_nullable() {
        let encoded = format!("{helper}.encodeWithCodec<{value_ty}, {object}?>({codec}, {expr}!)");
        return format!("{expr} == null\n    ? null\n    : {encoded}");
    }
    format!("{helper}.encodeWithCodec<{value_ty}, {object}?>({codec}, {expr})")
}

/// Renders JSON decoding through a user-provided codec.
pub(super) fn decode_with_codec(
    raw: &str,
    key: &str,
    ty: &TypeIr,
    codec: &str,
    names: &JsonNames,
) -> String {
    let codec = access_receiver(codec);
    let value_ty = OBJECT_NULLABLE_TYPES.render_non_nullable(ty);
    let helper = names.reference("JsonHelper");
    let object = names.reference("Object");
    let decoded = format!("{helper}.decodeWithCodec<{value_ty}, {object}?>({codec}, {raw}, {key})");
    if ty.is_nullable() {
        format!("{raw} == null\n    ? null\n    : {decoded}")
    } else {
        decoded
    }
}
