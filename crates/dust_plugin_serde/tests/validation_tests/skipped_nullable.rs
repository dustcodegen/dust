use super::*;
use dust_ir::SerdeFieldConfigIr;

#[test]
fn skipped_nullable_fields_allow_implicit_null_but_non_nullable_fields_need_defaults() {
    for nullable in [false, true] {
        for both_directions in [false, true] {
            let ty = if nullable {
                TypeIr::string().nullable()
            } else {
                TypeIr::string()
            };
            let mut skipped = field("local", ty.clone());
            skipped.serde = Some(SerdeFieldConfigIr {
                skip_deserializing: true,
                skip_serializing: both_directions,
                ..Default::default()
            });
            let target = class(
                "Skipped",
                vec![skipped],
                vec![constructor(
                    None,
                    vec![constructor_param("local", ty, ParamKind::Named)],
                )],
                &["dust_dart::Serialize", "dust_dart::Deserialize"],
            );
            let diagnostics = register_plugin().validate(&library(vec![target], vec![]));
            let messages = diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                messages,
                if nullable {
                    vec![]
                } else {
                    vec![
                        "field `local` on class `Skipped` uses `skipDeserializing` without a `defaultValue`",
                    ]
                }
            );
        }
    }
}
