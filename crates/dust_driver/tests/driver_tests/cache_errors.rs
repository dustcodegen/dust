use std::fs;

use dust_driver::{BuildRequest, CheckRequest, run_build, run_check};

use super::support::{DustImport, make_workspace, write_dust_file};

const VALID: &str = "part 'box.g.dart';\n@Derive([ToString()])\nclass Box with _$Box { const Box(this.value); final String value; }\n";
const INVALID_DERIVE: &str = "part 'box.g.dart';\n@Derive([CopyWith()])\nclass Box with _$Box { const Box(); final String value = 'fixed'; }\n";
const INVALID_SYNTAX: &str =
    "part 'box.g.dart';\n@Derive([ToString()])\nclass Box { final String value;";

#[test]
fn failed_builds_never_create_or_replace_successful_cache_state() {
    for fail_fast in [false, true] {
        for existing in [false, true] {
            for invalid in [INVALID_DERIVE, INVALID_SYNTAX] {
                let workspace = make_workspace();
                let root = workspace.path();
                let source = root.join("lib/box.dart");
                let output = root.join("lib/box.g.dart");
                let cache = root.join(".dart_tool/dust/build_cache_v1.json");
                let build = BuildRequest {
                    cwd: root.to_path_buf(),
                    fail_fast,
                    jobs: None,
                    db: Default::default(),
                };
                let check = CheckRequest {
                    cwd: root.to_path_buf(),
                    fail_fast,
                    jobs: None,
                    db: Default::default(),
                };
                if existing {
                    write_dust_file(&source, &[DustImport::Derive], VALID);
                    let built = run_build(build.clone());
                    assert!(!built.has_errors(), "{:?}", built.diagnostics);
                    assert!(output.exists());
                    assert!(cache.exists());
                    let cached = run_build(build.clone());
                    assert!(!cached.has_errors());
                    assert_eq!(cached.build_artifacts.len(), 1);
                    assert!(cached.build_artifacts[0].cached);
                }
                let previous_output = fs::read_to_string(&output).ok();
                let previous_cache = fs::read_to_string(&cache).ok();
                write_dust_file(&source, &[DustImport::Derive], invalid);
                for writing in [false, true, false, true] {
                    let result = if writing {
                        run_build(build.clone())
                    } else {
                        run_check(check.clone())
                    };
                    assert!(result.has_errors(), "{invalid}: {:?}", result.diagnostics);
                    assert_eq!(fs::read_to_string(&output).ok(), previous_output);
                    assert_eq!(fs::read_to_string(&cache).ok(), previous_cache);
                }
            }
        }
    }
}
