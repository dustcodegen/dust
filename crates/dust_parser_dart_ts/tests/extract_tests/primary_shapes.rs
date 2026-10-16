use dust_parser_dart::ParsedClassKind;

use crate::support::parse;

#[test]
fn primary_enum_variants_keep_their_names_and_serde_metadata() {
    let result = parse(
        313,
        r#"
@Derive([Serialize(), Deserialize()])
enum Status(final String label) {
  @SerDe(rename: 'enabled') active('Active'),
  inactive('Inactive');
}
"#,
    );
    assert_eq!(result.diagnostics, []);
    assert_eq!(result.library.enums.len(), 1);
    let enumeration = &result.library.enums[0];
    assert_eq!(enumeration.name, "Status");
    assert_eq!(enumeration.annotations[0].name, "Derive");
    assert_eq!(
        enumeration
            .variants
            .iter()
            .map(|variant| variant.name.as_str())
            .collect::<Vec<_>>(),
        ["active", "inactive"]
    );
    assert_eq!(enumeration.variants[0].annotations[0].name, "SerDe");
    assert_eq!(
        enumeration.variants[0].annotations[0]
            .arguments_source
            .as_deref(),
        Some("(rename: 'enabled')")
    );
    assert_eq!(enumeration.variants[1].annotations, []);
}

#[test]
fn explicit_new_header_is_unnamed_and_trivial_mixin_constructor_has_no_fields() {
    let result = parse(
        313,
        "class const Record.new(final int id); mixin class Marker();",
    );
    assert_eq!(result.diagnostics, []);
    let record = &result.library.classes[0];
    assert_eq!(record.name, "Record");
    assert_eq!(record.constructors.len(), 1);
    assert_eq!(record.constructors[0].name, None);
    assert_eq!(record.fields[0].name, "id");
    assert_eq!(record.fields[0].type_source.as_deref(), Some("int"));
    let marker = &result.library.classes[1];
    assert_eq!(marker.kind, ParsedClassKind::MixinClass);
    assert_eq!(marker.name, "Marker");
    assert_eq!(marker.fields, []);
    assert_eq!(marker.constructors.len(), 1);
    assert_eq!(marker.constructors[0].name, None);
    assert_eq!(marker.constructors[0].params, []);
}

#[test]
fn extension_type_representation_does_not_acquire_the_class_primary_version_gate() {
    for version in ["3.3", "3.13"] {
        let result = parse(
            313,
            &format!("// @dart={version}\nextension type const RecordId(int value) {{}}"),
        );
        assert_eq!(result.diagnostics, [], "{version}");
        assert_eq!(result.library.extension_types.len(), 1);
        let representation = &result.library.extension_types[0];
        assert_eq!(representation.name, "RecordId");
        assert_eq!(representation.representation_name, "value");
        assert_eq!(
            representation.representation_type_source.as_deref(),
            Some("int")
        );
    }
}
