use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use crate::support::{DustImport, make_workspace, write_dust_file, write_file};

#[test]
fn invalid_generic_mixins_preserve_previous_outputs_and_cache_in_build_and_check() {
    for fail_fast in [false, true] {
        let workspace = make_workspace();
        let root = workspace.path();
        write_file(
            &root.join("pubspec.yaml"),
            "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
        );
        let source = root.join("lib/box.dart");
        let output = root.join("lib/box.g.dart");
        let cache = root.join(".dart_tool/dust/build_cache_v1.json");
        write_dust_file(
            &source,
            &[DustImport::Derive],
            "part 'box.g.dart';\n@Derive([ToString()])\nclass Box with _$Box { const Box(); }\n",
        );
        let request = BuildRequest {
            cwd: root.to_path_buf(),
            fail_fast,
            jobs: None,
            db: Default::default(),
        };
        let built = run_build(request.clone());
        assert!(!built.has_errors(), "{:?}", built.diagnostics);
        let previous_output = fs::read(&output).unwrap();
        let previous_cache = fs::read(&cache).unwrap();

        for declaration in [
            "class Box<T> with _$Box { const Box(this.value); final T value; }",
            "class const Box<T>(final T value) with _$Box;",
            "class Box<T> with _$Box<int> { const Box(this.value); final T value; }",
            "class const Box<T>(final T value) with _$Box<int>;",
        ] {
            write_dust_file(
                &source,
                &[DustImport::Derive],
                &format!("part 'box.g.dart';\n@Derive([ToString()])\n{declaration}\n"),
            );
            let checked = run_check(CheckRequest {
                cwd: root.to_path_buf(),
                fail_fast,
                jobs: None,
                db: Default::default(),
            });
            let built = run_build(request.clone());
            for diagnostics in [&checked.diagnostics, &built.diagnostics] {
                assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
                let diagnostic = &diagnostics[0];
                assert!(diagnostic.is_error());
                assert_eq!(
                    diagnostic.message,
                    "Generic model `Box` must apply `with _$Box<T>`"
                );
                assert_eq!(diagnostic.labels.len(), 1);
                let text = fs::read_to_string(&source).unwrap();
                let label = &diagnostic.labels[0];
                assert_eq!(
                    &text[label.range.start().to_usize()..label.range.end().to_usize()],
                    if declaration.contains("_$Box<int>") {
                        "_$Box<int>"
                    } else {
                        "_$Box"
                    }
                );
            }
            assert_eq!(fs::read(&output).unwrap(), previous_output);
            assert_eq!(fs::read(&cache).unwrap(), previous_cache);
        }
    }
}
