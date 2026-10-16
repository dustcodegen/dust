use dust_cli::run_cli;

use super::helpers::{DustImport, make_workspace, write_dust_file, write_file};

#[test]
fn cli_build_renders_warning_details() {
    let workspace = make_workspace();
    write_dust_file(
        &workspace.path().join("lib/user.dart"),
        &[DustImport::Derive],
        "part 'user.g.dart';\n\
         @Derive([ToString(), UnknownTrait()])\n\
         class User {\n\
           final String id;\n\
           const User(this.id);\n\
         }\n",
    );

    let run = run_cli(["build", "--root", workspace.path().to_str().unwrap()]);

    assert_eq!(run.exit_code, 0, "{}", run.stderr);
    assert!(run.stderr.is_empty());
    assert!(
        run.stdout
            .contains("diagnostics  errors: 0  warnings: 1  notes: 0")
    );
    assert!(
        run.stdout
            .contains("warning: unknown derive trait or config `UnknownTrait`")
    );
    assert!(run.stdout.contains(&format!(
        "{}:3:",
        workspace.path().join("lib/user.dart").display()
    )));
    assert!(
        run.stdout
            .contains("3 | @Derive([ToString(), UnknownTrait()])")
    );
    assert!(
        run.stdout
            .contains("annotation member is not owned by any registered symbol")
    );
    assert!(run.stdout.contains("^^^^"));
}

#[test]
fn cli_build_renders_error_notes() {
    let workspace = make_workspace();
    write_dust_file(
        &workspace.path().join("lib/audit.dart"),
        &[DustImport::Derive],
        "part 'audit.g.dart';\n\
         @Derive([Serialize(), Deserialize()])\n\
         class Audit {\n\
           const Audit({required this.createdAt});\n\
           @SerDe(using: DateTimeCodec)\n\
           final DateTime createdAt;\n\
           factory Audit.fromJson(Map<String, Object?> json) => _$AuditFromJson(json);\n\
         }\n",
    );

    let run = run_cli(["build", "--root", workspace.path().to_str().unwrap()]);

    assert_eq!(run.exit_code, 1);
    assert!(run.stdout.is_empty());
    assert!(
        run.stderr
            .contains("diagnostics  errors: 1  warnings: 0  notes: 0")
    );
    assert!(run.stderr.contains(
        "error: field `createdAt` uses suspicious `SerDe(using: ...)` type reference `DateTimeCodec`"
    ));
    assert!(run.stderr.contains(
        "Use a codec object such as `const UnixEpochDateTimeCodec()` or `unixEpochDateTimeCodec`."
    ));
}

#[test]
fn unsupported_syntax_fails_build_and_check_without_replacing_output() {
    for (declaration, feature) in [
        (
            "class const User(final String name) with _$User;",
            "primary constructors",
        ),
        (
            "class User with _$User { const new create(this.name); final String name; }",
            "concise constructors",
        ),
        (
            "class User with _$User { factory create() => User._(); const User._(); }",
            "concise constructors",
        ),
    ] {
        for existing in [false, true] {
            let workspace = make_workspace();
            write_file(
                &workspace.path().join("pubspec.yaml"),
                "name: dust_test\nenvironment:\n  sdk: '>=3.13.0 <4.0.0'\n",
            );
            let input = workspace.path().join("lib/user.dart");
            let output = workspace.path().join("lib/user.g.dart");
            if existing {
                write_dust_file(
                    &input,
                    &[DustImport::Derive],
                    "part 'user.g.dart';\n@Derive([ToString()])\nclass User with _$User { const User(this.name); final String name; }\n",
                );
                let run = run_cli(["build", "--root", workspace.path().to_str().unwrap()]);
                assert_eq!(run.exit_code, 0, "{}", run.stderr);
            }
            let previous = std::fs::read(&output).ok();
            assert_eq!(previous.is_some(), existing);
            write_dust_file(
                &input,
                &[DustImport::Derive],
                &format!("part 'user.g.dart';\n@Derive([ToString()])\n{declaration}\n"),
            );
            for command in ["build", "check"] {
                let run = run_cli([command, "--root", workspace.path().to_str().unwrap()]);
                assert_eq!(run.exit_code, 1, "{command}: {}", run.stdout);
                assert!(run.stdout.is_empty());
                assert!(
                    run.stderr
                        .contains(&format!("Dust does not support {feature} yet"))
                );
                assert!(run.stderr.contains("Check this file with `dart analyze`."));
                assert!(run.stderr.contains("Dust's bundled parser"));
                assert!(run.stderr.contains("Use traditional in-body constructors"));
                assert!(run.stderr.contains(input.to_str().unwrap()));
                assert_eq!(std::fs::read(&output).ok(), previous);
            }
        }
    }
}
