use std::fs;

use super::{MODEL, build, workspace, write_file};

#[test]
fn equivalent_relative_primary_parts_preserve_exact_generation_and_warm_hits() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let output = fs::read(&output_path).unwrap();
    for reference in [
        "./model.g.dart",
        "scratch/../model.g.dart",
        "./scratch/../../lib/model.g.dart",
    ] {
        fs::remove_file(&output_path).unwrap();
        write_file(
            &model,
            &source.replace("part 'model.g.dart';", &format!("part '{reference}';")),
        );
        let cold = build(root);
        assert_eq!(cold.diagnostics, [], "{reference}");
        assert_eq!(cold.cache.unwrap().misses, 1);
        assert_eq!(fs::read(&output_path).unwrap(), output, "{reference}");
        let warm = build(root);
        assert_eq!(warm.diagnostics, [], "{reference}");
        assert_eq!(warm.cache.unwrap().hits, 1);
    }
}

#[test]
fn missing_foreign_part_is_not_a_generated_placeholder_even_with_the_same_basename() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    let source = format!("import 'package:dust_dart/derive.dart';\n{MODEL}");
    write_file(&model, &source);
    assert_eq!(build(root).diagnostics, []);
    let output_path = root.join("lib/model.g.dart");
    let cache_path = root.join(".dart_tool/dust/build_cache_v1.json");
    let output = fs::read(&output_path).unwrap();
    let cache = fs::read(&cache_path).unwrap();
    write_file(
        &model,
        &source.replace("part 'model.g.dart';", "part 'foreign/model.g.dart';"),
    );
    let failed = build(root);
    assert!(failed.has_errors());
    assert!(
        failed.diagnostics.iter().all(|error| error
            .message
            .contains("cannot read namespace source")
            && error.message.contains("foreign/model.g.dart")),
        "{:?}",
        failed.diagnostics
    );
    assert_eq!(fs::read(output_path).unwrap(), output);
    assert_eq!(fs::read(cache_path).unwrap(), cache);
    assert!(!root.join("lib/foreign/model.g.dart").exists());
}
