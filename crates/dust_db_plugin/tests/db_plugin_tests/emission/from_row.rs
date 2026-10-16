use dust_db_plugin::register_row_plugin;
use dust_plugin_api::{DustPlugin, SymbolPlan};

use crate::support::{library, row_class};

#[test]
fn private_named_fields_keep_public_labels_and_explicit_sql_column_names() {
    let plugin = register_row_plugin();
    let plan = SymbolPlan::default();
    let context = dust_plugin_api::PluginContext { symbol_plan: &plan };
    let public = plugin
        .generate(&library(vec![row_class()]), &context)
        .remove(0);
    let mut private_row = row_class();
    private_row
        .fields
        .iter_mut()
        .find(|field| field.name == "name")
        .unwrap()
        .name = "_name".to_owned();
    private_row.constructors[0]
        .params
        .iter_mut()
        .find(|param| param.name == "name")
        .unwrap()
        .name = "_name".to_owned();
    let private = plugin
        .generate(&library(vec![private_row]), &context)
        .remove(0);
    assert_eq!(private.support_types, public.support_types);
}

#[test]
fn emits_sqlx_style_from_row_mapper() {
    let plugin = register_row_plugin();
    let contribution = plugin
        .generate(
            &library(vec![row_class()]),
            &dust_plugin_api::PluginContext {
                symbol_plan: &SymbolPlan::default(),
            },
        )
        .into_iter()
        .next()
        .expect("plugin must generate one contribution");

    assert_eq!(
        contribution.support_types[0],
        r#"UserProfile _$UserProfileFromRow(Row row) {
  return UserProfile(
    id: row.read<int>('id'),
    name: row.read<String>('display_name'),
    bio: row.readNullable<Object?>('bio') == null ? '' : row.read<String>('bio'),
    sessionActive: false,
    preferences: UserPreferences.fromJson(decodeJsonObject(row.read<String>('preferences'))),
    status: const UserStatusFromInt().decode(row.read<int>('status')),
  );
}

/// Row deserializer for [UserProfile].
final class $UserProfileRowDeserializer implements RowDeserializer<UserProfile> {
  const $UserProfileRowDeserializer();

  @override
  UserProfile deserialize(Row row) => _$UserProfileFromRow(row);
}

/// Typed row query terminals for [UserProfile].
///
/// Resolved from the static type of the receiver, so a row type with no
/// `FromRow` has no terminals and the call does not compile.
extension $UserProfileQuery on QueryAs<UserProfile> {
  /// Fetches exactly one row.
  Future<Result<UserProfile, SqlxError>> fetchOne(Executor db) =>
      fetchOneWith(db, _$UserProfileFromRow);

  /// Fetches zero or one row.
  Future<Result<UserProfile?, SqlxError>> fetchOptional(Executor db) =>
      fetchOptionalWith(db, _$UserProfileFromRow);

  /// Fetches every row.
  Future<Result<List<UserProfile>, SqlxError>> fetchAll(Executor db) =>
      fetchAllWith(db, _$UserProfileFromRow);
}"#
    );
}
