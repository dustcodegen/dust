use std::fs;

use dust_db_plugin::register_plugin;
use dust_ir::{DartFileIr, MethodIr, TypeIr};
use dust_plugin_api::{DustPlugin, PluginContext, SymbolPlan};

use crate::support::*;

fn dao_library(root: &std::path::Path, import: &str) -> DartFileIr {
    let mut dao = dao_class();
    dao.methods = [
        ("one", TypeIr::named("UserProfile")),
        ("optional", TypeIr::named("UserProfile").nullable()),
        (
            "all",
            TypeIr::generic("List", vec![TypeIr::named("UserProfile")]),
        ),
    ]
    .into_iter()
    .map(|(name, ty)| MethodIr {
        name: name.to_owned(),
        return_type: result_type(ty),
        params: Vec::new(),
        configs: vec![config(
            "dust_dart::Query",
            "(r'SELECT id, display_name FROM users')",
        )],
        ..dao.methods[0].clone()
    })
    .collect();
    library_with_imports(root, "lib/dao/user.dart", vec![import], vec![dao])
}

fn row_library(root: &std::path::Path) -> DartFileIr {
    library_at(
        root,
        "lib/models/user.dart",
        vec![simple_user_row_class()],
        Vec::new(),
    )
}

fn primary_source(root: &std::path::Path) {
    fs::create_dir_all(root.join("lib/models")).unwrap();
    fs::write(
        root.join("lib/models/user.dart"),
        "@Derive([FromRow()])\nclass const UserProfile(final int id, final String name);\n",
    )
    .unwrap();
}

#[test]
fn imported_primary_rows_validate_from_workspace_facts() {
    let root = tempfile::tempdir().unwrap();
    primary_source(root.path());
    let plugin = register_plugin();
    let row = row_library(root.path());
    for import in ["../models/user.dart", "package:example/models/user.dart"] {
        let dao = dao_library(root.path(), import);
        assert_eq!(validate_in_package(&plugin, &[&row, &dao], &dao), []);
    }
}

#[test]
fn imported_primary_dao_output_matches_traditional_output_exactly() {
    let root = tempfile::tempdir().unwrap();
    primary_source(root.path());
    let plugin = register_plugin();
    let row = row_library(root.path());
    let dao = dao_library(root.path(), "../models/user.dart");
    fs::write(
        root.path().join("lib/models/user.dart"),
        "@Derive([FromRow()])\nclass UserProfile {}\n",
    )
    .unwrap();
    let expected = plugin.generate(
        &dao,
        &PluginContext {
            symbol_plan: &SymbolPlan::default(),
        },
    )[0]
    .support_types
    .clone();
    primary_source(root.path());
    let plan = package_plan(&plugin, &[&row, &dao]);
    let actual = plugin.generate(&dao, &PluginContext { symbol_plan: &plan });
    assert_eq!(actual[0].support_types, expected);
}

#[test]
fn workspace_row_facts_do_not_make_unimported_rows_eligible() {
    let root = tempfile::tempdir().unwrap();
    primary_source(root.path());
    let plugin = register_plugin();
    let mut row = row_library(root.path());
    let dao = dao_library(root.path(), "../models/user.dart");
    for foreign_package in [false, true] {
        if foreign_package {
            row.package_name = "other".to_owned();
            row.source_path = "lib/models/user.dart".to_owned();
        } else {
            row.source_path = "lib/not_imported.dart".to_owned();
        }
        let diagnostics = validate_in_package(&plugin, &[&row, &dao], &dao);
        let mut expected = vec![
            "unsupported SqlxDao result type `UserProfile` on `UserDao.one`",
            "unsupported SqlxDao result type `UserProfile?` on `UserDao.optional`",
            "unsupported SqlxDao result type `List<UserProfile>` on `UserDao.all`",
        ];
        if foreign_package {
            expected.extend([
                "queryAs<UserProfile> row type has no row mapping. Add `@Derive([FromRow()])` to `UserProfile`, or pass one with `mapper:` or `using:`";
                3
            ]);
        }
        assert_eq!(
            diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }
}
