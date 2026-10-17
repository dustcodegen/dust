use dust_ir::{FieldIr, TypeIr};

use crate::{enum_codecs::EnumCodecs, names::JsonNames};

/// Custom codec conversion expressions.
mod codec;
/// Structural JSON decoding expressions.
mod decode;
/// Structural JSON encoding expressions.
mod encode;

use codec::{decode_with_codec, encode_with_codec};
use decode::decode_expr;
use encode::encode_expr;

/// Renders a JSON encode expression for a class field.
pub(crate) fn encode_field_expr(
    expr: &str,
    field: &FieldIr,
    serializable_classes: &[&str],
    serializable_enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> String {
    match field
        .serde
        .as_ref()
        .and_then(|serde| serde.codec_source.as_deref())
    {
        Some(codec) => encode_with_codec(expr, &field.ty, codec, names),
        None => encode_expr(
            expr,
            &field.ty,
            serializable_classes,
            serializable_enums,
            names,
        ),
    }
}

/// Renders a JSON decode expression for a class field.
pub(crate) fn decode_field_expr(
    raw: &str,
    key: &str,
    field: &FieldIr,
    deserializable_classes: &[&str],
    deserializable_enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> String {
    match field
        .serde
        .as_ref()
        .and_then(|serde| serde.codec_source.as_deref())
    {
        Some(codec) => decode_with_codec(raw, key, &field.ty, codec, names),
        None => decode_expr(
            raw,
            key,
            &field.ty,
            deserializable_classes,
            deserializable_enums,
            names,
        ),
    }
}

/// Indents a multiline expression by a fixed number of spaces.
fn indent_expr(expr: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    expr.lines()
        .map(|line| format!("{pad}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
