use dust_ir::TypeIr;

use crate::DYNAMIC_TYPES;

/// Direction of a public generated Dust Serde codec.
#[derive(Clone, Copy)]
pub enum SerdeCodecDirection {
    /// Converts a Dart model into its JSON representation.
    Serialize,
    /// Converts a JSON representation into a Dart model.
    Deserialize,
}

/// Renders a public codec constructor, retaining import prefix and type arguments.
pub fn render_serde_codec_constructor(
    name: &str,
    args: &[TypeIr],
    direction: SerdeCodecDirection,
) -> String {
    let suffix = match direction {
        SerdeCodecDirection::Serialize => "Serializer",
        SerdeCodecDirection::Deserialize => "Deserializer",
    };
    let helper = match name.split_once('.') {
        Some((prefix, name)) => format!("{prefix}.${name}{suffix}"),
        None => format!("${name}{suffix}"),
    };
    let arguments = if args.is_empty() {
        String::new()
    } else {
        format!(
            "<{}>",
            args.iter()
                .map(|ty| DYNAMIC_TYPES.render(ty))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!("{helper}{arguments}()")
}

/// Constructor spelling and generic-argument regression tests.
#[cfg(test)]
mod tests {
    use super::*;

    /// Preserves public helpers for local, prefixed and generic models.
    #[test]
    fn codec_constructors_keep_public_names_prefixes_and_nested_type_arguments() {
        assert_eq!(
            render_serde_codec_constructor("Message", &[], SerdeCodecDirection::Serialize),
            "$MessageSerializer()"
        );
        assert_eq!(
            render_serde_codec_constructor("dto.Message", &[], SerdeCodecDirection::Deserialize),
            "dto.$MessageDeserializer()"
        );
        assert_eq!(
            render_serde_codec_constructor(
                "dto.Envelope",
                &[
                    TypeIr::generic("List", vec![TypeIr::named("dto.Event")]),
                    TypeIr::string().nullable()
                ],
                SerdeCodecDirection::Serialize
            ),
            "dto.$EnvelopeSerializer<List<dto.Event>, String?>()"
        );
    }
}
