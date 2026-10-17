use dust_dart_emit::{DART_BIG_INT, DART_DATE_TIME, DART_LIST, DART_MAP, DART_SET, DART_URI};
use dust_ir::TypeIr;

use super::*;
use crate::writer_type::{non_null_encode_expr, non_nullable};

/// Renders a JSON encode expression for a type.
pub(super) fn encode_expr(
    expr: &str,
    ty: &TypeIr,
    classes: &[&str],
    enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> String {
    if matches!(ty, TypeIr::Builtin { nullable: true, .. } | TypeIr::Dynamic) {
        return expr.to_owned();
    }
    if ty.is_nullable() {
        let encoded = encode_non_nullable_expr(
            &non_null_encode_expr(expr),
            &non_nullable(ty),
            classes,
            enums,
            names,
        );
        return format!("{expr} == null\n    ? null\n    : {encoded}");
    }
    encode_non_nullable_expr(expr, ty, classes, enums, names)
}

/// Renders a JSON encode expression for a non-nullable value.
fn encode_non_nullable_expr(
    expr: &str,
    ty: &TypeIr,
    classes: &[&str],
    enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> String {
    match ty {
        TypeIr::Builtin { .. }
        | TypeIr::Dynamic
        | TypeIr::Unknown
        | TypeIr::Function { .. }
        | TypeIr::Record { .. } => expr.to_owned(),
        TypeIr::Named { name, .. } if name.as_ref() == DART_DATE_TIME => {
            format!("{expr}.toIso8601String()")
        }
        TypeIr::Named { name, .. }
            if name.as_ref() == DART_URI || name.as_ref() == DART_BIG_INT =>
        {
            format!("{expr}.toString()")
        }
        TypeIr::Named { name, args, .. }
            if name.as_ref() == DART_LIST || name.as_ref() == DART_SET =>
        {
            let item = names.local("item");
            format!(
                "{expr}\n    .map(({item}) => {})\n    .toList()",
                encode_expr(item, &args[0], classes, enums, names)
            )
        }
        TypeIr::Named { name, args, .. } if name.as_ref() == DART_MAP => {
            let value = names.local("value");
            let key = names.local("key");
            let entry = names.reference("MapEntry");
            let value_expr = encode_expr(value, &args[1], classes, enums, names);
            if value_expr.contains('\n') {
                format!(
                    "{expr}\n    .map(\n      ({key}, {value}) => {entry}(\n        {key},\n{},\n      ),\n    )",
                    indent_expr(&value_expr, 8)
                )
            } else {
                format!("{expr}\n    .map(({key}, {value}) => {entry}({key}, {value_expr}))")
            }
        }
        TypeIr::Named { name, .. } => {
            if classes.contains(&name.as_ref()) {
                format!("_${name}Serialize({expr})")
            } else if let Some(encoded) = enums.encode(name, expr) {
                encoded
            } else {
                format!("{expr}.toJson()")
            }
        }
    }
}
