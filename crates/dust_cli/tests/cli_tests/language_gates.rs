use crate::helpers::run_cli;

use super::helpers::{DustImport, make_workspace, write_dust_file, write_file};

const USER: &str = "part 'user.g.dart';\n@Derive([ToString()])\nclass User with _$User { const User(this.name); final String name; }\n";

#[test]
fn relative_sdk_path_uses_process_directory_not_root_option() {
    let workspace = make_workspace();
    let root = workspace.path();
    let member = root.join("app");
    super::helpers::prepare(root, &member, "app");
    write_file(&member.join("pubspec.yaml"), "name: app\n");
    write_file(
        &member.join(".dart_tool/package_graph.json"),
        r#"{"configVersion":1,"roots":["app"],"packages":[]}"#,
    );
    write_dust_file(&member.join("lib/user.dart"), &[DustImport::Derive], USER);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_dust"))
        .current_dir(root)
        .env("PATH", ".dust-test-sdk/bin")
        .args(["build", "--root", "app"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(member.join("lib/user.g.dart").is_file());
}

#[test]
fn flutter_workspace_uses_its_sdk_when_standalone_dart_is_first() {
    let workspace = make_workspace();
    let root = workspace.path();
    write_file(
        &root.join("pubspec.yaml"),
        "name: dust_test\ndependencies:\n  flutter:\n    sdk: flutter\n",
    );
    let config_path = root.join(".dart_tool/package_config.json");
    let mut config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    let flutter = config["packages"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|package| package["name"] == "flutter")
        .unwrap();
    flutter["rootUri"] = "../flutter-sdk/packages/flutter".into();
    write_file(&config_path, &config.to_string());
    let sdk = root.join("flutter-sdk/bin/cache/dart-sdk");
    write_file(&sdk.join("version"), "3.13.5\n");
    write_file(
        &sdk.join("lib/libraries.json"),
        r#"{"vm":{"libraries":{"core":{"uri":"core/core.dart"},"ui":{"uri":"ui/ui.dart"}}}}"#,
    );
    write_file(&sdk.join("lib/core/core.dart"), "library dart.core;");
    write_file(&sdk.join("lib/ui/ui.dart"), "library dart.ui;");
    write_dust_file(
        &root.join("lib/user.dart"),
        &[DustImport::Derive],
        &format!("import 'dart:ui';\n{USER}"),
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_dust"))
        .current_dir(root)
        .env("PATH", root.join(".dust-test-sdk/bin"))
        .arg("build")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.join("lib/user.g.dart").is_file());
}

#[test]
fn language_gates_ignore_documentation_and_regex_strings_in_build_and_check() {
    let workspace = make_workspace();
    super::helpers::set_language_version(workspace.path(), "dust_test", "3.6");
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
    super::helpers::set_language_version(workspace.path(), "dust_test", "3.6");
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
