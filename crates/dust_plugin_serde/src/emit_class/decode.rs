//! Emitting the decode side of a generated codec, field by field.

use super::*;
use crate::enum_codecs::EnumCodecs;

/// Emits the expression that decodes one class field.
pub(super) fn emit_field_decode(
    class: &ClassIr,
    field: &dust_ir::FieldIr,
    deserializable_classes: &[&str],
    deserializable_enums: EnumCodecs<'_>,
    lines: &mut Vec<String>,
    names: &JsonNames,
) -> String {
    let serde = field.serde.as_ref();
    if serde.is_some_and(|serde| serde.skip_deserializing) {
        return serde
            .and_then(|serde| serde.default_value_source.clone())
            .unwrap_or_else(|| "null".to_owned());
    }

    let primary_key = json_key(class, &field.name, serde);
    let aliases = serde.map(|serde| serde.aliases.as_slice()).unwrap_or(&[]);
    let json = names.local("json");
    let has_expr = build_has_expr(&primary_key, aliases, json);
    let decoded = if aliases.is_empty() {
        decode_field_expr(
            &format!("{json}['{primary_key}']"),
            &format!("'{primary_key}'"),
            field,
            deserializable_classes,
            deserializable_enums,
            names,
        )
    } else {
        emit_alias_decode(
            field,
            &primary_key,
            aliases,
            deserializable_classes,
            deserializable_enums,
            lines,
            names,
        )
    };

    if !class.type_parameters.is_empty()
        && field.ty.is_builtin(dust_ir::BuiltinType::Bool)
        && !field.ty.is_nullable()
        && let Some(dust_ir::AnnotationValueIr::Bool(default)) =
            serde.and_then(|s| s.default_value.as_ref())
    {
        return if *default {
            let missing = if aliases.is_empty() {
                format!("!{has_expr}")
            } else {
                format!("!({has_expr})")
            };
            format!("{missing} || {decoded}")
        } else {
            let has_expr = if aliases.is_empty() {
                has_expr
            } else {
                format!("({has_expr})")
            };
            format!("{has_expr} && {decoded}")
        };
    }

    serde
        .and_then(|serde| serde.default_value_source.as_deref())
        .map_or(decoded.clone(), |default_value| {
            let inline = format!("{has_expr} ? {decoded} : {default_value}");
            if inline.len() <= 80 {
                inline
            } else {
                format!("{has_expr}\n    ? {decoded}\n    : {default_value}")
            }
        })
}

/// Builds the expression that checks primary and alias keys.
pub(super) fn build_has_expr(primary_key: &str, aliases: &[String], json: &str) -> String {
    if aliases.is_empty() {
        format!("{json}.containsKey('{primary_key}')")
    } else {
        std::iter::once(format!("{json}.containsKey('{primary_key}')"))
            .chain(
                aliases
                    .iter()
                    .map(|alias| format!("{json}.containsKey('{alias}')")),
            )
            .collect::<Vec<_>>()
            .join(" || ")
    }
}

/// Emits alias-aware decode setup and returns the final decode expression.
pub(super) fn emit_alias_decode(
    field: &dust_ir::FieldIr,
    primary_key: &str,
    aliases: &[String],
    deserializable_classes: &[&str],
    deserializable_enums: EnumCodecs<'_>,
    lines: &mut Vec<String>,
    names: &JsonNames,
) -> String {
    let raw_name = names
        .local(&format!("raw{}", AsPascalCase(&field.name)))
        .to_owned();
    let raw_key_name = names
        .local(&format!("raw{}Key", AsPascalCase(&field.name)))
        .to_owned();
    let json = names.local("json");
    let object = names.reference("Object");
    let decoded = decode_field_expr(
        &raw_name,
        &raw_key_name,
        field,
        deserializable_classes,
        deserializable_enums,
        names,
    );
    let uses_raw_key = decoded.contains(&raw_key_name);

    if uses_raw_key {
        lines.push(format!("  var {raw_key_name} = '{primary_key}';"));
    }
    lines.push(format!("  {object}? {raw_name};"));
    lines.push(format!("  if ({json}.containsKey('{primary_key}')) {{"));
    lines.push(format!("    {raw_name} = {json}['{primary_key}'];"));
    for alias in aliases {
        lines.push(format!("  }} else if ({json}.containsKey('{alias}')) {{"));
        if uses_raw_key {
            lines.push(format!("    {raw_key_name} = '{alias}';"));
        }
        lines.push(format!("    {raw_name} = {json}['{alias}'];"));
    }
    lines.push("  }".to_owned());

    decoded
}

/// Appends a constructor return statement, preserving multiline formatting.
pub(super) fn append_constructor_return(lines: &mut Vec<String>, call: &str) {
    lines.push(String::new());
    lines.push(format!(
        "  return {};",
        call.lines().next().unwrap_or_default()
    ));
    if call.lines().count() <= 1 {
        return;
    }

    lines.pop();
    let mut call_lines = call.lines();
    if let Some(first) = call_lines.next() {
        lines.push(format!("  return {first}"));
    }
    for line in call_lines {
        if line == ")" {
            lines.push(format!("  {line};"));
        } else {
            lines.push(format!("  {line}"));
        }
    }
}
