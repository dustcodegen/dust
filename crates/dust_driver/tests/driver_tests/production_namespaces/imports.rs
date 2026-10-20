use std::fs;

use super::{MODEL, build, workspace, write_file};

#[test]
fn shared_prefixes_and_qualifier_trivia_preserve_exact_direct_output() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    write_file(
        &model,
        &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let expected = fs::read(root.join("lib/model.g.dart")).unwrap();
    for imports in [
        "import 'package:bridge/bridge.dart' as d;",
        "import 'package:dust_dart/derive.dart' as d show Derive; import 'package:bridge/bridge.dart' as d show ToString;",
    ] {
        let source = MODEL.replace(
            "@Derive([ToString()])",
            "@d /* qualifier */ . Derive([d.ToString()])",
        );
        write_file(&model, &format!("{imports}\n{source}"));
        assert_eq!(build(root).diagnostics, []);
        assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), expected);
        let warm = build(root);
        assert_eq!(warm.diagnostics, []);
        assert_eq!(warm.cache.unwrap().hits, 1);
    }
}

#[test]
fn conditional_import_and_export_branches_require_the_same_real_owner() {
    for directive in ["import", "export"] {
        let workspace = workspace();
        let root = workspace.path();
        let model = root.join("lib/model.dart");
        let other = root.join("deps/bridge/lib/other.dart");
        write_file(&other, "export 'package:dust_dart/derive.dart';");
        let conditional = format!(
            "{directive} 'package:dust_dart/derive.dart' if (dart.library.io) 'package:bridge/other.dart';"
        );
        let import = if directive == "import" {
            conditional
        } else {
            write_file(&root.join("deps/bridge/lib/bridge.dart"), &conditional);
            "import 'package:bridge/bridge.dart';".into()
        };
        write_file(&model, &format!("{import}\n{MODEL}"));
        assert_eq!(build(root).diagnostics, []);
        let output = fs::read(root.join("lib/model.g.dart")).unwrap();
        let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
        let cache = fs::read(&cache_path).unwrap();
        write_file(
            &other,
            "export 'package:dust_dart/derive.dart' hide ToString; class ToString { const ToString(); }",
        );
        let failed = build(root);
        assert!(failed.has_errors());
        assert_eq!(
            failed
                .diagnostics
                .iter()
                .map(|error| error.message.as_str())
                .collect::<Vec<_>>(),
            ["annotation `ToString` has target-dependent conditional ownership"],
            "{directive}"
        );
        assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), output);
        assert_eq!(fs::read(&cache_path).unwrap(), cache);
        fs::remove_file(other).unwrap();
        let missing = build(root);
        assert!(missing.has_errors());
        assert!(
            missing.diagnostics.iter().all(|error| error
                .message
                .contains("cannot read namespace source")
                && error.message.contains("other.dart")),
            "{:?}",
            missing.diagnostics
        );
        assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), output);
        assert_eq!(fs::read(cache_path).unwrap(), cache);
    }
}

#[test]
fn deferred_metadata_fails_but_unrelated_missing_prefixes_do_not_poison_owners() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    write_file(
        &model,
        &format!("import 'package:bridge/bridge.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let output = fs::read(root.join("lib/model.g.dart")).unwrap();
    write_file(
        &model,
        &format!("import 'package:bridge/bridge.dart'; import 'missing.dart' as other;\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), output);
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let cache = fs::read(&cache_path).unwrap();
    write_file(
        &model,
        &format!(
            "import 'package:bridge/bridge.dart' deferred as d;\n{}",
            MODEL.replace("@Derive([ToString()])", "@d.Derive([d.ToString()])")
        ),
    );
    let failed = build(root);
    assert!(failed.has_errors());
    assert_eq!(
        failed
            .diagnostics
            .iter()
            .map(|error| error.message.as_str())
            .collect::<Vec<_>>(),
        ["metadata cannot use a deferred import"]
    );
    assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), output);
    assert_eq!(fs::read(cache_path).unwrap(), cache);
}
