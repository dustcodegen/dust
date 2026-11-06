use std::fs;

use super::{MODEL, build, workspace, write_file};

#[test]
fn newly_queued_earlier_sources_keep_their_diagnostic_ids() {
    let workspace = workspace();
    let root = workspace.path();
    let source = format!(
        "import 'package:dust_dart/derive.dart'; import 'package:bridge/bridge.dart' as t;\n{}",
        MODEL.replace("[ToString()]", "[t.ToString()]")
    );
    let model = root.join("lib/model.dart");
    let foreign = root.join("deps/bridge/lib/a.dart");
    let traits = root.join("deps/dust_dart/lib/src/derive/traits.dart");
    write_file(&model, &source);
    write_file(
        &root.join("deps/bridge/lib/bridge.dart"),
        "export 'package:dust_dart/derive.dart'; export 'a.dart';",
    );
    write_file(&foreign, "class ToString { const ToString(); }");

    let result = build(root);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].message,
        "annotation `ToString` is ambiguous between distinct declarations"
    );
    let labels: Vec<_> = result.diagnostics[0]
        .labels
        .iter()
        .map(|label| {
            let file = result
                .diagnostic_files
                .iter()
                .find(|file| file.file_id == label.file_id)
                .unwrap();
            (
                file.path.as_path(),
                label.file_id.raw(),
                &file.source_text()[label.range.start().to_usize()..label.range.end().to_usize()],
                label.message.as_str(),
            )
        })
        .collect();
    assert_eq!(
        labels,
        [
            (
                model.as_path(),
                1,
                "t.ToString()",
                "cannot bind annotation `t.ToString`",
            ),
            (
                foreign.as_path(),
                5,
                "class ToString { const ToString(); }",
                "package:bridge/a.dart declares ToString",
            ),
            (
                traits.as_path(),
                8,
                "class ToString { const ToString(); }",
                "package:dust_dart/src/derive/traits.dart declares ToString",
            ),
        ]
    );
    assert!(!root.join("lib/model.g.dart").exists());
}

#[test]
fn a_generated_primary_read_before_its_owner_keeps_exact_output_and_cache_hits() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/z.dart");
    let output = root.join("lib/z.a.dart");
    let cache = root.join(".dart_tool/dust/build_cache_v1.json");
    write_file(
        &root.join("dust.yaml"),
        "outputs:\n  primary_suffix: .a.dart\n",
    );
    write_file(
        &model,
        &format!(
            "import 'package:dust_dart/derive.dart';\n{}",
            MODEL.replace("model.g.dart", "z.a.dart")
        ),
    );
    assert_eq!(build(root).diagnostics, []);
    let expected = fs::read(&output).unwrap();

    // This source queues the marked part before z.dart has registered its ownership.
    write_file(&root.join("lib/a.dart"), "import 'z.a.dart';");
    fs::remove_file(cache).unwrap();
    let cold = build(root);
    assert_eq!(cold.diagnostics, []);
    assert_eq!(cold.cache.unwrap().misses, 1);
    assert_eq!(cold.build_artifacts.len(), 1);
    assert_eq!(cold.build_artifacts[0].source_path, model);
    assert_eq!(cold.build_artifacts[0].output_path, output);
    assert!(!cold.build_artifacts[0].written);
    assert_eq!(fs::read(&output).unwrap(), expected);

    let warm = build(root);
    assert_eq!(warm.diagnostics, []);
    assert_eq!(warm.cache.unwrap().hits, 1);
    assert_eq!(warm.build_artifacts.len(), 1);
    assert!(warm.build_artifacts[0].cached);
    assert_eq!(fs::read(output).unwrap(), expected);
}
