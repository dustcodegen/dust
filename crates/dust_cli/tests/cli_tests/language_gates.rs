use dust_cli::run_cli;

use super::helpers::{DustImport, make_workspace, write_dust_file, write_file};

const USER: &str = "part 'user.g.dart';\n@Derive([ToString()])\nclass User with _$User { const User(this.name); final String name; }\n";

#[test]
fn language_gates_ignore_documentation_and_regex_strings_in_build_and_check() {
    let workspace = make_workspace();
    write_file(
        &workspace.path().join("pubspec.yaml"),
        "name: dust_test\nenvironment:\n  sdk: '>=3.6.0 <4.0.0'\n",
    );
    let input = workspace.path().join("lib/user.dart");
    write_dust_file(&input, &[DustImport::Derive], USER);
    let build = run_cli(["build", "--root", workspace.path().to_str().unwrap()]);
    assert_eq!(build.exit_code, 0, "{}", build.stderr);
    let output = workspace.path().join("lib/user.g.dart");
    let expected = std::fs::read(&output).unwrap();
    write_dust_file(
        &input,
        &[DustImport::Derive],
        &format!(
            r#"// Documentation is not executable feature syntax: ...? = .ready
{USER}
const pattern = r'(?:[.,](\d+))?';
"#
        ),
    );
    for command in ["build", "check"] {
        let run = run_cli([command, "--root", workspace.path().to_str().unwrap()]);
        assert_eq!(run.exit_code, 0, "{}", run.stderr);
        assert_eq!(std::fs::read(&output).unwrap(), expected);
    }
}

#[test]
fn real_inline_future_syntax_preserves_successful_output_and_cache() {
    let workspace = make_workspace();
    write_file(
        &workspace.path().join("pubspec.yaml"),
        "name: dust_test\nenvironment:\n  sdk: '>=3.6.0 <4.0.0'\n",
    );
    let input = workspace.path().join("lib/user.dart");
    write_dust_file(&input, &[DustImport::Derive], USER);
    let build = run_cli(["build", "--root", workspace.path().to_str().unwrap()]);
    assert_eq!(build.exit_code, 0, "{}", build.stderr);
    let output = workspace.path().join("lib/user.g.dart");
    let cache = workspace.path().join(".dart_tool/dust/build_cache_v1.json");
    let previous_output = std::fs::read(&output).unwrap();
    let previous_cache = std::fs::read(&cache).unwrap();
    write_dust_file(
        &input,
        &[DustImport::Derive],
        &format!("{USER}\nint? value; final items=[?value];\n"),
    );
    for command in ["build", "check", "build"] {
        let run = run_cli([command, "--root", workspace.path().to_str().unwrap()]);
        assert_eq!(run.exit_code, 1, "{command}: {}", run.stdout);
        assert!(run.stdout.is_empty());
        assert!(
            run.stderr
                .contains("null-aware collection elements require Dart 3.8 or newer"),
            "{}",
            run.stderr
        );
        assert_eq!(std::fs::read(&output).unwrap(), previous_output);
        assert_eq!(std::fs::read(&cache).unwrap(), previous_cache);
    }
}
