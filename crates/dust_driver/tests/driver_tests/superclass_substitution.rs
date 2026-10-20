use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

fn source(primary: bool, opaque: bool) -> String {
    let ty = if opaque { "T Function(T)" } else { "T" };
    let classes = if primary {
        format!(
            "class const Base<T>(final {ty} value);
             class const Middle(super.value) extends Base<int>;
             @Derive([CopyWith()]) class const Child(super.value) extends Middle with _$Child;"
        )
    } else {
        format!(
            "class Base<T> {{ const Base(this.value); final {ty} value; }}
             class Middle extends Base<int> {{ const Middle(super.value); }}
             @Derive([CopyWith()]) class Child extends Middle with _$Child {{ const Child(super.value); }}"
        )
    };
    format!("import 'package:dust_dart/derive.dart';\npart 'child.g.dart';\n{classes}\n")
}

#[test]
fn inherited_errors_survive_unannotated_parent_cache_and_preserve_output() {
    for primary in [false, true] {
        for fail_fast in [false, true] {
            for existing in [false, true] {
                let workspace = make_workspace();
                let root = workspace.path();
                write_file(
                    &root.join("pubspec.yaml"),
                    "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
                );
                let input = root.join("lib/child.dart");
                let output = root.join("lib/child.g.dart");
                let cache = root.join(".dart_tool/dust/build_cache_v1.json");
                let request = BuildRequest {
                    sdk_source_root: Some(crate::support::sdk_source_root(root)),
                    cwd: root.to_path_buf(),
                    fail_fast,
                    jobs: None,
                    db: Default::default(),
                };
                if existing {
                    write_file(&input, &source(primary, false));
                    assert_eq!(run_build(request.clone()).diagnostics, []);
                    let warm = run_build(request.clone());
                    assert_eq!(warm.diagnostics, []);
                    assert!(warm.build_artifacts[0].cached);
                }
                let before_output = fs::read(&output).ok();
                let before_cache = fs::read(&cache).ok();
                let bad = source(primary, true);
                write_file(&input, &bad);
                for writing in [false, true] {
                    let result = if writing {
                        run_build(request.clone())
                    } else {
                        run_check(CheckRequest {
                            sdk_source_root: Some(crate::support::sdk_source_root(root)),
                            cwd: root.to_path_buf(),
                            fail_fast,
                            jobs: None,
                            db: Default::default(),
                        })
                    };
                    assert_eq!(
                        result
                            .diagnostics
                            .iter()
                            .map(|d| d.message.as_str())
                            .collect::<Vec<_>>(),
                        [
                            "Dust cannot substitute inherited opaque function type on `Middle.value` through `Base` yet"
                        ]
                    );
                    let diagnostic = &result.diagnostics[0];
                    assert!(diagnostic.is_error());
                    assert_eq!(diagnostic.labels.len(), 2);
                    let range = diagnostic.labels[0].range;
                    assert_eq!(
                        &bad[range.start().to_usize()..range.end().to_usize()],
                        "Base<int>"
                    );
                    assert_eq!(fs::read(&output).ok(), before_output);
                    assert_eq!(fs::read(&cache).ok(), before_cache);
                }
            }
        }
    }
}

#[test]
fn unused_opaque_inheritance_does_not_block_an_unrelated_model() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_file(
        &root.join("lib/child.dart"),
        "import 'package:dust_dart/derive.dart'; part 'child.g.dart';
         class Base<T> { final T Function(T) value; const Base(this.value); }
         class Unused extends Base<int> { const Unused(super.value); }
         @Derive([CopyWith()]) class Good with _$Good { const Good(this.value); final int value; }",
    );
    let report = run_build(BuildRequest {
        sdk_source_root: Some(crate::support::sdk_source_root(root)),
        cwd: root.to_path_buf(),
        fail_fast: false,
        jobs: None,
        db: Default::default(),
    });
    assert_eq!(report.diagnostics, []);
    assert_eq!(report.build_artifacts.len(), 1);
}
