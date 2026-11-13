use super::{MODEL, build, workspace, write_file};
use std::fs;

#[test]
fn multi_hop_external_aliases_match_direct_output_after_import_retries() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    write_file(
        &model,
        &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let expected = fs::read(root.join("lib/model.g.dart")).unwrap();
    write_file(
        &root.join("deps/bridge/lib/bridge.dart"),
        "export 'first.dart';",
    );
    write_file(
        &root.join("deps/bridge/lib/first.dart"),
        "import 'second.dart' as next; import 'unused.dart'; typedef Pretty = next.Middle; typedef Unused = Missing;",
    );
    write_file(
        &root.join("deps/bridge/lib/second.dart"),
        "import 'package:dust_dart/derive.dart' as d; typedef Middle = d.ToString;",
    );
    write_file(
        &root.join("deps/bridge/lib/unused.dart"),
        "class UnusedDependency {}",
    );
    write_file(
        &model,
        &format!(
            "import 'package:bridge/bridge.dart';\n{}",
            MODEL.replace("@Derive([ToString()])", "@Pretty()")
        ),
    );
    let result = build(root);
    assert_eq!(result.diagnostics, []);
    assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), expected);
    assert_eq!(build(root).diagnostics, []);
}

#[test]
fn external_part_aliases_use_the_owners_import_prefix_and_match_direct_output() {
    let workspace = workspace();
    let root = workspace.path();
    let model = root.join("lib/model.dart");
    write_file(
        &model,
        &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
    );
    assert_eq!(build(root).diagnostics, []);
    let expected = fs::read(root.join("lib/model.g.dart")).unwrap();
    write_file(
        &root.join("deps/bridge/lib/bridge.dart"),
        "import 'package:dust_dart/derive.dart' as d; part 'aliases.dart';",
    );
    write_file(
        &root.join("deps/bridge/lib/aliases.dart"),
        "part of 'bridge.dart'; typedef Pretty = d.ToString;",
    );
    write_file(
        &model,
        &format!(
            "import 'package:bridge/bridge.dart';\n{}",
            MODEL.replace("@Derive([ToString()])", "@Pretty()")
        ),
    );
    assert_eq!(build(root).diagnostics, []);
    assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), expected);
}

#[test]
fn pending_alias_terminal_states_preserve_previous_successful_output() {
    for (source, fails) in [
        ("import 'absent.dart'; typedef Pretty = Missing;", true),
        ("import 'cycle.dart'; typedef Pretty = Other;", false),
        (
            "import 'package:dust_dart/derive.dart' if (dart.library.io) 'foreign.dart' as d; typedef Pretty = d.ToString;",
            true,
        ),
    ] {
        let workspace = workspace();
        let root = workspace.path();
        let model = root.join("lib/model.dart");
        write_file(
            &model,
            &format!("import 'package:dust_dart/derive.dart';\n{MODEL}"),
        );
        assert_eq!(build(root).diagnostics, []);
        let output = fs::read(root.join("lib/model.g.dart")).unwrap();
        let cache = fs::read(root.join(".dart_tool/dust/build_cache_v1.json")).unwrap();
        write_file(&root.join("deps/bridge/lib/bridge.dart"), source);
        write_file(
            &root.join("deps/bridge/lib/cycle.dart"),
            "import 'bridge.dart'; typedef Other = Pretty;",
        );
        write_file(
            &root.join("deps/bridge/lib/foreign.dart"),
            "class ToString { const ToString(); }",
        );
        write_file(
            &model,
            &format!(
                "import 'package:bridge/bridge.dart';\n{}",
                MODEL.replace("@Derive([ToString()])", "@Pretty()")
            ),
        );
        let result = build(root);
        assert_eq!(
            result.has_errors(),
            fails,
            "{source}: {:?}",
            result.diagnostics
        );
        if !fails {
            assert!(result.build_artifacts.is_empty());
            assert_eq!(result.diagnostics, []);
        }
        assert_eq!(fs::read(root.join("lib/model.g.dart")).unwrap(), output);
        assert_eq!(
            fs::read(root.join(".dart_tool/dust/build_cache_v1.json")).unwrap(),
            cache
        );
    }
}
