use dust_dart_emit::{DART_BIG_INT, DART_DATE_TIME, DART_LIST, DART_MAP, DART_SET, DART_URI};
use dust_ir::{BuiltinType, TypeIr};

use super::*;
use crate::writer_type::non_nullable;

/// Renders a JSON decode expression for a type.
pub(super) fn decode_expr(
    raw: &str,
    key: &str,
    ty: &TypeIr,
    classes: &[&str],
    enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> String {
    if ty.is_nullable() {
        let decoded = decode_non_nullable_expr(raw, key, &non_nullable(ty), classes, enums, names);
        return format!("{raw} == null\n    ? null\n    : {decoded}");
    }
    decode_non_nullable_expr(raw, key, ty, classes, enums, names)
}

/// Renders a JSON decode expression for a non-nullable value.
fn decode_non_nullable_expr(
    raw: &str,
    key: &str,
    ty: &TypeIr,
    classes: &[&str],
    enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> String {
    let helper = names.reference("JsonHelper");
    match ty {
        TypeIr::Builtin { kind, .. } => {
            let kind_name = if *kind == BuiltinType::Double {
                "num"
            } else {
                kind.as_str()
            };
            let cast = names.reference(kind_name);
            let base = format!("{helper}.as<{cast}>({raw}, {key}, '{kind_name}')");
            if *kind == BuiltinType::Double {
                format!("{base}.toDouble()")
            } else {
                base
            }
        }
        TypeIr::Dynamic | TypeIr::Unknown | TypeIr::Function { .. } | TypeIr::Record { .. } => {
            raw.to_owned()
        }
        TypeIr::Named { name, .. } if name.as_ref() == DART_DATE_TIME => {
            format!("{helper}.asDateTime({raw}, {key})")
        }
        TypeIr::Named { name, .. } if name.as_ref() == DART_URI => {
            format!("{helper}.asUri({raw}, {key})")
        }
        TypeIr::Named { name, .. } if name.as_ref() == DART_BIG_INT => {
            format!("{helper}.asBigInt({raw}, {key})")
        }
        TypeIr::Named { name, args, .. }
            if name.as_ref() == DART_LIST || name.as_ref() == DART_SET =>
        {
            let item = names.local("item");
            let item_key = names.local("itemKey");
            let expr = decode_expr(item, item_key, &args[0], classes, enums, names);
            let method = if name.as_ref() == DART_LIST {
                "decodeList"
            } else {
                "decodeSet"
            };
            decode_collection_expr(helper, method, raw, key, item, item_key, &expr)
        }
        TypeIr::Named { name, args, .. } if name.as_ref() == DART_MAP => {
            let value = names.local("value");
            let value_key = names.local("valueKey");
            let expr = decode_expr(value, value_key, &args[1], classes, enums, names);
            decode_collection_expr(helper, "decodeMap", raw, key, value, value_key, &expr)
        }
        TypeIr::Named { name, .. } => {
            if classes.contains(&name.as_ref()) {
                format!("_${name}Deserialize({helper}.asMap({raw}, {key}))")
            } else if let Some(decoded) = enums.decode(name, raw, key) {
                decoded
            } else {
                format!("{name}.fromJson({helper}.asMap({raw}, {key}))")
            }
        }
    }
}

/// Renders a path-aware collection decode expression.
fn decode_collection_expr(
    helper: &str,
    method: &str,
    raw: &str,
    key: &str,
    value: &str,
    value_key: &str,
    expr: &str,
) -> String {
    if expr.contains('\n') {
        format!(
            "{helper}.{method}({raw}, {key},\n    ({value}, {value_key}) =>\n{})",
            indent_expr(expr, 8)
        )
    } else {
        format!("{helper}.{method}({raw}, {key},\n    ({value}, {value_key}) => {expr})")
    }
}
