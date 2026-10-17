use dust_dart_emit::{
    render_class_type, render_template, render_type_arguments, render_type_parameters,
};
use dust_ir::ClassIr;
use heck::AsPascalCase;
use serde::Serialize;

use crate::enum_codecs::EnumCodecs;
use crate::names::JsonNames;
use crate::{
    emit_support::format_prefixed_expr,
    writer::{
        all_allowed_keys, decode_field_expr, encode_field_expr, find_deserialize_constructor,
        json_key, render_constructor_call,
    },
};

/// Emitting the decode side of a generated codec, field by field.
mod decode;
use self::decode::*;

/// Public codecs and optional private implementation scopes.
mod codecs;
pub(crate) use codecs::{emit_deserializer_support_type, emit_serializer_support_type};

/// Dart's standard formatter line width.
const DART_LINE_WIDTH: usize = 80;

/// Width available before the enclosing mixin adds two spaces of indentation.
const MIXIN_MEMBER_WIDTH: usize = DART_LINE_WIDTH - 2;

/// Template context for generated class JSON helpers.
#[derive(Serialize)]
struct ClassTemplateContext<'a> {
    /// Source Dart class name.
    class_name: &'a str,
    /// Typed class use, forwarding the owning parameters.
    class_type: String,
    /// Helper-local declarations, including written bounds.
    type_parameters: String,
    /// Explicit arguments forwarded by wrapper aliases.
    type_arguments: String,
    /// Runtime JSON object type.
    json_type: String,
    /// Protected serialization receiver name.
    instance: String,
    /// Protected deserialization input name.
    json: String,
    /// Rendered helper body.
    body: String,
}

/// Renders the generated `serialize` and `toJson` mixin members for a class.
pub(crate) fn emit_serialize_mixin_members(
    class: &ClassIr,
    helper_class_name: &str,
    helper_type: Option<&dust_ir::TypeIr>,
    names: &JsonNames,
) -> Vec<String> {
    let arguments = helper_type
        .map(crate::variant_types::applied_arguments)
        .unwrap_or_else(|| render_type_arguments(&class.type_parameters));
    let helper_type = helper_type
        .map(|ty| dust_dart_emit::DYNAMIC_TYPES.render(ty))
        .unwrap_or_else(|| format!("{helper_class_name}{arguments}"));
    let json_type = names.json_type();
    let inline = format!(
        "{json_type} serialize() => _${helper_class_name}Serialize{arguments}(this as {helper_type});"
    );
    let serialize = if inline.len() <= MIXIN_MEMBER_WIDTH {
        inline
    } else {
        format!(
            "{json_type} serialize() =>\n    _${helper_class_name}Serialize{arguments}(this as {helper_type});"
        )
    };

    vec![serialize, format!("{json_type} toJson() => serialize();")]
}

