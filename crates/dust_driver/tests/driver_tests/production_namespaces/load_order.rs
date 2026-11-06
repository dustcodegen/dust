use std::{fs, path::Path};

use dust_driver::{BuildRequest, CheckRequest, CommandResult, run_build, run_check};

use super::{MODEL, workspace, write_file};

fn build(root: &Path, jobs: usize) -> CommandResult {
    let mut result = run_build(BuildRequest {
        sdk_source_root: Some(root.join("sdk")),
        cwd: root.into(),
        fail_fast: false,
        jobs: Some(jobs),
        db: Default::default(),
    });
    result.elapsed_ms = 0;
    result
}

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

    let result = build(root, 1);
    assert_eq!(build(root, 4), result);
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
    assert_eq!(build(root, 1).diagnostics, []);
    let expected = fs::read(&output).unwrap();

    // This source queues the marked part before z.dart has registered its ownership.
    write_file(&root.join("lib/a.dart"), "import 'z.a.dart';");
    let run = |jobs| {
        fs::remove_file(&cache).unwrap();
        let cold = build(root, jobs);
        assert_eq!(fs::read(&output).unwrap(), expected);
        let warm = build(root, jobs);
        assert_eq!(fs::read(&output).unwrap(), expected);
        let mut checked = run_check(CheckRequest {
            sdk_source_root: Some(root.join("sdk")),
            cwd: root.into(),
            fail_fast: false,
            jobs: Some(jobs),
            db: Default::default(),
        });
        checked.elapsed_ms = 0;
        (cold, warm, checked, fs::read(&cache).unwrap())
    };
    let serial = run(1);
    assert_eq!(run(4), serial);
    let (cold, warm, checked, _) = serial;

    assert_eq!(cold.diagnostics, []);
    assert_eq!(cold.cache.unwrap().misses, 1);
    assert_eq!(cold.build_artifacts.len(), 1);
    assert_eq!(cold.build_artifacts[0].source_path, model);
    assert_eq!(cold.build_artifacts[0].output_path, output);
    assert!(!cold.build_artifacts[0].written);
    assert_eq!(warm.diagnostics, []);
    assert_eq!(warm.cache.unwrap().hits, 1);
    assert_eq!(warm.build_artifacts.len(), 1);
    assert!(warm.build_artifacts[0].cached);
    assert_eq!(checked.diagnostics, []);
    assert_eq!(checked.checked_libraries.len(), 1);
    assert!(checked.checked_libraries[0].cached);
}

#[test]
fn parallel_parser_preserves_root_diagnostic_labels() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    write_file(
        &model,
        &format!(
            "// @dart=3.13\nimport 'package:dust_dart/derive.dart';\n{MODEL}\nenum Kind {{ ready }}\nKind choose() => .ready;"
        ),
    );
    let serial = build(root, 1);
    assert_eq!(build(root, 4), serial);
    assert_eq!(serial.diagnostics.len(), 1);
    let diagnostic = &serial.diagnostics[0];
    assert_eq!(
        diagnostic.message,
        "Dust does not support dot shorthands yet"
    );
    assert_eq!(diagnostic.labels.len(), 1);
    assert_eq!(diagnostic.labels[0].file_id.raw(), 1);
}
