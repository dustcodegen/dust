//! Known class binders must fail before JSON emission can guess a conversion.

use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{make_workspace, write_file};

fn model(primary: bool, parameter: &str, ty: &str) -> String {
    let (declaration, arguments) = if parameter.is_empty() {
        (String::new(), String::new())
    } else {
        (
            format!("<{parameter} extends num>"),
            format!("<{parameter}>"),
        )
    };
    let class = if primary {
        format!("class const Captured{declaration}(final {ty} value) with _$Captured{arguments};")
    } else {
        format!(
            "class Captured{declaration} with _$Captured{arguments} {{ const Captured(this.value); final {ty} value; }}"
        )
    };
    format!(
        "import 'package:dust_dart/derive.dart';\nimport 'package:dust_dart/serde.dart';\npart 'model.g.dart';\n@Derive([Serialize(), Deserialize()])\n{class}\n"
    )
}

#[test]
fn serde_parameters_report_field_errors_without_replacing_good_output_or_cache() {
    for primary in [false, true] {
        for fail_fast in [false, true] {
            for existing in [false, true] {
                let workspace = make_workspace();
                let root = workspace.path();
                write_file(
                    &root.join("pubspec.yaml"),
                    "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
                );
                let path = root.join("lib/model.dart");
                let output = root.join("lib/model.g.dart");
                let cache = root.join(".dart_tool/dust/build_cache_v1.json");
                let request = BuildRequest {
                    cwd: root.to_path_buf(),
                    fail_fast,
                    jobs: None,
                    db: Default::default(),
                };
                if existing {
                    write_file(&path, &model(primary, "", "int"));
                    assert_eq!(run_build(request.clone()).diagnostics, []);
                    let warm = run_build(request.clone());
                    assert_eq!(warm.diagnostics, []);
                    assert!(warm.build_artifacts[0].cached);
                }
                let before_output = fs::read(&output).ok();
                let before_cache = fs::read(&cache).ok();
                for (parameter, ty) in [
                    ("DateTime", "DateTime"),
                    ("Uri", "Uri"),
                    ("BigInt", "BigInt"),
                    ("Object", "Object"),
                    ("String", "String"),
                    ("List", "List"),
                    ("Set", "Set"),
                    ("Map", "Map"),
                    ("T", "T?"),
                    ("T", "Set<T>"),
                    ("T", "Map<String, List<T?>>"),
                ] {
                    let source = model(primary, parameter, ty);
                    write_file(&path, &source);
                    for writing in [false, true] {
                        let result = if writing {
                            run_build(request.clone())
                        } else {
                            run_check(CheckRequest {
                                cwd: root.to_path_buf(),
                                fail_fast,
                                jobs: None,
                                db: Default::default(),
                            })
                        };
                        assert_eq!(result.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
                                   ["Serialize", "Deserialize"].map(|direction| format!(
                            "`{direction}` cannot automatically map class parameter `{parameter}` on `Captured.value`; use `SerDe(using: ...)`")));
                        for diagnostic in result.diagnostics {
                            assert!(diagnostic.is_error());
                            assert_eq!(diagnostic.labels.len(), 1);
                            let range = diagnostic.labels[0].range;
                            let selected =
                                &source[range.start().to_usize()..range.end().to_usize()];
                            assert_eq!(selected, "value");
                            assert_eq!(range.start().to_usize(), source.rfind("value").unwrap());
                        }
                        assert_eq!(fs::read(&output).ok(), before_output);
                        assert_eq!(fs::read(&cache).ok(), before_cache);
                    }
                }
            }
        }
    }
}
