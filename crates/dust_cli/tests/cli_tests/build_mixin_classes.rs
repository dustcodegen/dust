use crate::helpers::run_cli;

use super::{DustImport, make_workspace, write_dust_file};

#[test]
fn plain_mixin_class_does_not_change_existing_model_output() {
    let workspace = make_workspace();
    let path = workspace.path().join("lib/user.dart");
    let source = "part 'user.g.dart';\n@Derive([ToString()])\nclass User with _$User { const User(this.id); final int id; }\n";
    write_dust_file(&path, &[DustImport::Derive], source);
    let run = run_cli(["build", "--root", workspace.path().to_str().unwrap()]);
    assert_eq!(run.exit_code, 0, "{}", run.stderr);
    let expected = std::fs::read(workspace.path().join("lib/user.g.dart")).unwrap();
    write_dust_file(
        &path,
        &[DustImport::Derive],
        &format!("{source}\nmixin class Helper {{}}\n"),
    );
    for command in ["build", "check"] {
        let run = run_cli([command, "--root", workspace.path().to_str().unwrap()]);
        assert_eq!(run.exit_code, 0, "{}", run.stderr);
        assert_eq!(
            std::fs::read(workspace.path().join("lib/user.g.dart")).unwrap(),
            expected
        );
    }
}