/// Renders the top-level helper that serializes a class instance.
pub(crate) fn emit_to_json_helper(
    class: &ClassIr,
    serializable_classes: &[&str],
    serializable_enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> String {
    let mut lines = Vec::new();
    for field in &class.fields {
        if field
            .serde
            .as_ref()
            .is_some_and(|serde| serde.skip_serializing)
        {
            continue;
        }

        let key = json_key(class, &field.name, field.serde.as_ref());
        let value = encode_field_expr(
            &format!("{}.{}", names.local("instance"), field.name),
            field,
            serializable_classes,
            serializable_enums,
            names,
        );
        lines.push(format_prefixed_expr(4, &format!("'{key}': "), &value, ","));
    }

    let body = if lines.is_empty() {
        format!("  return {}{{}};", names.json_arguments())
    } else {
        format!(
            "  return {}{{\n{}\n  }};",
            names.json_arguments(),
            lines.join("\n")
        )
    };

    render_template(
        "to_json_helper",
        include_str!("templates/to_json_helper.jinja"),
        ClassTemplateContext {
            class_name: &class.name,
            class_type: render_class_type(class),
            type_parameters: render_type_parameters(&class.type_parameters),
            type_arguments: render_type_arguments(&class.type_parameters),
            json_type: names.json_type(),
            instance: names.local("instance").into(),
            json: names.local("json").into(),
            body,
        },
    )
}

/// Renders the top-level helper that deserializes a class instance.
pub(crate) fn emit_from_json_helper(
    class: &ClassIr,
    deserializable_classes: &[&str],
    deserializable_enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> Option<String> {
    let constructor = find_deserialize_constructor(class)?;
    let body = emit_from_json_body(
        class,
        constructor,
        deserializable_classes,
        deserializable_enums,
        names,
    )?;
    Some(render_template(
        "from_json_helper",
        include_str!("templates/from_json_helper.jinja"),
        ClassTemplateContext {
            class_name: &class.name,
            class_type: render_class_type(class),
            type_parameters: render_type_parameters(&class.type_parameters),
            type_arguments: render_type_arguments(&class.type_parameters),
            json_type: names.json_type(),
            instance: names.local("instance").into(),
            json: names.local("json").into(),
            body,
        },
    ))
}

/// Renders a private sealed decoder for an explicitly selected constructor.
pub(crate) fn emit_constructor_decode_helper(
    class: &ClassIr,
    constructor: &dust_ir::ConstructorIr,
    function_name: &str,
    deserializable_classes: &[&str],
    deserializable_enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> Option<String> {
    let body = emit_from_json_body(
        class,
        constructor,
        deserializable_classes,
        deserializable_enums,
        names,
    )?;
    Some(format!(
        "{} {function_name}{}({} {}) {{\n{body}\n}}",
        render_class_type(class),
        render_type_parameters(&class.type_parameters),
        names.json_type(),
        names.local("json")
    ))
}

/// Shares field decoding and key validation without changing ordinary ctor selection.
fn emit_from_json_body(
    class: &ClassIr,
    constructor: &dust_ir::ConstructorIr,
    deserializable_classes: &[&str],
    deserializable_enums: EnumCodecs<'_>,
    names: &JsonNames,
) -> Option<String> {
    let mut lines = Vec::new();

    if class
        .serde
        .as_ref()
        .is_some_and(|serde| serde.disallow_unrecognized_keys)
    {
        emit_allowed_key_validation(class, &mut lines, names);
    }

    let mut values = Vec::new();
    for field in &class.fields {
        let field_var = names.local(&format!("{}Value", field.name)).to_owned();
        let value_expr = emit_field_decode(
            class,
            field,
            deserializable_classes,
            deserializable_enums,
            &mut lines,
            names,
        );
        lines.push(format_prefixed_expr(
            2,
            &format!("final {field_var} = "),
            &value_expr,
            ";",
        ));
        values.push((field.name.as_str(), field_var));
    }

    let call = render_constructor_call(class, constructor, &values)?;
    append_constructor_return(&mut lines, &call);

    Some(lines.join("\n"))
}

/// Emits runtime validation for allowed JSON keys.
fn emit_allowed_key_validation(class: &ClassIr, lines: &mut Vec<String>, names: &JsonNames) {
    let allowed_name = names.local("allowedKeys");
    let key_name = names.local("key");
    let json = names.local("json");
    let string = names.reference("String");
    let argument_error = names.reference("ArgumentError");
    let allowed_keys = all_allowed_keys(class);
    if allowed_keys.len() <= 4 {
        let allowed_keys = allowed_keys
            .into_iter()
            .map(|key| format!("'{key}'"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "  const {allowed_name} = <{string}>{{{allowed_keys}}};"
        ));
    } else {
        lines.push(format!("  const {allowed_name} = <{string}>{{"));
        for key in allowed_keys {
            lines.push(format!("    '{key}',"));
        }
        lines.push("  };".to_owned());
    }
    lines.push(format!("  for (final {key_name} in {json}.keys) {{"));
    lines.push(format!("    if (!{allowed_name}.contains({key_name})) {{"));
    lines.push(format!(
        "      throw {argument_error}.value({key_name}, 'json', 'unknown key for {}');",
        class.name
    ));
    lines.push("    }".to_owned());
    lines.push("  }".to_owned());
    lines.push(String::new());
}
