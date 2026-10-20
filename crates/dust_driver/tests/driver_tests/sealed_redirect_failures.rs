use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

fn source(primary: bool, target: &str) -> String {
    let header = if primary {
        "sealed class const Event<T>() with _$Event<T> { this;"
    } else {
        "sealed class Event<T> with _$Event<T> { const Event();"
    };
    format!(
        "import 'package:dust_dart/serde.dart'; part 'event.g.dart'; @Derive([Serialize(), Deserialize()]) @SerDe(tag: 'kind') {header} const factory Event.done(int number) = {target}; }}"
    )
}

#[test]
fn unsupported_sealed_target_applications_preserve_output_and_cache_in_build_and_check() {
    for primary in [false, true] {
        for fail_fast in [false, true] {
            for existing in [false, true] {
                for (target, reason) in [
                    (
                        "Done<List<T>>",
                        "generated target arguments must be bare owning type parameters",
                    ),
                    (
                        "p.Done<T>",
                        "redirect target ownership is not proven in this library",
                    ),
                    (
                        "Done<T, T>",
                        "generated target must carry every owning type parameter explicitly",
                    ),
                ] {
                    let workspace = make_workspace();
                    let root = workspace.path();
                    write_file(
                        &root.join("pubspec.yaml"),
                        "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
                    );
                    let input = root.join("lib/event.dart");
                    let output = root.join("lib/event.g.dart");
                    let cache = root.join(".dart_tool/dust/build_cache_v1.json");
                    let request = BuildRequest {
                        sdk_source_root: Some(crate::support::sdk_source_root(root)),
                        cwd: root.to_path_buf(),
                        fail_fast,
                        jobs: None,
                        db: Default::default(),
                    };
                    if existing {
                        write_file(&input, &source(primary, "Done<T>"));
                        assert_eq!(run_build(request.clone()).diagnostics, []);
                        let warm = run_build(request.clone());
                        assert_eq!(warm.diagnostics, []);
                        assert!(warm.build_artifacts[0].cached);
                    }
                    let before_output = fs::read(&output).ok();
                    let before_cache = fs::read(&cache).ok();
                    let bad = source(primary, target);
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
                                .map(|diagnostic| diagnostic.message.as_str())
                                .collect::<Vec<_>>(),
                            [format!(
                                "Dust cannot generate sealed variant `Event.done`: {reason}"
                            )]
                        );
                        let diagnostic = &result.diagnostics[0];
                        assert!(diagnostic.is_error());
                        assert_eq!(diagnostic.labels.len(), 1);
                        let range = diagnostic.labels[0].range;
                        assert_eq!(
                            &bad[range.start().to_usize()..range.end().to_usize()],
                            target
                        );
                        assert_eq!(fs::read(&output).ok(), before_output);
                        assert_eq!(fs::read(&cache).ok(), before_cache);
                    }
                }
            }
        }
    }
}
